//! A grazer's hunger at the moment it grazes (issue #606).
//!
//! Need-gated consumption (#600, #603) scales a consumer's drain by its
//! expression `1 / (1 + c·s)`, where `s` is its satiation: the lesser of its
//! reserve and the energy its free nutrient can match, in ticks of its own
//! maintenance. Recognition (#604) then withholds `r·w(d)` of capability from
//! a living target that resembles the consumer. So whether a kill was
//! *hunger-driven* is a question about the grazer's satiation in the drain
//! pass, and #605 found neither instrument could read it.
//!
//! The drain pass reads each consumer's state after the tick's first four
//! phases (photosynthesise, absorb nutrients, metabolise, grow). [`PreStep`]
//! captures the roster and nutrient grid before a step and replays those
//! phases on a copy with the stepper's own public phase functions, so the
//! state it returns is the state the drain pass read, exactly (pinned in the
//! tests against the stepper's own drains). [`Satiation`] reads both
//! currencies from it, and [`GrazerHunger`] tallies (grazer, victim) pairs
//! by kinship, trait distance and satiation.
//!
//! #621 redefined satiation as **surplus** above the grow phase's retention
//! buffer, read after metabolism and before growth, and #623 made the stepper
//! read it. [`PreStep::metabolised_agents`] replays the first three phases
//! and [`Surplus`] reads it there; its expression is the stepper's gate
//! (pinned in the tests). #622 measured its distribution
//! ([`SurplusDistribution`]) to set the default sensitivity. [`Satiation`]
//! is the previous, reserve-based reading (#600–#619), which the
//! [`GrazerHunger`] tallies still band by.
//!
//! Observer-side only: the world is never touched.

use explorers_sim::phase;
use explorers_sim::spatial::{NutrientGrid, SpatialGrid};
use explorers_sim::{Agent, TraitVector, World, WorldParameters};

/// The roster and nutrient grid before a step: enough to replay the tick's
/// phases up to the drain pass.
#[derive(Clone)]
pub struct PreStep {
    agents: Vec<Agent>,
    nutrient_grid: NutrientGrid,
}

impl PreStep {
    /// Capture `world` before it steps.
    pub fn capture(world: &World) -> Self {
        PreStep {
            agents: world.agents().to_vec(),
            nutrient_grid: world.nutrient_grid().clone(),
        }
    }

    /// The roster as the grow phase reads it: the tick's first three phases
    /// (photosynthesise, absorb nutrients, metabolise) replayed, in the
    /// stepper's order, on a copy of the pre-step state. Where [`Surplus`] is
    /// read (#622).
    pub fn metabolised_agents(&self, params: &WorldParameters) -> Vec<Agent> {
        let mut agents = self.agents.clone();
        let mut nutrient_grid = self.nutrient_grid.clone();
        let cell_size = params.light_competition_radius.max(1.0);
        let mut grid = SpatialGrid::new(params.world_extent, cell_size);
        for (i, a) in agents.iter().enumerate() {
            grid.insert(i as u64, a.position);
        }
        phase::photosynthesise(&mut agents, &grid, params);
        phase::absorb_nutrients(&mut agents, &mut nutrient_grid, params);
        phase::metabolise(&mut agents, params);
        agents
    }

    /// The roster as the drain pass read it: [`PreStep::metabolised_agents`]
    /// grown — the tick's first four phases.
    pub fn drain_time_agents(&self, params: &WorldParameters) -> Vec<Agent> {
        let mut agents = self.metabolised_agents(params);
        phase::grow(&mut agents, params);
        agents
    }
}

/// An agent's maintenance need: its per-tick metabolic cost, floored at zero.
/// Computed here rather than through the stepper so the reads work on a tree
/// without the need gate (pinned against `phase::metabolic_cost` in the tests).
fn maintenance_need(agent: &Agent, params: &WorldParameters) -> f32 {
    let x = params.maintenance_cost_exponent;
    let t = &agent.traits;
    (params.base_metabolic_rate
        + t.photosynthetic_absorption.powf(x) * params.photo_maintenance_cost
        + t.heterotrophy.powf(x) * params.heterotrophy_maintenance_cost
        + t.mobility.powf(x) * params.mobility_maintenance_cost
        + t.asexual_propensity.powf(x) * params.asexual_propensity_maintenance_cost
        + agent.structure * params.structure_maintenance_coefficient)
        .max(0.0)
}

