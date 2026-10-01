//! Invasion machinery shared by the invasion bins (`invasion_growth`, #443;
//! `reinvasion_barrier`, #587): a cohort injected into a forked resident,
//! followed by descent off the event log's `Born` events (the [`Lineage`]),
//! its per-capita rate `r`, and the seed-ensemble sign-test read of that rate
//! ([`summarise_rates`]). Moved here unchanged from `invasion_growth` so a
//! second instrument asks its question with the same protocol.

use std::collections::{HashMap, HashSet};

use explorers_genesis_eval::income::IncomeLedger;
use explorers_sim::event::{Event, EventKind};
use explorers_sim::{Agent, TraitVector, World};

/// Lineage size is sampled into the record every this many ticks.
pub const SERIES_INTERVAL: u64 = 25;

/// A lineage with no survivor is logged at half an agent so its rate is a
/// finite negative number; only the sign carries meaning for the criterion.
pub const EXTINCT_FLOOR: f64 = 0.5;

/// Consumption events by target kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConsumedCounts {
    pub living: usize,
    pub carcass: usize,
}

/// Energy drained (a `Consumed` event's `energy_delta`, before the trophic
/// efficiency is applied) by target kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DrainedEnergy {
    pub living: f64,
    pub carcass: f64,
}

/// The lineage's per-capita per-tick growth over a window:
/// `r = ln(max(N_end, ½) / N_0) / ticks`.
pub fn growth_rate(cohort: usize, end: usize, ticks: u64) -> f64 {
    let n_end = if end == 0 { EXTINCT_FLOOR } else { end as f64 };
    (n_end / cohort as f64).ln() / ticks.max(1) as f64
}

/// Add one agent with the given traits at each position, provisioned exactly
/// as `World::new` provisions founders (`energy_per_agent` split by
/// `provision_initial_reserve_structure`; the birth structure's bound nutrient
/// drawn from the pool under the agent, its shortfall from the nearest cells
/// when that cell cannot cover it; free store empty). Returns the new
/// agents' ids, in position order.
pub fn place_cohort(
    world: &mut World,
    traits: TraitVector,
    positions: &[(f32, f32)],
    energy_per_agent: f32,
) -> Vec<u64> {
    let (reserve, structure, _heat) =
        explorers_sim::provision_initial_reserve_structure(energy_per_agent, world.params());
    let mut ids = Vec::with_capacity(positions.len());
    for &(x, y) in positions {
        let agent = Agent::new(0, (x, y), reserve, structure, 0.0, traits);
        let bound = agent.bound_nutrient(world.params());
        if bound > 0.0 {
            world.nutrient_grid_mut().draw_nearest((x, y), bound);
        }
        world.add_agent(agent);
        ids.push(world.agents().last().expect("just added").id);
    }
    ids
}

/// What a window run returns.
pub struct WindowOutcome {
    pub ticks_run: u64,
    pub stopped: Option<&'static str>,
    pub series: Vec<usize>,
    pub alive: usize,
    pub extinct_tick: Option<u64>,
    pub lineage: Option<Lineage>,
}

/// Step a forked world through the window, following the lineage (if any)
/// off the log, and bring the income ledger (the role read) up to date.
pub fn run_window(
    world: &mut World,
    income: &mut IncomeLedger,
    mut lineage: Option<Lineage>,
    window: u64,
    max_population: usize,
) -> WindowOutcome {
    let mut cursor = world.event_log().len();
    let cohort = lineage.as_ref().map_or(0, |l| l.alive(world.agents()));
    let mut series = vec![cohort];
    let mut alive = cohort;
    let mut extinct_tick = None;
    let mut ticks_run = 0;
    let mut stopped = None;
    for t in 1..=window {
        world.step();
        ticks_run = t;
        if let Some(l) = lineage.as_mut() {
            l.absorb(world.event_log().since(cursor));
            cursor = world.event_log().len();
            alive = l.alive(world.agents());
            if alive == 0 && extinct_tick.is_none() {
                extinct_tick = Some(t);
            }
        }
        if t.is_multiple_of(SERIES_INTERVAL) {
            series.push(alive);
        }
        if world.agents().is_empty() {
            stopped = Some("extinct");
            break;
        }
        if world.agents().len() > max_population {
            stopped = Some("explosion");
            break;
        }
    }
    if !ticks_run.is_multiple_of(SERIES_INTERVAL) {
        series.push(alive);
    }
    income.update(world.event_log());
    WindowOutcome {
        ticks_run,
        stopped,
        series,
        alive,
        extinct_tick,
        lineage,
    }
}

