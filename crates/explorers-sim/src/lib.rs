use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, Normal};

pub fn toroidal_distance(a: (f32, f32), b: (f32, f32), extent: f32) -> f32 {
    let (dx, dy) = toroidal_displacement(a, b, extent);
    (dx * dx + dy * dy).sqrt()
}

fn toroidal_displacement(from: (f32, f32), to: (f32, f32), extent: f32) -> (f32, f32) {
    let mut dx = to.0 - from.0;
    let mut dy = to.1 - from.1;
    if dx > extent / 2.0 {
        dx -= extent;
    } else if dx < -extent / 2.0 {
        dx += extent;
    }
    if dy > extent / 2.0 {
        dy -= extent;
    } else if dy < -extent / 2.0 {
        dy += extent;
    }
    (dx, dy)
}

fn wrap_position(pos: (f32, f32), extent: f32) -> (f32, f32) {
    let half = extent / 2.0;
    let x = (pos.0 + half).rem_euclid(extent) - half;
    let y = (pos.1 + half).rem_euclid(extent) - half;
    (x, y)
}

#[derive(Clone, Copy)]
pub struct TraitVector {
    pub photosynthetic_absorption: f32,
    pub consumption_rate: f32,
    pub scavenging_rate: f32,
    pub mobility: f32,
    pub chemotaxis_sensitivity: f32,
    pub mate_selectivity: f32,
    pub sensing_range: f32,
    pub reproductive_investment: f32,
}

pub struct WorldParameters {
    pub solar_flux_magnitude: f32,
    pub consumption_efficiency: f32,
    pub decomposition_efficiency: f32,
    pub reproduction_efficiency: f32,
    pub base_metabolic_rate: f32,
    pub movement_cost_coefficient: f32,
    pub sensing_cost_coefficient: f32,
    pub reproduction_energy_threshold: f32,
    pub mutation_rate: f32,
    pub mutation_magnitude: f32,
    pub contact_radius: f32,
    pub world_extent: f32,
    pub initial_population_size: u32,
}

pub struct InitialDistribution {
    pub mean_traits: TraitVector,
    pub trait_covariance: f32,
    pub initial_cluster_count: u32,
    pub initial_energy_per_agent: f32,
}

pub struct Agent {
    pub position: (f32, f32),
    pub energy: f32,
    pub traits: TraitVector,
}

pub struct Carcass {
    pub position: (f32, f32),
    pub energy: f32,
}

pub struct World {
    params: WorldParameters,
    agents: Vec<Agent>,
    carcasses: Vec<Carcass>,
    dissipated_energy: f32,
    total_solar_input: f32,
    rng: ChaCha8Rng,
}

impl World {
    pub fn new(params: WorldParameters, distribution: InitialDistribution, seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let extent = params.world_extent;
        let pop_size = params.initial_population_size as usize;

        let pos_dist = rand::distr::Uniform::new(-extent / 2.0, extent / 2.0).unwrap();
        let trait_dist = Normal::new(0.0_f32, distribution.trait_covariance).unwrap();

        let agents = (0..pop_size)
            .map(|_| {
                let x = pos_dist.sample(&mut rng);
                let y = pos_dist.sample(&mut rng);
                let mean = &distribution.mean_traits;
                Agent {
                    position: (x, y),
                    energy: distribution.initial_energy_per_agent,
                    traits: TraitVector {
                        photosynthetic_absorption: mean.photosynthetic_absorption
                            + trait_dist.sample(&mut rng),
                        consumption_rate: mean.consumption_rate + trait_dist.sample(&mut rng),
                        scavenging_rate: mean.scavenging_rate + trait_dist.sample(&mut rng),
                        mobility: mean.mobility + trait_dist.sample(&mut rng),
                        chemotaxis_sensitivity: mean.chemotaxis_sensitivity
                            + trait_dist.sample(&mut rng),
                        mate_selectivity: mean.mate_selectivity + trait_dist.sample(&mut rng),
                        sensing_range: mean.sensing_range + trait_dist.sample(&mut rng),
                        reproductive_investment: mean.reproductive_investment
                            + trait_dist.sample(&mut rng),
                    },
                }
            })
            .collect();

        Self {
            params,
            agents,
            carcasses: Vec::new(),
            dissipated_energy: 0.0,
            total_solar_input: 0.0,
            rng,
        }
    }

