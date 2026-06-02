//! Structure-of-Arrays (SoA) mirror of the agent store, plus vectorised
//! implementations of the provably-elementwise tick phases.
//!
//! Spike #354. The thesis: the *elementwise* phases — `metabolise`, `grow`,
//! `apply_wear` — are per-agent with no cross-agent reduction, so a clean SoA
//! loop over the same agent index order is **bit-identical to the scalar
//! stepper by construction** (no reordering of any floating-point operation).
//! That makes vectorisation a transparent optimisation rather than a second
//! source of truth.
//!
//! The store is *dynamically sized* — `push`/`swap_remove` on the columns — so
//! births and deaths work exactly as the AoS `Vec<Agent>` does. There is no
//! fixed-capacity arena and no `N_max` here; those belong to the deferred
//! follow-up (#354 out-of-scope).

use crate::event::{Event, EventKind};
use crate::{
    Agent, FUNCTIONAL_TRAIT_COUNT, FUNCTIONAL_TRAIT_INDICES, TraitVector, WorldParameters,
    stoichiometric_demand,
};

/// Structure-of-Arrays mirror of `Vec<Agent>`. Every `Agent` field becomes a
/// parallel column. Row `i` across all columns is agent `i`, in the same order
/// as the source slice — this ordering is what makes the SoA phases
/// bit-identical to the AoS phases.
#[derive(Clone, Debug, Default)]
pub struct AgentSoA {
    pub id: Vec<u64>,
    pub pos_x: Vec<f32>,
    pub pos_y: Vec<f32>,
    pub reserve: Vec<f32>,
    pub structure: Vec<f32>,
    pub peak_structure: Vec<f32>,
    pub nutrient: Vec<f32>,
    pub repro_reserve: Vec<f32>,
    pub repro_nutrient: Vec<f32>,
    pub contact_time: Vec<u64>,
    // Trait lanes, one column per trait dimension (TraitVector::NUM_DIMS).
    pub t_photo: Vec<f32>,
    pub t_hetero: Vec<f32>,
    pub t_mobility: Vec<f32>,
    pub t_kappa: Vec<f32>,
    pub t_fecundity: Vec<f32>,
    pub t_asexual: Vec<f32>,
    pub t_dispersal: Vec<f32>,
    // Per-functional-trait wear, one column per functional trait.
    pub wear: [Vec<f32>; FUNCTIONAL_TRAIT_COUNT],
}

impl AgentSoA {
    pub fn len(&self) -> usize {
        self.id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.id.is_empty()
    }

    /// Build an SoA store from an AoS slice, preserving order.
    pub fn from_agents(agents: &[Agent]) -> Self {
        let n = agents.len();
        let mut s = AgentSoA {
            id: Vec::with_capacity(n),
            pos_x: Vec::with_capacity(n),
            pos_y: Vec::with_capacity(n),
            reserve: Vec::with_capacity(n),
            structure: Vec::with_capacity(n),
            peak_structure: Vec::with_capacity(n),
            nutrient: Vec::with_capacity(n),
            repro_reserve: Vec::with_capacity(n),
            repro_nutrient: Vec::with_capacity(n),
            contact_time: Vec::with_capacity(n),
            t_photo: Vec::with_capacity(n),
            t_hetero: Vec::with_capacity(n),
            t_mobility: Vec::with_capacity(n),
            t_kappa: Vec::with_capacity(n),
            t_fecundity: Vec::with_capacity(n),
            t_asexual: Vec::with_capacity(n),
            t_dispersal: Vec::with_capacity(n),
            wear: [
                Vec::with_capacity(n),
                Vec::with_capacity(n),
                Vec::with_capacity(n),
            ],
        };
        for a in agents {
            s.push_agent(a);
        }
        s
    }

    /// Append one agent's columns. Mirrors `Vec::push`.
    pub fn push_agent(&mut self, a: &Agent) {
        self.id.push(a.id);
        self.pos_x.push(a.position.0);
        self.pos_y.push(a.position.1);
        self.reserve.push(a.reserve);
        self.structure.push(a.structure);
        self.peak_structure.push(a.peak_structure);
        self.nutrient.push(a.nutrient);
        self.repro_reserve.push(a.repro_reserve);
        self.repro_nutrient.push(a.repro_nutrient);
        self.contact_time.push(a.contact_time);
        self.t_photo.push(a.traits.photosynthetic_absorption);
        self.t_hetero.push(a.traits.heterotrophy);
        self.t_mobility.push(a.traits.mobility);
        self.t_kappa.push(a.traits.kappa);
        self.t_fecundity.push(a.traits.fecundity);
        self.t_asexual.push(a.traits.asexual_propensity);
        self.t_dispersal.push(a.traits.dispersal);
        for ft in 0..FUNCTIONAL_TRAIT_COUNT {
            self.wear[ft].push(a.wear[ft]);
        }
    }