/// The reserve energy an agent's free nutrient can match when built into
/// structure, `N / (growth_efficiency × demand ratio)`: infinite when growth
/// binds no nutrient.
fn nutrient_matched_energy(agent: &Agent, params: &WorldParameters) -> f32 {
    let binding =
        params.growth_efficiency * explorers_sim::stoichiometric_demand(&agent.traits, 1.0, params);
    if binding > 0.0 {
        agent.nutrient.max(0.0) / binding
    } else {
        f32::INFINITY
    }
}

/// An agent's **surplus satiation** (world rules, *Capability and expression
/// are decoupled*; settled in #621): the reserve above the grow phase's
/// retention buffer, co-limited by the energy its free nutrient can match, in
/// ticks of its own maintenance. Read after metabolism and before growth
/// ([`PreStep::metabolised_agents`]): the surplus the tick's grow phase is
/// about to mobilise. Only the energy side is shifted by the buffer, so
/// `energy` is negative below it.
///
/// Computed here as well as by the stepper (#623) so the read works on the
/// replayed roster and splits the two sides; [`Surplus::expression`] is
/// pinned against `phase::consumption_expression` in the tests.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surplus {
    /// `(reserve − buffer) / m`, with `buffer = growth_retention_multiplier × m`.
    pub energy: f32,
    /// `N / (growth_efficiency × ratio) / m`.
    pub nutrient: f32,
}

impl Surplus {
    pub fn of(agent: &Agent, params: &WorldParameters) -> Self {
        let need = maintenance_need(agent, params);
        let buffer = params.growth_retention_multiplier * need;
        let ticks = |e: f32| if need > 0.0 { e / need } else { f32::INFINITY };
        Surplus {
            energy: ticks(agent.reserve - buffer),
            nutrient: ticks(nutrient_matched_energy(agent, params)),
        }
    }

    /// `s = max(0, min(energy, nutrient))`: the surplus the need gate
    /// reads.
    pub fn ticks(&self) -> f32 {
        self.energy.min(self.nutrient).max(0.0)
    }

    /// The need-gated expression `1 / (1 + c·s)` at this surplus: the
    /// stepper's gate (#623; pinned against `phase::consumption_expression`
    /// in the tests).
    pub fn expression(&self, params: &WorldParameters) -> f32 {
        let c = params.satiation_sensitivity;
        let s = self.ticks();
        if c <= 0.0 || s <= 0.0 {
            return 1.0;
        }
        1.0 / (1.0 + c * s)
    }
}

/// Lower edge of the [`SurplusDistribution`]'s log bins, in ticks of
/// maintenance; positive surplus below it is binned together.
pub const SURPLUS_LOW: f64 = 1e-3;
/// Log bins per decade.
pub const SURPLUS_BINS_PER_DECADE: usize = 20;
/// Decades the log bins span from [`SURPLUS_LOW`] (to 1e5 ticks); surplus at
/// or above the top edge (an infinite one included) is binned together.
pub const SURPLUS_DECADES: usize = 8;
const SURPLUS_LOG_BINS: usize = SURPLUS_BINS_PER_DECADE * SURPLUS_DECADES;

/// A distribution of [`Surplus::ticks`] over agent-samples: agents at
/// exactly zero (at or below the buffer, or without free nutrient) counted
/// apart, the rest in fine log bins, so percentiles read within a bin's
/// width (~12 %). `bins` is `[below SURPLUS_LOW, log bins…, at or above the
/// top]`, empty until something positive is recorded.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SurplusDistribution {
    pub zero: u64,
    pub bins: Vec<u64>,
    /// Samples recorded with [`SurplusDistribution::record_surplus`] whose
    /// nutrient side was the lesser.
    #[serde(default)]
    pub nutrient_limited: u64,
    /// Of the zeros, those held at zero by nutrient (no free nutrient) rather
    /// than by the buffer.
    #[serde(default)]
    pub zero_nutrient_limited: u64,
}

impl SurplusDistribution {
    /// Record a surplus read, noting whether nutrient bound it.
    pub fn record_surplus(&mut self, surplus: &Surplus) {
        let s = surplus.ticks();
        let nutrient = surplus.nutrient < surplus.energy;
        self.nutrient_limited += u64::from(nutrient);
        self.zero_nutrient_limited += u64::from(nutrient && s <= 0.0);
        self.record(s);
    }