/// An invader cohort and its descendants, followed by descent off the event
/// log's `Born` events (#443): a birth joins the lineage when *either* parent
/// is a member. Membership is permanent (the dead stay members), so the live
/// lineage at any tick is the intersection with the world's roster.
#[derive(Debug, Clone)]
pub struct Lineage {
    pub members: HashSet<u64>,
    /// Births with both parents in the lineage (or a single lineage parent).
    pub pure_births: usize,
    /// Sexual births with exactly one parent in the lineage: the lineage's
    /// genes leaving through a resident mate. Counted as members (the
    /// either-parent rule) but reported so the reader can judge the rule.
    pub cross_births: usize,
    /// `Consumed` events by a member, by target kind.
    pub consumed: ConsumedCounts,
    /// Energy drained by a member, by target kind (#587).
    pub drained: DrainedEnergy,
    /// Mortality attribution (#493 §6): `Consumed` events whose *target* is a
    /// living member (the lineage being eaten), and member deaths split by
    /// whether the member was drained in the tick it died.
    pub preyed_upon_events: usize,
    pub deaths_drained: usize,
    pub deaths_undrained: usize,
    /// Tick on which each member was last drained alive.
    last_drained: HashMap<u64, u64>,
}

impl Lineage {
    pub fn new(founders: impl IntoIterator<Item = u64>) -> Self {
        Lineage {
            members: founders.into_iter().collect(),
            pure_births: 0,
            cross_births: 0,
            consumed: ConsumedCounts::default(),
            drained: DrainedEnergy::default(),
            preyed_upon_events: 0,
            deaths_drained: 0,
            deaths_undrained: 0,
            last_drained: HashMap::new(),
        }
    }

    /// Absorb a slice of the event log (in sequence order): descent off
    /// `Born`, diet off `Consumed`.
    pub fn absorb(&mut self, events: &[Event]) {
        for ev in events {
            if ev.kind == EventKind::Consumed {
                if self.members.contains(&ev.source) {
                    let drain = ev.energy_delta as f64;
                    if ev.target_was_carcass {
                        self.consumed.carcass += 1;
                        self.drained.carcass += drain;
                    } else {
                        self.consumed.living += 1;
                        self.drained.living += drain;
                    }
                }
                if !ev.target_was_carcass && ev.target.is_some_and(|t| self.members.contains(&t)) {
                    self.preyed_upon_events += 1;
                    self.last_drained.insert(ev.target.unwrap(), ev.tick);
                }
                continue;
            }
            if ev.kind == EventKind::Died && self.members.contains(&ev.source) {
                if self.last_drained.get(&ev.source) == Some(&ev.tick) {
                    self.deaths_drained += 1;
                } else {
                    self.deaths_undrained += 1;
                }
                continue;
            }
            if ev.kind != EventKind::Born {
                continue;
            }
            let first = ev.target.is_some_and(|p| self.members.contains(&p));
            let second = ev.second_parent.is_some_and(|p| self.members.contains(&p));
            if !(first || second) {
                continue;
            }
            self.members.insert(ev.source);
            let outside_mate =
                (ev.target.is_some() && !first) || (ev.second_parent.is_some() && !second);
            if outside_mate {
                self.cross_births += 1;
            } else {
                self.pure_births += 1;
            }
        }
    }