    /// Remove row `i` by swapping the last row into its place. Mirrors
    /// `Vec::swap_remove`, including the O(1) reordering it implies.
    pub fn swap_remove(&mut self, i: usize) {
        self.id.swap_remove(i);
        self.pos_x.swap_remove(i);
        self.pos_y.swap_remove(i);
        self.reserve.swap_remove(i);
        self.structure.swap_remove(i);
        self.peak_structure.swap_remove(i);
        self.nutrient.swap_remove(i);
        self.repro_reserve.swap_remove(i);
        self.repro_nutrient.swap_remove(i);
        self.contact_time.swap_remove(i);
        self.t_photo.swap_remove(i);
        self.t_hetero.swap_remove(i);
        self.t_mobility.swap_remove(i);
        self.t_kappa.swap_remove(i);
        self.t_fecundity.swap_remove(i);
        self.t_asexual.swap_remove(i);
        self.t_dispersal.swap_remove(i);
        for ft in 0..FUNCTIONAL_TRAIT_COUNT {
            self.wear[ft].swap_remove(i);
        }
    }

    /// Reconstruct the `TraitVector` for row `i`.
    pub fn traits_at(&self, i: usize) -> TraitVector {
        TraitVector {
            photosynthetic_absorption: self.t_photo[i],
            heterotrophy: self.t_hetero[i],
            mobility: self.t_mobility[i],
            kappa: self.t_kappa[i],
            fecundity: self.t_fecundity[i],
            asexual_propensity: self.t_asexual[i],
            dispersal: self.t_dispersal[i],
        }
    }

    /// Reconstruct a full `Agent` from row `i`. Round-trips `from_agents`.
    pub fn agent_at(&self, i: usize) -> Agent {
        Agent {
            id: self.id[i],
            position: (self.pos_x[i], self.pos_y[i]),
            reserve: self.reserve[i],
            structure: self.structure[i],
            peak_structure: self.peak_structure[i],
            nutrient: self.nutrient[i],
            traits: self.traits_at(i),
            contact_time: self.contact_time[i],
            wear: [self.wear[0][i], self.wear[1][i], self.wear[2][i]],
            repro_reserve: self.repro_reserve[i],
            repro_nutrient: self.repro_nutrient[i],
        }
    }

    /// Materialise the whole store back into a `Vec<Agent>`.
    pub fn to_agents(&self) -> Vec<Agent> {
        (0..self.len()).map(|i| self.agent_at(i)).collect()
    }

    /// Per-row nominal trait for a functional-trait index (0..FUNCTIONAL_TRAIT_COUNT).
    fn nominal_functional(&self, ft: usize, i: usize) -> f32 {
        match FUNCTIONAL_TRAIT_INDICES[ft] {
            0 => self.t_photo[i],
            1 => self.t_hetero[i],
            2 => self.t_mobility[i],
            _ => unreachable!(),
        }
    }
}

/// Per-agent fixed maintenance cost. The single scalar expression of the cost,
/// shared by `metabolise_soa` and `grow_soa` so they cannot drift — and written
/// over column reads so the surrounding loop autovectorises.
#[inline]
fn maintenance_cost(s: &AgentSoA, i: usize, params: &WorldParameters, exp: f32) -> f32 {
    params.base_metabolic_rate
        + s.t_photo[i].powf(exp) * params.photo_maintenance_cost
        + s.t_hetero[i].powf(exp) * params.heterotrophy_maintenance_cost
        + s.t_mobility[i].powf(exp) * params.mobility_maintenance_cost
        + s.t_asexual[i].powf(exp) * params.asexual_propensity_maintenance_cost
        + s.structure[i] * params.structure_maintenance_coefficient
}

/// SoA `metabolise`: fixed costs only, per-agent. Bit-identical to
/// [`crate::phase::metabolise`] — same operations, same order.
pub fn metabolise_soa(s: &mut AgentSoA, params: &WorldParameters) -> (Vec<Event>, f32) {
    let mut events = Vec::with_capacity(s.len());
    let mut total_dissipated = 0.0_f32;
    let exp = params.maintenance_cost_exponent;

    for i in 0..s.len() {
        let cost = maintenance_cost(s, i, params, exp);
        s.reserve[i] -= cost;
        total_dissipated += cost;
        events.push(Event {
            tick: 0,
            seq: 0,
            kind: EventKind::Metabolized,
            source: s.id[i],
            target: None,
            energy_delta: cost,
            position: Some((s.pos_x[i], s.pos_y[i])),
            target_was_carcass: false,
        });
    }
    (events, total_dissipated)
}