    pub fn record(&mut self, s: f32) {
        if s <= 0.0 || s.is_nan() {
            self.zero += 1;
            return;
        }
        if self.bins.is_empty() {
            self.bins = vec![0; SURPLUS_LOG_BINS + 2];
        }
        let s = s as f64;
        let i = if s < SURPLUS_LOW {
            0
        } else {
            let k = ((s / SURPLUS_LOW).log10() * SURPLUS_BINS_PER_DECADE as f64).floor();
            (k as usize).min(SURPLUS_LOG_BINS) + 1
        };
        self.bins[i] += 1;
    }

    pub fn count(&self) -> u64 {
        self.zero + self.bins.iter().sum::<u64>()
    }

    /// Share of samples at zero surplus; `None` when empty.
    pub fn zero_share(&self) -> Option<f64> {
        let n = self.count();
        (n > 0).then(|| self.zero as f64 / n as f64)
    }

    /// The `q` quantile (`0 ≤ q ≤ 1`), interpolated geometrically within its
    /// log bin; `None` when empty. The bottom bin interpolates linearly from
    /// zero, and the top bin reads as its lower edge.
    pub fn percentile(&self, q: f64) -> Option<f64> {
        let n = self.count();
        if n == 0 {
            return None;
        }
        let mut rank = q.clamp(0.0, 1.0) * n as f64;
        if rank < self.zero as f64 {
            return Some(0.0);
        }
        rank -= self.zero as f64;
        let step = 10f64.powf(1.0 / SURPLUS_BINS_PER_DECADE as f64);
        let last = self.bins.len().saturating_sub(1);
        for (i, &c) in self.bins.iter().enumerate() {
            if c == 0 {
                continue;
            }
            if rank < c as f64 || i == last {
                let frac = (rank / c as f64).min(1.0);
                return Some(match i {
                    0 => SURPLUS_LOW * frac,
                    i if i == last => SURPLUS_LOW * step.powi(SURPLUS_LOG_BINS as i32),
                    i => SURPLUS_LOW * step.powf((i - 1) as f64 + frac),
                });
            }
            rank -= c as f64;
        }
        // Only reachable at q = 1 with an empty top bin: the last occupied
        // bin's upper edge.
        let top = self.bins.iter().rposition(|&c| c > 0)?;
        Some(SURPLUS_LOW * step.powi(top as i32))
    }

    /// The `q` quantile of the positive samples alone (the zeros left out);
    /// `None` when none is positive. The default-sensitivity criterion reads
    /// this: `c = 1 / s₂₅` over light-fed mixotrophs with surplus (#622).
    pub fn positive_percentile(&self, q: f64) -> Option<f64> {
        if self.bins.iter().all(|&c| c == 0) {
            return None;
        }
        SurplusDistribution {
            zero: 0,
            bins: self.bins.clone(),
            ..Default::default()
        }
        .percentile(q)
    }

    pub fn merge(&mut self, other: &SurplusDistribution) {
        self.zero += other.zero;
        self.nutrient_limited += other.nutrient_limited;
        self.zero_nutrient_limited += other.zero_nutrient_limited;
        if other.bins.is_empty() {
            return;
        }
        if self.bins.is_empty() {
            self.bins = vec![0; other.bins.len()];
        }
        for (a, b) in self.bins.iter_mut().zip(&other.bins) {
            *a += b;
        }
    }
}

/// An agent's satiation in both currencies, in ticks of its own maintenance
/// (its per-tick metabolic cost): `energy` is its reserve, `nutrient` the
/// energy its free nutrient can match when built into structure
/// (`N / (growth_efficiency × demand ratio)`), infinite when growth binds no
/// nutrient. The need gate read the lesser ([`Satiation::ticks`]) on the
/// whole reserve, after growth, until #623 moved it to [`Surplus`]; kept for
/// the #606 grazer-hunger tallies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Satiation {
    pub energy: f32,
    pub nutrient: f32,
}

impl Satiation {
    pub fn of(agent: &Agent, params: &WorldParameters) -> Self {
        let need = maintenance_need(agent, params);
        let ticks = |e: f32| if need > 0.0 { e / need } else { f32::INFINITY };
        Satiation {
            energy: ticks(agent.reserve.max(0.0)),
            nutrient: ticks(nutrient_matched_energy(agent, params)),
        }
    }

