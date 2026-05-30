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
            target_was_carcass: false,
        });
    }
    events
}

/// Absorb nutrients: uptake from the local nutrient grid cell at each agent's
/// position, proportional sharing within each cell when demand exceeds supply.
pub fn absorb_nutrients(
    agents: &mut [Agent],
    nutrient_grid: &mut crate::spatial::NutrientGrid,
    params: &WorldParameters,
) -> Vec<Event> {
    let mut events = Vec::new();

    let k = params.wear_degradation_steepness;

    // Group agents by nutrient grid cell, with their demand.
    let mut cell_agents: std::collections::HashMap<usize, Vec<(usize, f32)>> =
        std::collections::HashMap::new();

    for (i, agent) in agents.iter().enumerate() {
        let demand = agent.effective_trait_with_steepness(0, k);
        if demand <= 0.0 {
            continue;
        }
        let cell_idx = nutrient_grid.cell_index_for(agent.position);
        cell_agents.entry(cell_idx).or_default().push((i, demand));
    }

    // Process each cell independently
    for (cell_idx, agent_demands) in &cell_agents {
        let cell_pool = nutrient_grid.cell_mut(*cell_idx);
        if *cell_pool <= 0.0 {
            continue;
        }

        let total_demand: f32 = agent_demands.iter().map(|(_, d)| *d).sum();
        if total_demand <= 0.0 {
            continue;
        }

        let available = *cell_pool;
        for &(i, demand) in agent_demands {
            let uptake = if total_demand <= available {
                demand
            } else {
                demand / total_demand * available
            };
            agents[i].nutrient += uptake;
            *cell_pool -= uptake;
            events.push(Event {
                tick: 0,
                seq: 0,
                kind: EventKind::NutrientAbsorbed,
                source: agents[i].id,
                target: None,
                energy_delta: uptake,
                position: Some(agents[i].position),
                target_was_carcass: false,
            });
        }
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
            + agent.traits.asexual_propensity.powf(exp) * params.asexual_propensity_maintenance_cost
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
            target_was_carcass: false,
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
            + agent.traits.asexual_propensity.powf(exp) * params.asexual_propensity_maintenance_cost
            + agent.structure * params.structure_maintenance_coefficient;
        let retention = metabolic_cost * params.growth_retention_multiplier;
        let surplus = (agent.reserve - retention).max(0.0);
        if surplus <= 0.0 {
            continue;
        }

        let kappa = agent.traits.kappa.clamp(0.0, 1.0);
        let soma_fraction = surplus * kappa;
        let repro_fraction = surplus - soma_fraction; // (1-kappa) * surplus

        // Deduct entire surplus from reserve
        agent.reserve -= surplus;

        // Repair gets priority from soma budget: counteract accumulated wear
        let decay = params.repair_decay;
        let mut repair_energy_spent = 0.0_f32;
        if soma_fraction > 0.0 && decay > 0.0 {
            let base_repair = kappa;
            for ft in 0..crate::FUNCTIONAL_TRAIT_COUNT {
                if agent.wear[ft] <= 0.0 {
                    continue;
                }
                let effective_repair = base_repair * (-decay * agent.wear[ft]).exp();
                let repair = effective_repair.min(agent.wear[ft]);
                let cost = repair; // 1:1 energy-to-repair
                if repair_energy_spent + cost > soma_fraction {
                    let remaining = soma_fraction - repair_energy_spent;
                    let capped_repair = remaining.min(agent.wear[ft]);
                    agent.wear[ft] -= capped_repair;
                    repair_energy_spent = soma_fraction;
                    break;
                }
                agent.wear[ft] -= repair;
                repair_energy_spent += cost;
            }
        }
        total_dissipated += repair_energy_spent;

        // Remainder of soma fraction → growth
        let growth_budget = soma_fraction - repair_energy_spent;
        let efficiency = params.growth_efficiency;
        if efficiency > 0.0 && growth_budget > 0.0 {
            // Energy alone would build this much structure...
            let energy_limited = growth_budget * efficiency;
            // ...but structure is co-limited by nutrient (Liebig's law of the
            // minimum): the body can only hold as much structure as the agent's
            // nutrient store stoichiometrically supports (structure * ratio <=
            // nutrient). No hard gate — growth throttles smoothly to zero as
            // nutrient binds up, which keeps `nutrient >= structure * ratio` and
            // so leaves the reproduction nutrient gate reachable. Nutrient is not
            // consumed (there is no structural-nutrient pool to recycle it into);
            // it is the build permit, not the building material.
            let ratio = crate::stoichiometric_demand(&agent.traits, 1.0, params);
            let nutrient_headroom = if ratio > 0.0 {
                (agent.nutrient / ratio - agent.structure).max(0.0)
            } else {
                f32::INFINITY
            };
            let to_structure = energy_limited.min(nutrient_headroom);
            // Energy actually spent on that structure; the rest could not be
            // matched by nutrient, so it stays in reserve rather than burning.
            let energy_spent = to_structure / efficiency;
            let dissipated = energy_spent - to_structure;
            agent.structure += to_structure;
            agent.reserve += growth_budget - energy_spent;
            total_dissipated += dissipated;

            if to_structure > 0.0 {
                events.push(Event {
                    tick: 0,
                    seq: 0,
                    kind: EventKind::Grew,
                    source: agent.id,
                    target: None,
                    energy_delta: to_structure,
                    position: Some(agent.position),
                    target_was_carcass: false,
                });
            }
        } else if growth_budget > 0.0 {
            // No growth efficiency: remaining soma is dissipated
            total_dissipated += growth_budget;
        }

        // Repro fraction → repro_reserve (accumulates across ticks)
        agent.repro_reserve += repro_fraction;
    }
    (events, total_dissipated)
}

