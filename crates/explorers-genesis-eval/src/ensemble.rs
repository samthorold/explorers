//! Seed-ensemble aggregation for the example lens (#314).
//!
//! `eval_scenarios` runs each scenario over a deterministic seed ensemble
//! (`base_seed .. base_seed + N`, mirroring genesis's `run_ensemble`) so a
//! scenario's observed evidence is a *distribution* — a robust read — rather
//! than a single-draw artifact. The dynamics of regime-sensitive scenarios
//! (example6) flip between a stable stand and overshoot-collapse on small
//! changes; one seed can hinge the whole verdict on a lucky/unlucky draw.
//!
//! This module is **prediction-agnostic and verdict-free**: it emits the
//! distribution (per-seed evidence + an aggregate). The majority/supermajority
//! read against the declared prediction lives in `verdicts.md`, not here.

use serde::Serialize;

/// The median of `values` (sorted middle, or the mean of the two middles for an
/// even count). Empty → 0.0. Lifted to match genesis's `run_ensemble` precedent
/// so the two lenses aggregate identically.
pub fn median(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

/// A score's distribution across the seed ensemble: the median (the robust
/// central read) plus its min/max range (the spread that tells you how much the
/// seed draw matters — wide spread = regime-sensitive scenario).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Spread {
    pub median: f32,
    pub min: f32,
    pub max: f32,
}

impl Spread {
    /// Summarise `values` as median + min/max. Empty → all zero.
    pub fn of(values: &[f32]) -> Self {
        if values.is_empty() {
            return Spread {
                median: 0.0,
                min: 0.0,
                max: 0.0,
            };
        }
        Spread {
            median: median(values),
            min: values.iter().copied().fold(f32::INFINITY, f32::min),
            max: values.iter().copied().fold(f32::NEG_INFINITY, f32::max),
        }
    }
}

/// The seven failure-mode names the genesis evaluator emits, in canonical order
/// (`none` first). Used for deterministic tie-breaking and to give every mode a
/// stable slot in the distribution — modes never observed still read as 0.
/// `nutrient_lockup` is appended last so the existing modes keep their canonical
/// order and the committed snapshot's tie-breaks stay stable (issue #342).
pub const FAILURE_MODES: [&str; 7] = [
    "none",
    "extinction",
    "population_explosion",
    "energy_death",
    "monoculture",
    "generalist_dominance",
    "nutrient_lockup",
];

/// How a scenario's `failure_mode` is distributed across the seed ensemble: a
/// count per mode (over the canonical seven) and the modal mode. Distribution as
/// evidence — the modal mode is the robust read, the counts show how decisive
/// (or split) the ensemble is. Verdict-free: matching this against the declared
/// prediction is `verdicts.md`'s job.
#[derive(Debug, Clone, PartialEq)]
pub struct FailureDistribution {
    /// Count per mode, indexed parallel to [`FAILURE_MODES`].
    counts: [usize; 7],
}

impl FailureDistribution {
    /// Tally an iterator of failure-mode names. Unknown names are ignored (the
    /// evaluator only ever emits the canonical seven).
    pub fn of<'a, I: IntoIterator<Item = &'a str>>(modes: I) -> Self {
        let mut counts = [0usize; 7];
        for mode in modes {
            if let Some(i) = FAILURE_MODES.iter().position(|&m| m == mode) {
                counts[i] += 1;
            }
        }
        FailureDistribution { counts }
    }

    /// How many seeds tripped `mode`.
    pub fn count(&self, mode: &str) -> usize {
        FAILURE_MODES
            .iter()
            .position(|&m| m == mode)
            .map_or(0, |i| self.counts[i])
    }

    /// The most frequent mode. Ties break toward the earlier entry in
    /// [`FAILURE_MODES`] (canonical order), so the same ensemble always yields
    /// the same modal mode and the committed snapshot stays regenerable.
    pub fn modal(&self) -> &'static str {
        let mut best = 0;
        for i in 1..FAILURE_MODES.len() {
            if self.counts[i] > self.counts[best] {
                best = i;
            }
        }
        FAILURE_MODES[best]
    }
}