    pub fn add_agent(&mut self, agent: Agent) {
        self.agents.push(agent);
    }

    pub fn add_carcass(&mut self, carcass: Carcass) {
        self.carcasses.push(carcass);
    }

    pub fn step(&mut self) {
        let extent = self.params.world_extent;
        let agent_count = self.agents.len();

        // --- Sense & Decide: compute movement vectors for all agents ---
        let movements: Vec<(f32, f32)> = (0..agent_count)
            .map(|i| {
                let agent = &self.agents[i];
                if agent.traits.mobility <= 0.0 {
                    return (0.0, 0.0);
                }

                let mut chemotaxis_x = 0.0_f32;
                let mut chemotaxis_y = 0.0_f32;

                if agent.traits.consumption_rate > 0.0 {
                    for (j, other) in self.agents.iter().enumerate() {
                        if j == i {
                            continue;
                        }
                        let (dx, dy) =
                            toroidal_displacement(agent.position, other.position, extent);
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist > 0.0 && dist <= agent.traits.sensing_range {
                            let signal = agent.traits.consumption_rate / dist;
                            chemotaxis_x += signal * dx / dist;
                            chemotaxis_y += signal * dy / dist;
                        }
                    }
                }

                if agent.traits.scavenging_rate > 0.0 {
                    for carcass in &self.carcasses {
                        let (dx, dy) =
                            toroidal_displacement(agent.position, carcass.position, extent);
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist > 0.0 && dist <= agent.traits.sensing_range {
                            let signal = agent.traits.scavenging_rate / dist;
                            chemotaxis_x += signal * dx / dist;
                            chemotaxis_y += signal * dy / dist;
                        }
                    }
                }

                // Scale by chemotaxis sensitivity
                chemotaxis_x *= agent.traits.chemotaxis_sensitivity;
                chemotaxis_y *= agent.traits.chemotaxis_sensitivity;

                // Random exploration component
                let angle_dist = rand::distr::Uniform::new(0.0_f32, std::f32::consts::TAU).unwrap();
                let angle = angle_dist.sample(&mut self.rng);
                let explore_x = angle.cos();
                let explore_y = angle.sin();

                let mut move_x = chemotaxis_x + explore_x;
                let mut move_y = chemotaxis_y + explore_y;

                // Normalize and scale by mobility
                let mag = (move_x * move_x + move_y * move_y).sqrt();
                if mag > 0.0 {
                    move_x = move_x / mag * agent.traits.mobility;
                    move_y = move_y / mag * agent.traits.mobility;
                }

                (move_x, move_y)
            })
            .collect();

        // --- Act: apply movement, energy, death ---
        let mut next_agents: Vec<Agent> = Vec::with_capacity(agent_count);
        let mut next_carcasses: Vec<Carcass> = self.carcasses.drain(..).collect();

        for (i, agent) in self.agents.iter().enumerate() {
            let (mx, my) = movements[i];
            let new_pos = wrap_position(
                (agent.position.0 + mx, agent.position.1 + my),
                extent,
            );
            let distance_moved = (mx * mx + my * my).sqrt();
            let movement_cost = distance_moved * self.params.movement_cost_coefficient;

            let solar_gain =
                agent.traits.photosynthetic_absorption * self.params.solar_flux_magnitude;
            let metabolic_cost = self.params.base_metabolic_rate
                + agent.traits.sensing_range * self.params.sensing_cost_coefficient;
            self.total_solar_input += solar_gain;
            let energy = agent.energy + solar_gain - metabolic_cost - movement_cost;
            if energy <= 0.0 {
                next_carcasses.push(Carcass {
                    position: new_pos,
                    energy: agent.energy,
                });
                self.dissipated_energy += solar_gain;
            } else {
                self.dissipated_energy += metabolic_cost + movement_cost;
                next_agents.push(Agent {
                    position: new_pos,
                    energy,
                    traits: agent.traits,
                });
            }
        }
        self.agents = next_agents;
        self.carcasses = next_carcasses;
    }

    pub fn params(&self) -> &WorldParameters {
        &self.params
    }

    pub fn agents(&self) -> &[Agent] {
        &self.agents
    }

    pub fn carcasses(&self) -> &[Carcass] {
        &self.carcasses
    }

    pub fn dissipated_energy(&self) -> f32 {
        self.dissipated_energy
    }

