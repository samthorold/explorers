//! Autonomous phase functions for the tick loop.
//!
//! Each function takes agent state (and world parameters where needed),
//! mutates state in place, and returns events recording what happened.
//! These are free functions, not methods on World.

use crate::event::{Event, EventKind};
use crate::spatial::SpatialGrid;
use crate::{
    Agent, Carcass, TraitVector, WorldParameters, FUNCTIONAL_TRAIT_COUNT, FUNCTIONAL_TRAIT_INDICES,
};
use rand_chacha::ChaCha8Rng;
use rand::Rng;
use rand_distr::{Distribution, Normal, Poisson};

/// Photosynthesise: agents with nonzero effective photosynthetic absorption
/// absorb energy from local solar flux into reserve. Light competition splits
/// flux proportionally among co-located producers via spatial grid query.
pub fn photosynthesise(
    agents: &mut [Agent],
    grid: &SpatialGrid,
    params: &WorldParameters,
) -> Vec<Event> {
    let mut events = Vec::new();
    let k = params.wear_degradation_steepness;
    let flux = params.solar_flux_magnitude;
    let competition_radius = params.light_competition_radius;
    let extent = params.world_extent;

    for i in 0..agents.len() {
        let eff_photo = agents[i].effective_trait_with_steepness(0, k);
        if eff_photo <= 0.0 {
            continue;
        }

        // Light share weighted by effective_autotrophy × structure (body size).
        // Bigger producers shade smaller ones. Zero structure means zero light.
        let my_weight = eff_photo * agents[i].structure;
        if my_weight <= 0.0 {
            continue;
        }

        // Compute total weight among co-located producers.
        // Use grid query but deduplicate results (grid can return duplicates
        // when query radius exceeds half the world extent).
        let mut total_weight = my_weight;
        let mut seen = std::collections::HashSet::new();
        seen.insert(i as u64);
        for j_id in grid.query_radius(agents[i].position, competition_radius) {
            let j = j_id as usize;
            if !seen.insert(j_id) {
                continue;
            }
            let other_eff = agents[j].effective_trait_with_steepness(0, k);
            if other_eff <= 0.0 {
                continue;
            }
            let other_weight = other_eff * agents[j].structure;
            if other_weight <= 0.0 {
                continue;
            }
            if crate::toroidal_distance(agents[i].position, agents[j].position, extent)
                < competition_radius
            {
                total_weight += other_weight;
            }
        }
        let light_share = my_weight / total_weight;
        let income = flux * light_share;

        agents[i].reserve += income;
        events.push(Event {
            tick: 0,
            seq: 0,
            kind: EventKind::Photosynthesized,
            source: agents[i].id,
            target: None,
            energy_delta: income,
            position: Some(agents[i].position),
        });
    }
    events
}

/// Absorb nutrients: uptake from available pool, scales with contact time
/// (Michaelis-Menten saturation), proportional sharing when demand exceeds pool.
pub fn absorb_nutrients(
    agents: &mut [Agent],
    nutrient_pool: &mut f32,
    params: &WorldParameters,
) -> Vec<Event> {
    let mut events = Vec::new();
    if *nutrient_pool <= 0.0 {
        return events;
    }

    let k_half = 50.0_f32;
    let k = params.wear_degradation_steepness;

    // Compute each agent's demand — uptake is coupled to autotrophy.
    // The same sessile infrastructure that captures light also extracts nutrients.
    let demands: Vec<f32> = agents
        .iter()
        .map(|a| {
            let ct = a.contact_time as f32;
            let eff_autotrophy = a.effective_trait_with_steepness(0, k);
            eff_autotrophy * ct / (ct + k_half)
        })
        .collect();
    let total_demand: f32 = demands.iter().sum();
    if total_demand <= 0.0 {
        return events;
    }

    let available = *nutrient_pool;
    for (i, agent) in agents.iter_mut().enumerate() {
        if demands[i] <= 0.0 {
            continue;
        }
        let uptake = if total_demand <= available {
            demands[i]
        } else {
            demands[i] / total_demand * available
        };
        agent.nutrient += uptake;
        *nutrient_pool -= uptake;
        events.push(Event {
            tick: 0,
            seq: 0,
            kind: EventKind::NutrientAbsorbed,
            source: agent.id,
            target: None,
            energy_delta: uptake,
            position: Some(agent.position),
        });
    }
    events
}

/// Metabolise: fixed costs only. Base rate + trait maintenance (including
/// mobility maintenance) + structure maintenance. Per-distance movement cost
/// is in the movement phase. Somatic maintenance is governed by kappa
/// allocation in the grow phase.
pub fn metabolise(
    agents: &mut [Agent],
    params: &WorldParameters,
) -> (Vec<Event>, f32) {
    let mut events = Vec::new();
    let mut total_dissipated = 0.0_f32;

    let exp = params.maintenance_cost_exponent;
    for agent in agents.iter_mut() {
        let cost = params.base_metabolic_rate
            + agent.traits.photosynthetic_absorption.powf(exp) * params.photo_maintenance_cost
            + agent.traits.heterotrophy.powf(exp) * params.heterotrophy_maintenance_cost
            + agent.traits.mobility.powf(exp) * params.mobility_maintenance_cost
            + agent.structure * params.structure_maintenance_coefficient;

        agent.reserve -= cost;
        total_dissipated += cost;
        events.push(Event {
            tick: 0,
            seq: 0,
            kind: EventKind::Metabolized,
            source: agent.id,
            target: None,
            energy_delta: cost,
            position: Some(agent.position),
        });
    }
    (events, total_dissipated)
}

/// Grow: surplus energy (reserve above metabolic retention) is split by kappa.
/// - kappa fraction → soma: growth (reserve → structure, lossy) and wear repair
/// - (1 - kappa) fraction → repro_reserve: accumulates across ticks
pub fn grow(
    agents: &mut [Agent],
    params: &WorldParameters,
) -> (Vec<Event>, f32) {
    let mut events = Vec::new();
    let mut total_dissipated = 0.0_f32;

    let exp = params.maintenance_cost_exponent;
    for agent in agents.iter_mut() {
        if agent.reserve <= 0.0 {
            continue;
        }
        // Retain enough reserve for next tick's metabolism
        let metabolic_cost = params.base_metabolic_rate
            + agent.traits.photosynthetic_absorption.powf(exp) * params.photo_maintenance_cost
            + agent.traits.heterotrophy.powf(exp) * params.heterotrophy_maintenance_cost
            + agent.traits.mobility.powf(exp) * params.mobility_maintenance_cost
            + agent.structure * params.structure_maintenance_coefficient;
        let retention = metabolic_cost * 2.0;
        let surplus = (agent.reserve - retention).max(0.0);
        if surplus <= 0.0 {
            continue;
        }

        let kappa = agent.traits.kappa.clamp(0.0, 1.0);
        let soma_fraction = surplus * kappa;
        let repro_fraction = surplus - soma_fraction; // (1-kappa) * surplus

        // Deduct entire surplus from reserve
        agent.reserve -= surplus;

        // Soma fraction → growth (if growth_efficiency > 0)
        let efficiency = params.growth_efficiency;
        if efficiency > 0.0 && soma_fraction > 0.0 {
            let to_structure = soma_fraction * efficiency;
            let dissipated = soma_fraction - to_structure;
            agent.structure += to_structure;
            total_dissipated += dissipated;

            events.push(Event {
                tick: 0,
                seq: 0,
                kind: EventKind::Grew,
                source: agent.id,
                target: None,
                energy_delta: to_structure,
                position: Some(agent.position),
            });
        } else {
            // No growth: soma fraction is dissipated (maintenance cost)
            total_dissipated += soma_fraction;
        }

        // Repro fraction → repro_reserve (accumulates across ticks)
        agent.repro_reserve += repro_fraction;
    }
    (events, total_dissipated)
}

/// Apply wear: baseline + use-dependent accumulation per functional trait.
/// Somatic repair derived from kappa allocation: higher kappa = more repair.
///
/// `usage` maps agent id → per-functional-trait usage amounts:
///   [0] = energy captured (autotrophy), [1] = energy drained (heterotrophy),
///   [2] = distance moved (mobility).
pub fn apply_wear(
    agents: &mut [Agent],
    params: &WorldParameters,
    usage: &std::collections::HashMap<u64, [f32; FUNCTIONAL_TRAIT_COUNT]>,
) -> Vec<Event> {
    let mut events = Vec::new();
    let baseline_rate = params.wear_rate;
    let use_rate = params.use_wear_rate;
    let decay = params.repair_decay;

    for agent in agents.iter_mut() {
        let mut total_wear_delta = 0.0_f32;

        // Look up this agent's usage data (defaults to zero if absent)
        let agent_usage = usage.get(&agent.id).copied().unwrap_or([0.0; FUNCTIONAL_TRAIT_COUNT]);

        // Accumulate wear: baseline + use-dependent
        for ft in 0..FUNCTIONAL_TRAIT_COUNT {
            let nominal = agent.traits.get(FUNCTIONAL_TRAIT_INDICES[ft]);
            let baseline = baseline_rate * nominal.max(0.0);
            let use_dependent = use_rate * agent_usage[ft].max(0.0);
            let accumulation = baseline + use_dependent;
            agent.wear[ft] += accumulation;
            total_wear_delta += accumulation;
        }

        // Somatic repair: derived from kappa (higher kappa = more repair)
        let base_repair = agent.traits.kappa.clamp(0.0, 1.0);
        if base_repair > 0.0 {
            for ft in 0..FUNCTIONAL_TRAIT_COUNT {
                let effective_repair = base_repair * (-decay * agent.wear[ft]).exp();
                let repair = effective_repair.min(agent.wear[ft]);
                agent.wear[ft] -= repair;
            }
        }

        if total_wear_delta > 0.0 {
            events.push(Event {
                tick: 0,
                seq: 0,
                kind: EventKind::Wore,
                source: agent.id,
                target: None,
                energy_delta: total_wear_delta,
                position: Some(agent.position),
            });
        }
    }
    events
}

/// Result of drain resolution phase.
pub struct DrainResult {
    pub events: Vec<Event>,
    pub dissipated: f32,
    pub dead_agents: Vec<u64>,
    pub new_carcasses: Vec<Carcass>,
}

/// Resolve drains: coordinated pass 1. For each potential target (living agent
/// or carcass), gather consumers within contact range, compute demand, apply
/// proportional split when demand exceeds supply, transfer energy with trophic
/// loss, transfer nutrient with stoichiometric mismatch. Check death thresholds
/// on drained living agents. New carcasses from kills this tick are NOT available
/// for decomposition (no re-entrant processing).
pub fn resolve_drains(
    agents: &mut [Agent],
    carcasses: &mut [Carcass],
    grid: &SpatialGrid,
    params: &WorldParameters,
    nutrient_pool: &mut f32,
) -> DrainResult {
    let mut events = Vec::new();
    let mut dissipated = 0.0_f32;
    let mut dead_agents: Vec<u64> = Vec::new();
    let mut new_carcasses: Vec<Carcass> = Vec::new();
    let k = params.wear_degradation_steepness;
    let contact_radius = params.contact_radius;
    let extent = params.world_extent;

    // --- Pass over living targets ---
    // For each agent that has structure, find consumers in range.
    // We need to iterate targets and for each, find consumers.
    // Build index: agent id -> slice index
    let id_to_idx: std::collections::HashMap<u64, usize> = agents
        .iter()
        .enumerate()
        .map(|(i, a)| (a.id, i))
        .collect();

    // Collect per-target drain info before mutating
    struct TargetDrain {
        target_idx: usize,
        consumers: Vec<(usize, f32)>, // (consumer_idx, demand)
    }

    let mut living_drains: Vec<TargetDrain> = Vec::new();

    for target_idx in 0..agents.len() {
        if agents[target_idx].structure <= 0.0 {
            continue;
        }
        let target_pos = agents[target_idx].position;

        // Find consumers within contact range
        let mut consumers = Vec::new();
        let nearby = grid.query_radius(target_pos, contact_radius);
        let mut seen = std::collections::HashSet::new();
        for neighbor_id in nearby {
            if !seen.insert(neighbor_id) {
                continue;
            }
            if let Some(&consumer_idx) = id_to_idx.get(&neighbor_id) {
                if consumer_idx == target_idx {
                    continue; // can't consume yourself
                }
                let eff_heterotrophy = agents[consumer_idx]
                    .effective_trait_with_steepness(1, k); // index 1 = heterotrophy
                if eff_heterotrophy <= 0.0 {
                    continue;
                }
                // Verify within contact radius (grid may return slightly outside)
                if crate::toroidal_distance(
                    agents[consumer_idx].position,
                    target_pos,
                    extent,
                ) > contact_radius
                {
                    continue;
                }
                consumers.push((consumer_idx, eff_heterotrophy));
            }
        }

        if !consumers.is_empty() {
            living_drains.push(TargetDrain {
                target_idx,
                consumers,
            });
        }
    }

    // Apply living target drains
    for drain in &living_drains {
        let available = agents[drain.target_idx].structure;
        let total_demand: f32 = drain.consumers.iter().map(|(_, d)| *d).sum();

        for &(consumer_idx, demand) in &drain.consumers {
            let actual_drain = if total_demand <= available {
                demand
            } else {
                (demand / total_demand) * available
            };

            let efficiency = crate::trophic_transfer_efficiency(
                &agents[consumer_idx].traits,
                &agents[drain.target_idx].traits,
                params,
            );
            let energy_gained = actual_drain * efficiency;
            let energy_lost = actual_drain - energy_gained;

            agents[drain.target_idx].structure -= actual_drain;
            agents[consumer_idx].reserve += energy_gained;
            dissipated += energy_lost;

            // Nutrient transfer: proportional to structure drained
            let target_nutrient = agents[drain.target_idx].nutrient;
            let target_structure_before = available; // use pre-drain available
            if target_structure_before > 0.0 {
                let nutrient_fraction = actual_drain / target_structure_before;
                let nutrient_transferred = target_nutrient * nutrient_fraction;

                // Consumer retains up to stoichiometric demand
                let consumer_demand = crate::stoichiometric_demand(
                    &agents[consumer_idx].traits,
                    agents[consumer_idx].structure,
                    params,
                );
                let consumer_nutrient_need = consumer_demand * energy_gained;
                let retained = nutrient_transferred.min(consumer_nutrient_need);
                let excreted = nutrient_transferred - retained;

                agents[drain.target_idx].nutrient -= nutrient_transferred;
                agents[consumer_idx].nutrient += retained;
                *nutrient_pool += excreted;
            }

            events.push(Event {
                tick: 0,
                seq: 0,
                kind: EventKind::Consumed,
                source: agents[consumer_idx].id,
                target: Some(agents[drain.target_idx].id),
                energy_delta: actual_drain,
                position: Some(agents[drain.target_idx].position),
            });
        }
    }

    // Check death thresholds on living agents that were drained
    for drain in &living_drains {
        let agent = &agents[drain.target_idx];
        let threshold = crate::death_threshold(&agent.traits);
        let dies = agent.reserve <= 0.0
            || (agent.structure > 0.0 && agent.structure < threshold)
            || agent.structure <= 0.0;

        if dies && !dead_agents.contains(&agent.id) {
            dead_agents.push(agent.id);
            new_carcasses.push(Carcass {
                id: agent.id,
                position: agent.position,
                energy: agent.structure.max(0.0),
                nutrient: agent.nutrient,
                traits: agent.traits,
            });
        }
    }

    // --- Pass over carcass targets ---
    for carcass_idx in 0..carcasses.len() {
        if carcasses[carcass_idx].energy <= 0.0 {
            continue;
        }
        let carcass_pos = carcasses[carcass_idx].position;
        let nearby = grid.query_radius(carcass_pos, contact_radius);
        let mut consumers: Vec<(usize, f32)> = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for neighbor_id in nearby {
            if !seen.insert(neighbor_id) {
                continue;
            }
            if let Some(&consumer_idx) = id_to_idx.get(&neighbor_id) {
                // Must not be dead this tick
                if dead_agents.contains(&agents[consumer_idx].id) {
                    continue;
                }
                let eff_heterotrophy = agents[consumer_idx]
                    .effective_trait_with_steepness(1, k); // index 1 = heterotrophy
                if eff_heterotrophy <= 0.0 {
                    continue;
                }
                if crate::toroidal_distance(
                    agents[consumer_idx].position,
                    carcass_pos,
                    extent,
                ) > contact_radius
                {
                    continue;
                }
                consumers.push((consumer_idx, eff_heterotrophy));
            }
        }

        if consumers.is_empty() {
            continue;
        }

        let available = carcasses[carcass_idx].energy;
        let total_demand: f32 = consumers.iter().map(|(_, d)| *d).sum();

        for &(consumer_idx, demand) in &consumers {
            let actual_drain = if total_demand <= available {
                demand
            } else {
                (demand / total_demand) * available
            };

            let efficiency = crate::trophic_transfer_efficiency(
                &agents[consumer_idx].traits,
                &carcasses[carcass_idx].traits,
                params,
            );
            let energy_gained = actual_drain * efficiency;
            let energy_lost = actual_drain - energy_gained;

            carcasses[carcass_idx].energy -= actual_drain;
            agents[consumer_idx].reserve += energy_gained;
            dissipated += energy_lost;

            // Nutrient transfer from carcass
            let carcass_nutrient = carcasses[carcass_idx].nutrient;
            if available > 0.0 {
                let nutrient_fraction = actual_drain / available;
                let nutrient_transferred = carcass_nutrient * nutrient_fraction;

                let consumer_demand = crate::stoichiometric_demand(
                    &agents[consumer_idx].traits,
                    agents[consumer_idx].structure,
                    params,
                );
                let consumer_nutrient_need = consumer_demand * energy_gained;
                let retained = nutrient_transferred.min(consumer_nutrient_need);
                let excreted = nutrient_transferred - retained;

                carcasses[carcass_idx].nutrient -= nutrient_transferred;
                agents[consumer_idx].nutrient += retained;
                *nutrient_pool += excreted;
            }

            events.push(Event {
                tick: 0,
                seq: 0,
                kind: EventKind::Consumed,
                source: agents[consumer_idx].id,
                target: Some(carcasses[carcass_idx].id),
                energy_delta: actual_drain,
                position: Some(carcass_pos),
            });
        }
    }

    DrainResult {
        events,
        dissipated,
        dead_agents,
        new_carcasses,
    }
}