impl Serialize for FailureDistribution {
    /// Serialize as a `mode -> count` map over the canonical seven, so the JSON is
    /// self-describing and stable (every mode present, unobserved ones at 0).
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(FAILURE_MODES.len()))?;
        for (i, mode) in FAILURE_MODES.iter().enumerate() {
            map.serialize_entry(mode, &self.counts[i])?;
        }
        map.end()
    }
}

/// The five sensible-world criterion scores plus `fitness`, for one seed.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SeedScores {
    pub oscillation_strength: f32,
    pub clustering_strength: f32,
    pub coexistence_duration: f32,
    pub turnover_score: f32,
    pub trophic_balance_score: f32,
    pub fitness: f32,
}

/// One seed's evidence — the row `eval_scenarios` used to emit per single-seed
/// run, now one element of the ensemble.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SeedObservation {
    pub seed: u64,
    pub failure_mode: String,
    pub ticks_survived: u64,
    pub final_population: usize,
    pub total_births: usize,
    pub total_deaths: usize,
    pub scores: SeedScores,
}

/// The per-scenario aggregate over the seed ensemble: a failure-mode
/// distribution + modal mode, median/spread of every score and demographic
/// count, the seed set used (for reproducibility), and the per-seed breakdown.
///
/// Distribution as evidence — this carries no pass/fail and reads no declared
/// prediction. The verdict (majority/supermajority against the prediction) is
/// `verdicts.md`'s job.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenarioAggregate {
    pub scenario: String,
    pub max_ticks: u64,
    /// The ensemble's base seed; runs cover `base_seed .. base_seed + seeds`.
    pub base_seed: u64,
    /// Ensemble size.
    pub seeds: u64,
    pub modal_failure_mode: &'static str,
    pub failure_distribution: FailureDistribution,
    pub oscillation_strength: Spread,
    pub clustering_strength: Spread,
    pub coexistence_duration: Spread,
    pub turnover_score: Spread,
    pub trophic_balance_score: Spread,
    pub fitness: Spread,
    pub ticks_survived: Spread,
    pub final_population: Spread,
    pub total_births: Spread,
    pub total_deaths: Spread,
    pub per_seed: Vec<SeedObservation>,
}

