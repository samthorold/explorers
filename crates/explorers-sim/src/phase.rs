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
            spatial_decay_rate: 0.5,
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
}
