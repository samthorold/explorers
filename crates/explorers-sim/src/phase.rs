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

    // Compute each agent's demand
    let demands: Vec<f32> = agents
        .iter()
        .map(|a| {
            let ct = a.contact_time as f32;
            let eff_absorption = a.effective_trait_with_steepness(2, k);
            eff_absorption * ct / (ct + k_half)
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

/// Metabolise: fixed costs only. Base rate + trait maintenance + somatic
/// maintenance + structure maintenance. No movement cost (that's in the
/// movement phase).
pub fn metabolise(
    agents: &mut [Agent],
    params: &WorldParameters,
) -> (Vec<Event>, f32) {
    let mut events = Vec::new();
    let mut total_dissipated = 0.0_f32;

    for agent in agents.iter_mut() {
        let cost = params.base_metabolic_rate
            + agent.traits.sensing_range * params.sensing_cost_coefficient
            + agent.traits.photosynthetic_absorption * params.photo_maintenance_cost
            + agent.traits.heterotrophy * params.heterotrophy_maintenance_cost
            + agent.traits.nutrient_absorption * params.nutrient_absorption_maintenance_cost
            + agent.traits.somatic_maintenance * params.somatic_maintenance_cost_coefficient
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

/// Grow: convert reserve surplus above metabolic retention to structure
/// at growth_efficiency rate. The conversion is lossy.
pub fn grow(
    agents: &mut [Agent],
    params: &WorldParameters,
) -> (Vec<Event>, f32) {
    let mut events = Vec::new();
    let mut total_dissipated = 0.0_f32;
    let efficiency = params.growth_efficiency;
    if efficiency <= 0.0 {
        return (events, total_dissipated);
    }

    for agent in agents.iter_mut() {
        if agent.reserve <= 0.0 {
            continue;
        }
        // Retain enough reserve for next tick's metabolism
        let metabolic_cost = params.base_metabolic_rate
            + agent.traits.sensing_range * params.sensing_cost_coefficient
            + agent.traits.photosynthetic_absorption * params.photo_maintenance_cost
            + agent.traits.heterotrophy * params.heterotrophy_maintenance_cost
            + agent.traits.nutrient_absorption * params.nutrient_absorption_maintenance_cost
            + agent.traits.somatic_maintenance * params.somatic_maintenance_cost_coefficient
            + agent.structure * params.structure_maintenance_coefficient;
        let retention = metabolic_cost * 2.0;
        let surplus = (agent.reserve - retention).max(0.0);
        if surplus <= 0.0 {
            continue;
        }

        let to_structure = surplus * efficiency;
        let dissipated = surplus - to_structure;
        agent.reserve -= surplus;
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
    }
    (events, total_dissipated)
}

/// Apply wear: baseline + use-dependent accumulation per functional trait.
/// Somatic repair reduces wear.
pub fn apply_wear(
    agents: &mut [Agent],
    params: &WorldParameters,
) -> Vec<Event> {
    let mut events = Vec::new();
    let baseline_rate = params.wear_rate;
    let _use_rate = params.use_wear_rate;
    let decay = params.repair_decay;

    for agent in agents.iter_mut() {
        let mut total_wear_delta = 0.0_f32;

        // Accumulate wear
        for ft in 0..FUNCTIONAL_TRAIT_COUNT {
            let nominal = agent.traits.get(FUNCTIONAL_TRAIT_INDICES[ft]);
            let accumulation = baseline_rate * nominal.max(0.0);
            // Use-dependent wear is deferred until coordinated phases exist
            // (no throughput data available for autonomous-only tick)
            agent.wear[ft] += accumulation;
            total_wear_delta += accumulation;
        }

        // Somatic repair
        let base_repair = agent.traits.somatic_maintenance;
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

            let efficiency = params.consumption_efficiency;
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
                let consumer_demand = crate::stoichiometric_demand(&agents[consumer_idx].traits);
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

            let efficiency = params.decomposition_efficiency;
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

                let consumer_demand = crate::stoichiometric_demand(&agents[consumer_idx].traits);
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
pub fn check_death_thresholds(
    agents: &mut [Agent],
    _params: &WorldParameters,
) -> (Vec<Event>, Vec<Carcass>) {
    let mut events = Vec::new();
    let mut carcasses = Vec::new();

    for agent in agents.iter_mut() {
        let threshold = crate::death_threshold(&agent.traits);
        let dies = agent.reserve <= 0.0
            || (agent.structure > 0.0 && agent.structure < threshold);

        if dies {
            let carcass_energy = agent.structure.max(0.0);
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
            });
            // Mark for removal by setting reserve to 0
            agent.reserve = 0.0;
        }
    }
    (events, carcasses)
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
        let eff_mobility = agents[i].effective_trait_with_steepness(3, k);
        let eff_chemotaxis = agents[i].effective_trait_with_steepness(4, k);
        let eff_sensing = agents[i].effective_trait_with_steepness(5, k);
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

        // Deduct energy cost
        let cost = distance * params.movement_cost_coefficient;
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

/// Resolve reproduction: coordinated pass 2. Pairs eligible agents by closest
/// trait-space distance within spatial proximity, invests energy from both parents,
/// produces offspring with crossover traits and Gaussian mutation.
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

    // Build eligible set: alive, above energy threshold, sufficient nutrient.
    // Grid keys are slice indices (matching World::step grid construction).
    let eligible: std::collections::HashSet<usize> = agents
        .iter()
        .enumerate()
        .filter(|(_, a)| {
            !dead_ids.contains(&a.id)
                && a.reserve >= params.reproduction_energy_threshold
                && a.nutrient >= crate::stoichiometric_demand(&a.traits)
        })
        .map(|(i, _)| i)
        .collect();

    // For each eligible agent, find closest compatible mate within sensing range.
    // Grid keys are slice indices, so neighbor_id as usize gives the agent index.
    let mut pair_candidates: Vec<(usize, usize, f32)> = Vec::new();

    for &i in &eligible {
        let agent_i = &agents[i];
        let sensing = agent_i.traits.sensing_range;
        let selectivity = agent_i.traits.mate_selectivity;

        let nearby = grid.query_radius(agent_i.position, sensing);
        let mut best: Option<(usize, f32)> = None;
        let mut seen = std::collections::HashSet::new();
        seen.insert(i); // exclude self

        for neighbor_id in nearby {
            let j = neighbor_id as usize;
            if !seen.insert(j) {
                continue;
            }
            if j >= agents.len() || !eligible.contains(&j) {
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
            // Check mate selectivity: trait distance must be within selectivity
            if selectivity > 0.0 && trait_dist > selectivity {
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

    // Next agent ID: find max existing ID + 1
    let mut next_id = agents.iter().map(|a| a.id).max().unwrap_or(0) + 1;

    // Resolve each pair
    for (a_idx, b_idx) in &final_pairs {
        // Copy needed values before mutating
        let a_traits = agents[*a_idx].traits;
        let b_traits = agents[*b_idx].traits;
        let a_reserve = agents[*a_idx].reserve;
        let b_reserve = agents[*b_idx].reserve;
        let a_nutrient = agents[*a_idx].nutrient;
        let b_nutrient = agents[*b_idx].nutrient;
        let a_pos = agents[*a_idx].position;
        let b_pos = agents[*b_idx].position;
        let a_contact = agents[*a_idx].contact_time;
        let b_contact = agents[*b_idx].contact_time;
        let a_id = agents[*a_idx].id;
        let b_id = agents[*b_idx].id;

        // Investment: each parent invests their reproductive_investment trait, capped at reserve
        let invest_a = a_traits.reproductive_investment.min(a_reserve).max(0.0);
        let invest_b = b_traits.reproductive_investment.min(b_reserve).max(0.0);
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

        // Nutrient donation: each parent donates proportional to investment
        let nutrient_a = a_nutrient * (invest_a / a_reserve.max(1e-10)).min(0.5);
        let nutrient_b = b_nutrient * (invest_b / b_reserve.max(1e-10)).min(0.5);
        let nutrient_per_offspring = (nutrient_a + nutrient_b) / offspring_count as f32;

        // Deduct from parents
        agents[*a_idx].reserve -= invest_a;
        agents[*a_idx].nutrient -= nutrient_a;
        agents[*b_idx].reserve -= invest_b;
        agents[*b_idx].nutrient -= nutrient_b;

        // Parent midpoint for dispersal
        let mid_pos = (
            (a_pos.0 + b_pos.0) / 2.0,
            (a_pos.1 + b_pos.1) / 2.0,
        );

        // Dispersal radius scaled by contact time and sensing range
        let avg_sensing = (a_traits.sensing_range + b_traits.sensing_range) / 2.0;
        let avg_contact = (a_contact + b_contact) as f32 / 2.0;
        let dispersal_radius = avg_sensing * (avg_contact / (avg_contact + 50.0));

        for _ in 0..offspring_count {
            // Trait crossover: each dimension from one parent
            let mut child_traits = TraitVector {
                photosynthetic_absorption: 0.0,
                heterotrophy: 0.0,
                nutrient_absorption: 0.0,
                mobility: 0.0,
                chemotaxis_sensitivity: 0.0,
                mate_selectivity: 0.0,
                sensing_range: 0.0,
                reproductive_investment: 0.0,
                fecundity: 0.0,
                somatic_maintenance: 0.0,
            };
            for dim in 0..TraitVector::NUM_DIMS {
                let parent_val = if rng.random::<bool>() {
                    a_traits.get(dim)
                } else {
                    b_traits.get(dim)
                };
                // Mutation
                let val = if params.mutation_rate > 0.0
                    && rng.random::<f32>() < params.mutation_rate
                {
                    let normal = Normal::new(0.0_f32, params.mutation_magnitude).unwrap();
                    (parent_val + normal.sample(rng)).max(0.0)
                } else {
                    parent_val
                };
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
            };
            next_id += 1;
            offspring.push(child);
        }

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
            nutrient_absorption: 0.0,
            mobility: 0.0,
            chemotaxis_sensitivity: 0.0,
            mate_selectivity: 0.0,
            sensing_range: 0.0,
            reproductive_investment: 0.0,
            fecundity: 0.0,
            somatic_maintenance: 0.0,
        }
    }

    fn test_params() -> WorldParameters {
        WorldParameters {
            solar_flux_magnitude: 10.0,
            base_metabolic_rate: 0.1,
            sensing_cost_coefficient: 0.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.5,
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
            nutrient_absorption_maintenance_cost: 0.0,
            initial_nutrient_pool: 0.0,
            growth_efficiency: 0.0,
            wear_rate: 0.0,
            wear_degradation_steepness: 0.0,
            somatic_maintenance_cost_coefficient: 0.0,
            use_wear_rate: 0.0,
            structure_maintenance_coefficient: 0.0,
            repair_decay: 0.0,
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
            nutrient_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        agents[0].contact_time = 10;
        let mut pool = 100.0;

        let events = absorb_nutrients(&mut agents, &mut pool, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::NutrientAbsorbed);
        // demand = 0.5 * 10 / (10 + 50) = 0.5 * 10/60 ≈ 0.0833
        let expected = 0.5 * 10.0 / 60.0;
        assert!((agents[0].nutrient - expected).abs() < 1e-3);
        assert!((pool - (100.0 - expected)).abs() < 1e-3);
    }

    #[test]
    fn absorb_nutrients_shares_proportionally_when_demand_exceeds_pool() {
        let params = test_params();
        let traits = TraitVector {
            nutrient_absorption: 1.0,
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
            nutrient_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        agents[0].contact_time = 0;
        let mut pool = 100.0;

        let events = absorb_nutrients(&mut agents, &mut pool, &params);
        assert!(events.is_empty());
        assert!((agents[0].nutrient).abs() < 1e-6);
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

    // --- Grow ---

    #[test]
    fn grow_converts_surplus_reserve_to_structure() {
        let params = WorldParameters {
            base_metabolic_rate: 1.0,
            growth_efficiency: 0.8,
            ..test_params()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, zero_traits())];

        let (events, dissipated) = grow(&mut agents, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::Grew);
        // retention = 1.0 * 2.0 = 2.0
        // surplus = 100.0 - 2.0 = 98.0
        // to_structure = 98.0 * 0.8 = 78.4
        // dissipated = 98.0 - 78.4 = 19.6
        assert!((agents[0].structure - 78.4).abs() < 1e-3);
        assert!((agents[0].reserve - 2.0).abs() < 1e-3);
        assert!((dissipated - 19.6).abs() < 1e-3);
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

        let events = apply_wear(&mut agents, &params);

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
            somatic_maintenance: 0.1,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        agents[0].wear[0] = 1.0; // pre-existing wear

        let _events = apply_wear(&mut agents, &params);

        // Accumulation: 0.1 * 0.5 = 0.05, so wear goes to 1.05
        // Then repair: 0.1 * exp(-1.0 * 1.05) = 0.1 * 0.3499 = 0.03499
        // Final: 1.05 - 0.03499 ≈ 1.015
        assert!(agents[0].wear[0] < 1.05);
        assert!(agents[0].wear[0] > 1.0);
    }

    #[test]
    fn apply_wear_no_event_when_all_traits_zero() {
        let params = WorldParameters {
            wear_rate: 0.1,
            ..test_params()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, zero_traits())];

        let events = apply_wear(&mut agents, &params);
        assert!(events.is_empty());
    }

    // --- Check death thresholds ---

    #[test]
    fn check_death_kills_agent_with_zero_reserve() {
        let params = test_params();
        let mut agents = vec![make_agent(1, (0.0, 0.0), 0.0, zero_traits())];

        let (events, carcasses) = check_death_thresholds(&mut agents, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::Died);
        assert_eq!(carcasses.len(), 1);
        assert_eq!(carcasses[0].id, 1);
    }

    #[test]
    fn check_death_kills_agent_with_negative_reserve() {
        let params = test_params();
        let mut agents = vec![make_agent(1, (0.0, 0.0), -5.0, zero_traits())];

        let (events, carcasses) = check_death_thresholds(&mut agents, &params);

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
            nutrient_absorption: 0.1,
            mobility: 0.1,
            chemotaxis_sensitivity: 0.1,
            mate_selectivity: 0.1,
            sensing_range: 0.1,
            reproductive_investment: 0.1,
            fecundity: 0.1,
            somatic_maintenance: 0.1,
        };
        let threshold = crate::death_threshold(&traits);
        assert!(threshold > 0.0, "generalist should have nonzero threshold");

        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, traits)];
        agents[0].structure = threshold * 0.5; // below threshold

        let (events, carcasses) = check_death_thresholds(&mut agents, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::Died);
        assert_eq!(carcasses.len(), 1);
        assert!((carcasses[0].energy - threshold * 0.5).abs() < 1e-6);
    }

    #[test]
    fn check_death_preserves_healthy_agents() {
        let params = test_params();
        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, zero_traits())];

        let (events, carcasses) = check_death_thresholds(&mut agents, &params);

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

        let (_, carcasses) = check_death_thresholds(&mut agents, &params);

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
            nutrient_absorption: 0.1,
            mobility: 0.1,
            chemotaxis_sensitivity: 0.1,
            mate_selectivity: 0.1,
            sensing_range: 0.1,
            reproductive_investment: 0.1,
            fecundity: 0.1,
            somatic_maintenance: 0.1,
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
            mobility: 0.5,
            chemotaxis_sensitivity: 1.0,
            sensing_range: 50.0,
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
            mobility: 0.5,
            chemotaxis_sensitivity: 1.0,
            sensing_range: 50.0,
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
    fn move_energy_cost_proportional_to_distance() {
        use rand::SeedableRng;
        let mut params = test_params();
        params.movement_cost_coefficient = 2.0;
        let traits = TraitVector {
            mobility: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(0, (0.0, 0.0), 100.0, traits)];
        let carcasses = vec![];
        let grid = crate::spatial::SpatialGrid::new(100.0, 10.0);
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = move_agents(&mut agents, &carcasses, &grid, &params, &mut rng);

        // eff_mobility = 0.5 (no wear, k=0 so exp(0)=1)
        // cost = distance * coefficient = 0.5 * 2.0 = 1.0
        let expected_cost = 0.5 * 2.0;
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
            mobility: 0.5,
            sensing_range: 50.0,
            chemotaxis_sensitivity: 0.1,
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
            // Consumer has 1 nonzero trait -> stoichiometric_demand = 1.0
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
        // stoichiometric_demand for consumer = 1.0 (one nonzero trait)
        // consumer_nutrient_need = 1.0 * 1.0 = 1.0
        // retained = min(4.0, 1.0) = 1.0
        // excreted = 4.0 - 1.0 = 3.0
        assert!((agents[0].nutrient - 1.0).abs() < 1e-3,
            "consumer should retain 1.0 nutrient, got {}", agents[0].nutrient);
        assert!((nutrient_pool - 3.0).abs() < 1e-3,
            "nutrient pool should receive 3.0 excess, got {}", nutrient_pool);
        assert!((agents[1].nutrient - 16.0).abs() < 1e-3,
            "target nutrient should be 16.0 (20 - 4), got {}", agents[1].nutrient);
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
            reproductive_investment: 0.3,
            fecundity: 1.0,
            sensing_range: 10.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;

        let dead_ids: std::collections::HashSet<u64> = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        // Each parent invests reproductive_investment (0.3), capped at reserve
        // Offspring energy = (0.3 + 0.3) * 0.7 = 0.42
        // Dissipated = 0.6 - 0.42 = 0.18
        assert!(!result.offspring.is_empty(), "should produce offspring");
        let total_offspring_energy: f32 = result.offspring.iter().map(|o| o.reserve).sum();
        assert!((total_offspring_energy - 0.42).abs() < 1e-3,
            "offspring energy should be 0.42, got {}", total_offspring_energy);
        assert!((result.dissipated - 0.18).abs() < 1e-3,
            "dissipated should be 0.18, got {}", result.dissipated);
        // Parents should have paid investment
        assert!((agents[0].reserve - 99.7).abs() < 1e-3,
            "parent A reserve should be 99.7, got {}", agents[0].reserve);
        assert!((agents[1].reserve - 99.7).abs() < 1e-3,
            "parent B reserve should be 99.7, got {}", agents[1].reserve);
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
            reproductive_investment: 0.3,
            fecundity: 1.0,
            sensing_range: 10.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;

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
            reproductive_investment: 0.3,
            fecundity: 1.0,
            sensing_range: 10.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;

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
            reproductive_investment: 0.3,
            fecundity: 1.0,
            sensing_range: 10.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        // stoichiometric_demand for these traits = 4.0 (4 nonzero traits)
        // Set nutrient below demand
        agents[0].nutrient = 1.0;
        agents[1].nutrient = 1.0;

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
    fn reproduction_investment_capped_at_reserve() {
        use rand::SeedableRng;
        let params = WorldParameters {
            reproduction_efficiency: 0.7,
            reproduction_energy_threshold: 10.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        // reproductive_investment=50.0 but reserve is only 15.0
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            reproductive_investment: 50.0,
            fecundity: 1.0,
            sensing_range: 10.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 15.0, traits),
            make_agent(2, (1.0, 0.0), 15.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        // Investment capped at reserve: 15.0 each
        // Total = 30.0, offspring energy = 30.0 * 0.7 = 21.0
        assert!(!result.offspring.is_empty());
        let total_offspring_energy: f32 = result.offspring.iter().map(|o| o.reserve).sum();
        assert!((total_offspring_energy - 21.0).abs() < 1e-3,
            "offspring energy should be 21.0, got {}", total_offspring_energy);
        // Parents should have 0 reserve (invested all)
        assert!(agents[0].reserve.abs() < 1e-3,
            "parent A should have 0 reserve, got {}", agents[0].reserve);
        assert!(agents[1].reserve.abs() < 1e-3,
            "parent B should have 0 reserve, got {}", agents[1].reserve);
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
            reproductive_investment: 0.3,
            fecundity: 1.0,
            sensing_range: 50.0,
            ..zero_traits()
        };
        let traits_c = TraitVector {
            photosynthetic_absorption: 0.1,
            heterotrophy: 0.4,
            reproductive_investment: 0.3,
            fecundity: 1.0,
            sensing_range: 50.0,
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
            reproductive_investment: 0.3,
            fecundity: 1.0,
            sensing_range: 10.0,
            ..zero_traits()
        };
        let traits_b = TraitVector {
            photosynthetic_absorption: 0.0,
            heterotrophy: 0.8,
            reproductive_investment: 0.3,
            fecundity: 1.0,
            sensing_range: 10.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits_a),
            make_agent(2, (1.0, 0.0), 100.0, traits_b),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;

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
            reproductive_investment: 10.0,
            fecundity: 5.0, // Poisson mean=5
            sensing_range: 10.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 20.0;
        agents[1].nutrient = 20.0;

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
        let total_investment = 20.0; // 10.0 + 10.0 (capped at reserve is 100, so invest=10)
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
            reproductive_investment: 5.0,
            fecundity: 3.0,
            sensing_range: 20.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].contact_time = 100; // high contact time -> wider dispersal
        agents[1].contact_time = 100;

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
        // With high contact time and sensing range, offspring should be dispersed
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
            reproductive_investment: 0.3,
            fecundity: 1.0,
            sensing_range: 10.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;

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
            reproductive_investment: 5.0,
            fecundity: 3.0,
            sensing_range: 10.0,
            ..zero_traits()
        };

        let run = |seed: u64| -> (Vec<(f32, f32)>, Vec<f32>) {
            let mut agents = vec![
                make_agent(1, (0.0, 0.0), 100.0, traits),
                make_agent(2, (1.0, 0.0), 100.0, traits),
            ];
            agents[0].nutrient = 10.0;
            agents[1].nutrient = 10.0;

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
            reproductive_investment: 10.0,
            fecundity: 1.0,
            sensing_range: 10.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_reproduction(
            &mut agents, &dead_ids, &grid, &params, &mut rng,
        );

        // Total investment = 10.0 + 10.0 = 20.0
        // Offspring energy = 20.0 * 0.5 = 10.0
        // Dissipated = 20.0 - 10.0 = 10.0
        let total_offspring_energy: f32 = result.offspring.iter().map(|o| o.reserve).sum();
        assert!((total_offspring_energy - 10.0).abs() < 1e-3,
            "offspring energy should be 10.0, got {}", total_offspring_energy);
        assert!((result.dissipated - 10.0).abs() < 1e-3,
            "dissipated should be 10.0, got {}", result.dissipated);
        // Conservation: investment = offspring + dissipated
        assert!((total_offspring_energy + result.dissipated - 20.0).abs() < 1e-3,
            "energy should be conserved");
    }
}