    /// The co-limited satiation the pre-#623 need gate read: the scarcer currency.
    pub fn ticks(&self) -> f32 {
        self.energy.min(self.nutrient)
    }

    /// Whether nutrient, not energy, is the scarcer currency.
    pub fn nutrient_limited(&self) -> bool {
        self.nutrient < self.energy
    }

    /// The pre-#623 need-gated expression `1 / (1 + c·s)` at this satiation.
    pub fn expression(&self, params: &WorldParameters) -> f32 {
        let c = params.satiation_sensitivity;
        let s = self.ticks();
        if c <= 0.0 || s <= 0.0 {
            return 1.0;
        }
        1.0 / (1.0 + c * s)
    }
}

/// Upper edges of the trait-distance bands (on `TraitVector::distance`, the
/// recognition metric): `[0, 0.1)`, `[0.1, 0.25)`, `[0.25, 0.5)`,
/// `[0.5, 1)`, `≥ 1`. At the default recognition distance 0.5 the first three
/// are inside it, where resemblance `w` is above 0.92, 0.56 and 0.
pub const DISTANCE_EDGES: [f32; 4] = [0.1, 0.25, 0.5, 1.0];
pub const DISTANCE_BANDS: usize = DISTANCE_EDGES.len() + 1;
/// The bands inside the default recognition distance.
pub const RECOGNITION_BANDS: usize = 3;

/// Upper edges of the satiation bands, in ticks of maintenance:
/// `[0, 1)`, `[1, 5)`, `[5, 10)`, `[10, 20)`, `[20, 50)`, `≥ 50`. At the
/// pre-#623 default `satiation_sensitivity = 0.1`, 10 ticks was half
/// expression, from which recognition spares an identical target entirely.
pub const SATIATION_EDGES: [f32; 5] = [1.0, 5.0, 10.0, 20.0, 50.0];
pub const SATIATION_BANDS: usize = SATIATION_EDGES.len() + 1;
/// The bands under half expression (at the pre-#623 default): hungry.
pub const HUNGRY_BANDS: usize = 3;

fn band(value: f32, edges: &[f32]) -> usize {
    edges.iter().take_while(|&&e| value >= e).count()
}

/// Band index of a trait distance.
pub fn distance_band(d: f32) -> usize {
    band(d, &DISTANCE_EDGES)
}

/// (grazer, victim) pairs tallied by kinship, the pair's trait distance and
/// the grazer's drain-time satiation. Index `[kin]` is `0` for non-kin and
/// `1` for kin.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GrazerHunger {
    /// `[kin][distance band][satiation band]`.
    pub pairs: [[[u64; SATIATION_BANDS]; DISTANCE_BANDS]; 2],
    /// Of those, pairs whose grazer's scarcer currency was nutrient:
    /// `[kin][satiation band]`.
    pub nutrient_limited: [[u64; SATIATION_BANDS]; 2],
}

impl GrazerHunger {
    pub fn record(&mut self, kin: bool, distance: f32, satiation: Satiation) {
        let k = usize::from(kin);
        let s = band(satiation.ticks(), &SATIATION_EDGES);
        self.pairs[k][distance_band(distance)][s] += 1;
        if satiation.nutrient_limited() {
            self.nutrient_limited[k][s] += 1;
        }
    }

    pub fn merge(&mut self, other: &GrazerHunger) {
        for k in 0..2 {
            for d in 0..DISTANCE_BANDS {
                for s in 0..SATIATION_BANDS {
                    self.pairs[k][d][s] += other.pairs[k][d][s];
                }
            }
            for s in 0..SATIATION_BANDS {
                self.nutrient_limited[k][s] += other.nutrient_limited[k][s];
            }
        }
    }

    /// Pairs of this kinship.
    pub fn total(&self, kin: bool) -> u64 {
        self.pairs[usize::from(kin)].iter().flatten().sum()
    }

    /// Pairs of this kinship whose grazer was hungry (under half expression).
    pub fn hungry(&self, kin: bool) -> u64 {
        self.pairs[usize::from(kin)]
            .iter()
            .map(|row| row[..HUNGRY_BANDS].iter().sum::<u64>())
            .sum()
    }

    /// Pairs of this kinship in the first `bands` distance bands.
    pub fn within(&self, kin: bool, bands: usize) -> u64 {
        self.pairs[usize::from(kin)][..bands].iter().flatten().sum()
    }

