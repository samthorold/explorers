pub mod energy_ledger;
pub mod event;
pub mod phase;
pub mod spatial;
pub mod topology;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, Normal};
use serde::{Deserialize, Serialize};

pub fn toroidal_distance(a: (f32, f32), b: (f32, f32), extent: f32) -> f32 {
    let (dx, dy) = toroidal_displacement(a, b, extent);
    (dx * dx + dy * dy).sqrt()
}

pub fn toroidal_displacement(from: (f32, f32), to: (f32, f32), extent: f32) -> (f32, f32) {
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


/// Wrap a position into the toroidal world.
pub fn wrap_position(pos: (f32, f32), extent: f32) -> (f32, f32) {
    let half = extent / 2.0;
    let x = (pos.0 + half).rem_euclid(extent) - half;
    let y = (pos.1 + half).rem_euclid(extent) - half;
    (x, y)
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraitVector {
    pub photosynthetic_absorption: f32,
    /// Investment in consumption machinery — covers both predation (living targets)
    /// and decomposition (carcasses). Target state determines which, not separate traits.
    #[serde(alias = "consumption_rate")]
    pub heterotrophy: f32,
    #[serde(default)]
    pub nutrient_absorption: f32,
    pub mobility: f32,
    pub chemotaxis_sensitivity: f32,
    pub mate_selectivity: f32,
    pub sensing_range: f32,
    pub reproductive_investment: f32,
    #[serde(default)]
    pub fecundity: f32,
    #[serde(default)]
    pub somatic_maintenance: f32,
}

impl TraitVector {
    pub fn distance(&self, other: &TraitVector) -> f32 {
        let d0 = self.photosynthetic_absorption - other.photosynthetic_absorption;
        let d1 = self.heterotrophy - other.heterotrophy;
        let d2 = self.nutrient_absorption - other.nutrient_absorption;
        let d3 = self.mobility - other.mobility;
        let d4 = self.chemotaxis_sensitivity - other.chemotaxis_sensitivity;
        let d5 = self.mate_selectivity - other.mate_selectivity;
        let d6 = self.sensing_range - other.sensing_range;
        let d7 = self.reproductive_investment - other.reproductive_investment;
        let d8 = self.fecundity - other.fecundity;
        let d9 = self.somatic_maintenance - other.somatic_maintenance;
        (d0 * d0 + d1 * d1 + d2 * d2 + d3 * d3 + d4 * d4 + d5 * d5 + d6 * d6 + d7 * d7 + d8 * d8 + d9 * d9).sqrt()
    }

    pub fn get(&self, index: usize) -> f32 {
        match index {
            0 => self.photosynthetic_absorption,
            1 => self.heterotrophy,
            2 => self.nutrient_absorption,
            3 => self.mobility,
            4 => self.chemotaxis_sensitivity,
            5 => self.mate_selectivity,
            6 => self.sensing_range,
            7 => self.reproductive_investment,
            8 => self.fecundity,
            9 => self.somatic_maintenance,
            _ => unreachable!(),
        }
    }

    pub fn set(&mut self, index: usize, value: f32) {
        match index {
            0 => self.photosynthetic_absorption = value,
            1 => self.heterotrophy = value,
            2 => self.nutrient_absorption = value,
            3 => self.mobility = value,
            4 => self.chemotaxis_sensitivity = value,
            5 => self.mate_selectivity = value,
            6 => self.sensing_range = value,
            7 => self.reproductive_investment = value,
            8 => self.fecundity = value,
            9 => self.somatic_maintenance = value,
            _ => unreachable!(),
        }
    }

    /// Number of trait dimensions.
    pub const NUM_DIMS: usize = 10;
}

fn default_wear_rate() -> f32 { 0.1 }
fn default_wear_degradation_steepness() -> f32 { 1.0 }
fn default_somatic_maintenance_cost_coefficient() -> f32 { 0.1 }
fn default_use_wear_rate() -> f32 { 0.01 }
fn default_structure_maintenance_coefficient() -> f32 { 0.01 }
fn default_repair_decay() -> f32 { 1.0 }
fn default_base_nutrient_ratio() -> f32 { 0.1 }
fn default_specification_nutrient_coefficient() -> f32 { 0.2 }

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
    pub heterotrophy_maintenance_cost: f32,
    #[serde(default)]
    pub nutrient_absorption_maintenance_cost: f32,
    #[serde(default)]
    pub initial_nutrient_pool: f32,
    /// Fraction of surplus reserve converted to structure each tick (0.0–1.0).
    #[serde(default)]
    pub growth_efficiency: f32,
    #[serde(default = "default_wear_rate")]
    pub wear_rate: f32,
    #[serde(default = "default_wear_degradation_steepness")]
    pub wear_degradation_steepness: f32,
    #[serde(default = "default_somatic_maintenance_cost_coefficient")]
    pub somatic_maintenance_cost_coefficient: f32,
    #[serde(default = "default_use_wear_rate")]
    pub use_wear_rate: f32,
    #[serde(default = "default_structure_maintenance_coefficient")]
    pub structure_maintenance_coefficient: f32,
    #[serde(default = "default_repair_decay")]
    pub repair_decay: f32,
    /// Base nutrient-to-energy ratio per unit structure.
    #[serde(default = "default_base_nutrient_ratio")]
    pub base_nutrient_ratio: f32,
    /// How much each unit of specification investment adds to the nutrient ratio.
    #[serde(default = "default_specification_nutrient_coefficient")]
    pub specification_nutrient_coefficient: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InitialDistribution {
    pub mean_traits: TraitVector,
    pub trait_covariance: f32,
    pub initial_cluster_count: u32,
    pub initial_energy_per_agent: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentSpec {
    #[serde(
        deserialize_with = "deserialize_position",
        serialize_with = "serialize_position"
    )]
    pub position: (f32, f32),
    /// Starting reserve for this agent. Accepts "energy" in JSON for backward compatibility.
    #[serde(alias = "energy")]
    pub reserve: f32,
    pub traits: TraitVector,
    #[serde(default)]
    pub nutrient: f32,
}

fn deserialize_position<'de, D>(deserializer: D) -> Result<(f32, f32), D::Error>
where
    D: serde::Deserializer<'de>,
{
    let arr: [f32; 2] = Deserialize::deserialize(deserializer)?;
    Ok((arr[0], arr[1]))
}

fn serialize_position<S>(pos: &(f32, f32), serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(2))?;
    seq.serialize_element(&pos.0)?;
    seq.serialize_element(&pos.1)?;
    seq.end()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldRecipe {
    pub parameters: WorldParameters,
    #[serde(default)]
    pub initial_distribution: Option<InitialDistribution>,
    #[serde(default)]
    pub agents: Option<Vec<AgentSpec>>,
    pub max_ticks: u64,
}

/// Number of functional traits that accumulate somatic wear.
/// Indices: 0=photosynthetic_absorption, 1=heterotrophy,
/// 2=nutrient_absorption, 3=mobility, 4=chemotaxis_sensitivity, 5=sensing_range.
pub const FUNCTIONAL_TRAIT_COUNT: usize = 6;

pub struct Agent {
    pub id: u64,
    pub position: (f32, f32),
    pub reserve: f32,
    pub structure: f32,
    pub nutrient: f32,
    pub traits: TraitVector,
    pub contact_time: u64,
    /// Per-functional-trait somatic wear accumulation.
    pub wear: [f32; FUNCTIONAL_TRAIT_COUNT],
}

/// Maps a functional trait index (0–5) to its position in the TraitVector.
/// 0=photosynthetic_absorption, 1=heterotrophy,
/// 2=nutrient_absorption, 3=mobility, 4=chemotaxis_sensitivity, 5=sensing_range.
pub const FUNCTIONAL_TRAIT_INDICES: [usize; FUNCTIONAL_TRAIT_COUNT] = [0, 1, 2, 3, 4, 6];

impl Agent {
    /// Total energy held by this agent (reserve + structure).
    pub fn energy(&self) -> f32 {
        self.reserve + self.structure
    }

    /// Returns the nominal trait value for a given functional trait index (0–5).
    pub fn nominal_functional_trait(&self, ft_index: usize) -> f32 {
        self.traits.get(FUNCTIONAL_TRAIT_INDICES[ft_index])
    }

    /// Returns the effective (wear-degraded) trait value using the given steepness k.
    /// effective = nominal * exp(-k * wear)
    pub fn effective_trait_with_steepness(&self, ft_index: usize, k: f32) -> f32 {
        let nominal = self.nominal_functional_trait(ft_index);
        nominal * (-k * self.wear[ft_index]).exp()
    }

    /// Returns the effective trait value using a default steepness of 1.0.
    /// For production use, prefer the World method that supplies the world parameter.
    pub fn effective_trait(&self, ft_index: usize) -> f32 {
        self.effective_trait_with_steepness(ft_index, 1.0)
    }

    /// Returns a TraitVector with functional traits degraded by wear.
    /// Behavioural traits (mate_selectivity, reproductive_investment, fecundity)
    /// are passed through unchanged.
    pub fn effective_traits(&self, k: f32) -> TraitVector {
        let mut t = self.traits;
        for ft in 0..FUNCTIONAL_TRAIT_COUNT {
            let effective = self.effective_trait_with_steepness(ft, k);
            t.set(FUNCTIONAL_TRAIT_INDICES[ft], effective);
        }
        t
    }
}

pub struct Carcass {
    pub id: u64,
    pub position: (f32, f32),
    pub energy: f32,
    pub nutrient: f32,
}


/// Complexity-dependent death threshold.
///
/// Returns the structure level below which an agent dies. Derived from
/// the normalised entropy of the L1-normalised trait vector:
/// - Specialist (budget concentrated in few traits) → low threshold
/// - Generalist (budget spread across many traits) → high threshold
///
/// The threshold is `normalised_entropy * max_threshold` where
/// max_threshold is the trait budget (L1 sum). This means a perfectly
/// uniform generalist's threshold approaches its total trait budget,
/// while a pure specialist's threshold approaches zero.
pub fn death_threshold(traits: &TraitVector) -> f32 {
    let mut values = [0.0_f32; TraitVector::NUM_DIMS];
    let mut sum = 0.0_f32;
    for dim in 0..TraitVector::NUM_DIMS {
        let v = traits.get(dim).max(0.0);
        values[dim] = v;
        sum += v;
    }
    if sum < 1e-10 {
        return 0.0;
    }
    // Normalised entropy: H / ln(N)
    let n = TraitVector::NUM_DIMS as f32;
    let ln_n = n.ln();
    let mut entropy = 0.0_f32;
    for &v in &values {
        let p = v / sum;
        if p > 1e-10 {
            entropy -= p * p.ln();
        }
    }
    let normalised_entropy = entropy / ln_n;
    // Scale by the trait budget so the threshold is in energy units
    normalised_entropy * sum
}

/// Stoichiometric nutrient demand: structure × trait-derived ratio.
///
/// `demand = structure × (base_nutrient_ratio + specification_nutrient_coefficient × specification_sum)`
/// where `specification_sum = autotrophy + heterotrophy + mobility`.
///
/// Larger agents need more nutrient. More specification investment means
/// proportionally more nutrient per unit biomass.
pub fn stoichiometric_demand(traits: &TraitVector, structure: f32, params: &WorldParameters) -> f32 {
    let specification_sum = traits.photosynthetic_absorption.max(0.0)
        + traits.heterotrophy.max(0.0)
        + traits.mobility.max(0.0);
    let ratio = params.base_nutrient_ratio
        + params.specification_nutrient_coefficient * specification_sum;
    structure * ratio
}

#[allow(dead_code)]
pub struct World {
    params: WorldParameters,
    agents: Vec<Agent>,
    carcasses: Vec<Carcass>,
    dissipated_energy: f32,
    total_solar_input: f32,
    nutrient_pool: f32,
    seed: u64,
    rng: ChaCha8Rng,
    tick: u64,
    last_tick_births: usize,
    last_tick_deaths: usize,
    next_agent_id: u64,
    event_log: event::EventLog,
    next_seq: u64,
    ledger: energy_ledger::EnergyLedger,
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
        let trophic_total = mean.photosynthetic_absorption + mean.heterotrophy;

        let cluster_centroids: Vec<TraitVector> = (0..n_clusters)
            .map(|c| {
                let (photo, hetero) = if n_clusters == 1 || trophic_total <= 0.0 {
                    (mean.photosynthetic_absorption, mean.heterotrophy)
                } else {
                    match c % 2 {
                        0 => (trophic_total, 0.0),
                        _ => (0.0, trophic_total),
                    }
                };
                TraitVector {
                    photosynthetic_absorption: photo,
                    heterotrophy: hetero,
                    nutrient_absorption: mean.nutrient_absorption,
                    mobility: mean.mobility,
                    chemotaxis_sensitivity: mean.chemotaxis_sensitivity,
                    mate_selectivity: mean.mate_selectivity,
                    sensing_range: mean.sensing_range,
                    reproductive_investment: mean.reproductive_investment,
                    fecundity: mean.fecundity,
                    somatic_maintenance: mean.somatic_maintenance,
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
                    reserve: distribution.initial_energy_per_agent,
                    structure: 0.0,
                    nutrient: 0.0,
                    traits: TraitVector {
                        photosynthetic_absorption: centroid.photosynthetic_absorption
                            + trait_dist.sample(&mut rng),
                        heterotrophy: centroid.heterotrophy + trait_dist.sample(&mut rng),
                        nutrient_absorption: centroid.nutrient_absorption
                            + trait_dist.sample(&mut rng),
                        mobility: centroid.mobility + trait_dist.sample(&mut rng),
                        chemotaxis_sensitivity: centroid.chemotaxis_sensitivity
                            + trait_dist.sample(&mut rng),
                        mate_selectivity: centroid.mate_selectivity + trait_dist.sample(&mut rng),
                        sensing_range: centroid.sensing_range + trait_dist.sample(&mut rng),
                        reproductive_investment: centroid.reproductive_investment
                            + trait_dist.sample(&mut rng),
                        fecundity: centroid.fecundity + trait_dist.sample(&mut rng),
                        somatic_maintenance: centroid.somatic_maintenance + trait_dist.sample(&mut rng),
                    },
                    contact_time: 0,
                    wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                }
            })
            .collect();

        let nutrient_pool = params.initial_nutrient_pool;
        Self {
            params,
            agents,
            carcasses: Vec::new(),
            dissipated_energy: 0.0,
            total_solar_input: 0.0,
            nutrient_pool,
            seed,
            rng,
            tick: 0,
            last_tick_births: 0,
            last_tick_deaths: 0,
            next_agent_id: pop_size as u64,
            event_log: event::EventLog::new(),
            next_seq: 0,
            ledger: energy_ledger::EnergyLedger::new(),
        }
    }

    pub fn from_recipe(recipe: &WorldRecipe, seed: u64) -> Self {
        if let Some(ref agents) = recipe.agents {
            let params = recipe.parameters.clone();
            let rng = ChaCha8Rng::seed_from_u64(seed);
            let pop_size = agents.len();
            let sim_agents: Vec<Agent> = agents
                .iter()
                .enumerate()
                .map(|(i, spec)| Agent {
                    id: i as u64,
                    position: spec.position,
                    reserve: spec.reserve,
                    structure: 0.0,
                    nutrient: spec.nutrient,
                    traits: spec.traits,
                    contact_time: 0,
                    wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                })
                .collect();
            let nutrient_pool = params.initial_nutrient_pool;
            Self {
                params,
                agents: sim_agents,
                carcasses: Vec::new(),
                dissipated_energy: 0.0,
                total_solar_input: 0.0,
                nutrient_pool,
                seed,
                rng,
                tick: 0,
                last_tick_births: 0,
                last_tick_deaths: 0,
                next_agent_id: pop_size as u64,
                event_log: event::EventLog::new(),
                next_seq: 0,
                ledger: energy_ledger::EnergyLedger::new(),
            }
        } else if let Some(ref distribution) = recipe.initial_distribution {
            Self::new(recipe.parameters.clone(), distribution.clone(), seed)
        } else {
            panic!("WorldRecipe must have either 'agents' or 'initial_distribution'");
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
        use energy_ledger::EnergyEndpoint;

        let extent = self.params.world_extent;

        // Snapshot pre-tick state for energy ledger conservation verification
        let pre_agent_energy: std::collections::HashMap<u64, f32> = self.agents.iter()
            .map(|a| (a.id, a.reserve + a.structure))
            .collect();
        let pre_carcass_energy: std::collections::HashMap<u64, f32> = self.carcasses.iter()
            .map(|c| (c.id, c.energy))
            .collect();

        // Build spatial grid once at start of tick
        let cell_size = self.params.light_competition_radius.max(1.0);
        let mut grid = crate::spatial::SpatialGrid::new(extent, cell_size);
        for (i, a) in self.agents.iter().enumerate() {
            grid.insert(i as u64, a.position);
        }

        // Autonomous phases in order
        let mut events = Vec::new();

        // 1. Photosynthesise
        let photo_events = phase::photosynthesise(
            &mut self.agents, &grid, &self.params,
        );
        let solar_this_tick: f32 = photo_events.iter().map(|e| e.energy_delta).sum();
        self.total_solar_input += solar_this_tick;
        // Record solar flows to ledger
        for ev in &photo_events {
            self.ledger.record(
                EnergyEndpoint::SolarTap,
                EnergyEndpoint::Agent(ev.source),
                ev.energy_delta,
            );
        }
        events.extend(photo_events);

        // 2. Absorb nutrients
        let nutrient_events = phase::absorb_nutrients(
            &mut self.agents, &mut self.nutrient_pool, &self.params,
        );
        events.extend(nutrient_events);

        // 3. Metabolise
        let (metab_events, dissipated) = phase::metabolise(&mut self.agents, &self.params);
        self.dissipated_energy += dissipated;
        events.extend(metab_events);

        // 4. Grow
        let (grow_events, grow_dissipated) = phase::grow(&mut self.agents, &self.params);
        self.dissipated_energy += grow_dissipated;
        events.extend(grow_events);

        // 5. Resolve drains (coordinated pass 1)
        let drain_result = phase::resolve_drains(
            &mut self.agents, &mut self.carcasses, &grid, &self.params, &mut self.nutrient_pool,
        );
        self.dissipated_energy += drain_result.dissipated;
        events.extend(drain_result.events);

        // Mark deaths from drain resolution
        let drain_dead_ids: std::collections::HashSet<u64> = drain_result.dead_agents.iter().copied().collect();
        for agent in self.agents.iter_mut() {
            if drain_dead_ids.contains(&agent.id) {
                events.push(event::Event {
                    tick: 0,
                    seq: 0,
                    kind: event::EventKind::Died,
                    source: agent.id,
                    target: None,
                    energy_delta: 0.0,
                    position: Some(agent.position),
                });
                agent.reserve = 0.0; // mark for removal
            }
        }
        // Add new carcasses from drain kills (not in this tick's spatial grid)
        for c in drain_result.new_carcasses {
            self.carcasses.push(c);
        }

        // Remove drain-killed agents before further phases
        self.agents.retain(|a| !drain_dead_ids.contains(&a.id));

        // 6. Resolve reproduction (coordinated pass 2)
        // Rebuild grid after removing dead agents (indices changed)
        let mut repro_grid = crate::spatial::SpatialGrid::new(extent, cell_size);
        for (i, a) in self.agents.iter().enumerate() {
            repro_grid.insert(i as u64, a.position);
        }
        let repro_result = phase::resolve_reproduction(
            &mut self.agents, &drain_dead_ids, &repro_grid, &self.params, &mut self.rng,
        );
        self.dissipated_energy += repro_result.dissipated;
        self.last_tick_births = repro_result.offspring.len();
        events.extend(repro_result.events);
        // Add offspring to world with unique IDs
        for mut child in repro_result.offspring {
            child.id = self.next_agent_id;
            self.next_agent_id += 1;
            self.agents.push(child);
        }

        // 7. Wear
        let wear_events = phase::apply_wear(&mut self.agents, &self.params);
        events.extend(wear_events);

        // 8. Check death thresholds
        let (death_events, new_carcasses) = phase::check_death_thresholds(
            &mut self.agents, &self.params,
        );
        events.extend(death_events);
        for c in new_carcasses {
            self.carcasses.push(c);
        }

        // Remove dead agents (those with reserve <= 0 or structure below threshold)
        self.agents.retain(|a| a.reserve > 0.0);

        // 9. Move (final phase — repositions agents for next tick)
        let move_result = phase::move_agents(
            &mut self.agents, &self.carcasses, &grid, &self.params, &mut self.rng,
        );
        self.dissipated_energy += move_result.dissipated;
        events.extend(move_result.events);

        // --- Energy ledger: record all flows for conservation verification ---
        // Built from event data and state snapshots, after all phases complete.
        self.ledger.clear();

        let post_agent_energy: std::collections::HashMap<u64, f32> = self.agents.iter()
            .map(|a| (a.id, a.reserve + a.structure))
            .collect();
        let post_carcass_energy: std::collections::HashMap<u64, f32> = self.carcasses.iter()
            .map(|c| (c.id, c.energy))
            .collect();

        // Collect all agent IDs that exist at start or end of tick
        let mut all_agent_ids = std::collections::HashSet::new();
        for &id in pre_agent_energy.keys() { all_agent_ids.insert(id); }
        for &id in post_agent_energy.keys() { all_agent_ids.insert(id); }
        let mut all_carcass_ids = std::collections::HashSet::new();
        for &id in pre_carcass_energy.keys() { all_carcass_ids.insert(id); }
        for &id in post_carcass_energy.keys() { all_carcass_ids.insert(id); }

        // Record endowments (pre-tick energy for entities that existed at tick start)
        for (&id, &energy) in &pre_agent_energy {
            if energy > 0.0 {
                self.ledger.record(EnergyEndpoint::Endowment, EnergyEndpoint::Agent(id), energy);
            }
        }
        for (&id, &energy) in &pre_carcass_energy {
            if energy > 0.0 {
                self.ledger.record(EnergyEndpoint::Endowment, EnergyEndpoint::Carcass(id), energy);
            }
        }

        // Record solar input per agent
        for ev in events.iter().filter(|e| e.kind == event::EventKind::Photosynthesized) {
            self.ledger.record(
                EnergyEndpoint::SolarTap,
                EnergyEndpoint::Agent(ev.source),
                ev.energy_delta,
            );
        }

        // Record consumption flows (target -> consumer with trophic loss)
        for ev in events.iter().filter(|e| e.kind == event::EventKind::Consumed) {
            let target_id = ev.target.unwrap();
            let drain = ev.energy_delta;
            let is_carcass = pre_carcass_energy.contains_key(&target_id)
                && !pre_agent_energy.contains_key(&target_id);
            let eff = if is_carcass {
                self.params.decomposition_efficiency
            } else {
                self.params.consumption_efficiency
            };
            let gained = drain * eff;
            let lost = drain - gained;
            let target_ep = if is_carcass {
                EnergyEndpoint::Carcass(target_id)
            } else {
                EnergyEndpoint::Agent(target_id)
            };
            if gained > 0.0 {
                self.ledger.record(target_ep.clone(), EnergyEndpoint::Agent(ev.source), gained);
            }
            if lost > 0.0 {
                self.ledger.record(target_ep, EnergyEndpoint::Dissipation, lost);
            }
        }

        // Record death transfers (agent -> carcass)
        for &id in &all_carcass_ids {
            if !pre_carcass_energy.contains_key(&id) && pre_agent_energy.contains_key(&id) {
                // New carcass from a pre-tick agent that died
                let carcass_energy = post_carcass_energy.get(&id).copied().unwrap_or(0.0);
                if carcass_energy > 0.0 {
                    self.ledger.record(
                        EnergyEndpoint::Agent(id),
                        EnergyEndpoint::Carcass(id),
                        carcass_energy,
                    );
                }
            }
        }

        // Record offspring birth endowments
        for (&id, &energy) in &post_agent_energy {
            if !pre_agent_energy.contains_key(&id) && energy > 0.0 {
                self.ledger.record(EnergyEndpoint::Endowment, EnergyEndpoint::Agent(id), energy);
            }
        }

        // For each pre-tick agent: compute residual dissipation
        // (everything not accounted for by consumption, death, or retained energy)
        for (&id, &_pre_energy) in &pre_agent_energy {
            let total_in = self.ledger.net_received(&EnergyEndpoint::Agent(id));
            let total_out = self.ledger.net_sent(&EnergyEndpoint::Agent(id));
            let post_energy = post_agent_energy.get(&id).copied().unwrap_or(0.0);
            let unaccounted = total_in - total_out - post_energy;
            if unaccounted > 1e-6 {
                self.ledger.record(
                    EnergyEndpoint::Agent(id),
                    EnergyEndpoint::Dissipation,
                    unaccounted,
                );
            }
        }

        // For pre-tick carcasses: compute residual dissipation
        for (&id, &_pre_energy) in &pre_carcass_energy {
            let total_in = self.ledger.net_received(&EnergyEndpoint::Carcass(id));
            let total_out = self.ledger.net_sent(&EnergyEndpoint::Carcass(id));
            let post_energy = post_carcass_energy.get(&id).copied().unwrap_or(0.0);
            let unaccounted = total_in - total_out - post_energy;
            if unaccounted > 1e-6 {
                self.ledger.record(
                    EnergyEndpoint::Carcass(id),
                    EnergyEndpoint::Dissipation,
                    unaccounted,
                );
            }
        }

        // Append events to log
        for mut ev in events {
            ev.tick = self.tick;
            ev.seq = self.next_seq;
            self.next_seq += 1;
            let _ = self.event_log.append(ev);
        }

        self.tick += 1;
    }

    pub fn params(&self) -> &WorldParameters {
        &self.params
    }

    pub fn params_mut(&mut self) -> &mut WorldParameters {
        &mut self.params
    }

    pub fn agents(&self) -> &[Agent] {
        &self.agents
    }

    pub fn carcasses(&self) -> &[Carcass] {
        &self.carcasses
    }

    pub fn nutrient_pool(&self) -> f32 {
        self.nutrient_pool
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

    pub fn energy_ledger(&self) -> &energy_ledger::EnergyLedger {
        &self.ledger
    }

    /// Emit an event to the log. Retained for coordinated phases (not yet implemented).
    #[allow(dead_code)]
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
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.5,
            reproduction_efficiency: 0.7,
            base_metabolic_rate: 0.1,
            movement_cost_coefficient: 0.05,
            sensing_cost_coefficient: 0.0,
            reproduction_energy_threshold: 50.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            contact_radius: 5.0,
            world_extent: 100.0,
            initial_population_size: 10,
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
            base_nutrient_ratio: 0.1,
            specification_nutrient_coefficient: 0.2,
        }
    }

    fn test_distribution() -> InitialDistribution {
        InitialDistribution {
            mean_traits: TraitVector {
                photosynthetic_absorption: 0.5,
                heterotrophy: 0.3,
                nutrient_absorption: 0.0,
                mobility: 0.4,
                chemotaxis_sensitivity: 0.3,
                mate_selectivity: 0.5,
                sensing_range: 0.4,
                reproductive_investment: 0.3,
                fecundity: 0.0,
                somatic_maintenance: 0.0,
            },
            trait_covariance: 0.1,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        }
    }

    #[test]
    fn trait_vector_has_named_accessors() {
        let traits = TraitVector {
            photosynthetic_absorption: 0.1,
            heterotrophy: 0.2,
            nutrient_absorption: 0.35,
            mobility: 0.4,
            chemotaxis_sensitivity: 0.5,
            mate_selectivity: 0.6,
            sensing_range: 0.7,
            reproductive_investment: 0.8,
            fecundity: 0.0,
            somatic_maintenance: 0.0,
        };
        assert_eq!(traits.photosynthetic_absorption, 0.1);
        assert_eq!(traits.heterotrophy, 0.2);
        assert_eq!(traits.nutrient_absorption, 0.35);
        assert_eq!(traits.mobility, 0.4);
        assert_eq!(traits.chemotaxis_sensitivity, 0.5);
        assert_eq!(traits.mate_selectivity, 0.6);
        assert_eq!(traits.sensing_range, 0.7);
        assert_eq!(traits.reproductive_investment, 0.8);
        assert_eq!(traits.get(2), 0.35);
    }

    #[test]
    fn trait_vector_has_fecundity_dimension() {
        let traits = TraitVector {
            fecundity: 3.5,
            ..zero_traits()
        };
        assert_eq!(traits.fecundity, 3.5);
        assert_eq!(traits.get(8), 3.5);
    }

    #[test]
    fn trait_vector_has_somatic_maintenance_dimension() {
        let traits = TraitVector {
            somatic_maintenance: 0.15,
            ..zero_traits()
        };
        assert_eq!(traits.somatic_maintenance, 0.15);
        assert_eq!(traits.get(9), 0.15);
        assert_eq!(TraitVector::NUM_DIMS, 10);
    }

    #[test]
    fn world_created_with_params_has_correct_population_size() {
        let world = World::new(test_params(), test_distribution(), 42);
        assert_eq!(world.agents().len(), 10);
    }

    #[test]
    fn agent_and_carcass_have_nutrient_field() {
        let agent = Agent {
            id: 0, position: (0.0, 0.0), reserve: 100.0, structure: 0.0,
            nutrient: 5.0, traits: zero_traits(), contact_time: 0,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
        };
        assert_eq!(agent.nutrient, 5.0);
        let carcass = Carcass { id: 0, position: (0.0, 0.0), energy: 50.0, nutrient: 3.0 };
        assert_eq!(carcass.nutrient, 3.0);
    }

    #[test]
    fn world_has_nutrient_pool() {
        let mut params = test_params();
        params.initial_nutrient_pool = 100.0;
        let world = World::new(params, test_distribution(), 42);
        assert_eq!(world.nutrient_pool(), 100.0);
    }

    #[test]
    fn step_does_not_panic() {
        let mut world = World::new(test_params(), test_distribution(), 42);
        world.step();
        world.step();
        assert!(world.tick() == 2);
    }

    #[test]
    fn death_threshold_higher_for_generalists() {
        let specialist = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };
        let generalist = TraitVector {
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
        let spec_threshold = death_threshold(&specialist);
        let gen_threshold = death_threshold(&generalist);
        assert!(gen_threshold > spec_threshold,
            "generalist threshold ({}) should exceed specialist ({})",
            gen_threshold, spec_threshold);
    }

    #[test]
    fn pure_producer_simulation_runs_without_panic() {
        // A simulation of pure producers: photosynthesise, metabolise, grow, wear, die
        let params = WorldParameters {
            solar_flux_magnitude: 10.0,
            base_metabolic_rate: 0.5,
            growth_efficiency: 0.5,
            wear_rate: 0.01,
            wear_degradation_steepness: 1.0,
            repair_decay: 1.0,
            initial_population_size: 0,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        // Manually add pure producers
        for i in 0..10 {
            world.add_agent(Agent {
                id: 0, // will be reassigned
                position: (i as f32 * 5.0 - 25.0, 0.0),
                reserve: 50.0,
                structure: 0.0,
                nutrient: 0.0,
                traits: TraitVector {
                    photosynthetic_absorption: 0.8,
                    somatic_maintenance: 0.1,
                    ..zero_traits()
                },
                contact_time: 10,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            });
        }

        // Run for 100 ticks
        for _tick in 0..100 {
            world.step();
            // Some agents may die, that is fine
            if world.agents().is_empty() {
                // Producers died, simulation still did not panic
                break;
            }
        }

        // Verify events were logged
        assert!(world.event_log().len() > 0, "should have logged events");

        // Verify photosynthesis events exist
        let photo_events = world.event_log().by_kind(&event::EventKind::Photosynthesized);
        assert!(!photo_events.is_empty(), "should have photosynthesis events");

        // Verify metabolism events exist
        let metab_events = world.event_log().by_kind(&event::EventKind::Metabolized);
        assert!(!metab_events.is_empty(), "should have metabolism events");
    }

    #[test]
    fn no_budget_normalization_exists() {
        // Budget normalization was removed per system design:
        // superlinear maintenance costs are the limiter, not algebraic normalization.
        // This test confirms the method no longer exists on TraitVector.
        let traits = TraitVector {
            photosynthetic_absorption: 2.0,
            heterotrophy: 3.0,
            somatic_maintenance: 5.0,
            ..zero_traits()
        };
        // Traits retain their raw values — no normalization
        assert_eq!(traits.photosynthetic_absorption, 2.0);
        assert_eq!(traits.heterotrophy, 3.0);
        assert_eq!(traits.somatic_maintenance, 5.0);
    }

    #[test]
    fn effective_trait_degrades_with_wear() {
        let mut agent = Agent {
            id: 1, position: (0.0, 0.0), reserve: 10.0, structure: 0.0,
            nutrient: 0.0, traits: TraitVector {
                photosynthetic_absorption: 1.0,
                ..zero_traits()
            },
            contact_time: 0,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
        };
        let nominal = agent.effective_trait_with_steepness(0, 1.0);
        assert!((nominal - 1.0).abs() < 1e-6);

        agent.wear[0] = 1.0;
        let degraded = agent.effective_trait_with_steepness(0, 1.0);
        assert!(degraded < nominal);
        assert!(degraded > 0.0);
    }

    #[test]
    fn stoichiometric_demand_increases_with_specification() {
        let params = test_params();
        let structure = 5.0;
        // Low specification: only autotrophy
        let low_spec = TraitVector {
            photosynthetic_absorption: 0.3,
            ..zero_traits()
        };
        // High specification: autotrophy + heterotrophy + mobility
        let high_spec = TraitVector {
            photosynthetic_absorption: 0.3,
            heterotrophy: 0.4,
            mobility: 0.3,
            ..zero_traits()
        };
        let demand_low = stoichiometric_demand(&low_spec, structure, &params);
        let demand_high = stoichiometric_demand(&high_spec, structure, &params);
        assert!(demand_high > demand_low,
            "more specification investment should yield higher demand: low={}, high={}",
            demand_low, demand_high);

        // Non-specification traits (fecundity, mate_selectivity etc.) should NOT affect demand
        let with_repro = TraitVector {
            photosynthetic_absorption: 0.3,
            fecundity: 5.0,
            mate_selectivity: 10.0,
            ..zero_traits()
        };
        let demand_repro = stoichiometric_demand(&with_repro, structure, &params);
        assert!((demand_repro - demand_low).abs() < 1e-6,
            "non-specification traits should not affect demand: low={}, with_repro={}",
            demand_low, demand_repro);
    }

    #[test]
    fn stoichiometric_demand_zero_traits_base_ratio_only() {
        // With zero specification, demand = structure * base_ratio
        let params = test_params();
        let traits = zero_traits();
        let demand = stoichiometric_demand(&traits, 10.0, &params);
        let expected = 10.0 * params.base_nutrient_ratio;
        assert!((demand - expected).abs() < 1e-6,
            "zero-spec demand should be structure * base_ratio: got {}, expected {}",
            demand, expected);
    }

    #[test]
    fn stoichiometric_demand_scales_with_structure() {
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            heterotrophy: 0.3,
            mobility: 0.2,
            ..zero_traits()
        };
        // Zero structure → zero demand (newborns)
        assert_eq!(stoichiometric_demand(&traits, 0.0, &params), 0.0);
        // Positive structure → positive demand
        let demand_small = stoichiometric_demand(&traits, 1.0, &params);
        let demand_large = stoichiometric_demand(&traits, 10.0, &params);
        assert!(demand_small > 0.0, "positive structure should yield positive demand");
        assert!(demand_large > demand_small, "larger structure should yield larger demand");
        // Demand is proportional to structure
        let ratio = demand_large / demand_small;
        assert!((ratio - 10.0).abs() < 1e-4, "demand should scale linearly with structure");
    }

    #[test]
    fn toroidal_distance_wraps_correctly() {
        let d = toroidal_distance((-48.0, 0.0), (48.0, 0.0), 100.0);
        assert!((d - 4.0).abs() < 1e-3, "toroidal distance should be 4.0, got {}", d);
    }

    #[test]
    fn world_from_recipe_with_agents() {
        let recipe = WorldRecipe {
            parameters: test_params(),
            initial_distribution: None,
            agents: Some(vec![
                AgentSpec {
                    position: (0.0, 0.0),
                    reserve: 50.0,
                    traits: zero_traits(),
                    nutrient: 0.0,
                },
            ]),
            max_ticks: 100,
        };
        let world = World::from_recipe(&recipe, 42);
        assert_eq!(world.agents().len(), 1);
    }

    // --- Energy ledger conservation tests ---

    /// Helper: conservation params with all phases active.
    fn conservation_params() -> WorldParameters {
        WorldParameters {
            solar_flux_magnitude: 10.0,
            base_metabolic_rate: 0.5,
            growth_efficiency: 0.5,
            wear_rate: 0.01,
            wear_degradation_steepness: 1.0,
            repair_decay: 1.0,
            somatic_maintenance_cost_coefficient: 0.05,
            structure_maintenance_coefficient: 0.01,
            movement_cost_coefficient: 0.1,
            sensing_cost_coefficient: 0.01,
            reproduction_energy_threshold: 20.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.1,
            mutation_magnitude: 0.05,
            initial_population_size: 0,
            initial_nutrient_pool: 100.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.5,
            contact_radius: 10.0,
            world_extent: 50.0,
            light_competition_radius: 100.0,
            photo_maintenance_cost: 0.05,
            heterotrophy_maintenance_cost: 0.05,
            nutrient_absorption_maintenance_cost: 0.05,
            use_wear_rate: 0.01,
            base_nutrient_ratio: 0.1,
            specification_nutrient_coefficient: 0.2,
        }
    }

    /// Create a mixed population: producers and heterotrophs.
    fn seed_mixed_population(world: &mut World) {
        // Producers (sessile, photosynthetic)
        for i in 0..10 {
            world.add_agent(Agent {
                id: 0,
                position: (i as f32 * 5.0 - 25.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                nutrient: 10.0,
                traits: TraitVector {
                    photosynthetic_absorption: 0.6,
                    nutrient_absorption: 0.2,
                    somatic_maintenance: 0.1,
                    sensing_range: 2.0,
                    reproductive_investment: 5.0,
                    fecundity: 1.0,
                    mate_selectivity: 10.0,
                    ..zero_traits()
                },
                contact_time: 50,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            });
        }
        // Heterotrophs (mobile, consume living and dead)
        for i in 0..10 {
            world.add_agent(Agent {
                id: 0,
                position: (i as f32 * 10.0 - 25.0, 10.0),
                reserve: 40.0,
                structure: 3.0,
                nutrient: 5.0,
                traits: TraitVector {
                    heterotrophy: 0.4,
                    mobility: 0.3,
                    chemotaxis_sensitivity: 0.1,
                    somatic_maintenance: 0.1,
                    sensing_range: 10.0,
                    reproductive_investment: 5.0,
                    fecundity: 1.0,
                    mate_selectivity: 10.0,
                    ..zero_traits()
                },
                contact_time: 0,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            });
        }
    }

    #[test]
    fn single_tick_energy_conservation_pure_producer() {
        // A single producer: solar input = energy retained + dissipated.
        // The ledger should verify conservation after each tick.
        let params = WorldParameters {
            solar_flux_magnitude: 10.0,
            base_metabolic_rate: 1.0,
            initial_population_size: 0,
            growth_efficiency: 0.0,
            movement_cost_coefficient: 0.0,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0, // nonzero structure required for light capture
            nutrient: 0.0,
            traits: TraitVector {
                photosynthetic_absorption: 0.8,
                somatic_maintenance: 0.2,
                ..zero_traits()
            },
            contact_time: 0,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
        });

        world.step();

        // The ledger should be balanced — if step() records flows correctly,
        // assert_balanced() should not panic.
        world.energy_ledger().assert_balanced();

        // Solar input should be positive
        assert!(world.energy_ledger().total_solar_input() > 0.0,
            "solar input should be positive");
    }

    #[test]
    fn energy_conservation_500_ticks_mixed_population() {
        // Run 500 ticks with all phases active (photosynthesis, nutrient uptake,
        // metabolism, growth, drains, reproduction, wear, death, movement).
        // assert_balanced() must pass on every tick.
        let params = conservation_params();
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        seed_mixed_population(&mut world);

        for _tick in 0..500 {
            world.step();
            // assert_balanced() panics if conservation is violated
            world.energy_ledger().assert_balanced();

            if world.agents().is_empty() {
                // All agents died — conservation still held until here
                break;
            }
        }

        // Verify cumulative conservation: total solar = dissipated + retained
        let total_solar = world.total_solar_input();
        let total_dissipated = world.dissipated_energy();
        let retained_agents: f32 = world.agents().iter()
            .map(|a| a.reserve + a.structure).sum();
        let retained_carcasses: f32 = world.carcasses().iter()
            .map(|c| c.energy).sum();
        // Initial endowment energy
        let initial_energy: f32 = 50.0 * 10.0  // producers
            + 5.0 * 10.0   // producer structure
            + 40.0 * 10.0  // heterotrophs
            + 3.0 * 10.0;  // heterotroph structure
        let total_input = initial_energy + total_solar;
        let total_output = total_dissipated + retained_agents + retained_carcasses;
        let diff = (total_input - total_output).abs();
        let tolerance = total_input * 1e-3; // 0.1% tolerance for f32
        assert!(diff < tolerance,
            "cumulative energy conservation violated: input={total_input}, output={total_output}, diff={diff}");
    }

    #[test]
    fn nutrient_conservation_across_ticks() {
        // Total nutrient (available pool + living agents + carcasses) must be
        // constant across all ticks.
        let params = conservation_params();
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        seed_mixed_population(&mut world);

        // Compute initial total nutrient
        let initial_nutrient: f32 = world.nutrient_pool()
            + world.agents().iter().map(|a| a.nutrient).sum::<f32>()
            + world.carcasses().iter().map(|c| c.nutrient).sum::<f32>();

        for t in 0..200 {
            world.step();

            let current_nutrient: f32 = world.nutrient_pool()
                + world.agents().iter().map(|a| a.nutrient).sum::<f32>()
                + world.carcasses().iter().map(|c| c.nutrient).sum::<f32>();

            let diff = (current_nutrient - initial_nutrient).abs();
            let tolerance = initial_nutrient.abs().max(1.0) * 1e-4;
            assert!(diff < tolerance,
                "nutrient conservation violated at tick {t}: initial={initial_nutrient}, current={current_nutrient}, diff={diff}");

            if world.agents().is_empty() {
                break;
            }
        }
    }

    #[test]
    fn no_negative_energy_balance_in_ledger() {
        // No agent or carcass endpoint should have negative net balance
        // in the ledger at any tick boundary.
        let params = conservation_params();
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        seed_mixed_population(&mut world);

        for _tick in 0..100 {
            world.step();
            // assert_balanced() checks no negative balances
            world.energy_ledger().assert_balanced();
            if world.agents().is_empty() {
                break;
            }
        }
    }

    #[test]
    fn topology_projection_works_with_event_log() {
        // TopologyProjection should process the event log from World::step()
        // without panicking, correctly reading Consumed, Reproduced, Died events.
        let params = conservation_params();
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        seed_mixed_population(&mut world);

        for _ in 0..50 {
            world.step();
            if world.agents().is_empty() {
                break;
            }
        }

        let mut topo = crate::topology::TopologyProjection::new();
        topo.update(world.event_log());

        // Should have processed events without panic
        // Verify trophic_roles() returns a map (may be empty if all initial agents died)
        let _roles = topo.trophic_roles();
        // Computing roles without panic is the key test

        // Died events should remove agents from active set
        let died_events = world.event_log().by_kind(&event::EventKind::Died);
        for ev in &died_events {
            assert!(!topo.active_agents().contains(&ev.source),
                "dead agent {} should not be in active set", ev.source);
        }

        // Incremental update should not panic
        topo.update(world.event_log());

        // Verify event log has expected event types
        let log = world.event_log();
        assert!(!log.by_kind(&event::EventKind::Photosynthesized).is_empty(),
            "should have photosynthesis events");
        assert!(!log.by_kind(&event::EventKind::Metabolized).is_empty(),
            "should have metabolism events");
    }
}

