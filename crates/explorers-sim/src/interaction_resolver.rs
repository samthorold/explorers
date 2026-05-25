use std::collections::{HashMap, HashSet};

use crate::event;
use crate::spatial::SpatialGrid;
use crate::{
    Agent, Broadcast, Carcass, NearbyAgent, ProjectionData,
    emit_broadcasts, toroidal_distance,
};

/// Subset of WorldParameters needed for consumption resolution.
pub struct ConsumptionParams {
    pub contact_radius: f32,
    pub consumption_efficiency: f32,
    pub world_extent: f32,
    pub reproduction_energy_threshold: f32,
}

/// Accumulated mutations from resolving consumption interactions.
pub struct ResolverResult {
    pub events: Vec<event::Event>,
    pub broadcasts: Vec<Broadcast>,
    pub consumption_gains: Vec<f32>,
    pub consumption_losses: Vec<f32>,
    pub dead_agents: HashSet<usize>,
    pub new_carcasses: Vec<Carcass>,
    pub depleted_carcass_indices: HashSet<usize>,
    pub dissipated_energy: f32,
}

/// Resolve consumption interactions and their consequence cascades.
///
/// Takes an immutable snapshot of agent state and returns mutations for
/// `step()` to apply. The resolver maintains working copies of energies
/// to correctly handle sequential interactions within a single tick.
pub fn resolve_consumption(
    agents: &[Agent],
    agent_grid: &SpatialGrid,
    carcass_grid: &SpatialGrid,
    carcasses: &[Carcass],
    params: &ConsumptionParams,
    order: &[usize],
    dead_agents_before: &HashSet<usize>,
    nack_sets: &HashMap<u64, HashSet<event::EventKind>>,
    tick: u64,
    pre_tick_energies: &[f32],
) -> ResolverResult {
    let n = agents.len();
    let mut working_energies: Vec<f32> = agents.iter().map(|a| a.energy).collect();
    let mut consumption_gains = vec![0.0_f32; n];
    let mut consumption_losses = vec![0.0_f32; n];
    let mut events = Vec::new();
    let mut broadcasts = Vec::new();
    let mut dead_agents: HashSet<usize> = dead_agents_before.clone();
    let mut new_carcasses: Vec<Carcass> = Vec::new();
    let mut depleted_carcass_indices: HashSet<usize> = HashSet::new();
    let mut dissipated_energy = 0.0_f32;

    let mut consequence_queue = event::EventQueue::new();

    // Mutable copy of agent_grid for dead agent removal
    let mut agent_grid = agent_grid.clone();
    // Mutable copy of carcass_grid for new carcass insertions
    let mut carcass_grid = carcass_grid.clone();

    for &i in order {
        if dead_agents.contains(&i) || working_energies[i] <= 0.0 {
            continue;
        }

        // Try consumption
        if agents[i].traits.consumption_rate > 0.0 {
            let neighbors =
                agent_grid.query_radius(agents[i].position, params.contact_radius);
            let nearby_agents: Vec<NearbyAgent> = neighbors
                .iter()
                .filter_map(|&j_id| {
                    let j = j_id as usize;
                    if j == i
                        || dead_agents.contains(&j)
                        || working_energies[j] <= 0.0
                    {
                        return None;
                    }
                    let dist = toroidal_distance(
                        agents[i].position,
                        agents[j].position,
                        params.world_extent,
                    );
                    Some(NearbyAgent {
                        id: agents[j].id,
                        distance: dist,
                        energy: working_energies[j],
                        traits: agents[j].traits,
                    })
                })
                .collect();

            let data = ProjectionData {
                feeding_gradient: (0.0, 0.0),
                carcass_gradient: (0.0, 0.0),
                nearby_agents,
                nearby_carcasses: vec![],
                contact_radius: params.contact_radius,
                reproduction_energy_threshold: params.reproduction_energy_threshold,
            };
            let trigger = event::Event {
                tick,
                seq: 0,
                kind: event::EventKind::Consumed,
                source: 0,
                target: None,
                energy_delta: 0.0,
                position: Some(agents[i].position),
            };
            let response = agents[i].receive(&trigger, &data);
            if let Some(consumed) = response.events.first() {
                let target_id = consumed.target.unwrap();
                let target = (0..n).find(|&j| agents[j].id == target_id).unwrap();
                let drain = consumed.energy_delta;
                let gain = drain * params.consumption_efficiency;
                working_energies[i] += gain;
                working_energies[target] -= drain;
                consumption_gains[i] += gain;
                consumption_losses[target] += drain;
                dissipated_energy += drain * (1.0 - params.consumption_efficiency);

                events.push(event::Event {
                    tick,
                    seq: 0,
                    kind: event::EventKind::Consumed,
                    source: agents[i].id,
                    target: Some(agents[target].id),
                    energy_delta: drain,
                    position: Some(agents[i].position),
                });

                emit_broadcasts(
                    &mut broadcasts,
                    &event::EventKind::Consumed,
                    agents[i].position,
                    agents,
                    &dead_agents,
                    nack_sets,
                    params.world_extent,
                );

                // Queue consequence: target death
                if working_energies[target] <= 0.0 {
                    let carcass_energy =
                        (pre_tick_energies[target] - consumption_losses[target])
                            .max(0.0);
                    consequence_queue.push_high(event::Event {
                        tick,
                        seq: 0,
                        kind: event::EventKind::Died,
                        source: agents[target].id,
                        target: None,
                        energy_delta: carcass_energy,
                        position: Some(agents[target].position),
                    });
                }
            }
        }

        // Drain consequence queue
        while let Some(consequence) = consequence_queue.pop() {
            match consequence.kind {
                event::EventKind::Died => {
                    let dead_id = consequence.source;
                    let dead_pos = consequence.position.unwrap();
                    let carcass_energy = consequence.energy_delta;

                    events.push(event::Event {
                        tick,
                        seq: 0,
                        kind: event::EventKind::Died,
                        source: dead_id,
                        target: None,
                        energy_delta: 0.0,
                        position: Some(dead_pos),
                    });

                    consequence_queue.push_high(event::Event {
                        tick,
                        seq: 0,
                        kind: event::EventKind::CarcassCreated,
                        source: dead_id,
                        target: None,
                        energy_delta: carcass_energy,
                        position: Some(dead_pos),
                    });

                    emit_broadcasts(
                        &mut broadcasts,
                        &event::EventKind::Died,
                        dead_pos,
                        agents,
                        &dead_agents,
                        nack_sets,
                        params.world_extent,
                    );

                    let target_idx =
                        (0..n).find(|&j| agents[j].id == dead_id).unwrap();
                    dead_agents.insert(target_idx);
                    agent_grid.remove(target_idx as u64);
                }
                event::EventKind::CarcassCreated => {
                    let dead_id = consequence.source;
                    let dead_pos = consequence.position.unwrap();
                    let carcass_energy = consequence.energy_delta;

                    events.push(event::Event {
                        tick,
                        seq: 0,
                        kind: event::EventKind::CarcassCreated,
                        source: dead_id,
                        target: None,
                        energy_delta: carcass_energy,
                        position: Some(dead_pos),
                    });

                    let ci = carcasses.len() + new_carcasses.len();
                    new_carcasses.push(Carcass {
                        id: dead_id,
                        position: dead_pos,
                        energy: carcass_energy,
                    });
                    carcass_grid.insert(ci as u64, dead_pos);

                    emit_broadcasts(
                        &mut broadcasts,
                        &event::EventKind::CarcassCreated,
                        dead_pos,
                        agents,
                        &dead_agents,
                        nack_sets,
                        params.world_extent,
                    );
                }
                event::EventKind::CarcassDepleted => {
                    let carcass_id = consequence.source;
                    let carcass_pos = consequence.position.unwrap();

                    events.push(event::Event {
                        tick,
                        seq: 0,
                        kind: event::EventKind::CarcassDepleted,
                        source: carcass_id,
                        target: None,
                        energy_delta: 0.0,
                        position: Some(carcass_pos),
                    });

                    let ci = carcasses
                        .iter()
                        .position(|c| c.id == carcass_id)
                        .unwrap();
                    depleted_carcass_indices.insert(ci);
                    carcass_grid.remove(ci as u64);
                }
                _ => {}
            }
        }
    }

    ResolverResult {
        events,
        broadcasts,
        consumption_gains,
        consumption_losses,
        dead_agents,
        new_carcasses,
        depleted_carcass_indices,
        dissipated_energy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TraitVector;

    fn zero_traits() -> TraitVector {
        TraitVector {
            photosynthetic_absorption: 0.0,
            consumption_rate: 0.0,
            scavenging_rate: 0.0,
            mobility: 0.0,
            chemotaxis_sensitivity: 0.0,
            mate_selectivity: 0.0,
            sensing_range: 0.0,
            reproductive_investment: 0.0,
        }
    }

    #[test]
    fn single_consumption_returns_correct_energy_deltas() {
        let agents = vec![
            Agent {
                id: 0,
                position: (0.0, 0.0),
                energy: 50.0,
                traits: TraitVector {
                    consumption_rate: 2.0,
                    ..zero_traits()
                },
            },
            Agent {
                id: 1,
                position: (3.0, 0.0),
                energy: 50.0,
                traits: zero_traits(),
            },
        ];

        let extent = 100.0;
        let cell_size = 5.0;
        let mut agent_grid = SpatialGrid::new(extent, cell_size);
        for (i, a) in agents.iter().enumerate() {
            agent_grid.insert(i as u64, a.position);
        }
        let carcass_grid = SpatialGrid::new(extent, cell_size);

        let params = ConsumptionParams {
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
            world_extent: extent,
            reproduction_energy_threshold: 50.0,
        };

        let order = vec![0, 1];
        let pre_tick_energies = vec![50.0, 50.0];

        let result = resolve_consumption(
            &agents,
            &agent_grid,
            &carcass_grid,
            &[],
            &params,
            &order,
            &HashSet::new(),
            &HashMap::new(),
            0,
            &pre_tick_energies,
        );

        // drain = 2.0, gain = 2.0 * 0.5 = 1.0
        assert!(
            (result.consumption_gains[0] - 1.0).abs() < 1e-5,
            "consumer gain: {}",
            result.consumption_gains[0]
        );
        assert!(
            (result.consumption_losses[1] - 2.0).abs() < 1e-5,
            "target loss: {}",
            result.consumption_losses[1]
        );
        assert!(
            (result.dissipated_energy - 1.0).abs() < 1e-5,
            "dissipated: {}",
            result.dissipated_energy
        );
        assert!(result.dead_agents.is_empty());
        assert!(result.new_carcasses.is_empty());

        // Should have one Consumed event
        let consumed: Vec<_> = result
            .events
            .iter()
            .filter(|e| e.kind == event::EventKind::Consumed)
            .collect();
        assert_eq!(consumed.len(), 1);
        assert_eq!(consumed[0].source, 0);
        assert_eq!(consumed[0].target, Some(1));
        assert!((consumed[0].energy_delta - 2.0).abs() < 1e-5);
    }

    #[test]
    fn consumption_cascade_drain_to_death_to_carcass() {
        // Consumer with rate 20 kills a target with only 5 energy
        let agents = vec![
            Agent {
                id: 10,
                position: (0.0, 0.0),
                energy: 100.0,
                traits: TraitVector {
                    consumption_rate: 20.0,
                    ..zero_traits()
                },
            },
            Agent {
                id: 11,
                position: (1.0, 0.0),
                energy: 5.0,
                traits: zero_traits(),
            },
        ];

        let extent = 100.0;
        let cell_size = 5.0;
        let mut agent_grid = SpatialGrid::new(extent, cell_size);
        for (i, a) in agents.iter().enumerate() {
            agent_grid.insert(i as u64, a.position);
        }
        let carcass_grid = SpatialGrid::new(extent, cell_size);

        let params = ConsumptionParams {
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
            world_extent: extent,
            reproduction_energy_threshold: 50.0,
        };

        let order = vec![0, 1];
        let pre_tick_energies = vec![100.0, 5.0];

        let result = resolve_consumption(
            &agents,
            &agent_grid,
            &carcass_grid,
            &[],
            &params,
            &order,
            &HashSet::new(),
            &HashMap::new(),
            0,
            &pre_tick_energies,
        );

        // drain = 5.0 (capped at target energy), gain = 5.0 * 0.5 = 2.5
        assert!(
            (result.consumption_gains[0] - 2.5).abs() < 1e-5,
            "consumer gain: {}",
            result.consumption_gains[0]
        );
        assert!(
            (result.consumption_losses[1] - 5.0).abs() < 1e-5,
            "target loss: {}",
            result.consumption_losses[1]
        );

        // Target should be dead
        assert!(result.dead_agents.contains(&1), "target index 1 should be dead");

        // Carcass should be created with pre_tick - consumption_losses = 5.0 - 5.0 = 0.0
        assert_eq!(result.new_carcasses.len(), 1);
        assert_eq!(result.new_carcasses[0].id, 11);
        assert!((result.new_carcasses[0].energy - 0.0).abs() < 1e-5);

        // Event sequence: Consumed -> Died -> CarcassCreated
        let kinds: Vec<&event::EventKind> =
            result.events.iter().map(|e| &e.kind).collect();
        assert!(kinds.contains(&&event::EventKind::Consumed));
        assert!(kinds.contains(&&event::EventKind::Died));
        assert!(kinds.contains(&&event::EventKind::CarcassCreated));

        // Order: Consumed before Died, Died before CarcassCreated
        let consumed_pos = kinds
            .iter()
            .position(|k| **k == event::EventKind::Consumed)
            .unwrap();
        let died_pos = kinds
            .iter()
            .position(|k| **k == event::EventKind::Died)
            .unwrap();
        let carcass_pos = kinds
            .iter()
            .position(|k| **k == event::EventKind::CarcassCreated)
            .unwrap();
        assert!(consumed_pos < died_pos, "Consumed must precede Died");
        assert!(died_pos < carcass_pos, "Died must precede CarcassCreated");
    }
}
