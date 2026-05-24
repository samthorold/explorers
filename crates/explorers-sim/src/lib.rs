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

#[derive(Clone, Copy, Debug)]
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

impl TraitVector {
    pub fn distance(&self, other: &TraitVector) -> f32 {
        let d0 = self.photosynthetic_absorption - other.photosynthetic_absorption;
        let d1 = self.consumption_rate - other.consumption_rate;
        let d2 = self.scavenging_rate - other.scavenging_rate;
        let d3 = self.mobility - other.mobility;
        let d4 = self.chemotaxis_sensitivity - other.chemotaxis_sensitivity;
        let d5 = self.mate_selectivity - other.mate_selectivity;
        let d6 = self.sensing_range - other.sensing_range;
        let d7 = self.reproductive_investment - other.reproductive_investment;
        (d0 * d0 + d1 * d1 + d2 * d2 + d3 * d3 + d4 * d4 + d5 * d5 + d6 * d6 + d7 * d7).sqrt()
    }

    fn get(&self, index: usize) -> f32 {
        match index {
            0 => self.photosynthetic_absorption,
            1 => self.consumption_rate,
            2 => self.scavenging_rate,
            3 => self.mobility,
            4 => self.chemotaxis_sensitivity,
            5 => self.mate_selectivity,
            6 => self.sensing_range,
            7 => self.reproductive_investment,
            _ => unreachable!(),
        }
    }

    fn set(&mut self, index: usize, value: f32) {
        match index {
            0 => self.photosynthetic_absorption = value,
            1 => self.consumption_rate = value,
            2 => self.scavenging_rate = value,
            3 => self.mobility = value,
            4 => self.chemotaxis_sensitivity = value,
            5 => self.mate_selectivity = value,
            6 => self.sensing_range = value,
            7 => self.reproductive_investment = value,
            _ => unreachable!(),
        }
    }
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
    last_tick_births: usize,
    last_tick_deaths: usize,
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
            last_tick_births: 0,
            last_tick_deaths: 0,
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