    pub fn is_member(&self, id: u64) -> bool {
        self.members.contains(&id)
    }

    /// Members alive on the given roster.
    pub fn alive(&self, agents: &[Agent]) -> usize {
        agents.iter().filter(|a| self.is_member(a.id)).count()
    }
}

// ---------------------------------------------------------------------------
// Seed-ensemble statistics
// ---------------------------------------------------------------------------

fn binomial_cdf_half(n: usize, below: usize) -> f64 {
    // P(Bin(n, ½) < below)
    let mut total = 0.0;
    let mut c = 1.0_f64;
    for i in 0..below.min(n + 1) {
        if i > 0 {
            c = c * (n - i + 1) as f64 / i as f64;
        }
        total += c;
    }
    total / 2f64.powi(n as i32)
}

/// Rank `k` of the distribution-free two-sided ≥ 95 % confidence interval on
/// the median, `[x_(k), x_(n+1−k)]` (the sign test / order-statistic
/// interval — the binomial computation of #434 applied to the sign of each
/// seed's rate). `None` when no rank achieves it (`n < 6`).
pub fn median_interval_rank(n: usize) -> Option<usize> {
    (1..=n.div_ceil(2))
        .rev()
        .find(|&k| binomial_cdf_half(n, k) <= 0.025)
}

fn binomial_sf(k: usize, n: usize, p: f64) -> f64 {
    // P(X ≥ k | n, p)
    let mut total = 0.0;
    let mut c = 1.0_f64;
    for i in 0..=n {
        if i > 0 {
            c = c * (n - i + 1) as f64 / i as f64;
        }
        if i >= k {
            total += c * p.powi(i as i32) * (1.0 - p).powi((n - i) as i32);
        }
    }
    total
}

/// Exact two-sided 95 % Clopper–Pearson lower bound on a binomial proportion
/// `k/n` (bisection on the binomial tail, as #434).
pub fn clopper_pearson_lower(k: usize, n: usize) -> f64 {
    if k == 0 || n == 0 {
        return 0.0;
    }
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if binomial_sf(k, n, mid) < 0.025 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

pub fn median(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n == 0 {
        f64::NAN
    } else if n % 2 == 1 {
        sorted[n / 2]
    } else {
        0.5 * (sorted[n / 2 - 1] + sorted[n / 2])
    }
}

/// A role's invasion growth rate over the seed ensemble.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RateSummary {
    pub n: usize,
    pub median: f64,
    pub min: f64,
    pub max: f64,
    pub positive_seeds: usize,
    /// Exact 95 % Clopper–Pearson lower bound on the positive-seed fraction.
    pub positive_fraction_lower: f64,
    /// The order-statistic interval on the median (`None` below `n = 6`).
    pub interval: Option<(f64, f64)>,
    pub median_positive: bool,
    /// The criterion: median > 0 and the interval does not straddle zero.
    pub invades: bool,
}