    pub fn total_solar_input(&self) -> f32 {
        self.total_solar_input
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trait_vector_has_named_accessors() {
        let traits = TraitVector {
            photosynthetic_absorption: 0.1,
            consumption_rate: 0.2,
            scavenging_rate: 0.3,
            mobility: 0.4,
            chemotaxis_sensitivity: 0.5,
            mate_selectivity: 0.6,
            sensing_range: 0.7,
            reproductive_investment: 0.8,
        };
        assert_eq!(traits.photosynthetic_absorption, 0.1);
        assert_eq!(traits.consumption_rate, 0.2);
        assert_eq!(traits.scavenging_rate, 0.3);
        assert_eq!(traits.mobility, 0.4);
        assert_eq!(traits.chemotaxis_sensitivity, 0.5);
        assert_eq!(traits.mate_selectivity, 0.6);
        assert_eq!(traits.sensing_range, 0.7);
        assert_eq!(traits.reproductive_investment, 0.8);
    }

    fn test_params() -> WorldParameters {
        WorldParameters {
            solar_flux_magnitude: 1.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.5,
            reproduction_efficiency: 0.7,
            base_metabolic_rate: 0.1,
            movement_cost_coefficient: 0.05,
            sensing_cost_coefficient: 0.02,
            reproduction_energy_threshold: 50.0,
            mutation_rate: 0.1,
            mutation_magnitude: 0.05,
            contact_radius: 5.0,
            world_extent: 100.0,
            initial_population_size: 10,
        }
    }

    fn test_distribution() -> InitialDistribution {
        InitialDistribution {
            mean_traits: TraitVector {
                photosynthetic_absorption: 0.5,
                consumption_rate: 0.3,
                scavenging_rate: 0.2,
                mobility: 0.4,
                chemotaxis_sensitivity: 0.3,
                mate_selectivity: 0.5,
                sensing_range: 0.4,
                reproductive_investment: 0.3,
            },
            trait_covariance: 0.1,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        }
    }

    #[test]
    fn world_created_with_params_has_correct_population_size() {
        let world = World::new(test_params(), test_distribution(), 42);
        assert_eq!(world.agents().len(), 10);
    }

    #[test]
    fn same_seed_produces_identical_world() {
        let world1 = World::new(test_params(), test_distribution(), 42);
        let world2 = World::new(test_params(), test_distribution(), 42);
        for (a, b) in world1.agents().iter().zip(world2.agents().iter()) {
            assert_eq!(a.position, b.position);
            assert_eq!(a.energy, b.energy);
            assert_eq!(a.traits.photosynthetic_absorption, b.traits.photosynthetic_absorption);
            assert_eq!(a.traits.consumption_rate, b.traits.consumption_rate);
            assert_eq!(a.traits.mobility, b.traits.mobility);
        }
    }

    #[test]
    fn different_seeds_produce_different_populations() {
        let world1 = World::new(test_params(), test_distribution(), 42);
        let world2 = World::new(test_params(), test_distribution(), 99);
        let any_differ = world1
            .agents()
            .iter()
            .zip(world2.agents().iter())
            .any(|(a, b)| a.position != b.position);
        assert!(any_differ);
    }

    #[test]
    fn agents_start_with_initial_energy() {
        let world = World::new(test_params(), test_distribution(), 42);
        assert!(world.agents().iter().all(|a| a.energy == 100.0));
    }

    #[test]
    fn step_does_not_panic() {
        let mut world = World::new(test_params(), test_distribution(), 42);
        for _ in 0..100 {
            world.step();
        }
        assert_eq!(world.agents().len(), 10);
    }

    #[test]
    fn agent_at_zero_energy_becomes_carcass() {
        let params = WorldParameters {
            base_metabolic_rate: 100.0,
            sensing_cost_coefficient: 0.0,
            solar_flux_magnitude: 0.0,
            initial_population_size: 1,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: TraitVector {
                photosynthetic_absorption: 0.0,
                consumption_rate: 0.0,
                scavenging_rate: 0.0,
                mobility: 0.0,
                chemotaxis_sensitivity: 0.0,
                mate_selectivity: 0.0,
                sensing_range: 0.0,
                reproductive_investment: 0.0,
            },
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        };
        let mut world = World::new(params, dist, 42);
        world.step();
        assert_eq!(world.agents().len(), 0);
        assert_eq!(world.carcasses().len(), 1);
        assert_eq!(world.carcasses()[0].energy, 100.0);
    }

    #[test]
    fn energy_accounting_balances_over_many_ticks() {
        let params = WorldParameters {
            solar_flux_magnitude: 2.0,
            base_metabolic_rate: 0.5,
            sensing_cost_coefficient: 0.1,
            initial_population_size: 5,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: TraitVector {
                photosynthetic_absorption: 0.3,
                consumption_rate: 0.0,
                scavenging_rate: 0.0,
                mobility: 0.0,
                chemotaxis_sensitivity: 0.0,
                mate_selectivity: 0.0,
                sensing_range: 0.5,
                reproductive_investment: 0.0,
            },
            trait_covariance: 0.1,
            initial_cluster_count: 1,
            initial_energy_per_agent: 10.0,
        };
        let mut world = World::new(params, dist, 42);
        let initial_energy: f32 = world.agents().iter().map(|a| a.energy).sum();
        for _ in 0..50 {
            world.step();
        }
        let living_energy: f32 = world.agents().iter().map(|a| a.energy).sum();
        let carcass_energy: f32 = world.carcasses().iter().map(|c| c.energy).sum();
        let total = living_energy + carcass_energy + world.dissipated_energy();
        let expected = initial_energy + world.total_solar_input();
        let diff = (total - expected).abs();
        assert!(
            diff < 1e-3,
            "energy accounting off by {diff}: total={total}, expected={expected}"
        );
    }

    #[test]
    fn non_photosynthesiser_dies_in_predictable_ticks() {
        let base_metabolic_rate: f32 = 0.5;
        let sensing_cost_coefficient: f32 = 0.1;
        let sensing_range: f32 = 2.0;
        let initial_energy: f32 = 10.0;
        let metabolic_cost = base_metabolic_rate + sensing_range * sensing_cost_coefficient;
        let expected_ticks = (initial_energy / metabolic_cost).floor() as u32;
        let params = WorldParameters {
            base_metabolic_rate,
            sensing_cost_coefficient,
            solar_flux_magnitude: 0.0,
            initial_population_size: 1,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: TraitVector {
                photosynthetic_absorption: 0.0,
                consumption_rate: 0.0,
                scavenging_rate: 0.0,
                mobility: 0.0,
                chemotaxis_sensitivity: 0.0,
                mate_selectivity: 0.0,
                sensing_range,
                reproductive_investment: 0.0,
            },
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: initial_energy,
        };
        let mut world = World::new(params, dist, 42);
        for tick in 1..=expected_ticks + 1 {
            world.step();
            if tick < expected_ticks {
                assert_eq!(world.agents().len(), 1, "agent should be alive at tick {tick}");
            }
        }
        assert_eq!(world.agents().len(), 0, "agent should be dead");
        assert_eq!(world.carcasses().len(), 1);
    }

    #[test]
    fn carcasses_persist_without_energy_decay() {
        let params = WorldParameters {
            base_metabolic_rate: 50.0,
            sensing_cost_coefficient: 0.0,
            solar_flux_magnitude: 0.0,
            initial_population_size: 1,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: TraitVector {
                photosynthetic_absorption: 0.0,
                consumption_rate: 0.0,
                scavenging_rate: 0.0,
                mobility: 0.0,
                chemotaxis_sensitivity: 0.0,
                mate_selectivity: 0.0,
                sensing_range: 0.0,
                reproductive_investment: 0.0,
            },
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        };
        let mut world = World::new(params, dist, 42);
        // Tick 1: energy 100 - 50 = 50
        world.step();
        assert_eq!(world.agents().len(), 1);
        // Tick 2: energy 50 - 50 = 0 → carcass
        world.step();
        assert_eq!(world.agents().len(), 0);
        assert_eq!(world.carcasses().len(), 1);
        let carcass_energy = world.carcasses()[0].energy;
        // Tick 3-12: carcass should not decay
        for _ in 0..10 {
            world.step();
        }
        assert_eq!(world.carcasses().len(), 1);
        assert_eq!(world.carcasses()[0].energy, carcass_energy);
    }

    #[test]
    fn agents_pay_metabolic_cost_per_tick() {
        let params = WorldParameters {
            base_metabolic_rate: 0.5,
            sensing_cost_coefficient: 0.1,
            solar_flux_magnitude: 0.0,
            initial_population_size: 1,
            ..test_params()
        };
        let sensing_range = 3.0;
        let dist = InitialDistribution {
            mean_traits: TraitVector {
                photosynthetic_absorption: 0.0,
                consumption_rate: 0.0,
                scavenging_rate: 0.0,
                mobility: 0.0,
                chemotaxis_sensitivity: 0.0,
                mate_selectivity: 0.0,
                sensing_range,
                reproductive_investment: 0.0,
            },
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        };
        let mut world = World::new(params, dist, 42);
        world.step();
        let expected_cost = 0.5 + sensing_range * 0.1;
        assert_eq!(world.agents()[0].energy, 100.0 - expected_cost);
    }

    #[test]
    fn toroidal_distance_wraps_across_edges() {
        let extent = 100.0;
        // Two points near opposite edges: (-48, 0) and (48, 0)
        // Direct distance = 96, but toroidal distance = 4 (wrapping around)
        let a = (-48.0_f32, 0.0_f32);
        let b = (48.0_f32, 0.0_f32);
        let dist = toroidal_distance(a, b, extent);
        assert!((dist - 4.0).abs() < 1e-5, "expected ~4.0 but got {dist}");
    }

    #[test]
    fn toroidal_distance_same_point_is_zero() {
        let p = (10.0_f32, 20.0_f32);
        assert_eq!(toroidal_distance(p, p, 100.0), 0.0);
    }

    #[test]
    fn toroidal_distance_non_wrapping_is_euclidean() {
        let a = (0.0_f32, 0.0_f32);
        let b = (3.0_f32, 4.0_f32);
        let dist = toroidal_distance(a, b, 100.0);
        assert!((dist - 5.0).abs() < 1e-5);
    }

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
    fn consumer_moves_toward_living_agent() {
        let extent = 100.0;
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            world_extent: extent,
            initial_population_size: 0,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        };
        let mut world = World::new(params, dist, 42);
        // Manually place two agents: consumer at (0,0), food at (20,0)
        world.add_agent(Agent {
            position: (0.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                consumption_rate: 1.0,
                mobility: 5.0,
                chemotaxis_sensitivity: 1.0,
                sensing_range: 50.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            position: (20.0, 0.0),
            energy: 100.0,
            traits: zero_traits(),
        });
        let initial_dist = toroidal_distance(
            world.agents()[0].position,
            world.agents()[1].position,
            extent,
        );
        world.step();
        let final_dist = toroidal_distance(
            world.agents()[0].position,
            world.agents()[1].position,
            extent,
        );
        assert!(
            final_dist < initial_dist,
            "consumer should move closer: initial={initial_dist}, final={final_dist}"
        );
    }

    #[test]
    fn scavenger_moves_toward_carcass() {
        let extent = 100.0;
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            world_extent: extent,
            initial_population_size: 0,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        };
        let mut world = World::new(params, dist, 42);
        world.add_agent(Agent {
            position: (0.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                scavenging_rate: 1.0,
                mobility: 5.0,
                chemotaxis_sensitivity: 1.0,
                sensing_range: 50.0,
                ..zero_traits()
            },
        });
        world.add_carcass(Carcass {
            position: (20.0, 0.0),
            energy: 50.0,
        });
        let initial_dist = toroidal_distance(
            world.agents()[0].position,
            world.carcasses()[0].position,
            extent,
        );
        world.step();
        let final_dist = toroidal_distance(
            world.agents()[0].position,
            world.carcasses()[0].position,
            extent,
        );
        assert!(
            final_dist < initial_dist,
            "scavenger should move closer to carcass: initial={initial_dist}, final={final_dist}"
        );
    }

