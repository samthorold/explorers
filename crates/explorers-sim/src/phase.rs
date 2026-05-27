//! Autonomous phase functions for the tick loop.
//!
//! Each function takes agent state (and world parameters where needed),
//! mutates state in place, and returns events recording what happened.
//! These are free functions, not methods on World.

use crate::event::{Event, EventKind};
use crate::spatial::SpatialGrid;
use crate::{
    Agent, Carcass, WorldParameters, FUNCTIONAL_TRAIT_COUNT, FUNCTIONAL_TRAIT_INDICES,
};
use rand_chacha::ChaCha8Rng;
use rand::Rng;

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

        // Compute light share: proportional to this agent's effective
        // photosynthetic absorption relative to all co-located producers.
        // Use grid query but deduplicate results (grid can return duplicates
        // when query radius exceeds half the world extent).
        let mut total_photo = eff_photo;
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
            if crate::toroidal_distance(agents[i].position, agents[j].position, extent)
                < competition_radius
            {
                total_photo += other_eff;
            }
        }
        let light_share = eff_photo / total_photo;
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
            let eff_absorption = a.effective_trait_with_steepness(3, k);
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
            + agent.traits.consumption_rate * params.consumption_maintenance_cost
            + agent.traits.scavenging_rate * params.scavenging_maintenance_cost
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
            + agent.traits.consumption_rate * params.consumption_maintenance_cost
            + agent.traits.scavenging_rate * params.scavenging_maintenance_cost
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
                let eff_consumption = agents[consumer_idx]
                    .effective_trait_with_steepness(1, k); // index 1 = consumption_rate
                if eff_consumption <= 0.0 {
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
                consumers.push((consumer_idx, eff_consumption));
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
                let eff_scavenging = agents[consumer_idx]
                    .effective_trait_with_steepness(2, k); // index 2 = scavenging_rate
                if eff_scavenging <= 0.0 {
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
                consumers.push((consumer_idx, eff_scavenging));
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
        let eff_mobility = agents[i].effective_trait_with_steepness(4, k);
        let eff_chemotaxis = agents[i].effective_trait_with_steepness(5, k);
        let eff_sensing = agents[i].effective_trait_with_steepness(6, k);
        let eff_consumption = agents[i].effective_trait_with_steepness(1, k);
        let eff_scavenging = agents[i].effective_trait_with_steepness(2, k);

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
                // Attraction weighted by chemotaxis * consumption (toward living agents)
                let weight = eff_chemotaxis * eff_consumption / dist;
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
                // Attraction weighted by chemotaxis * scavenging (toward carcasses)
                let weight = eff_chemotaxis * eff_scavenging / dist;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TraitVector, WorldParameters, FUNCTIONAL_TRAIT_COUNT};

    fn zero_traits() -> TraitVector {
        TraitVector {
            photosynthetic_absorption: 0.0,
            consumption_rate: 0.0,
            scavenging_rate: 0.0,
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
            consumption_maintenance_cost: 0.0,
            scavenging_maintenance_cost: 0.0,
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
            consumption_rate: 0.1,
            scavenging_rate: 0.1,
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
            consumption_rate: 0.4,
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
            consumption_rate: 3.0,
            ..zero_traits()
        };
        let consumer_b_traits = TraitVector {
            consumption_rate: 1.0,
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
            consumption_rate: 0.1,
            scavenging_rate: 0.1,
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
            consumption_rate: 5.0, // high demand to drain target
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
            consumption_rate: 100.0,
            scavenging_rate: 10.0, // also a scavenger
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
    fn drain_dual_consumption_and_scavenging_same_tick() {
        // An agent with both consumption and scavenging traits can drain a
        // living target AND scavenge a carcass in the same tick.
        let params = test_params(); // consumption_efficiency=0.5, decomposition_efficiency=0.5
        let dual_traits = TraitVector {
            consumption_rate: 1.0,
            scavenging_rate: 2.0,
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, dual_traits),
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

        // From living target: drain 1.0, gain 1.0 * 0.5 = 0.5
        // From carcass: drain 2.0, gain 2.0 * 0.5 = 1.0
        // Consumer reserve: 10.0 + 0.5 + 1.0 = 11.5
        assert!((agents[0].reserve - 11.5).abs() < 1e-3,
            "consumer should gain from both living and carcass, got {}", agents[0].reserve);
        // Living target structure: 20.0 - 1.0 = 19.0
        assert!((agents[1].structure - 19.0).abs() < 1e-3,
            "target structure should be 19.0, got {}", agents[1].structure);
        // Carcass energy: 10.0 - 2.0 = 8.0
        assert!((carcasses[0].energy - 8.0).abs() < 1e-3,
            "carcass energy should be 8.0, got {}", carcasses[0].energy);
        // Events should include both living and carcass consumption
        assert!(result.events.len() >= 2,
            "should have events for both consumption types");
    }

    // --- Move agents ---

    #[test]
    fn move_direction_attracted_to_nearby_living_agent_by_consumption() {
        // Agent with consumption trait and chemotaxis should move toward a nearby living agent.
        use rand::SeedableRng;
        let mut params = test_params();
        params.movement_cost_coefficient = 0.0; // isolate direction test
        let mover_traits = TraitVector {
            consumption_rate: 0.5,
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
    fn move_direction_attracted_to_carcass_by_scavenging() {
        // Agent with scavenging trait and chemotaxis should move toward a nearby carcass.
        use rand::SeedableRng;
        let mut params = test_params();
        params.movement_cost_coefficient = 0.0;
        let mover_traits = TraitVector {
            scavenging_rate: 0.5,
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
            consumption_rate: 0.1,
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
            consumption_rate: 2.0,
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
}
