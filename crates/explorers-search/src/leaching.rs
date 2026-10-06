//! The calibration read for carcass leaching's rate (#700; world-rules.md,
//! *Carcass energy decays only through agents; carcass nutrient leaches*:
//! "the top is set by a calibration read").
//!
//! [`DrainClock`] follows every carcass formed in a run, from its death to
//! the first bite any drainer takes from it, off the event log alone.
//!
//! Observer-side only: the world is never touched.

use std::collections::HashMap;

use explorers_sim::Carcass;
use explorers_sim::event::{Event, EventKind};

/// One carcass formed in a run: the tick it died on, and the tick of the
/// first bite any drainer took from it (`None`: never drained before the
/// run ended, so censored at the run's end). Ticks are the world's tick after
/// the step the event fell in. `bitten`: drained while living in the step it
/// died (a kill, or a death a drain hastened), so its drainer is usually on
/// it already. Serialised as `[died, drained | null, bitten]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(from = "(u64, Option<u64>, bool)", into = "(u64, Option<u64>, bool)")]
pub struct CarcassFate {
    pub died: u64,
    pub drained: Option<u64>,
    pub bitten: bool,
}

impl From<(u64, Option<u64>, bool)> for CarcassFate {
    fn from((died, drained, bitten): (u64, Option<u64>, bool)) -> Self {
        Self {
            died,
            drained,
            bitten,
        }
    }
}

impl From<CarcassFate> for (u64, Option<u64>, bool) {
    fn from(f: CarcassFate) -> Self {
        (f.died, f.drained, f.bitten)
    }
}

/// Every carcass formed in a run (a `Died` event), timed to its first drain
/// (a `Consumed` event on it as a carcass). Carcasses placed at world
/// creation never died in the run and are not followed.
#[derive(Clone, Debug, Default)]
pub struct DrainClock {
    /// Undrained carcasses: id → (death tick, bitten to death).
    open: HashMap<u64, (u64, bool)>,
    fates: Vec<CarcassFate>,
}