/// SoA `grow`: surplus energy (reserve above metabolic retention) split by
/// kappa into soma (wear repair, then growth) and reproductive reserve.
/// Bit-identical to [`crate::phase::grow`] — the per-agent arithmetic and its
/// order are transcribed exactly.
pub fn grow_soa(s: &mut AgentSoA, params: &WorldParameters) -> (Vec<Event>, f32) {
    let mut events = Vec::new();
    let mut total_dissipated = 0.0_f32;
    let exp = params.maintenance_cost_exponent;

    for i in 0..s.len() {
        if s.reserve[i] <= 0.0 {
            continue;
        }
        let metabolic_cost = maintenance_cost(s, i, params, exp);
        let retention = metabolic_cost * params.growth_retention_multiplier;
        let surplus = (s.reserve[i] - retention).max(0.0);
        if surplus <= 0.0 {
            continue;
        }

        let kappa = s.t_kappa[i].clamp(0.0, 1.0);
        let soma_fraction = surplus * kappa;
        let repro_fraction = surplus - soma_fraction; // (1-kappa) * surplus

        s.reserve[i] -= surplus;

        // Repair gets priority from soma budget.
        let decay = params.repair_decay;
        let mut repair_energy_spent = 0.0_f32;
        if soma_fraction > 0.0 && decay > 0.0 {
            let base_repair = kappa;
            for ft in 0..FUNCTIONAL_TRAIT_COUNT {
                if s.wear[ft][i] <= 0.0 {
                    continue;
                }
                let effective_repair = base_repair * (-decay * s.wear[ft][i]).exp();
                let repair = effective_repair.min(s.wear[ft][i]);
                let cost = repair; // 1:1 energy-to-repair
                if repair_energy_spent + cost > soma_fraction {
                    let remaining = soma_fraction - repair_energy_spent;
                    let capped_repair = remaining.min(s.wear[ft][i]);
                    s.wear[ft][i] -= capped_repair;
                    repair_energy_spent = soma_fraction;
                    break;
                }
                s.wear[ft][i] -= repair;
                repair_energy_spent += cost;
            }
        }
        total_dissipated += repair_energy_spent;

        // Remainder of soma fraction → growth.
        let growth_budget = soma_fraction - repair_energy_spent;
        let efficiency = params.growth_efficiency;
        if efficiency > 0.0 && growth_budget > 0.0 {
            let energy_limited = growth_budget * efficiency;
            let traits = s.traits_at(i);
            let ratio = stoichiometric_demand(&traits, 1.0, params);
            let nutrient_limited = if ratio > 0.0 {
                (s.nutrient[i] / ratio).max(0.0)
            } else {
                f32::INFINITY
            };
            let to_structure = energy_limited.min(nutrient_limited);
            let energy_spent = to_structure / efficiency;
            let dissipated = energy_spent - to_structure;
            s.structure[i] += to_structure;
            // record_peak_structure
            if s.structure[i] > s.peak_structure[i] {
                s.peak_structure[i] = s.structure[i];
            }
            s.reserve[i] += growth_budget - energy_spent;
            s.nutrient[i] -= to_structure * ratio;
            total_dissipated += dissipated;

            if to_structure > 0.0 {
                events.push(Event {
                    tick: 0,
                    seq: 0,
                    kind: EventKind::Grew,
                    source: s.id[i],
                    target: None,
                    energy_delta: to_structure,
                    position: Some((s.pos_x[i], s.pos_y[i])),
                    target_was_carcass: false,
                });
            }
        } else if growth_budget > 0.0 {
            total_dissipated += growth_budget;
        }

        s.repro_reserve[i] += repro_fraction;
    }
    (events, total_dissipated)
}

/// SoA `apply_wear`: baseline + use-dependent wear accumulation per functional
/// trait. Bit-identical to [`crate::phase::apply_wear`]. The `usage` map is
/// keyed by agent id exactly as the scalar version.
pub fn apply_wear_soa(
    s: &mut AgentSoA,
    params: &WorldParameters,
    usage: &std::collections::HashMap<u64, [f32; FUNCTIONAL_TRAIT_COUNT]>,
) -> Vec<Event> {
    let mut events = Vec::new();
    let baseline_rate = params.wear_rate;
    let use_rate = params.use_wear_rate;

    for i in 0..s.len() {
        let mut total_wear_delta = 0.0_f32;

        let agent_usage = usage
            .get(&s.id[i])
            .copied()
            .unwrap_or([0.0; FUNCTIONAL_TRAIT_COUNT]);

        for ft in 0..FUNCTIONAL_TRAIT_COUNT {
            let nominal = s.nominal_functional(ft, i);
            let baseline = baseline_rate * nominal.max(0.0);
            let use_dependent = use_rate * agent_usage[ft].max(0.0);
            let accumulation = baseline + use_dependent;
            s.wear[ft][i] += accumulation;
            total_wear_delta += accumulation;
        }

        if total_wear_delta > 0.0 {
            events.push(Event {
                tick: 0,
                seq: 0,
                kind: EventKind::Wore,
                source: s.id[i],
                target: None,
                energy_delta: total_wear_delta,
                position: Some((s.pos_x[i], s.pos_y[i])),
                target_was_carcass: false,
            });
        }
    }
    events
}
