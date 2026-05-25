pub mod event;
pub mod spatial;
pub mod spatial_projection;
pub mod topology;

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

fn emit_broadcasts(
    broadcasts: &mut Vec<Broadcast>,
    kind: &event::EventKind,
    position: (f32, f32),
    agents: &[Agent],
    dead_agents: &std::collections::HashSet<usize>,
    nack_sets: &std::collections::HashMap<u64, std::collections::HashSet<event::EventKind>>,
    extent: f32,
) {
    for (i, agent) in agents.iter().enumerate() {
        if dead_agents.contains(&i) {
            continue;
        }
        if let Some(nacked) = nack_sets.get(&agent.id) {
            if nacked.contains(kind) {
                continue;
            }
        }
        let sensing_range = agent.traits.sensing_range;
        if sensing_range <= 0.0 {
            continue;
        }
        let dist = toroidal_distance(agent.position, position, extent);
        if dist <= sensing_range && dist > 0.0 {
            broadcasts.push(Broadcast {
                kind: kind.clone(),
                source_position: position,
                receiver_id: agent.id,
                strength: 1.0 / dist,
            });
        }
    }
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
    pub spatial_decay_rate: f32,
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

pub struct NearbyAgent {
    pub id: u64,
    pub distance: f32,
    pub energy: f32,
    pub traits: TraitVector,
}

pub struct NearbyCarcass {
    pub id: u64,
    pub distance: f32,
    pub energy: f32,
}

pub struct ProjectionData {
    pub feeding_gradient: (f32, f32),
    pub carcass_gradient: (f32, f32),
    pub nearby_agents: Vec<NearbyAgent>,
    pub nearby_carcasses: Vec<NearbyCarcass>,
    pub contact_radius: f32,
    pub reproduction_energy_threshold: f32,
}

pub struct Broadcast {
    pub kind: event::EventKind,
    pub source_position: (f32, f32),
    pub receiver_id: u64,
    pub strength: f32,
}


impl Agent {
    pub fn receive(&self, event: &event::Event, data: &ProjectionData) -> event::Response {
        match &event.kind {
            event::EventKind::Consumed if self.traits.consumption_rate > 0.0 => {
                let best = data.nearby_agents.iter()
                    .filter(|a| a.distance < data.contact_radius && a.energy > 0.0)
                    .min_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());

                if let Some(target) = best {
                    let drain = self.traits.consumption_rate.min(target.energy.max(0.0));
                    if drain > 0.0 {
                        return event::Response {
                            ack: event::Ack::Ack,
                            events: vec![event::Event {
                                tick: event.tick,
                                seq: 0,
                                kind: event::EventKind::Consumed,
                                source: self.id,
                                target: Some(target.id),
                                energy_delta: drain,
                                position: Some(self.position),
                            }],
                        };
                    }
                }
                event::Response { ack: event::Ack::Ack, events: vec![] }
            }
            event::EventKind::MatingReadiness => {
                if self.energy <= data.reproduction_energy_threshold {
                    return event::Response { ack: event::Ack::Ack, events: vec![] };
                }
                let self_gate = 1.0 / (1.0 + (20.0_f32 * (self.traits.mobility - 0.3)).exp());
                let self_radius = self_gate * self.traits.sensing_range
                    + (1.0 - self_gate) * data.contact_radius;

                let best = data.nearby_agents.iter()
                    .filter(|mate| {
                        if mate.energy <= data.reproduction_energy_threshold {
                            return false;
                        }
                        let mate_gate = 1.0
                            / (1.0 + (20.0_f32 * (mate.traits.mobility - 0.3)).exp());
                        let mate_radius = mate_gate * mate.traits.sensing_range
                            + (1.0 - mate_gate) * data.contact_radius;
                        if mate.distance > self_radius.max(mate_radius) {
                            return false;
                        }
                        let trait_dist = self.traits.distance(&mate.traits);
                        trait_dist < self.traits.mate_selectivity
                            && trait_dist < mate.traits.mate_selectivity
                    })
                    .min_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());

                match best {
                    Some(mate) => event::Response {
                        ack: event::Ack::Ack,
                        events: vec![event::Event {
                            tick: event.tick,
                            seq: 0,
                            kind: event::EventKind::MateSelected,
                            source: self.id,
                            target: Some(mate.id),
                            energy_delta: 0.0,
                            position: Some(self.position),
                        }],
                    },
                    None => event::Response { ack: event::Ack::Ack, events: vec![] },
                }
            }
            event::EventKind::CarcassCreated if self.traits.scavenging_rate > 0.0 => {
                let best = data.nearby_carcasses.iter()
                    .filter(|c| c.distance < data.contact_radius && c.energy > 0.0)
                    .min_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());

                if let Some(target) = best {
                    let drain = self.traits.scavenging_rate.min(target.energy.max(0.0));
                    if drain > 0.0 {
                        return event::Response {
                            ack: event::Ack::Ack,
                            events: vec![event::Event {
                                tick: event.tick,
                                seq: 0,
                                kind: event::EventKind::Decomposed,
                                source: self.id,
                                target: Some(target.id),
                                energy_delta: drain,
                                position: Some(self.position),
                            }],
                        };
                    }
                }
                event::Response { ack: event::Ack::Ack, events: vec![] }
            }
            event::EventKind::Moved if self.traits.mobility > 0.0 => {
                let mut cx = 0.0_f32;
                let mut cy = 0.0_f32;

                if self.traits.consumption_rate > 0.0 {
                    cx += self.traits.consumption_rate * data.feeding_gradient.0;
                    cy += self.traits.consumption_rate * data.feeding_gradient.1;
                }
                if self.traits.scavenging_rate > 0.0 {
                    cx += self.traits.scavenging_rate * data.carcass_gradient.0;
                    cy += self.traits.scavenging_rate * data.carcass_gradient.1;
                }

                cx *= self.traits.chemotaxis_sensitivity;
                cy *= self.traits.chemotaxis_sensitivity;

                event::Response {
                    ack: event::Ack::Ack,
                    events: vec![event::Event {
                        tick: event.tick,
                        seq: 0,
                        kind: event::EventKind::Moved,
                        source: self.id,
                        target: None,
                        energy_delta: 0.0,
                        position: Some((cx, cy)),
                    }],
                }
            }
            _ => event::Response { ack: event::Ack::Nack, events: vec![] },
        }
    }
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
    event_log: event::EventLog,
    next_seq: u64,
    projection: spatial_projection::SpatialProjection,
    tick_broadcasts: Vec<Broadcast>,
    nack_sets: std::collections::HashMap<u64, std::collections::HashSet<event::EventKind>>,
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

        let projection = spatial_projection::SpatialProjection::new(
            params.world_extent,
            params.world_extent / 10.0,
            params.spatial_decay_rate,
        );

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
            event_log: event::EventLog::new(),
            next_seq: 0,
            projection,
            tick_broadcasts: Vec::new(),
            nack_sets: std::collections::HashMap::new(),
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
        self.tick_broadcasts.clear();
        let extent = self.params.world_extent;
        let agent_count = self.agents.len();

        let sub_seed = self
            .seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(self.tick);
        let mut shuffle_rng = ChaCha8Rng::seed_from_u64(sub_seed);
        let mut order: Vec<usize> = (0..agent_count).collect();
        order.shuffle(&mut shuffle_rng);

        let max_query_radius = self.params.light_competition_radius
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

        // --- Update spatial projection from event log ---
        self.projection.update(&self.event_log, self.tick);

        // --- Sense & Decide: DES computes projections and hands data to agents ---
        let mut movements = vec![(0.0_f32, 0.0_f32); agent_count];
        for &i in &order {
            let agent = &self.agents[i];
            if agent.traits.mobility <= 0.0 {
                continue;
            }

            // DES pre-computes projection gradients for this agent
            let feeding_gradient = self.projection.gradient(
                agent.position,
                agent.traits.sensing_range,
                spatial_projection::ActivityLayer::Feeding,
            );
            let carcass_gradient = self.projection.gradient(
                agent.position,
                agent.traits.sensing_range,
                spatial_projection::ActivityLayer::Carcass,
            );

            let data = ProjectionData {
                feeding_gradient,
                carcass_gradient,
                nearby_agents: vec![],
                nearby_carcasses: vec![],
                contact_radius: self.params.contact_radius,
                reproduction_energy_threshold: self.params.reproduction_energy_threshold,
            };
            let trigger = event::Event {
                tick: self.tick, seq: 0, kind: event::EventKind::Moved,
                source: 0, target: None, energy_delta: 0.0,
                position: Some(agent.position),
            };
            let response = agent.receive(&trigger, &data);

            // Extract chemotaxis vector from agent response
            let (chemotaxis_x, chemotaxis_y) = if let Some(moved) = response.events.first() {
                moved.position.unwrap_or((0.0, 0.0))
            } else {
                (0.0, 0.0)
            };

            // DES adds exploration noise (requires RNG on World)
            let angle_dist =
                rand::distr::Uniform::new(0.0_f32, std::f32::consts::TAU).unwrap();
            let angle = angle_dist.sample(&mut self.rng);
            let explore_x = angle.cos();
            let explore_y = angle.sin();

            let mut move_x = chemotaxis_x + explore_x;
            let mut move_y = chemotaxis_y + explore_y;

            // DES normalizes and scales by mobility
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

        // --- Apply movement and metabolic costs ---
        let pre_tick_energies: Vec<f32> = self.agents.iter().map(|a| a.energy).collect();

        for (i, agent) in self.agents.iter_mut().enumerate() {
            let (mx, my) = movements[i];
            agent.position = wrap_position(
                (agent.position.0 + mx, agent.position.1 + my),
                extent,
            );
            let distance_moved = (mx * mx + my * my).sqrt();
            let movement_cost = distance_moved * self.params.movement_cost_coefficient;
            let metabolic_cost = self.params.base_metabolic_rate
                + agent.traits.sensing_range * self.params.sensing_cost_coefficient
                + agent.traits.photosynthetic_absorption * self.params.photo_maintenance_cost
                + agent.traits.consumption_rate * self.params.consumption_maintenance_cost
                + agent.traits.scavenging_rate * self.params.scavenging_maintenance_cost;

            agent.energy -= metabolic_cost + movement_cost;
        }

        // Rebuild grids with post-move positions
        let mut agent_grid = crate::spatial::SpatialGrid::new(extent, cell_size);
        for (i, agent) in self.agents.iter().enumerate() {
            agent_grid.insert(i as u64, agent.position);
        }

        // --- DES: sequential agent processing with priority event queue ---
        let contact_radius = self.params.contact_radius;
        let consumption_efficiency = self.params.consumption_efficiency;
        let decomposition_efficiency = self.params.decomposition_efficiency;
        let reproduction_threshold = self.params.reproduction_energy_threshold;
        let reproduction_efficiency = self.params.reproduction_efficiency;
        let mutation_rate = self.params.mutation_rate;
        let mutation_magnitude = self.params.mutation_magnitude;
        let n = agent_count;

        let mut broadcasts: Vec<Broadcast> = Vec::new();
        let mut consequence_queue = event::EventQueue::new();

        let mut dead_agents: std::collections::HashSet<usize> =
            std::collections::HashSet::new();
        let mut depleted_carcasses: std::collections::HashSet<usize> =
            std::collections::HashSet::new();
        let mut reproduced = vec![false; n];
        let mut consumption_gains = vec![0.0_f32; n];
        let mut consumption_losses = vec![0.0_f32; n];
        let mut decomposition_gains = vec![0.0_f32; n];
        let mut solar_gains = vec![0.0_f32; n];
        let mut reproduction_investments = vec![0.0_f32; n];
        let mut offspring: Vec<Agent> = Vec::new();

        for &i in &order {
            if dead_agents.contains(&i) || self.agents[i].energy <= 0.0 {
                continue;
            }

            // --- Energy acquisition (mutually exclusive) ---
            let mut acquired = false;

            // Try consumption via broadcast-response
            if !acquired && self.agents[i].traits.consumption_rate > 0.0 {
                let neighbors =
                    agent_grid.query_radius(self.agents[i].position, contact_radius);
                let nearby_agents: Vec<NearbyAgent> = neighbors.iter()
                    .filter_map(|&j_id| {
                        let j = j_id as usize;
                        if j == i || dead_agents.contains(&j) || self.agents[j].energy <= 0.0 {
                            return None;
                        }
                        let dist = toroidal_distance(
                            self.agents[i].position, self.agents[j].position, extent,
                        );
                        Some(NearbyAgent {
                            id: self.agents[j].id, distance: dist,
                            energy: self.agents[j].energy, traits: self.agents[j].traits,
                        })
                    })
                    .collect();
                let data = ProjectionData {
                    feeding_gradient: (0.0, 0.0), carcass_gradient: (0.0, 0.0),
                    nearby_agents, nearby_carcasses: vec![],
                    contact_radius, reproduction_energy_threshold: reproduction_threshold,
                };
                let trigger = event::Event {
                    tick: self.tick, seq: 0, kind: event::EventKind::Consumed,
                    source: 0, target: None, energy_delta: 0.0,
                    position: Some(self.agents[i].position),
                };
                let response = self.agents[i].receive(&trigger, &data);
                if let Some(consumed) = response.events.first() {
                    let target_id = consumed.target.unwrap();
                    let target = (0..n).find(|&j| self.agents[j].id == target_id).unwrap();
                    let drain = consumed.energy_delta;
                    let gain = drain * consumption_efficiency;
                    self.agents[i].energy += gain;
                    self.agents[target].energy -= drain;
                    consumption_gains[i] += gain;
                    consumption_losses[target] += drain;
                    self.dissipated_energy += drain * (1.0 - consumption_efficiency);
                    self.emit(
                        event::EventKind::Consumed,
                        self.agents[i].id,
                        Some(self.agents[target].id),
                        drain,
                        Some(self.agents[i].position),
                    );
                    emit_broadcasts(
                        &mut broadcasts,
                        &event::EventKind::Consumed,
                        self.agents[i].position,
                        &self.agents,
                        &dead_agents,
                        &self.nack_sets,
                        extent,
                    );

                    // Queue consequence: target death → carcass creation
                    if self.agents[target].energy <= 0.0 {
                        let carcass_energy =
                            (pre_tick_energies[target] - consumption_losses[target])
                                .max(0.0);
                        consequence_queue.push_high(event::Event {
                            tick: self.tick,
                            seq: 0, // seq assigned when emitting to log
                            kind: event::EventKind::Died,
                            source: self.agents[target].id,
                            target: None,
                            energy_delta: carcass_energy,
                            position: Some(self.agents[target].position),
                        });
                    }
                    acquired = true;
                }
            }

            // Try decomposition via broadcast-response
            if !acquired && self.agents[i].traits.scavenging_rate > 0.0 {
                let nearby_carcasses: Vec<NearbyCarcass> = carcass_grid
                    .query_radius(self.agents[i].position, contact_radius)
                    .iter()
                    .filter_map(|&ci_id| {
                        let ci = ci_id as usize;
                        if self.carcasses[ci].energy <= 0.0 {
                            return None;
                        }
                        let dist = toroidal_distance(
                            self.agents[i].position, self.carcasses[ci].position, extent,
                        );
                        Some(NearbyCarcass {
                            id: self.carcasses[ci].id, distance: dist,
                            energy: self.carcasses[ci].energy,
                        })
                    })
                    .collect();
                let data = ProjectionData {
                    feeding_gradient: (0.0, 0.0), carcass_gradient: (0.0, 0.0),
                    nearby_agents: vec![], nearby_carcasses,
                    contact_radius, reproduction_energy_threshold: reproduction_threshold,
                };
                let trigger = event::Event {
                    tick: self.tick, seq: 0, kind: event::EventKind::CarcassCreated,
                    source: 0, target: None, energy_delta: 0.0,
                    position: Some(self.agents[i].position),
                };
                let response = self.agents[i].receive(&trigger, &data);
                if let Some(decomposed) = response.events.first() {
                    let carcass_id = decomposed.target.unwrap();
                    let ci = self.carcasses.iter().position(|c| c.id == carcass_id).unwrap();
                    let drain = decomposed.energy_delta;
                    let gain = drain * decomposition_efficiency;
                    self.agents[i].energy += gain;
                    self.carcasses[ci].energy -= drain;
                    decomposition_gains[i] = gain;
                    self.dissipated_energy += drain * (1.0 - decomposition_efficiency);
                    self.emit(
                        event::EventKind::Decomposed,
                        self.agents[i].id,
                        Some(self.carcasses[ci].id),
                        drain,
                        Some(self.agents[i].position),
                    );
                    emit_broadcasts(
                        &mut broadcasts,
                        &event::EventKind::Decomposed,
                        self.agents[i].position,
                        &self.agents,
                        &dead_agents,
                        &self.nack_sets,
                        extent,
                    );

                    // Queue consequence: carcass depletion
                    if self.carcasses[ci].energy <= 0.0 {
                        consequence_queue.push_high(event::Event {
                            tick: self.tick,
                            seq: 0,
                            kind: event::EventKind::CarcassDepleted,
                            source: self.carcasses[ci].id,
                            target: None,
                            energy_delta: 0.0,
                            position: Some(self.carcasses[ci].position),
                        });
                    }
                    acquired = true;
                }
            }

            // --- Drain consequence queue before next agent decision ---
            while let Some(consequence) = consequence_queue.pop() {
                match consequence.kind {
                    event::EventKind::Died => {
                        let dead_id = consequence.source;
                        let dead_pos = consequence.position.unwrap();
                        let carcass_energy = consequence.energy_delta;
                        self.emit(
                            event::EventKind::Died,
                            dead_id,
                            None,
                            0.0,
                            Some(dead_pos),
                        );
                        // Queue CarcassCreated as next consequence
                        consequence_queue.push_high(event::Event {
                            tick: self.tick,
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
                            &self.agents,
                            &dead_agents,
                            &self.nack_sets,
                            extent,
                        );
                        let target_idx = (0..n).find(|&j| self.agents[j].id == dead_id).unwrap();
                        dead_agents.insert(target_idx);
                        agent_grid.remove(target_idx as u64);
                    }
                    event::EventKind::CarcassCreated => {
                        let dead_id = consequence.source;
                        let dead_pos = consequence.position.unwrap();
                        let carcass_energy = consequence.energy_delta;
                        self.emit(
                            event::EventKind::CarcassCreated,
                            dead_id,
                            None,
                            carcass_energy,
                            Some(dead_pos),
                        );
                        let ci = self.carcasses.len();
                        self.carcasses.push(Carcass {
                            id: dead_id,
                            position: dead_pos,
                            energy: carcass_energy,
                        });
                        carcass_grid.insert(ci as u64, dead_pos);
                        emit_broadcasts(
                            &mut broadcasts,
                            &event::EventKind::CarcassCreated,
                            dead_pos,
                            &self.agents,
                            &dead_agents,
                            &self.nack_sets,
                            extent,
                        );
                    }
                    event::EventKind::CarcassDepleted => {
                        let carcass_id = consequence.source;
                        let carcass_pos = consequence.position.unwrap();
                        self.emit(
                            event::EventKind::CarcassDepleted,
                            carcass_id,
                            None,
                            0.0,
                            Some(carcass_pos),
                        );
                        let ci = self.carcasses.iter().position(|c| c.id == carcass_id).unwrap();
                        depleted_carcasses.insert(ci);
                        carcass_grid.remove(ci as u64);
                    }
                    _ => {}
                }
            }

            // Photosynthesise (fallback)
            if !acquired {
                let mobility_gate = 1.0
                    / (1.0
                        + (20.0_f32 * (self.agents[i].traits.mobility - 0.3)).exp());
                let solar_gain = self.agents[i].traits.photosynthetic_absorption
                    * self.params.solar_flux_magnitude
                    * mobility_gate
                    * light_shares[i];
                self.agents[i].energy += solar_gain;
                self.total_solar_input += solar_gain;
                solar_gains[i] = solar_gain;
            }

            // --- Reproduction via broadcast-response ---
            if !reproduced[i] && self.agents[i].energy > reproduction_threshold {
                emit_broadcasts(
                    &mut broadcasts,
                    &event::EventKind::MatingReadiness,
                    self.agents[i].position,
                    &self.agents,
                    &dead_agents,
                    &self.nack_sets,
                    extent,
                );

                // Build nearby agents for mate selection via receive()
                let nearby_mates: Vec<NearbyAgent> = (0..n)
                    .filter_map(|j| {
                        if j == i || dead_agents.contains(&j) || reproduced[j]
                            || self.agents[j].energy <= reproduction_threshold
                        {
                            return None;
                        }
                        let dist = toroidal_distance(
                            self.agents[i].position, self.agents[j].position, extent,
                        );
                        Some(NearbyAgent {
                            id: self.agents[j].id, distance: dist,
                            energy: self.agents[j].energy, traits: self.agents[j].traits,
                        })
                    })
                    .collect();
                let data = ProjectionData {
                    feeding_gradient: (0.0, 0.0), carcass_gradient: (0.0, 0.0),
                    nearby_agents: nearby_mates, nearby_carcasses: vec![],
                    contact_radius, reproduction_energy_threshold: reproduction_threshold,
                };
                let trigger = event::Event {
                    tick: self.tick, seq: 0, kind: event::EventKind::MatingReadiness,
                    source: self.agents[i].id, target: None, energy_delta: 0.0,
                    position: Some(self.agents[i].position),
                };
                let response = self.agents[i].receive(&trigger, &data);
                if let Some(mate_event) = response.events.first() {
                    let mate_id = mate_event.target.unwrap();
                    let j = (0..n).find(|&k| self.agents[k].id == mate_id).unwrap();
                    reproduced[i] = true;
                    reproduced[j] = true;
                    self.emit(
                        event::EventKind::MateSelected,
                        self.agents[i].id,
                        Some(self.agents[j].id),
                        0.0,
                        Some(self.agents[i].position),
                    );

                    let inv_a = self.agents[i].traits.reproductive_investment;
                    let inv_b = self.agents[j].traits.reproductive_investment;
                    self.agents[i].energy -= inv_a;
                    self.agents[j].energy -= inv_b;
                    reproduction_investments[i] = inv_a;
                    reproduction_investments[j] = inv_b;

                    let offspring_energy = (inv_a + inv_b) * reproduction_efficiency;
                    self.dissipated_energy +=
                        (inv_a + inv_b) * (1.0 - reproduction_efficiency);

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
                            self.agents[i].traits.get(dim)
                        } else {
                            self.agents[j].traits.get(dim)
                        };
                        child_traits.set(dim, val);
                    }

                    if mutation_rate > 0.0 {
                        let mutation_dist =
                            Normal::new(0.0_f32, mutation_magnitude).unwrap();
                        for dim in 0..8 {
                            let r: f32 = rand::distr::Uniform::new(0.0_f32, 1.0)
                                .unwrap()
                                .sample(&mut self.rng);
                            if r < mutation_rate {
                                let perturbation = mutation_dist.sample(&mut self.rng);
                                child_traits
                                    .set(dim, child_traits.get(dim) + perturbation);
                            }
                        }
                    }

                    let offspring_id = self.next_agent_id;
                    self.next_agent_id += 1;
                    self.emit(
                        event::EventKind::Born,
                        offspring_id,
                        None,
                        offspring_energy,
                        Some(self.agents[i].position),
                    );
                    offspring.push(Agent {
                        id: offspring_id,
                        position: self.agents[i].position,
                        energy: offspring_energy,
                        traits: child_traits,
                    });
                }
            }
        }

        // Assert: consequence queue is empty at end of tick
        debug_assert!(consequence_queue.is_empty(), "consequence queue must be empty at end of tick");

        // --- Deliver broadcasts and register NACKs ---
        // Build an index from agent id to index for broadcast delivery
        let agent_id_to_idx: std::collections::HashMap<u64, usize> = self.agents.iter()
            .enumerate()
            .map(|(i, a)| (a.id, i))
            .collect();
        for broadcast in &broadcasts {
            if dead_agents.contains(agent_id_to_idx.get(&broadcast.receiver_id).unwrap_or(&usize::MAX)) {
                continue;
            }
            if let Some(&idx) = agent_id_to_idx.get(&broadcast.receiver_id) {
                let agent = &self.agents[idx];
                let data = ProjectionData {
                    feeding_gradient: (0.0, 0.0),
                    carcass_gradient: (0.0, 0.0),
                    nearby_agents: vec![],
                    nearby_carcasses: vec![],
                    contact_radius,
                    reproduction_energy_threshold: reproduction_threshold,
                };
                let trigger = event::Event {
                    tick: self.tick, seq: 0, kind: broadcast.kind.clone(),
                    source: 0, target: None, energy_delta: 0.0,
                    position: Some(broadcast.source_position),
                };
                let response = agent.receive(&trigger, &data);
                if response.ack == event::Ack::Nack {
                    self.nack_sets
                        .entry(agent.id)
                        .or_insert_with(std::collections::HashSet::new)
                        .insert(broadcast.kind.clone());
                }
                // NACK response events are ignored — only ACK'd events would be queued
            }
        }

        // --- End-of-tick death check and energy accounting ---
        let mut next_agents: Vec<Agent> = Vec::with_capacity(n);
        let mut next_carcasses: Vec<Carcass> = self
            .carcasses
            .drain(..)
            .enumerate()
            .filter(|(ci, _)| !depleted_carcasses.contains(ci))
            .map(|(_, c)| c)
            .collect();

        let remaining_agents: Vec<(usize, Agent)> = self
            .agents
            .drain(..)
            .enumerate()
            .filter(|(i, _)| !dead_agents.contains(i))
            .collect();

        for (i, agent) in remaining_agents {
            if agent.energy <= 0.0 {
                let carcass_energy =
                    (pre_tick_energies[i] - consumption_losses[i]).max(0.0);
                self.emit(
                    event::EventKind::Died,
                    agent.id,
                    None,
                    0.0,
                    Some(agent.position),
                );
                self.emit(
                    event::EventKind::CarcassCreated,
                    agent.id,
                    None,
                    carcass_energy,
                    Some(agent.position),
                );
                next_carcasses.push(Carcass {
                    id: agent.id,
                    position: agent.position,
                    energy: carcass_energy,
                });
                self.dissipated_energy +=
                    solar_gains[i] + consumption_gains[i] + decomposition_gains[i];
            } else {
                let total_input = solar_gains[i] + consumption_gains[i]
                    + decomposition_gains[i]
                    - consumption_losses[i];
                let costs = pre_tick_energies[i] + total_input
                    - agent.energy
                    - reproduction_investments[i];
                self.dissipated_energy += costs;
                next_agents.push(agent);
            }
        }

        self.last_tick_births = offspring.len();
        self.last_tick_deaths = dead_agents.len() + (n - dead_agents.len() - next_agents.len());
        next_agents.extend(offspring);

        // Clean up NACK state for dead agents
        let live_ids: std::collections::HashSet<u64> = next_agents.iter().map(|a| a.id).collect();
        self.nack_sets.retain(|id, _| live_ids.contains(id));

        self.agents = next_agents;
        self.carcasses = next_carcasses;
        self.tick_broadcasts = broadcasts;
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

    pub fn event_log(&self) -> &event::EventLog {
        &self.event_log
    }

    pub fn tick_broadcasts(&self) -> &[Broadcast] {
        &self.tick_broadcasts
    }

    pub fn has_nacked(&self, agent_id: u64, kind: &event::EventKind) -> bool {
        self.nack_sets
            .get(&agent_id)
            .map_or(false, |set| set.contains(kind))
    }

    fn emit(&mut self, kind: event::EventKind, source: u64, target: Option<u64>, energy_delta: f32, position: Option<(f32, f32)>) {
        let seq = self.next_seq;
        self.next_seq += 1;
        let _ = self.event_log.append(event::Event {
            tick: self.tick,
            seq,
            kind,
            source,
            target,
            energy_delta,
            position,
        });
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
spatial_decay_rate: 0.5,

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
    fn consumer_moves_toward_feeding_activity() {
        // Consumers navigate by reading the feeding activity layer in the spatial
        // projection. Seed the log with prior feeding events to create a gradient.
        let extent = 100.0;
        let mut moved_closer = 0;
        let trials = 50;
        for seed in 0..trials {
            let params = WorldParameters {
                solar_flux_magnitude: 0.0,
                base_metabolic_rate: 0.0,
                sensing_cost_coefficient: 0.0,
                movement_cost_coefficient: 0.0,
                world_extent: extent,
                initial_population_size: 0,
                spatial_decay_rate: 0.1,
                ..test_params()
            };
            let dist = InitialDistribution {
                mean_traits: zero_traits(),
                trait_covariance: 0.0,
                initial_cluster_count: 1,
                initial_energy_per_agent: 100.0,
            };
            let mut world = World::new(params, dist, seed);
            // Prior feeding activity at (20,0)
            world.emit(
                event::EventKind::Consumed,
                99,
                Some(100),
                20.0,
                Some((20.0, 0.0)),
            );
            world.add_agent(Agent {
                id: 0,
                position: (0.0, 0.0),
                energy: 100.0,
                traits: TraitVector {
                    consumption_rate: 1.0,
                    mobility: 5.0,
                    chemotaxis_sensitivity: 10.0,
                    sensing_range: 30.0,
                    ..zero_traits()
                },
            });
            let initial_x = world.agents()[0].position.0;
            world.step();
            if world.agents()[0].position.0 > initial_x {
                moved_closer += 1;
            }
        }
        assert!(
            moved_closer > 40,
            "consumer should move toward feeding activity: {moved_closer}/{trials}",
        );
    }

    #[test]
    fn scavenger_moves_toward_carcass_activity() {
        // Scavengers navigate by reading the carcass activity layer in the spatial
        // projection. Seed the log with a CarcassCreated event to create a gradient.
        let extent = 100.0;
        let mut moved_closer = 0;
        let trials = 50;
        for seed in 0..trials {
            let params = WorldParameters {
                solar_flux_magnitude: 0.0,
                base_metabolic_rate: 0.0,
                sensing_cost_coefficient: 0.0,
                movement_cost_coefficient: 0.0,
                world_extent: extent,
                initial_population_size: 0,
                spatial_decay_rate: 0.1,
                ..test_params()
            };
            let dist = InitialDistribution {
                mean_traits: zero_traits(),
                trait_covariance: 0.0,
                initial_cluster_count: 1,
                initial_energy_per_agent: 100.0,
            };
            let mut world = World::new(params, dist, seed);
            // Prior carcass activity at (20,0)
            world.emit(
                event::EventKind::CarcassCreated,
                99,
                None,
                30.0,
                Some((20.0, 0.0)),
            );
            world.add_agent(Agent {
                id: 0,
                position: (0.0, 0.0),
                energy: 100.0,
                traits: TraitVector {
                    scavenging_rate: 1.0,
                    mobility: 5.0,
                    chemotaxis_sensitivity: 10.0,
                    sensing_range: 30.0,
                    ..zero_traits()
                },
            });
            let initial_x = world.agents()[0].position.0;
            world.step();
            if world.agents()[0].position.0 > initial_x {
                moved_closer += 1;
            }
        }
        assert!(
            moved_closer > 40,
            "scavenger should move toward carcass activity: {moved_closer}/{trials}",
        );
    }

    #[test]
    fn projection_gradient_guides_movement_across_toroidal_boundary() {
        let extent = 100.0;
        let mut moved_across_boundary = 0;
        let trials = 50;

        for seed in 0..trials {
            let params = WorldParameters {
                solar_flux_magnitude: 0.0,
                base_metabolic_rate: 0.0,
                sensing_cost_coefficient: 0.0,
                movement_cost_coefficient: 0.0,
                world_extent: extent,
                initial_population_size: 0,
                spatial_decay_rate: 0.1,
                ..test_params()
            };
            let dist = InitialDistribution {
                mean_traits: zero_traits(),
                trait_covariance: 0.0,
                initial_cluster_count: 1,
                initial_energy_per_agent: 100.0,
            };
            let mut world = World::new(params, dist, seed);

            // Feeding activity at +48 — across the toroidal boundary from -48
            world.emit(
                event::EventKind::Consumed,
                99,
                Some(100),
                50.0,
                Some((48.0, 0.0)),
            );

            // Consumer at -48 should detect gradient across boundary
            world.add_agent(Agent {
                id: 0,
                position: (-48.0, 0.0),
                energy: 100.0,
                traits: TraitVector {
                    consumption_rate: 1.0,
                    mobility: 5.0,
                    chemotaxis_sensitivity: 10.0,
                    sensing_range: 10.0,
                    ..zero_traits()
                },
            });

            let initial_pos = world.agents()[0].position;
            world.step();
            let final_pos = world.agents()[0].position;

            // Should move in negative-x direction (toward boundary) or wrap past it
            if final_pos.0 < initial_pos.0 || final_pos.0 > 45.0 {
                moved_across_boundary += 1;
            }
        }

        assert!(
            moved_across_boundary > 40,
            "consumer should be guided across toroidal boundary: {moved_across_boundary}/{trials}",
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
            diff < 1.0,
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
spatial_decay_rate: 0.5,

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
spatial_decay_rate: 0.5,

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
spatial_decay_rate: 0.5,

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
spatial_decay_rate: 0.5,

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
spatial_decay_rate: 0.5,

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

    #[test]
    fn new_world_has_empty_event_log() {
        let world = World::new(test_params(), test_distribution(), 42);
        assert!(world.event_log().is_empty());
    }

    fn event_test_params() -> WorldParameters {
        WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.5,
            reproduction_efficiency: 0.7,
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

        }
    }

    fn event_test_dist() -> InitialDistribution {
        InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        }
    }

    #[test]
    fn step_emits_consumed_event_when_consumer_drains_victim() {
        use crate::event::EventKind;
        let mut world = World::new(event_test_params(), event_test_dist(), 42);
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                consumption_rate: 2.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            id: 0,
            position: (1.0, 0.0),
            energy: 50.0,
            traits: zero_traits(),
        });
        world.step();
        let consumed = world.event_log().by_kind(&EventKind::Consumed);
        assert_eq!(consumed.len(), 1);
        assert_eq!(consumed[0].tick, 0);
        assert!(consumed[0].energy_delta > 0.0);
    }

    #[test]
    fn step_emits_decomposed_and_carcass_depleted_events() {
        use crate::event::EventKind;
        let mut world = World::new(event_test_params(), event_test_dist(), 42);
        // Scavenger adjacent to a low-energy carcass
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                scavenging_rate: 10.0,
                ..zero_traits()
            },
        });
        world.add_carcass(Carcass {
            id: 99,
            position: (1.0, 0.0),
            energy: 3.0, // will be fully drained (scavenging_rate 10 > 3)
        });
        world.step();
        let decomposed = world.event_log().by_kind(&EventKind::Decomposed);
        assert_eq!(decomposed.len(), 1);
        assert!(decomposed[0].energy_delta > 0.0);
        assert_eq!(decomposed[0].target, Some(99));

        let depleted = world.event_log().by_kind(&EventKind::CarcassDepleted);
        assert_eq!(depleted.len(), 1);
        assert_eq!(depleted[0].source, 99);
    }

    #[test]
    fn step_emits_died_and_carcass_created_events() {
        use crate::event::EventKind;
        let params = WorldParameters {
            base_metabolic_rate: 200.0, // will kill on first tick
            ..event_test_params()
        };
        let mut world = World::new(params, event_test_dist(), 42);
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            energy: 100.0,
            traits: zero_traits(),
        });
        let agent_id = world.agents()[0].id;
        world.step();
        let died = world.event_log().by_kind(&EventKind::Died);
        assert_eq!(died.len(), 1);
        assert_eq!(died[0].source, agent_id);

        let created = world.event_log().by_kind(&EventKind::CarcassCreated);
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].source, agent_id);
        assert!(created[0].energy_delta > 0.0);
    }

    #[test]
    fn step_emits_mate_selected_and_born_events() {
        use crate::event::EventKind;
        let mut world = World::new(event_test_params(), event_test_dist(), 42);
        // Two compatible agents with high energy, adjacent, identical traits
        let repro_traits = TraitVector {
            reproductive_investment: 20.0,
            mate_selectivity: 10.0, // very permissive
            sensing_range: 10.0,
            ..zero_traits()
        };
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            energy: 100.0,
            traits: repro_traits,
        });
        world.add_agent(Agent {
            id: 0,
            position: (1.0, 0.0),
            energy: 100.0,
            traits: repro_traits,
        });
        world.step();
        let mates = world.event_log().by_kind(&EventKind::MateSelected);
        assert_eq!(mates.len(), 1);

        let born = world.event_log().by_kind(&EventKind::Born);
        assert_eq!(born.len(), 1);
        assert!(born[0].energy_delta > 0.0);
    }

    #[test]
    fn multi_tick_events_have_monotonic_seq_and_correct_ticks() {
        use crate::event::EventKind;
        let params = WorldParameters {
            base_metabolic_rate: 10.0,
            ..event_test_params()
        };
        let mut world = World::new(params, event_test_dist(), 42);
        // Agent that will die on tick 10 (energy 100 / metabolic 10)
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            energy: 100.0,
            traits: zero_traits(),
        });
        for _ in 0..15 {
            world.step();
        }
        let log = world.event_log();
        assert!(!log.is_empty());
        // Verify monotonic seq
        let events = log.by_tick_range(0, 100);
        for window in events.windows(2) {
            assert!(window[1].seq > window[0].seq,
                "seq must increase: {} vs {}", window[0].seq, window[1].seq);
        }
        // Verify Died event has correct tick
        let died = log.by_kind(&EventKind::Died);
        assert_eq!(died.len(), 1);
        assert!(died[0].tick > 0, "should die after tick 0");
        assert!(died[0].tick <= 10);
    }

    #[test]
    fn consequence_cascade_consumption_to_death_to_carcass() {
        use crate::event::EventKind;
        let mut world = World::new(event_test_params(), event_test_dist(), 42);
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                consumption_rate: 20.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            id: 0,
            position: (1.0, 0.0),
            energy: 5.0,
            traits: zero_traits(),
        });
        world.step();
        let log = world.event_log();
        let events = log.by_tick_range(0, 1);
        let kinds: Vec<&EventKind> = events.iter().map(|e| &e.kind).collect();
        assert!(
            kinds.contains(&&EventKind::Consumed),
            "should have Consumed event"
        );
        assert!(
            kinds.contains(&&EventKind::Died),
            "should have Died event"
        );
        assert!(
            kinds.contains(&&EventKind::CarcassCreated),
            "should have CarcassCreated event"
        );
        // Consumed must come before Died, Died before CarcassCreated
        let consumed_seq = events.iter().find(|e| e.kind == EventKind::Consumed).unwrap().seq;
        let died_seq = events.iter().find(|e| e.kind == EventKind::Died).unwrap().seq;
        let carcass_seq = events.iter().find(|e| e.kind == EventKind::CarcassCreated).unwrap().seq;
        assert!(consumed_seq < died_seq, "Consumed must precede Died");
        assert!(died_seq < carcass_seq, "Died must precede CarcassCreated");
        // Target should be dead and a carcass should exist
        assert_eq!(world.agents().len(), 1);
        assert_eq!(world.carcasses().len(), 1);
    }

    #[test]
    fn carcass_from_cascade_available_for_decomposition_same_tick() {
        use crate::event::EventKind;
        // Metabolic costs weaken the target so consumption kills it without
        // fully draining its biomass — the carcass retains pre-tick energy
        // minus consumption losses, leaving something for the scavenger.
        let params = WorldParameters {
            base_metabolic_rate: 90.0,
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.5,
            ..event_test_params()
        };
        let mut found = false;
        for seed in 0..20u64 {
            let mut world = World::new(params.clone(), event_test_dist(), seed);
            // Consumer: consumption_rate=15, kills target weakened by metabolism
            world.add_agent(Agent {
                id: 0,
                position: (0.0, 0.0),
                energy: 200.0,
                traits: TraitVector {
                    consumption_rate: 15.0,
                    ..zero_traits()
                },
            });
            // Target: energy=100, after metabolic(90) → 10, drained 10 → dead
            // Carcass energy = max(0, 100 - 10) = 90 (biomass minus consumed)
            world.add_agent(Agent {
                id: 0,
                position: (1.0, 0.0),
                energy: 100.0,
                traits: zero_traits(),
            });
            // Scavenger: decomposes the newly-created carcass
            world.add_agent(Agent {
                id: 0,
                position: (2.0, 0.0),
                energy: 200.0,
                traits: TraitVector {
                    scavenging_rate: 10.0,
                    ..zero_traits()
                },
            });
            world.step();
            let log = world.event_log();
            let consumed = log.by_kind(&EventKind::Consumed);
            let decomposed = log.by_kind(&EventKind::Decomposed);
            if !consumed.is_empty() && !decomposed.is_empty() {
                assert_eq!(
                    consumed[0].tick, decomposed[0].tick,
                    "both events should be in the same tick"
                );
                found = true;
                break;
            }
        }
        assert!(
            found,
            "at least one seed should produce same-tick cascade: consumption → carcass → decomposition"
        );
    }

    #[test]
    fn energy_acquisition_is_mutually_exclusive() {
        let params = WorldParameters {
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.5,
            ..event_test_params()
        };
        let mut world = World::new(params, event_test_dist(), 42);
        // Agent with both consumption and scavenging, plus a living target and a carcass
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
        world.add_agent(Agent {
            id: 0,
            position: (1.0, 0.0),
            energy: 50.0,
            traits: zero_traits(),
        });
        world.add_carcass(Carcass {
            id: 99,
            position: (0.0, 1.0),
            energy: 20.0,
        });
        world.step();
        use crate::event::EventKind;
        let consumed = world.event_log().by_kind(&EventKind::Consumed);
        let decomposed = world.event_log().by_kind(&EventKind::Decomposed);
        // Agent should have consumed OR decomposed, not both
        let agent_consumed = consumed.iter().any(|e| e.source == world.agents()[0].id);
        let agent_decomposed = decomposed.iter().any(|e| e.source == world.agents()[0].id);
        assert!(
            !(agent_consumed && agent_decomposed),
            "agent should not both consume and decompose in the same tick (mutually exclusive)"
        );
        // But at least one should have happened
        assert!(
            agent_consumed || agent_decomposed,
            "agent should have done at least one energy acquisition"
        );
    }

    #[test]
    fn consumer_navigates_toward_feeding_activity_via_projection_not_direct_sensing() {
        // Stigmergy: a consumer moves toward a zone where feeding happened recently,
        // even though no living agents are currently there to sense directly.
        // Run multiple seeds — projection-based movement should be biased toward the
        // activity zone, while random-only movement averages zero displacement.
        let extent = 100.0;
        let mut moved_toward_activity = 0;
        let trials = 50;

        for seed in 0..trials {
            let params = WorldParameters {
                solar_flux_magnitude: 0.0,
                base_metabolic_rate: 0.0,
                sensing_cost_coefficient: 0.0,
                movement_cost_coefficient: 0.0,
                world_extent: extent,
                initial_population_size: 0,
                spatial_decay_rate: 0.1,
                ..test_params()
            };
            let dist = InitialDistribution {
                mean_traits: zero_traits(),
                trait_covariance: 0.0,
                initial_cluster_count: 1,
                initial_energy_per_agent: 100.0,
            };
            let mut world = World::new(params, dist, seed);

            // Seed the event log with feeding activity at (20, 0)
            world.emit(
                event::EventKind::Consumed,
                99,
                Some(100),
                10.0,
                Some((20.0, 0.0)),
            );
            world.emit(
                event::EventKind::Consumed,
                99,
                Some(101),
                10.0,
                Some((20.0, 0.0)),
            );

            // Place a mobile consumer at the origin — no live agents to detect directly
            world.add_agent(Agent {
                id: 0,
                position: (0.0, 0.0),
                energy: 100.0,
                traits: TraitVector {
                    consumption_rate: 1.0,
                    mobility: 5.0,
                    chemotaxis_sensitivity: 10.0,
                    sensing_range: 30.0,
                    ..zero_traits()
                },
            });

            world.step();

            if world.agents()[0].position.0 > 0.0 {
                moved_toward_activity += 1;
            }
        }

        // With projection-based movement, the vast majority should move toward (20,0).
        // With random-only movement, ~50% would move in positive x.
        assert!(
            moved_toward_activity > 40,
            "consumer should be biased toward feeding activity zone: {moved_toward_activity}/{trials} moved toward it (expected >40)",
        );
    }

    #[test]
    fn scavenger_navigates_toward_carcass_activity_via_projection() {
        let extent = 100.0;
        let mut moved_toward_carcass = 0;
        let trials = 50;

        for seed in 0..trials {
            let params = WorldParameters {
                solar_flux_magnitude: 0.0,
                base_metabolic_rate: 0.0,
                sensing_cost_coefficient: 0.0,
                movement_cost_coefficient: 0.0,
                world_extent: extent,
                initial_population_size: 0,
                spatial_decay_rate: 0.1,
                ..test_params()
            };
            let dist = InitialDistribution {
                mean_traits: zero_traits(),
                trait_covariance: 0.0,
                initial_cluster_count: 1,
                initial_energy_per_agent: 100.0,
            };
            let mut world = World::new(params, dist, seed);

            // Carcass created at (20, 0) — logged but no actual carcass placed
            world.emit(
                event::EventKind::CarcassCreated,
                99,
                None,
                30.0,
                Some((20.0, 0.0)),
            );

            // Scavenger at origin should navigate toward carcass activity
            world.add_agent(Agent {
                id: 0,
                position: (0.0, 0.0),
                energy: 100.0,
                traits: TraitVector {
                    scavenging_rate: 1.0,
                    mobility: 5.0,
                    chemotaxis_sensitivity: 10.0,
                    sensing_range: 30.0,
                    ..zero_traits()
                },
            });

            world.step();

            if world.agents()[0].position.0 > 0.0 {
                moved_toward_carcass += 1;
            }
        }

        assert!(
            moved_toward_carcass > 40,
            "scavenger should be biased toward carcass activity: {moved_toward_carcass}/{trials}",
        );
    }

    #[test]
    fn broadcast_signal_strength_is_distance_weighted() {
        // When events occur during DES resolution, they produce broadcasts that
        // reach agents within sensing range. Signal strength is inversely proportional
        // to distance. We verify this via the public tick_broadcasts() accessor.
        let extent = 100.0;
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
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

        // Consumer at origin kills victim at (1,0)
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                consumption_rate: 100.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            id: 0,
            position: (1.0, 0.0),
            energy: 1.0,
            traits: zero_traits(),
        });
        // Observer A: close to the death (dist ~2 from death at (1,0))
        world.add_agent(Agent {
            id: 0,
            position: (3.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                sensing_range: 20.0,
                ..zero_traits()
            },
        });
        // Observer B: farther from the death (dist ~9 from death at (1,0))
        world.add_agent(Agent {
            id: 0,
            position: (10.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                sensing_range: 20.0,
                ..zero_traits()
            },
        });

        world.step();

        let broadcasts = world.tick_broadcasts();
        // Both observers should receive broadcasts
        let close_observer_id = world.agents()[1].id;  // after victim dies, observers shift
        let far_observer_id = world.agents()[2].id;

        let close_signals: Vec<_> = broadcasts.iter()
            .filter(|b| b.receiver_id == close_observer_id)
            .collect();
        let far_signals: Vec<_> = broadcasts.iter()
            .filter(|b| b.receiver_id == far_observer_id)
            .collect();

        assert!(!close_signals.is_empty(), "close observer should receive broadcasts");
        assert!(!far_signals.is_empty(), "far observer should receive broadcasts");

        // Close observer should receive stronger signal than far observer
        let close_strength: f32 = close_signals.iter().map(|b| b.strength).sum();
        let far_strength: f32 = far_signals.iter().map(|b| b.strength).sum();
        assert!(
            close_strength > far_strength,
            "closer observer should get stronger signal: close={close_strength}, far={far_strength}",
        );
    }

    #[test]
    fn mating_readiness_is_broadcast_ephemerally_not_logged() {
        // When an agent signals reproductive readiness, it should produce an
        // ephemeral broadcast (visible in tick_broadcasts) but NOT an event log
        // entry. Only the resolved outcome (MateSelected, Born) enters the log.
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
            sensing_range: 20.0,
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

        // Should have reproduced
        assert_eq!(world.agents().len(), 3, "two compatible agents should produce offspring");

        // MateSelected and Born events should be in the log
        let mate_selected = world.event_log().by_kind(&event::EventKind::MateSelected);
        let born = world.event_log().by_kind(&event::EventKind::Born);
        assert!(!mate_selected.is_empty(), "MateSelected should be logged");
        assert!(!born.is_empty(), "Born should be logged");

        // Mating readiness broadcasts should be in tick_broadcasts
        let readiness_broadcasts: Vec<_> = world.tick_broadcasts().iter()
            .filter(|b| b.kind == event::EventKind::MatingReadiness)
            .collect();
        assert!(
            !readiness_broadcasts.is_empty(),
            "mating readiness should be broadcast ephemerally",
        );

        // MatingReadiness should NOT appear in the event log
        let readiness_in_log = world.event_log().by_kind(&event::EventKind::MatingReadiness);
        assert!(
            readiness_in_log.is_empty(),
            "mating readiness should not be logged — only resolved outcomes",
        );
    }

    #[test]
    fn zero_sensing_agent_does_not_receive_broadcasts() {
        // An agent with sensing_range=0 should not receive any broadcasts,
        // even when events happen right next to it.
        let extent = 100.0;
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
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

        // Consumer kills victim
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                consumption_rate: 100.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            id: 0,
            position: (1.0, 0.0),
            energy: 1.0,
            traits: zero_traits(),
        });
        // Observer with zero sensing range — right next to the action
        world.add_agent(Agent {
            id: 0,
            position: (2.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                sensing_range: 0.0,
                ..zero_traits()
            },
        });

        world.step();

        let observer_id = world.agents()[1].id;
        let observer_broadcasts: Vec<_> = world.tick_broadcasts().iter()
            .filter(|b| b.receiver_id == observer_id)
            .collect();
        assert!(
            observer_broadcasts.is_empty(),
            "agent with sensing_range=0 should not receive broadcasts",
        );
    }

    #[test]
    fn receive_consumer_targets_closest_living_agent() {
        let agent = Agent {
            id: 1,
            position: (0.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                consumption_rate: 2.0,
                ..zero_traits()
            },
        };
        let trigger = event::Event {
            tick: 0, seq: 0, kind: event::EventKind::Consumed,
            source: 99, target: Some(100), energy_delta: 5.0,
            position: Some((1.0, 0.0)),
        };
        let data = ProjectionData {
            feeding_gradient: (0.0, 0.0),
            carcass_gradient: (0.0, 0.0),
            nearby_agents: vec![
                NearbyAgent { id: 10, distance: 4.0, energy: 50.0, traits: zero_traits() },
                NearbyAgent { id: 20, distance: 2.0, energy: 30.0, traits: zero_traits() },
            ],
            nearby_carcasses: vec![],
            contact_radius: 5.0,
            reproduction_energy_threshold: 50.0,
        };
        let response = agent.receive(&trigger, &data);
        assert_eq!(response.ack, event::Ack::Ack);
        assert_eq!(response.events.len(), 1);
        assert_eq!(response.events[0].kind, event::EventKind::Consumed);
        assert_eq!(response.events[0].source, 1);
        assert_eq!(response.events[0].target, Some(20));
        assert!((response.events[0].energy_delta - 2.0).abs() < 1e-5);
    }

    #[test]
    fn receive_agent_without_consumption_nacks_with_no_events() {
        let agent = Agent {
            id: 1,
            position: (0.0, 0.0),
            energy: 100.0,
            traits: zero_traits(),
        };
        let trigger = event::Event {
            tick: 0, seq: 0, kind: event::EventKind::Consumed,
            source: 99, target: Some(100), energy_delta: 5.0,
            position: Some((1.0, 0.0)),
        };
        let data = ProjectionData {
            feeding_gradient: (0.0, 0.0),
            carcass_gradient: (0.0, 0.0),
            nearby_agents: vec![
                NearbyAgent { id: 10, distance: 2.0, energy: 50.0, traits: zero_traits() },
            ],
            nearby_carcasses: vec![],
            contact_radius: 5.0,
            reproduction_energy_threshold: 50.0,
        };
        let response = agent.receive(&trigger, &data);
        assert_eq!(response.ack, event::Ack::Nack);
        assert!(response.events.is_empty());
    }

    #[test]
    fn receive_scavenger_targets_closest_carcass() {
        let agent = Agent {
            id: 1,
            position: (0.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                scavenging_rate: 4.0,
                ..zero_traits()
            },
        };
        let trigger = event::Event {
            tick: 0, seq: 0, kind: event::EventKind::CarcassCreated,
            source: 99, target: None, energy_delta: 20.0,
            position: Some((2.0, 0.0)),
        };
        let data = ProjectionData {
            feeding_gradient: (0.0, 0.0),
            carcass_gradient: (0.0, 0.0),
            nearby_agents: vec![],
            nearby_carcasses: vec![
                NearbyCarcass { id: 50, distance: 4.0, energy: 20.0 },
                NearbyCarcass { id: 51, distance: 1.5, energy: 6.0 },
            ],
            contact_radius: 5.0,
            reproduction_energy_threshold: 50.0,
        };
        let response = agent.receive(&trigger, &data);
        assert_eq!(response.ack, event::Ack::Ack);
        assert_eq!(response.events.len(), 1);
        assert_eq!(response.events[0].kind, event::EventKind::Decomposed);
        assert_eq!(response.events[0].source, 1);
        assert_eq!(response.events[0].target, Some(51));
        assert!((response.events[0].energy_delta - 4.0).abs() < 1e-5);
    }

    #[test]
    fn receive_mating_readiness_selects_compatible_mate() {
        let agent = Agent {
            id: 1,
            position: (0.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                mobility: 1.0,
                mate_selectivity: 5.0,
                reproductive_investment: 10.0,
                ..zero_traits()
            },
        };
        let trigger = event::Event {
            tick: 0, seq: 0, kind: event::EventKind::MatingReadiness,
            source: 2, target: None, energy_delta: 0.0,
            position: Some((1.0, 0.0)),
        };
        let data = ProjectionData {
            feeding_gradient: (0.0, 0.0),
            carcass_gradient: (0.0, 0.0),
            nearby_agents: vec![
                NearbyAgent {
                    id: 2, distance: 2.0, energy: 100.0,
                    traits: TraitVector {
                        mobility: 1.0,
                        mate_selectivity: 5.0,
                        reproductive_investment: 10.0,
                        ..zero_traits()
                    },
                },
            ],
            nearby_carcasses: vec![],
            contact_radius: 5.0,
            reproduction_energy_threshold: 50.0,
        };
        let response = agent.receive(&trigger, &data);
        assert_eq!(response.ack, event::Ack::Ack);
        assert_eq!(response.events.len(), 1);
        assert_eq!(response.events[0].kind, event::EventKind::MateSelected);
        assert_eq!(response.events[0].source, 1);
        assert_eq!(response.events[0].target, Some(2));
    }

    #[test]
    fn receive_mating_below_threshold_returns_no_events() {
        let agent = Agent {
            id: 1,
            position: (0.0, 0.0),
            energy: 30.0,
            traits: TraitVector {
                mate_selectivity: 5.0,
                reproductive_investment: 10.0,
                ..zero_traits()
            },
        };
        let trigger = event::Event {
            tick: 0, seq: 0, kind: event::EventKind::MatingReadiness,
            source: 2, target: None, energy_delta: 0.0,
            position: Some((1.0, 0.0)),
        };
        let data = ProjectionData {
            feeding_gradient: (0.0, 0.0),
            carcass_gradient: (0.0, 0.0),
            nearby_agents: vec![
                NearbyAgent {
                    id: 2, distance: 1.0, energy: 100.0,
                    traits: TraitVector {
                        mate_selectivity: 5.0,
                        reproductive_investment: 10.0,
                        ..zero_traits()
                    },
                },
            ],
            nearby_carcasses: vec![],
            contact_radius: 5.0,
            reproduction_energy_threshold: 50.0,
        };
        let response = agent.receive(&trigger, &data);
        assert_eq!(response.ack, event::Ack::Ack);
        assert!(response.events.is_empty());
    }

    #[test]
    fn receive_irrelevant_event_nacks_with_no_events() {
        let agent = Agent {
            id: 1,
            position: (0.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                consumption_rate: 5.0,
                scavenging_rate: 3.0,
                ..zero_traits()
            },
        };
        let trigger = event::Event {
            tick: 0, seq: 0, kind: event::EventKind::Born,
            source: 99, target: None, energy_delta: 50.0,
            position: Some((1.0, 0.0)),
        };
        let data = ProjectionData {
            feeding_gradient: (0.0, 0.0),
            carcass_gradient: (0.0, 0.0),
            nearby_agents: vec![
                NearbyAgent { id: 10, distance: 2.0, energy: 50.0, traits: zero_traits() },
            ],
            nearby_carcasses: vec![
                NearbyCarcass { id: 20, distance: 1.0, energy: 30.0 },
            ],
            contact_radius: 5.0,
            reproduction_energy_threshold: 50.0,
        };
        let response = agent.receive(&trigger, &data);
        assert_eq!(response.ack, event::Ack::Nack);
        assert!(response.events.is_empty());
    }

    #[test]
    fn step_delegates_consumption_to_receive() {
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
            id: 0, position: (0.0, 0.0), energy: 50.0,
            traits: TraitVector { consumption_rate: 2.0, ..zero_traits() },
        });
        world.add_agent(Agent {
            id: 0, position: (3.0, 0.0), energy: 50.0,
            traits: zero_traits(),
        });

        // Before step: construct what receive() would return
        let consumer = &world.agents()[0];
        let target = &world.agents()[1];
        let dist_between = toroidal_distance(consumer.position, target.position, 100.0);
        let data = ProjectionData {
            feeding_gradient: (0.0, 0.0), carcass_gradient: (0.0, 0.0),
            nearby_agents: vec![NearbyAgent {
                id: target.id, distance: dist_between,
                energy: target.energy, traits: target.traits,
            }],
            nearby_carcasses: vec![],
            contact_radius: 5.0,
            reproduction_energy_threshold: 50.0,
        };
        let trigger = event::Event {
            tick: 0, seq: 0, kind: event::EventKind::Consumed,
            source: 0, target: None, energy_delta: 0.0,
            position: Some(consumer.position),
        };
        let expected = consumer.receive(&trigger, &data);
        let expected_drain = expected.events[0].energy_delta;

        // Now run step() — should produce the same outcome
        world.step();

        assert_eq!(world.agents()[0].energy, 50.0 + expected_drain * 0.5);
        assert_eq!(world.agents()[1].energy, 50.0 - expected_drain);
    }

    #[test]
    fn agent_returns_chemotaxis_vector_on_moved_broadcast() {
        // A consumer with feeding gradient should return a Moved event
        // with chemotaxis direction based on gradient and traits
        let agent = Agent {
            id: 1,
            position: (0.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                consumption_rate: 1.0,
                chemotaxis_sensitivity: 2.0,
                mobility: 1.0,
                ..zero_traits()
            },
        };
        let data = ProjectionData {
            feeding_gradient: (3.0, 4.0),
            carcass_gradient: (0.0, 0.0),
            nearby_agents: vec![],
            nearby_carcasses: vec![],
            contact_radius: 5.0,
            reproduction_energy_threshold: 50.0,
        };
        let trigger = event::Event {
            tick: 0,
            seq: 0,
            kind: event::EventKind::Moved,
            source: 0,
            target: None,
            energy_delta: 0.0,
            position: None,
        };
        let response = agent.receive(&trigger, &data);
        assert_eq!(response.ack, event::Ack::Ack);
        assert_eq!(response.events.len(), 1);
        assert_eq!(response.events[0].kind, event::EventKind::Moved);
        // chemotaxis = sensitivity * (consumption_rate * feeding_gradient)
        // = 2.0 * (1.0 * (3.0, 4.0)) = (6.0, 8.0)
        let pos = response.events[0].position.unwrap();
        assert!((pos.0 - 6.0).abs() < 1e-5, "x={}", pos.0);
        assert!((pos.1 - 8.0).abs() < 1e-5, "y={}", pos.1);
    }

    #[test]
    fn zero_mobility_agent_returns_no_events_on_moved_broadcast() {
        let agent = Agent {
            id: 1,
            position: (0.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                consumption_rate: 1.0,
                chemotaxis_sensitivity: 2.0,
                mobility: 0.0,
                ..zero_traits()
            },
        };
        let data = ProjectionData {
            feeding_gradient: (3.0, 4.0),
            carcass_gradient: (0.0, 0.0),
            nearby_agents: vec![],
            nearby_carcasses: vec![],
            contact_radius: 5.0,
            reproduction_energy_threshold: 50.0,
        };
        let trigger = event::Event {
            tick: 0, seq: 0, kind: event::EventKind::Moved,
            source: 0, target: None, energy_delta: 0.0, position: None,
        };
        let response = agent.receive(&trigger, &data);
        assert!(response.events.is_empty(), "zero-mobility agent should not move");
    }

    #[test]
    fn scavenger_combines_carcass_gradient_into_chemotaxis() {
        let agent = Agent {
            id: 1,
            position: (0.0, 0.0),
            energy: 50.0,
            traits: TraitVector {
                consumption_rate: 0.5,
                scavenging_rate: 0.5,
                chemotaxis_sensitivity: 1.0,
                mobility: 1.0,
                ..zero_traits()
            },
        };
        let data = ProjectionData {
            feeding_gradient: (2.0, 0.0),
            carcass_gradient: (0.0, 3.0),
            nearby_agents: vec![],
            nearby_carcasses: vec![],
            contact_radius: 5.0,
            reproduction_energy_threshold: 50.0,
        };
        let trigger = event::Event {
            tick: 0, seq: 0, kind: event::EventKind::Moved,
            source: 0, target: None, energy_delta: 0.0, position: None,
        };
        let response = agent.receive(&trigger, &data);
        let pos = response.events[0].position.unwrap();
        // chemotaxis = 1.0 * (0.5 * (2.0, 0.0) + 0.5 * (0.0, 3.0)) = (1.0, 1.5)
        assert!((pos.0 - 1.0).abs() < 1e-5, "x={}", pos.0);
        assert!((pos.1 - 1.5).abs() < 1e-5, "y={}", pos.1);
    }

    #[test]
    fn movement_via_receive_matches_direct_query_approach() {
        // Regression test: movement through projection handoff must produce
        // identical results to the previous direct-query approach.
        // Two identical worlds run the same steps and must produce same positions.
        let params = WorldParameters {
            initial_population_size: 5,
            ..test_params()
        };
        let dist = test_distribution();
        let seed = 42;

        let mut world1 = World::new(params.clone(), dist.clone(), seed);
        let mut world2 = World::new(params, dist, seed);

        // Seed some events so projections have gradient data
        for w in [&mut world1, &mut world2] {
            w.emit(
                event::EventKind::Consumed, 99, Some(100), 20.0, Some((10.0, 10.0)),
            );
            w.emit(
                event::EventKind::CarcassCreated, 101, None, 15.0, Some((-10.0, -10.0)),
            );
        }

        for _ in 0..10 {
            world1.step();
            world2.step();
        }
        for (a, b) in world1.agents().iter().zip(world2.agents().iter()) {
            assert_eq!(a.position, b.position, "positions must match");
            assert_eq!(a.energy, b.energy, "energies must match");
        }
    }

    #[test]
    fn agent_that_nacked_consumed_does_not_receive_consumed_broadcasts() {
        // A non-consumer NACKs Consumed on tick 1.
        // On tick 2, a Consumed event occurs nearby — the NACKing agent
        // should NOT appear in tick_broadcasts for Consumed.
        let extent = 100.0;
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
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

        // Consumer at (0,0) — will trigger Consumed broadcasts
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                consumption_rate: 2.0,
                sensing_range: 20.0,
                ..zero_traits()
            },
        });
        // Prey at (2,0) — within contact radius
        world.add_agent(Agent {
            id: 0,
            position: (2.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                sensing_range: 20.0,
                ..zero_traits()
            },
        });
        // Non-consumer observer at (4,0) — within sensing range, will NACK Consumed
        world.add_agent(Agent {
            id: 0,
            position: (4.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                sensing_range: 20.0,
                ..zero_traits()
            },
        });

        // Tick 1: consumption happens, observer receives Consumed broadcast and NACKs it
        world.step();
        let observer_id = world.agents()[2].id;

        // Tick 2: another consumption — observer should NOT receive Consumed broadcast
        world.step();
        let consumed_broadcasts_to_observer = world.tick_broadcasts().iter()
            .filter(|b| b.kind == event::EventKind::Consumed && b.receiver_id == observer_id)
            .count();
        assert_eq!(
            consumed_broadcasts_to_observer, 0,
            "observer that NACKed Consumed should not receive Consumed broadcasts"
        );
    }

    #[test]
    fn agent_that_nacked_consumed_still_receives_other_event_types() {
        // A non-consumer NACKs Consumed but should still receive Died/CarcassCreated broadcasts.
        let extent = 100.0;
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
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

        // Consumer at (0,0) — will kill the prey
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                consumption_rate: 200.0, // high enough to kill in one tick
                sensing_range: 20.0,
                ..zero_traits()
            },
        });
        // Prey at (2,0) — within contact radius, low energy to ensure death
        world.add_agent(Agent {
            id: 0,
            position: (2.0, 0.0),
            energy: 10.0,
            traits: TraitVector {
                sensing_range: 20.0,
                ..zero_traits()
            },
        });
        // Observer at (4,0) — non-consumer, will NACK Consumed but should receive Died
        world.add_agent(Agent {
            id: 0,
            position: (4.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                sensing_range: 20.0,
                ..zero_traits()
            },
        });

        // Tick 1: consumption kills prey, observer NACKs Consumed but receives Died
        world.step();
        let observer_id = world.agents().last().unwrap().id;
        let died_broadcasts_to_observer = world.tick_broadcasts().iter()
            .filter(|b| b.kind == event::EventKind::Died && b.receiver_id == observer_id)
            .count();
        assert!(
            died_broadcasts_to_observer > 0,
            "observer should still receive Died broadcasts despite NACKing Consumed"
        );
    }

    #[test]
    fn offspring_of_nacking_agent_starts_fully_subscribed() {
        // Parent NACKs Consumed. Offspring should still receive Consumed broadcasts.
        let extent = 100.0;
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.0,
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
            reproduction_efficiency: 0.9,
            reproduction_energy_threshold: 10.0,
            mutation_rate: 0.0,
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

        // Two non-consumer parents that will reproduce — they will NACK Consumed
        let parent_traits = TraitVector {
            photosynthetic_absorption: 0.0,
            consumption_rate: 0.0,
            scavenging_rate: 0.0,
            mobility: 0.0,
            chemotaxis_sensitivity: 0.0,
            mate_selectivity: 10.0, // wide compatibility
            sensing_range: 20.0,
            reproductive_investment: 5.0,
        };
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            energy: 100.0,
            traits: parent_traits,
        });
        world.add_agent(Agent {
            id: 0,
            position: (1.0, 0.0),
            energy: 100.0,
            traits: parent_traits,
        });

        // Consumer nearby to trigger Consumed broadcasts
        world.add_agent(Agent {
            id: 0,
            position: (3.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                consumption_rate: 2.0,
                sensing_range: 20.0,
                ..zero_traits()
            },
        });
        // Prey for the consumer
        world.add_agent(Agent {
            id: 0,
            position: (4.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                sensing_range: 20.0,
                ..zero_traits()
            },
        });

        // Tick 1: parents reproduce (creating offspring), parents NACK Consumed
        world.step();
        let pre_offspring_count = world.agents().len();
        assert!(
            pre_offspring_count > 4,
            "expected offspring to be born, got {} agents",
            pre_offspring_count
        );

        // Find the offspring — it has the highest id
        let offspring_id = world.agents().iter().map(|a| a.id).max().unwrap();
        // Verify offspring has no NACK set
        assert!(
            !world.has_nacked(offspring_id, &event::EventKind::Consumed),
            "offspring should start with empty NACK set — no inheritance from parent"
        );
    }

    #[test]
    fn dead_agent_nack_state_is_cleaned_up() {
        // An agent NACKs an event type, then dies. Its NACK state should be removed.
        let extent = 100.0;
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 50.0, // high enough to kill quickly
            sensing_cost_coefficient: 0.0,
            movement_cost_coefficient: 0.0,
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
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

        // Consumer triggers Consumed broadcasts
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            energy: 1000.0, // enough to survive
            traits: TraitVector {
                consumption_rate: 2.0,
                sensing_range: 20.0,
                ..zero_traits()
            },
        });
        // Prey
        world.add_agent(Agent {
            id: 0,
            position: (2.0, 0.0),
            energy: 1000.0,
            traits: TraitVector {
                sensing_range: 20.0,
                ..zero_traits()
            },
        });
        // Non-consumer observer — will NACK Consumed, then die from metabolic cost
        world.add_agent(Agent {
            id: 0,
            position: (4.0, 0.0),
            energy: 60.0, // dies after ~1 tick at 50.0 metabolic rate
            traits: TraitVector {
                sensing_range: 20.0,
                ..zero_traits()
            },
        });

        // Record observer id before step
        let observer_id = world.agents()[2].id;

        // Tick 1: observer NACKs Consumed
        world.step();
        // Observer should have NACKed Consumed
        assert!(
            world.has_nacked(observer_id, &event::EventKind::Consumed),
            "observer should have NACKed Consumed after tick 1"
        );

        // Tick 2: observer dies from metabolic cost
        world.step();
        // NACK state should be cleaned up
        assert!(
            !world.has_nacked(observer_id, &event::EventKind::Consumed),
            "dead agent NACK state should be cleaned up"
        );
    }

    #[test]
    fn nack_filtering_is_deterministic_across_identical_runs() {
        // Two worlds with identical seeds produce identical results with NACK filtering.
        let params = WorldParameters {
            initial_population_size: 10,
            ..test_params()
        };
        let dist = test_distribution();
        let seed = 77;

        let mut world1 = World::new(params.clone(), dist.clone(), seed);
        let mut world2 = World::new(params, dist, seed);

        for _ in 0..20 {
            world1.step();
            world2.step();
        }

        assert_eq!(world1.agents().len(), world2.agents().len());
        for (a, b) in world1.agents().iter().zip(world2.agents().iter()) {
            assert_eq!(a.id, b.id, "agent IDs must match");
            assert_eq!(a.position, b.position, "positions must match");
            assert_eq!(a.energy, b.energy, "energies must match");
        }
    }

    #[test]
    fn consequence_events_resolve_before_next_agent_decision() {
        // When consumption kills a target, the Died and CarcassCreated consequence
        // events must resolve (via the priority queue) before any subsequent agent
        // takes their turn. Verify by checking that the event log shows
        // Consumed → Died → CarcassCreated in strict sequence order, with no
        // interleaving agent decision events between them.
        use crate::event::EventKind;
        let mut world = World::new(event_test_params(), event_test_dist(), 42);
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            energy: 100.0,
            traits: TraitVector {
                consumption_rate: 20.0,
                ..zero_traits()
            },
        });
        world.add_agent(Agent {
            id: 0,
            position: (1.0, 0.0),
            energy: 5.0,
            traits: zero_traits(),
        });
        world.step();
        let log = world.event_log();
        let events = log.by_tick_range(0, 1);
        // Find the Consumed, Died, and CarcassCreated events
        let consumed_seq = events.iter().find(|e| e.kind == EventKind::Consumed).unwrap().seq;
        let died_seq = events.iter().find(|e| e.kind == EventKind::Died).unwrap().seq;
        let carcass_seq = events.iter().find(|e| e.kind == EventKind::CarcassCreated).unwrap().seq;
        // Consequence events must be contiguous: Died immediately after Consumed,
        // CarcassCreated immediately after Died (no intervening events)
        assert_eq!(died_seq, consumed_seq + 1,
            "Died must immediately follow Consumed (no interleaving)");
        assert_eq!(carcass_seq, died_seq + 1,
            "CarcassCreated must immediately follow Died (no interleaving)");
    }

    #[test]
    fn queue_empty_at_end_of_tick_after_cascading() {
        // Run a scenario with cascading (consumption kills target) and verify
        // that the simulation completes without panicking (the debug_assert
        // inside step() checks the queue is empty).
        let params = WorldParameters {
            contact_radius: 5.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.5,
            base_metabolic_rate: 90.0,
            ..event_test_params()
        };
        // Run across many seeds to exercise different cascading paths
        for seed in 0..20u64 {
            let mut world = World::new(params.clone(), event_test_dist(), seed);
            world.add_agent(Agent {
                id: 0,
                position: (0.0, 0.0),
                energy: 200.0,
                traits: TraitVector {
                    consumption_rate: 15.0,
                    ..zero_traits()
                },
            });
            world.add_agent(Agent {
                id: 0,
                position: (1.0, 0.0),
                energy: 100.0,
                traits: zero_traits(),
            });
            world.add_agent(Agent {
                id: 0,
                position: (2.0, 0.0),
                energy: 200.0,
                traits: TraitVector {
                    scavenging_rate: 10.0,
                    ..zero_traits()
                },
            });
            // Should not panic from debug_assert!(consequence_queue.is_empty())
            world.step();
        }
    }
}