    #[test]
    fn sensing_detects_across_toroidal_boundary() {
        let extent = 100.0;
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            world_extent: extent,
            initial_population_size: 0,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        };
        let mut world = World::new(params, dist, 42);
        // Consumer at -48, food at +48 → toroidal distance = 4
        world.add_agent(Agent {
            position: (-48.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                consumption_rate: 1.0,
                mobility: 5.0,
                chemotaxis_sensitivity: 1.0,
                sensing_range: 10.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            position: (48.0, 0.0),
            energy: 100.0,
            traits: zero_traits(),
        });
        let initial_pos = world.agents()[0].position;
        world.step();
        let final_pos = world.agents()[0].position;
        // Should move in the negative-x direction (toward the boundary, wrapping to +48)
        assert!(
            final_pos.0 < initial_pos.0 || final_pos.0 > 45.0,
            "consumer should move toward food across boundary, not away: initial_x={}, final_x={}",
            initial_pos.0,
            final_pos.0
        );
    }

    #[test]
    fn same_seed_produces_identical_movement() {
        let params_fn = || WorldParameters {
            initial_population_size: 5,
            ..test_params()
        };
        let dist_fn = || test_distribution();
        let seed = 42;

        let mut world1 = World::new(params_fn(), dist_fn(), seed);
        let mut world2 = World::new(params_fn(), dist_fn(), seed);
        for _ in 0..10 {
            world1.step();
            world2.step();
        }
        for (a, b) in world1.agents().iter().zip(world2.agents().iter()) {
            assert_eq!(a.position, b.position);
            assert_eq!(a.energy, b.energy);
        }
    }

