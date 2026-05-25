use std::collections::{HashMap, HashSet};

use rand::distr::Distribution;
use rand_chacha::ChaCha8Rng;
use rand_distr::Normal;

use crate::event;
use crate::spatial::SpatialGrid;
use crate::{
    Agent, Broadcast, Carcass, NearbyAgent, ProjectionData, TraitVector,
    emit_broadcasts, toroidal_distance,
};

/// Subset of WorldParameters needed for interaction resolution.
pub struct ResolverParams {
    pub contact_radius: f32,
    pub consumption_efficiency: f32,
    pub decomposition_efficiency: f32,
    pub world_extent: f32,
    pub reproduction_energy_threshold: f32,
    pub solar_flux_magnitude: f32,
    pub reproduction_efficiency: f32,
    pub mutation_rate: f32,
    pub mutation_magnitude: f32,
}

/// Accumulated mutations from resolving all interactions.
pub struct ResolverResult {
    pub events: Vec<event::Event>,
    pub broadcasts: Vec<Broadcast>,
    pub consumption_gains: Vec<f32>,
    pub consumption_losses: Vec<f32>,
    pub dead_agents: HashSet<usize>,
    pub decomposition_gains: Vec<f32>,
    pub new_carcasses: Vec<Carcass>,
    pub depleted_carcass_indices: HashSet<usize>,
    pub dissipated_energy: f32,
    pub solar_gains: Vec<f32>,
    pub total_solar_input: f32,
    pub offspring: Vec<Agent>,
    pub reproduction_investments: Vec<f32>,
    pub next_agent_id: u64,
}