impl DrainClock {
    /// Read one step's events; `tick` is the world's tick after the step.
    pub fn observe(&mut self, tick: u64, events: &[Event]) {
        let bitten: std::collections::HashSet<u64> = events
            .iter()
            .filter(|e| e.kind == EventKind::Consumed && !e.target_was_carcass)
            .filter_map(|e| e.target)
            .collect();
        for e in events {
            match e.kind {
                EventKind::Died => {
                    self.open
                        .insert(e.source, (tick, bitten.contains(&e.source)));
                }
                EventKind::Consumed if e.target_was_carcass => {
                    if let Some((died, bitten)) = e.target.and_then(|t| self.open.remove(&t)) {
                        self.fates.push(CarcassFate {
                            died,
                            drained: Some(tick),
                            bitten,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    /// Every carcass followed, the undrained ones censored, ordered by death
    /// tick then first drain (undrained last).
    pub fn finish(self) -> Vec<CarcassFate> {
        let mut fates = self.fates;
        fates.extend(self.open.into_values().map(|(died, bitten)| CarcassFate {
            died,
            drained: None,
            bitten,
        }));
        fates.sort_by_key(|f| (f.died, f.drained.is_none(), f.drained, !f.bitten));
        fates
    }
}

/// Time to first drain over a set of carcasses: each drained one's ticks from
/// death to its first bite, each undrained one censored at its run's end.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FirstDrain {
    pub carcasses: u64,
    pub drained: u64,
    /// Drained carcasses' times, sorted.
    times: Vec<u64>,
    /// Every carcass as `(ticks, drained)`: the time to first drain, or to
    /// the run's end when censored. Sorted by ticks.
    survival: Vec<(u64, bool)>,
}

impl FirstDrain {
    /// Pool runs, each its carcasses and the tick it ended on (the
    /// censoring time), keeping the carcasses `keep` selects (the settled
    /// half: died at or after `horizon / 2 + 1`; or by `bitten`).
    pub fn of<'a>(
        runs: impl IntoIterator<Item = (&'a [CarcassFate], u64)>,
        keep: impl Fn(&CarcassFate) -> bool,
    ) -> Self {
        let mut d = FirstDrain::default();
        for (fates, end) in runs {
            for f in fates.iter().filter(|f| keep(f)) {
                d.carcasses += 1;
                match f.drained {
                    Some(t) => {
                        d.drained += 1;
                        d.times.push(t - f.died);
                        d.survival.push((t - f.died, true));
                    }
                    None => d.survival.push((end.saturating_sub(f.died), false)),
                }
            }
        }
        d.times.sort_unstable();
        d.survival.sort_unstable();
        d
    }

    /// The share of carcasses never drained before their run ended.
    pub fn never_share(&self) -> Option<f64> {
        (self.carcasses > 0).then(|| (self.carcasses - self.drained) as f64 / self.carcasses as f64)
    }

    /// Quantile `q` of the drained carcasses' times (linear between order
    /// statistics), ignoring the undrained.
    pub fn drained_quantile(&self, q: f64) -> Option<f64> {
        let n = self.times.len();
        if n == 0 {
            return None;
        }
        let x = q.clamp(0.0, 1.0) * (n - 1) as f64;
        let (lo, hi) = (x.floor() as usize, x.ceil() as usize);
        let (a, b) = (self.times[lo] as f64, self.times[hi] as f64);
        Some(a + (b - a) * (x - lo as f64))
    }

    /// The Kaplan–Meier median time to first drain, the undrained carcasses
    /// censored at their run's end: the first drain time at which the
    /// estimated share still undrained falls to one half. `None` when it
    /// never does.
    pub fn km_median(&self) -> Option<f64> {
        let mut at_risk = self.survival.len() as f64;
        let mut s = 1.0;
        let mut i = 0;
        while i < self.survival.len() {
            let t = self.survival[i].0;
            let (mut events, mut leaving) = (0.0, 0.0);
            while i < self.survival.len() && self.survival[i].0 == t {
                events += f64::from(u8::from(self.survival[i].1));
                leaving += 1.0;
                i += 1;
            }
            if events > 0.0 {
                s *= 1.0 - events / at_risk;
                if s <= 0.5 {
                    return Some(t as f64);
                }
            }
            at_risk -= leaving;
        }
        None
    }
}

/// The proposed top of the leaching rate's range from a typical time to
/// first drain `t*`: `ln 2 / t*`, at which a carcass leaches half its
/// leachable store in the time a drainer takes to reach it.
pub fn lambda_max(t_star: Option<f64>) -> Option<f64> {
    t_star
        .filter(|&t| t > 0.0)
        .map(|t| std::f64::consts::LN_2 / t)
}

/// Carcass nutrient leaving the dead pool, by route: leached to the
/// available pool (`Leached` events), or drained (everything else a carcass
/// lost in the step: what drainers retained and released, and an exhausted
/// carcass's residue). Summed over the steps observed.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CarcassNutrient {
    pub leached: f64,
    pub drained: f64,
}

impl CarcassNutrient {
    /// Read one step: the carcasses `before` it (as the world held them, not
    /// leached), those `after` it, and its events. A carcass formed in the
    /// step has lost nothing yet.
    pub fn observe(&mut self, before: &[Carcass], after: &[Carcass], events: &[Event]) {
        let after: HashMap<u64, f32> = after.iter().map(|c| (c.id, c.nutrient)).collect();
        let lost: f64 = before
            .iter()
            .map(|c| f64::from(c.nutrient - after.get(&c.id).copied().unwrap_or(0.0)))
            .sum();
        let leached: f64 = events
            .iter()
            .filter(|e| e.kind == EventKind::Leached)
            .map(|e| f64::from(e.nutrient_delta))
            .sum();
        self.leached += leached;
        self.drained += lost - leached;
    }

    pub fn merge(&mut self, o: &CarcassNutrient) {
        self.leached += o.leached;
        self.drained += o.drained;
    }

    /// The leached share of the carcass nutrient that left; `None` when
    /// none left.
    pub fn leached_share(&self) -> Option<f64> {
        let total = self.leached + self.drained;
        (total > 0.0).then(|| self.leached / total)
    }
}

/// One run's (config × seed) leaching readout: raw enough that every
/// reading in #700 (pooled, per config, settled half, censored) is computed
/// offline from these rows.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LeachRun {
    pub seed: u64,
    /// The tick the run ended on (the horizon, or an early stop): the
    /// censoring time of its undrained carcasses.
    pub end_tick: u64,
    /// The census's verdict (`persisted` or a failure label).
    pub outcome: String,
    /// The dead pool's share of conserved nutrient, as the evaluator's
    /// carcass-locked-fraction descriptor reads it (the trailing mean over
    /// the lockup gate's window), but read on every run, gated or not.
    pub carcass_locked_tail: f32,
    /// The same series' mean over the settled half's ticks (`None` when the
    /// run stopped before it).
    pub carcass_locked_settled: Option<f32>,
    /// Carcass nutrient out by route, whole run and settled half.
    pub nutrient: CarcassNutrient,
    pub nutrient_settled: CarcassNutrient,
    /// Second-half carcass structure drained by light-fed mixotrophs
    /// (kin_killer_diet's H bucket), and by every attributed recipient.
    pub lfm_drained_settled: f64,
    pub attributed_drained_settled: f64,
    pub carcasses: Vec<CarcassFate>,
}

/// A sweep's quantities over a set of runs (#700, item 3).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SweepReading {
    pub runs: u64,
    /// Runs per verdict label.
    pub outcomes: std::collections::BTreeMap<String, u64>,
    /// Mean over runs of `carcass_locked_tail`, and of
    /// `carcass_locked_settled` over the runs that reached the settled half.
    pub carcass_locked_tail: Option<f64>,
    pub carcass_locked_settled: Option<f64>,
    pub nutrient: CarcassNutrient,
    pub nutrient_settled: CarcassNutrient,
    /// Light-fed mixotrophs' share of the attributed second-half carcass
    /// structure drained, pooled.
    pub lfm_share_settled: Option<f64>,
}

impl SweepReading {
    pub fn of<'a>(runs: impl IntoIterator<Item = &'a LeachRun>) -> Self {
        let mut r = SweepReading::default();
        let (mut tail, mut settled, mut settled_n) = (0.0, 0.0, 0u64);
        let (mut lfm, mut attributed) = (0.0, 0.0);
        for run in runs {
            r.runs += 1;
            *r.outcomes.entry(run.outcome.clone()).or_default() += 1;
            tail += f64::from(run.carcass_locked_tail);
            if let Some(s) = run.carcass_locked_settled {
                settled += f64::from(s);
                settled_n += 1;
            }
            r.nutrient.merge(&run.nutrient);
            r.nutrient_settled.merge(&run.nutrient_settled);
            lfm += run.lfm_drained_settled;
            attributed += run.attributed_drained_settled;
        }
        r.carcass_locked_tail = (r.runs > 0).then(|| tail / r.runs as f64);
        r.carcass_locked_settled = (settled_n > 0).then(|| settled / settled_n as f64);
        r.lfm_share_settled = (attributed > 0.0).then(|| lfm / attributed);
        r
    }

    /// Runs with this verdict label.
    pub fn count(&self, label: &str) -> u64 {
        self.outcomes.get(label).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(kind: EventKind, source: u64, target: Option<u64>, carcass: bool) -> Event {
        Event {
            tick: 0,
            seq: 0,
            kind,
            source,
            target,
            energy_delta: 1.0,
            position: None,
            target_was_carcass: carcass,
            second_parent: None,
            nutrient_delta: 0.0,
        }
    }

    fn died(id: u64) -> Event {
        event(EventKind::Died, id, None, false)
    }

    fn bite(consumer: u64, carcass: u64) -> Event {
        event(EventKind::Consumed, consumer, Some(carcass), true)
    }

    fn fate(died: u64, drained: Option<u64>) -> CarcassFate {
        CarcassFate {
            died,
            drained,
            bitten: false,
        }
    }

    #[test]
    fn a_carcass_is_timed_from_its_death_to_the_first_bite_any_drainer_takes() {
        let mut clock = DrainClock::default();
        clock.observe(10, &[died(1), died(2)]);
        // A living drain on agent 3 is not a carcass bite.
        clock.observe(11, &[event(EventKind::Consumed, 9, Some(3), false)]);
        clock.observe(14, &[bite(9, 1)]);
        clock.observe(15, &[bite(8, 1), bite(9, 7)]);
        clock.observe(20, &[died(3)]);
        assert_eq!(
            clock.finish(),
            vec![fate(10, Some(14)), fate(10, None), fate(20, None),]
        );
    }

    #[test]
    fn a_carcass_drained_while_living_in_the_step_it_died_was_bitten_to_death() {
        let mut clock = DrainClock::default();
        clock.observe(
            5,
            &[
                event(EventKind::Consumed, 9, Some(1), false),
                died(1),
                died(2),
            ],
        );
        clock.observe(6, &[bite(9, 1), bite(9, 2)]);
        let fates = clock.finish();
        assert_eq!(
            fates,
            vec![
                CarcassFate {
                    died: 5,
                    drained: Some(6),
                    bitten: true
                },
                fate(5, Some(6)),
            ]
        );
        // Kills only, or deaths a drainer has to find.
        let kills = FirstDrain::of([(&fates[..], 10)], |f| f.bitten);
        let found = FirstDrain::of([(&fates[..], 10)], |f| !f.bitten);
        assert_eq!((kills.carcasses, found.carcasses), (1, 1));
    }

    fn carcass(id: u64, nutrient: f32) -> Carcass {
        Carcass {
            id,
            position: (0.0, 0.0),
            energy: 1.0,
            nutrient,
            traits: explorers_sim::TraitVector {
                photosynthetic_absorption: 0.0,
                heterotrophy: 0.5,
                mobility: 0.0,
                kappa: 0.5,
                fecundity: 1.0,
                asexual_propensity: 0.5,
                dispersal: 0.3,
            },
        }
    }

    #[test]
    fn carcass_nutrient_leaves_by_leaching_or_by_drains() {
        let mut n = CarcassNutrient::default();
        let mut leach = event(EventKind::Leached, 1, None, false);
        leach.nutrient_delta = 1.0;
        // Carcass 1 leaches 1 and is drained of 2; carcass 2 is drained
        // whole and leaves; carcass 3 is new this step and counts nothing.
        n.observe(
            &[carcass(1, 5.0), carcass(2, 3.0)],
            &[carcass(1, 2.0), carcass(3, 4.0)],
            &[leach],
        );
        assert_eq!(
            n,
            CarcassNutrient {
                leached: 1.0,
                drained: 5.0
            }
        );
        assert_eq!(n.leached_share(), Some(1.0 / 6.0));
        assert_eq!(CarcassNutrient::default().leached_share(), None);
    }

    #[test]
    fn first_drain_reads_drained_times_and_censors_the_undrained_at_the_run_s_end() {
        let a = [fate(10, Some(14)), fate(10, None), fate(20, Some(21))];
        let b = [fate(30, None)];
        let d = FirstDrain::of([(&a[..], 100), (&b[..], 50)], |_| true);
        assert_eq!((d.carcasses, d.drained), (4, 2));
        assert_eq!(d.never_share(), Some(0.5));
        // Drained times 4 and 1.
        assert_eq!(d.drained_quantile(0.5), Some(2.5));
        assert_eq!(d.drained_quantile(0.0), Some(1.0));
        // Kaplan–Meier over 1 (drained), 4 (drained), 20 and 90 (censored):
        // S(1) = 3/4, S(4) = 3/4 · 2/3 = 1/2, so the median is 4.
        assert_eq!(d.km_median(), Some(4.0));
    }

    #[test]
    fn the_settled_half_keeps_carcasses_that_died_in_it() {
        let a = [fate(10, Some(14)), fate(20, Some(21)), fate(30, None)];
        let d = FirstDrain::of([(&a[..], 100)], |f| f.died >= 20);
        assert_eq!((d.carcasses, d.drained), (2, 1));
        assert_eq!(d.drained_quantile(0.5), Some(1.0));
        assert_eq!(d.km_median(), Some(1.0));
    }

    #[test]
    fn the_censored_median_is_unread_when_survival_stays_above_a_half() {
        let a = [
            fate(0, Some(5)),
            fate(0, None),
            fate(0, None),
            fate(0, None),
        ];
        let d = FirstDrain::of([(&a[..], 100)], |_| true);
        assert_eq!(d.drained_quantile(0.5), Some(5.0));
        assert_eq!(d.km_median(), None);
        let empty = FirstDrain::of([(&[][..], 100)], |_| true);
        assert_eq!(empty.never_share(), None);
        assert_eq!(empty.drained_quantile(0.5), None);
        assert_eq!(empty.km_median(), None);
    }

    #[test]
    fn a_censored_carcass_at_a_drain_time_is_still_at_risk_there() {
        // Drained at 2, censored at 2, drained at 3: S(2) = 2/3, S(3) = 0.
        let a = [fate(0, Some(2)), fate(0, Some(3))];
        let b = [fate(0, None)];
        let d = FirstDrain::of([(&a[..], 100), (&b[..], 2)], |_| true);
        assert_eq!(d.km_median(), Some(3.0));
    }

    #[test]
    fn lambda_max_halves_a_typical_store_in_the_time_to_first_drain() {
        let l = lambda_max(Some(10.0)).unwrap();
        assert!((l - std::f64::consts::LN_2 / 10.0).abs() < 1e-12);
        assert_eq!(lambda_max(Some(0.0)), None);
        assert_eq!(lambda_max(None), None);
    }

    #[test]
    fn a_sweep_reading_pools_runs() {
        let run = |outcome: &str, tail: f32, settled: Option<f32>, leached: f64| LeachRun {
            outcome: outcome.into(),
            carcass_locked_tail: tail,
            carcass_locked_settled: settled,
            nutrient: CarcassNutrient {
                leached,
                drained: 3.0,
            },
            lfm_drained_settled: 1.0,
            attributed_drained_settled: 4.0,
            ..Default::default()
        };
        let runs = [
            run("persisted", 0.1, Some(0.2), 1.0),
            run("nutrient_lockup", 0.5, None, 0.0),
        ];
        let r = SweepReading::of(&runs);
        assert_eq!(r.runs, 2);
        assert_eq!((r.count("persisted"), r.count("nutrient_lockup")), (1, 1));
        assert!((r.carcass_locked_tail.unwrap() - 0.3).abs() < 1e-6);
        assert!((r.carcass_locked_settled.unwrap() - 0.2).abs() < 1e-6);
        assert_eq!(r.nutrient.leached_share(), Some(1.0 / 7.0));
        assert_eq!(r.lfm_share_settled, Some(0.25));
        assert_eq!(SweepReading::of(&[]).lfm_share_settled, None);
    }

    /// On a stepping world with leaching on, the clock times real carcasses
    /// (each drained after it died) and the nutrient split reads both
    /// routes, no step's drains negative beyond rounding.
    #[test]
    fn the_readout_follows_a_stepping_world() {
        use crate::config_source::{ConfigSource, resolve_config, sampled_units};
        let (mut params, dist) = resolve_config(
            ConfigSource::SAMPLE,
            31,
            &Default::default(),
            &sampled_units(),
        );
        params.leaching_rate = 0.05;
        let mut world = explorers_sim::World::new(params, dist, 1000);
        world.retain_event_kinds(&[EventKind::Died, EventKind::Consumed, EventKind::Leached]);
        let (mut clock, mut nutrient) = (DrainClock::default(), CarcassNutrient::default());
        for _ in 0..200 {
            let before = world.carcasses().to_vec();
            let cursor = world.event_log().len();
            world.step();
            let events = world.event_log().since(cursor).to_vec();
            let mut step = CarcassNutrient::default();
            step.observe(&before, world.carcasses(), &events);
            assert!(step.drained > -1e-3, "{step:?}");
            nutrient.merge(&step);
            clock.observe(world.tick(), &events);
            world.compact_event_log_before(world.event_log().len());
        }
        let fates = clock.finish();
        assert!(fates.iter().filter(|f| f.drained.is_some()).count() > 10);
        assert!(fates.iter().all(|f| f.drained.is_none_or(|t| t > f.died)));
        assert!(
            nutrient.leached > 0.0 && nutrient.drained > 0.0,
            "{nutrient:?}"
        );
    }

    #[test]
    fn a_fate_serialises_as_a_pair() {
        let f = CarcassFate {
            died: 3,
            drained: None,
            bitten: true,
        };
        let json = serde_json::to_string(&f).unwrap();
        assert_eq!(json, "[3,null,true]");
        assert_eq!(serde_json::from_str::<CarcassFate>(&json).unwrap(), f);
    }
}