    #[test]
    fn agent_wraps_around_world_edge_after_movement() {
        let extent = 100.0;
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            world_extent: extent,
            initial_population_size: 0,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        };
        let mut world = World::new(params, dist, 42);
        // Place consumer near edge, food just across boundary
        // Consumer at (48, 0), food at (-48, 0) → toroidal dist = 4
        world.add_agent(Agent {
            position: (48.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                consumption_rate: 1.0,
                mobility: 5.0,
                chemotaxis_sensitivity: 10.0, // strong chemotaxis to dominate random
                sensing_range: 50.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            position: (-48.0, 0.0),
            energy: 100.0,
            traits: zero_traits(),
        });
        world.step();
        let pos = world.agents()[0].position;
        let half = extent / 2.0;
        assert!(
            pos.0 >= -half && pos.0 < half && pos.1 >= -half && pos.1 < half,
            "position should be within world bounds: ({}, {})",
            pos.0,
            pos.1
        );
    }

    #[test]
    fn movement_costs_energy_proportional_to_distance() {
        let extent = 100.0;
        let movement_cost_coefficient = 0.5;
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient,
            world_extent: extent,
            initial_population_size: 0,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        };
        let mut world = World::new(params, dist, 42);
        let mobility = 3.0;
        world.add_agent(Agent {
            position: (0.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                mobility,
                ..zero_traits()
            },
        });
        world.step();
        let pos = world.agents()[0].position;
        let distance_moved = (pos.0 * pos.0 + pos.1 * pos.1).sqrt();
        let expected_cost = distance_moved * movement_cost_coefficient;
        let expected_energy = 100.0 - expected_cost;
        assert!(
            (world.agents()[0].energy - expected_energy).abs() < 1e-5,
            "energy={}, expected={}",
            world.agents()[0].energy,
            expected_energy
        );
    }

    #[test]
    fn zero_mobility_agent_does_not_move_or_pay_movement_cost() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.5,
            initial_population_size: 1,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: TraitVector {
                photosynthetic_absorption: 0.0,
                consumption_rate: 0.0,
                scavenging_rate: 0.0,
                mobility: 0.0,
                chemotaxis_sensitivity: 0.0,
                mate_selectivity: 0.0,
                sensing_range: 0.0,
                reproductive_investment: 0.0,
            },
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        };
        let mut world = World::new(params, dist, 42);
        let initial_pos = world.agents()[0].position;
        let initial_energy = world.agents()[0].energy;
        world.step();
        assert_eq!(world.agents()[0].position, initial_pos);
        assert_eq!(world.agents()[0].energy, initial_energy);
    }

    #[test]
    fn agents_gain_energy_from_solar_flux() {
        let params = WorldParameters {
            initial_population_size: 1,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: TraitVector {
                photosynthetic_absorption: 0.8,
                consumption_rate: 0.0,
                scavenging_rate: 0.0,
                mobility: 0.0,
                chemotaxis_sensitivity: 0.0,
                mate_selectivity: 0.0,
                sensing_range: 0.0,
                reproductive_investment: 0.0,
            },
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        };
        let mut world = World::new(params, dist, 42);
        let initial_energy = world.agents()[0].energy;
        world.step();
        let photosynthesis = world.agents()[0].traits.photosynthetic_absorption
            * world.params().solar_flux_magnitude;
        let metabolic_cost = world.params().base_metabolic_rate
            + world.agents()[0].traits.sensing_range * world.params().sensing_cost_coefficient;
        let expected = initial_energy + photosynthesis - metabolic_cost;
        assert_eq!(world.agents()[0].energy, expected);
    }
}
