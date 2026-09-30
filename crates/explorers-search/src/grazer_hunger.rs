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

    /// The roster as the drain pass read it: the tick's first four phases
    /// replayed, in the stepper's order, on a copy of the pre-step state.
    pub fn drain_time_agents(&self, params: &WorldParameters) -> Vec<Agent> {
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
        phase::grow(&mut agents, params);
        agents
    }
}

/// An agent's satiation in both currencies, in ticks of its own maintenance
/// (its per-tick metabolic cost): `energy` is its reserve, `nutrient` the
/// energy its free nutrient can match when built into structure
/// (`N / (growth_efficiency × demand ratio)`), infinite when growth binds no
/// nutrient. The need gate reads the lesser ([`Satiation::ticks`]).
///
/// Computed here rather than through the stepper so the same read works on a
/// tree without the need gate (its formula is pinned against the stepper's in
/// the tests).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Satiation {
    pub energy: f32,
    pub nutrient: f32,
}

impl Satiation {
    pub fn of(agent: &Agent, params: &WorldParameters) -> Self {
        let x = params.maintenance_cost_exponent;
        let t = &agent.traits;
        let need = (params.base_metabolic_rate
            + t.photosynthetic_absorption.powf(x) * params.photo_maintenance_cost
            + t.heterotrophy.powf(x) * params.heterotrophy_maintenance_cost
            + t.mobility.powf(x) * params.mobility_maintenance_cost
            + t.asexual_propensity.powf(x) * params.asexual_propensity_maintenance_cost
            + agent.structure * params.structure_maintenance_coefficient)
            .max(0.0);
        let binding =
            params.growth_efficiency * explorers_sim::stoichiometric_demand(t, 1.0, params);
        let matched = if binding > 0.0 {
            agent.nutrient.max(0.0) / binding
        } else {
            f32::INFINITY
        };
        let ticks = |e: f32| if need > 0.0 { e / need } else { f32::INFINITY };
        Satiation {
            energy: ticks(agent.reserve.max(0.0)),
            nutrient: ticks(matched),
        }
    }

    /// The co-limited satiation the need gate reads: the scarcer currency.
    pub fn ticks(&self) -> f32 {
        self.energy.min(self.nutrient)
    }

    /// Whether nutrient, not energy, is the scarcer currency.
    pub fn nutrient_limited(&self) -> bool {
        self.nutrient < self.energy
    }

    /// The need-gated expression `1 / (1 + c·s)` at this satiation.
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
/// default `satiation_sensitivity = 0.1`, 10 ticks is half expression, from
/// which recognition spares an identical target entirely.
pub const SATIATION_EDGES: [f32; 5] = [1.0, 5.0, 10.0, 20.0, 50.0];
pub const SATIATION_BANDS: usize = SATIATION_EDGES.len() + 1;
/// The bands under half expression (at the default sensitivity): hungry.
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

    /// Replaying the pre-step state to the drain pass recovers the state the
    /// stepper drained from: every living drain a lone consumer took from a
    /// target that could cover it equals `h_eff · u_H · max(0, E − r·w)` read
    /// off the replayed roster, bit for bit.
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
                let expression = (phase::consumption_expression(c, &params)
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

    /// Satiation reads both currencies in ticks of maintenance, and its
    /// expression is the stepper's need gate: `1 / (1 + c·min(s_E, s_N))`.
    #[test]
    fn satiation_is_the_need_gate_s_co_limited_reading() {
        let mut world = sample_31_world(1001);
        for _ in 0..60 {
            world.step();
        }
        let params = world.params().clone();
        let drain_time = PreStep::capture(&world).drain_time_agents(&params);
        let (mut energy_limited, mut nutrient_limited) = (0, 0);
        for a in &drain_time {
            let s = Satiation::of(a, &params);
            let want = phase::consumption_expression(a, &params);
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