/// Aggregate a scenario's per-seed evidence into its ensemble distribution.
/// Pure: same rows → same aggregate.
pub fn aggregate(
    scenario: &str,
    max_ticks: u64,
    base_seed: u64,
    per_seed: Vec<SeedObservation>,
) -> ScenarioAggregate {
    let failure_distribution =
        FailureDistribution::of(per_seed.iter().map(|r| r.failure_mode.as_str()));
    let modal_failure_mode = failure_distribution.modal();

    let spread = |f: &dyn Fn(&SeedObservation) -> f32| {
        Spread::of(&per_seed.iter().map(f).collect::<Vec<_>>())
    };

    ScenarioAggregate {
        scenario: scenario.to_string(),
        max_ticks,
        base_seed,
        seeds: per_seed.len() as u64,
        modal_failure_mode,
        failure_distribution,
        oscillation_strength: spread(&|r| r.scores.oscillation_strength),
        clustering_strength: spread(&|r| r.scores.clustering_strength),
        coexistence_duration: spread(&|r| r.scores.coexistence_duration),
        turnover_score: spread(&|r| r.scores.turnover_score),
        trophic_balance_score: spread(&|r| r.scores.trophic_balance_score),
        fitness: spread(&|r| r.scores.fitness),
        ticks_survived: spread(&|r| r.ticks_survived as f32),
        final_population: spread(&|r| r.final_population as f32),
        total_births: spread(&|r| r.total_births as f32),
        total_deaths: spread(&|r| r.total_deaths as f32),
        per_seed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_of_odd_count() {
        assert_eq!(median(&[3.0, 1.0, 2.0]), 2.0);
    }

    #[test]
    fn median_of_even_count() {
        assert_eq!(median(&[4.0, 1.0, 3.0, 2.0]), 2.5);
    }

    #[test]
    fn median_of_empty_is_zero() {
        assert_eq!(median(&[]), 0.0);
    }

    #[test]
    fn spread_reports_median_min_max() {
        let s = Spread::of(&[3.0, 1.0, 2.0, 5.0, 4.0]);
        assert_eq!(s.median, 3.0);
        assert_eq!(s.min, 1.0);
        assert_eq!(s.max, 5.0);
    }

    #[test]
    fn spread_of_empty_is_all_zero() {
        let s = Spread::of(&[]);
        assert_eq!(s.median, 0.0);
        assert_eq!(s.min, 0.0);
        assert_eq!(s.max, 0.0);
    }

    #[test]
    fn failure_distribution_counts_each_mode() {
        let modes = ["none", "extinction", "none", "energy_death", "none"];
        let dist = FailureDistribution::of(modes.iter().copied());
        assert_eq!(dist.count("none"), 3);
        assert_eq!(dist.count("extinction"), 1);
        assert_eq!(dist.count("energy_death"), 1);
        assert_eq!(dist.count("monoculture"), 0);
    }

    #[test]
    fn failure_distribution_modal_is_most_frequent() {
        let modes = ["none", "extinction", "none", "energy_death", "none"];
        let dist = FailureDistribution::of(modes.iter().copied());
        assert_eq!(dist.modal(), "none");
    }

    #[test]
    fn failure_distribution_modal_breaks_ties_deterministically() {
        // 2 vs 2: the same input must always yield the same modal mode so the
        // committed snapshot is regenerable. We break ties by the genesis
        // evaluator's canonical mode order (none < extinction < ...).
        let modes = ["extinction", "none", "extinction", "none"];
        let dist = FailureDistribution::of(modes.iter().copied());
        let dist2 =
            FailureDistribution::of(["none", "extinction", "none", "extinction"].iter().copied());
        assert_eq!(dist.modal(), dist2.modal());
        assert_eq!(dist.modal(), "none");
    }

    fn seed_obs(seed: u64, failure: &str, fitness: f32, pop: usize) -> SeedObservation {
        SeedObservation {
            seed,
            failure_mode: failure.to_string(),
            ticks_survived: 100,
            final_population: pop,
            total_births: 10,
            total_deaths: 8,
            scores: SeedScores {
                oscillation_strength: 0.0,
                clustering_strength: 0.0,
                coexistence_duration: 0.0,
                turnover_score: 0.0,
                trophic_balance_score: 0.0,
                fitness,
            },
        }
    }

    #[test]
    fn aggregate_records_seed_set_and_per_seed_rows() {
        let rows = vec![
            seed_obs(1, "none", 0.5, 11),
            seed_obs(2, "extinction", 0.0, 0),
        ];
        let agg = aggregate("example6.json", 2000, 1, rows.clone());
        assert_eq!(agg.scenario, "example6.json");
        assert_eq!(agg.max_ticks, 2000);
        assert_eq!(agg.base_seed, 1);
        assert_eq!(agg.seeds, 2);
        assert_eq!(agg.per_seed, rows);
    }

    #[test]
    fn aggregate_takes_modal_failure_and_median_fitness() {
        let rows = vec![
            seed_obs(1, "none", 0.6, 11),
            seed_obs(2, "none", 0.4, 9),
            seed_obs(3, "extinction", 0.0, 0),
        ];
        let agg = aggregate("s.json", 100, 1, rows);
        assert_eq!(agg.modal_failure_mode, "none");
        assert_eq!(agg.failure_distribution.count("none"), 2);
        assert_eq!(agg.failure_distribution.count("extinction"), 1);
        // median fitness of [0.6, 0.4, 0.0] = 0.4
        assert_eq!(agg.fitness.median, 0.4);
        assert_eq!(agg.fitness.min, 0.0);
        assert_eq!(agg.fitness.max, 0.6);
    }

    #[test]
    fn aggregate_spreads_demographics() {
        let rows = vec![
            seed_obs(1, "none", 0.5, 11),
            seed_obs(2, "none", 0.5, 9),
            seed_obs(3, "none", 0.5, 7),
        ];
        let agg = aggregate("s.json", 100, 1, rows);
        assert_eq!(agg.final_population.median, 9.0);
        assert_eq!(agg.final_population.min, 7.0);
        assert_eq!(agg.final_population.max, 11.0);
    }
}