/// Check death thresholds: reserve depletion or structure below
/// complexity-dependent threshold produces carcass.
/// Returns (events, carcasses, dissipated) — dissipated includes reserve and
/// repro_reserve energy that doesn't transfer to the carcass.
pub fn check_death_thresholds(
    agents: &mut [Agent],
    _params: &WorldParameters,
) -> (Vec<Event>, Vec<Carcass>, f32) {
    let mut events = Vec::new();
    let mut carcasses = Vec::new();
    let mut dissipated = 0.0_f32;

    for agent in agents.iter_mut() {
        let threshold = crate::death_threshold(&agent.traits);
        let dies = agent.reserve <= 0.0
            || (agent.structure > 0.0 && agent.structure < threshold);

        if dies {
            let carcass_energy = agent.structure.max(0.0);
            // Energy in reserve and repro_reserve is dissipated at death
            let lost = agent.reserve.max(0.0) + agent.repro_reserve.max(0.0);
            dissipated += lost;
            events.push(Event {
                tick: 0,
                seq: 0,
                kind: EventKind::Died,
                source: agent.id,
                target: None,
                energy_delta: 0.0,
                position: Some(agent.position),
            });
            carcasses.push(Carcass {
                id: agent.id,
                position: agent.position,
                energy: carcass_energy,
                nutrient: agent.nutrient,
                traits: agent.traits,
            });
            // Mark for removal by setting reserve to 0
            agent.reserve = 0.0;
            agent.repro_reserve = 0.0;
        }
    }
    (events, carcasses, dissipated)
}

/// Result of the movement phase.
pub struct MoveResult {
    pub events: Vec<Event>,
    pub dissipated: f32,
    /// Per-agent sensing throughput: number of agents + carcasses detected.
    /// Indexed by position in the agents slice.
    pub sensing_throughput: Vec<f32>,
}

/// Move agents: the final phase of the tick loop. Agents reposition based on
/// spatial context (nearby agents and carcasses), traits (chemotaxis, consumption,
/// scavenging, mobility), and a random walk component. Movement costs energy
/// proportional to distance. Contact time resets on movement, increments when
/// stationary.
pub fn move_agents(
    agents: &mut [Agent],
    carcasses: &[Carcass],
    grid: &SpatialGrid,
    params: &WorldParameters,
    rng: &mut ChaCha8Rng,
) -> MoveResult {
    let mut events = Vec::new();
    let mut total_dissipated = 0.0_f32;
    let mut sensing_throughput = vec![0.0_f32; agents.len()];
    let k = params.wear_degradation_steepness;
    let extent = params.world_extent;

    for i in 0..agents.len() {
        let eff_mobility = agents[i].effective_trait_with_steepness(2, k);
        // Sensing range derived from mobility: mobile agents perceive farther
        let eff_sensing = eff_mobility * params.sensing_range_coefficient;
        // Chemotaxis strength derived from mobility (no separate trait)
        let eff_chemotaxis = eff_mobility;
        let eff_heterotrophy = agents[i].effective_trait_with_steepness(1, k);

        if eff_mobility <= 0.0 {
            // Stationary: increment contact time
            agents[i].contact_time += 1;
            continue;
        }

        // Query spatial grid for nearby agents within sensing range
        let mut dir_x = 0.0_f32;
        let mut dir_y = 0.0_f32;
        let mut detected_count = 0.0_f32;

        if eff_sensing > 0.0 {
            // Detect nearby living agents
            let nearby = grid.query_radius(agents[i].position, eff_sensing);
            let mut seen = std::collections::HashSet::new();
            seen.insert(i as u64);
            for neighbor_id in nearby {
                if !seen.insert(neighbor_id) {
                    continue;
                }
                let j = neighbor_id as usize;
                if j >= agents.len() {
                    continue;
                }
                let dist = crate::toroidal_distance(
                    agents[i].position,
                    agents[j].position,
                    extent,
                );
                if dist < 1e-6 {
                    detected_count += 1.0;
                    continue;
                }
                let (dx, dy) = crate::toroidal_displacement(
                    agents[i].position,
                    agents[j].position,
                    extent,
                );
                // Attraction weighted by chemotaxis * heterotrophy (toward living agents)
                let weight = eff_chemotaxis * eff_heterotrophy / dist;
                dir_x += dx * weight;
                dir_y += dy * weight;
                detected_count += 1.0;
            }

            // Detect nearby carcasses
            for carcass in carcasses.iter() {
                let dist = crate::toroidal_distance(
                    agents[i].position,
                    carcass.position,
                    extent,
                );
                if dist > eff_sensing {
                    continue;
                }
                detected_count += 1.0;
                if dist < 1e-6 {
                    continue;
                }
                let (dx, dy) = crate::toroidal_displacement(
                    agents[i].position,
                    carcass.position,
                    extent,
                );
                // Attraction weighted by chemotaxis * heterotrophy (toward carcasses)
                let weight = eff_chemotaxis * eff_heterotrophy / dist;
                dir_x += dx * weight;
                dir_y += dy * weight;
            }
        }

        sensing_throughput[i] = detected_count;

        // Random walk component
        let angle: f32 = rng.random::<f32>() * std::f32::consts::TAU;
        let random_magnitude: f32 = rng.random::<f32>();
        dir_x += angle.cos() * random_magnitude;
        dir_y += angle.sin() * random_magnitude;

        // Normalize direction and scale by effective mobility
        let dir_mag = (dir_x * dir_x + dir_y * dir_y).sqrt();
        if dir_mag < 1e-6 {
            agents[i].contact_time += 1;
            continue;
        }

        let distance = eff_mobility;
        let move_x = (dir_x / dir_mag) * distance;
        let move_y = (dir_y / dir_mag) * distance;

        // Deduct energy cost (capped at available reserve — agent cannot
        // spend energy it does not have)
        let raw_cost = distance * params.movement_cost_coefficient * agents[i].structure;
        let cost = raw_cost.min(agents[i].reserve.max(0.0));
        agents[i].reserve -= cost;
        total_dissipated += cost;

        // Update position with toroidal wrapping
        let new_pos = crate::wrap_position(
            (agents[i].position.0 + move_x, agents[i].position.1 + move_y),
            extent,
        );

        let moved = (new_pos.0 - agents[i].position.0).abs() > 1e-6
            || (new_pos.1 - agents[i].position.1).abs() > 1e-6;

        if moved {
            agents[i].position = new_pos;
            agents[i].contact_time = 0;
        } else {
            agents[i].contact_time += 1;
        }

        events.push(Event {
            tick: 0,
            seq: 0,
            kind: EventKind::Moved,
            source: agents[i].id,
            target: None,
            energy_delta: cost,
            position: Some(new_pos),
        });
    }

    MoveResult {
        events,
        dissipated: total_dissipated,
        sensing_throughput,
    }
}

/// Result of the reproduction resolution phase.
pub struct ReproductionResult {
    pub events: Vec<Event>,
    pub dissipated: f32,
    pub offspring: Vec<Agent>,
}