/// Resolve trophic interactions (consumption and decomposition) and their
/// consequence cascades.
///
/// Takes an immutable snapshot of agent state and returns mutations for
/// `step()` to apply. The resolver maintains working copies of energies
/// to correctly handle sequential interactions within a single tick.
pub fn resolve_interactions(
    agents: &[Agent],
    agent_grid: &SpatialGrid,
    carcass_grid: &SpatialGrid,
    carcasses: &[Carcass],
    params: &ResolverParams,
    order: &[usize],
    dead_agents_before: &HashSet<usize>,
    nack_sets: &HashMap<u64, HashSet<event::EventKind>>,
    tick: u64,
    pre_tick_energies: &[f32],
    light_shares: &[f32],
    rng: &mut ChaCha8Rng,
    next_agent_id: u64,
) -> ResolverResult {
    let n = agents.len();
    let mut working_energies: Vec<f32> = agents.iter().map(|a| a.energy).collect();
    let mut consumption_gains = vec![0.0_f32; n];
    let mut consumption_losses = vec![0.0_f32; n];
    let mut decomposition_gains = vec![0.0_f32; n];
    let mut events = Vec::new();
    let mut broadcasts = Vec::new();
    let mut dead_agents: HashSet<usize> = dead_agents_before.clone();
    let mut new_carcasses: Vec<Carcass> = Vec::new();
    let mut depleted_carcass_indices: HashSet<usize> = HashSet::new();
    let mut dissipated_energy = 0.0_f32;
    let mut solar_gains = vec![0.0_f32; n];
    let mut total_solar_input = 0.0_f32;

    let mut consequence_queue = event::EventQueue::new();

    // Mutable copy of agent_grid for dead agent removal
    let mut agent_grid = agent_grid.clone();
    // Mutable copy of carcass_grid for new carcass insertions
    let mut carcass_grid = carcass_grid.clone();

    // Working copies of carcass energies for decomposition tracking
    let mut working_carcass_energies: Vec<f32> =
        carcasses.iter().map(|c| c.energy).collect();

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

        // Try decomposition (mutually exclusive with consumption)
        if consumption_gains[i] == 0.0 && agents[i].traits.scavenging_rate > 0.0 {
            let nearby_carcass_ids =
                carcass_grid.query_radius(agents[i].position, params.contact_radius);
            let nearby_carcasses: Vec<crate::NearbyCarcass> = nearby_carcass_ids
                .iter()
                .filter_map(|&ci_id| {
                    let ci = ci_id as usize;
                    // Get energy from working copy; handle both original and new carcasses
                    let energy = if ci < working_carcass_energies.len() {
                        working_carcass_energies[ci]
                    } else {
                        let new_idx = ci - carcasses.len();
                        if new_idx < new_carcasses.len() {
                            new_carcasses[new_idx].energy
                        } else {
                            return None;
                        }
                    };
                    if energy <= 0.0 {
                        return None;
                    }
                    let carcass_pos = if ci < carcasses.len() {
                        carcasses[ci].position
                    } else {
                        new_carcasses[ci - carcasses.len()].position
                    };
                    let carcass_id = if ci < carcasses.len() {
                        carcasses[ci].id
                    } else {
                        new_carcasses[ci - carcasses.len()].id
                    };
                    let dist = toroidal_distance(
                        agents[i].position,
                        carcass_pos,
                        params.world_extent,
                    );
                    Some(crate::NearbyCarcass {
                        id: carcass_id,
                        distance: dist,
                        energy,
                    })
                })
                .collect();

            let data = ProjectionData {
                feeding_gradient: (0.0, 0.0),
                carcass_gradient: (0.0, 0.0),
                nearby_agents: vec![],
                nearby_carcasses,
                contact_radius: params.contact_radius,
                reproduction_energy_threshold: params.reproduction_energy_threshold,
            };
            let trigger = event::Event {
                tick,
                seq: 0,
                kind: event::EventKind::CarcassCreated,
                source: 0,
                target: None,
                energy_delta: 0.0,
                position: Some(agents[i].position),
            };
            let response = agents[i].receive(&trigger, &data);
            if let Some(decomposed) = response.events.first() {
                let carcass_id = decomposed.target.unwrap();
                let drain = decomposed.energy_delta;
                let gain = drain * params.decomposition_efficiency;

                // Find carcass index and update working energy
                if let Some(ci) = carcasses.iter().position(|c| c.id == carcass_id) {
                    working_carcass_energies[ci] -= drain;
                    working_energies[i] += gain;
                    decomposition_gains[i] = gain;
                    dissipated_energy += drain * (1.0 - params.decomposition_efficiency);

                    events.push(event::Event {
                        tick,
                        seq: 0,
                        kind: event::EventKind::Decomposed,
                        source: agents[i].id,
                        target: Some(carcass_id),
                        energy_delta: drain,
                        position: Some(agents[i].position),
                    });

                    emit_broadcasts(
                        &mut broadcasts,
                        &event::EventKind::Decomposed,
                        agents[i].position,
                        agents,
                        &dead_agents,
                        nack_sets,
                        params.world_extent,
                    );

                    // Queue consequence: carcass depletion
                    if working_carcass_energies[ci] <= 0.0 {
                        consequence_queue.push_high(event::Event {
                            tick,
                            seq: 0,
                            kind: event::EventKind::CarcassDepleted,
                            source: carcass_id,
                            target: None,
                            energy_delta: 0.0,
                            position: Some(carcasses[ci].position),
                        });
                    }
                } else if let Some(ni) =
                    new_carcasses.iter().position(|c| c.id == carcass_id)
                {
                    new_carcasses[ni].energy -= drain;
                    working_energies[i] += gain;
                    decomposition_gains[i] = gain;
                    dissipated_energy += drain * (1.0 - params.decomposition_efficiency);

                    events.push(event::Event {
                        tick,
                        seq: 0,
                        kind: event::EventKind::Decomposed,
                        source: agents[i].id,
                        target: Some(carcass_id),
                        energy_delta: drain,
                        position: Some(new_carcasses[ni].position),
                    });

                    emit_broadcasts(
                        &mut broadcasts,
                        &event::EventKind::Decomposed,
                        agents[i].position,
                        agents,
                        &dead_agents,
                        nack_sets,
                        params.world_extent,
                    );

                    // Queue consequence: carcass depletion for new carcass
                    if new_carcasses[ni].energy <= 0.0 {
                        // For new carcasses, CarcassDepleted needs special handling
                        // since they're not in the original carcasses slice
                        consequence_queue.push_high(event::Event {
                            tick,
                            seq: 0,
                            kind: event::EventKind::CarcassDepleted,
                            source: carcass_id,
                            target: None,
                            energy_delta: 0.0,
                            position: Some(new_carcasses[ni].position),
                        });
                    }
                }
            }
        }

        // Photosynthesise (fallback: only if agent didn't consume or decompose)
        let acquired = consumption_gains[i] > 0.0 || decomposition_gains[i] > 0.0;
        if !acquired {
            let mobility_gate = 1.0
                / (1.0 + (20.0_f32 * (agents[i].traits.mobility - 0.3)).exp());
            let solar_gain = agents[i].traits.photosynthetic_absorption
                * params.solar_flux_magnitude
                * mobility_gate
                * light_shares[i];
            working_energies[i] += solar_gain;
            solar_gains[i] = solar_gain;
            total_solar_input += solar_gain;
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

                    if let Some(ci) =
                        carcasses.iter().position(|c| c.id == carcass_id)
                    {
                        depleted_carcass_indices.insert(ci);
                        carcass_grid.remove(ci as u64);
                    } else {
                        // New carcass created this tick
                        let ni = new_carcasses
                            .iter()
                            .position(|c| c.id == carcass_id)
                            .unwrap();
                        let grid_idx = carcasses.len() + ni;
                        carcass_grid.remove(grid_idx as u64);
                    }
                }
                _ => {}
            }
        }
    }

    // --- Reproduction ---
    let mut reproduction_investments = vec![0.0_f32; n];
    let mut reproduced = vec![false; n];
    let mut offspring: Vec<Agent> = Vec::new();
    let mut next_agent_id = next_agent_id;

    for &i in order {
        if dead_agents.contains(&i) || working_energies[i] <= 0.0 {
            continue;
        }
        if reproduced[i] || working_energies[i] <= params.reproduction_energy_threshold {
            continue;
        }

        emit_broadcasts(
            &mut broadcasts,
            &event::EventKind::MatingReadiness,
            agents[i].position,
            agents,
            &dead_agents,
            nack_sets,
            params.world_extent,
        );

        // Build nearby agents for mate selection via receive()
        let nearby_mates: Vec<NearbyAgent> = (0..n)
            .filter_map(|j| {
                if j == i || dead_agents.contains(&j) || reproduced[j]
                    || working_energies[j] <= params.reproduction_energy_threshold
                {
                    return None;
                }
                let dist = toroidal_distance(
                    agents[i].position, agents[j].position, params.world_extent,
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
            nearby_agents: nearby_mates,
            nearby_carcasses: vec![],
            contact_radius: params.contact_radius,
            reproduction_energy_threshold: params.reproduction_energy_threshold,
        };
        let trigger = event::Event {
            tick,
            seq: 0,
            kind: event::EventKind::MatingReadiness,
            source: agents[i].id,
            target: None,
            energy_delta: 0.0,
            position: Some(agents[i].position),
        };
        let response = agents[i].receive(&trigger, &data);

        if let Some(mate_event) = response.events.first() {
            let mate_id = mate_event.target.unwrap();
            let j = (0..n).find(|&k| agents[k].id == mate_id).unwrap();
            reproduced[i] = true;
            reproduced[j] = true;

            events.push(event::Event {
                tick,
                seq: 0,
                kind: event::EventKind::MateSelected,
                source: agents[i].id,
                target: Some(agents[j].id),
                energy_delta: 0.0,
                position: Some(agents[i].position),
            });

            let inv_a = agents[i].traits.reproductive_investment;
            let inv_b = agents[j].traits.reproductive_investment;
            working_energies[i] -= inv_a;
            working_energies[j] -= inv_b;
            reproduction_investments[i] = inv_a;
            reproduction_investments[j] = inv_b;

            let offspring_energy =
                (inv_a + inv_b) * params.reproduction_efficiency;
            dissipated_energy +=
                (inv_a + inv_b) * (1.0 - params.reproduction_efficiency);

            // Trait inheritance: uniform crossover
            let mut child_traits = TraitVector {
                photosynthetic_absorption: 0.0,
                consumption_rate: 0.0,
                scavenging_rate: 0.0,
                mobility: 0.0,
                chemotaxis_sensitivity: 0.0,
                mate_selectivity: 0.0,
                sensing_range: 0.0,
                reproductive_investment: 0.0,
            };
            for dim in 0..8 {
                let from_a: bool = rand::distr::Uniform::new(0u32, 2)
                    .unwrap()
                    .sample(rng)
                    == 0;
                let val = if from_a {
                    agents[i].traits.get(dim)
                } else {
                    agents[j].traits.get(dim)
                };
                child_traits.set(dim, val);
            }

            // Mutation
            if params.mutation_rate > 0.0 {
                let mutation_dist =
                    Normal::new(0.0_f32, params.mutation_magnitude).unwrap();
                for dim in 0..8 {
                    let r: f32 = rand::distr::Uniform::new(0.0_f32, 1.0)
                        .unwrap()
                        .sample(rng);
                    if r < params.mutation_rate {
                        let perturbation = mutation_dist.sample(rng);
                        child_traits.set(dim, child_traits.get(dim) + perturbation);
                    }
                }
            }

            let offspring_id = next_agent_id;
            next_agent_id += 1;

            events.push(event::Event {
                tick,
                seq: 0,
                kind: event::EventKind::Born,
                source: offspring_id,
                target: None,
                energy_delta: offspring_energy,
                position: Some(agents[i].position),
            });

            offspring.push(Agent {
                id: offspring_id,
                position: agents[i].position,
                energy: offspring_energy,
                traits: child_traits,
            });
        }
    }

    ResolverResult {
        events,
        broadcasts,
        consumption_gains,
        consumption_losses,
        decomposition_gains,
        dead_agents,
        new_carcasses,
        depleted_carcass_indices,
        dissipated_energy,
        solar_gains,
        total_solar_input,
        offspring,
        reproduction_investments,
        next_agent_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TraitVector;
    use rand::SeedableRng;

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

        let params = ResolverParams {
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.5,
            world_extent: extent,
            reproduction_energy_threshold: 50.0,
            solar_flux_magnitude: 0.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
        };

        let order = vec![0, 1];
        let pre_tick_energies = vec![50.0, 50.0];
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_interactions(
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
            &vec![0.0; agents.len()],
            &mut rng,
            100,
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

        let params = ResolverParams {
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.5,
            world_extent: extent,
            reproduction_energy_threshold: 50.0,
            solar_flux_magnitude: 0.0,
        reproduction_efficiency: 0.7,
        mutation_rate: 0.0,
        mutation_magnitude: 0.0,
        };

        let order = vec![0, 1];
        let pre_tick_energies = vec![100.0, 5.0];let mut rng = ChaCha8Rng::seed_from_u64(42);



        let result = resolve_interactions(
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
            &vec![0.0; agents.len()],
        &mut rng,
        100,
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

    #[test]
    fn decomposition_drains_carcass_energy() {
        // A scavenger near a carcass should decompose it
        let agents = vec![Agent {
            id: 0,
            position: (0.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                scavenging_rate: 4.0,
                ..zero_traits()
            },
        }];

        let extent = 100.0;
        let cell_size = 5.0;
        let mut agent_grid = SpatialGrid::new(extent, cell_size);
        agent_grid.insert(0, agents[0].position);

        let carcasses = vec![Carcass {
            id: 99,
            position: (2.0, 0.0),
            energy: 10.0,
        }];
        let mut carcass_grid = SpatialGrid::new(extent, cell_size);
        carcass_grid.insert(0, carcasses[0].position);

        let params = ResolverParams {
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.8,
            world_extent: extent,
            reproduction_energy_threshold: 50.0,
            solar_flux_magnitude: 0.0,
        reproduction_efficiency: 0.7,
        mutation_rate: 0.0,
        mutation_magnitude: 0.0,
        };

        let order = vec![0];
        let pre_tick_energies = vec![50.0];let mut rng = ChaCha8Rng::seed_from_u64(42);



        let result = resolve_interactions(
            &agents,
            &agent_grid,
            &carcass_grid,
            &carcasses,
            &params,
            &order,
            &HashSet::new(),
            &HashMap::new(),
            0,
            &pre_tick_energies,
            &vec![0.0; agents.len()],
        &mut rng,
        100,
        );

        // drain = 4.0 (scavenging_rate, capped at carcass energy 10.0)
        // gain = 4.0 * 0.8 = 3.2
        assert!(
            (result.decomposition_gains[0] - 3.2).abs() < 1e-5,
            "decomposition gain: {}",
            result.decomposition_gains[0]
        );
        // dissipated = 4.0 * (1 - 0.8) = 0.8
        assert!(
            (result.dissipated_energy - 0.8).abs() < 1e-5,
            "dissipated: {}",
            result.dissipated_energy
        );

        // Should have a Decomposed event
        let decomposed: Vec<_> = result
            .events
            .iter()
            .filter(|e| e.kind == event::EventKind::Decomposed)
            .collect();
        assert_eq!(decomposed.len(), 1);
        assert_eq!(decomposed[0].source, 0);
        assert_eq!(decomposed[0].target, Some(99));
        assert!((decomposed[0].energy_delta - 4.0).abs() < 1e-5);
    }

    #[test]
    fn decomposition_depletes_carcass_via_cascade() {
        // A scavenger with rate >= carcass energy fully depletes the carcass
        let agents = vec![Agent {
            id: 0,
            position: (0.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                scavenging_rate: 15.0,
                ..zero_traits()
            },
        }];

        let extent = 100.0;
        let cell_size = 5.0;
        let mut agent_grid = SpatialGrid::new(extent, cell_size);
        agent_grid.insert(0, agents[0].position);

        let carcasses = vec![Carcass {
            id: 42,
            position: (1.0, 0.0),
            energy: 10.0,
        }];
        let mut carcass_grid = SpatialGrid::new(extent, cell_size);
        carcass_grid.insert(0, carcasses[0].position);

        let params = ResolverParams {
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.8,
            world_extent: extent,
            reproduction_energy_threshold: 50.0,
            solar_flux_magnitude: 0.0,
        reproduction_efficiency: 0.7,
        mutation_rate: 0.0,
        mutation_magnitude: 0.0,
        };

        let order = vec![0];
        let pre_tick_energies = vec![50.0];let mut rng = ChaCha8Rng::seed_from_u64(42);



        let result = resolve_interactions(
            &agents,
            &agent_grid,
            &carcass_grid,
            &carcasses,
            &params,
            &order,
            &HashSet::new(),
            &HashMap::new(),
            0,
            &pre_tick_energies,
            &vec![0.0; agents.len()],
        &mut rng,
        100,
        );

        // drain = 10.0 (capped at carcass energy), gain = 10.0 * 0.8 = 8.0
        assert!(
            (result.decomposition_gains[0] - 8.0).abs() < 1e-5,
            "decomposition gain: {}",
            result.decomposition_gains[0]
        );

        // Carcass should be depleted
        assert!(
            result.depleted_carcass_indices.contains(&0),
            "carcass index 0 should be depleted"
        );

        // Event sequence should include Decomposed and CarcassDepleted
        let kinds: Vec<&event::EventKind> =
            result.events.iter().map(|e| &e.kind).collect();
        assert!(kinds.contains(&&event::EventKind::Decomposed));
        assert!(kinds.contains(&&event::EventKind::CarcassDepleted));

        // CarcassDepleted must come after Decomposed
        let decomposed_pos = kinds
            .iter()
            .position(|k| **k == event::EventKind::Decomposed)
            .unwrap();
        let depleted_pos = kinds
            .iter()
            .position(|k| **k == event::EventKind::CarcassDepleted)
            .unwrap();
        assert!(
            decomposed_pos < depleted_pos,
            "Decomposed must precede CarcassDepleted"
        );
    }

    #[test]
    fn photosynthesis_applies_solar_gain_based_on_light_share() {
        // Two agents with different light shares should receive proportional solar gain
        let agents = vec![
            Agent {
                id: 0,
                position: (0.0, 0.0),
                energy: 50.0,
                traits: TraitVector {
                    photosynthetic_absorption: 1.0,
                    mobility: 0.0, // low mobility => gate ~1.0
                    ..zero_traits()
                },
            },
            Agent {
                id: 1,
                position: (10.0, 10.0),
                energy: 50.0,
                traits: TraitVector {
                    photosynthetic_absorption: 1.0,
                    mobility: 0.0,
                    ..zero_traits()
                },
            },
        ];

        let extent = 100.0;
        let cell_size = 5.0;
        let mut agent_grid = SpatialGrid::new(extent, cell_size);
        for (i, a) in agents.iter().enumerate() {
            agent_grid.insert(i as u64, a.position);
        }
        let carcass_grid = SpatialGrid::new(extent, cell_size);

        let params = ResolverParams {
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.5,
            world_extent: extent,
            reproduction_energy_threshold: 50.0,
            solar_flux_magnitude: 10.0,
        reproduction_efficiency: 0.7,
        mutation_rate: 0.0,
        mutation_magnitude: 0.0,
        };

        let order = vec![0, 1];
        let pre_tick_energies = vec![50.0, 50.0];
        let light_shares = vec![0.8, 0.2];let mut rng = ChaCha8Rng::seed_from_u64(42);



        let result = resolve_interactions(
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
            &light_shares,
        &mut rng,
        100,
        );

        // mobility_gate for mobility=0.0: 1/(1+exp(20*(0-0.3))) = 1/(1+exp(-6)) ≈ 0.9975
        let gate = 1.0 / (1.0 + (20.0_f32 * (0.0 - 0.3)).exp());
        let expected_0 = 1.0 * 10.0 * gate * 0.8;
        let expected_1 = 1.0 * 10.0 * gate * 0.2;

        assert!(
            (result.solar_gains[0] - expected_0).abs() < 1e-4,
            "agent 0 solar gain: got {}, expected {}",
            result.solar_gains[0], expected_0
        );
        assert!(
            (result.solar_gains[1] - expected_1).abs() < 1e-4,
            "agent 1 solar gain: got {}, expected {}",
            result.solar_gains[1], expected_1
        );
        assert!(
            (result.total_solar_input - (expected_0 + expected_1)).abs() < 1e-4,
            "total solar input: got {}, expected {}",
            result.total_solar_input, expected_0 + expected_1
        );
    }

    #[test]
    fn mobility_gate_reduces_solar_gain_for_high_mobility() {
        // High-mobility agent should receive much less solar gain than
        // low-mobility agent, even with the same absorption and light share.
        let agents = vec![
            Agent {
                id: 0,
                position: (0.0, 0.0),
                energy: 50.0,
                traits: TraitVector {
                    photosynthetic_absorption: 1.0,
                    mobility: 0.0, // low mobility
                    ..zero_traits()
                },
            },
            Agent {
                id: 1,
                position: (50.0, 50.0),
                energy: 50.0,
                traits: TraitVector {
                    photosynthetic_absorption: 1.0,
                    mobility: 0.8, // high mobility
                    ..zero_traits()
                },
            },
        ];

        let extent = 100.0;
        let cell_size = 5.0;
        let mut agent_grid = SpatialGrid::new(extent, cell_size);
        for (i, a) in agents.iter().enumerate() {
            agent_grid.insert(i as u64, a.position);
        }
        let carcass_grid = SpatialGrid::new(extent, cell_size);

        let params = ResolverParams {
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.5,
            world_extent: extent,
            reproduction_energy_threshold: 50.0,
            solar_flux_magnitude: 10.0,
        reproduction_efficiency: 0.7,
        mutation_rate: 0.0,
        mutation_magnitude: 0.0,
        };

        let order = vec![0, 1];
        let pre_tick_energies = vec![50.0, 50.0];
        let light_shares = vec![0.5, 0.5];let mut rng = ChaCha8Rng::seed_from_u64(42);



        let result = resolve_interactions(
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
            &light_shares,
        &mut rng,
        100,
        );

        let gate_low = 1.0 / (1.0 + (20.0_f32 * (0.0 - 0.3)).exp());
        let gate_high = 1.0 / (1.0 + (20.0_f32 * (0.8 - 0.3)).exp());

        let expected_low = 1.0 * 10.0 * gate_low * 0.5;
        let expected_high = 1.0 * 10.0 * gate_high * 0.5;

        assert!(
            (result.solar_gains[0] - expected_low).abs() < 1e-4,
            "low-mobility gain: got {}, expected {}",
            result.solar_gains[0], expected_low
        );
        assert!(
            (result.solar_gains[1] - expected_high).abs() < 1e-4,
            "high-mobility gain: got {}, expected {}",
            result.solar_gains[1], expected_high
        );
        // High-mobility agent should get significantly less
        assert!(
            result.solar_gains[1] < result.solar_gains[0] * 0.01,
            "high-mobility agent should get <1% of low-mobility agent's gain: {} vs {}",
            result.solar_gains[1], result.solar_gains[0]
        );
    }

    #[test]
    fn consumer_does_not_photosynthesize() {
        // An agent that successfully consumed should not also get solar gain
        let agents = vec![
            Agent {
                id: 0,
                position: (0.0, 0.0),
                energy: 50.0,
                traits: TraitVector {
                    consumption_rate: 2.0,
                    photosynthetic_absorption: 1.0,
                    mobility: 0.0,
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

        let params = ResolverParams {
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.5,
            world_extent: extent,
            reproduction_energy_threshold: 50.0,
            solar_flux_magnitude: 10.0,
        reproduction_efficiency: 0.7,
        mutation_rate: 0.0,
        mutation_magnitude: 0.0,
        };

        let order = vec![0, 1];
        let pre_tick_energies = vec![50.0, 50.0];
        let light_shares = vec![1.0, 1.0];let mut rng = ChaCha8Rng::seed_from_u64(42);



        let result = resolve_interactions(
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
            &light_shares,
        &mut rng,
        100,
        );

        // Agent 0 consumed, so should have zero solar gain
        assert!(
            result.consumption_gains[0] > 0.0,
            "agent 0 should have consumed"
        );
        assert!(
            result.solar_gains[0] == 0.0,
            "consumer should not photosynthesize, got: {}",
            result.solar_gains[0]
        );
    }

    #[test]
    fn decomposer_does_not_photosynthesize() {
        // An agent that successfully decomposed should not also get solar gain
        let agents = vec![Agent {
            id: 0,
            position: (0.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                scavenging_rate: 4.0,
                photosynthetic_absorption: 1.0,
                mobility: 0.0,
                ..zero_traits()
            },
        }];

        let extent = 100.0;
        let cell_size = 5.0;
        let mut agent_grid = SpatialGrid::new(extent, cell_size);
        agent_grid.insert(0, agents[0].position);

        let carcasses = vec![Carcass {
            id: 99,
            position: (2.0, 0.0),
            energy: 10.0,
        }];
        let mut carcass_grid = SpatialGrid::new(extent, cell_size);
        carcass_grid.insert(0, carcasses[0].position);

        let params = ResolverParams {
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.8,
            world_extent: extent,
            reproduction_energy_threshold: 50.0,
            solar_flux_magnitude: 10.0,
        reproduction_efficiency: 0.7,
        mutation_rate: 0.0,
        mutation_magnitude: 0.0,
        };

        let order = vec![0];
        let pre_tick_energies = vec![50.0];
        let light_shares = vec![1.0];let mut rng = ChaCha8Rng::seed_from_u64(42);



        let result = resolve_interactions(
            &agents,
            &agent_grid,
            &carcass_grid,
            &carcasses,
            &params,
            &order,
            &HashSet::new(),
            &HashMap::new(),
            0,
            &pre_tick_energies,
            &light_shares,
        &mut rng,
        100,
        );

        assert!(
            result.decomposition_gains[0] > 0.0,
            "agent 0 should have decomposed"
        );
        assert!(
            result.solar_gains[0] == 0.0,
            "decomposer should not photosynthesize, got: {}",
            result.solar_gains[0]
        );
    }

    #[test]
    fn consumer_does_not_decompose_in_same_tick() {
        // An agent with both consumption_rate and scavenging_rate that
        // successfully consumes should NOT also decompose.
        let agents = vec![
            Agent {
                id: 0,
                position: (0.0, 0.0),
                energy: 50.0,
                traits: TraitVector {
                    consumption_rate: 2.0,
                    scavenging_rate: 4.0,
                    ..zero_traits()
                },
            },
            Agent {
                id: 1,
                position: (1.0, 0.0),
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

        let carcasses = vec![Carcass {
            id: 99,
            position: (1.0, 0.0),
            energy: 10.0,
        }];
        let mut carcass_grid = SpatialGrid::new(extent, cell_size);
        carcass_grid.insert(0, carcasses[0].position);

        let params = ResolverParams {
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.8,
            world_extent: extent,
            reproduction_energy_threshold: 50.0,
            solar_flux_magnitude: 0.0,
        reproduction_efficiency: 0.7,
        mutation_rate: 0.0,
        mutation_magnitude: 0.0,
        };

        let order = vec![0, 1];
        let pre_tick_energies = vec![50.0, 50.0];let mut rng = ChaCha8Rng::seed_from_u64(42);



        let result = resolve_interactions(
            &agents,
            &agent_grid,
            &carcass_grid,
            &carcasses,
            &params,
            &order,
            &HashSet::new(),
            &HashMap::new(),
            0,
            &pre_tick_energies,
            &vec![0.0; agents.len()],
        &mut rng,
        100,
        );

        // Agent 0 should have consumed (gain > 0)
        assert!(
            result.consumption_gains[0] > 0.0,
            "agent 0 should have consumed"
        );
        // Agent 0 should NOT have decomposed
        assert!(
            result.decomposition_gains[0] == 0.0,
            "agent 0 should not decompose after consuming, got: {}",
            result.decomposition_gains[0]
        );
        // No Decomposed events should exist for agent 0
        let decomposed_by_0: Vec<_> = result
            .events
            .iter()
            .filter(|e| {
                e.kind == event::EventKind::Decomposed && e.source == 0
            })
            .collect();
        assert!(
            decomposed_by_0.is_empty(),
            "agent 0 must not emit Decomposed"
        );
    }

    #[test]
    fn reproduction_creates_offspring_with_correct_energy_and_investment() {
        // Two compatible agents above threshold should reproduce:
        // each invests their reproductive_investment, offspring gets
        // (inv_a + inv_b) * reproduction_efficiency
        let shared_traits = TraitVector {
            mobility: 1.0,
            mate_selectivity: 5.0,
            reproductive_investment: 10.0,
            ..zero_traits()
        };
        let agents = vec![
            Agent {
                id: 0,
                position: (0.0, 0.0),
                energy: 50.0,
                traits: shared_traits,
            },
            Agent {
                id: 1,
                position: (2.0, 0.0),
                energy: 50.0,
                traits: shared_traits,
            },
        ];

        let extent = 100.0;
        let cell_size = 5.0;
        let mut agent_grid = SpatialGrid::new(extent, cell_size);
        for (i, a) in agents.iter().enumerate() {
            agent_grid.insert(i as u64, a.position);
        }
        let carcass_grid = SpatialGrid::new(extent, cell_size);

        let params = ResolverParams {
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.5,
            world_extent: extent,
            reproduction_energy_threshold: 10.0,
            solar_flux_magnitude: 0.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
        };

        let order = vec![0, 1];
        let pre_tick_energies = vec![50.0, 50.0];
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_interactions(
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
            &vec![0.0; agents.len()],
            &mut rng,
            100,
        );

        // One offspring should be created
        assert_eq!(result.offspring.len(), 1, "should produce one offspring");

        // Offspring energy = (10 + 10) * 0.7 = 14.0
        let offspring = &result.offspring[0];
        assert!(
            (offspring.energy - 14.0).abs() < 1e-5,
            "offspring energy: {}",
            offspring.energy
        );

        // Parental investments tracked
        assert!(
            (result.reproduction_investments[0] - 10.0).abs() < 1e-5,
            "parent A investment: {}",
            result.reproduction_investments[0]
        );
        assert!(
            (result.reproduction_investments[1] - 10.0).abs() < 1e-5,
            "parent B investment: {}",
            result.reproduction_investments[1]
        );

        // next_agent_id should be incremented
        assert_eq!(result.next_agent_id, 101);
    }

    #[test]
    fn reproduction_efficiency_dissipation_tracked() {
        // With reproduction_efficiency = 0.6, parents investing 15 and 8,
        // dissipated = (15 + 8) * (1 - 0.6) = 9.2
        let agents = vec![
            Agent {
                id: 0,
                position: (0.0, 0.0),
                energy: 50.0,
                traits: TraitVector {
                    mobility: 1.0,
                    mate_selectivity: 10.0,
                    reproductive_investment: 15.0,
                    ..zero_traits()
                },
            },
            Agent {
                id: 1,
                position: (2.0, 0.0),
                energy: 50.0,
                traits: TraitVector {
                    mobility: 1.0,
                    mate_selectivity: 10.0,
                    reproductive_investment: 8.0,
                    ..zero_traits()
                },
            },
        ];

        let extent = 100.0;
        let cell_size = 5.0;
        let mut agent_grid = SpatialGrid::new(extent, cell_size);
        for (i, a) in agents.iter().enumerate() {
            agent_grid.insert(i as u64, a.position);
        }
        let carcass_grid = SpatialGrid::new(extent, cell_size);

        let params = ResolverParams {
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.5,
            world_extent: extent,
            reproduction_energy_threshold: 10.0,
            solar_flux_magnitude: 0.0,
            reproduction_efficiency: 0.6,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
        };

        let order = vec![0, 1];
        let pre_tick_energies = vec![50.0, 50.0];
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let result = resolve_interactions(
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
            &vec![0.0; agents.len()],
            &mut rng,
            100,
        );

        assert_eq!(result.offspring.len(), 1);

        // Offspring energy = (15 + 8) * 0.6 = 13.8
        assert!(
            (result.offspring[0].energy - 13.8).abs() < 1e-5,
            "offspring energy: {}",
            result.offspring[0].energy
        );

        // Dissipated = (15 + 8) * 0.4 = 9.2
        assert!(
            (result.dissipated_energy - 9.2).abs() < 1e-5,
            "dissipated: {}",
            result.dissipated_energy
        );
    }
}
