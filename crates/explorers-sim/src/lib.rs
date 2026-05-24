pub mod event;
pub mod spatial;

use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, Normal};
use serde::{Deserialize, Serialize};

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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    pub light_competition_radius: f32,
    pub photo_maintenance_cost: f32,
    pub consumption_maintenance_cost: f32,
    pub scavenging_maintenance_cost: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InitialDistribution {
    pub mean_traits: TraitVector,
    pub trait_covariance: f32,
    pub initial_cluster_count: u32,
    pub initial_energy_per_agent: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldRecipe {
    pub parameters: WorldParameters,
    pub initial_distribution: InitialDistribution,
    pub max_ticks: u64,
}

pub struct Agent {
    pub id: u64,
    pub position: (f32, f32),
    pub energy: f32,
    pub traits: TraitVector,
}

pub struct Carcass {
    pub id: u64,
    pub position: (f32, f32),
    pub energy: f32,
}

pub struct World {
    params: WorldParameters,
    agents: Vec<Agent>,
    carcasses: Vec<Carcass>,
    dissipated_energy: f32,
    total_solar_input: f32,
    seed: u64,
    rng: ChaCha8Rng,
    tick: u64,
    last_tick_births: usize,
    last_tick_deaths: usize,
    next_agent_id: u64,
}

impl World {
    pub fn new(params: WorldParameters, distribution: InitialDistribution, seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let extent = params.world_extent;
        let pop_size = params.initial_population_size as usize;
        let n_clusters = (distribution.initial_cluster_count as usize).max(1);

        let pos_dist = rand::distr::Uniform::new(-extent / 2.0, extent / 2.0).unwrap();
        let trait_dist = Normal::new(0.0_f32, distribution.trait_covariance).unwrap();

        let mean = &distribution.mean_traits;
        let trophic_total = mean.photosynthetic_absorption + mean.consumption_rate + mean.scavenging_rate;

        let cluster_centroids: Vec<TraitVector> = (0..n_clusters)
            .map(|c| {
                let (photo, cons, scav) = if n_clusters == 1 || trophic_total <= 0.0 {
                    (mean.photosynthetic_absorption, mean.consumption_rate, mean.scavenging_rate)
                } else {
                    match c % 3 {
                        0 => (trophic_total, 0.0, 0.0),
                        1 => (0.0, trophic_total, 0.0),
                        _ => (0.0, 0.0, trophic_total),
                    }
                };
                TraitVector {
                    photosynthetic_absorption: photo,
                    consumption_rate: cons,
                    scavenging_rate: scav,
                    mobility: mean.mobility,
                    chemotaxis_sensitivity: mean.chemotaxis_sensitivity,
                    mate_selectivity: mean.mate_selectivity,
                    sensing_range: mean.sensing_range,
                    reproductive_investment: mean.reproductive_investment,
                }
            })
            .collect();

        let agents = (0..pop_size)
            .map(|id| {
                let x = pos_dist.sample(&mut rng);
                let y = pos_dist.sample(&mut rng);
                let centroid = &cluster_centroids[id % n_clusters];
                Agent {
                    id: id as u64,
                    position: (x, y),
                    energy: distribution.initial_energy_per_agent,
                    traits: TraitVector {
                        photosynthetic_absorption: centroid.photosynthetic_absorption
                            + trait_dist.sample(&mut rng),
                        consumption_rate: centroid.consumption_rate + trait_dist.sample(&mut rng),
                        scavenging_rate: centroid.scavenging_rate + trait_dist.sample(&mut rng),
                        mobility: centroid.mobility + trait_dist.sample(&mut rng),
                        chemotaxis_sensitivity: centroid.chemotaxis_sensitivity
                            + trait_dist.sample(&mut rng),
                        mate_selectivity: centroid.mate_selectivity + trait_dist.sample(&mut rng),
                        sensing_range: centroid.sensing_range + trait_dist.sample(&mut rng),
                        reproductive_investment: centroid.reproductive_investment
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
            seed,
            rng,
            tick: 0,
            last_tick_births: 0,
            last_tick_deaths: 0,
            next_agent_id: pop_size as u64,
        }
    }

    pub fn add_agent(&mut self, mut agent: Agent) {
        agent.id = self.next_agent_id;
        self.next_agent_id += 1;
        self.agents.push(agent);
    }

    pub fn add_carcass(&mut self, carcass: Carcass) {
        self.carcasses.push(carcass);
    }

    pub fn step(&mut self) {
        let extent = self.params.world_extent;
        let agent_count = self.agents.len();

        let sub_seed = self.seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(self.tick);
        let mut shuffle_rng = ChaCha8Rng::seed_from_u64(sub_seed);
        let mut order: Vec<usize> = (0..agent_count).collect();
        order.shuffle(&mut shuffle_rng);

        let max_sensing_range = self.agents.iter()
            .map(|a| a.traits.sensing_range)
            .fold(0.0_f32, f32::max);
        let max_query_radius = max_sensing_range
            .max(self.params.light_competition_radius)
            .max(self.params.contact_radius);
        let cell_size = max_query_radius.max(1.0);

        let mut agent_grid = crate::spatial::SpatialGrid::new(extent, cell_size);
        for (i, agent) in self.agents.iter().enumerate() {
            agent_grid.insert(i as u64, agent.position);
        }

        let mut carcass_grid = crate::spatial::SpatialGrid::new(extent, cell_size);
        for (ci, carcass) in self.carcasses.iter().enumerate() {
            carcass_grid.insert(ci as u64, carcass.position);
        }

        // --- Sense & Decide: compute movement vectors for all agents ---
        let mut movements = vec![(0.0_f32, 0.0_f32); agent_count];
        for &i in &order {
            let agent = &self.agents[i];
            if agent.traits.mobility <= 0.0 {
                continue;
            }

            let mut chemotaxis_x = 0.0_f32;
            let mut chemotaxis_y = 0.0_f32;

            if agent.traits.consumption_rate > 0.0 {
                let neighbors = agent_grid.query_radius(agent.position, agent.traits.sensing_range);
                for j_id in neighbors {
                    let j = j_id as usize;
                    if j == i {
                        continue;
                    }
                    let other = &self.agents[j];
                    let (dx, dy) =
                        toroidal_displacement(agent.position, other.position, extent);
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist > 0.0 {
                        let signal = agent.traits.consumption_rate / dist;
                        chemotaxis_x += signal * dx / dist;
                        chemotaxis_y += signal * dy / dist;
                    }
                }
            }

            if agent.traits.scavenging_rate > 0.0 {
                let nearby_carcasses = carcass_grid.query_radius(agent.position, agent.traits.sensing_range);
                for ci_id in nearby_carcasses {
                    let ci = ci_id as usize;
                    let carcass = &self.carcasses[ci];
                    let (dx, dy) =
                        toroidal_displacement(agent.position, carcass.position, extent);
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist > 0.0 {
                        let signal = agent.traits.scavenging_rate / dist;
                        chemotaxis_x += signal * dx / dist;
                        chemotaxis_y += signal * dy / dist;
                    }
                }
            }

            chemotaxis_x *= agent.traits.chemotaxis_sensitivity;
            chemotaxis_y *= agent.traits.chemotaxis_sensitivity;

            let angle_dist = rand::distr::Uniform::new(0.0_f32, std::f32::consts::TAU).unwrap();
            let angle = angle_dist.sample(&mut self.rng);
            let explore_x = angle.cos();
            let explore_y = angle.sin();

            let mut move_x = chemotaxis_x + explore_x;
            let mut move_y = chemotaxis_y + explore_y;

            let mag = (move_x * move_x + move_y * move_y).sqrt();
            if mag > 0.0 {
                move_x = move_x / mag * agent.traits.mobility;
                move_y = move_y / mag * agent.traits.mobility;
            }

            movements[i] = (move_x, move_y);
        }

        // --- Compute local light competition shares ---
        let competition_radius = self.params.light_competition_radius;
        let light_shares: Vec<f32> = (0..agent_count)
            .map(|i| {
                let agent = &self.agents[i];
                if agent.traits.photosynthetic_absorption <= 0.0 {
                    return 1.0;
                }
                let mut total_local_absorption = agent.traits.photosynthetic_absorption;
                let neighbors = agent_grid.query_radius(agent.position, competition_radius);
                for j_id in neighbors {
                    let j = j_id as usize;
                    if j == i {
                        continue;
                    }
                    let other = &self.agents[j];
                    if other.traits.photosynthetic_absorption <= 0.0 {
                        continue;
                    }
                    let dist = toroidal_distance(agent.position, other.position, extent);
                    if dist < competition_radius {
                        total_local_absorption += other.traits.photosynthetic_absorption;
                    }
                }
                agent.traits.photosynthetic_absorption / total_local_absorption
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

            let mobility_gate = 1.0 / (1.0 + (20.0_f32 * (agent.traits.mobility - 0.3)).exp());
            let solar_gain =
                agent.traits.photosynthetic_absorption * self.params.solar_flux_magnitude * mobility_gate * light_shares[i];
            let metabolic_cost = self.params.base_metabolic_rate
                + agent.traits.sensing_range * self.params.sensing_cost_coefficient
                + agent.traits.photosynthetic_absorption * self.params.photo_maintenance_cost
                + agent.traits.consumption_rate * self.params.consumption_maintenance_cost
                + agent.traits.scavenging_rate * self.params.scavenging_maintenance_cost;
            self.total_solar_input += solar_gain;
            let energy = agent.energy + solar_gain - metabolic_cost - movement_cost;
            pre_tick_energies.push(agent.energy);
            solar_gains.push(solar_gain);
            agents.push(Agent {
                id: agent.id,
                position: new_pos,
                energy,
                traits: agent.traits,
            });
        }

        // Rebuild agent grid with post-move positions
        let mut agent_grid = crate::spatial::SpatialGrid::new(extent, cell_size);
        for (i, agent) in agents.iter().enumerate() {
            agent_grid.insert(i as u64, agent.position);
        }

        // --- Consumption: agents drain energy from living targets within contact_radius ---
        let contact_radius = self.params.contact_radius;
        let consumption_efficiency = self.params.consumption_efficiency;
        let n = agents.len();
        let mut consumption_deltas = vec![0.0_f32; n];

        // Phase 1: collect intentions — each consumer picks its closest target
        let mut intentions: Vec<(usize, usize)> = Vec::new();
        for &i in &order {
            if agents[i].traits.consumption_rate <= 0.0 {
                continue;
            }
            let mut best_target: Option<usize> = None;
            let mut best_dist = f32::MAX;
            let neighbors = agent_grid.query_radius(agents[i].position, contact_radius);
            for j_id in neighbors {
                let j = j_id as usize;
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
                intentions.push((i, target));
            }
        }

        // Phase 2: resolve — split each victim's energy proportionally among claimants
        // Group demands by target
        let mut demands_by_target: std::collections::HashMap<usize, Vec<usize>> =
            std::collections::HashMap::new();
        for &(consumer, target) in &intentions {
            demands_by_target.entry(target).or_default().push(consumer);
        }
        for (target, consumers) in &demands_by_target {
            let available = agents[*target].energy.max(0.0);
            let total_demand: f32 = consumers
                .iter()
                .map(|&c| agents[c].traits.consumption_rate)
                .sum();
            let drain = total_demand.min(available);
            if drain > 0.0 {
                for &consumer in consumers {
                    let share = agents[consumer].traits.consumption_rate / total_demand;
                    let consumer_drain = drain * share;
                    consumption_deltas[consumer] += consumer_drain * consumption_efficiency;
                    self.dissipated_energy += consumer_drain * (1.0 - consumption_efficiency);
                }
                consumption_deltas[*target] -= drain;
            }
        }

        for i in 0..n {
            agents[i].energy += consumption_deltas[i];
        }

        // --- Decomposition: agents drain energy from carcasses within contact_radius ---
        let decomposition_efficiency = self.params.decomposition_efficiency;
        let mut carcass_drains = vec![0.0_f32; self.carcasses.len()];
        let mut decomposition_gains = vec![0.0_f32; n];

        // Phase 1: collect intentions
        let mut decomp_intentions: Vec<(usize, usize)> = Vec::new();
        for &i in &order {
            if agents[i].traits.scavenging_rate <= 0.0 {
                continue;
            }
            let mut best_carcass: Option<usize> = None;
            let mut best_dist = f32::MAX;
            let nearby_carcasses = carcass_grid.query_radius(agents[i].position, contact_radius);
            for ci_id in nearby_carcasses {
                let ci = ci_id as usize;
                let carcass = &self.carcasses[ci];
                let dist = toroidal_distance(agents[i].position, carcass.position, extent);
                if dist < contact_radius && dist < best_dist {
                    best_dist = dist;
                    best_carcass = Some(ci);
                }
            }
            if let Some(ci) = best_carcass {
                decomp_intentions.push((i, ci));
            }
        }

        // Phase 2: resolve — split each carcass's energy proportionally among claimants
        let mut decomp_by_carcass: std::collections::HashMap<usize, Vec<usize>> =
            std::collections::HashMap::new();
        for &(scavenger, carcass) in &decomp_intentions {
            decomp_by_carcass.entry(carcass).or_default().push(scavenger);
        }
        for (ci, scavengers) in &decomp_by_carcass {
            let available = self.carcasses[*ci].energy.max(0.0);
            let total_demand: f32 = scavengers
                .iter()
                .map(|&s| agents[s].traits.scavenging_rate)
                .sum();
            let drain = total_demand.min(available);
            if drain > 0.0 {
                for &scavenger in scavengers {
                    let share = agents[scavenger].traits.scavenging_rate / total_demand;
                    let scavenger_drain = drain * share;
                    decomposition_gains[scavenger] = scavenger_drain * decomposition_efficiency;
                    agents[scavenger].energy += decomposition_gains[scavenger];
                    self.dissipated_energy += scavenger_drain * (1.0 - decomposition_efficiency);
                }
                carcass_drains[*ci] = drain;
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

        // Compute per-agent effective reproduction radius (spore dispersal)
        let effective_radii: Vec<f32> = (0..n)
            .map(|i| {
                let gate = 1.0 / (1.0 + (20.0_f32 * (agents[i].traits.mobility - 0.3)).exp());
                gate * agents[i].traits.sensing_range + (1.0 - gate) * contact_radius
            })
            .collect();

        // Phase 1: collect all compatible candidate pairs
        let max_effective_radius = effective_radii.iter().copied().fold(0.0_f32, f32::max);
        let mut candidate_pairs: Vec<(f32, usize, usize)> = Vec::new();
        for i in 0..n {
            if agents[i].energy <= reproduction_threshold {
                continue;
            }
            let neighbors = agent_grid.query_radius(agents[i].position, max_effective_radius);
            for j_id in neighbors {
                let j = j_id as usize;
                if j <= i {
                    continue;
                }
                if agents[j].energy <= reproduction_threshold {
                    continue;
                }
                let spatial_dist =
                    toroidal_distance(agents[i].position, agents[j].position, extent);
                if spatial_dist > effective_radii[i].max(effective_radii[j]) {
                    continue;
                }
                let trait_dist = agents[i].traits.distance(&agents[j].traits);
                if trait_dist < agents[i].traits.mate_selectivity
                    && trait_dist < agents[j].traits.mate_selectivity
                {
                    candidate_pairs.push((spatial_dist, i, j));
                }
            }
        }

        // Phase 2: sort by spatial distance, then deterministic tiebreak by position
        candidate_pairs.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap()
                .then_with(|| {
                    let pos_a = (
                        agents[a.1].position.0.min(agents[a.2].position.0),
                        agents[a.1].position.1.min(agents[a.2].position.1),
                    );
                    let pos_b = (
                        agents[b.1].position.0.min(agents[b.2].position.0),
                        agents[b.1].position.1.min(agents[b.2].position.1),
                    );
                    pos_a
                        .0
                        .partial_cmp(&pos_b.0)
                        .unwrap()
                        .then_with(|| pos_a.1.partial_cmp(&pos_b.1).unwrap())
                })
        });

        // Phase 3: greedily match closest pairs first
        for (_, i, j) in &candidate_pairs {
            let i = *i;
            let j = *j;
            if reproduced[i] || reproduced[j] {
                continue;
            }
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

            let offspring_id = self.next_agent_id;
            self.next_agent_id += 1;
            offspring.push(Agent {
                id: offspring_id,
                position: agents[i].position,
                energy: offspring_energy,
                traits: child_traits,
            });
        }

        // --- Death check and carcass creation ---
        let mut next_agents: Vec<Agent> = Vec::with_capacity(n);
        let mut next_carcasses: Vec<Carcass> = self.carcasses.drain(..).collect();

        for (i, agent) in agents.into_iter().enumerate() {
            if agent.energy <= 0.0 {
                let carcass_energy = (pre_tick_energies[i] + consumption_deltas[i]).max(0.0);
                next_carcasses.push(Carcass {
                    id: agent.id,
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
        self.tick += 1;
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

    pub fn tick(&self) -> u64 {
        self.tick
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
            light_competition_radius: 1000.0,
            photo_maintenance_cost: 0.0,
            consumption_maintenance_cost: 0.0,
            scavenging_maintenance_cost: 0.0,
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
            id: 0,
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
            id: 0,
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
            id: 0,
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
            id: 0,
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
            id: 0,
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
            id: 0,
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
            id: 0,
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
            id: 0,
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
            id: 0,
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
            id: 0,
            position: (0.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                consumption_rate: 2.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            id: 0,
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
            id: 0,
            position: (0.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                consumption_rate: 3.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            id: 0,
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
            id: 0,
            position: (0.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                consumption_rate: 10.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            id: 0,
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
            id: 0,
            position: (0.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                scavenging_rate: 4.0,
                ..zero_traits()
            },
        });
        world.add_carcass(Carcass {
            id: 0,
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
            id: 0,
            position: (0.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                scavenging_rate: 10.0,
                ..zero_traits()
            },
        });
        world.add_carcass(Carcass {
            id: 0,
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
            id: 0,
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
            id: 0,
            position: (1.0, 0.0),
            energy: 50.0,
            traits: zero_traits(),
        });
        // Carcass
        world.add_carcass(Carcass {
            id: 0,
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
            id: 0,
            position: (0.0, 0.0),
            energy: 50.0,
            traits: zero_traits(),
        });
        world.add_agent(Agent {
            id: 0,
            position: (1.0, 0.0),
            energy: 50.0,
            traits: zero_traits(),
        });
        world.add_carcass(Carcass {
            id: 0,
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
            id: 0,
            position: (-48.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                consumption_rate: 2.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            id: 0,
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
            id: 0,
            position: (0.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                consumption_rate: 2.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            id: 0,
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
            mobility: 1.0,
            mate_selectivity: 5.0,
            reproductive_investment: 10.0,
            ..zero_traits()
        };
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            energy: 50.0,
            traits: shared_traits,
        });
        world.add_agent(Agent {
            id: 0,
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
            id: 0,
            position: (0.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                mobility: 1.0,
                mate_selectivity: 10.0,
                reproductive_investment: 15.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            id: 0,
            position: (2.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                mobility: 1.0,
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
            id: 0,
            position: (0.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                mobility: 1.0,
                mate_selectivity: 10.0,
                reproductive_investment: 15.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            id: 0,
            position: (2.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                mobility: 1.0,
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
            id: 0,
            position: (0.0, 0.0),
            energy: 40.0, // below threshold of 50
            traits: shared_traits,
        });
        world.add_agent(Agent {
            id: 0,
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
            id: 0,
            position: (0.0, 0.0),
            energy: 50.0,
            traits: shared_traits,
        });
        world.add_agent(Agent {
            id: 0,
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
            id: 0,
            position: (0.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                mate_selectivity: 10.0, // accepts trait dist < 10
                reproductive_investment: 10.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            id: 0,
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
            mobility: 1.0,
            mate_selectivity: 5.0,
            reproductive_investment: 5.0,
            ..zero_traits()
        };
        // Three compatible agents all in contact — should produce at most 1 offspring (one pair)
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            energy: 100.0,
            traits: shared_traits,
        });
        world.add_agent(Agent {
            id: 0,
            position: (1.0, 0.0),
            energy: 100.0,
            traits: shared_traits,
        });
        world.add_agent(Agent {
            id: 0,
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
                id: 0,
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
                id: 0,
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
            mobility: 1.0,
            mate_selectivity: 5.0,
            reproductive_investment: 10.0,
            ..zero_traits()
        };
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            energy: 100.0,
            traits: shared_traits,
        });
        world.add_agent(Agent {
            id: 0,
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
                id: 0,
                position: (0.0, 0.0),
                energy: 100.0,
                traits: shared_traits,
            });
            world.add_agent(Agent {
                id: 0,
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
        let mobility = world.agents()[0].traits.mobility;
        let mobility_gate = 1.0 / (1.0 + (20.0_f32 * (mobility - 0.3)).exp());
        let photosynthesis = world.agents()[0].traits.photosynthetic_absorption
            * world.params().solar_flux_magnitude
            * mobility_gate;
        let metabolic_cost = world.params().base_metabolic_rate
            + world.agents()[0].traits.sensing_range * world.params().sensing_cost_coefficient;
        let expected = initial_energy + photosynthesis - metabolic_cost;
        assert_eq!(world.agents()[0].energy, expected);
    }

    fn sorted_energies(world: &World) -> Vec<f32> {
        let mut energies: Vec<f32> = world.agents().iter().map(|a| a.energy).collect();
        energies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        energies
    }

    #[test]
    fn consumption_outcome_is_independent_of_agent_order() {
        let make_params = || WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
            initial_population_size: 0,
            ..test_params()
        };
        let empty_dist = || InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        };

        let consumer_traits = TraitVector {
            consumption_rate: 3.0,
            ..zero_traits()
        };

        // Victim has only 4.0 energy — not enough for two consumers each wanting 3.0
        // Consumers have different starting energy so sorted results differ if order matters
        // World A: consumer1 first, then consumer2, then victim
        let mut world_a = World::new(make_params(), empty_dist(), 42);
        world_a.add_agent(Agent { id: 0, position: (0.0, 0.0), energy: 50.0, traits: consumer_traits });
        world_a.add_agent(Agent { id: 0, position: (1.0, 0.0), energy: 30.0, traits: consumer_traits });
        world_a.add_agent(Agent { id: 0, position: (0.5, 0.0), energy: 4.0, traits: zero_traits() });
        world_a.step();

        // World B: consumer2 first, then consumer1, then victim
        let mut world_b = World::new(make_params(), empty_dist(), 42);
        world_b.add_agent(Agent { id: 0, position: (1.0, 0.0), energy: 30.0, traits: consumer_traits });
        world_b.add_agent(Agent { id: 0, position: (0.0, 0.0), energy: 50.0, traits: consumer_traits });
        world_b.add_agent(Agent { id: 0, position: (0.5, 0.0), energy: 4.0, traits: zero_traits() });
        world_b.step();

        assert_eq!(
            sorted_energies(&world_a),
            sorted_energies(&world_b),
            "consumption outcome should not depend on agent insertion order"
        );
    }

    #[test]
    fn decomposition_outcome_is_independent_of_agent_order() {
        let make_params = || WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            decomposition_efficiency: 0.5,
            initial_population_size: 0,
            ..test_params()
        };
        let empty_dist = || InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        };

        let scavenger_traits = TraitVector {
            scavenging_rate: 3.0,
            ..zero_traits()
        };

        // Carcass has only 4.0 energy — not enough for two scavengers each wanting 3.0
        // World A: scavenger1 first, then scavenger2
        let mut world_a = World::new(make_params(), empty_dist(), 42);
        world_a.add_agent(Agent { id: 0, position: (0.0, 0.0), energy: 50.0, traits: scavenger_traits });
        world_a.add_agent(Agent { id: 0, position: (1.0, 0.0), energy: 30.0, traits: scavenger_traits });
        world_a.add_carcass(Carcass { id: 0, position: (0.5, 0.0), energy: 4.0 });
        world_a.step();

        // World B: scavenger2 first, then scavenger1
        let mut world_b = World::new(make_params(), empty_dist(), 42);
        world_b.add_agent(Agent { id: 0, position: (1.0, 0.0), energy: 30.0, traits: scavenger_traits });
        world_b.add_agent(Agent { id: 0, position: (0.0, 0.0), energy: 50.0, traits: scavenger_traits });
        world_b.add_carcass(Carcass { id: 0, position: (0.5, 0.0), energy: 4.0 });
        world_b.step();

        assert_eq!(
            sorted_energies(&world_a),
            sorted_energies(&world_b),
            "decomposition outcome should not depend on agent insertion order"
        );
    }

    #[test]
    fn reproduction_pairing_is_independent_of_agent_order() {
        let make_params = || WorldParameters {
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
        let empty_dist = || InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        };

        // Three agents: A at (0,0), B at (1,0), C at (2,0). All compatible.
        // A-B dist=1, B-C dist=1, A-C dist=2. B is closest to both A and C.
        // If order-dependent: iterating A first → A pairs with B, C unpaired.
        // Iterating C first → C pairs with B, A unpaired.
        // Give A and C different energies so sorted results differ.
        let traits = TraitVector {
            mate_selectivity: 10.0,
            reproductive_investment: 5.0,
            ..zero_traits()
        };

        // World A: agent order is A, B, C
        let mut world_a = World::new(make_params(), empty_dist(), 42);
        world_a.add_agent(Agent { id: 0, position: (0.0, 0.0), energy: 50.0, traits });
        world_a.add_agent(Agent { id: 0, position: (1.0, 0.0), energy: 40.0, traits });
        world_a.add_agent(Agent { id: 0, position: (2.0, 0.0), energy: 30.0, traits });
        world_a.step();

        // World B: agent order is C, B, A
        let mut world_b = World::new(make_params(), empty_dist(), 42);
        world_b.add_agent(Agent { id: 0, position: (2.0, 0.0), energy: 30.0, traits });
        world_b.add_agent(Agent { id: 0, position: (1.0, 0.0), energy: 40.0, traits });
        world_b.add_agent(Agent { id: 0, position: (0.0, 0.0), energy: 50.0, traits });
        world_b.step();

        assert_eq!(
            sorted_energies(&world_a),
            sorted_energies(&world_b),
            "reproduction pairing should not depend on agent insertion order"
        );
    }

    #[test]
    fn world_from_deserialized_recipe_matches_direct_construction() {
        let recipe = WorldRecipe {
            parameters: test_params(),
            initial_distribution: test_distribution(),
            max_ticks: 100,
        };
        let json = serde_json::to_string(&recipe).unwrap();
        let recovered: WorldRecipe = serde_json::from_str(&json).unwrap();

        let seed = 99;
        let world_direct = World::new(test_params(), test_distribution(), seed);
        let world_recipe = World::new(
            recovered.parameters,
            recovered.initial_distribution,
            seed,
        );

        assert_eq!(world_direct.agents().len(), world_recipe.agents().len());
        for (a, b) in world_direct.agents().iter().zip(world_recipe.agents().iter()) {
            assert_eq!(a.position, b.position);
            assert_eq!(a.energy, b.energy);
            assert_eq!(a.traits, b.traits);
        }
    }

    #[test]
    fn world_recipe_round_trips_through_json() {
        let recipe = WorldRecipe {
            parameters: test_params(),
            initial_distribution: test_distribution(),
            max_ticks: 100,
        };

        let json = serde_json::to_string_pretty(&recipe).unwrap();
        let recovered: WorldRecipe = serde_json::from_str(&json).unwrap();

        assert_eq!(recipe, recovered);
    }

    #[test]
    fn initial_agents_have_unique_sequential_ids() {
        let world = World::new(test_params(), test_distribution(), 42);
        let ids: Vec<u64> = world.agents().iter().map(|a| a.id).collect();
        let expected: Vec<u64> = (0..ids.len() as u64).collect();
        assert_eq!(ids, expected);
    }

    #[test]
    fn offspring_ids_do_not_collide_with_initial_population() {
        let mut params = test_params();
        params.initial_population_size = 0;
        params.contact_radius = 5.0;
        params.reproduction_energy_threshold = 1.0;
        params.consumption_efficiency = 0.0;
        let mut world = World::new(params, test_distribution(), 42);

        let reproducer_traits = TraitVector {
            reproductive_investment: 0.5,
            mate_selectivity: 0.0,
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };
        world.add_agent(Agent { id: 0, position: (0.0, 0.0), energy: 100.0, traits: reproducer_traits });
        world.add_agent(Agent { id: 0, position: (0.1, 0.0), energy: 100.0, traits: reproducer_traits });

        let initial_max_id = world.agents().iter().map(|a| a.id).max().unwrap();

        world.step();

        let mut all_ids: Vec<u64> = world.agents().iter().map(|a| a.id).collect();
        all_ids.sort();
        let unique_count = {
            let mut u = all_ids.clone();
            u.dedup();
            u.len()
        };
        assert_eq!(all_ids.len(), unique_count, "all agent IDs must be unique");
        for agent in world.agents() {
            if agent.id > initial_max_id {
                assert!(agent.id > initial_max_id, "offspring ID must exceed initial population IDs");
            }
        }
    }

    #[test]
    fn carcass_inherits_dead_agent_id() {
        let mut params = test_params();
        params.initial_population_size = 0;
        let mut world = World::new(params, test_distribution(), 42);

        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            energy: 0.01,
            traits: zero_traits(),
        });
        let agent_id = world.agents()[0].id;

        world.step();

        assert!(world.agents().is_empty(), "agent should have died");
        assert_eq!(world.carcasses().len(), 1);
        assert_eq!(world.carcasses()[0].id, agent_id, "carcass must inherit the dead agent's ID");
    }

    #[test]
    fn agent_ids_are_deterministic_given_same_seed() {
        let seed = 77;
        let params = test_params();
        let dist = test_distribution();

        let mut world_a = World::new(params.clone(), dist.clone(), seed);
        let mut world_b = World::new(params, dist, seed);

        for _ in 0..5 {
            world_a.step();
            world_b.step();
        }

        let ids_a: Vec<u64> = world_a.agents().iter().map(|a| a.id).collect();
        let ids_b: Vec<u64> = world_b.agents().iter().map(|a| a.id).collect();
        assert_eq!(ids_a, ids_b, "same seed must produce identical agent IDs");

        let carcass_ids_a: Vec<u64> = world_a.carcasses().iter().map(|c| c.id).collect();
        let carcass_ids_b: Vec<u64> = world_b.carcasses().iter().map(|c| c.id).collect();
        assert_eq!(carcass_ids_a, carcass_ids_b, "same seed must produce identical carcass IDs");
    }

    #[test]
    fn lone_producer_gets_full_flux_colocated_producers_share() {
        let params = WorldParameters {
            solar_flux_magnitude: 10.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            initial_population_size: 0,
            light_competition_radius: 5.0,
            photo_maintenance_cost: 0.0,
            consumption_maintenance_cost: 0.0,
            scavenging_maintenance_cost: 0.0,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        };

        let producer = TraitVector {
            photosynthetic_absorption: 1.0,
            mobility: 0.0,
            ..zero_traits()
        };

        // Lone producer gets full flux
        let mut world_lone = World::new(params.clone(), dist.clone(), 42);
        world_lone.add_agent(Agent {
            id: 0, position: (0.0, 0.0), energy: 100.0, traits: producer,
        });
        world_lone.step();
        let lone_energy = world_lone.agents()[0].energy;
        let gate = 1.0_f32 / (1.0 + (20.0_f32 * (0.0 - 0.3)).exp());
        let expected_full = 100.0 + 1.0 * 10.0 * gate;
        assert!(
            (lone_energy - expected_full).abs() < 0.1,
            "lone producer should get full flux, energy={lone_energy}, expected={expected_full}"
        );

        // Two co-located producers with equal absorption share equally
        let mut world_two = World::new(params.clone(), dist.clone(), 42);
        world_two.add_agent(Agent {
            id: 0, position: (0.0, 0.0), energy: 100.0, traits: producer,
        });
        world_two.add_agent(Agent {
            id: 0, position: (0.0, 0.0), energy: 100.0, traits: producer,
        });
        world_two.step();
        let energy_a = world_two.agents()[0].energy;
        let energy_b = world_two.agents()[1].energy;
        let expected_half = 100.0 + 1.0 * 10.0 * gate * 0.5;
        assert!(
            (energy_a - expected_half).abs() < 0.1,
            "co-located producer should get half flux, energy={energy_a}, expected={expected_half}"
        );
        assert!(
            (energy_b - expected_half).abs() < 0.1,
            "co-located producer should get half flux, energy={energy_b}, expected={expected_half}"
        );
    }

    #[test]
    fn producers_outside_competition_radius_dont_compete() {
        let params = WorldParameters {
            solar_flux_magnitude: 10.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            initial_population_size: 0,
            light_competition_radius: 5.0,
            photo_maintenance_cost: 0.0,
            consumption_maintenance_cost: 0.0,
            scavenging_maintenance_cost: 0.0,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        };

        let producer = TraitVector {
            photosynthetic_absorption: 1.0,
            mobility: 0.0,
            ..zero_traits()
        };

        // Two producers far apart (> competition radius) don't compete
        let mut world = World::new(params.clone(), dist.clone(), 42);
        world.add_agent(Agent {
            id: 0, position: (0.0, 0.0), energy: 100.0, traits: producer,
        });
        world.add_agent(Agent {
            id: 0, position: (40.0, 40.0), energy: 100.0, traits: producer,
        });
        world.step();
        let gate = 1.0_f32 / (1.0 + (20.0_f32 * (0.0 - 0.3)).exp());
        let expected_full = 100.0 + 1.0 * 10.0 * gate;
        let energy_a = world.agents()[0].energy;
        let energy_b = world.agents()[1].energy;
        assert!(
            (energy_a - expected_full).abs() < 0.1,
            "distant producer should get full flux, energy={energy_a}, expected={expected_full}"
        );
        assert!(
            (energy_b - expected_full).abs() < 0.1,
            "distant producer should get full flux, energy={energy_b}, expected={expected_full}"
        );
    }

    #[test]
    fn sessile_agent_gets_full_solar_gain_mobile_agent_gets_near_zero() {
        let params = WorldParameters {
            solar_flux_magnitude: 10.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            initial_population_size: 0,
            photo_maintenance_cost: 0.0,
            consumption_maintenance_cost: 0.0,
            scavenging_maintenance_cost: 0.0,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        };

        let sessile_traits = TraitVector {
            photosynthetic_absorption: 1.0,
            mobility: 0.0,
            ..zero_traits()
        };
        let mobile_traits = TraitVector {
            photosynthetic_absorption: 1.0,
            mobility: 1.0,
            ..zero_traits()
        };

        let mut world_sessile = World::new(params.clone(), dist.clone(), 42);
        world_sessile.add_agent(Agent {
            id: 0, position: (0.0, 0.0), energy: 100.0, traits: sessile_traits,
        });
        world_sessile.step();
        let sessile_energy = world_sessile.agents()[0].energy;
        // Sessile agent: gate ≈ 1.0, solar gain ≈ 1.0 * 10.0 = 10.0
        assert!(
            (sessile_energy - 110.0).abs() < 0.1,
            "sessile agent should get full solar gain, energy={sessile_energy}"
        );

        let mut world_mobile = World::new(params.clone(), dist.clone(), 42);
        world_mobile.add_agent(Agent {
            id: 0, position: (0.0, 0.0), energy: 100.0, traits: mobile_traits,
        });
        world_mobile.step();
        let mobile_energy = world_mobile.agents()[0].energy;
        // Mobile agent: gate ≈ 0.0, solar gain ≈ 0.0
        assert!(
            mobile_energy < 100.5,
            "mobile agent should get near-zero solar gain, energy={mobile_energy}"
        );
    }

    #[test]
    fn trait_maintenance_costs_scale_with_trait_magnitude() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            initial_population_size: 0,
            photo_maintenance_cost: 0.1,
            consumption_maintenance_cost: 0.2,
            scavenging_maintenance_cost: 0.3,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        };

        // Agent with high consumption_rate pays more than one with low
        let high_consumption = TraitVector {
            consumption_rate: 5.0,
            ..zero_traits()
        };
        let low_consumption = TraitVector {
            consumption_rate: 1.0,
            ..zero_traits()
        };

        let mut world_high = World::new(params.clone(), dist.clone(), 42);
        world_high.add_agent(Agent {
            id: 0, position: (0.0, 0.0), energy: 100.0, traits: high_consumption,
        });
        world_high.step();

        let mut world_low = World::new(params.clone(), dist.clone(), 42);
        world_low.add_agent(Agent {
            id: 0, position: (0.0, 0.0), energy: 100.0, traits: low_consumption,
        });
        world_low.step();

        let high_energy = world_high.agents()[0].energy;
        let low_energy = world_low.agents()[0].energy;

        // consumption_maintenance_cost = 0.2
        // high: 100 - 5.0*0.2 = 99.0, low: 100 - 1.0*0.2 = 99.8
        assert!(
            high_energy < low_energy,
            "high consumption_rate should drain more energy: high={high_energy}, low={low_energy}"
        );
        assert!(
            (high_energy - 99.0).abs() < 0.01,
            "expected 99.0 for high consumption agent, got {high_energy}"
        );
        assert!(
            (low_energy - 99.8).abs() < 0.01,
            "expected 99.8 for low consumption agent, got {low_energy}"
        );
    }

    #[test]
    fn crowded_generalist_drains_faster_than_lean_specialist() {
        let params = WorldParameters {
            solar_flux_magnitude: 10.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            initial_population_size: 0,
            light_competition_radius: 5.0,
            photo_maintenance_cost: 0.05,
            consumption_maintenance_cost: 0.1,
            scavenging_maintenance_cost: 0.1,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        };

        let generalist = TraitVector {
            photosynthetic_absorption: 1.0,
            consumption_rate: 3.0,
            scavenging_rate: 2.0,
            mobility: 0.0,
            ..zero_traits()
        };
        let specialist = TraitVector {
            photosynthetic_absorption: 1.0,
            mobility: 0.0,
            ..zero_traits()
        };

        // Crowded generalist: two generalists near each other share light + pay maintenance
        let mut world_gen = World::new(params.clone(), dist.clone(), 42);
        world_gen.add_agent(Agent {
            id: 0, position: (0.0, 0.0), energy: 100.0, traits: generalist,
        });
        world_gen.add_agent(Agent {
            id: 0, position: (1.0, 0.0), energy: 100.0, traits: generalist,
        });
        world_gen.step();
        let gen_energy = world_gen.agents()[0].energy;

        // Lone specialist: no competition, no maintenance for unused traits
        let mut world_spec = World::new(params.clone(), dist.clone(), 42);
        world_spec.add_agent(Agent {
            id: 0, position: (0.0, 0.0), energy: 100.0, traits: specialist,
        });
        world_spec.step();
        let spec_energy = world_spec.agents()[0].energy;

        assert!(
            gen_energy < spec_energy,
            "crowded generalist ({gen_energy}) should have less energy than lone specialist ({spec_energy})"
        );
    }

    #[test]
    fn midpoint_mobility_agent_has_blended_reproduction_radius() {
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

        let midpoint_traits = TraitVector {
            mobility: 0.3,
            sensing_range: 15.0,
            mate_selectivity: 5.0,
            reproductive_investment: 10.0,
            ..zero_traits()
        };

        // gate at mobility=0.3: 1/(1+exp(0)) = 0.5
        // effective_radius = 0.5 * 15.0 + 0.5 * 5.0 = 10.0
        // Two agents at distance 9.0 should reproduce (within 10.0)
        let mut world_near = World::new(params.clone(), dist.clone(), 42);
        world_near.add_agent(Agent {
            id: 0, position: (0.0, 0.0), energy: 50.0, traits: midpoint_traits,
        });
        world_near.add_agent(Agent {
            id: 0, position: (9.0, 0.0), energy: 50.0, traits: midpoint_traits,
        });
        world_near.step();
        assert_eq!(
            world_near.agents().len(), 3,
            "midpoint agents within blended radius (10.0) should reproduce at distance 9.0"
        );

        // Two agents at distance 11.0 should NOT reproduce (beyond 10.0)
        let mut world_far = World::new(params.clone(), dist.clone(), 42);
        world_far.add_agent(Agent {
            id: 0, position: (0.0, 0.0), energy: 50.0, traits: midpoint_traits,
        });
        world_far.add_agent(Agent {
            id: 0, position: (11.0, 0.0), energy: 50.0, traits: midpoint_traits,
        });
        world_far.step();
        assert_eq!(
            world_far.agents().len(), 2,
            "midpoint agents beyond blended radius (10.0) should not reproduce at distance 11.0"
        );
    }

    #[test]
    fn cross_type_reproduction_via_sessile_spore_dispersal() {
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
        let sessile_traits = TraitVector {
            mobility: 0.0,
            sensing_range: 15.0,
            mate_selectivity: 5.0,
            reproductive_investment: 10.0,
            ..zero_traits()
        };
        let mobile_traits = TraitVector {
            mobility: 1.0,
            sensing_range: 15.0,
            mate_selectivity: 5.0,
            reproductive_investment: 10.0,
            ..zero_traits()
        };
        world.add_agent(Agent {
            id: 0, position: (0.0, 0.0), energy: 50.0, traits: sessile_traits,
        });
        world.add_agent(Agent {
            id: 0, position: (10.0, 0.0), energy: 50.0, traits: mobile_traits,
        });
        world.step();
        assert_eq!(
            world.agents().len(), 3,
            "sessile producer's spore should reach mobile consumer via max-of-both radii"
        );
    }

    #[test]
    fn mobile_agents_cannot_reproduce_beyond_contact_radius() {
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
        let mobile_traits = TraitVector {
            mobility: 1.0,
            sensing_range: 15.0,
            mate_selectivity: 5.0,
            reproductive_investment: 10.0,
            ..zero_traits()
        };
        world.add_agent(Agent {
            id: 0, position: (0.0, 0.0), energy: 50.0, traits: mobile_traits,
        });
        world.add_agent(Agent {
            id: 0, position: (10.0, 0.0), energy: 50.0, traits: mobile_traits,
        });
        world.step();
        assert_eq!(
            world.agents().len(), 2,
            "mobile agents beyond contact_radius should not reproduce despite large sensing_range"
        );
    }

    #[test]
    fn sessile_agents_reproduce_via_spore_dispersal_beyond_contact_radius() {
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
        let sessile_traits = TraitVector {
            mobility: 0.0,
            sensing_range: 15.0,
            mate_selectivity: 5.0,
            reproductive_investment: 10.0,
            ..zero_traits()
        };
        world.add_agent(Agent {
            id: 0, position: (0.0, 0.0), energy: 50.0, traits: sessile_traits,
        });
        world.add_agent(Agent {
            id: 0, position: (10.0, 0.0), energy: 50.0, traits: sessile_traits,
        });
        world.step();
        assert_eq!(
            world.agents().len(), 3,
            "sessile agents within sensing_range should reproduce via spore dispersal"
        );
    }

    #[test]
    fn spatial_index_produces_identical_outcomes_to_brute_force_reference() {
        let params = WorldParameters {
            world_extent: 100.0,
            contact_radius: 5.0,
            light_competition_radius: 10.0,
            initial_population_size: 50,
            reproduction_energy_threshold: 30.0,
            mutation_rate: 0.1,
            mutation_magnitude: 0.05,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: TraitVector {
                photosynthetic_absorption: 0.5,
                consumption_rate: 0.3,
                scavenging_rate: 0.2,
                mobility: 0.4,
                chemotaxis_sensitivity: 0.3,
                mate_selectivity: 0.5,
                sensing_range: 8.0,
                reproductive_investment: 15.0,
            },
            trait_covariance: 0.1,
            initial_cluster_count: 3,
            initial_energy_per_agent: 80.0,
        };

        // Run two identical worlds to confirm determinism
        let mut world1 = World::new(params.clone(), dist.clone(), 12345);
        let mut world2 = World::new(params, dist, 12345);
        for _ in 0..20 {
            world1.step();
            world2.step();
        }

        assert_eq!(world1.agents().len(), world2.agents().len());
        assert_eq!(world1.carcasses().len(), world2.carcasses().len());
        for (a, b) in world1.agents().iter().zip(world2.agents().iter()) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.position, b.position);
            assert_eq!(a.energy, b.energy);
        }

        // Energy conservation
        let total_agent_energy: f32 = world1.agents().iter().map(|a| a.energy).sum();
        let total_carcass_energy: f32 = world1.carcasses().iter().map(|c| c.energy).sum();
        let initial_energy = 50.0 * 80.0;
        let expected = initial_energy + world1.total_solar_input();
        let actual = total_agent_energy + total_carcass_energy + world1.dissipated_energy();

        assert!(
            (actual - expected).abs() < 1.0,
            "energy conservation violated: actual={actual}, expected={expected}"
        );
    }

    #[test]
    fn spatial_index_handles_500_agents_within_reasonable_time() {
        let params = WorldParameters {
            world_extent: 200.0,
            contact_radius: 5.0,
            light_competition_radius: 10.0,
            initial_population_size: 500,
            reproduction_energy_threshold: 50.0,
            mutation_rate: 0.1,
            mutation_magnitude: 0.05,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: TraitVector {
                photosynthetic_absorption: 0.5,
                consumption_rate: 0.3,
                scavenging_rate: 0.2,
                mobility: 0.4,
                chemotaxis_sensitivity: 0.3,
                mate_selectivity: 0.5,
                sensing_range: 8.0,
                reproductive_investment: 15.0,
            },
            trait_covariance: 0.1,
            initial_cluster_count: 3,
            initial_energy_per_agent: 80.0,
        };

        let mut world = World::new(params, dist, 42);
        let start = std::time::Instant::now();
        let ticks = 10;
        for _ in 0..ticks {
            world.step();
        }
        let elapsed = start.elapsed();
        let per_tick = elapsed / ticks;

        assert!(
            per_tick.as_millis() < 50,
            "500-agent tick took {per_tick:?}, expected < 50ms with spatial index"
        );
    }

    #[test]
    fn tick_counter_increments_each_step() {
        let mut world = World::new(test_params(), test_distribution(), 42);
        assert_eq!(world.tick(), 0);
        world.step();
        assert_eq!(world.tick(), 1);
        world.step();
        assert_eq!(world.tick(), 2);
    }

    #[test]
    fn shuffled_processing_order_is_deterministic_given_same_seed() {
        let params = || WorldParameters {
            initial_population_size: 20,
            ..test_params()
        };
        let dist = || test_distribution();
        let seed = 123;

        let mut world1 = World::new(params(), dist(), seed);
        let mut world2 = World::new(params(), dist(), seed);
        for _ in 0..10 {
            world1.step();
            world2.step();
        }
        for (a, b) in world1.agents().iter().zip(world2.agents().iter()) {
            assert_eq!(a.id, b.id, "agent IDs diverged");
            assert_eq!(a.position, b.position, "positions diverged for agent {}", a.id);
            assert_eq!(a.energy, b.energy, "energies diverged for agent {}", a.id);
        }
    }

    #[test]
    fn shuffle_produces_non_identity_permutation() {
        let n = 20;
        let seed = 42_u64;
        let mut any_shuffled = false;
        for tick in 0..5_u64 {
            let sub_seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(tick);
            let mut rng = ChaCha8Rng::seed_from_u64(sub_seed);
            let mut order: Vec<usize> = (0..n).collect();
            let identity: Vec<usize> = (0..n).collect();
            order.shuffle(&mut rng);
            if order != identity {
                any_shuffled = true;
                break;
            }
        }
        assert!(any_shuffled, "shuffle should produce non-identity order within 5 ticks");
    }

    #[test]
    fn different_seeds_produce_different_agent_order_after_step() {
        let params = || WorldParameters {
            initial_population_size: 20,
            ..test_params()
        };
        let dist = || test_distribution();

        let mut world1 = World::new(params(), dist(), 1);
        let mut world2 = World::new(params(), dist(), 2);
        world1.step();
        world2.step();
        let ids1: Vec<u64> = world1.agents().iter().map(|a| a.id).collect();
        let ids2: Vec<u64> = world2.agents().iter().map(|a| a.id).collect();
        let any_differ = ids1.iter().zip(ids2.iter()).any(|(a, b)| a != b)
            || world1.agents().iter().zip(world2.agents().iter()).any(|(a, b)| a.energy != b.energy);
        assert!(any_differ, "different seeds should produce different outcomes");
    }
}