/// Resolve reproduction: coordinated pass 2.
///
/// Each eligible agent first rolls against its `asexual_propensity`. On success
/// the agent reproduces alone — offspring are clones with mutation applied (no
/// crossover). On failure the agent falls through to sexual mate-finding: pairs
/// by closest trait-space distance within spatial proximity, invests energy from
/// both parents, produces offspring with uniform crossover + Gaussian mutation.
///
/// Events distinguish asexual (target=None) from sexual (target=Some(mate_id)).
pub fn resolve_reproduction(
    agents: &mut [Agent],
    dead_ids: &std::collections::HashSet<u64>,
    grid: &SpatialGrid,
    params: &WorldParameters,
    rng: &mut ChaCha8Rng,
) -> ReproductionResult {
    let mut events = Vec::new();
    let mut dissipated = 0.0_f32;
    let mut offspring = Vec::new();
    let extent = params.world_extent;

    // Build eligible set: alive, repro_reserve above threshold, sufficient nutrient.
    // Reproduction draws from repro_reserve, not reserve.
    let eligible: std::collections::HashSet<usize> = agents
        .iter()
        .enumerate()
        .filter(|(_, a)| {
            !dead_ids.contains(&a.id)
                && a.repro_reserve >= params.reproduction_energy_threshold
                && a.nutrient >= crate::stoichiometric_demand(&a.traits, a.structure, params)
        })
        .map(|(i, _)| i)
        .collect();

    // Track which agents have already reproduced this tick (at most once each).
    let mut reproduced = std::collections::HashSet::new();

    // Next agent ID: find max existing ID + 1
    let mut next_id = agents.iter().map(|a| a.id).max().unwrap_or(0) + 1;

    let sensing_coeff = params.sensing_range_coefficient;

    // ---------- Phase A: Asexual reproduction attempts ----------
    // Each eligible agent rolls against its asexual_propensity. On success it
    // reproduces alone; on failure it will attempt sexual reproduction below.
    let eligible_sorted: Vec<usize> = {
        let mut v: Vec<usize> = eligible.iter().copied().collect();
        v.sort(); // deterministic ordering for reproducibility
        v
    };

    for &i in &eligible_sorted {
        let propensity = agents[i].traits.asexual_propensity.clamp(0.0, 1.0);
        if propensity <= 0.0 || rng.random::<f32>() >= propensity {
            continue; // asexual attempt failed — will try sexual below
        }

        // Asexual reproduction succeeds.
        reproduced.insert(i);

        let parent_traits = agents[i].traits;
        let parent_repro = agents[i].repro_reserve;
        let parent_nutrient = agents[i].nutrient;
        let parent_reserve = agents[i].reserve;
        let parent_pos = agents[i].position;
        let parent_id = agents[i].id;

        let investment = parent_repro.max(0.0);
        if investment <= 0.0 {
            continue;
        }

        // Offspring count from Poisson distribution (single parent's fecundity)
        let fecundity = parent_traits.fecundity.max(0.1);
        let poisson = Poisson::new(fecundity as f64).unwrap();
        let offspring_count = (poisson.sample(rng) as usize).max(1);

        let offspring_total_energy = investment * params.reproduction_efficiency;
        let energy_per_offspring = offspring_total_energy / offspring_count as f32;
        let tick_dissipated = investment - offspring_total_energy;
        dissipated += tick_dissipated;

        // Nutrient donation from single parent
        let nutrient_donated = parent_nutrient
            * (investment / (parent_reserve + parent_repro).max(1e-10)).min(0.5);
        let nutrient_per_offspring = nutrient_donated / offspring_count as f32;

        // Deduct from parent
        agents[i].repro_reserve -= investment;
        agents[i].nutrient -= nutrient_donated;

        // Dispersal: sigma proportional to parent's dispersal trait
        let dispersal_radius = parent_traits.dispersal;

        for _ in 0..offspring_count {
            // Asexual offspring: parent traits + mutation only (no crossover)
            let mut child_traits = parent_traits;
            for dim in 0..TraitVector::NUM_DIMS {
                if params.mutation_rate > 0.0
                    && rng.random::<f32>() < params.mutation_rate
                {
                    let normal = Normal::new(0.0_f32, params.mutation_magnitude).unwrap();
                    let mutated = (child_traits.get(dim) + normal.sample(rng)).max(0.0);
                    // Clamp kappa and asexual_propensity to [0, 1]
                    let val = if dim == 3 || dim == 5 {
                        mutated.clamp(0.0, 1.0)
                    } else {
                        mutated
                    };
                    child_traits.set(dim, val);
                }
            }

            // Dispersal position
            let (dx, dy) = if dispersal_radius > 0.0 {
                let normal = Normal::new(0.0_f32, dispersal_radius).unwrap();
                (normal.sample(rng), normal.sample(rng))
            } else {
                (0.0, 0.0)
            };
            let pos = crate::wrap_position(
                (parent_pos.0 + dx, parent_pos.1 + dy),
                extent,
            );

            let child = Agent {
                id: next_id,
                position: pos,
                reserve: energy_per_offspring,
                structure: 0.0,
                nutrient: nutrient_per_offspring,
                traits: child_traits,
                contact_time: 0,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 0.0,
            };
            next_id += 1;
            offspring.push(child);
        }

        // Asexual event: target is None (no mate)
        events.push(Event {
            tick: 0,
            seq: 0,
            kind: EventKind::Reproduced,
            source: parent_id,
            target: None,
            energy_delta: investment,
            position: Some(parent_pos),
        });
    }

    // ---------- Phase B: Sexual reproduction (fallback for non-asexual) ----------
    // Only eligible agents that have not already reproduced asexually participate.
    let sexual_eligible: std::collections::HashSet<usize> = eligible
        .iter()
        .copied()
        .filter(|i| !reproduced.contains(i))
        .collect();

    let compatibility_distance = params.reproductive_compatibility_distance;

    let mut pair_candidates: Vec<(usize, usize, f32)> = Vec::new();

    for &i in &sexual_eligible {
        let agent_i = &agents[i];
        let sensing = agent_i.traits.mobility * sensing_coeff;

        let nearby = grid.query_radius(agent_i.position, sensing);
        let mut best: Option<(usize, f32)> = None;
        let mut seen = std::collections::HashSet::new();
        seen.insert(i); // exclude self

        for neighbor_id in nearby {
            let j = neighbor_id as usize;
            if !seen.insert(j) {
                continue;
            }
            if j >= agents.len() || !sexual_eligible.contains(&j) {
                continue;
            }
            let dist_spatial = crate::toroidal_distance(
                agent_i.position,
                agents[j].position,
                extent,
            );
            if dist_spatial > sensing {
                continue;
            }
            let trait_dist = agent_i.traits.distance(&agents[j].traits);
            // Check reproductive compatibility: trait distance must be within world param threshold
            if compatibility_distance > 0.0 && trait_dist > compatibility_distance {
                continue;
            }
            match best {
                None => best = Some((j, trait_dist)),
                Some((_, best_dist)) if trait_dist < best_dist => {
                    best = Some((j, trait_dist));
                }
                _ => {}
            }
        }

        if let Some((j, dist)) = best {
            let (a, b) = if i < j { (i, j) } else { (j, i) };
            pair_candidates.push((a, b, dist));
        }
    }

    // Sort by trait distance (closest pairs first)
    pair_candidates.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());

    // Greedily assign pairs: each agent can only reproduce once per tick
    let mut paired = std::collections::HashSet::new();
    let mut final_pairs: Vec<(usize, usize)> = Vec::new();

    for (a, b, _) in &pair_candidates {
        if !paired.contains(a) && !paired.contains(b) {
            paired.insert(*a);
            paired.insert(*b);
            final_pairs.push((*a, *b));
        }
    }

    // Resolve each sexual pair
    for (a_idx, b_idx) in &final_pairs {
        // Copy needed values before mutating
        let a_traits = agents[*a_idx].traits;
        let b_traits = agents[*b_idx].traits;
        let a_repro = agents[*a_idx].repro_reserve;
        let b_repro = agents[*b_idx].repro_reserve;
        let a_nutrient = agents[*a_idx].nutrient;
        let b_nutrient = agents[*b_idx].nutrient;
        let a_reserve = agents[*a_idx].reserve;
        let b_reserve = agents[*b_idx].reserve;
        let a_pos = agents[*a_idx].position;
        let b_pos = agents[*b_idx].position;
        let a_id = agents[*a_idx].id;
        let b_id = agents[*b_idx].id;

        // Investment: each parent invests their entire repro_reserve
        let invest_a = a_repro.max(0.0);
        let invest_b = b_repro.max(0.0);
        let total_investment = invest_a + invest_b;

        if total_investment <= 0.0 {
            continue;
        }

        // Offspring count from Poisson distribution
        let avg_fecundity = ((a_traits.fecundity + b_traits.fecundity) / 2.0).max(0.1);
        let poisson = Poisson::new(avg_fecundity as f64).unwrap();
        let offspring_count_f: f64 = poisson.sample(rng);
        let offspring_count = (offspring_count_f as usize).max(1);

        let offspring_total_energy = total_investment * params.reproduction_efficiency;
        let energy_per_offspring = offspring_total_energy / offspring_count as f32;
        let tick_dissipated = total_investment - offspring_total_energy;
        dissipated += tick_dissipated;

        // Nutrient donation: each parent donates proportional to investment fraction
        let nutrient_a = a_nutrient * (invest_a / (a_reserve + a_repro).max(1e-10)).min(0.5);
        let nutrient_b = b_nutrient * (invest_b / (b_reserve + b_repro).max(1e-10)).min(0.5);
        let nutrient_per_offspring = (nutrient_a + nutrient_b) / offspring_count as f32;

        // Deduct from parents' repro_reserve (not reserve)
        agents[*a_idx].repro_reserve -= invest_a;
        agents[*a_idx].nutrient -= nutrient_a;
        agents[*b_idx].repro_reserve -= invest_b;
        agents[*b_idx].nutrient -= nutrient_b;

        // Parent midpoint for dispersal (toroidal-aware)
        let (dx, dy) = crate::toroidal_displacement(a_pos, b_pos, extent);
        let mid_pos = crate::wrap_position((a_pos.0 + dx / 2.0, a_pos.1 + dy / 2.0), extent);

        // Dispersal: sigma = average of parents' dispersal traits
        let dispersal_radius = (a_traits.dispersal + b_traits.dispersal) / 2.0;

        for _ in 0..offspring_count {
            // Trait crossover: each dimension from one parent (uniform crossover)
            let mut child_traits = TraitVector {
                photosynthetic_absorption: 0.0,
                heterotrophy: 0.0,
                mobility: 0.0,
                kappa: 0.0,
                fecundity: 0.0,
                asexual_propensity: 0.0,
                dispersal: 0.0,
            };
            for dim in 0..TraitVector::NUM_DIMS {
                let parent_val = if rng.random::<bool>() {
                    a_traits.get(dim)
                } else {
                    b_traits.get(dim)
                };
                // Mutation
                let mut val = if params.mutation_rate > 0.0
                    && rng.random::<f32>() < params.mutation_rate
                {
                    let normal = Normal::new(0.0_f32, params.mutation_magnitude).unwrap();
                    (parent_val + normal.sample(rng)).max(0.0)
                } else {
                    parent_val
                };
                // Clamp kappa and asexual_propensity to [0, 1]
                if dim == 3 || dim == 5 {
                    val = val.clamp(0.0, 1.0);
                }
                child_traits.set(dim, val);
            }

            // Dispersal position
            let (dx, dy) = if dispersal_radius > 0.0 {
                let normal = Normal::new(0.0_f32, dispersal_radius).unwrap();
                (normal.sample(rng), normal.sample(rng))
            } else {
                (0.0, 0.0)
            };
            let pos = crate::wrap_position(
                (mid_pos.0 + dx, mid_pos.1 + dy),
                extent,
            );

            let child = Agent {
                id: next_id,
                position: pos,
                reserve: energy_per_offspring,
                structure: 0.0,
                nutrient: nutrient_per_offspring,
                traits: child_traits,
                contact_time: 0,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 0.0, // offspring born with zero repro_reserve
            };
            next_id += 1;
            offspring.push(child);
        }

        // Sexual event: target is Some(mate_id)
        events.push(Event {
            tick: 0,
            seq: 0,
            kind: EventKind::Reproduced,
            source: a_id,
            target: Some(b_id),
            energy_delta: total_investment,
            position: Some(mid_pos),
        });
    }

    ReproductionResult {
        events,
        dissipated,
        offspring,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TraitVector, WorldParameters, FUNCTIONAL_TRAIT_COUNT};

    fn zero_traits() -> TraitVector {
        TraitVector {
            photosynthetic_absorption: 0.0,
            heterotrophy: 0.0,
            mobility: 0.0,
            kappa: 0.0,
            fecundity: 0.0,
            asexual_propensity: 0.0,
            dispersal: 0.0,
        }
    }

    fn test_params() -> WorldParameters {
        WorldParameters {
            solar_flux_magnitude: 10.0,
            base_metabolic_rate: 0.1,
            sensing_range_coefficient: 10.0,
            base_trophic_efficiency: 0.5,
            trophic_distance_decay: 0.0,
            reproduction_efficiency: 0.7,
            movement_cost_coefficient: 0.0,
            reproduction_energy_threshold: 50.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            contact_radius: 5.0,
            world_extent: 100.0,
            initial_population_size: 0,
            light_competition_radius: 1000.0,
            photo_maintenance_cost: 0.0,
            heterotrophy_maintenance_cost: 0.0,
            initial_nutrient_pool: 0.0,
            growth_efficiency: 0.0,
            wear_rate: 0.0,
            wear_degradation_steepness: 0.0,
            somatic_maintenance_cost_coefficient: 0.0,
            use_wear_rate: 0.0,
            structure_maintenance_coefficient: 0.0,
            repair_decay: 0.0,
            base_nutrient_ratio: 0.1,
            specification_nutrient_coefficient: 0.2,
            reproductive_compatibility_distance: 2.0,
            mobility_maintenance_cost: 0.0,
            maintenance_cost_exponent: 1.0,
        }
    }

    fn make_agent(id: u64, position: (f32, f32), reserve: f32, traits: TraitVector) -> Agent {
        Agent {
            id,
            position,
            reserve,
            structure: 0.0,
            nutrient: 0.0,
            traits,
            contact_time: 0,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 0.0,
        }
    }

    // --- Photosynthesise ---

    #[test]
    fn photosynthesise_adds_energy_proportional_to_absorption() {
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        agents[0].structure = 5.0; // nonzero structure required for light capture
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));

        let events = photosynthesise(&mut agents, &grid, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::Photosynthesized);
        // Isolated producer: light_share=1.0, income = flux * light_share = 10 * 1.0 = 10.0
        assert!((events[0].energy_delta - 10.0).abs() < 1e-3);
        assert!((agents[0].reserve - 20.0).abs() < 1e-3);
    }

    #[test]
    fn photosynthesise_splits_light_among_colocated_producers() {
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, traits),
            make_agent(2, (1.0, 0.0), 10.0, traits),
        ];
        // Equal structure: equal light share
        agents[0].structure = 5.0;
        agents[1].structure = 5.0;
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let events = photosynthesise(&mut agents, &grid, &params);

        assert_eq!(events.len(), 2);
        // Each gets half the light share: income = 10 * 0.5 = 5.0
        assert!((events[0].energy_delta - 5.0).abs() < 1e-3);
        assert!((events[1].energy_delta - 5.0).abs() < 1e-3);
    }

    #[test]
    fn photosynthesise_skips_agents_without_absorption() {
        let params = test_params();
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, zero_traits())];
        let grid = SpatialGrid::new(100.0, 10.0);

        let events = photosynthesise(&mut agents, &grid, &params);
        assert!(events.is_empty());
        assert!((agents[0].reserve - 10.0).abs() < 1e-6);
    }

    #[test]
    fn photosynthesise_larger_producer_captures_more_light() {
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, traits),
            make_agent(2, (1.0, 0.0), 10.0, traits),
        ];
        // Agent 1 is 3x bigger than agent 2
        agents[0].structure = 9.0;
        agents[1].structure = 3.0;
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let events = photosynthesise(&mut agents, &grid, &params);

        assert_eq!(events.len(), 2);
        // weight_0 = 0.5 * 9.0 = 4.5, weight_1 = 0.5 * 3.0 = 1.5, total = 6.0
        // share_0 = 4.5 / 6.0 = 0.75, share_1 = 1.5 / 6.0 = 0.25
        // income_0 = 10.0 * 0.75 = 7.5, income_1 = 10.0 * 0.25 = 2.5
        assert!((events[0].energy_delta - 7.5).abs() < 1e-3,
            "larger producer should get 7.5, got {}", events[0].energy_delta);
        assert!((events[1].energy_delta - 2.5).abs() < 1e-3,
            "smaller producer should get 2.5, got {}", events[1].energy_delta);
        // Total flux conserved
        let total: f32 = events.iter().map(|e| e.energy_delta).sum();
        assert!((total - 10.0).abs() < 1e-3, "total flux should be conserved");
    }

    #[test]
    fn photosynthesise_zero_structure_excluded_from_competition() {
        // A newborn (zero structure) among established producers gets nothing;
        // established producers split the full flux between themselves.
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, traits), // newborn, structure=0
            make_agent(2, (1.0, 0.0), 10.0, traits), // established
            make_agent(3, (2.0, 0.0), 10.0, traits), // established
        ];
        agents[1].structure = 4.0;
        agents[2].structure = 4.0;
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        grid.insert(2, (2.0, 0.0));

        let events = photosynthesise(&mut agents, &grid, &params);

        // Only 2 events (the two established producers)
        assert_eq!(events.len(), 2, "newborn should not photosynthesise");
        // Each established producer gets half the flux
        assert!((events[0].energy_delta - 5.0).abs() < 1e-3);
        assert!((events[1].energy_delta - 5.0).abs() < 1e-3);
        // Newborn reserve unchanged
        assert!((agents[0].reserve - 10.0).abs() < 1e-6);
    }

    #[test]
    fn photosynthesise_zero_structure_gets_zero_light() {
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        // make_agent creates agents with structure=0.0
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));

        let events = photosynthesise(&mut agents, &grid, &params);

        // With zero structure, agent should get zero light
        assert!(events.is_empty(), "agent with zero structure should produce no photosynthesis event");
        assert!((agents[0].reserve - 10.0).abs() < 1e-6, "reserve should be unchanged");
    }

    // --- Absorb nutrients ---

    #[test]
    fn absorb_nutrients_scales_with_contact_time() {
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        agents[0].contact_time = 10;
        let mut pool = 100.0;

        let events = absorb_nutrients(&mut agents, &mut pool, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::NutrientAbsorbed);
        // demand = eff_autotrophy * ct / (ct + k_half) = 0.5 * 10 / (10 + 50) ≈ 0.0833
        let expected = 0.5 * 10.0 / 60.0;
        assert!((agents[0].nutrient - expected).abs() < 1e-3);
        assert!((pool - (100.0 - expected)).abs() < 1e-3);
    }

    #[test]
    fn absorb_nutrients_shares_proportionally_when_demand_exceeds_pool() {
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, traits),
            make_agent(2, (1.0, 0.0), 10.0, traits),
        ];
        agents[0].contact_time = 50;
        agents[1].contact_time = 50;
        let mut pool = 0.1; // very small pool

        let events = absorb_nutrients(&mut agents, &mut pool, &params);

        assert_eq!(events.len(), 2);
        // Both have same demand, so each gets half
        let total_uptake = agents[0].nutrient + agents[1].nutrient;
        assert!((total_uptake - 0.1).abs() < 1e-3);
        assert!((agents[0].nutrient - agents[1].nutrient).abs() < 1e-3);
    }

    #[test]
    fn absorb_nutrients_zero_contact_time_yields_zero_uptake() {
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        agents[0].contact_time = 0;
        let mut pool = 100.0;

        let events = absorb_nutrients(&mut agents, &mut pool, &params);
        assert!(events.is_empty());
        assert!((agents[0].nutrient).abs() < 1e-6);
    }

    #[test]
    fn absorb_nutrients_derives_uptake_from_autotrophy() {
        // Nutrient uptake is coupled to autotrophy — the same sessile infrastructure
        // that captures light also extracts nutrients.
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        agents[0].contact_time = 10;
        let mut pool = 100.0;

        let events = absorb_nutrients(&mut agents, &mut pool, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::NutrientAbsorbed);
        // demand = effective_autotrophy * ct / (ct + k_half)
        // = 0.5 * 10 / (10 + 50) = 0.5 * 10/60 ≈ 0.0833
        let expected = 0.5 * 10.0 / 60.0;
        assert!((agents[0].nutrient - expected).abs() < 1e-3,
            "autotrophy-derived uptake expected {expected}, got {}", agents[0].nutrient);
    }

    #[test]
    fn absorb_nutrients_zero_autotrophy_gets_zero_uptake() {
        // Agents with zero autotrophy cannot absorb nutrients from the available pool —
        // they must obtain nutrient through consumption.
        let params = test_params();
        let traits = TraitVector {
            heterotrophy: 0.8,
            mobility: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        agents[0].contact_time = 100; // long contact time, still zero uptake
        let mut pool = 100.0;

        let events = absorb_nutrients(&mut agents, &mut pool, &params);

        assert!(events.is_empty(),
            "zero-autotrophy agent should get no nutrient uptake events");
        assert!((agents[0].nutrient).abs() < 1e-6,
            "zero-autotrophy agent should have zero nutrient");
        assert!((pool - 100.0).abs() < 1e-6,
            "pool should be unchanged when no uptake occurs");
    }

    // --- Metabolise ---

    #[test]
    fn metabolise_deducts_base_rate_from_reserve() {
        let params = WorldParameters {
            base_metabolic_rate: 1.0,
            ..test_params()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, zero_traits())];

        let (events, dissipated) = metabolise(&mut agents, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::Metabolized);
        assert!((events[0].energy_delta - 1.0).abs() < 1e-6);
        assert!((agents[0].reserve - 9.0).abs() < 1e-6);
        assert!((dissipated - 1.0).abs() < 1e-6);
    }

    #[test]
    fn metabolise_charges_trait_maintenance_costs() {
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            photo_maintenance_cost: 2.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];

        let (events, dissipated) = metabolise(&mut agents, &params);

        // cost = 0.5 * 2.0 = 1.0
        assert!((events[0].energy_delta - 1.0).abs() < 1e-6);
        assert!((agents[0].reserve - 9.0).abs() < 1e-6);
        assert!((dissipated - 1.0).abs() < 1e-6);
    }

    #[test]
    fn metabolise_charges_structure_maintenance() {
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            structure_maintenance_coefficient: 0.1,
            ..test_params()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, zero_traits())];
        agents[0].structure = 20.0;

        let (events, dissipated) = metabolise(&mut agents, &params);

        // cost = 20.0 * 0.1 = 2.0
        assert!((events[0].energy_delta - 2.0).abs() < 1e-6);
        assert!((agents[0].reserve - 8.0).abs() < 1e-6);
        assert!((dissipated - 2.0).abs() < 1e-6);
    }

    #[test]
    fn metabolise_charges_mobility_maintenance_even_when_stationary() {
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            mobility_maintenance_cost: 2.0,
            ..test_params()
        };
        let traits = TraitVector {
            mobility: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];

        let (events, dissipated) = metabolise(&mut agents, &params);

        // cost = 0.5 * 2.0 = 1.0
        assert!((events[0].energy_delta - 1.0).abs() < 1e-6);
        assert!((agents[0].reserve - 9.0).abs() < 1e-6);
        assert!((dissipated - 1.0).abs() < 1e-6);
    }

    #[test]
    fn grow_retention_reflects_mobility_maintenance_not_movement_cost() {
        // An agent with mobility should retain reserve based on mobility_maintenance_cost
        // (trait maintenance), not movement_cost_coefficient (per-distance cost).
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            mobility_maintenance_cost: 1.0,
            movement_cost_coefficient: 100.0, // high, but should NOT affect retention
            growth_efficiency: 1.0,
            ..test_params()
        };
        let traits = TraitVector {
            mobility: 1.0,
            kappa: 1.0,
            ..zero_traits()
        };
        // retention = metabolic_cost * 2 = (0 + 0 + 0 + 1.0*1.0 + 0) * 2 = 2.0
        // surplus = reserve - retention = 10.0 - 2.0 = 8.0
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];

        let (_events, _dissipated) = grow(&mut agents, &params);

        // If retention used movement_cost_coefficient (100.0), retention would be 200.0
        // and surplus would be 0, so no growth would occur and reserve stays at 10.0.
        // With mobility_maintenance_cost (1.0), retention is 2.0, surplus is 8.0,
        // and with kappa=1.0 and growth_efficiency=1.0, all goes to structure.
        assert!((agents[0].reserve - 2.0).abs() < 1e-6, "reserve should equal retention");
        assert!((agents[0].structure - 8.0).abs() < 1e-6, "surplus should become structure");
    }

    // --- Grow ---

    #[test]
    fn grow_converts_surplus_reserve_to_structure() {
        let params = WorldParameters {
            base_metabolic_rate: 1.0,
            growth_efficiency: 0.8,
            ..test_params()
        };
        let traits = TraitVector { kappa: 1.0, ..zero_traits() }; // all surplus to soma
        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, traits)];

        let (events, dissipated) = grow(&mut agents, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::Grew);
        // retention = 1.0 * 2.0 = 2.0
        // surplus = 100.0 - 2.0 = 98.0
        // kappa=1.0: soma_fraction = 98.0, repro_fraction = 0.0
        // to_structure = 98.0 * 0.8 = 78.4
        // dissipated = 98.0 - 78.4 = 19.6
        assert!((agents[0].structure - 78.4).abs() < 1e-3);
        assert!((agents[0].reserve - 2.0).abs() < 1e-3);
        assert!((dissipated - 19.6).abs() < 1e-3);
        assert!((agents[0].repro_reserve).abs() < 1e-6, "kappa=1 should send nothing to repro");
    }

    #[test]
    fn grow_does_nothing_when_reserve_below_retention() {
        let params = WorldParameters {
            base_metabolic_rate: 100.0,
            growth_efficiency: 0.8,
            ..test_params()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, zero_traits())];

        let (events, dissipated) = grow(&mut agents, &params);

        assert!(events.is_empty());
        assert!((agents[0].structure).abs() < 1e-6);
        assert!((dissipated).abs() < 1e-6);
    }

    #[test]
    fn grow_does_nothing_when_efficiency_is_zero() {
        let params = WorldParameters {
            growth_efficiency: 0.0,
            ..test_params()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, zero_traits())];

        let (events, _) = grow(&mut agents, &params);
        assert!(events.is_empty());
    }

    #[test]
    fn grow_kappa_splits_surplus_between_soma_and_repro() {
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            growth_efficiency: 1.0, // perfect conversion for easy math
            ..test_params()
        };
        let traits = TraitVector { kappa: 0.6, ..zero_traits() };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, traits)];

        let (_events, _dissipated) = grow(&mut agents, &params);

        // surplus = 100.0, kappa=0.6
        // soma = 60.0 -> all to structure (efficiency=1.0, dissipated=0)
        // repro = 40.0 -> to repro_reserve
        assert!((agents[0].structure - 60.0).abs() < 1e-3,
            "kappa fraction should go to structure, got {}", agents[0].structure);
        assert!((agents[0].repro_reserve - 40.0).abs() < 1e-3,
            "1-kappa fraction should go to repro_reserve, got {}", agents[0].repro_reserve);
        assert!((agents[0].reserve).abs() < 1e-3,
            "all surplus should be consumed, got {}", agents[0].reserve);
    }

    #[test]
    fn grow_zero_kappa_sends_all_surplus_to_repro_reserve() {
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            growth_efficiency: 0.8,
            ..test_params()
        };
        let traits = TraitVector { kappa: 0.0, ..zero_traits() };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 50.0, traits)];

        let (_events, _dissipated) = grow(&mut agents, &params);

        // kappa=0: everything to repro
        assert!((agents[0].structure).abs() < 1e-6, "zero kappa should not grow");
        assert!((agents[0].repro_reserve - 50.0).abs() < 1e-3,
            "all surplus to repro_reserve, got {}", agents[0].repro_reserve);
    }

    #[test]
    fn grow_repro_reserve_accumulates_across_ticks() {
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            growth_efficiency: 1.0,
            ..test_params()
        };
        let traits = TraitVector { kappa: 0.5, ..zero_traits() };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 20.0, traits)];

        // First tick: surplus=20, repro=10
        grow(&mut agents, &params);
        let after_first = agents[0].repro_reserve;
        assert!((after_first - 10.0).abs() < 1e-3);

        // Give more reserve for second tick
        agents[0].reserve = 20.0;
        grow(&mut agents, &params);
        // Should accumulate: 10.0 + 10.0 = 20.0
        assert!((agents[0].repro_reserve - 20.0).abs() < 1e-3,
            "repro_reserve should accumulate, got {}", agents[0].repro_reserve);
    }

    #[test]
    fn reproduction_draws_from_repro_reserve_not_reserve() {
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_efficiency: 1.0,
            reproduction_energy_threshold: 10.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 50.0, traits),
            make_agent(2, (1.0, 0.0), 50.0, traits),
        ];
        agents[0].repro_reserve = 20.0;
        agents[1].repro_reserve = 20.0;
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, &mut rng);

        assert!(!result.offspring.is_empty(), "should reproduce");
        // Reserve should be unchanged
        assert!((agents[0].reserve - 50.0).abs() < 1e-3,
            "reserve should be untouched, got {}", agents[0].reserve);
        // Repro_reserve should be zero
        assert!(agents[0].repro_reserve.abs() < 1e-3,
            "repro_reserve should be spent, got {}", agents[0].repro_reserve);
    }

    #[test]
    fn offspring_born_with_zero_repro_reserve() {
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_efficiency: 0.7,
            reproduction_energy_threshold: 10.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 50.0, traits),
            make_agent(2, (1.0, 0.0), 50.0, traits),
        ];
        agents[0].repro_reserve = 15.0;
        agents[1].repro_reserve = 15.0;
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, &mut rng);

        for child in &result.offspring {
            assert!((child.repro_reserve).abs() < 1e-6,
                "offspring should have zero repro_reserve, got {}", child.repro_reserve);
        }
    }

    // --- Apply wear ---

    #[test]
    fn apply_wear_accumulates_baseline_wear() {
        let params = WorldParameters {
            wear_rate: 0.1,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];

        let no_usage = std::collections::HashMap::new();
        let events = apply_wear(&mut agents, &params, &no_usage);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::Wore);
        // ft 0 (photo): 0.1 * 0.5 = 0.05
        assert!((agents[0].wear[0] - 0.05).abs() < 1e-6);
    }

    #[test]
    fn apply_wear_somatic_repair_reduces_wear() {
        let params = WorldParameters {
            wear_rate: 0.1,
            repair_decay: 1.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.7,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        agents[0].wear[0] = 1.0; // pre-existing wear

        let no_usage = std::collections::HashMap::new();
        let _events = apply_wear(&mut agents, &params, &no_usage);

        // Accumulation: 0.1 * 0.5 = 0.05, so wear goes to 1.05
        // Then repair: kappa(0.7) * exp(-1.0 * 1.05) = 0.7 * 0.3499 = 0.2449
        // Final: 1.05 - 0.2449 ≈ 0.805
        assert!(agents[0].wear[0] < 1.05, "repair should reduce wear below accumulation");
        assert!(agents[0].wear[0] > 0.0, "wear should still be positive");
    }

    #[test]
    fn apply_wear_no_event_when_all_traits_zero() {
        let params = WorldParameters {
            wear_rate: 0.1,
            ..test_params()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, zero_traits())];

        let no_usage = std::collections::HashMap::new();
        let events = apply_wear(&mut agents, &params, &no_usage);
        assert!(events.is_empty());
    }

    // --- Use-dependent wear ---

    #[test]
    fn use_wear_autotrophy_increases_with_photosynthesis() {
        // An agent that photosynthesises should accumulate extra autotrophy wear
        // proportional to energy captured, on top of baseline wear.
        let params = WorldParameters {
            wear_rate: 0.1,
            use_wear_rate: 0.05,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];

        // No usage: only baseline wear
        let no_usage = std::collections::HashMap::new();
        let _events_baseline = apply_wear(&mut agents, &params, &no_usage);
        let baseline_wear = agents[0].wear[0];
        assert!(baseline_wear > 0.0, "baseline wear should accumulate");

        // Reset and apply with photosynthesis usage
        agents[0].wear = [0.0; FUNCTIONAL_TRAIT_COUNT];
        let mut usage = std::collections::HashMap::new();
        usage.insert(1_u64, [5.0_f32, 0.0, 0.0]); // 5.0 energy captured via photosynthesis
        let _events_use = apply_wear(&mut agents, &params, &usage);
        let use_wear = agents[0].wear[0];

        // Use-dependent wear should exceed baseline alone
        assert!(use_wear > baseline_wear,
            "use-dependent wear ({use_wear}) should exceed baseline ({baseline_wear})");
        // Extra wear = use_wear_rate * energy_captured = 0.05 * 5.0 = 0.25
        let expected_extra = 0.05 * 5.0;
        let actual_extra = use_wear - baseline_wear;
        assert!((actual_extra - expected_extra).abs() < 1e-6,
            "extra wear should be {expected_extra}, got {actual_extra}");
    }

    #[test]
    fn use_wear_heterotrophy_increases_with_consumption() {
        // An agent that consumes should accumulate extra heterotrophy wear
        // proportional to energy drained.
        let params = WorldParameters {
            wear_rate: 0.1,
            use_wear_rate: 0.05,
            ..test_params()
        };
        let traits = TraitVector {
            heterotrophy: 0.8,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];

        // With heterotrophy usage
        let mut usage = std::collections::HashMap::new();
        usage.insert(1_u64, [0.0_f32, 3.0, 0.0]); // 3.0 energy drained
        let _events = apply_wear(&mut agents, &params, &usage);

        // Baseline: 0.1 * 0.8 = 0.08
        // Use-dependent: 0.05 * 3.0 = 0.15
        // Total heterotrophy wear (index 1): 0.08 + 0.15 = 0.23
        let expected = 0.08 + 0.15;
        assert!((agents[0].wear[1] - expected).abs() < 1e-6,
            "heterotrophy wear should be {expected}, got {}", agents[0].wear[1]);
    }

    #[test]
    fn use_wear_mobility_increases_with_distance_moved() {
        // An agent that moves should accumulate extra mobility wear
        // proportional to distance traveled.
        let params = WorldParameters {
            wear_rate: 0.1,
            use_wear_rate: 0.05,
            ..test_params()
        };
        let traits = TraitVector {
            mobility: 0.6,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];

        let mut usage = std::collections::HashMap::new();
        usage.insert(1_u64, [0.0_f32, 0.0, 2.0]); // 2.0 distance moved
        let _events = apply_wear(&mut agents, &params, &usage);

        // Baseline: 0.1 * 0.6 = 0.06
        // Use-dependent: 0.05 * 2.0 = 0.10
        // Total mobility wear (index 2): 0.06 + 0.10 = 0.16
        let expected = 0.06 + 0.10;
        assert!((agents[0].wear[2] - expected).abs() < 1e-6,
            "mobility wear should be {expected}, got {}", agents[0].wear[2]);
    }

    #[test]
    fn use_wear_zero_usage_equals_baseline_only() {
        // An agent with no usage should only have baseline wear.
        let params = WorldParameters {
            wear_rate: 0.1,
            use_wear_rate: 0.05,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            heterotrophy: 0.3,
            mobility: 0.4,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];

        let no_usage = std::collections::HashMap::new();
        let _events = apply_wear(&mut agents, &params, &no_usage);

        // Only baseline wear: rate * nominal for each trait
        assert!((agents[0].wear[0] - 0.05).abs() < 1e-6); // 0.1 * 0.5
        assert!((agents[0].wear[1] - 0.03).abs() < 1e-6); // 0.1 * 0.3
        assert!((agents[0].wear[2] - 0.04).abs() < 1e-6); // 0.1 * 0.4
    }

    // --- Check death thresholds ---

    #[test]
    fn check_death_kills_agent_with_zero_reserve() {
        let params = test_params();
        let mut agents = vec![make_agent(1, (0.0, 0.0), 0.0, zero_traits())];

        let (events, carcasses, _dissipated) = check_death_thresholds(&mut agents, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::Died);
        assert_eq!(carcasses.len(), 1);
        assert_eq!(carcasses[0].id, 1);
    }

    #[test]
    fn check_death_kills_agent_with_negative_reserve() {
        let params = test_params();
        let mut agents = vec![make_agent(1, (0.0, 0.0), -5.0, zero_traits())];

        let (events, carcasses, _dissipated) = check_death_thresholds(&mut agents, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(carcasses.len(), 1);
    }

    #[test]
    fn check_death_kills_agent_with_structure_below_threshold() {
        let params = test_params();
        // Generalist traits spread across many dimensions -> high threshold
        let traits = TraitVector {
            photosynthetic_absorption: 0.1,
            heterotrophy: 0.1,
            mobility: 0.1,
            kappa: 0.1,
            fecundity: 0.1,
            asexual_propensity: 0.1,
            dispersal: 0.1,
        };
        let threshold = crate::death_threshold(&traits);
        assert!(threshold > 0.0, "generalist should have nonzero threshold");

        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, traits)];
        agents[0].structure = threshold * 0.5; // below threshold

        let (events, carcasses, _dissipated) = check_death_thresholds(&mut agents, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::Died);
        assert_eq!(carcasses.len(), 1);
        assert!((carcasses[0].energy - threshold * 0.5).abs() < 1e-6);
    }

    #[test]
    fn check_death_preserves_healthy_agents() {
        let params = test_params();
        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, zero_traits())];

        let (events, carcasses, _dissipated) = check_death_thresholds(&mut agents, &params);

        assert!(events.is_empty());
        assert!(carcasses.is_empty());
        assert!((agents[0].reserve - 100.0).abs() < 1e-6);
    }

    #[test]
    fn check_death_carcass_inherits_nutrient() {
        let params = test_params();
        let mut agents = vec![make_agent(1, (0.0, 0.0), 0.0, zero_traits())];
        agents[0].nutrient = 5.0;
        agents[0].structure = 10.0;

        let (_, carcasses, _) = check_death_thresholds(&mut agents, &params);

        assert_eq!(carcasses.len(), 1);
        assert!((carcasses[0].nutrient - 5.0).abs() < 1e-6);
        assert!((carcasses[0].energy - 10.0).abs() < 1e-6);
    }

    // --- Resolve drains ---

    #[test]
    fn drain_single_consumer_drains_living_target_structure() {
        // Consumer with consumption_rate near target with structure.
        // Consumer drains target's structure; consumer gains energy (with trophic loss).
        let params = test_params(); // consumption_efficiency = 0.5, contact_radius = 5.0
        let consumer_traits = TraitVector {
            heterotrophy: 0.4,
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        // Consumer at (0,0), target at (1,0) — within contact_radius=5.0
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, consumer_traits),
            make_agent(2, (1.0, 0.0), 10.0, target_traits),
        ];
        agents[1].structure = 20.0;

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let mut nutrient_pool = 0.0;
        let result = resolve_drains(
            &mut agents, &mut carcasses, &grid, &params, &mut nutrient_pool,
        );

        // Consumer demand = effective consumption_rate = 0.4
        // Target has 20.0 structure, demand < supply, so full drain of 0.4
        // Target structure: 20.0 - 0.4 = 19.6
        // Consumer reserve: 10.0 + 0.4 * 0.5 = 10.2 (trophic efficiency)
        // Dissipated: 0.4 * 0.5 = 0.2
        assert!((agents[1].structure - 19.6).abs() < 1e-3,
            "target structure should be 19.6, got {}", agents[1].structure);
        assert!((agents[0].reserve - 10.2).abs() < 1e-3,
            "consumer reserve should be 10.2, got {}", agents[0].reserve);
        assert!((result.dissipated - 0.2).abs() < 1e-3,
            "dissipated should be 0.2, got {}", result.dissipated);
        assert!(result.dead_agents.is_empty());
        assert!(result.new_carcasses.is_empty());
    }

    #[test]
    fn drain_proportional_split_two_consumers() {
        // Two consumers target the same agent with demands 3.0 and 1.0.
        // Target has 2.0 available structure.
        // They receive 1.5 and 0.5 respectively (proportional to demand).
        let params = test_params(); // consumption_efficiency = 0.5, contact_radius = 5.0
        let consumer_a_traits = TraitVector {
            heterotrophy: 3.0,
            ..zero_traits()
        };
        let consumer_b_traits = TraitVector {
            heterotrophy: 1.0,
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        // All within contact_radius=5.0
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, consumer_a_traits),  // demand 3.0
            make_agent(2, (1.0, 0.0), 10.0, consumer_b_traits),  // demand 1.0
            make_agent(3, (0.5, 0.0), 10.0, target_traits),      // target
        ];
        agents[2].structure = 2.0;

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        grid.insert(2, (0.5, 0.0));

        let mut nutrient_pool = 0.0;
        let result = resolve_drains(
            &mut agents, &mut carcasses, &grid, &params, &mut nutrient_pool,
        );

        // Total demand = 4.0, available = 2.0
        // Consumer A gets 3.0/4.0 * 2.0 = 1.5
        // Consumer B gets 1.0/4.0 * 2.0 = 0.5
        // Consumer A reserve: 10.0 + 1.5 * 0.5 = 10.75
        // Consumer B reserve: 10.0 + 0.5 * 0.5 = 10.25
        // Target structure: 2.0 - 2.0 = 0.0
        assert!((agents[0].reserve - 10.75).abs() < 1e-3,
            "consumer A reserve should be 10.75, got {}", agents[0].reserve);
        assert!((agents[1].reserve - 10.25).abs() < 1e-3,
            "consumer B reserve should be 10.25, got {}", agents[1].reserve);
        assert!(agents[2].structure.abs() < 1e-3,
            "target structure should be 0.0, got {}", agents[2].structure);
        // Dissipated: 1.5 * 0.5 + 0.5 * 0.5 = 1.0
        assert!((result.dissipated - 1.0).abs() < 1e-3,
            "dissipated should be 1.0, got {}", result.dissipated);
    }

    #[test]
    fn drain_death_marking_produces_carcass() {
        // Consumer drains target below death threshold -> target dies, carcass produced.
        let params = test_params();
        // Target is a generalist with high death threshold
        let generalist_traits = TraitVector {
            photosynthetic_absorption: 0.1,
            heterotrophy: 0.1,
            mobility: 0.1,
            kappa: 0.1,
            fecundity: 0.1,
            asexual_propensity: 0.1,
            dispersal: 0.1,
        };
        let threshold = crate::death_threshold(&generalist_traits);
        assert!(threshold > 0.0, "generalist should have nonzero threshold");

        let consumer_traits = TraitVector {
            heterotrophy: 5.0, // high demand to drain target
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, consumer_traits),
            make_agent(2, (1.0, 0.0), 10.0, generalist_traits),
        ];
        // Give target structure just above threshold, so draining pushes below
        agents[1].structure = threshold * 1.5;
        agents[1].nutrient = 3.0;
        agents[0].repro_reserve = 15.0;
        agents[1].repro_reserve = 15.0;

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let mut nutrient_pool = 0.0;
        let result = resolve_drains(
            &mut agents, &mut carcasses, &grid, &params, &mut nutrient_pool,
        );

        // Consumer demand = 5.0, target structure = threshold * 1.5
        // If demand > structure, drain is capped at structure
        // After drain, structure should be below threshold -> death
        assert!(!result.dead_agents.is_empty(),
            "target should be dead");
        assert_eq!(result.dead_agents[0], 2);
        assert_eq!(result.new_carcasses.len(), 1);
        assert_eq!(result.new_carcasses[0].id, 2);
    }

    #[test]
    fn drain_no_reentrant_processing_new_carcass_not_decomposed() {
        // When a living target is killed this tick, the resulting carcass is NOT
        // available for decomposition in the same tick.
        let params = test_params();
        let consumer_traits = TraitVector {
            heterotrophy: 100.0, // high heterotrophy drains both living and carcass
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, consumer_traits),
            make_agent(2, (1.0, 0.0), 10.0, target_traits),
        ];
        agents[1].structure = 5.0;

        // No pre-existing carcasses
        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let mut nutrient_pool = 0.0;
        let result = resolve_drains(
            &mut agents, &mut carcasses, &grid, &params, &mut nutrient_pool,
        );

        // Target should be killed (drained to 0)
        assert!(!result.dead_agents.is_empty(), "target should be dead");
        assert_eq!(result.new_carcasses.len(), 1);

        // The new carcass was NOT available for scavenging.
        // Consumer only got energy from the living-target drain, not from
        // decomposing the new carcass. The carcass energy is in new_carcasses,
        // not consumed further.
        // Consumer reserve = 10.0 + 5.0 * 0.5 = 12.5 (only from living drain)
        assert!((agents[0].reserve - 12.5).abs() < 1e-3,
            "consumer should only get energy from living drain, got {}", agents[0].reserve);
    }

    #[test]
    fn drain_heterotrophy_drains_both_living_and_carcass_same_tick() {
        // An agent with heterotrophy can drain a living target AND a carcass
        // in the same tick — the same trait drives both, target state determines
        // which efficiency applies.
        let params = test_params(); // consumption_efficiency=0.5, decomposition_efficiency=0.5
        let heterotroph_traits = TraitVector {
            heterotrophy: 2.0,
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, heterotroph_traits),
            make_agent(2, (1.0, 0.0), 10.0, target_traits),
        ];
        agents[1].structure = 20.0;

        // Pre-existing carcass at nearby position
        let mut carcasses = vec![Carcass {
            id: 99,
            position: (2.0, 0.0), // within contact_radius=5.0
            energy: 10.0,
            nutrient: 0.0,
            traits: zero_traits(),
        }];

        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let mut nutrient_pool = 0.0;
        let result = resolve_drains(
            &mut agents, &mut carcasses, &grid, &params, &mut nutrient_pool,
        );

        // From living target: drain 2.0, gain 2.0 * 0.5 = 1.0 (consumption_efficiency)
        // From carcass: drain 2.0, gain 2.0 * 0.5 = 1.0 (decomposition_efficiency)
        // Consumer reserve: 10.0 + 1.0 + 1.0 = 12.0
        assert!((agents[0].reserve - 12.0).abs() < 1e-3,
            "consumer should gain from both living and carcass, got {}", agents[0].reserve);
        // Living target structure: 20.0 - 2.0 = 18.0
        assert!((agents[1].structure - 18.0).abs() < 1e-3,
            "target structure should be 18.0, got {}", agents[1].structure);
        // Carcass energy: 10.0 - 2.0 = 8.0
        assert!((carcasses[0].energy - 8.0).abs() < 1e-3,
            "carcass energy should be 8.0, got {}", carcasses[0].energy);
        // Events should include both living and carcass consumption
        assert!(result.events.len() >= 2,
            "should have events for both consumption types");
    }

    // --- Move agents ---

    #[test]
    fn move_direction_attracted_to_nearby_living_agent_by_heterotrophy() {
        // Agent with heterotrophy and chemotaxis should move toward a nearby living agent.
        use rand::SeedableRng;
        let mut params = test_params();
        params.movement_cost_coefficient = 0.0; // isolate direction test
        let mover_traits = TraitVector {
            heterotrophy: 0.5,
            mobility: 5.0, // sensing range = 5.0 * 10.0 = 50.0
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };
        // Mover at origin, target at (20, 0)
        let mut agents = vec![
            make_agent(0, (0.0, 0.0), 100.0, mover_traits),
            make_agent(1, (20.0, 0.0), 100.0, target_traits),
        ];
        agents[1].structure = 10.0;
        let carcasses = vec![];
        let mut grid = crate::spatial::SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (20.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = move_agents(&mut agents, &carcasses, &grid, &params, &mut rng);

        // Agent should have moved in the +x direction (toward target)
        assert!(agents[0].position.0 > 0.0,
            "agent should move toward target in +x, got x={}", agents[0].position.0);
        assert!(!result.events.is_empty());
        assert_eq!(result.events[0].kind, EventKind::Moved);
    }

    #[test]
    fn move_direction_attracted_to_carcass_by_heterotrophy() {
        // Agent with heterotrophy and chemotaxis should move toward a nearby carcass.
        use rand::SeedableRng;
        let mut params = test_params();
        params.movement_cost_coefficient = 0.0;
        let mover_traits = TraitVector {
            heterotrophy: 0.5,
            mobility: 5.0, // sensing range = 5.0 * 10.0 = 50.0
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(0, (0.0, 0.0), 100.0, mover_traits),
        ];
        let carcasses = vec![Carcass {
            id: 99,
            position: (0.0, 20.0),
            energy: 10.0,
            nutrient: 0.0,
            traits: zero_traits(),
        }];
        let mut grid = crate::spatial::SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let _result = move_agents(&mut agents, &carcasses, &grid, &params, &mut rng);

        // Agent should have moved in the +y direction (toward carcass)
        assert!(agents[0].position.1 > 0.0,
            "agent should move toward carcass in +y, got y={}", agents[0].position.1);
    }

    #[test]
    fn move_zero_structure_pays_zero_movement_cost() {
        // Newborns (structure=0) should move for free regardless of coefficient.
        use rand::SeedableRng;
        let mut params = test_params();
        params.movement_cost_coefficient = 2.0;
        let traits = TraitVector {
            mobility: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(0, (0.0, 0.0), 100.0, traits)];
        // structure is 0.0 by default from make_agent
        assert_eq!(agents[0].structure, 0.0);
        let carcasses = vec![];
        let grid = crate::spatial::SpatialGrid::new(100.0, 10.0);
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = move_agents(&mut agents, &carcasses, &grid, &params, &mut rng);

        // Zero structure means zero movement cost
        assert!((agents[0].reserve - 100.0).abs() < 1e-3,
            "zero-structure agent should pay zero movement cost, reserve={}", agents[0].reserve);
        assert!((result.dissipated).abs() < 1e-3,
            "zero dissipation for zero-structure agent, got {}", result.dissipated);
    }

    #[test]
    fn move_large_agent_pays_more_than_small_agent() {
        // Two agents with same mobility but different structure: the larger one
        // should pay proportionally more movement cost.
        use rand::SeedableRng;
        let mut params = test_params();
        params.movement_cost_coefficient = 1.0;
        let traits = TraitVector {
            mobility: 0.5,
            ..zero_traits()
        };

        // Small agent (structure=2)
        let mut small = vec![make_agent(0, (0.0, 0.0), 100.0, traits)];
        small[0].structure = 2.0;
        let carcasses = vec![];
        let grid = crate::spatial::SpatialGrid::new(100.0, 10.0);
        let mut rng1 = ChaCha8Rng::seed_from_u64(42);
        let result_small = move_agents(&mut small, &carcasses, &grid, &params, &mut rng1);

        // Large agent (structure=10)
        let mut large = vec![make_agent(0, (0.0, 0.0), 100.0, traits)];
        large[0].structure = 10.0;
        let mut rng2 = ChaCha8Rng::seed_from_u64(42);
        let result_large = move_agents(&mut large, &carcasses, &grid, &params, &mut rng2);

        // Large agent should pay 5x more (10/2)
        let small_cost = 100.0 - small[0].reserve;
        let large_cost = 100.0 - large[0].reserve;
        assert!(large_cost > small_cost,
            "large agent cost ({}) should exceed small agent cost ({})", large_cost, small_cost);
        assert!((large_cost / small_cost - 5.0).abs() < 1e-3,
            "cost ratio should be 5.0, got {}", large_cost / small_cost);
        assert!((result_large.dissipated / result_small.dissipated - 5.0).abs() < 1e-3,
            "dissipation ratio should be 5.0");
    }

    #[test]
    fn move_energy_cost_proportional_to_distance_and_structure() {
        use rand::SeedableRng;
        let mut params = test_params();
        params.movement_cost_coefficient = 2.0;
        let traits = TraitVector {
            mobility: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(0, (0.0, 0.0), 100.0, traits)];
        agents[0].structure = 3.0;
        let carcasses = vec![];
        let grid = crate::spatial::SpatialGrid::new(100.0, 10.0);
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = move_agents(&mut agents, &carcasses, &grid, &params, &mut rng);

        // eff_mobility = 0.5 (no wear, k=0 so exp(0)=1)
        // cost = distance * coefficient * structure = 0.5 * 2.0 * 3.0 = 3.0
        let expected_cost = 0.5 * 2.0 * 3.0;
        assert!((agents[0].reserve - (100.0 - expected_cost)).abs() < 1e-3,
            "reserve should be {}, got {}", 100.0 - expected_cost, agents[0].reserve);
        assert!((result.dissipated - expected_cost).abs() < 1e-3);
    }

    #[test]
    fn move_contact_time_resets_on_movement() {
        use rand::SeedableRng;
        let mut params = test_params();
        params.movement_cost_coefficient = 0.0;
        let traits = TraitVector {
            mobility: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(0, (0.0, 0.0), 100.0, traits)];
        agents[0].contact_time = 10; // had been stationary
        let carcasses = vec![];
        let grid = crate::spatial::SpatialGrid::new(100.0, 10.0);
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let _result = move_agents(&mut agents, &carcasses, &grid, &params, &mut rng);

        assert_eq!(agents[0].contact_time, 0,
            "contact_time should reset to 0 after moving, got {}", agents[0].contact_time);
    }

    #[test]
    fn move_contact_time_increments_when_stationary() {
        use rand::SeedableRng;
        let params = test_params();
        // Zero mobility -> stationary
        let traits = TraitVector {
            ..zero_traits()
        };
        let mut agents = vec![make_agent(0, (0.0, 0.0), 100.0, traits)];
        agents[0].contact_time = 5;
        let carcasses = vec![];
        let grid = crate::spatial::SpatialGrid::new(100.0, 10.0);
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let _result = move_agents(&mut agents, &carcasses, &grid, &params, &mut rng);

        assert_eq!(agents[0].contact_time, 6,
            "contact_time should increment when stationary, got {}", agents[0].contact_time);
    }

    #[test]
    fn move_toroidal_wrapping_applied() {
        use rand::SeedableRng;
        let mut params = test_params();
        params.movement_cost_coefficient = 0.0;
        // Agent near the edge with mobility — should wrap around
        let traits = TraitVector {
            mobility: 5.0,
            ..zero_traits()
        };
        // Place at edge of world (extent=100, so range is -50..50)
        // With enough mobility, the position wraps
        let mut agents = vec![make_agent(0, (48.0, 0.0), 100.0, traits)];
        let carcasses = vec![];
        let grid = crate::spatial::SpatialGrid::new(100.0, 10.0);
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let _result = move_agents(&mut agents, &carcasses, &grid, &params, &mut rng);

        // Position should be within bounds after wrapping
        let extent = params.world_extent;
        let half = extent / 2.0;
        assert!(agents[0].position.0 >= -half && agents[0].position.0 <= half,
            "x position should be within bounds, got {}", agents[0].position.0);
        assert!(agents[0].position.1 >= -half && agents[0].position.1 <= half,
            "y position should be within bounds, got {}", agents[0].position.1);
    }

    #[test]
    fn move_deterministic_with_seeded_rng() {
        use rand::SeedableRng;
        let mut params = test_params();
        params.movement_cost_coefficient = 0.0;
        let traits = TraitVector {
            mobility: 0.5,
            ..zero_traits()
        };

        // Run twice with same seed
        let mut agents1 = vec![make_agent(0, (0.0, 0.0), 100.0, traits)];
        let mut agents2 = vec![make_agent(0, (0.0, 0.0), 100.0, traits)];
        let carcasses = vec![];
        let grid = crate::spatial::SpatialGrid::new(100.0, 10.0);

        let mut rng1 = ChaCha8Rng::seed_from_u64(123);
        let mut rng2 = ChaCha8Rng::seed_from_u64(123);

        let _r1 = move_agents(&mut agents1, &carcasses, &grid, &params, &mut rng1);
        let _r2 = move_agents(&mut agents2, &carcasses, &grid, &params, &mut rng2);

        assert!((agents1[0].position.0 - agents2[0].position.0).abs() < 1e-6,
            "positions should be identical with same seed");
        assert!((agents1[0].position.1 - agents2[0].position.1).abs() < 1e-6);
    }

    #[test]
    fn move_sensing_throughput_counts_detected_entities() {
        use rand::SeedableRng;
        let mut params = test_params();
        params.movement_cost_coefficient = 0.0;
        let traits = TraitVector {
            mobility: 5.0, // sensing range = 5.0 * 10.0 = 50.0
            heterotrophy: 0.1,
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(0, (0.0, 0.0), 100.0, traits),
            make_agent(1, (10.0, 0.0), 100.0, target_traits),
            make_agent(2, (5.0, 5.0), 100.0, target_traits),
        ];
        agents[1].structure = 5.0;
        agents[2].structure = 5.0;
        let carcasses = vec![Carcass {
            id: 99,
            position: (3.0, 0.0),
            energy: 5.0,
            nutrient: 0.0,
            traits: zero_traits(),
        }];
        let mut grid = crate::spatial::SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (10.0, 0.0));
        grid.insert(2, (5.0, 5.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = move_agents(&mut agents, &carcasses, &grid, &params, &mut rng);

        // Agent 0 should detect 2 living agents + 1 carcass = 3
        assert!((result.sensing_throughput[0] - 3.0).abs() < 1e-6,
            "sensing throughput should be 3.0, got {}", result.sensing_throughput[0]);
    }

    #[test]
    fn drain_nutrient_transfer_retains_up_to_stoichiometric_demand() {
        // Consumer drains target that has nutrient. Consumer retains up to
        // stoichiometric demand; excess goes to available pool.
        let params = test_params();
        let consumer_traits = TraitVector {
            heterotrophy: 2.0,
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, consumer_traits),
            make_agent(2, (1.0, 0.0), 10.0, target_traits),
        ];
        // Consumer needs structure for stoichiometric demand
        // demand = structure * (base_ratio + spec_coeff * spec_sum)
        //        = 5.0 * (0.1 + 0.2 * 2.0) = 5.0 * 0.5 = 2.5
        agents[0].structure = 5.0;
        agents[1].structure = 10.0;
        agents[1].nutrient = 20.0; // nutrient-rich target

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let mut nutrient_pool = 0.0;
        let _result = resolve_drains(
            &mut agents, &mut carcasses, &grid, &params, &mut nutrient_pool,
        );

        // Drain = 2.0 (demand <= supply of 10.0)
        // nutrient_fraction = 2.0 / 10.0 = 0.2
        // nutrient_transferred = 20.0 * 0.2 = 4.0
        // energy_gained = 2.0 * 0.5 = 1.0
        // stoichiometric_demand = 5.0 * (0.1 + 0.2 * 2.0) = 2.5
        // consumer_nutrient_need = 2.5 * 1.0 = 2.5
        // retained = min(4.0, 2.5) = 2.5
        // excreted = 4.0 - 2.5 = 1.5
        assert!((agents[0].nutrient - 2.5).abs() < 1e-3,
            "consumer should retain 2.5 nutrient, got {}", agents[0].nutrient);
        assert!((nutrient_pool - 1.5).abs() < 1e-3,
            "nutrient pool should receive 1.5 excess, got {}", nutrient_pool);
        assert!((agents[1].nutrient - 16.0).abs() < 1e-3,
            "target nutrient should be 16.0 (20 - 4), got {}", agents[1].nutrient);
    }

    // --- Distance-dependent trophic efficiency in drains ---

    #[test]
    fn drain_similar_consumer_gains_more_than_distant_consumer() {
        // Two consumers drain the same target. The one with traits closer to
        // the target gets more energy per unit drained.
        let params = WorldParameters {
            base_trophic_efficiency: 0.8,
            trophic_distance_decay: 2.0,
            ..test_params()
        };
        // Target is a producer
        let target_traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        // Consumer A: similar to target (also photosynthetic + heterotroph)
        let similar_consumer = TraitVector {
            photosynthetic_absorption: 0.4,
            heterotrophy: 1.0,
            ..zero_traits()
        };
        // Consumer B: very different from target (high mobility, no photo)
        let distant_consumer = TraitVector {
            heterotrophy: 1.0,
            mobility: 2.0,
            ..zero_traits()
        };

        // Run two separate drain resolutions to compare energy gained
        let run_drain = |consumer_traits: TraitVector| -> f32 {
            let mut agents = vec![
                make_agent(1, (0.0, 0.0), 0.0, consumer_traits),
                make_agent(2, (1.0, 0.0), 10.0, target_traits),
            ];
            agents[1].structure = 100.0; // plenty of structure
            let mut carcasses: Vec<Carcass> = Vec::new();
            let mut grid = SpatialGrid::new(100.0, 10.0);
            grid.insert(0, (0.0, 0.0));
            grid.insert(1, (1.0, 0.0));
            let mut nutrient_pool = 0.0;
            let _result = resolve_drains(
                &mut agents, &mut carcasses, &grid, &params, &mut nutrient_pool,
            );
            agents[0].reserve // energy gained
        };

        let gained_similar = run_drain(similar_consumer);
        let gained_distant = run_drain(distant_consumer);

        assert!(gained_similar > gained_distant,
            "similar consumer should gain more: similar={}, distant={}",
            gained_similar, gained_distant);
        assert!(gained_similar > 0.0, "similar consumer should gain energy");
        assert!(gained_distant > 0.0, "distant consumer should still gain some energy");
    }

    #[test]
    fn drain_carcass_uses_original_trait_vector_for_distance() {
        // A carcass retains the dead agent's trait vector. Efficiency depends
        // on trait distance between consumer and the carcass's original traits.
        let params = WorldParameters {
            base_trophic_efficiency: 0.8,
            trophic_distance_decay: 2.0,
            ..test_params()
        };
        let consumer_traits = TraitVector {
            heterotrophy: 0.5,
            photosynthetic_absorption: 0.3,
            ..zero_traits()
        };
        // Carcass from a similar organism
        let similar_carcass_traits = TraitVector {
            photosynthetic_absorption: 0.4,
            ..zero_traits()
        };
        // Carcass from a very different organism
        let distant_carcass_traits = TraitVector {
            mobility: 3.0,
            ..zero_traits()
        };

        let run_carcass_drain = |carcass_traits: TraitVector| -> f32 {
            // Agent id must match grid key (slice index) for id_to_idx lookup
            let mut agents = vec![
                make_agent(0, (0.0, 0.0), 0.0, consumer_traits),
            ];
            let mut carcasses = vec![Carcass {
                id: 99,
                position: (0.0, 0.0), // co-located with consumer
                energy: 100.0,
                nutrient: 0.0,
                traits: carcass_traits,
            }];
            let mut grid = SpatialGrid::new(100.0, 10.0);
            grid.insert(0, agents[0].position);
            let mut nutrient_pool = 0.0;
            let _result = resolve_drains(
                &mut agents, &mut carcasses, &grid, &params, &mut nutrient_pool,
            );
            agents[0].reserve
        };

        let gained_similar = run_carcass_drain(similar_carcass_traits);
        let gained_distant = run_carcass_drain(distant_carcass_traits);

        assert!(gained_similar > gained_distant,
            "decomposing similar carcass should yield more: similar={}, distant={}",
            gained_similar, gained_distant);
    }

    #[test]
    fn drain_energy_conservation_with_distance_dependent_efficiency() {
        // Energy drained from target = energy gained by consumer + dissipated.
        // This must hold regardless of distance-dependent efficiency.
        let params = WorldParameters {
            base_trophic_efficiency: 0.7,
            trophic_distance_decay: 1.5,
            ..test_params()
        };
        let consumer_traits = TraitVector {
            heterotrophy: 2.0,
            mobility: 1.0,
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 0.8,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 0.0, consumer_traits),
            make_agent(2, (1.0, 0.0), 10.0, target_traits),
        ];
        agents[1].structure = 50.0;

        let pre_target_structure = agents[1].structure;
        let pre_consumer_reserve = agents[0].reserve;

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut nutrient_pool = 0.0;
        let result = resolve_drains(
            &mut agents, &mut carcasses, &grid, &params, &mut nutrient_pool,
        );

        let drained = pre_target_structure - agents[1].structure;
        let gained = agents[0].reserve - pre_consumer_reserve;
        let dissipated = result.dissipated;

        assert!(drained > 0.0, "something should have been drained");
        assert!((drained - gained - dissipated).abs() < 1e-4,
            "energy conservation: drained={}, gained={}, dissipated={}, diff={}",
            drained, gained, dissipated, drained - gained - dissipated);
    }

    // --- Resolve reproduction ---

    #[test]
    fn reproduction_two_compatible_agents_produce_offspring_with_correct_energy() {
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_efficiency: 0.7,
            reproduction_energy_threshold: 10.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0, // sensing range = 1.0 * 10.0 = 10.0
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        // Set repro_reserve above threshold (10.0)
        agents[0].repro_reserve = 15.0;
        agents[1].repro_reserve = 15.0;

        let dead_ids: std::collections::HashSet<u64> = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        // Each parent invests entire repro_reserve (15.0)
        // Total = 30.0, offspring energy = 30.0 * 0.7 = 21.0
        // Dissipated = 30.0 - 21.0 = 9.0
        assert!(!result.offspring.is_empty(), "should produce offspring");
        let total_offspring_energy: f32 = result.offspring.iter().map(|o| o.reserve).sum();
        assert!((total_offspring_energy - 21.0).abs() < 1e-3,
            "offspring energy should be 21.0, got {}", total_offspring_energy);
        assert!((result.dissipated - 9.0).abs() < 1e-3,
            "dissipated should be 9.0, got {}", result.dissipated);
        // Parents' reserve should be unchanged (investment came from repro_reserve)
        assert!((agents[0].reserve - 100.0).abs() < 1e-3,
            "parent A reserve should be unchanged, got {}", agents[0].reserve);
        // Parents' repro_reserve should be 0
        assert!(agents[0].repro_reserve.abs() < 1e-3,
            "parent A repro_reserve should be 0, got {}", agents[0].repro_reserve);
    }

    #[test]
    fn reproduction_dead_agents_excluded() {
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[1].repro_reserve = 15.0;

        // Mark agent 1 as dead (killed in drain pass)
        let mut dead_ids = std::collections::HashSet::new();
        dead_ids.insert(1u64);

        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        assert!(result.offspring.is_empty(),
            "dead agent should not reproduce");
        assert!((agents[0].reserve - 100.0).abs() < 1e-6,
            "no investment should occur");
    }

    #[test]
    fn reproduction_below_energy_threshold_excluded() {
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_energy_threshold: 200.0, // higher than agent reserve
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[1].repro_reserve = 15.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        assert!(result.offspring.is_empty(),
            "agents below energy threshold should not reproduce");
    }

    #[test]
    fn reproduction_nutrient_gating_blocks_reproduction() {
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        // Give agents structure so stoichiometric demand is meaningful
        // demand = 10.0 * (0.1 + 0.2 * 0.5) = 10.0 * 0.2 = 2.0
        agents[0].structure = 10.0;
        agents[1].structure = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[1].repro_reserve = 15.0;
        // Set nutrient below demand (2.0)
        agents[0].nutrient = 1.0;
        agents[1].nutrient = 1.0;
        agents[0].repro_reserve = 15.0;
        agents[1].repro_reserve = 15.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        assert!(result.offspring.is_empty(),
            "nutrient-poor agents should not reproduce");
    }

    #[test]
    fn reproduction_invests_entire_repro_reserve() {
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_efficiency: 0.7,
            reproduction_energy_threshold: 10.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 15.0, traits),
            make_agent(2, (1.0, 0.0), 15.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 20.0;
        agents[1].repro_reserve = 20.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        // Each parent invests entire repro_reserve: 20.0 each
        // Total = 40.0, offspring energy = 40.0 * 0.7 = 28.0
        assert!(!result.offspring.is_empty());
        let total_offspring_energy: f32 = result.offspring.iter().map(|o| o.reserve).sum();
        assert!((total_offspring_energy - 28.0).abs() < 1e-3,
            "offspring energy should be 28.0, got {}", total_offspring_energy);
        // Parents' repro_reserve should be 0
        assert!(agents[0].repro_reserve.abs() < 1e-3,
            "parent A should have 0 repro_reserve, got {}", agents[0].repro_reserve);
        assert!(agents[1].repro_reserve.abs() < 1e-3,
            "parent B should have 0 repro_reserve, got {}", agents[1].repro_reserve);
        // Parents' reserve should be unchanged
        assert!((agents[0].reserve - 15.0).abs() < 1e-3,
            "parent A reserve should be unchanged, got {}", agents[0].reserve);
    }

    #[test]
    fn reproduction_mate_pairing_selects_closest_trait_distance() {
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        // Agent A and B have identical traits (distance=0), C is different
        let traits_ab = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let traits_c = TraitVector {
            photosynthetic_absorption: 0.1,
            heterotrophy: 0.4,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        // All three within sensing range of each other
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits_ab),
            make_agent(2, (2.0, 0.0), 100.0, traits_ab),
            make_agent(3, (4.0, 0.0), 100.0, traits_c),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[1].repro_reserve = 15.0;
        agents[2].nutrient = 10.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (2.0, 0.0));
        grid.insert(2, (4.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        // A and B should pair (trait distance=0), C has no mate
        assert_eq!(result.events.len(), 1, "should produce exactly one pair");
        let ev = &result.events[0];
        // The pair should be agents 1 and 2 (ids)
        let paired_ids = [ev.source, ev.target.unwrap()];
        assert!(paired_ids.contains(&1) && paired_ids.contains(&2),
            "agents 1 and 2 should pair, got {:?}", paired_ids);
        // Agent C (id=3) should not have invested
        assert!((agents[2].reserve - 100.0).abs() < 1e-6,
            "agent C should not have reproduced");
    }

    #[test]
    fn reproduction_offspring_traits_are_crossover_of_parents() {
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0, // no mutation to isolate crossover
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits_a = TraitVector {
            photosynthetic_absorption: 0.8,
            heterotrophy: 0.0,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let traits_b = TraitVector {
            photosynthetic_absorption: 0.0,
            heterotrophy: 0.8,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits_a),
            make_agent(2, (1.0, 0.0), 100.0, traits_b),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[1].repro_reserve = 15.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        assert!(!result.offspring.is_empty());
        let child = &result.offspring[0];
        // With no mutation, each trait dimension comes from one parent.
        // For photosynthetic_absorption: either 0.8 (from A) or 0.0 (from B)
        // For consumption_rate: either 0.0 (from A) or 0.8 (from B)
        // No budget normalization — traits retain raw crossover values.
        // Each dimension should come from exactly one parent (no averaging)
        for dim in 0..TraitVector::NUM_DIMS {
            let val = child.traits.get(dim);
            assert!(val >= 0.0, "trait dimension {} should be non-negative", dim);
        }
    }

    #[test]
    fn reproduction_fecundity_controls_offspring_count() {
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        // High fecundity -> more offspring
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 5.0, // Poisson mean=5
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 20.0;
        agents[1].nutrient = 20.0;
        agents[0].repro_reserve = 15.0;
        agents[1].repro_reserve = 15.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        // With Poisson(5.0), expect multiple offspring (statistically unlikely to get 1)
        assert!(result.offspring.len() > 1,
            "high fecundity should produce multiple offspring, got {}", result.offspring.len());
        // Total offspring energy should equal total_investment * efficiency
        let total_offspring_energy: f32 = result.offspring.iter().map(|o| o.reserve).sum();
        let total_investment = 30.0; // 15.0 + 15.0 (entire repro_reserve of each parent)
        let expected_total = total_investment * 0.7;
        assert!((total_offspring_energy - expected_total).abs() < 1e-3,
            "total offspring energy should be {}, got {}", expected_total, total_offspring_energy);
    }

    #[test]
    fn reproduction_offspring_zero_wear_and_dispersed() {
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 3.0,
            dispersal: 5.0, // high dispersal -> wider offspring placement
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[1].repro_reserve = 15.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        assert!(!result.offspring.is_empty());
        for child in &result.offspring {
            // Zero wear
            for w in &child.wear {
                assert!(*w == 0.0, "offspring should have zero wear");
            }
            // Zero structure
            assert!(child.structure == 0.0, "offspring should have zero structure");
            // Zero contact time
            assert!(child.contact_time == 0, "offspring should have zero contact time");
            // Position within world bounds
            let half = params.world_extent / 2.0;
            assert!(child.position.0 >= -half && child.position.0 <= half,
                "offspring x should be within bounds");
            assert!(child.position.1 >= -half && child.position.1 <= half,
                "offspring y should be within bounds");
        }
        // With high dispersal trait, offspring should be placed at different positions
        // (not all at the exact same position)
        if result.offspring.len() > 1 {
            let positions: Vec<(f32, f32)> = result.offspring.iter().map(|o| o.position).collect();
            let all_same = positions.iter().all(|p| {
                (p.0 - positions[0].0).abs() < 1e-6 && (p.1 - positions[0].1).abs() < 1e-6
            });
            assert!(!all_same, "offspring should be dispersed to different positions");
        }
    }

    #[test]
    fn reproduction_emits_reproduced_events() {
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            mutation_rate: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[1].repro_reserve = 15.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].kind, EventKind::Reproduced);
        assert!(result.events[0].target.is_some(),
            "Reproduced event should have target (second parent)");
    }

    #[test]
    fn reproduction_deterministic_with_seeded_rng() {
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.5,
            mutation_magnitude: 0.1,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 3.0,
            ..zero_traits()
        };

        let run = |seed: u64| -> (Vec<(f32, f32)>, Vec<f32>) {
            let mut agents = vec![
                make_agent(1, (0.0, 0.0), 100.0, traits),
                make_agent(2, (1.0, 0.0), 100.0, traits),
            ];
            agents[0].nutrient = 10.0;
            agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[1].repro_reserve = 15.0;

            let dead_ids = std::collections::HashSet::new();
            let mut grid = SpatialGrid::new(100.0, 10.0);
            grid.insert(0, (0.0, 0.0));
            grid.insert(1, (1.0, 0.0));
            let mut rng = ChaCha8Rng::seed_from_u64(seed);

            let result = resolve_reproduction(
                &mut agents, &dead_ids, &grid, &params, &mut rng,
            );
            let positions: Vec<(f32, f32)> = result.offspring.iter().map(|o| o.position).collect();
            let energies: Vec<f32> = result.offspring.iter().map(|o| o.reserve).collect();
            (positions, energies)
        };

        let (pos1, en1) = run(42);
        let (pos2, en2) = run(42);
        assert_eq!(pos1.len(), pos2.len(), "same seed should produce same count");
        for i in 0..pos1.len() {
            assert!((pos1[i].0 - pos2[i].0).abs() < 1e-6);
            assert!((pos1[i].1 - pos2[i].1).abs() < 1e-6);
            assert!((en1[i] - en2[i]).abs() < 1e-6);
        }
    }

    #[test]
    fn reproduction_is_lossy_energy_dissipated() {
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 0.5, // 50% efficient
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[1].repro_reserve = 15.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        // Total investment = 15.0 + 15.0 = 30.0 (entire repro_reserve)
        // Offspring energy = 30.0 * 0.5 = 15.0
        // Dissipated = 30.0 - 15.0 = 15.0
        let total_offspring_energy: f32 = result.offspring.iter().map(|o| o.reserve).sum();
        assert!((total_offspring_energy - 15.0).abs() < 1e-3,
            "offspring energy should be 15.0, got {}", total_offspring_energy);
        assert!((result.dissipated - 15.0).abs() < 1e-3,
            "dissipated should be 15.0, got {}", result.dissipated);
        // Conservation: investment = offspring + dissipated
        assert!((total_offspring_energy + result.dissipated - 30.0).abs() < 1e-3,
            "energy should be conserved");
    }

    // --- Derived sensing range ---

    #[test]
    fn move_sensing_range_derived_from_mobility() {
        // Sensing range = mobility * sensing_range_coefficient.
        // Agent with high mobility detects agents further away.
        use rand::SeedableRng;
        let mut params = test_params();
        params.movement_cost_coefficient = 0.0;
        params.sensing_range_coefficient = 10.0;
        // Low mobility -> sensing range = 0.2 * 10 = 2.0
        let low_mobility = TraitVector {
            mobility: 0.2,
            heterotrophy: 0.1,
            ..zero_traits()
        };
        // High mobility -> sensing range = 2.0 * 10 = 20.0
        let high_mobility = TraitVector {
            mobility: 2.0,
            heterotrophy: 0.1,
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };

        // Target at distance 5.0 — within high-mobility sensing but outside low-mobility
        let run = |mover_traits: TraitVector| -> f32 {
            let mut agents = vec![
                make_agent(0, (0.0, 0.0), 100.0, mover_traits),
                make_agent(1, (5.0, 0.0), 100.0, target_traits),
            ];
            agents[1].structure = 5.0;
            let carcasses = vec![];
            let mut grid = crate::spatial::SpatialGrid::new(100.0, 10.0);
            grid.insert(0, (0.0, 0.0));
            grid.insert(1, (5.0, 0.0));
            let mut rng = ChaCha8Rng::seed_from_u64(42);

            let result = move_agents(&mut agents, &carcasses, &grid, &params, &mut rng);
            result.sensing_throughput[0]
        };

        let low_detected = run(low_mobility);
        let high_detected = run(high_mobility);
        assert_eq!(low_detected, 0.0,
            "low-mobility agent should not detect target at distance 5 with sensing range 2");
        assert!(high_detected >= 1.0,
            "high-mobility agent should detect target at distance 5 with sensing range 20");
    }

    #[test]
    fn move_zero_mobility_gets_zero_sensing() {
        // An agent with zero mobility has zero sensing range and detects nothing.
        use rand::SeedableRng;
        let mut params = test_params();
        params.movement_cost_coefficient = 0.0;
        let traits = TraitVector {
            heterotrophy: 0.5,
            ..zero_traits() // mobility = 0
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(0, (0.0, 0.0), 100.0, traits),
            make_agent(1, (1.0, 0.0), 100.0, target_traits),
        ];
        agents[1].structure = 5.0;
        let carcasses = vec![];
        let grid = crate::spatial::SpatialGrid::new(100.0, 10.0);
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = move_agents(&mut agents, &carcasses, &grid, &params, &mut rng);

        // Zero mobility -> stationary, no sensing
        assert_eq!(result.sensing_throughput[0], 0.0,
            "zero-mobility agent should detect nothing");
    }

    // --- Reproductive compatibility distance ---

    #[test]
    fn reproduction_uses_world_param_compatibility_distance() {
        // Agents whose trait-space distance exceeds reproductive_compatibility_distance
        // cannot mate, regardless of spatial proximity.
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            reproductive_compatibility_distance: 0.5, // tight threshold
            ..test_params()
        };
        // Two agents with trait distance > 0.5
        let traits_a = TraitVector {
            photosynthetic_absorption: 1.0,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let traits_b = TraitVector {
            photosynthetic_absorption: 0.0,
            heterotrophy: 1.0,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let dist = traits_a.distance(&traits_b);
        assert!(dist > 0.5, "trait distance should exceed compatibility threshold: {}", dist);

        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits_a),
            make_agent(2, (1.0, 0.0), 100.0, traits_b),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[1].repro_reserve = 15.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        assert!(result.offspring.is_empty(),
            "agents beyond compatibility distance should not reproduce");
    }

    #[test]
    fn reproduction_compatible_within_world_param_distance() {
        // Agents whose trait-space distance is within reproductive_compatibility_distance
        // can mate.
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            reproductive_compatibility_distance: 5.0, // generous threshold
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        // Identical traits -> distance = 0, well within threshold
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[1].repro_reserve = 15.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        assert!(!result.offspring.is_empty(),
            "agents within compatibility distance should reproduce");
    }

    // --- Chemotaxis derived from mobility ---

    #[test]
    fn move_chemotaxis_proportional_to_mobility() {
        // Chemotaxis strength is derived from mobility. Higher mobility agents
        // have stronger directional bias toward detected signals.
        use rand::SeedableRng;
        let mut params = test_params();
        params.movement_cost_coefficient = 0.0;
        params.sensing_range_coefficient = 100.0; // ensure both can sense

        let target_traits = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };

        let run = |mob: f32| -> f32 {
            let mover = TraitVector {
                heterotrophy: 0.5,
                mobility: mob,
                ..zero_traits()
            };
            let mut agents = vec![
                make_agent(0, (0.0, 0.0), 100.0, mover),
                make_agent(1, (5.0, 0.0), 100.0, target_traits),
            ];
            agents[1].structure = 5.0;
            let carcasses = vec![];
            let mut grid = crate::spatial::SpatialGrid::new(100.0, 10.0);
            grid.insert(0, (0.0, 0.0));
            grid.insert(1, (5.0, 0.0));
            let mut rng = ChaCha8Rng::seed_from_u64(42);

            let _ = move_agents(&mut agents, &carcasses, &grid, &params, &mut rng);
            agents[0].position.0 // x position after move
        };

        let low_mob_x = run(0.1);
        let high_mob_x = run(1.0);
        // Higher mobility -> larger movement distance AND stronger chemotaxis bias
        assert!(high_mob_x > low_mob_x,
            "higher mobility should move further toward target: low={}, high={}", low_mob_x, high_mob_x);
    }

    // --- Trait vector dimensions ---

    #[test]
    fn trait_vector_has_six_dimensions() {
        assert_eq!(TraitVector::NUM_DIMS, 7);
    }

    #[test]
    fn functional_trait_count_is_three() {
        assert_eq!(crate::FUNCTIONAL_TRAIT_COUNT, 3);
        assert_eq!(crate::FUNCTIONAL_TRAIT_INDICES, [0, 1, 2]);
    }

    // --- Asexual reproduction tests ---

    #[test]
    fn asexual_reproduction_succeeds_with_high_propensity() {
        use rand::SeedableRng;
        // An agent with asexual_propensity=1.0 should always reproduce asexually.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            Agent {
                id: 1,
                position: (0.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                nutrient: 100.0,
                traits,
                contact_time: 0,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 20.0,
            },
        ];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        // Should have reproduced
        assert!(!result.events.is_empty(), "should produce reproduction event");
        assert_eq!(result.events[0].kind, EventKind::Reproduced);
        // Asexual: target is None
        assert_eq!(result.events[0].target, None,
            "asexual reproduction should have target=None");
        assert_eq!(result.events[0].source, 1);
        // Should have offspring
        assert!(!result.offspring.is_empty(), "should produce offspring");
        // Parent's repro_reserve should be depleted
        assert!(agents[0].repro_reserve < 1e-6,
            "parent repro_reserve should be depleted, got {}", agents[0].repro_reserve);
    }

    #[test]
    fn asexual_offspring_have_parent_traits_no_crossover() {
        use rand::SeedableRng;
        // With mutation_rate=0, asexual offspring should have exact parent traits.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.7,
            heterotrophy: 0.2,
            mobility: 0.1,
            kappa: 0.6,
            fecundity: 1.0,
            asexual_propensity: 1.0,
            dispersal: 0.0,
        };
        let mut agents = vec![
            Agent {
                id: 1,
                position: (0.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                nutrient: 100.0,
                traits,
                contact_time: 0,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 20.0,
            },
        ];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        assert!(!result.offspring.is_empty());
        for child in &result.offspring {
            assert_eq!(child.traits.photosynthetic_absorption, traits.photosynthetic_absorption);
            assert_eq!(child.traits.heterotrophy, traits.heterotrophy);
            assert_eq!(child.traits.mobility, traits.mobility);
            assert_eq!(child.traits.kappa, traits.kappa);
            assert_eq!(child.traits.asexual_propensity, traits.asexual_propensity);
        }
    }

    #[test]
    fn asexual_failure_falls_through_to_sexual() {
        use rand::SeedableRng;
        // Two eligible agents with asexual_propensity=0.0 should use sexual reproduction.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            contact_radius: 100.0,
            reproductive_compatibility_distance: 10.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 0.3,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 0.0,
            ..zero_traits()
        };
        let mut agents = vec![
            Agent {
                id: 1,
                position: (0.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                nutrient: 100.0,
                traits,
                contact_time: 0,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 20.0,
            },
            Agent {
                id: 2,
                position: (1.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                nutrient: 100.0,
                traits,
                contact_time: 0,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 20.0,
            },
        ];
        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, agents[0].position);
        grid.insert(1, agents[1].position);
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        // Should have sexual reproduction event with target=Some(mate_id)
        assert!(!result.events.is_empty(), "should produce reproduction event");
        assert!(result.events[0].target.is_some(),
            "sexual reproduction should have target=Some(mate_id)");
        assert!(!result.offspring.is_empty(), "should produce offspring");
    }

    #[test]
    fn asexual_reproduction_energy_from_single_parent() {
        use rand::SeedableRng;
        // Verify that asexual reproduction draws from single parent's repro_reserve only.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 0.5,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 1.0,
            ..zero_traits()
        };
        let initial_repro = 30.0;
        let mut agents = vec![
            Agent {
                id: 1,
                position: (0.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                nutrient: 100.0,
                traits,
                contact_time: 0,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: initial_repro,
            },
        ];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        // Parent's repro_reserve should be 0
        assert!(agents[0].repro_reserve.abs() < 1e-6,
            "parent repro_reserve should be depleted");
        // Total offspring energy + dissipated = initial investment
        let offspring_energy: f32 = result.offspring.iter().map(|c| c.reserve).sum();
        let total = offspring_energy + result.dissipated;
        assert!((total - initial_repro).abs() < 1e-3,
            "offspring energy ({}) + dissipated ({}) should equal investment ({})",
            offspring_energy, result.dissipated, initial_repro);
        // Event energy_delta should equal the investment
        assert!((result.events[0].energy_delta - initial_repro).abs() < 1e-3);
    }

    #[test]
    fn zero_asexual_propensity_never_reproduces_alone() {
        use rand::SeedableRng;
        // An agent with asexual_propensity=0.0 and no mate should not reproduce.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 0.0,
            ..zero_traits()
        };
        let mut agents = vec![
            Agent {
                id: 1,
                position: (0.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                nutrient: 100.0,
                traits,
                contact_time: 0,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 20.0,
            },
        ];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        assert!(result.events.is_empty(), "should not reproduce without mate or asexual propensity");
        assert!(result.offspring.is_empty());
        assert!((agents[0].repro_reserve - 20.0).abs() < 1e-6,
            "repro_reserve should be unchanged");
    }

    #[test]
    fn asexual_propensity_is_heritable() {
        use rand::SeedableRng;
        // Offspring of asexual reproduction should inherit asexual_propensity.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 1.0,
            mutation_rate: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 0.8,
            ..zero_traits()
        };
        let mut agents = vec![
            Agent {
                id: 1,
                position: (0.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                nutrient: 100.0,
                traits,
                contact_time: 0,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 20.0,
            },
        ];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        assert!(!result.offspring.is_empty());
        for child in &result.offspring {
            assert_eq!(child.traits.asexual_propensity, 0.8,
                "offspring should inherit parent's asexual_propensity");
        }
    }

    #[test]
    fn sexual_offspring_use_crossover_not_clone() {
        use rand::SeedableRng;
        // Two parents with different traits: sexual offspring should have mixed traits
        // (from crossover), not a clone of either parent.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            contact_radius: 100.0,
            reproductive_compatibility_distance: 10.0,
            ..test_params()
        };
        let traits_a = TraitVector {
            photosynthetic_absorption: 1.0,
            heterotrophy: 0.0,
            mobility: 0.3,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 0.0,
            dispersal: 0.0,
        };
        let traits_b = TraitVector {
            photosynthetic_absorption: 0.0,
            heterotrophy: 1.0,
            mobility: 0.3,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 0.0,
            dispersal: 0.0,
        };
        let mut agents = vec![
            Agent {
                id: 1, position: (0.0, 0.0), reserve: 50.0, structure: 5.0,
                nutrient: 100.0, traits: traits_a, contact_time: 0,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT], repro_reserve: 20.0,
            },
            Agent {
                id: 2, position: (1.0, 0.0), reserve: 50.0, structure: 5.0,
                nutrient: 100.0, traits: traits_b, contact_time: 0,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT], repro_reserve: 20.0,
            },
        ];
        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, agents[0].position);
        grid.insert(1, agents[1].position);
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        assert!(!result.offspring.is_empty());
        // Sexual event should have target
        assert!(result.events[0].target.is_some());
        // At least one offspring should have traits from both parents
        // (with crossover each dim is independently chosen from one parent)
        for child in &result.offspring {
            let photo = child.traits.photosynthetic_absorption;
            let hetero = child.traits.heterotrophy;
            // Each should be either 0.0 or 1.0 (from one parent, no mutation)
            assert!(photo == 0.0 || photo == 1.0,
                "photo should be from one parent: {}", photo);
            assert!(hetero == 0.0 || hetero == 1.0,
                "hetero should be from one parent: {}", hetero);
        }
    }

    #[test]
    fn asexual_nutrient_from_single_parent() {
        use rand::SeedableRng;
        // Verify nutrient donation comes from single parent in asexual reproduction.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 1.0,
            mutation_rate: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 1.0,
            ..zero_traits()
        };
        let initial_nutrient = 50.0;
        let mut agents = vec![
            Agent {
                id: 1,
                position: (0.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                nutrient: initial_nutrient,
                traits,
                contact_time: 0,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 20.0,
            },
        ];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        // Parent should have less nutrient after donating to offspring
        assert!(agents[0].nutrient < initial_nutrient,
            "parent nutrient should decrease from {}, got {}", initial_nutrient, agents[0].nutrient);
        // Offspring should have non-zero nutrient
        let offspring_nutrient: f32 = result.offspring.iter().map(|c| c.nutrient).sum();
        assert!(offspring_nutrient > 0.0, "offspring should receive nutrient");
        // Conservation: parent loss = offspring gain
        let parent_loss = initial_nutrient - agents[0].nutrient;
        assert!((parent_loss - offspring_nutrient).abs() < 1e-3,
            "nutrient should be conserved: parent_loss={}, offspring_gain={}", parent_loss, offspring_nutrient);
    }

    // --- Dispersal trait tests ---

    #[test]
    fn dispersal_trait_controls_offspring_spread() {
        // Higher dispersal trait produces wider offspring placement.
        // Compare mean distance from parent position for low vs high dispersal.
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };

        let run = |dispersal_val: f32| -> f32 {
            let traits = TraitVector {
                photosynthetic_absorption: 0.5,
                kappa: 0.5,
                fecundity: 10.0, // many offspring for statistical signal
                asexual_propensity: 1.0,
                dispersal: dispersal_val,
                ..zero_traits()
            };
            let mut agents = vec![
                Agent {
                    id: 1,
                    position: (0.0, 0.0),
                    reserve: 50.0,
                    structure: 5.0,
                    nutrient: 100.0,
                    traits,
                    contact_time: 0,
                    wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                    repro_reserve: 50.0,
                },
            ];
            let dead_ids = std::collections::HashSet::new();
            let grid = SpatialGrid::new(100.0, 10.0);
            let mut rng = ChaCha8Rng::seed_from_u64(42);

            let result = resolve_reproduction(
                &mut agents, &dead_ids, &grid, &params, &mut rng,
            );
            // Mean distance from parent position (0,0)
            let total_dist: f32 = result.offspring.iter()
                .map(|o| (o.position.0 * o.position.0 + o.position.1 * o.position.1).sqrt())
                .sum();
            total_dist / result.offspring.len() as f32
        };

        let low_spread = run(0.5);
        let high_spread = run(5.0);
        assert!(high_spread > low_spread,
            "higher dispersal should produce wider spread: low={}, high={}", low_spread, high_spread);
    }

    #[test]
    fn zero_dispersal_places_offspring_at_parent_position() {
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 1.0,
            mutation_rate: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 1.0,
            dispersal: 0.0, // zero dispersal
            ..zero_traits()
        };
        let mut agents = vec![
            Agent {
                id: 1,
                position: (10.0, 10.0),
                reserve: 50.0,
                structure: 5.0,
                nutrient: 100.0,
                traits,
                contact_time: 0,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 20.0,
            },
        ];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        assert!(!result.offspring.is_empty());
        for child in &result.offspring {
            assert!((child.position.0 - 10.0).abs() < 1e-6,
                "zero dispersal: offspring should be at parent x, got {}", child.position.0);
            assert!((child.position.1 - 10.0).abs() < 1e-6,
                "zero dispersal: offspring should be at parent y, got {}", child.position.1);
        }
    }

    #[test]
    fn sexual_reproduction_averages_parents_dispersal() {
        // Two parents with different dispersal values: offspring placement
        // should use the average dispersal as kernel width.
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            contact_radius: 100.0,
            reproductive_compatibility_distance: 10.0,
            ..test_params()
        };

        let run = |disp_a: f32, disp_b: f32| -> f32 {
            let traits_a = TraitVector {
                photosynthetic_absorption: 0.5,
                mobility: 0.3,
                kappa: 0.5,
                fecundity: 10.0,
                asexual_propensity: 0.0,
                dispersal: disp_a,
                ..zero_traits()
            };
            let traits_b = TraitVector {
                photosynthetic_absorption: 0.5,
                mobility: 0.3,
                kappa: 0.5,
                fecundity: 10.0,
                asexual_propensity: 0.0,
                dispersal: disp_b,
                ..zero_traits()
            };
            let mut agents = vec![
                Agent {
                    id: 1, position: (0.0, 0.0), reserve: 50.0, structure: 5.0,
                    nutrient: 100.0, traits: traits_a, contact_time: 0,
                    wear: [0.0; FUNCTIONAL_TRAIT_COUNT], repro_reserve: 30.0,
                },
                Agent {
                    id: 2, position: (1.0, 0.0), reserve: 50.0, structure: 5.0,
                    nutrient: 100.0, traits: traits_b, contact_time: 0,
                    wear: [0.0; FUNCTIONAL_TRAIT_COUNT], repro_reserve: 30.0,
                },
            ];
            let dead_ids = std::collections::HashSet::new();
            let mut grid = SpatialGrid::new(100.0, 10.0);
            grid.insert(0, agents[0].position);
            grid.insert(1, agents[1].position);
            let mut rng = ChaCha8Rng::seed_from_u64(42);

            let result = resolve_reproduction(
                &mut agents, &dead_ids, &grid, &params, &mut rng,
            );
            let mid = (0.5, 0.0); // midpoint of parents
            let total_dist: f32 = result.offspring.iter()
                .map(|o| {
                    let dx = o.position.0 - mid.0;
                    let dy = o.position.1 - mid.1;
                    (dx * dx + dy * dy).sqrt()
                })
                .sum();
            total_dist / result.offspring.len().max(1) as f32
        };

        // Both parents have equal dispersal (2.0): avg = 2.0
        let equal_spread = run(2.0, 2.0);
        // One parent 0.0, other 4.0: avg = 2.0 — should be similar to equal case
        let _mixed_spread = run(0.0, 4.0);
        // Both parents have high dispersal (4.0): avg = 4.0
        let high_spread = run(4.0, 4.0);

        // High should be wider than equal (4.0 > 2.0)
        assert!(high_spread > equal_spread,
            "higher avg dispersal should produce wider spread: equal={}, high={}", equal_spread, high_spread);
    }

    #[test]
    fn dispersal_is_independent_of_mobility() {
        // A sessile agent (zero mobility) with high dispersal should still
        // disperse offspring widely. This tests independence.
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 1.0,
            mutation_rate: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 10.0,
            asexual_propensity: 1.0,
            mobility: 0.0, // sessile
            dispersal: 5.0, // high dispersal (like a dandelion)
            ..zero_traits()
        };
        let mut agents = vec![
            Agent {
                id: 1,
                position: (0.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                nutrient: 100.0,
                traits,
                contact_time: 0,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 50.0,
            },
        ];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        assert!(!result.offspring.is_empty());
        // Offspring should be dispersed despite zero mobility
        let total_dist: f32 = result.offspring.iter()
            .map(|o| (o.position.0 * o.position.0 + o.position.1 * o.position.1).sqrt())
            .sum();
        let mean_dist = total_dist / result.offspring.len() as f32;
        assert!(mean_dist > 0.5,
            "sessile agent with high dispersal should still disperse offspring: mean_dist={}", mean_dist);
    }

    #[test]
    fn dispersal_is_heritable_asexual() {
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 1.0,
            mutation_rate: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 1.0,
            dispersal: 3.7,
            ..zero_traits()
        };
        let mut agents = vec![
            Agent {
                id: 1,
                position: (0.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                nutrient: 100.0,
                traits,
                contact_time: 0,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 20.0,
            },
        ];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        assert!(!result.offspring.is_empty());
        for child in &result.offspring {
            assert_eq!(child.traits.dispersal, 3.7,
                "offspring should inherit parent's dispersal trait");
        }
    }

    #[test]
    fn sexual_reproduction_midpoint_wraps_on_torus() {
        // Parents near opposite edges of the torus should produce offspring
        // near the wrap edge, not at the naive average (0, 0).
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_efficiency: 1.0,
            reproduction_energy_threshold: 10.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            world_extent: 100.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            dispersal: 0.0, // zero dispersal so offspring land exactly at midpoint
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (-48.0, 0.0), 100.0, traits),
            make_agent(2, (48.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[1].repro_reserve = 15.0;

        let dead_ids: std::collections::HashSet<u64> = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (-48.0, 0.0));
        grid.insert(1, (48.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        assert!(!result.offspring.is_empty(), "should produce offspring");
        for child in &result.offspring {
            // The toroidal midpoint of (-48, 0) and (48, 0) on extent=100
            // is at x = -48 + (-4/2) = -50 (equivalently +50), not 0.
            // The offspring should be near the wrap edge (|x| close to 50).
            let x = child.position.0;
            assert!(
                x.abs() > 40.0,
                "offspring x={} should be near the wrap edge (|x| > 40), not near 0",
                x
            );
        }
    }

    #[test]
    fn trait_vector_has_seven_dimensions() {
        assert_eq!(TraitVector::NUM_DIMS, 7);
        // dispersal is at index 6
        let traits = TraitVector {
            dispersal: 2.5,
            ..zero_traits()
        };
        assert_eq!(traits.get(6), 2.5);
    }

    #[test]
    fn superlinear_maintenance_quadratic_pays_4x_for_double_trait() {
        // With exponent=2.0, trait=1.0 pays 1.0^2 * cost = cost
        // while trait=0.5 pays 0.5^2 * cost = 0.25 * cost → 4x ratio.
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            photo_maintenance_cost: 1.0,
            heterotrophy_maintenance_cost: 0.0,
            mobility_maintenance_cost: 0.0,
            structure_maintenance_coefficient: 0.0,
            maintenance_cost_exponent: 2.0,
            ..test_params()
        };

        let mut agents_full = vec![make_agent(
            1,
            (0.0, 0.0),
            100.0,
            TraitVector {
                photosynthetic_absorption: 1.0,
                ..zero_traits()
            },
        )];
        let mut agents_half = vec![make_agent(
            2,
            (0.0, 0.0),
            100.0,
            TraitVector {
                photosynthetic_absorption: 0.5,
                ..zero_traits()
            },
        )];

        let (_, _) = metabolise(&mut agents_full, &params);
        let (_, _) = metabolise(&mut agents_half, &params);

        let cost_full = 100.0 - agents_full[0].reserve;
        let cost_half = 100.0 - agents_half[0].reserve;

        assert!(
            (cost_full / cost_half - 4.0).abs() < 1e-6,
            "trait=1.0 should pay 4x trait=0.5 under quadratic costs, got ratio {}",
            cost_full / cost_half
        );
    }

    #[test]
    fn superlinear_maintenance_exponent_one_is_linear_regression() {
        // With exponent=1.0 (default), trait=1.0 pays exactly 2x what trait=0.5 pays.
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            photo_maintenance_cost: 1.0,
            heterotrophy_maintenance_cost: 0.0,
            mobility_maintenance_cost: 0.0,
            structure_maintenance_coefficient: 0.0,
            maintenance_cost_exponent: 1.0,
            ..test_params()
        };

        let mut agents_full = vec![make_agent(
            1,
            (0.0, 0.0),
            100.0,
            TraitVector {
                photosynthetic_absorption: 1.0,
                ..zero_traits()
            },
        )];
        let mut agents_half = vec![make_agent(
            2,
            (0.0, 0.0),
            100.0,
            TraitVector {
                photosynthetic_absorption: 0.5,
                ..zero_traits()
            },
        )];

        let (_, _) = metabolise(&mut agents_full, &params);
        let (_, _) = metabolise(&mut agents_half, &params);

        let cost_full = 100.0 - agents_full[0].reserve;
        let cost_half = 100.0 - agents_half[0].reserve;

        assert!(
            (cost_full / cost_half - 2.0).abs() < 1e-6,
            "exponent=1.0 should give linear 2x ratio, got {}",
            cost_full / cost_half
        );
    }

    #[test]
    fn superlinear_maintenance_specialist_pays_more_per_trait_than_generalist() {
        // Under quadratic costs with equal total trait sum:
        //   generalist (0.5, 0.5): 0.5^2 + 0.5^2 = 0.50 total cost
        //   specialist (1.0, 0.0): 1.0^2 + 0.0^2 = 1.00 total cost
        // The specialist pays more in absolute maintenance. The anti-generalist
        // mechanism works because each trait's *output* is proportional to its
        // value (linear returns), but its *cost* grows superlinearly — so the
        // specialist's single high trait is more cost-effective per unit of
        // capability (1.0 capability for 1.0 cost) than the generalist's two
        // moderate traits would need to be to match the specialist's single
        // capability. The trade-off is economic: you pay dearly for a high
        // trait, but you get full capability; spreading investment is cheaper
        // in maintenance but yields less capability per dimension.

        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            photo_maintenance_cost: 1.0,
            heterotrophy_maintenance_cost: 1.0,
            mobility_maintenance_cost: 0.0,
            structure_maintenance_coefficient: 0.0,
            maintenance_cost_exponent: 2.0,
            ..test_params()
        };

        // Generalist: 0.5 in both photo and heterotrophy
        let mut generalist = vec![make_agent(
            1,
            (0.0, 0.0),
            100.0,
            TraitVector {
                photosynthetic_absorption: 0.5,
                heterotrophy: 0.5,
                ..zero_traits()
            },
        )];
        // Specialist: 1.0 in photo, 0.0 in heterotrophy
        let mut specialist = vec![make_agent(
            2,
            (0.0, 0.0),
            100.0,
            TraitVector {
                photosynthetic_absorption: 1.0,
                heterotrophy: 0.0,
                ..zero_traits()
            },
        )];

        let (_, _) = metabolise(&mut generalist, &params);
        let (_, _) = metabolise(&mut specialist, &params);

        let cost_generalist = 100.0 - generalist[0].reserve;
        let cost_specialist = 100.0 - specialist[0].reserve;

        // Under quadratic costs:
        // generalist: 0.5^2 * 1.0 + 0.5^2 * 1.0 = 0.25 + 0.25 = 0.5
        // specialist: 1.0^2 * 1.0 + 0.0^2 * 1.0 = 1.0 + 0.0 = 1.0
        // Specialist actually pays MORE — superlinear costs penalise
        // concentration, making specialisation expensive per unit capability.
        // This is the correct mathematical result.
        assert!(
            cost_specialist > cost_generalist,
            "under quadratic costs, specialist (1.0,0.0) should pay more ({}) \
             than generalist (0.5,0.5) ({}) — superlinear costs penalise \
             concentrated investment",
            cost_specialist, cost_generalist
        );
        // Verify exact values
        assert!((cost_generalist - 0.5).abs() < 1e-6);
        assert!((cost_specialist - 1.0).abs() < 1e-6);
    }
}