        // --- Act: apply movement, energy ---
        let mut agents: Vec<Agent> = Vec::with_capacity(agent_count);
        let mut pre_tick_energies: Vec<f32> = Vec::with_capacity(agent_count);
        let mut solar_gains: Vec<f32> = Vec::with_capacity(agent_count);

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
            pre_tick_energies.push(agent.energy);
            solar_gains.push(solar_gain);
            agents.push(Agent {
                position: new_pos,
                energy,
                traits: agent.traits,
            });
        }

        // --- Consumption: agents drain energy from living targets within contact_radius ---
        let contact_radius = self.params.contact_radius;
        let consumption_efficiency = self.params.consumption_efficiency;
        let n = agents.len();
        let mut consumption_deltas = vec![0.0_f32; n];

        for i in 0..n {
            if agents[i].traits.consumption_rate <= 0.0 {
                continue;
            }
            let mut best_target: Option<usize> = None;
            let mut best_dist = f32::MAX;
            for j in 0..n {
                if j == i {
                    continue;
                }
                let dist = toroidal_distance(agents[i].position, agents[j].position, extent);
                if dist < contact_radius && dist < best_dist {
                    best_dist = dist;
                    best_target = Some(j);
                }
            }
            if let Some(target) = best_target {
                let available = (agents[target].energy + consumption_deltas[target]).max(0.0);
                let drain = agents[i].traits.consumption_rate.min(available);
                if drain > 0.0 {
                    consumption_deltas[i] += drain * consumption_efficiency;
                    consumption_deltas[target] -= drain;
                    self.dissipated_energy += drain * (1.0 - consumption_efficiency);
                }
            }
        }

        for i in 0..n {
            agents[i].energy += consumption_deltas[i];
        }

        // --- Decomposition: agents drain energy from carcasses within contact_radius ---
        let decomposition_efficiency = self.params.decomposition_efficiency;
        let mut carcass_drains = vec![0.0_f32; self.carcasses.len()];
        let mut decomposition_gains = vec![0.0_f32; n];

        for i in 0..n {
            if agents[i].traits.scavenging_rate <= 0.0 {
                continue;
            }
            let mut best_carcass: Option<usize> = None;
            let mut best_dist = f32::MAX;
            for (ci, carcass) in self.carcasses.iter().enumerate() {
                let dist = toroidal_distance(agents[i].position, carcass.position, extent);
                if dist < contact_radius && dist < best_dist {
                    best_dist = dist;
                    best_carcass = Some(ci);
                }
            }
            if let Some(ci) = best_carcass {
                let available = (self.carcasses[ci].energy - carcass_drains[ci]).max(0.0);
                let drain = agents[i].traits.scavenging_rate.min(available);
                if drain > 0.0 {
                    decomposition_gains[i] = drain * decomposition_efficiency;
                    agents[i].energy += decomposition_gains[i];
                    carcass_drains[ci] += drain;
                    self.dissipated_energy += drain * (1.0 - decomposition_efficiency);
                }
            }
        }

        for (ci, carcass) in self.carcasses.iter_mut().enumerate() {
            carcass.energy -= carcass_drains[ci];
        }
        self.carcasses.retain(|c| c.energy > 0.0);

        // --- Reproduction: find compatible mates and produce offspring ---
        let mut reproduced = vec![false; n];
        let mut reproduction_investments = vec![0.0_f32; n];
        let mut offspring: Vec<Agent> = Vec::new();
        let reproduction_threshold = self.params.reproduction_energy_threshold;
        let reproduction_efficiency = self.params.reproduction_efficiency;
        let mutation_rate = self.params.mutation_rate;
        let mutation_magnitude = self.params.mutation_magnitude;

        for i in 0..n {
            if reproduced[i] || agents[i].energy <= reproduction_threshold {
                continue;
            }
            let mut best_mate: Option<usize> = None;
            let mut best_dist = f32::MAX;
            for j in 0..n {
                if j == i || reproduced[j] || agents[j].energy <= reproduction_threshold {
                    continue;
                }
                let spatial_dist =
                    toroidal_distance(agents[i].position, agents[j].position, extent);
                if spatial_dist > contact_radius {
                    continue;
                }
                let trait_dist = agents[i].traits.distance(&agents[j].traits);
                if trait_dist < agents[i].traits.mate_selectivity
                    && trait_dist < agents[j].traits.mate_selectivity
                    && spatial_dist < best_dist
                {
                    best_dist = spatial_dist;
                    best_mate = Some(j);
                }
            }
            if let Some(j) = best_mate {
                reproduced[i] = true;
                reproduced[j] = true;

                let inv_a = agents[i].traits.reproductive_investment;
                let inv_b = agents[j].traits.reproductive_investment;
                agents[i].energy -= inv_a;
                agents[j].energy -= inv_b;
                reproduction_investments[i] = inv_a;
                reproduction_investments[j] = inv_b;

                let offspring_energy = (inv_a + inv_b) * reproduction_efficiency;
                self.dissipated_energy += (inv_a + inv_b) * (1.0 - reproduction_efficiency);

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
                        .sample(&mut self.rng)
                        == 0;
                    let val = if from_a {
                        agents[i].traits.get(dim)
                    } else {
                        agents[j].traits.get(dim)
                    };
                    child_traits.set(dim, val);
                }

                if mutation_rate > 0.0 {
                    let mutation_dist = Normal::new(0.0_f32, mutation_magnitude).unwrap();
                    for dim in 0..8 {
                        let r: f32 = rand::distr::Uniform::new(0.0_f32, 1.0)
                            .unwrap()
                            .sample(&mut self.rng);
                        if r < mutation_rate {
                            let perturbation = mutation_dist.sample(&mut self.rng);
                            child_traits.set(dim, child_traits.get(dim) + perturbation);
                        }
                    }
                }

                offspring.push(Agent {
                    position: agents[i].position,
                    energy: offspring_energy,
                    traits: child_traits,
                });
            }
        }

        // --- Death check and carcass creation ---
        let mut next_agents: Vec<Agent> = Vec::with_capacity(n);
        let mut next_carcasses: Vec<Carcass> = self.carcasses.drain(..).collect();

        for (i, agent) in agents.into_iter().enumerate() {
            if agent.energy <= 0.0 {
                let carcass_energy = (pre_tick_energies[i] + consumption_deltas[i]).max(0.0);
                next_carcasses.push(Carcass {
                    position: agent.position,
                    energy: carcass_energy,
                });
                self.dissipated_energy += solar_gains[i] + decomposition_gains[i];
            } else {
                let total_input = solar_gains[i] + consumption_deltas[i] + decomposition_gains[i];
                let costs =
                    pre_tick_energies[i] + total_input - agent.energy - reproduction_investments[i];
                self.dissipated_energy += costs;
                next_agents.push(agent);
            }
        }

        self.last_tick_births = offspring.len();
        self.last_tick_deaths = n - next_agents.len();
        next_agents.extend(offspring);
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

    pub fn last_tick_births(&self) -> usize {
        self.last_tick_births
    }

    pub fn last_tick_deaths(&self) -> usize {
        self.last_tick_deaths
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
    fn consumer_drains_energy_from_living_agent_within_contact_radius() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
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
        // Consumer at (0,0), target at (3,0) — within contact_radius of 5
        world.add_agent(Agent {
            position: (0.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                consumption_rate: 2.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            position: (3.0, 0.0),
            energy: 50.0,
            traits: zero_traits(),
        });
        world.step();
        // Consumer should have gained energy, target should have lost energy
        let consumer = &world.agents()[0];
        let target = &world.agents()[1];
        // Drained = consumption_rate = 2.0, gained = 2.0 * 0.5 = 1.0
        assert_eq!(consumer.energy, 50.0 + 1.0);
        assert_eq!(target.energy, 50.0 - 2.0);
    }

    #[test]
    fn consumption_energy_accounting_with_efficiency() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            consumption_efficiency: 0.7,
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
            energy: 50.0,
            traits: TraitVector {
                consumption_rate: 3.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            position: (2.0, 0.0),
            energy: 50.0,
            traits: zero_traits(),
        });
        world.step();
        // Drain = 3.0, gained = 3.0 * 0.7 = 2.1, dissipated = 3.0 * 0.3 = 0.9
        assert!((world.agents()[0].energy - 52.1).abs() < 1e-5);
        assert!((world.agents()[1].energy - 47.0).abs() < 1e-5);
        assert!((world.dissipated_energy() - 0.9).abs() < 1e-5);
    }

    #[test]
    fn consumed_agent_becomes_carcass_at_zero_energy() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
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
            energy: 50.0,
            traits: TraitVector {
                consumption_rate: 10.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            position: (2.0, 0.0),
            energy: 5.0,
            traits: zero_traits(),
        });
        world.step();
        // Drain capped at target's energy = 5.0
        // Consumer gains 5.0 * 0.5 = 2.5
        // Target dies → carcass with 0 energy (fully drained)
        assert_eq!(world.agents().len(), 1);
        assert!((world.agents()[0].energy - 52.5).abs() < 1e-5);
        assert_eq!(world.carcasses().len(), 1);
        assert_eq!(world.carcasses()[0].energy, 0.0);
    }

    #[test]
    fn scavenger_drains_energy_from_carcass_within_contact_radius() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            decomposition_efficiency: 0.6,
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
            energy: 50.0,
            traits: TraitVector {
                scavenging_rate: 4.0,
                ..zero_traits()
            },
        });
        world.add_carcass(Carcass {
            position: (3.0, 0.0),
            energy: 20.0,
        });
        world.step();
        // Drain = 4.0, gained = 4.0 * 0.6 = 2.4, dissipated = 4.0 * 0.4 = 1.6
        assert!((world.agents()[0].energy - 52.4).abs() < 1e-5);
        assert!((world.carcasses()[0].energy - 16.0).abs() < 1e-5);
        assert!((world.dissipated_energy() - 1.6).abs() < 1e-5);
    }

    #[test]
    fn fully_decomposed_carcass_is_removed() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            decomposition_efficiency: 0.5,
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
            energy: 50.0,
            traits: TraitVector {
                scavenging_rate: 10.0,
                ..zero_traits()
            },
        });
        world.add_carcass(Carcass {
            position: (2.0, 0.0),
            energy: 6.0,
        });
        world.step();
        // Drain capped at carcass energy = 6.0
        // Gained = 6.0 * 0.5 = 3.0, dissipated = 6.0 * 0.5 = 3.0
        assert!((world.agents()[0].energy - 53.0).abs() < 1e-5);
        assert_eq!(world.carcasses().len(), 0);
        assert!((world.dissipated_energy() - 3.0).abs() < 1e-5);
    }

    #[test]
    fn agent_with_both_traits_consumes_and_decomposes_in_one_tick() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.5,
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
        // Agent with both consumption and scavenging
        world.add_agent(Agent {
            position: (0.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                consumption_rate: 2.0,
                scavenging_rate: 3.0,
                ..zero_traits()
            },
        });
        // Living target
        world.add_agent(Agent {
            position: (1.0, 0.0),
            energy: 50.0,
            traits: zero_traits(),
        });
        // Carcass
        world.add_carcass(Carcass {
            position: (0.0, 1.0),
            energy: 20.0,
        });
        world.step();
        // Consumption: drain 2.0 from living, gain 1.0
        // Decomposition: drain 3.0 from carcass, gain 1.5
        assert!((world.agents()[0].energy - 52.5).abs() < 1e-5);
        assert!((world.agents()[1].energy - 48.0).abs() < 1e-5);
        assert!((world.carcasses()[0].energy - 17.0).abs() < 1e-5);
    }

    #[test]
    fn zero_rate_agent_does_not_drain_even_in_contact() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
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
            energy: 50.0,
            traits: zero_traits(),
        });
        world.add_agent(Agent {
            position: (1.0, 0.0),
            energy: 50.0,
            traits: zero_traits(),
        });
        world.add_carcass(Carcass {
            position: (0.0, 1.0),
            energy: 20.0,
        });
        world.step();
        assert_eq!(world.agents()[0].energy, 50.0);
        assert_eq!(world.agents()[1].energy, 50.0);
        assert_eq!(world.carcasses()[0].energy, 20.0);
    }

    #[test]
    fn contact_uses_toroidal_distance() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
            world_extent: 100.0,
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
        // Consumer at -48, target at +48 → toroidal distance = 4 (within contact_radius 5)
        world.add_agent(Agent {
            position: (-48.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                consumption_rate: 2.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            position: (48.0, 0.0),
            energy: 50.0,
            traits: zero_traits(),
        });
        world.step();
        // Should drain across boundary
        assert!((world.agents()[0].energy - 51.0).abs() < 1e-5);
        assert!((world.agents()[1].energy - 48.0).abs() < 1e-5);
    }

    #[test]
    fn energy_accounting_holds_with_consumption_and_decomposition() {
        let params = WorldParameters {
            solar_flux_magnitude: 1.5,
            base_metabolic_rate: 0.3,
            sensing_cost_coefficient: 0.05,
            movement_cost_coefficient: 0.02,
            contact_radius: 8.0,
            consumption_efficiency: 0.6,
            decomposition_efficiency: 0.4,
            initial_population_size: 10,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: TraitVector {
                photosynthetic_absorption: 0.3,
                consumption_rate: 0.5,
                scavenging_rate: 0.3,
                mobility: 0.4,
                chemotaxis_sensitivity: 0.3,
                mate_selectivity: 0.0,
                sensing_range: 5.0,
                reproductive_investment: 0.0,
            },
            trait_covariance: 0.1,
            initial_cluster_count: 1,
            initial_energy_per_agent: 20.0,
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
            diff < 1e-2,
            "energy accounting off by {diff}: total={total}, expected={expected}"
        );
    }

    #[test]
    fn no_consumption_when_outside_contact_radius() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
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
        // Consumer at (0,0), target at (10,0) — outside contact_radius of 5
        world.add_agent(Agent {
            position: (0.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                consumption_rate: 2.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            position: (10.0, 0.0),
            energy: 50.0,
            traits: zero_traits(),
        });
        world.step();
        assert_eq!(world.agents()[0].energy, 50.0);
        assert_eq!(world.agents()[1].energy, 50.0);
    }

    #[test]
    fn compatible_agents_produce_one_offspring() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            reproduction_efficiency: 0.7,
            reproduction_energy_threshold: 10.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
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
        let shared_traits = TraitVector {
            mate_selectivity: 5.0,
            reproductive_investment: 10.0,
            ..zero_traits()
        };
        world.add_agent(Agent {
            position: (0.0, 0.0),
            energy: 50.0,
            traits: shared_traits,
        });
        world.add_agent(Agent {
            position: (2.0, 0.0),
            energy: 50.0,
            traits: shared_traits,
        });
        world.step();
        assert_eq!(
            world.agents().len(),
            3,
            "two compatible agents should produce one offspring"
        );
    }

    #[test]
    fn parents_lose_energy_equal_to_reproductive_investment() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            reproduction_efficiency: 0.7,
            reproduction_energy_threshold: 10.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
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
            energy: 50.0,
            traits: TraitVector {
                mate_selectivity: 10.0,
                reproductive_investment: 15.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            position: (2.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                mate_selectivity: 10.0,
                reproductive_investment: 8.0,
                ..zero_traits()
            },
        });
        world.step();
        assert_eq!(world.agents().len(), 3);
        assert!((world.agents()[0].energy - 35.0).abs() < 1e-5, "parent A: {}", world.agents()[0].energy);
        assert!((world.agents()[1].energy - 42.0).abs() < 1e-5, "parent B: {}", world.agents()[1].energy);
    }

    #[test]
    fn offspring_energy_is_lossy() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            reproduction_efficiency: 0.7,
            reproduction_energy_threshold: 10.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
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
            energy: 50.0,
            traits: TraitVector {
                mate_selectivity: 10.0,
                reproductive_investment: 15.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            position: (2.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                mate_selectivity: 10.0,
                reproductive_investment: 8.0,
                ..zero_traits()
            },
        });
        world.step();
        // Offspring energy = (15 + 8) * 0.7 = 16.1
        let offspring = &world.agents()[2];
        assert!(
            (offspring.energy - 16.1).abs() < 1e-5,
            "offspring energy: {}",
            offspring.energy
        );
    }

    #[test]
    fn below_energy_threshold_no_reproduction() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            reproduction_energy_threshold: 50.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
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
        let shared_traits = TraitVector {
            mate_selectivity: 5.0,
            reproductive_investment: 10.0,
            ..zero_traits()
        };
        world.add_agent(Agent {
            position: (0.0, 0.0),
            energy: 40.0, // below threshold of 50
            traits: shared_traits,
        });
        world.add_agent(Agent {
            position: (2.0, 0.0),
            energy: 100.0,
            traits: shared_traits,
        });
        world.step();
        assert_eq!(world.agents().len(), 2, "no reproduction when one parent below threshold");
    }

    #[test]
    fn no_reproduction_outside_contact_radius() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            reproduction_energy_threshold: 10.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
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
        let shared_traits = TraitVector {
            mate_selectivity: 5.0,
            reproductive_investment: 10.0,
            ..zero_traits()
        };
        world.add_agent(Agent {
            position: (0.0, 0.0),
            energy: 50.0,
            traits: shared_traits,
        });
        world.add_agent(Agent {
            position: (20.0, 0.0), // far outside contact_radius of 5
            energy: 50.0,
            traits: shared_traits,
        });
        world.step();
        assert_eq!(world.agents().len(), 2, "no reproduction when outside contact radius");
    }

    #[test]
    fn asymmetric_mate_selectivity_prevents_reproduction() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            reproduction_energy_threshold: 10.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
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
        // Agent A has high selectivity (accepts B), agent B has low selectivity (rejects A)
        // Trait distance between them = sqrt((10-0.5)^2) = 9.5 (via mate_selectivity difference)
        world.add_agent(Agent {
            position: (0.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                mate_selectivity: 10.0, // accepts trait dist < 10
                reproductive_investment: 10.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            position: (2.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                mate_selectivity: 0.5, // rejects trait dist >= 0.5
                reproductive_investment: 10.0,
                ..zero_traits()
            },
        });
        world.step();
        assert_eq!(world.agents().len(), 2, "asymmetric selectivity should prevent reproduction");
    }

    #[test]
    fn agent_reproduces_at_most_once_per_tick() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            reproduction_efficiency: 0.7,
            reproduction_energy_threshold: 10.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
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
        let shared_traits = TraitVector {
            mate_selectivity: 5.0,
            reproductive_investment: 5.0,
            ..zero_traits()
        };
        // Three compatible agents all in contact — should produce at most 1 offspring (one pair)
        world.add_agent(Agent {
            position: (0.0, 0.0),
            energy: 100.0,
            traits: shared_traits,
        });
        world.add_agent(Agent {
            position: (1.0, 0.0),
            energy: 100.0,
            traits: shared_traits,
        });
        world.add_agent(Agent {
            position: (2.0, 0.0),
            energy: 100.0,
            traits: shared_traits,
        });
        world.step();
        // One pair reproduces, one agent left out → 3 + 1 = 4
        assert_eq!(world.agents().len(), 4, "three agents should produce exactly one offspring");
    }

    #[test]
    fn uniform_crossover_draws_each_dimension_from_one_parent() {
        let mut from_a_counts = [0u32; 8];
        let total_offspring = 200;

        for seed in 0..total_offspring {
            let params = WorldParameters {
                solar_flux_magnitude: 0.0,
                base_metabolic_rate: 0.0,
                sensing_cost_coefficient: 0.0,
                movement_cost_coefficient: 0.0,
                contact_radius: 5.0,
                reproduction_efficiency: 0.7,
                reproduction_energy_threshold: 10.0,
                mutation_rate: 0.0,
                mutation_magnitude: 0.0,
                initial_population_size: 0,
                ..test_params()
            };
            let dist = InitialDistribution {
                mean_traits: zero_traits(),
                trait_covariance: 0.0,
                initial_cluster_count: 1,
                initial_energy_per_agent: 100.0,
            };
            let mut world = World::new(params, dist, seed);
            // Parents with distinct trait values per dimension so we can tell who contributed
            world.add_agent(Agent {
                position: (0.0, 0.0),
                energy: 100.0,
                traits: TraitVector {
                    photosynthetic_absorption: 1.0,
                    consumption_rate: 1.0,
                    scavenging_rate: 1.0,
                    mobility: 1.0,
                    chemotaxis_sensitivity: 1.0,
                    mate_selectivity: 100.0,
                    sensing_range: 1.0,
                    reproductive_investment: 10.0,
                },
            });
            world.add_agent(Agent {
                position: (1.0, 0.0),
                energy: 100.0,
                traits: TraitVector {
                    photosynthetic_absorption: 2.0,
                    consumption_rate: 2.0,
                    scavenging_rate: 2.0,
                    mobility: 2.0,
                    chemotaxis_sensitivity: 2.0,
                    mate_selectivity: 101.0,
                    sensing_range: 2.0,
                    reproductive_investment: 20.0,
                },
            });
            world.step();
            assert_eq!(world.agents().len(), 3);
            let child = &world.agents()[2];
            let parent_a_vals = [1.0, 1.0, 1.0, 1.0, 1.0, 100.0, 1.0, 10.0];
            for dim in 0..8 {
                let val = child.traits.get(dim);
                if (val - parent_a_vals[dim]).abs() < 1e-5 {
                    from_a_counts[dim] += 1;
                }
            }
        }

        // Each dimension should be ~50% from parent A (binomial, p=0.5, n=200)
        // 99% CI is roughly [78, 122] for 200 trials
        for dim in 0..8 {
            let count = from_a_counts[dim];
            assert!(
                count > 60 && count < 140,
                "dim {dim}: from_a={count}/{total_offspring}, expected ~50%"
            );
        }
    }

    #[test]
    fn mutation_perturbs_offspring_traits() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            reproduction_efficiency: 0.7,
            reproduction_energy_threshold: 10.0,
            mutation_rate: 1.0, // mutate every dimension
            mutation_magnitude: 0.5,
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
        let shared_traits = TraitVector {
            mate_selectivity: 5.0,
            reproductive_investment: 10.0,
            ..zero_traits()
        };
        world.add_agent(Agent {
            position: (0.0, 0.0),
            energy: 100.0,
            traits: shared_traits,
        });
        world.add_agent(Agent {
            position: (1.0, 0.0),
            energy: 100.0,
            traits: shared_traits,
        });
        world.step();
        assert_eq!(world.agents().len(), 3);
        let child = &world.agents()[2];
        // With mutation_rate=1.0, every dimension should be perturbed
        // At least some dimensions should differ from both parents
        let mut any_mutated = false;
        for dim in 0..8 {
            let val = child.traits.get(dim);
            let parent_val = shared_traits.get(dim);
            if (val - parent_val).abs() > 1e-10 {
                any_mutated = true;
            }
        }
        assert!(any_mutated, "mutation_rate=1.0 should perturb at least one dimension");
    }

    #[test]
    fn reproduction_is_deterministic_given_seed() {
        let make_world = |seed| {
            let params = WorldParameters {
                solar_flux_magnitude: 0.0,
                base_metabolic_rate: 0.0,
                sensing_cost_coefficient: 0.0,
                movement_cost_coefficient: 0.0,
                contact_radius: 5.0,
                reproduction_efficiency: 0.7,
                reproduction_energy_threshold: 10.0,
                mutation_rate: 0.5,
                mutation_magnitude: 0.3,
                initial_population_size: 0,
                ..test_params()
            };
            let dist = InitialDistribution {
                mean_traits: zero_traits(),
                trait_covariance: 0.0,
                initial_cluster_count: 1,
                initial_energy_per_agent: 100.0,
            };
            let mut world = World::new(params, dist, seed);
            let shared_traits = TraitVector {
                mate_selectivity: 5.0,
                reproductive_investment: 10.0,
                ..zero_traits()
            };
            world.add_agent(Agent {
                position: (0.0, 0.0),
                energy: 100.0,
                traits: shared_traits,
            });
            world.add_agent(Agent {
                position: (1.0, 0.0),
                energy: 100.0,
                traits: shared_traits,
            });
            world.step();
            world
        };
        let w1 = make_world(42);
        let w2 = make_world(42);
        assert_eq!(w1.agents().len(), w2.agents().len());
        for (a, b) in w1.agents().iter().zip(w2.agents().iter()) {
            assert_eq!(a.energy, b.energy);
            for dim in 0..8 {
                assert_eq!(a.traits.get(dim), b.traits.get(dim));
            }
        }
    }

    #[test]
    fn energy_accounting_holds_with_reproduction() {
        let params = WorldParameters {
            solar_flux_magnitude: 2.0,
            base_metabolic_rate: 0.3,
            sensing_cost_coefficient: 0.05,
            movement_cost_coefficient: 0.02,
            contact_radius: 8.0,
            consumption_efficiency: 0.6,
            decomposition_efficiency: 0.4,
            reproduction_efficiency: 0.7,
            reproduction_energy_threshold: 15.0,
            mutation_rate: 0.1,
            mutation_magnitude: 0.05,
            initial_population_size: 10,
            world_extent: 50.0,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: TraitVector {
                photosynthetic_absorption: 0.5,
                consumption_rate: 0.3,
                scavenging_rate: 0.2,
                mobility: 0.3,
                chemotaxis_sensitivity: 0.3,
                mate_selectivity: 2.0,
                sensing_range: 5.0,
                reproductive_investment: 5.0,
            },
            trait_covariance: 0.1,
            initial_cluster_count: 1,
            initial_energy_per_agent: 30.0,
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
            diff < 1e-1,
            "energy accounting off by {diff}: total={total}, expected={expected}, \
             living={living_energy}, carcass={carcass_energy}, dissipated={}, solar={}, \
             agents={}, carcasses={}",
            world.dissipated_energy(),
            world.total_solar_input(),
            world.agents().len(),
            world.carcasses().len()
        );
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