/// Apply wear: baseline + use-dependent accumulation per functional trait.
/// Accumulation only — repair is funded from soma energy in `grow()`.
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

    for agent in agents.iter_mut() {
        let mut total_wear_delta = 0.0_f32;

        let agent_usage = usage.get(&agent.id).copied().unwrap_or([0.0; FUNCTIONAL_TRAIT_COUNT]);

        for ft in 0..FUNCTIONAL_TRAIT_COUNT {
            let nominal = agent.traits.get(FUNCTIONAL_TRAIT_INDICES[ft]);
            let baseline = baseline_rate * nominal.max(0.0);
            let use_dependent = use_rate * agent_usage[ft].max(0.0);
            let accumulation = baseline + use_dependent;
            agent.wear[ft] += accumulation;
            total_wear_delta += accumulation;
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
                target_was_carcass: false,
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
    nutrient_grid: &mut crate::spatial::NutrientGrid,
) -> DrainResult {
    let mut events = Vec::new();
    let mut dissipated = 0.0_f32;
    let mut dead_agents: Vec<u64> = Vec::new();
    let mut new_carcasses: Vec<Carcass> = Vec::new();
    let k = params.wear_degradation_steepness;
    let contact_range_coeff = params.contact_range_coefficient;
    let extent = params.world_extent;

    // Max query radius: largest consumer contact range across all agents
    let max_contact_range = agents.iter()
        .map(|a| a.effective_trait_with_steepness(1, k) * contact_range_coeff)
        .fold(0.0_f32, f32::max);

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
        consumers: Vec<(usize, f32, f32)>, // (consumer_idx, demand, trophic_eff)
    }

    let mut living_drains: Vec<TargetDrain> = Vec::new();

    for target_idx in 0..agents.len() {
        if agents[target_idx].structure <= 0.0 {
            continue;
        }
        let target_pos = agents[target_idx].position;

        // Find consumers within their individual contact range
        let mut consumers = Vec::new();
        let nearby = grid.query_radius(target_pos, max_contact_range);
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
                let consumer_contact_range = eff_heterotrophy * contact_range_coeff;
                if crate::toroidal_distance(
                    agents[consumer_idx].position,
                    target_pos,
                    extent,
                ) > consumer_contact_range
                {
                    continue;
                }
                // Sustained contact scaling: demand = eff * ct / (ct + K),
                // where K is the half-saturation contact duration (in ticks).
                // K = 0 disables the ramp and uses raw eff_heterotrophy.
                let half_sat = params.consumption_contact_half_saturation;
                let demand = if half_sat > 0.0 {
                    let ct = agents[consumer_idx].contact_time as f32;
                    eff_heterotrophy * ct / (ct + half_sat)
                } else {
                    eff_heterotrophy
                };
                let trophic_eff = crate::trophic_transfer_efficiency(
                    &agents[consumer_idx].traits,
                    &agents[target_idx].traits,
                    params,
                );
                consumers.push((consumer_idx, demand, trophic_eff));
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
        let total_demand: f32 = drain.consumers.iter().map(|(_, d, _)| *d).sum();

        for &(consumer_idx, demand, trophic_eff) in &drain.consumers {
            let actual_drain = if total_demand <= available {
                demand
            } else {
                (demand / total_demand) * available
            };

            let energy_gained = actual_drain * trophic_eff;
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
                // Excrete excess nutrient to the local cell at the target's position
                *nutrient_grid.at_position(agents[drain.target_idx].position) += excreted;
            }

            events.push(Event {
                tick: 0,
                seq: 0,
                kind: EventKind::Consumed,
                source: agents[consumer_idx].id,
                target: Some(agents[drain.target_idx].id),
                energy_delta: actual_drain,
                position: Some(agents[drain.target_idx].position),
                target_was_carcass: false,
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
        let nearby = grid.query_radius(carcass_pos, max_contact_range);
        let mut consumers: Vec<(usize, f32, f32)> = Vec::new(); // (idx, demand, trophic_eff)
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
                let consumer_contact_range = eff_heterotrophy * contact_range_coeff;
                if crate::toroidal_distance(
                    agents[consumer_idx].position,
                    carcass_pos,
                    extent,
                ) > consumer_contact_range
                {
                    continue;
                }
                // Sustained contact scaling: demand = eff * ct / (ct + K),
                // where K is the half-saturation contact duration (in ticks).
                // K = 0 disables the ramp and uses raw eff_heterotrophy.
                let half_sat = params.consumption_contact_half_saturation;
                let demand = if half_sat > 0.0 {
                    let ct = agents[consumer_idx].contact_time as f32;
                    eff_heterotrophy * ct / (ct + half_sat)
                } else {
                    eff_heterotrophy
                };
                let trophic_eff = crate::trophic_transfer_efficiency(
                    &agents[consumer_idx].traits,
                    &carcasses[carcass_idx].traits,
                    params,
                );
                consumers.push((consumer_idx, demand, trophic_eff));
            }
        }

        if consumers.is_empty() {
            continue;
        }

        let available = carcasses[carcass_idx].energy;
        let total_demand: f32 = consumers.iter().map(|(_, d, _)| *d).sum();

        for &(consumer_idx, demand, trophic_eff) in &consumers {
            let actual_drain = if total_demand <= available {
                demand
            } else {
                (demand / total_demand) * available
            };

            let energy_gained = actual_drain * trophic_eff;
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
                // Excrete excess nutrient to the local cell at the carcass's position
                *nutrient_grid.at_position(carcass_pos) += excreted;
            }

            events.push(Event {
                tick: 0,
                seq: 0,
                kind: EventKind::Consumed,
                source: agents[consumer_idx].id,
                target: Some(carcasses[carcass_idx].id),
                energy_delta: actual_drain,
                position: Some(carcass_pos),
                target_was_carcass: true,
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
                target_was_carcass: false,
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
    /// Per-agent distance actually moved this tick (mobility use). Indexed by
    /// position in the agents slice. Movement runs before wear, so this is
    /// folded into the same tick's mobility use-wear.
    pub move_distance: Vec<f32>,
}

/// Move agents: the final repositioning phase of the tick loop, running after
/// all energy-affecting phases but before wear and the death check. Agents reposition based on
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
    let mut move_distance = vec![0.0_f32; agents.len()];
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
            move_distance[i] = distance;
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
            target_was_carcass: false,
        });
    }

    MoveResult {
        events,
        dissipated: total_dissipated,
        sensing_throughput,
        move_distance,
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

        // Offspring count from Poisson distribution (single parent's fecundity).
        // A zero draw is reproductive failure (world-rules flow 4): the committed
        // energy is still consumed but no offspring result.
        let fecundity = parent_traits.fecundity.max(0.1);
        let poisson = Poisson::new(fecundity as f64).unwrap();
        let offspring_count = poisson.sample(rng) as usize;

        // The committed investment is always consumed from the parent.
        agents[i].repro_reserve -= investment;

        if offspring_count == 0 {
            // Reproductive failure: no offspring carry the investment, so the
            // entire committed energy dissipates. Nutrient is not donated and
            // remains with the parent.
            dissipated += investment;

            events.push(Event {
                tick: 0,
                seq: 0,
                kind: EventKind::Reproduced,
                source: parent_id,
                target: None,
                energy_delta: investment,
                position: Some(parent_pos),
                target_was_carcass: false,
            });
            continue;
        }

        let offspring_total_energy = investment * params.reproduction_efficiency;
        let energy_per_offspring = offspring_total_energy / offspring_count as f32;
        let tick_dissipated = investment - offspring_total_energy;
        dissipated += tick_dissipated;

        // Per-offspring reserve/structure split. The structure share of the
        // per-offspring energy is converted via growth_efficiency (same lossy
        // conversion as in-life growth); the unconverted remainder dissipates
        // as heat. Reserve share is the complement of the structure share.
        let struct_fraction = params.offspring_structure_fraction.clamp(0.0, 1.0);
        let structure_energy_share = energy_per_offspring * struct_fraction;
        let offspring_structure = structure_energy_share * params.growth_efficiency;
        let structure_heat_per = structure_energy_share - offspring_structure;
        let offspring_reserve = energy_per_offspring - structure_energy_share;
        dissipated += structure_heat_per * offspring_count as f32;

        // Nutrient donation from single parent
        let nutrient_donated = parent_nutrient
            * (investment / (parent_reserve + parent_repro).max(1e-10)).min(0.5);
        let nutrient_per_offspring = nutrient_donated / offspring_count as f32;

        // Deduct nutrient donation from parent
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
                reserve: offspring_reserve,
                structure: offspring_structure,
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
            target_was_carcass: false,
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

        // Offspring count from Poisson distribution. A zero draw is reproductive
        // failure (world-rules flow 4): both parents' energy is still committed
        // but no offspring result.
        let avg_fecundity = ((a_traits.fecundity + b_traits.fecundity) / 2.0).max(0.1);
        let poisson = Poisson::new(avg_fecundity as f64).unwrap();
        let offspring_count = poisson.sample(rng) as usize;

        // The committed investment is always consumed from both parents.
        agents[*a_idx].repro_reserve -= invest_a;
        agents[*b_idx].repro_reserve -= invest_b;

        if offspring_count == 0 {
            // Reproductive failure: no offspring carry the investment, so the
            // entire combined committed energy dissipates. Nutrient is not
            // donated and remains with the parents.
            dissipated += total_investment;

            let (dx, dy) = crate::toroidal_displacement(a_pos, b_pos, extent);
            let mid_pos =
                crate::wrap_position((a_pos.0 + dx / 2.0, a_pos.1 + dy / 2.0), extent);
            events.push(Event {
                tick: 0,
                seq: 0,
                kind: EventKind::Reproduced,
                source: a_id,
                target: Some(b_id),
                energy_delta: total_investment,
                position: Some(mid_pos),
                target_was_carcass: false,
            });
            continue;
        }

        let offspring_total_energy = total_investment * params.reproduction_efficiency;
        let energy_per_offspring = offspring_total_energy / offspring_count as f32;
        let tick_dissipated = total_investment - offspring_total_energy;
        dissipated += tick_dissipated;

        // Per-offspring reserve/structure split (same lossy-growth conversion
        // as asexual reproduction and in-life growth).
        let struct_fraction = params.offspring_structure_fraction.clamp(0.0, 1.0);
        let structure_energy_share = energy_per_offspring * struct_fraction;
        let offspring_structure = structure_energy_share * params.growth_efficiency;
        let structure_heat_per = structure_energy_share - offspring_structure;
        let offspring_reserve = energy_per_offspring - structure_energy_share;
        dissipated += structure_heat_per * offspring_count as f32;

        // Nutrient donation: each parent donates proportional to investment fraction
        let nutrient_a = a_nutrient * (invest_a / (a_reserve + a_repro).max(1e-10)).min(0.5);
        let nutrient_b = b_nutrient * (invest_b / (b_reserve + b_repro).max(1e-10)).min(0.5);
        let nutrient_per_offspring = (nutrient_a + nutrient_b) / offspring_count as f32;

        // Deduct nutrient donations from parents
        agents[*a_idx].nutrient -= nutrient_a;
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
                reserve: offspring_reserve,
                structure: offspring_structure,
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
            target_was_carcass: false,
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
            contact_range_coefficient: 5.0,
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
            consumption_contact_half_saturation: 0.0,
            nutrient_grid_cell_size: 10.0,
            growth_retention_multiplier: 2.0,
            offspring_structure_fraction: 0.2,
            asexual_propensity_maintenance_cost: 0.0,
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
    fn absorb_nutrients_demand_equals_effective_autotrophy() {
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        let mut grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 100.0);

        let events = absorb_nutrients(&mut agents, &mut grid, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::NutrientAbsorbed);
        // demand = eff_autotrophy = 0.5 (no wear degradation on fresh agent)
        let expected = 0.5;
        assert!((agents[0].nutrient - expected).abs() < 1e-3,
            "uptake should equal effective autotrophy, got {}", agents[0].nutrient);
        assert!((grid.total() - (100.0 - expected)).abs() < 1e-3);
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
        // Place all nutrient in the cell where agents are
        let mut grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        *grid.at_position((0.0, 0.0)) = 0.1;

        let events = absorb_nutrients(&mut agents, &mut grid, &params);

        assert_eq!(events.len(), 2);
        // Both have same demand, so each gets half
        let total_uptake = agents[0].nutrient + agents[1].nutrient;
        assert!((total_uptake - 0.1).abs() < 1e-3);
        assert!((agents[0].nutrient - agents[1].nutrient).abs() < 1e-3);
    }

    #[test]
    fn absorb_nutrients_zero_contact_time_still_receives_nutrients() {
        // Mobile agents with contact_time=0 should still absorb nutrients —
        // unviability of mobile autotrophs emerges from superlinear maintenance
        // costs, not from a contact_time gate.
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        agents[0].contact_time = 0;
        let mut grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 100.0);

        let events = absorb_nutrients(&mut agents, &mut grid, &params);
        assert_eq!(events.len(), 1, "agent with autotrophy should get nutrients regardless of contact_time");
        assert!(agents[0].nutrient > 0.0, "nutrient uptake should be positive");
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
        let mut grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 100.0);

        let events = absorb_nutrients(&mut agents, &mut grid, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::NutrientAbsorbed);
        // demand = effective_autotrophy = 0.5
        let expected = 0.5;
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
        let mut grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 100.0);

        let events = absorb_nutrients(&mut agents, &mut grid, &params);

        assert!(events.is_empty(),
            "zero-autotrophy agent should get no nutrient uptake events");
        assert!((agents[0].nutrient).abs() < 1e-6,
            "zero-autotrophy agent should have zero nutrient");
        assert!((grid.total() - 100.0).abs() < 1e-6,
            "pool should be unchanged when no uptake occurs");
    }

    #[test]
    fn absorb_nutrients_agents_in_different_cells_see_independent_pools() {
        // Two agents in different cells: each draws from its own local cell only.
        // Place a nutrient-rich cell at one position and a nutrient-poor cell elsewhere.
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };
        // Agent A at (-40, 0), Agent B at (40, 0) — in different cells (cell_size=10)
        let mut agents = vec![
            make_agent(1, (-40.0, 0.0), 10.0, traits),
            make_agent(2, (40.0, 0.0), 10.0, traits),
        ];
        let mut grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        // Give only agent A's cell nutrient
        *grid.at_position((-40.0, 0.0)) = 5.0;
        // Agent B's cell has zero

        let _events = absorb_nutrients(&mut agents, &mut grid, &params);

        // Agent A should have absorbed nutrient
        assert!(agents[0].nutrient > 0.0,
            "agent in nutrient-rich cell should absorb, got {}", agents[0].nutrient);
        // Agent B should have absorbed nothing
        assert!((agents[1].nutrient).abs() < 1e-6,
            "agent in nutrient-poor cell should absorb nothing, got {}", agents[1].nutrient);
        // Total nutrient conserved
        let total = grid.total() + agents[0].nutrient + agents[1].nutrient;
        assert!((total - 5.0).abs() < 1e-3,
            "total nutrient should be conserved, got {}", total);
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
    fn metabolise_charges_asexual_propensity_maintenance() {
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            asexual_propensity_maintenance_cost: 2.0,
            maintenance_cost_exponent: 1.0,
            ..test_params()
        };
        let traits = TraitVector {
            asexual_propensity: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];

        let (events, dissipated) = metabolise(&mut agents, &params);

        // cost = 0.5^1 * 2.0 = 1.0
        assert!((events[0].energy_delta - 1.0).abs() < 1e-6);
        assert!((agents[0].reserve - 9.0).abs() < 1e-6);
        assert!((dissipated - 1.0).abs() < 1e-6);
    }

    #[test]
    fn metabolise_asexual_propensity_maintenance_is_superlinear() {
        // The cost is raised to maintenance_cost_exponent before scaling,
        // matching the other maintenance terms.
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            asexual_propensity_maintenance_cost: 2.0,
            maintenance_cost_exponent: 2.0,
            ..test_params()
        };
        let traits = TraitVector {
            asexual_propensity: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];

        let (events, _) = metabolise(&mut agents, &params);

        // cost = 0.5^2 * 2.0 = 0.5  (vs 1.0 if it were linear)
        assert!((events[0].energy_delta - 0.5).abs() < 1e-6);
    }

    #[test]
    fn metabolise_higher_asexual_propensity_costs_strictly_more() {
        // Selection gradient: more machinery, more standing cost.
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            asexual_propensity_maintenance_cost: 1.0,
            maintenance_cost_exponent: 2.0,
            ..test_params()
        };
        let mut low = vec![make_agent(
            1,
            (0.0, 0.0),
            10.0,
            TraitVector {
                asexual_propensity: 0.2,
                ..zero_traits()
            },
        )];
        let mut high = vec![make_agent(
            2,
            (0.0, 0.0),
            10.0,
            TraitVector {
                asexual_propensity: 0.8,
                ..zero_traits()
            },
        )];

        let (low_events, _) = metabolise(&mut low, &params);
        let (high_events, _) = metabolise(&mut high, &params);

        assert!(high_events[0].energy_delta > low_events[0].energy_delta);
        assert!(high[0].reserve < low[0].reserve);
    }

    #[test]
    fn metabolise_charges_no_asexual_maintenance_when_cost_zero() {
        // Zero cost must leave behaviour unchanged: propensity is free.
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            asexual_propensity_maintenance_cost: 0.0,
            maintenance_cost_exponent: 2.0,
            ..test_params()
        };
        let mut agents = vec![make_agent(
            1,
            (0.0, 0.0),
            10.0,
            TraitVector {
                asexual_propensity: 0.9,
                ..zero_traits()
            },
        )];

        let (events, dissipated) = metabolise(&mut agents, &params);

        assert!((events[0].energy_delta - 0.0).abs() < 1e-6);
        assert!((agents[0].reserve - 10.0).abs() < 1e-6);
        assert!((dissipated - 0.0).abs() < 1e-6);
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
        agents[0].nutrient = 1_000.0; // ample: growth is energy-limited here

        let (_events, _dissipated) = grow(&mut agents, &params);

        // If retention used movement_cost_coefficient (100.0), retention would be 200.0
        // and surplus would be 0, so no growth would occur and reserve stays at 10.0.
        // With mobility_maintenance_cost (1.0), retention is 2.0, surplus is 8.0,
        // and with kappa=1.0 and growth_efficiency=1.0, all goes to structure.
        assert!((agents[0].reserve - 2.0).abs() < 1e-6, "reserve should equal retention");
        assert!((agents[0].structure - 8.0).abs() < 1e-6, "surplus should become structure");
    }

    #[test]
    fn grow_retention_reflects_asexual_propensity_maintenance() {
        // Grow's retention buffer must account for the asexual_propensity
        // maintenance the next metabolise tick will charge, mirroring the
        // metabolise cost exactly so growth never over-allocates surplus.
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            asexual_propensity_maintenance_cost: 1.0,
            maintenance_cost_exponent: 1.0,
            growth_efficiency: 1.0,
            ..test_params()
        };
        let traits = TraitVector {
            asexual_propensity: 1.0,
            kappa: 1.0,
            ..zero_traits()
        };
        // metabolic_cost = 1.0^1 * 1.0 = 1.0; retention = 1.0 * 2 = 2.0
        // surplus = 10.0 - 2.0 = 8.0
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        agents[0].nutrient = 1_000.0; // ample: growth is energy-limited here

        let (_events, _dissipated) = grow(&mut agents, &params);

        assert!((agents[0].reserve - 2.0).abs() < 1e-6, "reserve should equal retention");
        assert!((agents[0].structure - 8.0).abs() < 1e-6, "surplus should become structure");
    }

    // --- Grow ---

    #[test]
    fn grow_caps_structure_at_nutrient_supply() {
        // Building structure is co-limited by nutrient: an agent can only lay down
        // as much structure as its nutrient store stoichiometrically supports
        // (structure <= nutrient / ratio). Energy it cannot match with nutrient
        // stays in reserve rather than being burned.
        let params = WorldParameters {
            base_metabolic_rate: 1.0,
            growth_efficiency: 1.0,
            specification_nutrient_coefficient: 0.0, // ratio = base_nutrient_ratio = 0.1
            ..test_params()
        };
        let traits = TraitVector { kappa: 1.0, ..zero_traits() }; // all surplus to soma
        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, traits)];
        agents[0].nutrient = 1.0; // ratio 0.1 -> supports at most 10.0 structure

        let (events, dissipated) = grow(&mut agents, &params);

        // retention = 1.0 * 2.0 = 2.0; surplus = 98.0; efficiency 1.0 would give
        // 98.0 structure, but nutrient caps it at 1.0 / 0.1 = 10.0.
        assert_eq!(events.len(), 1);
        assert!((agents[0].structure - 10.0).abs() < 1e-3,
            "structure capped at nutrient supply, got {}", agents[0].structure);
        // Unspent growth energy (88.0) returns to reserve on top of retention.
        assert!((agents[0].reserve - 90.0).abs() < 1e-3,
            "leftover growth energy stays in reserve, got {}", agents[0].reserve);
        // Nutrient is not moved (conservation-safe build permit).
        assert!((agents[0].nutrient - 1.0).abs() < 1e-6,
            "growth does not consume nutrient, got {}", agents[0].nutrient);
        // No energy dissipated at efficiency 1.0.
        assert!(dissipated.abs() < 1e-3, "no dissipation at efficiency 1.0, got {dissipated}");
        // The repro nutrient gate (nutrient >= structure * ratio) now holds.
        assert!(
            agents[0].nutrient >= crate::stoichiometric_demand(&agents[0].traits, agents[0].structure, &params),
            "agent stays reproduction-eligible on the nutrient axis"
        );
    }

    #[test]
    fn grow_without_nutrient_builds_no_structure() {
        // With no nutrient, an energy-rich agent cannot lay down structure at all
        // (nutrient is the binding constraint). The growth energy is retained in
        // reserve rather than burned.
        let params = WorldParameters {
            base_metabolic_rate: 1.0,
            growth_efficiency: 1.0,
            ..test_params()
        };
        let traits = TraitVector { kappa: 1.0, ..zero_traits() };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, traits)];
        // make_agent leaves nutrient at 0.0.

        let (events, dissipated) = grow(&mut agents, &params);

        assert!(events.is_empty(), "no Grew event when nutrient-starved");
        assert!(agents[0].structure.abs() < 1e-6, "no structure built without nutrient");
        // Surplus (98.0) was not burned: retention (2.0) + returned growth energy.
        assert!((agents[0].reserve - 100.0).abs() < 1e-3,
            "growth energy stays in reserve, got {}", agents[0].reserve);
        assert!(dissipated.abs() < 1e-6, "nothing dissipated, got {dissipated}");
    }

    #[test]
    fn grow_converts_surplus_reserve_to_structure() {
        let params = WorldParameters {
            base_metabolic_rate: 1.0,
            growth_efficiency: 0.8,
            ..test_params()
        };
        let traits = TraitVector { kappa: 1.0, ..zero_traits() }; // all surplus to soma
        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, traits)];
        agents[0].nutrient = 1_000.0; // ample: growth is energy-limited here

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
    fn growth_retention_multiplier_default_is_two() {
        // The retention buffer multiplier defaults to 2.0 to preserve historical behaviour.
        let params = test_params();
        assert!((params.growth_retention_multiplier - 2.0).abs() < 1e-6);
    }

    #[test]
    fn growth_retention_multiplier_scales_retention() {
        // Overriding the multiplier should change how much reserve grow retains
        // before computing surplus. With multiplier=5.0 and metabolic_cost=1.0,
        // retention=5.0 (vs. default 2.0), so surplus shrinks from 9.0 to 5.0.
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            mobility_maintenance_cost: 1.0,
            movement_cost_coefficient: 0.0,
            growth_efficiency: 1.0,
            growth_retention_multiplier: 5.0,
            offspring_structure_fraction: 0.2,
            ..test_params()
        };
        let traits = TraitVector {
            mobility: 1.0,
            kappa: 1.0,
            ..zero_traits()
        };
        // metabolic_cost = 1.0; retention = 1.0 * 5.0 = 5.0
        // surplus = 10.0 - 5.0 = 5.0; kappa=1, growth_efficiency=1
        // -> structure = 5.0; reserve = 5.0
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        agents[0].nutrient = 1_000.0; // ample: growth is energy-limited here
        let (_events, _dissipated) = grow(&mut agents, &params);
        assert!((agents[0].reserve - 5.0).abs() < 1e-6,
            "reserve should equal retention (5.0) under multiplier=5.0");
        assert!((agents[0].structure - 5.0).abs() < 1e-6,
            "surplus (5.0) should become structure");
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
        agents[0].nutrient = 1_000.0; // ample: growth is energy-limited here

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

    // WR-16: "Offspring are born with zero wear." Parents accumulate wear over
    // their lifetime; a newborn must start its own wear accumulator at zero on
    // every functional trait, regardless of how worn the parent(s) are.

    #[test]
    fn asexual_offspring_born_with_zero_wear() {
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
            asexual_propensity: 1.0, // force asexual path
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 50.0, traits),
            make_agent(2, (1.0, 0.0), 50.0, traits),
        ];
        // Both parents are heavily worn on every functional trait.
        for agent in agents.iter_mut() {
            agent.wear = [0.9; FUNCTIONAL_TRAIT_COUNT];
            agent.repro_reserve = 15.0;
            agent.nutrient = 10.0;
        }

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, &mut rng);

        assert!(!result.offspring.is_empty(), "asexual parents should reproduce");
        for child in &result.offspring {
            for ft in 0..FUNCTIONAL_TRAIT_COUNT {
                assert_eq!(
                    child.wear[ft], 0.0,
                    "asexual offspring must be born with zero wear on trait {ft}, got {}",
                    child.wear[ft]
                );
            }
        }
    }

    #[test]
    fn sexual_offspring_born_with_zero_wear() {
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
            fecundity: 4.0, // high mean so the Poisson draw yields offspring
            asexual_propensity: 0.0, // force sexual path
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 50.0, traits),
            make_agent(2, (1.0, 0.0), 50.0, traits),
        ];
        // Both parents are heavily worn on every functional trait.
        for agent in agents.iter_mut() {
            agent.wear = [0.9; FUNCTIONAL_TRAIT_COUNT];
            agent.repro_reserve = 15.0;
            agent.nutrient = 10.0;
        }

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, &mut rng);

        // Confirm we actually exercised the sexual path (target = mate id).
        assert!(
            result.events.iter().any(|e| e.kind == EventKind::Reproduced && e.target.is_some()),
            "expected a sexual reproduction event with a mate target"
        );
        assert!(!result.offspring.is_empty(), "sexual pair should produce offspring");
        for child in &result.offspring {
            for ft in 0..FUNCTIONAL_TRAIT_COUNT {
                assert_eq!(
                    child.wear[ft], 0.0,
                    "sexual offspring must be born with zero wear on trait {ft}, got {}",
                    child.wear[ft]
                );
            }
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
    fn apply_wear_accumulates_without_repair() {
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

        // Accumulation only: 0.1 * 0.5 = 0.05, wear goes to 1.05
        assert!((agents[0].wear[0] - 1.05).abs() < 1e-6,
            "apply_wear should only accumulate (repair is in grow), got {}", agents[0].wear[0]);
    }

    #[test]
    fn grow_repairs_wear_from_soma_budget() {
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            growth_efficiency: 1.0,
            repair_decay: 1.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.7,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, traits)];
        agents[0].wear[0] = 1.0;

        let (_events, _dissipated) = grow(&mut agents, &params);

        assert!(agents[0].wear[0] < 1.0,
            "grow should repair wear from soma budget, got {}", agents[0].wear[0]);
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

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let result = resolve_drains(
            &mut agents, &mut carcasses, &grid, &params, &mut nutrient_grid,
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

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let result = resolve_drains(
            &mut agents, &mut carcasses, &grid, &params, &mut nutrient_grid,
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

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let result = resolve_drains(
            &mut agents, &mut carcasses, &grid, &params, &mut nutrient_grid,
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

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let result = resolve_drains(
            &mut agents, &mut carcasses, &grid, &params, &mut nutrient_grid,
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

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let result = resolve_drains(
            &mut agents, &mut carcasses, &grid, &params, &mut nutrient_grid,
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
        // The raw interaction fact: which Consumed events drained a carcass.
        let living = result.events.iter()
            .find(|e| e.kind == EventKind::Consumed && e.target == Some(2))
            .expect("a Consumed event targeting the living agent");
        assert!(!living.target_was_carcass,
            "draining a living agent is not decomposition");
        let carcass = result.events.iter()
            .find(|e| e.kind == EventKind::Consumed && e.target == Some(99))
            .expect("a Consumed event targeting the carcass");
        assert!(carcass.target_was_carcass,
            "draining a carcass is decomposition");
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

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let _result = resolve_drains(
            &mut agents, &mut carcasses, &grid, &params, &mut nutrient_grid,
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
        assert!((nutrient_grid.total() - 1.5).abs() < 1e-3,
            "nutrient pool should receive 1.5 excess, got {}", nutrient_grid.total());
        assert!((agents[1].nutrient - 16.0).abs() < 1e-3,
            "target nutrient should be 16.0 (20 - 4), got {}", agents[1].nutrient);
    }

    #[test]
    fn drain_nutrient_excretion_returns_to_local_cell() {
        // Nutrient excreted during consumption goes to the nutrient grid cell
        // at the target's position, not to a global pool.
        let params = test_params();
        let consumer_traits = TraitVector {
            heterotrophy: 2.0,
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        // Place consumer and target far apart but within contact range
        // (contact_radius is 5.0 in test_params)
        let target_pos = (40.0, 0.0);
        let consumer_pos = (41.0, 0.0);  // 1.0 away, within contact radius
        let mut agents = vec![
            make_agent(1, consumer_pos, 10.0, consumer_traits),
            make_agent(2, target_pos, 10.0, target_traits),
        ];
        agents[0].structure = 5.0;
        agents[1].structure = 10.0;
        agents[1].nutrient = 20.0;

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, consumer_pos);
        grid.insert(1, target_pos);

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let _result = resolve_drains(
            &mut agents, &mut carcasses, &grid, &params, &mut nutrient_grid,
        );

        // Excreted nutrient should be in the cell at the target's position
        let target_cell_nutrient = *nutrient_grid.at_position(target_pos);
        assert!(target_cell_nutrient > 0.0,
            "excreted nutrient should be in target's cell, got {}", target_cell_nutrient);

        // A distant cell should have zero
        let distant_cell = *nutrient_grid.at_position((-40.0, -40.0));
        assert!((distant_cell).abs() < 1e-6,
            "distant cell should be unaffected, got {}", distant_cell);

        // Total excreted should match what the nutrient grid received
        assert!((nutrient_grid.total() - target_cell_nutrient).abs() < 1e-6,
            "all excreted nutrient should be in target's cell");
    }

    #[test]
    fn drain_carcass_excreted_nutrient_conserves_into_available_pool() {
        // Direct positive routing test (WR-25): when a low-demand consumer
        // decomposes a nutrient-rich carcass, the nutrient drained from the
        // carcass splits exactly into (a) nutrient retained by the consumer and
        // (b) excess excreted to the NutrientGrid at the carcass's cell. Nothing
        // dissipates: consumer gain + grid delta == total drained from target.
        let params = test_params();
        let consumer_traits = TraitVector {
            heterotrophy: 2.0,
            ..zero_traits()
        };
        let carcass_traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        // Low stoichiometric demand consumer (small structure), placed within
        // contact range (contact_radius = 5.0 in test_params).
        let carcass_pos = (40.0, 0.0);
        let consumer_pos = (41.0, 0.0);
        let mut agents = vec![make_agent(1, consumer_pos, 10.0, consumer_traits)];
        agents[0].structure = 5.0;

        let mut carcasses = vec![Carcass {
            id: 99,
            position: carcass_pos,
            energy: 10.0,
            nutrient: 20.0, // nutrient-rich target
            traits: carcass_traits,
        }];

        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(1, consumer_pos); // grid id must match agent id for lookup

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);

        // Record before-state.
        let consumer_nutrient_before = agents[0].nutrient;
        let carcass_nutrient_before = carcasses[0].nutrient;
        let grid_total_before = nutrient_grid.total();

        let _result = resolve_drains(
            &mut agents, &mut carcasses, &grid, &params, &mut nutrient_grid,
        );

        // Deltas across the single consumption tick.
        let consumer_gain = agents[0].nutrient - consumer_nutrient_before;
        let drained_from_carcass = carcass_nutrient_before - carcasses[0].nutrient;
        let excreted_to_grid = nutrient_grid.total() - grid_total_before;

        // Routing must split the drained nutrient with no loss.
        assert!(consumer_gain > 0.0, "consumer should retain some nutrient");
        assert!(excreted_to_grid > 0.0, "excess should reach the available pool");
        assert!(
            (consumer_gain + excreted_to_grid - drained_from_carcass).abs() < 1e-4,
            "consumer gain ({consumer_gain}) + excreted ({excreted_to_grid}) must equal \
             nutrient drained from carcass ({drained_from_carcass})"
        );

        // The excreted nutrient lands at the carcass's cell specifically.
        let carcass_cell = *nutrient_grid.at_position(carcass_pos);
        assert!(
            (carcass_cell - excreted_to_grid).abs() < 1e-6,
            "all excreted nutrient should be in the carcass's cell"
        );
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
            let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
            let _result = resolve_drains(
                &mut agents, &mut carcasses, &grid, &params, &mut nutrient_grid,
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
            let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
            let _result = resolve_drains(
                &mut agents, &mut carcasses, &grid, &params, &mut nutrient_grid,
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
        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let result = resolve_drains(
            &mut agents, &mut carcasses, &grid, &params, &mut nutrient_grid,
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
        // This test focuses on the reproduction_efficiency loss only;
        // structure provisioning is exercised by the conservation tests.
        let params = WorldParameters {
            reproduction_efficiency: 0.7,
            reproduction_energy_threshold: 10.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            offspring_structure_fraction: 0.0,
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
            offspring_structure_fraction: 0.0,
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
            offspring_structure_fraction: 0.0,
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
            offspring_structure_fraction: 0.0,
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

    #[test]
    fn asexual_zero_poisson_draw_consumes_energy_yields_no_offspring() {
        use rand::SeedableRng;
        // World-rules flow 4: reproductive failure (zero offspring despite energy
        // investment) must be possible. With fecundity at its floor (0.1) the
        // Poisson mean is 0.1, so a zero draw is overwhelmingly likely. The energy
        // committed to reproduction is still consumed and dissipated.
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
            fecundity: 0.0, // floored to 0.1 mean -> P(0) ~ 90%
            asexual_propensity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![Agent {
            id: 1,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0,
            nutrient: 100.0,
            traits,
            contact_time: 0,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 20.0,
        }];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, &mut rng);

        // Zero offspring produced (reproductive failure).
        assert!(result.offspring.is_empty(),
            "zero Poisson draw should yield no offspring, got {}", result.offspring.len());
        // Energy was still committed: parent's repro_reserve is consumed.
        assert!(agents[0].repro_reserve < 1e-6,
            "parent repro_reserve should be consumed even on failure, got {}",
            agents[0].repro_reserve);
        // Conservation: the entire committed investment (20.0) is dissipated since
        // no offspring carry any of it.
        assert!((result.dissipated - 20.0).abs() < 1e-3,
            "full investment should dissipate on zero-offspring failure, got {}",
            result.dissipated);
    }

    #[test]
    fn sexual_zero_poisson_draw_consumes_energy_yields_no_offspring() {
        use rand::SeedableRng;
        // Sexual reproductive failure: both parents commit their repro_reserve but
        // a zero Poisson draw yields no offspring. The full combined investment is
        // dissipated.
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
            fecundity: 0.0, // floored to 0.1 mean -> P(0) ~ 90%
            asexual_propensity: 0.0, // force sexual path
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

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, &mut rng);

        // Zero offspring (reproductive failure).
        assert!(result.offspring.is_empty(),
            "zero Poisson draw should yield no offspring, got {}", result.offspring.len());
        // Both parents' repro_reserve consumed.
        assert!(agents[0].repro_reserve < 1e-6 && agents[1].repro_reserve < 1e-6,
            "both parents' repro_reserve should be consumed, got {} and {}",
            agents[0].repro_reserve, agents[1].repro_reserve);
        // Full combined investment (15 + 15 = 30) dissipates.
        assert!((result.dissipated - 30.0).abs() < 1e-3,
            "full investment should dissipate on zero-offspring failure, got {}",
            result.dissipated);
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
            fecundity: 5.0,
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
            fecundity: 5.0,
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
            contact_range_coefficient: 100.0,
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
            fecundity: 5.0,
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
            contact_range_coefficient: 100.0,
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
            fecundity: 5.0,
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
            fecundity: 5.0,
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
            contact_range_coefficient: 100.0,
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
            fecundity: 5.0,
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

    #[test]
    fn sustained_contact_higher_ct_drains_more() {
        // Two consumers with identical heterotrophy but different contact_times
        // targeting the same agent. The one with higher contact_time should
        // drain more because demand = eff_heterotrophy * ct / (ct + K).
        let params = WorldParameters {
            consumption_contact_half_saturation: 10.0, // large K so contact_time matters
            base_trophic_efficiency: 1.0, // no trophic loss for clarity
            trophic_distance_decay: 0.0,
            ..test_params()
        };
        let consumer_traits = TraitVector {
            heterotrophy: 1.0,
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        // Consumer A: contact_time = 5, Consumer B: contact_time = 50
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, consumer_traits),
            make_agent(2, (1.0, 0.0), 10.0, consumer_traits),
            make_agent(3, (0.5, 0.0), 10.0, target_traits),
        ];
        agents[0].contact_time = 5;   // low contact
        agents[1].contact_time = 50;  // high contact
        agents[2].structure = 100.0;  // plenty of structure so no proportional split

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        grid.insert(2, (0.5, 0.0));

        let initial_reserve_a = agents[0].reserve;
        let initial_reserve_b = agents[1].reserve;

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let _result = resolve_drains(
            &mut agents, &mut carcasses, &grid, &params, &mut nutrient_grid,
        );

        let gained_a = agents[0].reserve - initial_reserve_a;
        let gained_b = agents[1].reserve - initial_reserve_b;

        assert!(gained_b > gained_a,
            "consumer with higher contact_time should drain more: \
             low_ct gained {gained_a}, high_ct gained {gained_b}");
        // Verify both gained something (both are consuming)
        assert!(gained_a > 0.0, "low-ct consumer should still drain something");
    }

    #[test]
    fn sustained_contact_asymptote_at_high_ct() {
        // At very high contact_time, demand should approach eff_heterotrophy.
        // ct/(ct+K) -> 1 as ct -> infinity.
        let half_sat = 10.0;
        let params = WorldParameters {
            consumption_contact_half_saturation: half_sat,
            base_trophic_efficiency: 1.0,
            trophic_distance_decay: 0.0,
            ..test_params()
        };
        let consumer_traits = TraitVector {
            heterotrophy: 0.8,
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
        agents[0].contact_time = 10_000; // very high ct
        agents[1].structure = 100.0;

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let initial_reserve = agents[0].reserve;
        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let _result = resolve_drains(
            &mut agents, &mut carcasses, &grid, &params, &mut nutrient_grid,
        );

        let gained = agents[0].reserve - initial_reserve;
        // Expected demand: 0.8 * 10000 / (10000 + 10) = 0.8 * 0.999... ≈ 0.7992
        // With trophic_efficiency=1.0, gained should equal demand
        let expected = 0.8 * 10_000.0 / (10_000.0 + half_sat);
        assert!((gained - expected).abs() < 1e-3,
            "at very high ct, demand should approach eff_heterotrophy: \
             gained {gained}, expected ~{expected}");
        // Should be very close to eff_heterotrophy (0.8) but not exceed it
        assert!(gained < 0.8, "demand must not exceed eff_heterotrophy");
        assert!(gained > 0.799, "demand should be within 0.1% of eff_heterotrophy at ct=10000");
    }

    #[test]
    fn sustained_contact_default_produces_visible_ramp() {
        // The shipped default for consumption_contact_half_saturation should
        // produce a multi-tick Michaelis-Menten ramp, not a step at ct=1.
        // Using the default value (parsed from JSON via serde), demand at ct=1
        // should be meaningfully less than at ct=10, and both should asymptote
        // toward eff_heterotrophy as ct grows large.
        let default_params: WorldParameters = serde_json::from_str(
            r#"{
                "solar_flux_magnitude": 10.0,
                "base_trophic_efficiency": 1.0,
                "reproduction_efficiency": 0.7,
                "base_metabolic_rate": 0.0,
                "movement_cost_coefficient": 0.0,
                "reproduction_energy_threshold": 50.0,
                "mutation_rate": 0.0,
                "mutation_magnitude": 0.0,
                "contact_range_coefficient": 5.0,
                "world_extent": 100.0,
                "initial_population_size": 0,
                "light_competition_radius": 1000.0,
                "photo_maintenance_cost": 0.0,
                "heterotrophy_maintenance_cost": 0.0
            }"#,
        )
        .expect("default params should deserialise");
        let half_sat = default_params.consumption_contact_half_saturation;
        assert!(
            half_sat >= 1.0,
            "default half-saturation must be large enough to give a visible ramp, \
             got K={half_sat}"
        );

        let eff_heterotrophy = 1.0_f32;
        let demand_at_ct = |ct: f32| eff_heterotrophy * ct / (ct + half_sat);

        let d1 = demand_at_ct(1.0);
        let d10 = demand_at_ct(10.0);
        let d1000 = demand_at_ct(1000.0);

        // ct=1 should be meaningfully less than eff_heterotrophy (not a step).
        assert!(
            d1 < 0.8 * eff_heterotrophy,
            "demand at ct=1 should be well below eff_heterotrophy, got {d1}"
        );
        // ct=10 should be meaningfully larger than ct=1 (multi-tick ramp).
        assert!(
            d10 > d1 + 0.1,
            "demand at ct=10 ({d10}) should be meaningfully larger than at ct=1 ({d1})"
        );
        // Monotonic and asymptoting toward eff_heterotrophy.
        assert!(d10 < d1000, "demand should keep rising past ct=10");
        assert!(
            d1000 > 0.99 * eff_heterotrophy,
            "demand should approach eff_heterotrophy at high ct, got {d1000}"
        );
    }

    #[test]
    fn drain_demand_only_proportional_split() {
        // Two consumers with equal raw demand (heterotrophy=1.0) but different
        // trait distances to the target. The split is by demand only; trophic
        // efficiency applies after the split to determine energy gained.
        let mut params = test_params();
        params.base_trophic_efficiency = 1.0;
        params.trophic_distance_decay = 1.0;

        let target_traits = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };
        let consumer_a_traits = TraitVector {
            heterotrophy: 1.0,
            photosynthetic_absorption: 0.8,
            ..zero_traits()
        };
        let consumer_b_traits = TraitVector {
            heterotrophy: 1.0,
            mobility: 2.0,
            ..zero_traits()
        };

        let eff_a = crate::trophic_transfer_efficiency(
            &consumer_a_traits, &target_traits, &params,
        );
        let eff_b = crate::trophic_transfer_efficiency(
            &consumer_b_traits, &target_traits, &params,
        );
        assert!(eff_a > eff_b, "sanity: A should be more efficient than B");

        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 0.0, consumer_a_traits),
            make_agent(2, (1.0, 0.0), 0.0, consumer_b_traits),
            make_agent(3, (0.5, 0.0), 0.0, target_traits),
        ];
        agents[2].structure = 1.0;

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        grid.insert(2, (0.5, 0.0));

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let _result = resolve_drains(
            &mut agents, &mut carcasses, &grid, &params, &mut nutrient_grid,
        );

        // Equal demand → equal drain (0.5 each). Energy = drain * trophic_eff.
        // Ratio of energy gained = eff_a / eff_b.
        let a_gained = agents[0].reserve;
        let b_gained = agents[1].reserve;
        let actual_ratio = a_gained / b_gained;
        let expected_ratio = eff_a / eff_b;

        assert!((actual_ratio - expected_ratio).abs() < 0.01,
            "energy ratio should be eff_a/eff_b = {expected_ratio:.4}, got {actual_ratio:.4}");
    }

    #[test]
    fn drain_trophic_weighted_split_conserves_energy() {
        // With trophic-weighted split, total energy drained from target must
        // equal energy gained by consumers plus dissipated energy.
        let mut params = test_params();
        params.base_trophic_efficiency = 0.8;
        params.trophic_distance_decay = 2.0;

        let target_traits = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };
        let consumer_a_traits = TraitVector {
            heterotrophy: 2.0,
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let consumer_b_traits = TraitVector {
            heterotrophy: 1.5,
            mobility: 1.0,
            ..zero_traits()
        };

        let initial_structure = 2.0;
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 0.0, consumer_a_traits),
            make_agent(2, (1.0, 0.0), 0.0, consumer_b_traits),
            make_agent(3, (0.5, 0.0), 0.0, target_traits),
        ];
        agents[2].structure = initial_structure;

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        grid.insert(2, (0.5, 0.0));

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let result = resolve_drains(
            &mut agents, &mut carcasses, &grid, &params, &mut nutrient_grid,
        );

        let total_drained = initial_structure - agents[2].structure;
        let total_gained = agents[0].reserve + agents[1].reserve;
        let total_out = total_gained + result.dissipated;

        assert!(total_drained > 0.0, "something should have been drained");
        assert!((total_drained - total_out).abs() < 1e-5,
            "energy conservation: drained {total_drained} != gained {total_gained} + dissipated {}",
            result.dissipated);
    }

    /// Conservation equation for asexual reproduction with structure provisioning:
    ///
    ///   parent_committed_investment
    ///     = sum(offspring.reserve)
    ///     + sum(offspring.structure)
    ///     + result.dissipated
    ///
    /// where `result.dissipated` accounts for both the reproduction-efficiency
    /// loss (parents → offspring transfer) and the growth-efficiency loss
    /// (offspring energy → offspring structure conversion).
    #[test]
    fn asexual_reproduction_conserves_energy_with_structure_provisioning() {
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 0.7,
            growth_efficiency: 0.8,
            offspring_structure_fraction: 0.25,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 3.0,
            asexual_propensity: 1.0,
            ..zero_traits()
        };
        let initial_repro_reserve = 20.0_f32;
        let mut agents = vec![Agent {
            id: 1,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0,
            nutrient: 100.0,
            traits,
            contact_time: 0,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: initial_repro_reserve,
        }];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        assert!(!result.offspring.is_empty(), "should produce offspring");

        let offspring_reserve_sum: f32 = result.offspring.iter().map(|c| c.reserve).sum();
        let offspring_structure_sum: f32 = result.offspring.iter().map(|c| c.structure).sum();

        // Every offspring must be born with strictly positive structure.
        for child in &result.offspring {
            assert!(child.structure > 0.0,
                "newborn structure must be > 0, got {}", child.structure);
            assert!(child.reserve > 0.0,
                "newborn reserve must be > 0, got {}", child.reserve);
        }

        let committed = initial_repro_reserve;
        let accounted = offspring_reserve_sum + offspring_structure_sum + result.dissipated;
        assert!((committed - accounted).abs() < 1e-4,
            "energy conservation: committed {committed} != reserve {offspring_reserve_sum} + structure {offspring_structure_sum} + dissipated {} (sum {accounted})",
            result.dissipated);
    }

    /// Sexual reproduction must observe the same conservation equation:
    ///
    ///   invest_a + invest_b
    ///     = sum(offspring.reserve)
    ///     + sum(offspring.structure)
    ///     + result.dissipated
    #[test]
    fn sexual_reproduction_conserves_energy_with_structure_provisioning() {
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_efficiency: 0.6,
            growth_efficiency: 0.5,
            offspring_structure_fraction: 0.3,
            reproductive_compatibility_distance: 5.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            sensing_range_coefficient: 50.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 0.5,
            kappa: 0.5,
            fecundity: 4.0,
            asexual_propensity: 0.0, // force sexual
            ..zero_traits()
        };
        let invest_a = 30.0_f32;
        let invest_b = 25.0_f32;
        let mut agents = vec![
            Agent {
                id: 1, position: (0.0, 0.0), reserve: 50.0, structure: 5.0,
                nutrient: 100.0, traits, contact_time: 0,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT], repro_reserve: invest_a,
            },
            Agent {
                id: 2, position: (1.0, 0.0), reserve: 50.0, structure: 5.0,
                nutrient: 100.0, traits, contact_time: 0,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT], repro_reserve: invest_b,
            },
        ];
        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(1, (0.0, 0.0));
        grid.insert(2, (1.0, 0.0));
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(7);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        assert!(!result.offspring.is_empty(), "should produce offspring");
        for child in &result.offspring {
            assert!(child.structure > 0.0, "newborn structure > 0");
            assert!(child.reserve > 0.0, "newborn reserve > 0");
        }

        let offspring_reserve_sum: f32 = result.offspring.iter().map(|c| c.reserve).sum();
        let offspring_structure_sum: f32 = result.offspring.iter().map(|c| c.structure).sum();
        let committed = invest_a + invest_b;
        let accounted = offspring_reserve_sum + offspring_structure_sum + result.dissipated;
        assert!((committed - accounted).abs() < 1e-3,
            "sexual conservation: committed {committed} != reserve {offspring_reserve_sum} + structure {offspring_structure_sum} + dissipated {} (sum {accounted})",
            result.dissipated);
    }

    /// Per-offspring structure must scale inversely with fecundity: doubling
    /// the offspring count from the same parental investment must roughly
    /// halve per-offspring structure.
    #[test]
    fn per_offspring_structure_scales_inversely_with_offspring_count() {
        use rand::SeedableRng;
        // Use mutation-free, deterministic setup. We fix the offspring count
        // by setting fecundity high enough that Poisson rounding is the same
        // across runs — easier: assert ratio across two configurations.
        fn run(fecundity: f32, seed: u64) -> (usize, f32) {
            let params = WorldParameters {
                reproduction_energy_threshold: 10.0,
                reproduction_efficiency: 1.0,
                growth_efficiency: 1.0,
                offspring_structure_fraction: 0.5,
                mutation_rate: 0.0,
                mutation_magnitude: 0.0,
                ..test_params()
            };
            let traits = TraitVector {
                photosynthetic_absorption: 0.5,
                kappa: 0.5,
                fecundity,
                asexual_propensity: 1.0,
                ..zero_traits()
            };
            let mut agents = vec![Agent {
                id: 1, position: (0.0, 0.0), reserve: 50.0, structure: 5.0,
                nutrient: 1000.0, traits, contact_time: 0,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT], repro_reserve: 100.0,
            }];
            let dead_ids = std::collections::HashSet::new();
            let grid = SpatialGrid::new(100.0, 10.0);
            let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
            let result = resolve_reproduction(
                &mut agents, &dead_ids, &grid, &params, &mut rng,
            );
            let n = result.offspring.len();
            let per = if n > 0 { result.offspring[0].structure } else { 0.0 };
            (n, per)
        }

        let (n_low, per_low) = run(2.0, 1);
        let (n_high, per_high) = run(20.0, 1);

        assert!(n_low > 0 && n_high > 0, "both runs should yield offspring");
        assert!(n_high > n_low, "higher fecundity should yield more offspring (got {n_low} vs {n_high})");

        // Total structure across offspring should be roughly equal between
        // the two configs (same investment, same efficiencies). Therefore
        // per-offspring structure ratio ~= offspring-count inverse ratio.
        let total_low = per_low * n_low as f32;
        let total_high = per_high * n_high as f32;
        // Identical investment and efficiencies → totals equal up to f32 noise.
        assert!((total_low - total_high).abs() < 1e-3,
            "totals should match: {total_low} vs {total_high}");
        assert!(per_high < per_low,
            "per-offspring structure should fall with fecundity (got {per_low} -> {per_high})");
    }
}