    /// Pairs of this kinship by satiation band, summed over distance.
    pub fn by_satiation(&self, kin: bool) -> [u64; SATIATION_BANDS] {
        let mut out = [0; SATIATION_BANDS];
        for row in &self.pairs[usize::from(kin)] {
            for (o, v) in out.iter_mut().zip(row) {
                *o += v;
            }
        }
        out
    }

    /// Pairs of this kinship by distance band, summed over satiation.
    pub fn by_distance(&self, kin: bool) -> [u64; DISTANCE_BANDS] {
        let mut out = [0; DISTANCE_BANDS];
        for (o, row) in out.iter_mut().zip(&self.pairs[usize::from(kin)]) {
            *o = row.iter().sum();
        }
        out
    }
}

/// Trait distances tallied in the [`DISTANCE_EDGES`] bands, with their mean:
/// the parent–offspring distance a census reads off `Born` events.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TraitDistances {
    pub bands: [u64; DISTANCE_BANDS],
    pub sum: f64,
    pub sum_sq: f64,
}

impl TraitDistances {
    pub fn record(&mut self, a: &TraitVector, b: &TraitVector) {
        let d = a.distance(b);
        self.bands[distance_band(d)] += 1;
        self.sum += d as f64;
        self.sum_sq += (d as f64) * (d as f64);
    }

    pub fn count(&self) -> u64 {
        self.bands.iter().sum()
    }

    pub fn mean(&self) -> Option<f64> {
        let n = self.count();
        (n > 0).then(|| self.sum / n as f64)
    }

    /// Root mean square distance: `σ·√(7·p)` for an asexual birth at
    /// mutation magnitude `σ` and rate `p`, away from the trait floors.
    pub fn rms(&self) -> Option<f64> {
        let n = self.count();
        (n > 0).then(|| (self.sum_sq / n as f64).sqrt())
    }