pub fn summarise_rates(rates: &[f64]) -> RateSummary {
    let mut sorted = rates.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("finite rates"));
    let n = sorted.len();
    let median = median(&sorted);
    let positive_seeds = sorted.iter().filter(|r| **r > 0.0).count();
    let interval = median_interval_rank(n).map(|k| (sorted[k - 1], sorted[n - k]));
    let median_positive = median > 0.0;
    RateSummary {
        n,
        median,
        min: sorted.first().copied().unwrap_or(f64::NAN),
        max: sorted.last().copied().unwrap_or(f64::NAN),
        positive_seeds,
        positive_fraction_lower: clopper_pearson_lower(positive_seeds, n),
        interval,
        median_positive,
        invades: median_positive && interval.is_some_and(|(lo, _)| lo > 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn born(seq: u64, child: u64, parent: u64, mate: Option<u64>) -> Event {
        Event {
            tick: 1,
            seq,
            kind: EventKind::Born,
            source: child,
            target: Some(parent),
            energy_delta: 0.0,
            position: None,
            target_was_carcass: false,
            second_parent: mate,
        }
    }

    fn died(seq: u64, id: u64) -> Event {
        Event {
            tick: 2,
            seq,
            kind: EventKind::Died,
            source: id,
            target: None,
            energy_delta: 0.0,
            position: None,
            target_was_carcass: false,
            second_parent: None,
        }
    }

    fn roster(ids: &[u64]) -> Vec<Agent> {
        ids.iter()
            .map(|&id| {
                Agent::new(
                    id,
                    (0.0, 0.0),
                    1.0,
                    1.0,
                    0.0,
                    TraitVector {
                        photosynthetic_absorption: 0.0,
                        heterotrophy: 0.0,
                        mobility: 0.0,
                        kappa: 0.0,
                        fecundity: 0.0,
                        asexual_propensity: 0.0,
                        dispersal: 0.0,
                    },
                )
            })
            .collect()
    }

    /// The distribution-free interval on the median is the sign test's: at
    /// `n = 8` the narrowest ≥ 95 % interval is `[x_(1), x_(8)]`, so "does not
    /// straddle zero" means every seed agrees with the median's sign. Below
    /// `n = 6` no such interval exists.
    #[test]
    fn median_interval_rank_is_the_sign_tests() {
        assert_eq!(median_interval_rank(8), Some(1));
        assert_eq!(median_interval_rank(6), Some(1));
        assert_eq!(median_interval_rank(20), Some(6));
        assert_eq!(median_interval_rank(5), None);
        assert_eq!(median_interval_rank(0), None);
    }

    /// Exact two-sided 95 % Clopper–Pearson lower bound on the positive-seed
    /// fraction, pinned to the #434 table.
    #[test]
    fn clopper_pearson_lower_matches_the_434_table() {
        assert!((clopper_pearson_lower(8, 8) - 0.631).abs() < 2e-3);
        assert!((clopper_pearson_lower(7, 8) - 0.473).abs() < 2e-3);
        assert!((clopper_pearson_lower(4, 8) - 0.157).abs() < 2e-3);
        assert_eq!(clopper_pearson_lower(0, 8), 0.0);
    }

    #[test]
    fn rate_summary_reads_the_median_its_interval_and_the_positive_count() {
        let s = summarise_rates(&[0.01, 0.02, -0.005, 0.03, 0.015, 0.004, 0.02, 0.01]);
        assert_eq!(s.n, 8);
        assert_eq!(s.positive_seeds, 7);
        assert!((s.median - 0.0125).abs() < 1e-12);
        assert_eq!(s.interval, Some((-0.005, 0.03)));
        assert!(s.median_positive && !s.invades);
        let s = summarise_rates(&[0.01, 0.02, 0.005, 0.03, 0.015, 0.004, 0.02, 0.01]);
        assert_eq!(s.positive_seeds, 8);
        assert!(s.invades);
        let s = summarise_rates(&[-0.01, -0.02, -0.005, -0.03, -0.015, -0.004, -0.02, -0.01]);
        assert!(!s.median_positive && !s.invades);
        assert_eq!(s.positive_seeds, 0);
        let s = summarise_rates(&[0.01, 0.02]);
        assert_eq!(s.interval, None);
        assert!(!s.invades, "no interval at n = 2, so no verdict");
        assert!(summarise_rates(&[]).median.is_nan());
    }

    /// Founders 10 and 11 are injected into a resident world of 0..3.
    /// Descent: 10 → 20 (asexual); 11 × resident 2 → 21 (cross); 20 → 22
    /// (grandchild); resident 1 → 23 (not lineage). 10 then dies.
    #[test]
    fn lineage_follows_descent_through_either_parent_and_counts_the_living() {
        let mut lineage = Lineage::new([10, 11]);
        lineage.absorb(&[
            born(0, 20, 10, None),
            born(1, 21, 2, Some(11)),
            born(2, 23, 1, None),
        ]);
        lineage.absorb(&[born(3, 22, 20, Some(23)), died(4, 10)]);
        assert!(lineage.is_member(20) && lineage.is_member(21) && lineage.is_member(22));
        assert!(!lineage.is_member(23) && !lineage.is_member(1));
        assert_eq!(lineage.pure_births, 1);
        assert_eq!(lineage.cross_births, 2);
        // 10 is dead (off the roster); 11, 20, 21, 22 alive; 23 and residents not counted.
        let alive = lineage.alive(&roster(&[0, 1, 2, 3, 11, 20, 21, 22, 23]));
        assert_eq!(alive, 4);
    }

    fn consumed(seq: u64, eater: u64, target: u64, carcass: bool) -> Event {
        Event {
            tick: 3,
            seq,
            kind: EventKind::Consumed,
            source: eater,
            target: Some(target),
            energy_delta: 0.0,
            position: None,
            target_was_carcass: carcass,
            second_parent: None,
        }
    }

    /// The lineage's diet is read off `Consumed` events whose source is a
    /// member, split by whether the target was a carcass; residents eating
    /// (even eating a lineage member) do not count.
    #[test]
    fn lineage_counts_its_members_consumption_split_by_carcass() {
        let mut lineage = Lineage::new([10, 11]);
        lineage.absorb(&[
            consumed(0, 10, 1, false),
            consumed(1, 11, 2, true),
            consumed(2, 10, 3, true),
            consumed(3, 1, 10, false),
        ]);
        assert_eq!(lineage.consumed.living, 1);
        assert_eq!(lineage.consumed.carcass, 2);
        // Event 3 is a resident (1) draining member 10 alive.
        assert_eq!(lineage.preyed_upon_events, 1);
    }

    /// A member's death is attributed to the drain if it was drained alive in
    /// the tick it died, otherwise to starvation / the structure threshold.
    #[test]
    fn lineage_splits_deaths_by_whether_the_member_was_drained_that_tick() {
        let died = |tick: u64, seq: u64, id: u64| Event {
            tick,
            seq,
            kind: EventKind::Died,
            source: id,
            target: None,
            energy_delta: 0.0,
            position: None,
            target_was_carcass: false,
            second_parent: None,
        };
        let mut lineage = Lineage::new([10, 11, 12]);
        // 10 is drained on tick 3 and dies on tick 3; 11 is drained on tick 3
        // and dies later (tick 7) undrained; 12 is never drained.
        lineage.absorb(&[consumed(0, 1, 10, false), consumed(1, 1, 11, false)]);
        lineage.absorb(&[died(3, 2, 10)]);
        lineage.absorb(&[died(7, 0, 11), died(7, 1, 12)]);
        assert_eq!(lineage.preyed_upon_events, 2);
        assert_eq!(lineage.deaths_drained, 1);
        assert_eq!(lineage.deaths_undrained, 2);
        // A non-member's death is not counted.
        lineage.absorb(&[died(8, 0, 99)]);
        assert_eq!(lineage.deaths_drained + lineage.deaths_undrained, 3);
    }

    /// The lineage tallies the energy its members drain (a `Consumed`
    /// event's `energy_delta`), split living / carcass; a resident's drain is
    /// not counted.
    #[test]
    fn lineage_tallies_the_energy_its_members_drain_split_by_carcass() {
        let bite = |seq, eater, carcass, drain| Event {
            energy_delta: drain,
            ..consumed(seq, eater, 1, carcass)
        };
        let mut lineage = Lineage::new([10]);
        lineage.absorb(&[
            bite(0, 10, true, 0.5),
            bite(1, 10, true, 0.25),
            bite(2, 10, false, 2.0),
            bite(3, 1, true, 9.0),
        ]);
        assert_eq!(lineage.drained.carcass, 0.75);
        assert_eq!(lineage.drained.living, 2.0);
    }
}