    pub fn merge(&mut self, other: &TraitDistances) {
        for (a, b) in self.bands.iter_mut().zip(other.bands) {
            *a += b;
        }
        self.sum += other.sum;
        self.sum_sq += other.sum_sq;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_source::{ConfigSource, resolve_config, sampled_units};
    use crate::search::default_ranges;
    use explorers_sim::event::EventKind;
    use std::collections::HashMap;

    fn sample_31_world(seed: u64) -> World {
        let (params, dist) = resolve_config(
            ConfigSource::SAMPLE,
            31,
            &Default::default(),
            &sampled_units(default_ranges().len()),
        );
        World::new(params, dist, seed)
    }

    /// Replaying the pre-step state recovers what the stepper drained with:
    /// every living drain a lone consumer took from a target that could cover
    /// it equals `h_eff · u_H · max(0, E − r·w)`, with `E` read off the
    /// metabolised roster (before growth, #623) and the target's structure off
    /// the drain-time roster, bit for bit.
    #[test]
    fn replayed_state_reproduces_the_stepper_s_living_drains() {
        let mut world = sample_31_world(1000);
        world.retain_event_kinds(&[EventKind::Consumed]);
        let params = world.params().clone();
        let k = params.wear_degradation_steepness;
        let mut checked = 0;
        for _ in 0..120 {
            let pre = PreStep::capture(&world);
            let cursor = world.event_log().len();
            world.step();
            let events: Vec<_> = world
                .event_log()
                .since(cursor)
                .iter()
                .filter(|e| e.kind == EventKind::Consumed && !e.target_was_carcass)
                .cloned()
                .collect();
            if events.is_empty() {
                continue;
            }
            let drain_time = pre.drain_time_agents(&params);
            let by_id: HashMap<u64, &Agent> = drain_time.iter().map(|a| (a.id, a)).collect();
            let metabolised = pre.metabolised_agents(&params);
            let gate_of: HashMap<u64, f32> = metabolised
                .iter()
                .map(|a| (a.id, phase::consumption_expression(a, &params)))
                .collect();
            let mut consumers_of: HashMap<u64, usize> = HashMap::new();
            for e in &events {
                *consumers_of.entry(e.target.unwrap()).or_default() += 1;
            }
            for e in &events {
                let target = e.target.unwrap();
                if consumers_of[&target] != 1 {
                    continue;
                }
                let (c, t) = (by_id[&e.source], by_id[&target]);
                let expression = (gate_of[&e.source]
                    - phase::RECOGNITION_RESTRAINT
                        * phase::resemblance(&c.traits, &t.traits, &params))
                .max(0.0);
                let demand = c.effective_trait_with_steepness(1, k)
                    * explorers_sim::units::HETEROTROPHY_STRUCTURE_DRAIN_PER_TICK
                    * expression;
                if demand > t.structure {
                    continue;
                }
                assert_eq!(e.energy_delta.to_bits(), demand.to_bits(), "{e:?}");
                checked += 1;
            }
            world.compact_event_log_before(world.event_log().len());
        }
        assert!(checked > 10, "only {checked} drains checked");
    }

    /// Satiation reads both currencies in ticks of maintenance, the reading
    /// the need gate took before #623: `1 / (1 + c·min(s_E, s_N))` on the
    /// whole reserve at drain time.
    #[test]
    fn satiation_is_the_reserve_based_co_limited_reading() {
        let mut world = sample_31_world(1001);
        for _ in 0..60 {
            world.step();
        }
        let params = world.params().clone();
        let drain_time = PreStep::capture(&world).drain_time_agents(&params);
        let (mut energy_limited, mut nutrient_limited) = (0, 0);
        for a in &drain_time {
            let s = Satiation::of(a, &params);
            let want = 1.0 / (1.0 + params.satiation_sensitivity * s.ticks());
            let got = s.expression(&params);
            assert!(
                (got - want).abs() <= 1e-6 * want.max(1e-6),
                "{s:?}: {got} vs {want}"
            );
            let m = phase::metabolic_cost(a, &params);
            assert!((s.energy - a.reserve.max(0.0) / m).abs() <= 1e-5 * s.energy.max(1.0));
            assert_eq!(s.ticks(), s.energy.min(s.nutrient));
            if s.nutrient_limited() {
                nutrient_limited += 1;
            } else {
                energy_limited += 1;
            }
        }
        assert!(energy_limited + nutrient_limited > 10);
    }

    /// The stepper reads surplus satiation itself (#623): on the metabolised
    /// roster, [`Surplus`]'s expression `1 / (1 + c·s)` is the stepper's
    /// need gate, and some agent is gated by it.
    #[test]
    fn surplus_expression_is_the_stepper_s_need_gate() {
        let mut world = sample_31_world(1001);
        for _ in 0..60 {
            world.step();
        }
        let params = world.params().clone();
        let metabolised = PreStep::capture(&world).metabolised_agents(&params);
        let mut gated = 0;
        for a in &metabolised {
            let want = phase::consumption_expression(a, &params);
            let got = Surplus::of(a, &params).expression(&params);
            assert!(
                (got - want).abs() <= 1e-5 * want.max(1e-6),
                "{:?}: {got} vs {want}",
                Surplus::of(a, &params)
            );
            gated += usize::from(want < 1.0);
        }
        assert!(gated > 0, "some agent carries a surplus the gate reads");
    }

    fn lone_agent(reserve: f32, nutrient: f32) -> (Agent, WorldParameters) {
        let params = sample_31_world(1000).params().clone();
        let traits = TraitVector {
            photosynthetic_absorption: 0.8,
            heterotrophy: 0.2,
            mobility: 0.0,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 0.5,
            dispersal: 0.3,
        };
        (
            Agent::new(0, (0.0, 0.0), reserve, 1.0, nutrient, traits),
            params,
        )
    }

    /// Surplus satiation (#621, #622) counts reserve above the grow phase's
    /// retention buffer, in ticks of maintenance.
    #[test]
    fn surplus_counts_reserve_above_the_retention_buffer() {
        let (probe, params) = lone_agent(0.0, 1e6);
        let m = phase::metabolic_cost(&probe, &params);
        let buffer = params.growth_retention_multiplier * m;
        let (above, _) = lone_agent(buffer + 3.0 * m, 1e6);
        let s = Surplus::of(&above, &params).ticks();
        assert!((s - 3.0).abs() < 1e-4, "{s}");
    }

    /// An agent at or below its buffer has no surplus, however much reserve
    /// it holds: `s` is floored at zero, and the energy side reads negative.
    #[test]
    fn surplus_is_zero_at_and_below_the_buffer() {
        let (probe, params) = lone_agent(0.0, 1e6);
        let m = phase::metabolic_cost(&probe, &params);
        let buffer = params.growth_retention_multiplier * m;
        assert!(buffer > m, "a buffer of several ticks: {buffer} vs {m}");
        let (at, _) = lone_agent(buffer, 1e6);
        assert!(Surplus::of(&at, &params).ticks().abs() < 1e-5);
        let (below, _) = lone_agent(buffer - 0.5 * m, 1e6);
        let s = Surplus::of(&below, &params);
        assert_eq!(s.ticks(), 0.0);
        assert!((s.energy + 0.5).abs() < 1e-4, "{s:?}");
        let (empty, _) = lone_agent(0.0, 1e6);
        assert_eq!(Surplus::of(&empty, &params).ticks(), 0.0);
    }

    /// Surplus energy counts only as far as free nutrient could match it in
    /// growth, and the nutrient side is not shifted by the buffer: an agent
    /// far above its buffer, holding nutrient to match 2 ticks of maintenance,
    /// reads 2 ticks (not 2 minus the buffer's depth).
    #[test]
    fn surplus_is_co_limited_by_unbuffered_free_nutrient() {
        let (probe, params) = lone_agent(0.0, 0.0);
        let m = phase::metabolic_cost(&probe, &params);
        let buffer = params.growth_retention_multiplier * m;
        let binding = params.growth_efficiency
            * explorers_sim::stoichiometric_demand(&probe.traits, 1.0, &params);
        assert!(binding > 0.0 && params.growth_retention_multiplier > 2.0);
        let (limited, _) = lone_agent(buffer + 10.0 * m, 2.0 * m * binding);
        let s = Surplus::of(&limited, &params);
        assert!((s.nutrient - 2.0).abs() < 1e-4, "{s:?}");
        assert!((s.ticks() - 2.0).abs() < 1e-4, "{s:?}");
        let (starved, _) = lone_agent(buffer + 10.0 * m, 0.0);
        assert_eq!(Surplus::of(&starved, &params).ticks(), 0.0);
    }

    /// The surplus read sits between metabolism and growth: growing the
    /// metabolised roster is the drain-time roster, bit for bit, and the
    /// surplus read off it is the stepper's buffer arithmetic.
    #[test]
    fn metabolised_roster_is_the_drain_time_roster_before_growth() {
        let mut world = sample_31_world(1002);
        for _ in 0..60 {
            world.step();
        }
        let params = world.params().clone();
        let pre = PreStep::capture(&world);
        let mut grown = pre.metabolised_agents(&params);
        let metabolised = grown.clone();
        phase::grow(&mut grown, &params);
        let drain_time = pre.drain_time_agents(&params);
        assert_eq!(grown.len(), drain_time.len());
        let mut positive = 0;
        for (a, b) in grown.iter().zip(&drain_time) {
            assert_eq!(a.reserve.to_bits(), b.reserve.to_bits());
            assert_eq!(a.structure.to_bits(), b.structure.to_bits());
            assert_eq!(a.nutrient.to_bits(), b.nutrient.to_bits());
        }
        for a in &metabolised {
            let m = phase::metabolic_cost(a, &params);
            let s = Surplus::of(a, &params);
            let want = (a.reserve - params.growth_retention_multiplier * m) / m;
            assert!(
                (s.energy - want).abs() <= 1e-4 * want.abs().max(1.0),
                "{s:?}"
            );
            positive += usize::from(s.ticks() > 0.0);
        }
        assert!(positive > 0, "some agent holds a surplus before growth");
    }

    /// The surplus distribution counts agents at zero apart and reads
    /// percentiles off fine log bins, within a bin's width; merging adds.
    #[test]
    fn surplus_distribution_reads_the_zero_share_and_percentiles() {
        let mut d = SurplusDistribution::default();
        assert_eq!(d.percentile(0.5), None);
        for _ in 0..100 {
            d.record(0.0);
        }
        for i in 1..=300 {
            d.record(i as f32 * 0.1);
        }
        assert_eq!(d.count(), 400);
        assert_eq!(d.zero, 100);
        assert!((d.zero_share().unwrap() - 0.25).abs() < 1e-12);
        assert_eq!(d.percentile(0.2), Some(0.0));
        // The 50th percentile is the 100th positive value (10.0), the 75th the
        // 200th (20.0); bins are 20 per decade, so within ~12 %.
        let close = |got: f64, want: f64| (got / want - 1.0).abs() < 0.13;
        assert!(close(d.percentile(0.5).unwrap(), 10.0), "{d:?}");
        assert!(close(d.percentile(0.75).unwrap(), 20.0));
        d.record(f32::INFINITY);
        d.record(1e-6);
        assert_eq!(d.count(), 402);
        let mut sum = d.clone();
        sum.merge(&d);
        assert_eq!(sum.count(), 804);
        assert_eq!(sum.zero, 200);
        assert!(close(
            sum.percentile(0.5).unwrap(),
            d.percentile(0.5).unwrap()
        ));
    }

    /// The positive-surplus percentile ignores the zeros: the criterion for
    /// the default sensitivity (#622) reads only agents that can be sated.
    #[test]
    fn positive_percentile_reads_only_samples_above_zero() {
        let mut d = SurplusDistribution::default();
        assert_eq!(d.positive_percentile(0.25), None);
        for _ in 0..1000 {
            d.record(0.0);
        }
        assert_eq!(d.positive_percentile(0.25), None);
        for i in 1..=400 {
            d.record(i as f32 * 0.1);
        }
        assert_eq!(d.percentile(0.25), Some(0.0));
        // The 100th of 400 positive values is 10.0.
        let got = d.positive_percentile(0.25).unwrap();
        assert!((got / 10.0 - 1.0).abs() < 0.13, "{got}");
    }

    /// Recording a [`Surplus`] also counts which side bound it: samples
    /// whose nutrient side is the lesser, and of the zeros, those held there
    /// by nutrient rather than by the buffer.
    #[test]
    fn surplus_distribution_splits_nutrient_limited_samples() {
        let mut d = SurplusDistribution::default();
        d.record_surplus(&Surplus {
            energy: 3.0,
            nutrient: 1.0,
        });
        d.record_surplus(&Surplus {
            energy: 3.0,
            nutrient: 0.0,
        });
        d.record_surplus(&Surplus {
            energy: -1.0,
            nutrient: 5.0,
        });
        d.record_surplus(&Surplus {
            energy: 2.0,
            nutrient: 5.0,
        });
        assert_eq!((d.count(), d.zero), (4, 2));
        assert_eq!((d.nutrient_limited, d.zero_nutrient_limited), (2, 1));
        let mut sum = d.clone();
        sum.merge(&d);
        assert_eq!((sum.nutrient_limited, sum.zero_nutrient_limited), (4, 2));
    }

    fn sated(ticks: f32, nutrient_limited: bool) -> Satiation {
        if nutrient_limited {
            Satiation {
                energy: ticks + 1.0,
                nutrient: ticks,
            }
        } else {
            Satiation {
                energy: ticks,
                nutrient: ticks + 1.0,
            }
        }
    }

    /// A pair lands in its kinship, trait-distance and satiation bands; the
    /// hungry share is the pairs whose grazer held under half-expression
    /// satiation, and merging adds tallies.
    #[test]
    fn grazer_hunger_bands_pairs_by_kinship_distance_and_satiation() {
        let mut h = GrazerHunger::default();
        h.record(true, 0.05, sated(0.5, true));
        h.record(true, 0.3, sated(12.0, false));
        h.record(false, 1.4, sated(60.0, false));
        assert_eq!(h.pairs[1][0][0], 1);
        assert_eq!(h.pairs[1][2][3], 1);
        assert_eq!(h.pairs[0][4][5], 1);
        assert_eq!(h.nutrient_limited[1][0], 1);
        assert_eq!(h.total(true), 2);
        assert_eq!(h.hungry(true), 1);
        assert_eq!(h.within(true, RECOGNITION_BANDS), 2);
        let mut sum = h;
        sum.merge(&h);
        assert_eq!(sum.total(true), 4);
    }

    #[test]
    fn trait_distances_band_and_average_pair_distances() {
        let a = TraitVector {
            photosynthetic_absorption: 1.0,
            heterotrophy: 0.0,
            mobility: 0.0,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 0.5,
            dispersal: 0.3,
        };
        let mut t = TraitDistances::default();
        t.record(
            &a,
            &TraitVector {
                heterotrophy: 0.05,
                ..a
            },
        );
        t.record(
            &a,
            &TraitVector {
                heterotrophy: 0.6,
                ..a
            },
        );
        assert_eq!(t.bands, [1, 0, 0, 1, 0]);
        assert!((t.mean().unwrap() - 0.325).abs() < 1e-6);
        assert!((t.rms().unwrap() - ((0.0025 + 0.36) / 2.0f64).sqrt()).abs() < 1e-6);
        assert_eq!(TraitDistances::default().mean(), None);
    }
}
