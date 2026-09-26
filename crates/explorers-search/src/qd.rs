//! Quality-Diversity (CMA-MAE) outer search — the genesis *illumination* loop.
//!
//! This is the production form of the QD atlas the #363 spike validated (verdict
//! GO, merged #364). It **illuminates** world-parameter space into an
//! [`Atlas`](crate::qd::Atlas): a soft-archive of the best surviving worlds binned
//! on the three behaviour axes, plus a dead frontier keyed by which cliff a
//! config died on. See `docs/system-design/genesis-search.md` for the contract.
//!
//! ## What it reuses unchanged (the guardrail: call the pipeline, don't fork it)
//!
//! - [`crate::search::decode`] — the `[0,1]^d` → `(WorldParameters,
//!   InitialDistribution)` decoder. Every point the emitter proposes is fed to
//!   `decode` verbatim.
//! - [`explorers_genesis::run_ensemble`] — the unchanged seed-ensemble rollout.
//!   The three behaviour axes and the decomposer-guild signal now ride on the
//!   per-seed [`FitnessBreakdown`] (issue #365), so the descriptors are read off
//!   `run_ensemble`'s output — no `run_single` mirror. The search calls it
//!   under a per-rollout wall-clock budget (`run_ensemble_within`, #562).
//!
//! ## Rollout budgets and reproducibility
//!
//! Every seed rollout, in the search and in refinement, runs under a
//! [`RolloutBudget`] with the research sweeps' semantics
//! (`--run-timeout-secs` / `--eval-timeout-secs`). A seed that exhausts it is
//! unfinished: it enters neither the archive nor the dead frontier and is only
//! counted. The search is deterministic in `(config, base_seed, rng)` exactly
//! while no budget fires; wall clock decides which seeds are unfinished, so a
//! budget that fires breaks bit-reproducibility, and a resumed search makes
//! the same decisions only when budgets are unhit.
//!
//! ## Soft archive (CMA-MAE)
//!
//! Each filled cell keeps an elite *and* a rolling acceptance threshold. A new
//! solution is accepted into a cell when its fitness clears that threshold (not
//! merely the sitting elite), and the threshold is then nudged toward the new
//! fitness by an archive learning rate. This tolerates the descriptor noise the
//! design names (a cell's elite is a noisy median-over-seeds) instead of letting
//! one lucky draw stick (genesis-search.md, "soft per-cell acceptance threshold").
//!
//! ## Covariance-adapting emitter
//!
//! A separable CMA-style emitter maintains a mean and a per-dimension variance,
//! sampling each batch around the mean and adapting the mean + variances toward
//! the batch's *improving* solutions. Covariance adaptation learns the relevant
//! subspace as it moves, retiring the Sobol dimension-fixing prefilter the GP-BO
//! incumbent needed (genesis-search.md).

use rand::Rng;

use explorers_genesis::{
    EnsembleConfig, EnsembleResult, EvalConfig, FailureMode, FitnessBreakdown, RolloutBudget,
    RunConfig, RunResult, run_ensemble_within,
};
use explorers_sim::WorldRecipe;

use crate::bifurcation::{branching_distance, oscillation_distance};
use crate::prefilter::prefilter_cliff;
use crate::search::{ParameterRange, SearchBoxMismatch, check_search_box, decode, default_ranges};

/// Bins per behaviour axis. Coarse, per the spike (20×20×20).
pub const RESOLUTION: usize = 20;

/// The six terminal cliffs a config can die on — the dead-frontier key. A gated
/// config gets no behaviour cell (its descriptors are degenerate); it is tallied
/// here instead. This tally is the atlas's dead-frontier layer.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum Cliff {
    Extinction,
    PopulationExplosion,
    EnergyDeath,
    NutrientLockup,
    Monoculture,
    GeneralistDominance,
}

impl Cliff {
    pub fn from_failure(f: &FailureMode) -> Self {
        match f {
            FailureMode::Extinction => Cliff::Extinction,
            FailureMode::PopulationExplosion => Cliff::PopulationExplosion,
            FailureMode::EnergyDeath => Cliff::EnergyDeath,
            FailureMode::NutrientLockup => Cliff::NutrientLockup,
            FailureMode::Monoculture => Cliff::Monoculture,
            FailureMode::GeneralistDominance => Cliff::GeneralistDominance,
        }
    }

    /// The canonical snake_case label (matches the evaluator's failure-mode names).
    pub fn label(&self) -> &'static str {
        match self {
            Cliff::Extinction => "extinction",
            Cliff::PopulationExplosion => "population_explosion",
            Cliff::EnergyDeath => "energy_death",
            Cliff::NutrientLockup => "nutrient_lockup",
            Cliff::Monoculture => "monoculture",
            Cliff::GeneralistDominance => "generalist_dominance",
        }
    }
}

/// The three behaviour-axis coordinates of a config, each in `[0, 1]`.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Descriptors {
    /// Frozen ↔ oscillation axis (`FitnessBreakdown::oscillation_strength`).
    pub oscillation: f32,
    /// Monoculture ↔ coexistence axis (`FitnessBreakdown::clustering_strength`).
    pub clustering: f32,
    /// Healthy-throughput ↔ nutrient-lockup axis
    /// (`FitnessBreakdown::carcass_locked_fraction`).
    pub carcass: f32,
}

/// Hard-bin a descriptor triple into a `RESOLUTION³` cell index. Inputs are
/// clamped to `[0, 1]`.
pub fn cell_of(d: &Descriptors) -> (usize, usize, usize) {
    let bin = |x: f32| -> usize {
        let c = x.clamp(0.0, 1.0);
        ((c * RESOLUTION as f32) as usize).min(RESOLUTION - 1)
    };
    (bin(d.oscillation), bin(d.clustering), bin(d.carcass))
}

/// One config's ensemble verdict, reduced exactly as the incumbent reduced
/// (median fitness over the seed ensemble). The median-fitness seed decides
/// dead-vs-live and supplies the descriptors; the decomposer fraction is the
/// share of the ensemble that sprouted a persistent guild.
#[derive(Clone, Debug)]
pub struct ConfigEval {
    pub median_fitness: f32,
    /// `Some(cliff)` if the median-fitness seed is gated (→ dead frontier),
    /// `None` if it is a live world (→ a behaviour cell).
    pub cliff: Option<Cliff>,
    /// Descriptors of the median-fitness seed (meaningful only when `cliff` is
    /// `None`).
    pub descriptors: Descriptors,
    /// Fraction of the ensemble whose seed read a decomposer guild — a sustained,
    /// recruiting population (`has_decomposer_guild`, #490). A reported
    /// distribution, never an axis or fitness term.
    pub decomposer_fraction: f32,
    /// The consumer twin of `decomposer_fraction` (`has_consumer_guild`, #490):
    /// same computation, same authority boundary.
    pub consumer_fraction: f32,
    /// Fraction of the ensemble that lands in the coexisting regime — alive and
    /// either clustering or coexisting (the #359 small-N disjunction). A reported
    /// per-seed distribution under the *same* authority boundary as
    /// `decomposer_fraction`: never an axis, never a fitness term. It feeds the
    /// projection's robustness floor only (genesis-search.md).
    pub coexistence_fraction: f32,
    /// The per-cell sample count (the seed-ensemble size).
    pub sample_count: u32,
    /// Observed coexistence duration of the median seed
    /// ([`FitnessBreakdown::coexistence_duration`]) — carried only to tag the
    /// branching cross-check's regime (the #359 small-N borderline signature).
    pub coexistence_duration: f32,
    /// Predicted signed distance to the frozen↔oscillation (Hopf) boundary of the
    /// living-mass↔available-pool coupling ([`crate::bifurcation::oscillation_distance`]),
    /// computed once from the decoded `(WorldParameters, founder mean)`. A
    /// **descriptor**, never summed into fitness nor binned on. 0 on the gated
    /// (degenerate) path.
    pub predicted_oscillation_distance: f32,
    /// Predicted signed distance-to-branching `D` (the monoculture↔coexistence
    /// invasion margin, [`crate::bifurcation::branching_distance`]), computed once
    /// from the decoded `(WorldParameters, founder mean)`. A **descriptor**, never
    /// summed into fitness nor binned on. 0 on the gated path.
    pub predicted_branching_distance: f32,
    /// The early-stop cross-check read off this ensemble (#506): carried seeds
    /// and their disagreements. Surfaced on the atlas, never a cell property.
    pub early_stop_crosscheck: EarlyStopCrosscheck,
}

/// One seed lands in the coexisting regime when it is alive (no failure mode) and
/// the clustering/coexistence observables read positive. The `||` is the #359
/// small-N disjunction: below n≈4 the multi-peak `clustering_strength` silently
/// zeroes even while coexistence genuinely persists, so a positive
/// `coexistence_duration` still counts the seed as coexisting and the fraction
/// does not under-count.
fn is_coexisting(r: &RunResult) -> bool {
    r.failure.is_none()
        && (r.breakdown.clustering_strength > 0.0 || r.breakdown.coexistence_duration > 0.0)
}

/// Which per-seed predicate the projection's coexistence floor reads (#538).
/// `Plain` is [`is_coexisting`], the default. The guild-aware variants also
/// require the seed to hold a heterotroph guild (#490): #494's option 3,
/// "fold guild presence into coexisting", as a **projection-only** comparison
/// instrument. Never a fitness term, never a binning axis (genesis-search.md,
/// *Authority boundary*).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub enum CoexistenceFloor {
    #[default]
    Plain,
    Decomposer,
    Consumer,
    Either,
}

impl CoexistenceFloor {
    pub const ALL: [CoexistenceFloor; 4] = [
        CoexistenceFloor::Plain,
        CoexistenceFloor::Decomposer,
        CoexistenceFloor::Consumer,
        CoexistenceFloor::Either,
    ];

    /// The CLI spelling (`--coexistence-floor`).
    pub fn label(self) -> &'static str {
        match self {
            CoexistenceFloor::Plain => "plain",
            CoexistenceFloor::Decomposer => "decomposer",
            CoexistenceFloor::Consumer => "consumer",
            CoexistenceFloor::Either => "either",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.label() == label)
    }

    fn holds(self, r: &RunResult) -> bool {
        let b = &r.breakdown;
        is_coexisting(r)
            && match self {
                CoexistenceFloor::Plain => true,
                CoexistenceFloor::Decomposer => b.has_decomposer_guild,
                CoexistenceFloor::Consumer => b.has_consumer_guild,
                CoexistenceFloor::Either => b.has_decomposer_guild || b.has_consumer_guild,
            }
    }
}

/// An ensemble's coexistence fraction under every [`CoexistenceFloor`], read
/// off the same seeds — so one refinement lays out every floor, and no
/// guild-aware fraction can exceed the plain one.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub struct CoexistenceFractions {
    pub plain: f32,
    pub decomposer: f32,
    pub consumer: f32,
    pub either: f32,
}

impl CoexistenceFractions {
    pub fn of_seeds(run_results: &[RunResult]) -> Self {
        let fraction = |floor: CoexistenceFloor| {
            if run_results.is_empty() {
                0.0
            } else {
                run_results.iter().filter(|r| floor.holds(r)).count() as f32
                    / run_results.len() as f32
            }
        };
        CoexistenceFractions {
            plain: fraction(CoexistenceFloor::Plain),
            decomposer: fraction(CoexistenceFloor::Decomposer),
            consumer: fraction(CoexistenceFloor::Consumer),
            either: fraction(CoexistenceFloor::Either),
        }
    }

    pub fn under(&self, floor: CoexistenceFloor) -> f32 {
        match floor {
            CoexistenceFloor::Plain => self.plain,
            CoexistenceFloor::Decomposer => self.decomposer,
            CoexistenceFloor::Consumer => self.consumer,
            CoexistenceFloor::Either => self.either,
        }
    }
}

/// Reduce a `run_ensemble` result to a [`ConfigEval`], reading the three axes and
/// the decomposer signal off the per-seed [`FitnessBreakdown`] — no `run_single`
/// mirror. The median-fitness seed (lower-middle for an even ensemble) is the
/// representative, matching the incumbent's median reduction.
pub fn config_eval_from_ensemble(result: &EnsembleResult) -> ConfigEval {
    let sample_count = result.run_results.len() as u32;
    let seed_fraction = |read: fn(&FitnessBreakdown) -> bool| {
        if sample_count == 0 {
            0.0
        } else {
            result
                .run_results
                .iter()
                .filter(|r| read(&r.breakdown))
                .count() as f32
                / sample_count as f32
        }
    };
    let decomposer_fraction = seed_fraction(|b| b.has_decomposer_guild);
    let consumer_fraction = seed_fraction(|b| b.has_consumer_guild);
    // Reported per-seed distribution (never an axis, never fitness): the share of
    // the ensemble that lands in the coexisting regime. The projection's
    // robustness floor reads this; binning and fitness do not.
    let coexistence_fraction = CoexistenceFractions::of_seeds(&result.run_results).plain;

    // Order the seeds by fitness; the lower-middle element is the representative
    // (the median seed). An empty ensemble degenerates to a zero-fitness extinct
    // verdict so the archive simply ignores it.
    let mut idx: Vec<usize> = (0..result.run_results.len()).collect();
    idx.sort_by(|&a, &b| {
        result.run_results[a]
            .fitness
            .partial_cmp(&result.run_results[b].fitness)
            .unwrap()
    });
    if idx.is_empty() {
        return ConfigEval {
            median_fitness: 0.0,
            cliff: Some(Cliff::Extinction),
            descriptors: Descriptors {
                oscillation: 0.0,
                clustering: 0.0,
                carcass: 0.0,
            },
            decomposer_fraction: 0.0,
            consumer_fraction: 0.0,
            coexistence_fraction: 0.0,
            sample_count: 0,
            coexistence_duration: 0.0,
            predicted_oscillation_distance: 0.0,
            predicted_branching_distance: 0.0,
            early_stop_crosscheck: EarlyStopCrosscheck::default(),
        };
    }
    let rep = &result.run_results[idx[idx.len() / 2]];
    ConfigEval {
        median_fitness: rep.fitness,
        cliff: rep.failure.as_ref().map(Cliff::from_failure),
        descriptors: Descriptors {
            oscillation: rep.breakdown.oscillation_strength,
            clustering: rep.breakdown.clustering_strength,
            carcass: rep.breakdown.carcass_locked_fraction,
        },
        decomposer_fraction,
        consumer_fraction,
        coexistence_fraction,
        sample_count,
        coexistence_duration: rep.breakdown.coexistence_duration,
        // Predicted bifurcation coordinates are filled by the caller, which holds
        // the decoded `(WorldParameters, founder mean)`; the ensemble result alone
        // cannot compute them. Default 0 until then.
        predicted_oscillation_distance: 0.0,
        predicted_branching_distance: 0.0,
        early_stop_crosscheck: EarlyStopCrosscheck::default(),
    }
}

/// One filled cell of the soft archive: the current elite plus the rolling
/// acceptance threshold that makes the cell descriptor-noise tolerant (CMA-MAE).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CellRecord {
    pub fitness: f32,
    pub descriptors: Descriptors,
    pub unit: Vec<f64>,
    pub decomposer_fraction: f32,
    pub consumer_fraction: f32,
    /// Reported coexistence fraction of the cell's seed ensemble (see
    /// [`ConfigEval::coexistence_fraction`]) — read by the projection's robustness
    /// floor only, never binned on nor summed into fitness.
    pub coexistence_fraction: f32,
    pub sample_count: u32,
    /// Predicted bifurcation descriptors of the elite (see [`ConfigEval`]) —
    /// reported on the cell, never binned on nor summed into fitness.
    pub predicted_oscillation_distance: f32,
    pub predicted_branching_distance: f32,
    /// The cell's rolling acceptance threshold. A new solution is accepted when
    /// its fitness clears this (not merely the sitting elite); the threshold is
    /// then nudged toward the accepted fitness by the archive learning rate.
    threshold: f32,
}

/// The soft (CMA-MAE) MAP-Elites archive over the three behaviour axes, plus the
/// dead frontier. Live configs fill cells under a rolling per-cell acceptance
/// threshold; gated configs are tallied to the frontier by cliff.
#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Archive {
    #[serde(with = "sorted_entries")]
    cells: std::collections::HashMap<(usize, usize, usize), CellRecord>,
    /// Observed deaths: configs that cleared the prefilter (or were never
    /// prefiltered) and died in their seed-ensemble rollout, keyed by cliff.
    #[serde(with = "sorted_entries")]
    frontier: std::collections::HashMap<Cliff, usize>,
    /// A priori deaths: configs the viability prefilter proved dead in closed
    /// form, routed here without spending an ensemble (`crate::prefilter`). The
    /// two tallies are kept apart so the atlas can distinguish a-priori from
    /// observed deaths (genesis-search.md, "the dead frontier is the atlas's most
    /// valuable layer").
    #[serde(with = "sorted_entries")]
    apriori_frontier: std::collections::HashMap<Cliff, usize>,
    /// Archive learning rate (CMA-MAE α): how far each accepted solution drags
    /// the cell's acceptance threshold toward its fitness. 0 ⇒ classic hard
    /// elitism (threshold tracks the elite); 1 ⇒ threshold jumps to each elite.
    archive_learning_rate: f32,
}

impl Archive {
    pub fn new(archive_learning_rate: f32) -> Self {
        Archive {
            cells: std::collections::HashMap::new(),
            frontier: std::collections::HashMap::new(),
            apriori_frontier: std::collections::HashMap::new(),
            archive_learning_rate: archive_learning_rate.clamp(0.0, 1.0),
        }
    }

    /// Tally an **a priori** death: a config the viability prefilter proved dead
    /// in closed form, routed to the dead frontier without spending an ensemble.
    pub fn insert_apriori(&mut self, cliff: Cliff) {
        *self.apriori_frontier.entry(cliff).or_insert(0) += 1;
    }

    /// Place one evaluated config. Gated → frontier tally; live → soft-archive
    /// cell. Returns the *improvement* over the cell's acceptance threshold the
    /// config achieved (0 if rejected or gated) — the signal the CMA-MAE emitter
    /// ranks its batch on.
    pub fn insert(&mut self, unit: &[f64], eval: &ConfigEval) -> f32 {
        match eval.cliff {
            Some(cliff) => {
                *self.frontier.entry(cliff).or_insert(0) += 1;
                0.0
            }
            None => {
                let cell = cell_of(&eval.descriptors);
                match self.cells.get_mut(&cell) {
                    None => {
                        // First occupant: it defines the cell, threshold starts at
                        // its own fitness.
                        self.cells.insert(
                            cell,
                            CellRecord {
                                fitness: eval.median_fitness,
                                descriptors: eval.descriptors,
                                unit: unit.to_vec(),
                                decomposer_fraction: eval.decomposer_fraction,
                                consumer_fraction: eval.consumer_fraction,
                                coexistence_fraction: eval.coexistence_fraction,
                                sample_count: eval.sample_count,
                                predicted_oscillation_distance: eval.predicted_oscillation_distance,
                                predicted_branching_distance: eval.predicted_branching_distance,
                                threshold: eval.median_fitness,
                            },
                        );
                        eval.median_fitness.max(0.0)
                    }
                    Some(rec) => {
                        let improvement = eval.median_fitness - rec.threshold;
                        if improvement > 0.0 {
                            // Soft acceptance: clears the rolling threshold. Raise
                            // the threshold toward the accepted fitness; replace the
                            // sitting elite if this also beats it (the elite tracks
                            // the best seen, the threshold lags it by α).
                            rec.threshold +=
                                self.archive_learning_rate * (eval.median_fitness - rec.threshold);
                            if eval.median_fitness > rec.fitness {
                                rec.fitness = eval.median_fitness;
                                rec.descriptors = eval.descriptors;
                                rec.unit = unit.to_vec();
                                rec.decomposer_fraction = eval.decomposer_fraction;
                                rec.consumer_fraction = eval.consumer_fraction;
                                rec.coexistence_fraction = eval.coexistence_fraction;
                                rec.sample_count = eval.sample_count;
                                rec.predicted_oscillation_distance =
                                    eval.predicted_oscillation_distance;
                                rec.predicted_branching_distance =
                                    eval.predicted_branching_distance;
                            }
                            improvement
                        } else {
                            0.0
                        }
                    }
                }
            }
        }
    }

    /// Filled-cell count.
    pub fn coverage(&self) -> usize {
        self.cells.len()
    }

    /// QD-score: the sum of elite fitnesses over filled cells, accumulated in
    /// cell-index order so the f32 rounding is reproducible across runs (a
    /// `HashMap` iteration order is not stable, and a sum over it is not either).
    pub fn qd_score(&self) -> f32 {
        let mut keyed: Vec<(&(usize, usize, usize), f32)> =
            self.cells.iter().map(|(k, c)| (k, c.fitness)).collect();
        keyed.sort_by_key(|(k, _)| **k);
        keyed.into_iter().map(|(_, f)| f).sum()
    }

    pub fn best_fitness(&self) -> f32 {
        self.cells.values().map(|c| c.fitness).fold(0.0, f32::max)
    }

    /// Iterate the filled cells with their indices.
    pub fn cells(&self) -> impl Iterator<Item = (&(usize, usize, usize), &CellRecord)> {
        self.cells.iter()
    }

    /// The dead frontier as a label-keyed tally — the count of configs that died
    /// on each cliff, **a priori and observed combined**. The atlas's
    /// negative-space layer.
    pub fn dead_frontier(&self) -> std::collections::BTreeMap<String, usize> {
        let mut out: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        for (k, &v) in self.frontier.iter().chain(self.apriori_frontier.iter()) {
            *out.entry(k.label().to_string()).or_insert(0) += v;
        }
        out
    }

    /// The **a priori** layer of the dead frontier: configs the viability
    /// prefilter proved dead in closed form, keyed by cliff label. These spent no
    /// ensemble. The complement (`dead_frontier` minus this) is the observed
    /// deaths.
    pub fn dead_frontier_apriori(&self) -> std::collections::BTreeMap<String, usize> {
        self.apriori_frontier
            .iter()
            .map(|(k, &v)| (k.label().to_string(), v))
            .collect()
    }

    /// The unit-cube elites currently in the archive (the emitter's parent pool),
    /// in a deterministic cell-index order so random parent selection is
    /// reproducible across runs (a `HashMap` iteration order is not stable).
    fn elite_units(&self) -> Vec<Vec<f64>> {
        let mut keyed: Vec<(&(usize, usize, usize), &CellRecord)> = self.cells.iter().collect();
        keyed.sort_by_key(|(k, _)| **k);
        keyed.into_iter().map(|(_, c)| c.unit.clone()).collect()
    }
}

/// Serialise a `HashMap` as its entries sorted by key, so the written form is
/// deterministic (a `HashMap` iteration order is not) and needs no string keys.
mod sorted_entries {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;
    use std::hash::Hash;

    pub fn serialize<K, V, S>(map: &HashMap<K, V>, s: S) -> Result<S::Ok, S::Error>
    where
        K: Ord + Serialize,
        V: Serialize,
        S: Serializer,
    {
        let mut entries: Vec<(&K, &V)> = map.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        entries.serialize(s)
    }

    pub fn deserialize<'de, K, V, D>(d: D) -> Result<HashMap<K, V>, D::Error>
    where
        K: Eq + Hash + Deserialize<'de>,
        V: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        Ok(Vec::<(K, V)>::deserialize(d)?.into_iter().collect())
    }
}

// ---------------------------------------------------------------------------
// Carcass-directed seeding (reaching the nutrient-lockup region)
// ---------------------------------------------------------------------------

/// A unit-cube seed point biased toward the **high-carcass / nutrient-lockup**
/// region of the atlas. The nutrient-lockup cliff cannot be prefiltered, so the
/// only way to populate the atlas's lockup layer is to *run* worlds that strand
/// their nutrient in the dead pool — and a random emitter never lands in that thin
/// region (the #363 spike: 0 lockup-frontier configs). This seed directs the
/// search there by setting the dims that empirically drive dead-pool accumulation
/// to their carcass-favouring extremes (genesis-search.md, "reaching that thin
/// high-carcass region is the emitter's job, directed exploration along the carcass
/// axis").
///
/// The drivers (found by sweeping `decode`'s dims against the carcass-locked
/// fraction): maximal photosynthetic production feeding biomass, **zero
/// heterotrophy** so no decomposer turns the dead pool over, a high
/// nutrient-per-structure ratio so the death flux strands nutrient in carcasses,
/// superlinear maintenance + high metabolic cost to pump biomass into death, low
/// kappa (little standing structure to retain nutrient in the living pool) and a
/// large founder population to drain the grid nutrient into agents that then die.
/// Dims `decode` does not read are left at the cube midpoint. Returns a point in
/// `[0, 1]^d`; the search feeds it to `decode` verbatim like any other proposal.
pub fn carcass_seed_unit(ranges: &[ParameterRange]) -> Vec<f64> {
    let mut unit = vec![0.5_f64; ranges.len()];
    let mut set = |name: &str, v: f64| {
        if let Some(i) = ranges.iter().position(|r| r.name == name) {
            unit[i] = v.clamp(0.0, 1.0);
        }
    };
    // Production on, decomposition off — the dead pool fills and nothing recovers it.
    set("mean_photosynthetic_absorption", 1.0);
    set("mean_heterotrophy", 0.0);
    // Nutrient rides into structure (and thus carcasses) at a high ratio.
    set("base_nutrient_ratio", 1.0);
    set("specification_nutrient_coefficient", 1.0);
    // Pump biomass into death: superlinear maintenance + high metabolic burn.
    set("maintenance_cost_exponent", 1.0);
    set("base_metabolic_rate", 0.9);
    // Little standing structure in the living pool; many founders to drain the grid.
    set("mean_kappa", 0.05);
    set("initial_population_size", 1.0);
    unit
}

/// Replace the head of a bootstrap `batch` with `count` carcass-directed seeds
/// ([`carcass_seed_unit`]), leaving the random tail intact. This is how carcass
/// direction reaches the lockup region: a handful of guaranteed high-carcass
/// starting points the covariance-adapting emitter then breeds *toward* the cliff,
/// while the rest of the batch keeps illuminating the live manifold. `count` is
/// clamped to the batch length; 0 disables direction (an untouched random batch).
fn inject_carcass_seeds(batch: &mut [Vec<f64>], ranges: &[ParameterRange], count: usize) {
    let seed = carcass_seed_unit(ranges);
    for slot in batch.iter_mut().take(count) {
        *slot = seed.clone();
    }
}

// ---------------------------------------------------------------------------
// Covariance-adapting emitter (separable CMA-ME)
// ---------------------------------------------------------------------------

/// A separable (diagonal-covariance) CMA-style emitter. It maintains a mean and a
/// per-dimension standard deviation in unit-cube coordinates, samples each batch
/// around the mean, and adapts the mean + per-dimension deviations toward the
/// batch's improving solutions (ranked by archive improvement). The per-dimension
/// adaptation *is* the covariance learning that retires the Sobol dimension-fixing
/// prefilter: dimensions whose spread among improvers is wide stay explored,
/// dimensions that converge shrink — the search learns which axes matter, rather
/// than fixing the "unimportant" ones once up front.
#[derive(serde::Serialize, serde::Deserialize)]
struct CmaEmitter {
    dims: usize,
    mean: Vec<f64>,
    /// Per-dimension standard deviation (the diagonal of the covariance).
    sigma: Vec<f64>,
    /// Learning rate for the mean update toward the improver centroid.
    mean_lr: f64,
    /// Learning rate for the per-dimension deviation update.
    sigma_lr: f64,
    /// Floor on each deviation so the emitter never fully collapses (keeps
    /// illuminating).
    sigma_floor: f64,
}

impl CmaEmitter {
    fn new(dims: usize, initial_sigma: f64) -> Self {
        CmaEmitter {
            dims,
            mean: vec![0.5; dims],
            sigma: vec![initial_sigma; dims],
            mean_lr: 0.5,
            sigma_lr: 0.2,
            sigma_floor: 0.02,
        }
    }

    /// Sample one unit-cube point from the current Gaussian, clamped to `[0,1]`.
    fn sample(&self, rng: &mut impl Rng) -> Vec<f64> {
        (0..self.dims)
            .map(|d| (self.mean[d] + self.sigma[d] * gaussian(rng)).clamp(0.0, 1.0))
            .collect()
    }

    /// Adapt the mean and per-dimension deviations toward the improving members of
    /// the just-evaluated batch. `improvements[i]` is the archive improvement of
    /// `batch[i]` (≤ 0 means it did not improve any cell). Only positive-
    /// improvement members steer the update; if none improved, the emitter widens
    /// slightly to escape the stagnant region.
    fn adapt(&mut self, batch: &[Vec<f64>], improvements: &[f32]) {
        let improvers: Vec<usize> = (0..batch.len())
            .filter(|&i| improvements[i] > 0.0)
            .collect();

        if improvers.is_empty() {
            // No improvement: re-inflate the deviations a touch so the next batch
            // explores wider (the restart pressure that keeps QD illuminating).
            for s in &mut self.sigma {
                *s = (*s * 1.1).min(0.5);
            }
            return;
        }

        // Improvement-weighted centroid → new mean.
        let total: f32 = improvers.iter().map(|&i| improvements[i]).sum();
        let mut centroid = vec![0.0_f64; self.dims];
        for &i in &improvers {
            let w = (improvements[i] / total) as f64;
            for d in 0..self.dims {
                centroid[d] += w * batch[i][d];
            }
        }
        for d in 0..self.dims {
            self.mean[d] += self.mean_lr * (centroid[d] - self.mean[d]);
            self.mean[d] = self.mean[d].clamp(0.0, 1.0);
        }

        // Per-dimension spread of the improvers about the new mean → new sigma.
        for d in 0..self.dims {
            let mut var = 0.0_f64;
            for &i in &improvers {
                let w = (improvements[i] / total) as f64;
                let delta = batch[i][d] - self.mean[d];
                var += w * delta * delta;
            }
            let target = var.sqrt().max(self.sigma_floor);
            self.sigma[d] += self.sigma_lr * (target - self.sigma[d]);
            self.sigma[d] = self.sigma[d].max(self.sigma_floor);
        }
    }
}

/// Standard normal via Box-Muller (avoids adding a `rand_distr` dependency to the
/// search crate).
fn gaussian(rng: &mut impl Rng) -> f64 {
    let u1: f64 = rng.random_range(f64::MIN_POSITIVE..1.0);
    let u2: f64 = rng.random_range(0.0..1.0);
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

// ---------------------------------------------------------------------------
// Atlas — the search output
// ---------------------------------------------------------------------------

/// A single live cell of the atlas, serialised for the search output.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AtlasCell {
    /// The behaviour-axis cell index `(oscillation, clustering, carcass)`.
    pub cell: [usize; 3],
    pub fitness: f32,
    pub oscillation: f32,
    pub clustering: f32,
    pub carcass: f32,
    /// Per-cell decomposer-guild distribution: the fraction of the cell's seed
    /// ensemble that read a decomposer guild — a sustained, recruiting
    /// population (#490) — reported, never optimised.
    pub decomposer_fraction: f32,
    /// The consumer twin of `decomposer_fraction` (#490): same read for the
    /// consumer role, same authority boundary.
    pub consumer_fraction: f32,
    /// Per-cell coexistence distribution: the fraction of the cell's seed ensemble
    /// that lands in the coexisting regime. Reported like `decomposer_fraction`;
    /// read only by the projection's robustness floor — never a binning axis nor a
    /// fitness term (genesis-search.md, the authority boundary).
    pub coexistence_fraction: f32,
    /// The seed-ensemble sample count behind that fraction.
    pub sample_count: u32,
    /// Predicted signed distance to the frozen↔oscillation (Hopf) boundary
    /// ([`crate::bifurcation::oscillation_distance`]) — a reported descriptor, never
    /// a binning axis nor a fitness term (genesis-search.md, the authority boundary).
    pub predicted_oscillation_distance: f32,
    /// Predicted signed distance-to-branching `D`
    /// ([`crate::bifurcation::branching_distance`]) — likewise a reported descriptor.
    pub predicted_branching_distance: f32,
    /// The unit-cube elite — the recipe projection for this cell.
    pub unit: Vec<f64>,
}

/// Decide whether a cross-check rollout *disagrees* with the prefilter that gated
/// the config. The prefilter said dead on `predicted`; if the rollout shows life
/// (no failure mode and positive fitness), the two disagree and the case is
/// returned for surfacing — a mis-drawn gate (viability.md). Agreement (the
/// rollout also dies) returns `None`.
fn crosscheck_disagreement(
    predicted: Cliff,
    rollout: &ConfigEval,
    unit: &[f64],
) -> Option<PrefilterDisagreement> {
    if rollout.cliff.is_none() && rollout.median_fitness > 0.0 {
        Some(PrefilterDisagreement {
            predicted_cliff: predicted.label().to_string(),
            observed_fitness: rollout.median_fitness,
            unit: unit.to_vec(),
        })
    } else {
        None
    }
}

/// A surfaced prefilter disagreement: a config the viability prefilter proved
/// dead on `predicted_cliff` that the agreement cross-check rollout nonetheless
/// showed *alive* (positive fitness, no failure). This localises a mis-drawn gate
/// — the prefilter says dead, the run shows life (viability.md). Surfaced, never
/// swallowed.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PrefilterDisagreement {
    /// The cliff the prefilter predicted (the gate that fired).
    pub predicted_cliff: String,
    /// The median fitness the cross-check rollout actually observed.
    pub observed_fitness: f32,
    /// The unit-cube point that disagreed.
    pub unit: Vec<f64>,
}

/// The regime a bifurcation cross-check disagreement falls in — the #358/#359
/// localisation tag. A disagreement in the [`CrosscheckRegime::WeakObservable`]
/// regime localises to the genesis *observable* (or its geometry), not to F's
/// closed-form spectral reading; a [`CrosscheckRegime::Validated`]-regime
/// disagreement implicates the indicator itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CrosscheckRegime {
    /// The observable is trustworthy here (the spike's validated regime), so a
    /// disagreement implicates F's reading.
    Validated,
    /// The observable is known-weak here (#358's flat, demographic-pulsing-
    /// dominated `oscillation_strength`; #359's `clustering_strength` silently
    /// zeroing below n=4), so a disagreement localises to the observable, not F.
    WeakObservable,
}

/// A surfaced **bifurcation** cross-check disagreement: a live config whose
/// predicted distance-to-bifurcation contradicts the observed behaviour-axis
/// boundary on that axis. Modelled on [`PrefilterDisagreement`] — surfaced, never
/// summed into fitness, never a binning axis. The `regime` tag is what makes the
/// disagreement actionable (see [`CrosscheckRegime`]).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BifurcationDisagreement {
    /// Which axis disagreed: `"branching"` (monoculture↔coexistence) or
    /// `"oscillation"` (frozen↔oscillation).
    pub axis: String,
    /// The predicted signed distance-to-bifurcation on that axis.
    pub predicted: f32,
    /// The observed behaviour-axis descriptor the prediction is read against
    /// (`clustering_strength` for branching, `oscillation_strength` for oscillation).
    pub observed: f32,
    /// The localisation tag (see [`CrosscheckRegime`]).
    pub regime: CrosscheckRegime,
}

/// Run the bifurcation cross-check over one **live** config, returning every
/// predicted-vs-observed disagreement (0–2: branching and/or oscillation).
///
/// - **Branching axis.** Disagreement when `sign(predicted_branching_distance)`
///   contradicts the observed `clustering_strength` boundary (`> 0` ⇒ a clustered,
///   coexistence reading). Regime is [`CrosscheckRegime::WeakObservable`] exactly on
///   the #359 small-N borderline signature — `clustering_strength == 0` while
///   `coexistence_duration > 0` (the multi-peak test silently zeroes below n=4
///   even though coexistence persists) — else [`CrosscheckRegime::Validated`]. Wear
///   is off for every searched config, so the other validated-regime condition
///   #359 named already holds.
/// - **Oscillation axis.** Disagreement when `sign(predicted_oscillation_distance)`
///   contradicts the observed `oscillation_strength` boundary (`> 0` ⇒ an
///   oscillatory reading). The regime is **always** [`CrosscheckRegime::WeakObservable`]
///   at current genesis scale: #358's verdict is that `oscillation_strength` is flat
///   and demographic-pulsing-dominated and cannot adjudicate the Hopf crossing. The
///   tag flips to `Validated` once a hardened cycle-detector lands (separate issue,
///   the #358 objective-promotion gate).
fn bifurcation_crosscheck(eval: &ConfigEval) -> Vec<BifurcationDisagreement> {
    let mut out = Vec::new();

    // Branching axis: predicted D vs the observed clustering boundary.
    let predicted_coexistence = eval.predicted_branching_distance > 0.0;
    let observed_coexistence = eval.descriptors.clustering > 0.0;
    if predicted_coexistence != observed_coexistence {
        let regime = if eval.descriptors.clustering == 0.0 && eval.coexistence_duration > 0.0 {
            CrosscheckRegime::WeakObservable
        } else {
            CrosscheckRegime::Validated
        };
        out.push(BifurcationDisagreement {
            axis: "branching".to_string(),
            predicted: eval.predicted_branching_distance,
            observed: eval.descriptors.clustering,
            regime,
        });
    }

    // Oscillation axis: predicted |λ|−1 vs the observed oscillation boundary. The
    // observable cannot adjudicate the crossing at genesis scale (#358), so the
    // regime is constant WeakObservable until a hardened cycle-detector lands.
    let predicted_oscillation = eval.predicted_oscillation_distance > 0.0;
    let observed_oscillation = eval.descriptors.oscillation > 0.0;
    if predicted_oscillation != observed_oscillation {
        out.push(BifurcationDisagreement {
            axis: "oscillation".to_string(),
            predicted: eval.predicted_oscillation_distance,
            observed: eval.descriptors.oscillation,
            regime: CrosscheckRegime::WeakObservable,
        });
    }

    out
}

/// The genesis [`Atlas`](../../CONTEXT.md): the live archive (cells binned on the
/// three behaviour axes) paired with the dead frontier (keyed by cliff), plus the
/// coverage / QD-score summary. This is the search's output — a map onto
/// behaviour, not a ranked list.
/// A surfaced early-stop cross-check disagreement (#506): a rollout an
/// incremental dead-pool gate stopped as dead at `stop_tick` that, carried to
/// the horizon, read alive on the full series — the gate fired on a collapse
/// the world recovered from.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EarlyStopDisagreement {
    /// The gate that stopped it (`energy_death` or `nutrient_lockup`).
    pub gate: String,
    /// The tick the gate fired on.
    pub stop_tick: u64,
    /// Which seed of the ensemble (its index) was carried.
    pub seed_index: usize,
    /// The fitness the carried run read at the horizon.
    pub observed_fitness: f32,
    /// The unit-cube point that disagreed.
    pub unit: Vec<f64>,
}

/// The early-stop cross-check read off one ensemble: how many seeds were
/// carried, and the disagreements among them.
#[derive(Clone, Debug, Default)]
pub struct EarlyStopCrosscheck {
    pub carried: usize,
    pub disagreements: Vec<EarlyStopDisagreement>,
}

/// Read the early-stop cross-check off an ensemble: every seed a dead-pool gate
/// stopped and the carry took to the horizon counts toward `carried`; those
/// alive on the full series are the disagreements. Seeds that were stopped but
/// not carried, or never stopped, have nothing to compare.
pub fn early_stop_crosscheck(result: &EnsembleResult, unit: &[f64]) -> EarlyStopCrosscheck {
    let mut check = EarlyStopCrosscheck::default();
    for (seed_index, run) in result.run_results.iter().enumerate() {
        let Some(stop) = run.early_stop.as_ref() else {
            continue;
        };
        if stop.horizon.is_none() {
            continue;
        }
        check.carried += 1;
        if let Some(horizon) = stop.disagreement() {
            check.disagreements.push(EarlyStopDisagreement {
                gate: Cliff::from_failure(&stop.failure).label().to_string(),
                stop_tick: stop.tick,
                seed_index,
                observed_fitness: horizon.fitness,
                unit: unit.to_vec(),
            });
        }
    }
    check
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Atlas {
    /// The search that drew this atlas: its base seed and horizon (#531). The
    /// refinement re-evaluates cells at the horizon, seeded off the base seed,
    /// so this is what lets a recipe be re-projected from the atlas file alone.
    /// `None` on an atlas written before it was recorded.
    #[serde(default)]
    pub provenance: Option<AtlasProvenance>,
    /// The search box the atlas was drawn under (#559): the ranges a cell's
    /// `unit` decodes over. A unit vector names a world only together with its
    /// box, so every reader decodes the cells over this one — read it through
    /// [`Atlas::search_box`]. `None` on an atlas written before it was
    /// recorded, which was searched under the full box, `default_ranges()`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_box: Option<Vec<ParameterRange>>,
    /// The live cells in cell-index order, so the same search writes the same
    /// atlas bytes and `atlas:N` in the research sweeps names the same config
    /// (#536) — not in the archive's `HashMap` order.
    pub cells: Vec<AtlasCell>,
    /// Dead-frontier tally: how many configs died on each cliff (by label),
    /// **a priori and observed combined**. Label-ordered, like `cells`, so the
    /// written atlas is canonical (#536).
    pub dead_frontier: std::collections::BTreeMap<String, usize>,
    /// The **a priori** layer of the dead frontier: configs the viability
    /// prefilter (`crate::prefilter`) proved dead in closed form, by cliff label.
    /// These spent no ensemble. The observed deaths are `dead_frontier` minus
    /// this. Genesis-search.md: "the dead frontier is the atlas's most valuable
    /// layer", and the a-priori-vs-observed split is its agreement cross-check.
    pub dead_frontier_apriori: std::collections::BTreeMap<String, usize>,
    /// Ensemble rollouts the prefilter skipped (the budget saved) — one per
    /// a-priori death that was *not* drawn into the agreement cross-check sample.
    pub rollouts_skipped: usize,
    /// Seed rollouts that exhausted a wall-clock budget (#562): no verdict, so
    /// in neither the archive nor the dead frontier. A config is read off its
    /// finished seeds, and placed nowhere if none finished. Written only when
    /// non-zero, so an atlas no budget touched keeps its pre-#562 bytes.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rollouts_unfinished: usize,
    /// Prefilter disagreements surfaced: configs the prefilter proved dead that a
    /// cross-check rollout nonetheless showed alive. A non-empty list localises a
    /// mis-drawn gate (viability.md, *Place in the validation triad*); it is
    /// reported, never swallowed.
    pub prefilter_disagreements: Vec<PrefilterDisagreement>,
    /// Bifurcation cross-check disagreements surfaced: live configs whose predicted
    /// distance-to-bifurcation contradicts the observed behaviour-axis boundary,
    /// each tagged with a [`CrosscheckRegime`]. This is the validation-triad
    /// cross-check #358/#359 prize — a disagreement in the `WeakObservable` regime
    /// localises to the observable, not F's spectral reading. Reported, never
    /// swallowed; never summed into fitness nor a binning axis.
    pub bifurcation_disagreements: Vec<BifurcationDisagreement>,
    /// Rollouts an incremental dead-pool gate stopped that were nonetheless
    /// carried to the horizon for the early-stop cross-check (#506) — the
    /// sample size behind `early_stop_disagreements`.
    pub early_stop_crosschecks: usize,
    /// Early-stop cross-check disagreements surfaced: rollouts the energy-death
    /// or lockup gate stopped as dead that, carried to the horizon, read *alive*
    /// on the full series. Each localises a gate that fired on a reversible
    /// collapse. Reported, never swallowed; the run itself stayed a frontier
    /// entry.
    pub early_stop_disagreements: Vec<EarlyStopDisagreement>,
    /// Filled-cell count.
    pub coverage: usize,
    /// Total cells in the binning (`RESOLUTION³`).
    pub total_cells: usize,
    /// Σ elite fitness over filled cells.
    pub qd_score: f32,
    /// The best elite fitness found (the recipe projection's fitness).
    pub best_fitness: f32,
}

/// What an [`Atlas`] records of the search that drew it (#531): everything the
/// projection reads that the cells themselves do not carry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AtlasProvenance {
    /// The search's base seed; refinement seeds are drawn off it.
    pub seed: u64,
    /// The search's rollout horizon (`max_ticks`), which the refinement and the
    /// recipe share.
    pub max_ticks: u64,
}

/// The carcass-locked fraction at which the evaluator gates a world as
/// `NutrientLockup` — mirrored here so the search can cross-check its atlas against
/// the gate. **Must equal** `LOCK_FRACTION` in `explorers-genesis-eval`
/// (`is_nutrient_locked`); the cross-check test pins the two together so a future
/// drift in the evaluator gate surfaces here as a failing test.
pub const LOCK_FRACTION: f32 = 0.4;

/// The minimum `coexistence_fraction` a cell must clear for the default projection
/// to pick it (selection only — not a binning axis, not a fitness term). 0.5
/// operationalizes CONTEXT.md:269's bar — *"a parameterisation is accepted only
/// when most runs in the ensemble produce sensible worlds"* — at the projection
/// step, so a monoculture↔coexistence bifurcation straddler that won a cell on a
/// lucky seed draw is not certified as the playable recipe (#401). The straddler
/// stays a recorded cell; only the recipe pick reads this floor.
pub const COEXISTENCE_FLOOR: f32 = 0.5;

/// The lockup boundary cross-check (genesis-search.md; the #363 spike's promoted
/// check). Splits the atlas's live cells at the [`LOCK_FRACTION`] gate and reads
/// the `NutrientLockup` dead-frontier tally, so the design's invariant can be
/// asserted: gated configs sit at high carcass fraction (they are on the frontier,
/// not in a cell), and live cells *mostly* sit below the gate (a locked world is
/// gated, never a live cell).
#[derive(Clone, Copy, Debug)]
pub struct LockupCrosscheck {
    /// Live cells whose carcass fraction is below the lock gate (the expected
    /// healthy-throughput majority).
    pub live_below_gate: usize,
    /// Live cells at/above the lock gate — a boundary violation to surface (a world
    /// that reads live yet sits where the evaluator gates lockup).
    pub live_at_or_above_gate: usize,
    /// Live cells in the **high-carcass band near the cliff** — carcass ≥ half the
    /// gate but still below it. These are the live worlds the carcass axis exists to
    /// place: nutrient piling into the dead pool, not yet locked. Reaching them is
    /// the "live cells at high carcass fraction" half of the acceptance criterion.
    pub live_high_carcass: usize,
    /// Configs the rollout carried into the `NutrientLockup` cliff (the frontier
    /// tally for that cliff).
    pub lockup_frontier_deaths: usize,
}

impl LockupCrosscheck {
    /// The atlas's lockup layer is non-empty — the acceptance criterion. Either a
    /// `NutrientLockup` death is on the frontier, or a live cell reaches the
    /// high-carcass region near the cliff. Before carcass direction (#363) both were
    /// zero: the lockup layer the carcass axis exists to map went unreached.
    pub fn lockup_layer_populated(&self) -> bool {
        self.lockup_frontier_deaths > 0 || self.live_high_carcass > 0
    }
}

impl Atlas {
    /// The search box this atlas's cells decode over (#559): the one it
    /// records, or the full box `default_ranges()` for an atlas from before the
    /// box was recorded (the box every such atlas was searched under).
    pub fn search_box(&self) -> Vec<ParameterRange> {
        self.search_box.clone().unwrap_or_else(default_ranges)
    }

    /// Check that `ranges`, the box a reader would decode this atlas's cells
    /// over, is the atlas's own [`Atlas::search_box`] (#559).
    pub fn check_search_box(&self, ranges: &[ParameterRange]) -> Result<(), SearchBoxMismatch> {
        check_search_box(&self.search_box(), ranges)
    }

    /// [`Atlas::check_search_box`], stopping loudly on a mismatch: the recipe
    /// readers below cannot report one, and must not decode the wrong worlds.
    fn assert_search_box(&self, ranges: &[ParameterRange]) {
        if let Err(e) = self.check_search_box(ranges) {
            panic!("{e}");
        }
    }

    /// Run the lockup boundary cross-check over this atlas (see
    /// [`LockupCrosscheck`]). Counts live cells either side of the
    /// [`LOCK_FRACTION`] gate and reads the `NutrientLockup` frontier tally.
    pub fn lockup_boundary_crosscheck(&self) -> LockupCrosscheck {
        let mut live_below_gate = 0;
        let mut live_at_or_above_gate = 0;
        let mut live_high_carcass = 0;
        for c in &self.cells {
            if c.carcass >= LOCK_FRACTION {
                live_at_or_above_gate += 1;
            } else {
                live_below_gate += 1;
                if c.carcass >= LOCK_FRACTION / 2.0 {
                    live_high_carcass += 1;
                }
            }
        }
        let lockup_frontier_deaths = self
            .dead_frontier
            .get(Cliff::NutrientLockup.label())
            .copied()
            .unwrap_or(0);
        LockupCrosscheck {
            live_below_gate,
            live_at_or_above_gate,
            live_high_carcass,
            lockup_frontier_deaths,
        }
    }

    /// The world recipe drawn from the highest-fitness live cell that is *robustly
    /// sensible* — its `coexistence_fraction` clears [`COEXISTENCE_FLOOR`] (most of
    /// its seed ensemble coexists). Falls back to plain argmax-fitness when no live
    /// cell clears the floor, so the projection stays total over live atlases (the
    /// app always gets a world). Selection only: binning, fitness, and the straddler
    /// cell itself are untouched (genesis-search.md, "the recipe is a projection";
    /// #401). `None` if no cell is live (every config died — all frontier).
    ///
    /// Panics if `ranges` is not the atlas's own [`Atlas::search_box`] (#559).
    pub fn best_recipe(&self, ranges: &[ParameterRange], max_ticks: u64) -> Option<WorldRecipe> {
        self.assert_search_box(ranges);
        let pick = self
            .cells
            .iter()
            .filter(|c| c.coexistence_fraction >= COEXISTENCE_FLOOR)
            .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap())
            .or_else(|| {
                self.cells
                    .iter()
                    .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap())
            })?;
        Some(recipe_from_unit(&pick.unit, ranges, max_ticks))
    }

    /// The world recipe for a specific live cell index, if that cell is filled —
    /// the atlas's honest stance that *any* viable cell yields a recipe, not only
    /// the best one. Panics if `ranges` is not the atlas's own
    /// [`Atlas::search_box`] (#559).
    pub fn recipe_for_cell(
        &self,
        cell: [usize; 3],
        ranges: &[ParameterRange],
        max_ticks: u64,
    ) -> Option<WorldRecipe> {
        self.assert_search_box(ranges);
        let c = self.cells.iter().find(|c| c.cell == cell)?;
        Some(recipe_from_unit(&c.unit, ranges, max_ticks))
    }
}

fn recipe_from_unit(unit: &[f64], ranges: &[ParameterRange], max_ticks: u64) -> WorldRecipe {
    let (parameters, initial_distribution) = decode(unit, ranges);
    WorldRecipe {
        parameters,
        initial_distribution: Some(initial_distribution),
        agents: None,
        carcasses: None,
        max_ticks,
    }
}

// ---------------------------------------------------------------------------
// Gated elite refinement of the projection (#404)
// ---------------------------------------------------------------------------

/// Default number of top-fitness live cells to re-evaluate before projecting.
/// Small by design — refinement is the projection's last gate, not a second
/// search — so its cost stays bounded (genesis-search.md, "the recipe is a
/// projection").
pub const REFINE_TOP_K: usize = 10;

/// Default refinement ensemble size: the larger, independent draw that hardens the
/// high-variance in-run n=5 estimate the floor reads (#404). 32 ≫ the 5-seed search
/// ensemble. It is sized as a *separator*, not an estimator (#434,
/// `docs/research/434-ensemble-confidence.md`): the fixed rule `k/n ≥ 0.5` at n=32
/// tells a straddler at p=0.35 (the #401 leader re-read ~3/8) from a robust cell at
/// p=0.65 with α = β ≈ 5 % (the fixed-n optimum for that separation is n=29). It does
/// **not** tell 0.4 from 0.6 (n=67 needed), and the two-sided 95 % Clopper–Pearson
/// interval at 16/32 is still [0.32, 0.68] — so the refined fraction is a gate input,
/// never a tight point estimate near the monoculture↔coexistence bifurcation.
pub const REFINE_ENSEMBLE_SIZE: u32 = 32;

/// Seed offset that puts every refinement ensemble far above any seed the search
/// itself drew, so the re-evaluation is an **independent** draw rather than a
/// re-read of the in-run seeds (#404). The search assigns config base seeds as
/// `base_seed + config_index*1000` (`run_qd`), bounded well under `2^40` for any
/// real budget (`batch*(generations+1)` configs ≪ `2^40 / 1000`); a refinement
/// block at `base_seed + 2^40 + rank*n` therefore never collides with a search
/// ensemble. Fixed, so a given `(atlas, base_seed)` refines bit-reproducibly.
const REFINEMENT_SEED_OFFSET: u64 = 1 << 40;

/// One top-K cell re-evaluated at the larger refinement ensemble — the recorded
/// (in-run) estimate beside the refined one, reported so the projection's gate is
/// auditable. Selection only: this never rewrites the cell's atlas record.
#[derive(Clone, Debug, serde::Serialize)]
pub struct RefinedCell {
    /// The behaviour-axis cell index `(oscillation, clustering, carcass)`.
    pub cell: [usize; 3],
    /// The cell's recorded (atlas) elite fitness — the ranking authority, untouched
    /// by refinement.
    pub recorded_fitness: f32,
    /// The in-run coexistence fraction (the high-variance n=5 estimate the floor
    /// read before #404).
    pub recorded_coexistence_fraction: f32,
    /// The refined coexistence fraction over the larger independent ensemble,
    /// under every [`CoexistenceFloor`] — the one the projection's floor selects
    /// is what the pick reads (#538).
    pub refined_fractions: CoexistenceFractions,
    /// Share of the refinement ensemble holding a decomposer guild (#490),
    /// reported beside the fractions so a guild-aware pick is auditable.
    pub refined_decomposer_fraction: f32,
    /// The consumer twin of `refined_decomposer_fraction`.
    pub refined_consumer_fraction: f32,
    /// The refined median fitness over the larger ensemble (reported for audit;
    /// never the ranking key — that stays the recorded fitness).
    pub refined_median_fitness: f32,
    /// The finished refinement seeds behind the refined fraction.
    pub refined_sample_count: u32,
    /// Refinement seeds that exhausted a wall-clock budget (#562), excluded
    /// from the refined fraction's denominator.
    #[serde(skip_serializing_if = "is_zero")]
    pub refined_unfinished: usize,
    /// Whether the refined fraction under the projection's floor clears
    /// [`COEXISTENCE_FLOOR`].
    pub clears_floor: bool,
}

/// What the refinement reads off one cell's larger ensemble.
struct RefinedEval {
    fractions: CoexistenceFractions,
    median_fitness: f32,
    sample_count: u32,
    unfinished: usize,
    decomposer_fraction: f32,
    consumer_fraction: f32,
}

/// The refined projection: the picked recipe plus the audit trail (#404). The pick
/// is the highest recorded-fitness top-K cell whose **refined** coexistence
/// fraction clears [`COEXISTENCE_FLOOR`]; it falls back to plain argmax-fitness
/// (with `cleared_floor = false`) when no refined cell clears, mirroring
/// [`Atlas::best_recipe`] so a live atlas always yields a world.
#[derive(Clone, Debug)]
pub struct RefinedProjection {
    /// The projected recipe (`None` only when the atlas has no live cell).
    pub recipe: Option<WorldRecipe>,
    /// `true` when the pick cleared the refined floor; `false` on the argmax
    /// fallback (the warning path — no robustly-coexisting cell at this budget).
    pub cleared_floor: bool,
    /// The floor the pick read (#538); `Plain` unless one was chosen.
    pub floor: CoexistenceFloor,
    /// The refined top-K, in recorded-fitness-descending (ranking) order.
    pub refined: Vec<RefinedCell>,
    /// Live cells beyond the top-K that were *not* refined — the bounded-cost
    /// disclosure (a robust cell ranked below the cut is not reached).
    pub unrefined_live_cells: usize,
}

/// Configuration for the gated elite refinement step (#404).
#[derive(Clone, Debug)]
pub struct RefinementConfig {
    /// Top-fitness live cells to re-evaluate (the gate, not a search).
    pub top_k: usize,
    /// Refinement ensemble size (the larger independent draw).
    pub ensemble_size: u32,
    /// Rollout horizon, matching the search's `max_ticks`.
    pub max_ticks: u64,
    /// Which coexistence predicate the floor reads (#538). A selection setting:
    /// it never changes which cells are refined, on which seeds.
    pub floor: CoexistenceFloor,
    /// Wall-clock budget on each refinement rollout (#562).
    pub rollout_budget: RolloutBudget,
}

impl Default for RefinementConfig {
    fn default() -> Self {
        RefinementConfig {
            top_k: REFINE_TOP_K,
            ensemble_size: REFINE_ENSEMBLE_SIZE,
            max_ticks: 2000,
            floor: CoexistenceFloor::Plain,
            rollout_budget: SEARCH_ROLLOUT_BUDGET,
        }
    }
}

/// Project a recipe from the atlas after hardening the top-K cells' coexistence
/// estimate against `evaluate` — the pure selection core (#404). `evaluate(rank,
/// unit)` returns the [`RefinedEval`] of the cell at ranking position `rank`, and
/// `floor` picks which of its fractions the floor reads (#538); the sim-backed
/// wrapper is [`refined_best_recipe`]. Selection only: `atlas` is read, never
/// mutated, and the **recorded** fitness remains the ranking key.
fn project_with_refinement(
    atlas: &Atlas,
    ranges: &[ParameterRange],
    max_ticks: u64,
    top_k: usize,
    floor: CoexistenceFloor,
    mut evaluate: impl FnMut(usize, &[f64]) -> RefinedEval,
) -> RefinedProjection {
    // Rank live cells by recorded fitness (descending), with a deterministic
    // tiebreak on the cell index so the ranking — and thus the per-rank refinement
    // seeds — is independent of the archive's HashMap iteration order.
    let mut ranked: Vec<&AtlasCell> = atlas.cells.iter().collect();
    ranked.sort_by(|a, b| {
        b.fitness
            .partial_cmp(&a.fitness)
            .unwrap()
            .then_with(|| a.cell.cmp(&b.cell))
    });

    if ranked.is_empty() {
        return RefinedProjection {
            recipe: None,
            cleared_floor: false,
            floor,
            refined: Vec::new(),
            unrefined_live_cells: 0,
        };
    }

    let k = top_k.min(ranked.len());
    let unrefined_live_cells = ranked.len() - k;

    // Re-evaluate the top-K at the larger ensemble. The list stays in
    // recorded-fitness order, so the first floor-clearing entry is the
    // highest-fitness robust cell.
    let refined: Vec<RefinedCell> = ranked[..k]
        .iter()
        .enumerate()
        .map(|(rank, c)| {
            let eval = evaluate(rank, &c.unit);
            RefinedCell {
                cell: c.cell,
                recorded_fitness: c.fitness,
                recorded_coexistence_fraction: c.coexistence_fraction,
                refined_fractions: eval.fractions,
                refined_decomposer_fraction: eval.decomposer_fraction,
                refined_consumer_fraction: eval.consumer_fraction,
                refined_median_fitness: eval.median_fitness,
                refined_sample_count: eval.sample_count,
                refined_unfinished: eval.unfinished,
                clears_floor: eval.fractions.under(floor) >= COEXISTENCE_FLOOR,
            }
        })
        .collect();

    // Pick the highest recorded-fitness top-K cell whose refined fraction clears
    // the floor; fall back to plain argmax-fitness (the warning path) when none
    // does, so a live atlas always yields a world.
    match refined.iter().position(|r| r.clears_floor) {
        Some(pos) => RefinedProjection {
            recipe: Some(recipe_from_unit(&ranked[pos].unit, ranges, max_ticks)),
            cleared_floor: true,
            floor,
            refined,
            unrefined_live_cells,
        },
        None => RefinedProjection {
            recipe: Some(recipe_from_unit(&ranked[0].unit, ranges, max_ticks)),
            cleared_floor: false,
            floor,
            refined,
            unrefined_live_cells,
        },
    }
}

/// Re-evaluate the atlas's top-K live cells at the larger refinement ensemble and
/// project the recipe from the highest-fitness cell that clears the refined
/// coexistence floor (#404). Hardens the high-variance in-run n=5 estimate that
/// both fitness and the floor depend on, using deterministic seeds disjoint from
/// the search's (see [`REFINEMENT_SEED_OFFSET`]). Selection only — never rewrites
/// the atlas map's binning or per-cell fitness (genesis-search.md, the authority
/// boundary). Deterministic in `(atlas, config, base_seed)`. Panics if `ranges`
/// is not the atlas's own [`Atlas::search_box`] (#559).
pub fn refined_best_recipe(
    atlas: &Atlas,
    ranges: &[ParameterRange],
    config: &RefinementConfig,
    base_seed: u64,
) -> RefinedProjection {
    atlas.assert_search_box(ranges);
    let ensemble_config = EnsembleConfig {
        ensemble_size: config.ensemble_size,
        run_config: RunConfig {
            max_ticks: config.max_ticks,
            eval_config: EvalConfig::default(),
            // Selection only: a refinement never surfaces a disagreement, so
            // it never pays for a carry.
            early_stop_crosscheck_fraction: 0.0,
        },
    };
    project_with_refinement(
        atlas,
        ranges,
        config.max_ticks,
        config.top_k,
        config.floor,
        |rank, unit| {
            let (wp, dist) = decode(unit, ranges);
            // Disjoint, deterministic refinement seeds: a block per rank, far above
            // any search seed (independent draw, reproducible for a fixed seed).
            let refine_seed = base_seed
                .wrapping_add(REFINEMENT_SEED_OFFSET)
                .wrapping_add(rank as u64 * config.ensemble_size as u64);
            let result = run_ensemble_within(
                &wp,
                &dist,
                &ensemble_config,
                refine_seed,
                config.rollout_budget,
            );
            let eval = config_eval_from_ensemble(&result);
            RefinedEval {
                fractions: CoexistenceFractions::of_seeds(&result.run_results),
                median_fitness: eval.median_fitness,
                sample_count: eval.sample_count,
                unfinished: result.unfinished,
                decomposer_fraction: eval.decomposer_fraction,
                consumer_fraction: eval.consumer_fraction,
            }
        },
    )
}

// ---------------------------------------------------------------------------
// QD driver
// ---------------------------------------------------------------------------

/// Configuration for the QD outer search.
#[derive(Clone, Debug)]
pub struct QdConfig {
    pub ranges: Vec<ParameterRange>,
    pub ensemble_size: u32,
    pub max_ticks: u64,
    /// Solutions evaluated per generation.
    pub batch: usize,
    /// Number of adaptation generations after the random bootstrap batch.
    pub generations: usize,
    /// Initial per-dimension emitter deviation in unit-cube coordinates.
    pub sigma: f64,
    /// CMA-MAE archive learning rate (soft-acceptance threshold drag).
    pub archive_learning_rate: f32,
    /// Fraction of prefilter-gated (a priori dead) configs that are *still* rolled
    /// out as an agreement cross-check, in `[0, 1]`. A prefilter-says-dead /
    /// rollout-shows-life disagreement on a sampled config localises a mis-drawn
    /// gate and is surfaced on the atlas (viability.md, *Place in the validation
    /// triad*). 0 disables the cross-check (every gated config is taken on faith).
    pub prefilter_crosscheck_fraction: f32,
    /// Fraction of rollouts an incremental dead-pool gate stopped (energy death,
    /// nutrient lockup) that are carried to the horizon anyway and re-verdicted
    /// on the full series, in `[0, 1]` — the prefilter cross-check's interlock
    /// moved along the trajectory (#506; `RunConfig::early_stop_crosscheck_fraction`).
    /// A stopped-dead / alive-at-`T` disagreement is surfaced on the atlas
    /// (`early_stop_disagreements`), never swallowed. 0 disables the carry.
    pub early_stop_crosscheck_fraction: f32,
    /// How many **carcass-directed seeds** ([`carcass_seed_unit`]) to inject at the
    /// head of the bootstrap batch. The nutrient-lockup cliff cannot be
    /// prefiltered, so the atlas's lockup layer is populated only by *running*
    /// worlds that strand their nutrient — and a random emitter never lands there
    /// (#363: 0 lockup-frontier configs). Seeding a few guaranteed high-carcass
    /// starting points gives the covariance-adapting emitter a foothold on the
    /// carcass axis to breed toward the cliff. 0 disables carcass direction (the
    /// historical random-bootstrap behaviour).
    pub carcass_seed_count: usize,
    /// Wall-clock budget on each seed rollout (#562). Not part of what a
    /// checkpoint must match: it changes the atlas only when it fires.
    pub rollout_budget: RolloutBudget,
}

/// The search's and the refinement's default [`RolloutBudget`] (#562):
/// generous against the seconds a `T = 2000` rollout usually takes, so only a
/// stalled one — a dense world — ever reaches it, and the committed atlas
/// reproduces unbudgeted.
pub const SEARCH_ROLLOUT_BUDGET: RolloutBudget = RolloutBudget {
    simulation: std::time::Duration::from_secs(DEFAULT_SEARCH_TIMEOUT_SECS),
    evaluation: std::time::Duration::from_secs(DEFAULT_SEARCH_TIMEOUT_SECS),
};

/// Both halves of [`SEARCH_ROLLOUT_BUDGET`], in seconds.
pub const DEFAULT_SEARCH_TIMEOUT_SECS: u64 = 600;

fn is_zero(n: &usize) -> bool {
    *n == 0
}

impl Default for QdConfig {
    fn default() -> Self {
        QdConfig {
            ranges: default_ranges(),
            ensemble_size: 5,
            max_ticks: 2000,
            batch: 32,
            generations: 10,
            sigma: 0.15,
            archive_learning_rate: 0.5,
            prefilter_crosscheck_fraction: 0.05,
            early_stop_crosscheck_fraction: 0.05,
            carcass_seed_count: 2,
            rollout_budget: SEARCH_ROLLOUT_BUDGET,
        }
    }
}

/// What the search reports to its caller as each generation completes (#529).
/// Reporting only: it is read off the archive after the generation's inserts and
/// never feeds back into the search.
#[derive(Clone, Debug)]
pub struct GenerationReport {
    /// Index of the generation just completed; 0 is the bootstrap batch.
    pub generation: usize,
    /// The last generation index (`QdConfig::generations`): the search runs
    /// `generations + 1` generations in all.
    pub generations: usize,
    /// Live cells filled so far, of `total_cells`.
    pub coverage: usize,
    pub total_cells: usize,
    /// QD-score (Σ elite fitness) of the archive so far.
    pub qd_score: f32,
    /// Best elite fitness in the archive so far.
    pub best_fitness: f32,
    /// Configs in this generation's batch that were rolled out (cleared, or
    /// gated but drawn into the prefilter cross-check).
    pub rolled_out: usize,
    /// Configs in this generation's batch the a-priori prefilter skipped.
    pub skipped: usize,
    /// Seed rollouts in this generation that exhausted their budget (#562).
    pub unfinished: usize,
    /// Wall-clock spent on this generation.
    pub generation_elapsed: std::time::Duration,
    /// Wall-clock since the search began.
    pub elapsed: std::time::Duration,
}

impl std::fmt::Display for GenerationReport {
    /// One progress line, read at a glance in a terminal while the search runs —
    /// not a format to parse.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let width = self.generations.to_string().len();
        write!(
            f,
            "gen {:>width$}/{}  coverage {}/{} ({:.2}%)  qd {:.3}  best {:.4}  \
             rolled out {}, skipped {}  took {}  elapsed {}",
            self.generation,
            self.generations,
            self.coverage,
            self.total_cells,
            100.0 * self.coverage as f64 / self.total_cells.max(1) as f64,
            self.qd_score,
            self.best_fitness,
            self.rolled_out,
            self.skipped,
            clock(self.generation_elapsed),
            clock(self.elapsed),
        )?;
        if self.unfinished > 0 {
            write!(f, "  unfinished {}", self.unfinished)?;
        }
        Ok(())
    }
}

/// A wall-clock span as `42s`, `3m12s` or `3h12m05s`.
fn clock(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    if h > 0 {
        format!("{h}h{m:02}m{s:02}s")
    } else if m > 0 {
        format!("{m}m{s:02}s")
    } else {
        format!("{s}s")
    }
}

/// Run the QD outer search and return the illuminated [`Atlas`]. Reuses
/// [`decode`] and [`run_ensemble`] unchanged — descriptors are read off the
/// per-seed breakdowns. Deterministic in `(config, base_seed, rng)`.
pub fn run_qd(config: &QdConfig, base_seed: u64, rng: &mut impl Rng) -> Atlas {
    run_qd_observed(config, base_seed, rng, &mut |_: &GenerationReport| {})
}

/// [`run_qd`], calling `observer` with a [`GenerationReport`] as each generation
/// completes (#529). The observer sees the search; it cannot steer it — the atlas
/// is identical with or without one.
pub fn run_qd_observed(
    config: &QdConfig,
    base_seed: u64,
    rng: &mut impl Rng,
    observer: &mut impl FnMut(&GenerationReport),
) -> Atlas {
    let never = &mut |_: &SearchState<_>| Ok::<(), std::convert::Infallible>(());
    match SearchState::start(config, rng).run(config, base_seed, observer, never) {
        Ok(atlas) => atlas,
        Err(infallible) => match infallible {},
    }
}

/// Everything the search carries from one generation to the next (#530). A
/// generation boundary is fully described by this: the loop reads nothing else,
/// so persisting it and running on from it continues the search exactly — the
/// checkpoint ([`crate::checkpoint`]) is this state, serialised.
///
/// The archive is carried whole (its cells with their private acceptance
/// thresholds, both frontier tallies, its learning rate) rather than its atlas
/// projection, which drops the thresholds.
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct SearchState<R> {
    /// The generation to run next; `generations + 1` once the search is done.
    pub(crate) generation: usize,
    /// Monotonic config counter that derives each config's ensemble base seed.
    config_index: u64,
    /// The batch the next generation evaluates — emitted at the end of the
    /// previous generation, so loop-carried rather than recomputable.
    batch: Vec<Vec<f64>>,
    archive: Archive,
    emitter: CmaEmitter,
    /// The search's random stream: the prefilter cross-check coin, the emitter's
    /// samples and the parent selection draw from it, in that order.
    rng: R,
    rollouts_skipped: usize,
    #[serde(default, skip_serializing_if = "is_zero")]
    rollouts_unfinished: usize,
    prefilter_disagreements: Vec<PrefilterDisagreement>,
    bifurcation_disagreements: Vec<BifurcationDisagreement>,
    early_stop_crosschecks: usize,
    early_stop_disagreements: Vec<EarlyStopDisagreement>,
}

impl<R: Rng> SearchState<R> {
    /// The state before generation 0: an empty archive, a fresh emitter, and the
    /// bootstrap batch drawn from `rng`.
    pub(crate) fn start(config: &QdConfig, mut rng: R) -> Self {
        let emitter = CmaEmitter::new(config.ranges.len(), config.sigma);
        // Generation 0: a random bootstrap batch over the cube (the QD analogue of
        // the incumbent's LHS stage), drawn from the emitter's initial wide
        // Gaussian — with the head replaced by carcass-directed seeds so the
        // bootstrap reaches the thin high-carcass / nutrient-lockup region a
        // random emitter misses (#363; the cliff cannot be prefiltered, so it
        // must be reached by running).
        let mut batch: Vec<Vec<f64>> = (0..config.batch)
            .map(|_| emitter.sample(&mut rng))
            .collect();
        inject_carcass_seeds(&mut batch, &config.ranges, config.carcass_seed_count);
        SearchState {
            generation: 0,
            config_index: 0,
            batch,
            archive: Archive::new(config.archive_learning_rate),
            emitter,
            rng,
            rollouts_skipped: 0,
            rollouts_unfinished: 0,
            prefilter_disagreements: Vec::new(),
            bifurcation_disagreements: Vec::new(),
            early_stop_crosschecks: 0,
            early_stop_disagreements: Vec::new(),
        }
    }

    /// Run the remaining generations and return the atlas. `boundary` is handed
    /// the state at every generation boundary — after the generation's inserts
    /// and the next batch's emission, before its report reaches `observer` — so
    /// a report on screen means the boundary it describes has been handed off.
    /// An error from `boundary` stops the search and is returned.
    pub(crate) fn run<E>(
        mut self,
        config: &QdConfig,
        base_seed: u64,
        observer: &mut impl FnMut(&GenerationReport),
        boundary: &mut impl FnMut(&Self) -> Result<(), E>,
    ) -> Result<Atlas, E> {
        let ensemble_config = EnsembleConfig {
            ensemble_size: config.ensemble_size,
            run_config: RunConfig {
                max_ticks: config.max_ticks,
                eval_config: EvalConfig::default(),
                early_stop_crosscheck_fraction: config.early_stop_crosscheck_fraction,
            },
        };

        // Wall-clock for the per-generation report only (#529); never read by the
        // search. A resumed search counts from the resume.
        let started = std::time::Instant::now();
        let mut elapsed_before = std::time::Duration::ZERO;

        while self.generation <= config.generations {
            let generation = self.generation;
            let unfinished_before = self.rollouts_unfinished;
            let (improvements, rolled_out, skipped) =
                self.evaluate_batch(config, base_seed, &ensemble_config);

            if generation < config.generations {
                // Adapt the emitter toward the improvers, then emit the next batch.
                self.emitter.adapt(&self.batch, &improvements);
                self.emit_next_batch(config);
            } else {
                self.batch.clear();
            }
            self.generation += 1;
            boundary(&self)?;

            let elapsed = started.elapsed();
            observer(&GenerationReport {
                generation,
                generations: config.generations,
                coverage: self.archive.coverage(),
                total_cells: RESOLUTION.pow(3),
                qd_score: self.archive.qd_score(),
                best_fitness: self.archive.best_fitness(),
                rolled_out,
                skipped,
                unfinished: self.rollouts_unfinished - unfinished_before,
                generation_elapsed: elapsed - elapsed_before,
                elapsed,
            });
            elapsed_before = elapsed;
        }

        let mut atlas = self.atlas();
        atlas.provenance = Some(AtlasProvenance {
            seed: base_seed,
            max_ticks: config.max_ticks,
        });
        atlas.search_box = Some(config.ranges.clone());
        Ok(atlas)
    }

    /// Evaluate the current batch and route every config into the archive, the
    /// frontier and the cross-check tallies. Returns the emitter's improvement
    /// signal and `(rolled_out, skipped)` for the generation's report.
    fn evaluate_batch(
        &mut self,
        config: &QdConfig,
        base_seed: u64,
        ensemble_config: &EnsembleConfig,
    ) -> (Vec<f32>, usize, usize) {
        let skipped_before = self.rollouts_skipped;
        // Distinct per-config ensemble base seeds, derived from a monotonic config
        // counter so evaluation order is immaterial (each config's seed is fixed).
        let seeds: Vec<u64> = (0..self.batch.len())
            .map(|_| {
                let s = base_seed.wrapping_add(self.config_index.wrapping_mul(1000));
                self.config_index += 1;
                s
            })
            .collect();

        // A priori viability prefilter: run the two committed closed-form gates
        // (`crate::prefilter`) ahead of each rollout. A gated config is a proven
        // a-priori death — it spends no ensemble — unless it is drawn into the
        // agreement cross-check sample, which still rolls it out to falsify the
        // gate. Drawing the cross-check coin here (sequentially, before the
        // rollouts) keeps the search deterministic in `(config, base_seed, rng)`.
        let crosscheck_fraction = config.prefilter_crosscheck_fraction.clamp(0.0, 1.0);
        let rng = &mut self.rng;
        let gates: Vec<(Option<Cliff>, bool)> = self
            .batch
            .iter()
            .map(|unit| {
                let (wp, _) = decode(unit, &config.ranges);
                let cliff = prefilter_cliff(&wp);
                let crosscheck = cliff.is_some()
                    && crosscheck_fraction > 0.0
                    && rng.random::<f32>() < crosscheck_fraction;
                (cliff, crosscheck)
            })
            .collect();

        // Run only the rollouts that are actually needed: cleared configs, and the
        // gated configs sampled for the cross-check. Gated-and-skipped configs run
        // no sim — that is the saved budget.
        let evals: Vec<(Option<ConfigEval>, usize)> = self
            .batch
            .iter()
            .zip(seeds.iter())
            .zip(gates.iter())
            .map(|((unit, &seed), &(cliff, crosscheck))| {
                if cliff.is_some() && !crosscheck {
                    (None, 0)
                } else {
                    let (wp, dist) = decode(unit, &config.ranges);
                    let result = run_ensemble_within(
                        &wp,
                        &dist,
                        ensemble_config,
                        seed,
                        config.rollout_budget,
                    );
                    // A config no seed finished has no verdict (#562): it is
                    // placed nowhere, and only its unfinished seeds count.
                    if result.run_results.is_empty() {
                        return (None, result.unfinished);
                    }
                    let mut eval = config_eval_from_ensemble(&result);
                    // The early-stop cross-check rides on every rolled-out
                    // ensemble: seeds a dead-pool gate stopped that the carry
                    // took to the horizon are compared, and a stopped-dead /
                    // alive-at-T disagreement is surfaced (#506). The verdict
                    // the archive sees is unchanged by the carry.
                    eval.early_stop_crosscheck = early_stop_crosscheck(&result, unit);
                    // Predicted bifurcation coordinates — a closed-form reading of
                    // the decoded `(WorldParameters, founder mean)`, negligible vs
                    // the rollout (~5–10 ms vs ~0.85 s). Surfaced only for live
                    // cells; a gated cross-check rollout discards them.
                    eval.predicted_oscillation_distance =
                        oscillation_distance(&wp, &dist.mean_traits);
                    eval.predicted_branching_distance = branching_distance(&wp, &dist.mean_traits);
                    (Some(eval), result.unfinished)
                }
            })
            .collect();
        self.rollouts_unfinished += evals.iter().map(|&(_, n)| n).sum::<usize>();
        let evals: Vec<Option<ConfigEval>> = evals.into_iter().map(|(e, _)| e).collect();

        // Route each config and compute the emitter's improvement signal.
        let improvements: Vec<f32> = self
            .batch
            .iter()
            .zip(gates.iter())
            .zip(evals.iter())
            .map(|((unit, &(cliff, crosscheck)), eval)| {
                // Every rolled-out ensemble carries its early-stop cross-check
                // (#506), whether the config was cleared or a prefilter
                // cross-check: tally the carried seeds and surface the
                // stopped-dead / alive-at-T disagreements, never swallowed.
                if let Some(ev) = eval {
                    self.early_stop_crosschecks += ev.early_stop_crosscheck.carried;
                    self.early_stop_disagreements
                        .extend(ev.early_stop_crosscheck.disagreements.iter().cloned());
                }
                match cliff {
                    Some(cliff) => {
                        // Proven dead a priori: it lands on the dead frontier as an
                        // a-priori death regardless of whether it was cross-checked.
                        self.archive.insert_apriori(cliff);
                        if crosscheck {
                            // The cross-check rolled it out: if the rollout shows life,
                            // the prefilter and the rollout disagree — surface it (a
                            // mis-drawn gate), never swallow it.
                            if let Some(ev) = eval {
                                if let Some(d) = crosscheck_disagreement(cliff, ev, unit) {
                                    self.prefilter_disagreements.push(d);
                                }
                            }
                        } else {
                            self.rollouts_skipped += 1;
                        }
                        // An a-priori death never improves a cell.
                        0.0
                    }
                    None => {
                        // A live config: cross-check its predicted distance-to-
                        // bifurcation against the observed behaviour-axis boundaries
                        // before placing it (the check is per-config, independent of
                        // whether it wins its cell). Disagreements are surfaced with a
                        // regime tag, never swallowed.
                        // An unfinished config never improves a cell.
                        let Some(ev) = eval.as_ref() else {
                            return 0.0;
                        };
                        self.bifurcation_disagreements
                            .extend(bifurcation_crosscheck(ev));
                        self.archive.insert(unit, ev)
                    }
                }
            })
            .collect();

        let rolled_out = gates
            .iter()
            .filter(|&&(cliff, crosscheck)| cliff.is_none() || crosscheck)
            .count();
        (
            improvements,
            rolled_out,
            self.rollouts_skipped - skipped_before,
        )
    }

    /// Emit the next generation's batch. When the archive is still empty the
    /// emitter has nothing to centre on, so it keeps sampling its (widening)
    /// Gaussian — the bootstrap continues.
    fn emit_next_batch(&mut self, config: &QdConfig) {
        let dims = config.ranges.len();
        let elites = self.archive.elite_units();
        let rng = &mut self.rng;
        let emitter = &self.emitter;
        self.batch = (0..config.batch)
            .map(|_| {
                if elites.is_empty() {
                    emitter.sample(rng)
                } else {
                    // Re-centre the emitter draw on a random elite (the MAP-Elites
                    // parent-selection step) while keeping the adapted per-dimension
                    // deviations — the covariance the emitter has learned.
                    let parent = &elites[rng.random_range(0..elites.len())];
                    (0..dims)
                        .map(|d| (parent[d] + emitter.sigma[d] * gaussian(rng)).clamp(0.0, 1.0))
                        .collect()
                }
            })
            .collect();
    }
}

impl<R> SearchState<R> {
    /// Overwrite one emitter deviation — how tests reach a state the search
    /// itself should never produce (a non-finite value).
    #[cfg(test)]
    pub(crate) fn poison_emitter_for_test(&mut self, sigma: f64) {
        self.emitter.sigma[0] = sigma;
    }

    /// The atlas projection of the archive plus the surfaced tallies.
    fn atlas(&self) -> Atlas {
        let mut cells: Vec<AtlasCell> = self
            .archive
            .cells()
            .map(|(idx, rec)| AtlasCell {
                cell: [idx.0, idx.1, idx.2],
                fitness: rec.fitness,
                oscillation: rec.descriptors.oscillation,
                clustering: rec.descriptors.clustering,
                carcass: rec.descriptors.carcass,
                decomposer_fraction: rec.decomposer_fraction,
                consumer_fraction: rec.consumer_fraction,
                coexistence_fraction: rec.coexistence_fraction,
                sample_count: rec.sample_count,
                predicted_oscillation_distance: rec.predicted_oscillation_distance,
                predicted_branching_distance: rec.predicted_branching_distance,
                unit: rec.unit.clone(),
            })
            .collect();
        cells.sort_by_key(|c| c.cell);

        Atlas {
            provenance: None,
            search_box: None,
            coverage: self.archive.coverage(),
            total_cells: RESOLUTION.pow(3),
            qd_score: self.archive.qd_score(),
            best_fitness: self.archive.best_fitness(),
            dead_frontier: self.archive.dead_frontier(),
            dead_frontier_apriori: self.archive.dead_frontier_apriori(),
            rollouts_skipped: self.rollouts_skipped,
            rollouts_unfinished: self.rollouts_unfinished,
            prefilter_disagreements: self.prefilter_disagreements.clone(),
            bifurcation_disagreements: self.bifurcation_disagreements.clone(),
            early_stop_crosschecks: self.early_stop_crosschecks,
            early_stop_disagreements: self.early_stop_disagreements.clone(),
            cells,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_horizon_is_the_settled_community_working_value() {
        // genesis-search.md, *Current values*: `T = 2000` is the working horizon
        // (#507). The search, the refinement and the CLI all read it from these
        // defaults, so the three must agree — one place each, no hard-coded copy.
        assert_eq!(QdConfig::default().max_ticks, 2000);
        assert_eq!(RefinementConfig::default().max_ticks, 2000);
        assert_eq!(
            crate::search::SearchConfig::default().max_ticks,
            QdConfig::default().max_ticks
        );
    }

    fn descr(o: f32, c: f32, k: f32) -> Descriptors {
        Descriptors {
            oscillation: o,
            clustering: c,
            carcass: k,
        }
    }

    fn live(fitness: f32, d: Descriptors) -> ConfigEval {
        ConfigEval {
            median_fitness: fitness,
            cliff: None,
            descriptors: d,
            decomposer_fraction: 0.0,
            consumer_fraction: 0.0,
            coexistence_fraction: 1.0,
            sample_count: 5,
            coexistence_duration: 0.0,
            predicted_oscillation_distance: 0.0,
            predicted_branching_distance: 0.0,
            early_stop_crosscheck: EarlyStopCrosscheck::default(),
        }
    }

    #[test]
    fn carcass_seed_unit_drives_the_carcass_axis_extremes() {
        // The carcass-directed seed must push the dims that empirically accumulate
        // dead-pool nutrient toward their carcass-favouring extremes: maximal
        // photosynthetic production feeding biomass, ZERO heterotrophy so nothing
        // turns the dead pool over, and high nutrient-per-structure so the death
        // flux strands nutrient in carcasses. This is the seed the bootstrap injects
        // to reach the thin high-carcass / nutrient-lockup region (genesis-search.md,
        // "reaching that thin high-carcass region is the emitter's job").
        let ranges = default_ranges();
        let unit = carcass_seed_unit(&ranges);
        assert_eq!(unit.len(), ranges.len());
        for &u in &unit {
            assert!(
                (0.0..=1.0).contains(&u),
                "every seed coord stays in the cube"
            );
        }
        let at = |name: &str| {
            let i = ranges.iter().position(|r| r.name == name).unwrap();
            unit[i]
        };
        // Production maxed, decomposition off: the dead pool fills and nothing
        // recovers it.
        assert!(at("mean_photosynthetic_absorption") > 0.9);
        assert!(at("mean_heterotrophy") < 0.1);
        // Nutrient rides into structure (and thus carcasses) at a high ratio.
        assert!(at("base_nutrient_ratio") > 0.9);
        assert!(at("specification_nutrient_coefficient") > 0.9);
    }

    fn atlas_cell(carcass: f32) -> AtlasCell {
        AtlasCell {
            cell: cell_of(&descr(0.0, 0.0, carcass)).into(),
            fitness: 0.3,
            oscillation: 0.0,
            clustering: 0.0,
            carcass,
            decomposer_fraction: 0.0,
            consumer_fraction: 0.0,
            coexistence_fraction: 1.0,
            sample_count: 5,
            predicted_oscillation_distance: 0.0,
            predicted_branching_distance: 0.0,
            unit: vec![0.5; 3],
        }
    }

    fn atlas_with(cells: Vec<AtlasCell>, lockup_deaths: usize) -> Atlas {
        let mut frontier = std::collections::BTreeMap::new();
        if lockup_deaths > 0 {
            frontier.insert(Cliff::NutrientLockup.label().to_string(), lockup_deaths);
        }
        Atlas {
            provenance: None,
            search_box: None,
            coverage: cells.len(),
            total_cells: RESOLUTION.pow(3),
            qd_score: 0.0,
            best_fitness: 0.0,
            dead_frontier: frontier,
            dead_frontier_apriori: std::collections::BTreeMap::new(),
            rollouts_skipped: 0,
            rollouts_unfinished: 0,
            prefilter_disagreements: Vec::new(),
            bifurcation_disagreements: Vec::new(),
            early_stop_crosschecks: 0,
            early_stop_disagreements: Vec::new(),
            cells,
        }
    }

    #[test]
    fn lockup_crosscheck_splits_live_cells_at_the_lock_gate() {
        // The lockup boundary cross-check (genesis-search.md; the #363 spike's
        // promoted check): a `NutrientLockup`-gated config sits at high carcass
        // fraction, and live cells *mostly* sit BELOW the LOCK_FRACTION gate (a
        // live world is, by definition, not locked). The helper reports the split.
        // Three healthy-throughput cells (carcass well below 0.4) and one near-cliff
        // live cell just below the gate, with one lockup death on the frontier.
        let cells = vec![
            atlas_cell(0.05),
            atlas_cell(0.12),
            atlas_cell(0.30),
            atlas_cell(0.38),
        ];
        let atlas = atlas_with(cells, 1);
        let check = atlas.lockup_boundary_crosscheck();

        // The lock fraction the search cross-checks against matches the evaluator's
        // gate exactly (explorers-genesis-eval, LOCK_FRACTION = 0.4).
        assert!((LOCK_FRACTION - 0.4).abs() < 1e-6);
        // Every live cell is below the gate (none is locked — a locked world is
        // gated, never a cell).
        assert_eq!(check.live_below_gate, 4);
        assert_eq!(check.live_at_or_above_gate, 0);
        // Two of them are in the high-carcass band near the cliff (≥ 0.2).
        assert_eq!(check.live_high_carcass, 2);
        // The lockup layer is non-empty: a NutrientLockup death is on the frontier.
        assert_eq!(check.lockup_frontier_deaths, 1);
        assert!(check.lockup_layer_populated());
    }

    #[test]
    fn lockup_crosscheck_flags_a_live_cell_above_the_gate_as_a_boundary_violation() {
        // A live cell at/above the lock fraction is a boundary violation: a world
        // the evaluator should have gated as nutrient-locked yet which reads live.
        // The helper must surface it (live_at_or_above_gate > 0), not hide it.
        let cells = vec![atlas_cell(0.1), atlas_cell(0.45)];
        let atlas = atlas_with(cells, 0);
        let check = atlas.lockup_boundary_crosscheck();
        assert_eq!(check.live_at_or_above_gate, 1);
        assert_eq!(check.live_below_gate, 1);
        // No lockup death tallied → the lockup layer is reached only via the live
        // high-carcass cell, not the frontier here.
        assert_eq!(check.lockup_frontier_deaths, 0);
    }

    #[test]
    fn carcass_seeds_replace_the_head_of_the_bootstrap_batch() {
        // Carcass direction injects `count` carcass-directed seeds at the head of
        // the bootstrap batch, replacing the random draws there, and leaves the
        // tail untouched (the random exploration the bootstrap still needs). With
        // count 0 the batch is unchanged.
        let ranges = default_ranges();
        let seed = carcass_seed_unit(&ranges);

        let mut batch: Vec<Vec<f64>> = vec![vec![0.5; ranges.len()]; 4];
        // Mark the random tail so we can prove it survives.
        for (k, row) in batch.iter_mut().enumerate() {
            row[0] = 0.123 + k as f64 * 0.01;
        }
        let original_tail = batch[2..].to_vec();

        inject_carcass_seeds(&mut batch, &ranges, 2);

        assert_eq!(batch[0], seed, "first slot is a carcass seed");
        assert_eq!(batch[1], seed, "second slot is a carcass seed");
        assert_eq!(
            &batch[2..],
            &original_tail[..],
            "the random tail is untouched"
        );

        // count 0 is a no-op (carcass direction off).
        let mut untouched = original_tail.clone();
        let before = untouched.clone();
        inject_carcass_seeds(&mut untouched, &ranges, 0);
        assert_eq!(untouched, before);

        // count larger than the batch fills every slot, never panics.
        let mut small = vec![vec![0.5; ranges.len()]; 1];
        inject_carcass_seeds(&mut small, &ranges, 5);
        assert_eq!(small[0], seed);
    }

    #[test]
    fn cell_of_clamps_and_bins() {
        assert_eq!(cell_of(&descr(0.0, 0.0, 0.0)), (0, 0, 0));
        assert_eq!(
            cell_of(&descr(1.0, 1.0, 1.0)),
            (RESOLUTION - 1, RESOLUTION - 1, RESOLUTION - 1)
        );
        assert_eq!(
            cell_of(&descr(-0.5, 2.0, 0.5)),
            (0, RESOLUTION - 1, RESOLUTION / 2)
        );
    }

    #[test]
    fn live_config_fills_a_cell_and_better_elite_wins() {
        // Classic-elitism archive (learning rate 0): a live config takes its cell,
        // a worse config in the same cell is rejected, a better one displaces it.
        let mut archive = Archive::new(0.0);
        let unit = vec![0.5; 3];
        archive.insert(&unit, &live(0.3, descr(0.5, 0.6, 0.2)));
        assert_eq!(archive.coverage(), 1);

        archive.insert(&unit, &live(0.1, descr(0.5, 0.6, 0.2)));
        assert!((archive.best_fitness() - 0.3).abs() < 1e-6);

        archive.insert(&unit, &live(0.42, descr(0.5, 0.6, 0.2)));
        assert_eq!(archive.coverage(), 1);
        assert!((archive.best_fitness() - 0.42).abs() < 1e-6);
    }

    #[test]
    fn gated_config_goes_to_frontier_not_a_cell() {
        let mut archive = Archive::new(0.5);
        let unit = vec![0.1; 3];
        let mut eval = live(0.0, descr(0.0, 0.0, 0.0));
        eval.cliff = Some(Cliff::Monoculture);
        archive.insert(&unit, &eval);
        assert_eq!(archive.coverage(), 0);
        assert_eq!(archive.dead_frontier().get("monoculture"), Some(&1));
    }

    #[test]
    fn apriori_death_is_tallied_separately_from_observed_death() {
        // A prefilter-gated config lands on the dead frontier as an *a priori*
        // death (spending no ensemble); an ensemble-observed death lands as an
        // *observed* death. The total frontier counts both; the a-priori layer
        // counts only the prefiltered ones (genesis-search.md, a-priori vs
        // observed).
        let mut archive = Archive::new(0.5);

        archive.insert_apriori(Cliff::Extinction);
        archive.insert_apriori(Cliff::Extinction);
        archive.insert_apriori(Cliff::EnergyDeath);

        // An observed extinction (a config that cleared the prefilter but died in
        // rollout) is tallied to the same cliff, but as observed.
        let mut observed = live(0.0, descr(0.0, 0.0, 0.0));
        observed.cliff = Some(Cliff::Extinction);
        archive.insert(&vec![0.5; 3], &observed);

        // Total frontier: 3 extinction (2 a priori + 1 observed) + 1 energy death.
        assert_eq!(archive.dead_frontier().get("extinction"), Some(&3));
        assert_eq!(archive.dead_frontier().get("energy_death"), Some(&1));

        // A-priori layer: only the prefiltered deaths.
        assert_eq!(archive.dead_frontier_apriori().get("extinction"), Some(&2));
        assert_eq!(
            archive.dead_frontier_apriori().get("energy_death"),
            Some(&1)
        );
    }

    #[test]
    fn soft_threshold_lags_the_elite_so_near_misses_still_accept() {
        // CMA-MAE: the rolling threshold lags the elite by the learning rate, so a
        // solution that beats the *threshold* but not the sitting elite is still
        // accepted as an improvement (descriptor-noise tolerance) — unlike hard
        // elitism, which would reject it. After a strong elite at 0.8 with α=0.5,
        // the threshold sits at 0.8 only if α=1; with α=0.5 a follow-up at 0.6
        // clears the lagging threshold.
        let mut archive = Archive::new(0.5);
        let cell = descr(0.5, 0.5, 0.5);
        // Seed the cell low so the threshold starts low, then push a higher elite.
        archive.insert(&vec![0.5; 3], &live(0.4, cell));
        // threshold now 0.4, elite 0.4. A 0.8 solution: improvement 0.4 > 0,
        // threshold -> 0.4 + 0.5*(0.8-0.4) = 0.6, elite -> 0.8.
        let imp = archive.insert(&vec![0.6; 3], &live(0.8, cell));
        assert!(imp > 0.0);
        assert!((archive.best_fitness() - 0.8).abs() < 1e-6);
        // A 0.65 solution clears the lagging threshold (0.6) though it is below the
        // 0.8 elite: soft acceptance. Hard elitism (α=0) would reject it.
        let imp2 = archive.insert(&vec![0.55; 3], &live(0.65, cell));
        assert!(
            imp2 > 0.0,
            "a near-miss above the lagging threshold should be accepted"
        );
        // The elite is unchanged (0.65 < 0.8) — the threshold moved, the elite did not.
        assert!((archive.best_fitness() - 0.8).abs() < 1e-6);
    }

    #[test]
    fn a_restored_archive_keeps_each_cells_lagging_acceptance_threshold() {
        // #530: the rolling threshold is invisible in the atlas, so a checkpoint
        // that dropped it (resetting it to the elite, or to zero) would change
        // acceptance silently from the resume on. Restore the archive from its
        // serialised form and a near-miss is judged exactly as before.
        let mut archive = Archive::new(0.5);
        let cell = descr(0.5, 0.5, 0.5);
        archive.insert(&[0.5; 3], &live(0.4, cell));
        archive.insert(&[0.6; 3], &live(0.8, cell));
        // Threshold 0.6, elite 0.8: the threshold lags.

        let json = serde_json::to_string(&archive).unwrap();
        let mut restored: Archive = serde_json::from_str(&json).unwrap();

        let near_miss = live(0.65, cell);
        let original = archive.insert(&[0.55; 3], &near_miss);
        let resumed = restored.insert(&[0.55; 3], &near_miss);
        assert!(original > 0.0, "the near-miss clears the lagging threshold");
        assert_eq!(
            resumed, original,
            "the restored threshold must be the original"
        );
        // And the thresholds moved identically, so the next insert agrees too.
        let next = live(0.7, cell);
        assert_eq!(
            restored.insert(&[0.52; 3], &next),
            archive.insert(&[0.52; 3], &next)
        );
    }

    #[test]
    fn qd_score_and_coverage_sum_over_filled_cells() {
        let mut archive = Archive::new(0.0);
        archive.insert(&vec![0.1; 3], &live(0.3, descr(0.1, 0.1, 0.1)));
        archive.insert(&vec![0.9; 3], &live(0.5, descr(0.9, 0.9, 0.9)));
        assert_eq!(archive.coverage(), 2);
        assert!((archive.qd_score() - 0.8).abs() < 1e-6);
    }

    #[test]
    fn emitter_adapts_mean_toward_improvers() {
        // The covariance-adapting emitter must move its mean toward the batch's
        // improving solutions — the behaviour that replaces Sobol dimension
        // fixing. Two improvers clustered near 0.8 pull the mean (started at 0.5)
        // upward on that dimension.
        let mut emitter = CmaEmitter::new(1, 0.15);
        let batch = vec![vec![0.8], vec![0.82], vec![0.1]];
        let improvements = vec![0.3, 0.4, 0.0]; // the third did not improve
        emitter.adapt(&batch, &improvements);
        assert!(
            emitter.mean[0] > 0.5,
            "mean should move toward the improvers near 0.8, got {}",
            emitter.mean[0]
        );
    }

    #[test]
    fn prefilter_routes_apriori_deaths_and_skips_their_rollouts() {
        // With ranges that force every config below the extinction flux floor
        // (F ≤ B everywhere), the prefilter must route every config to the dead
        // frontier as an a-priori extinction, spend ZERO ensembles, and report the
        // full batch as budget saved. No live cell can form.
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        // Pin solar_flux_magnitude ≤ base_metabolic_rate across the whole cube.
        let mut ranges = default_ranges();
        for r in &mut ranges {
            if r.name == "solar_flux_magnitude" {
                r.min = 0.05;
                r.max = 0.1;
            }
            if r.name == "base_metabolic_rate" {
                r.min = 0.4;
                r.max = 0.5;
            }
        }

        let config = QdConfig {
            ranges,
            ensemble_size: 3,
            max_ticks: 50,
            batch: 5,
            generations: 1,
            ..QdConfig::default()
        };

        let mut rng = ChaCha8Rng::seed_from_u64(1);
        let atlas = run_qd(&config, 1, &mut rng);

        // Two batches × 5 = 10 configs, every one prefiltered extinct a priori.
        let total_evaluated = config.batch * (config.generations + 1);
        assert_eq!(atlas.coverage, 0, "no live cell can form when all gated");
        assert_eq!(
            atlas.dead_frontier_apriori.get("extinction"),
            Some(&total_evaluated),
            "every config should be an a-priori extinction"
        );
        // Budget saved == every rollout skipped (none were run).
        assert_eq!(atlas.rollouts_skipped, total_evaluated);
        // No observed deaths — the prefilter caught them all, the frontier total
        // equals the a-priori count.
        let dead: usize = atlas.dead_frontier.values().sum();
        assert_eq!(dead, total_evaluated);
    }

    #[test]
    fn crosscheck_surfaces_disagreement_when_a_gated_config_shows_life() {
        // The cross-check must SURFACE — not swallow — the case where the
        // prefilter proved a config dead but the rollout shows life (positive
        // fitness, no failure). That case localises a mis-drawn gate.
        let unit = vec![0.5; 3];
        let alive = live(0.42, descr(0.4, 0.5, 0.3)); // cliff None, fitness > 0
        let d = crosscheck_disagreement(Cliff::Extinction, &alive, &unit)
            .expect("prefilter-dead but rollout-alive must be surfaced");
        assert_eq!(d.predicted_cliff, "extinction");
        assert!((d.observed_fitness - 0.42).abs() < 1e-6);
        assert_eq!(d.unit, unit);
    }

    #[test]
    fn crosscheck_is_silent_when_the_rollout_agrees_with_the_gate() {
        // Agreement (the rollout also dies) is NOT a disagreement — a correct gate
        // produces no false alarm.
        let unit = vec![0.5; 3];
        let mut dead = live(0.0, descr(0.0, 0.0, 0.0));
        dead.cliff = Some(Cliff::Extinction);
        assert!(crosscheck_disagreement(Cliff::Extinction, &dead, &unit).is_none());

        // A zero-fitness "live" verdict (no failure but no positive fitness) is not
        // counted as life either — only positive fitness contradicts a dead gate.
        let zero = live(0.0, descr(0.0, 0.0, 0.0));
        assert!(crosscheck_disagreement(Cliff::EnergyDeath, &zero, &unit).is_none());
    }

    #[test]
    fn slow_full_crosscheck_rolls_out_every_gated_config_and_confirms_the_gates() {
        // With the cross-check fraction at 1.0, every prefilter-gated config is
        // *also* rolled out (none are taken on faith) — so no budget is skipped.
        // Because the extinction gate is correct, the rollouts agree (they die),
        // so NO disagreement is surfaced: a true gate produces an empty
        // disagreement list. `slow_` — it steps real sims for every config.
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let mut ranges = default_ranges();
        for r in &mut ranges {
            if r.name == "solar_flux_magnitude" {
                r.min = 0.05;
                r.max = 0.1;
            }
            if r.name == "base_metabolic_rate" {
                r.min = 0.4;
                r.max = 0.5;
            }
        }

        // Founders start with up to 50 energy (`initial_energy_per_agent`'s
        // range top) and lose at least `B ≥ 0.4` per tick under `F ≤ B`, so a
        // rollout must run past 125 ticks to observe the guaranteed extinction;
        // 40 ticks used to suffice only because most founders were dead on
        // arrival with a NaN reserve (#444).
        let config = QdConfig {
            ranges,
            ensemble_size: 2,
            max_ticks: 160,
            batch: 4,
            generations: 1,
            prefilter_crosscheck_fraction: 1.0,
            ..QdConfig::default()
        };

        let mut rng = ChaCha8Rng::seed_from_u64(3);
        let atlas = run_qd(&config, 3, &mut rng);

        let total = config.batch * (config.generations + 1);
        // Every gated config was cross-checked, so nothing was skipped.
        assert_eq!(
            atlas.rollouts_skipped, 0,
            "full cross-check rolls out every gated config — no budget saved"
        );
        // Still all a-priori extinctions (the verdict the prefilter assigned).
        assert_eq!(atlas.dead_frontier_apriori.get("extinction"), Some(&total));
        // The gate is correct: rollouts of forced-extinct configs show no life,
        // so the disagreement list is empty (agreement, not swallowed silence).
        assert!(
            atlas.prefilter_disagreements.is_empty(),
            "a correct gate must surface zero disagreements, got {:?}",
            atlas.prefilter_disagreements
        );
    }

    #[test]
    fn slow_carcass_direction_populates_the_lockup_layer_across_a_seed_sweep() {
        // Acceptance (#367): with carcass direction on, the atlas's lockup layer is
        // NON-EMPTY across a multi-seed sweep — live cells in the high-carcass band
        // near the cliff and/or a NutrientLockup dead-frontier entry — where the
        // #363 random emitter reached neither (0 lockup-frontier configs, 0
        // high-carcass live cells). And the lockup boundary cross-check holds:
        // gated lockup configs sit at high carcass (on the frontier, never a cell)
        // while live cells sit mostly below the LOCK_FRACTION gate. `slow_` — it
        // steps real sims over several seeds.
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        // Carcass accumulates over a long trailing window, so the rollout needs
        // enough ticks for the dead pool to build; the ensemble and batch stay
        // small to keep the sweep foreground-fast. Under the binary-reach drain
        // (#380) any heterotroph that reaches a carcass now drains it every tick
        // (no contact-duration warm-up), so the dead pool turns over faster than it
        // did under the ramp bug — the rollout needs a longer horizon (600 ticks)
        // and a wider seed sweep to still land a config in the thin high-carcass
        // band the lockup layer lives in.
        let config = QdConfig {
            ensemble_size: 2,
            max_ticks: 600,
            batch: 6,
            generations: 2,
            carcass_seed_count: 3,
            ..QdConfig::default()
        };

        let mut any_populated = false;
        let mut total_live = 0usize;
        let mut total_below = 0usize;
        let mut total_above = 0usize;
        let mut best_live_carcass = 0.0f32;
        // Sweep seed 37 was retired under #503: with the behaviour axes read
        // off the settled window the emitter's path on that seed reaches, in
        // its last generation, a knife-edge config whose second ensemble seed
        // blooms past 5,000 agents by tick 200 (the sibling seed peaks at 41)
        // and the sim then costs seconds per tick — a cost problem at high
        // density, not a property this test is about.
        for &seed in &[11_u64, 23, 41, 5, 19] {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let atlas = run_qd(&config, seed, &mut rng);
            let check = atlas.lockup_boundary_crosscheck();
            any_populated |= check.lockup_layer_populated();
            total_live += atlas.cells.len();
            total_below += check.live_below_gate;
            total_above += check.live_at_or_above_gate;
            best_live_carcass = atlas
                .cells
                .iter()
                .map(|c| c.carcass)
                .fold(best_live_carcass, f32::max);
        }

        assert!(
            any_populated,
            "carcass direction should populate the lockup layer in at least one \
             seed (high-carcass live cell or a NutrientLockup frontier death); the \
             best live carcass fraction reached was {best_live_carcass:.3}"
        );
        // The carcass axis is genuinely exercised: a live world reaches the
        // high-carcass band near the cliff (≥ half the gate), where the #363 random
        // emitter topped out far below (best carcass ~0.03).
        assert!(
            best_live_carcass >= LOCK_FRACTION / 2.0,
            "a live cell should reach the high-carcass band near the cliff, got \
             best carcass {best_live_carcass:.3} (gate {LOCK_FRACTION})"
        );

        // The lockup boundary cross-check: live cells sit MOSTLY below the gate (a
        // live world is not locked). Allow the occasional descriptor-noise straggler
        // but the majority must be healthy-throughput.
        assert!(
            total_live == 0 || total_below * 2 >= total_live,
            "most live cells should sit below the LOCK_FRACTION gate ({total_below} \
             below, {total_above} at/above, {total_live} total)"
        );
    }

    #[test]
    fn slow_smoke_search_at_the_default_horizon_reads_live_cells_over_the_settled_window() {
        // End-to-end at the settled-community horizon (#507): a reduced search
        // (tiny batch, no adaptation generations, one seed per config) runs the
        // pipeline at the DEFAULT `max_ticks` — the search bin's working value —
        // and every config lands on a live cell, the dead frontier, or the
        // prefilter's skip tally. `slow_` — real 2000-tick sims, but few.
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let horizon = QdConfig::default().max_ticks;
        let config = QdConfig {
            ensemble_size: 1,
            batch: 3,
            generations: 0,
            ..QdConfig::default()
        };
        let base_seed = 11;
        let mut rng = ChaCha8Rng::seed_from_u64(base_seed);
        let atlas = run_qd(&config, base_seed, &mut rng);

        let dead: usize = atlas.dead_frontier.values().sum();
        assert!(
            atlas.coverage + dead + atlas.rollouts_skipped >= 1,
            "every config routes to a cell, the frontier or the skip tally"
        );
        assert!(
            !atlas.cells.is_empty(),
            "the smoke search should illuminate at least one live cell to read"
        );

        // A live cell's guild fractions are the evaluator's reads over the
        // settled window `(T/2, T]` of the rollout that placed it. With one seed
        // per config the placing rollout is recoverable: bootstrap config `i`
        // rolls seed `base_seed + i*1000`, and the cell's recorded fitness is
        // that run's fitness bit-for-bit. Re-running it at the horizon must
        // reach `T` (a live seed is alive at `T`, so the window `(1000, 2000]`
        // is fully populated) and reproduce the guild reads the cell carries.
        for cell in &atlas.cells {
            assert_eq!(cell.sample_count, 1);
            let (wp, dist) = decode(&cell.unit, &config.ranges);
            let run_config = RunConfig {
                max_ticks: horizon,
                eval_config: EvalConfig::default(),
                early_stop_crosscheck_fraction: 0.0,
            };
            let placing = (0..config.batch as u64)
                .map(|i| {
                    explorers_genesis::run_single(&wp, &dist, &run_config, base_seed + i * 1000)
                })
                .find(|r| r.fitness.to_bits() == cell.fitness.to_bits())
                .expect("the rollout that placed the live cell is one of the bootstrap seeds");
            assert_eq!(
                placing.termination_tick, 2000,
                "a live seed reaches the horizon"
            );
            assert!(placing.failure.is_none());
            let read = |guild: bool| if guild { 1.0f32 } else { 0.0 };
            assert_eq!(
                cell.decomposer_fraction,
                read(placing.breakdown.has_decomposer_guild)
            );
            assert_eq!(
                cell.consumer_fraction,
                read(placing.breakdown.has_consumer_guild)
            );
        }
    }

    #[test]
    fn slow_run_qd_produces_an_atlas_reproducibly() {
        // End-to-end tracer bullet: a tiny QD search illuminates into an Atlas,
        // routing every config to either a live cell or the dead frontier, and is
        // bit-reproducible for a fixed (config, base_seed, rng). `slow_` — it steps
        // real sims.
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let config = QdConfig {
            ensemble_size: 1,
            max_ticks: 20,
            batch: 4,
            generations: 1,
            ..QdConfig::default()
        };

        let mut rng1 = ChaCha8Rng::seed_from_u64(42);
        let atlas1 = run_qd(&config, 42, &mut rng1);
        let mut rng2 = ChaCha8Rng::seed_from_u64(42);
        let atlas2 = run_qd(&config, 42, &mut rng2);

        // Every config (2 batches × 4) lands somewhere: live cells (coverage,
        // which may collapse when configs share a cell) + dead-frontier tallies.
        let dead: usize = atlas1.dead_frontier.values().sum();
        assert!(
            atlas1.coverage + dead >= 1,
            "the search should route at least one config to a cell or the frontier"
        );
        assert_eq!(atlas1.total_cells, RESOLUTION.pow(3));

        // Reproducible: same coverage, same dead tally, same best fitness.
        assert_eq!(atlas1.coverage, atlas2.coverage);
        assert_eq!(atlas1.dead_frontier, atlas2.dead_frontier);
        assert_eq!(atlas1.best_fitness, atlas2.best_fitness);

        // The predicted bifurcation coordinates on live cells are finite and
        // bit-reproducible, and every surfaced bifurcation disagreement carries a
        // regime tag (never an untagged disagreement). (The new #372 fields.)
        let coords = |atlas: &Atlas| -> Vec<(f32, f32)> {
            atlas
                .cells
                .iter()
                .map(|c| {
                    assert!(
                        c.predicted_oscillation_distance.is_finite()
                            && c.predicted_branching_distance.is_finite(),
                        "live-cell predicted coords must be finite"
                    );
                    (
                        c.predicted_oscillation_distance,
                        c.predicted_branching_distance,
                    )
                })
                .collect()
        };
        assert_eq!(
            coords(&atlas1),
            coords(&atlas2),
            "predicted coords must be reproducible"
        );
        assert_eq!(
            atlas1.bifurcation_disagreements.len(),
            atlas2.bifurcation_disagreements.len(),
            "the disagreement set must be reproducible"
        );
        for d in &atlas1.bifurcation_disagreements {
            assert!(
                matches!(
                    d.regime,
                    CrosscheckRegime::Validated | CrosscheckRegime::WeakObservable
                ),
                "every bifurcation disagreement must carry a regime tag"
            );
        }
    }

    #[test]
    fn slow_run_qd_reports_each_generation_as_it_completes() {
        // #529: the search reports each completed generation to its caller —
        // the bootstrap plus every adaptation generation, in order.
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let config = QdConfig {
            ensemble_size: 1,
            max_ticks: 20,
            batch: 4,
            generations: 2,
            ..QdConfig::default()
        };
        let mut reports: Vec<GenerationReport> = Vec::new();
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        run_qd_observed(&config, 42, &mut rng, &mut |r: &GenerationReport| {
            reports.push(r.clone())
        });

        assert_eq!(reports.len(), config.generations + 1);
        for (i, r) in reports.iter().enumerate() {
            assert_eq!(r.generation, i, "generation index must increase by one");
            assert_eq!(r.generations, config.generations);
        }
    }

    #[test]
    fn slow_generation_reports_track_the_archive_and_the_rollout_budget() {
        // #529: each report reads the archive as the generation leaves it, so the
        // last one agrees with the atlas; every config in a batch is either
        // rolled out or skipped by the prefilter, and the skips add up to the
        // atlas's tally; wall-clock is cumulative.
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let config = QdConfig {
            ensemble_size: 1,
            max_ticks: 20,
            batch: 6,
            generations: 2,
            ..QdConfig::default()
        };
        let mut reports: Vec<GenerationReport> = Vec::new();
        let mut rng = ChaCha8Rng::seed_from_u64(9);
        let atlas = run_qd_observed(&config, 9, &mut rng, &mut |r: &GenerationReport| {
            reports.push(r.clone())
        });

        let last = reports.last().expect("at least one generation");
        assert_eq!(last.coverage, atlas.coverage);
        assert_eq!(last.total_cells, atlas.total_cells);
        assert_eq!(last.qd_score, atlas.qd_score);
        assert_eq!(last.best_fitness, atlas.best_fitness);

        for r in &reports {
            assert_eq!(r.rolled_out + r.skipped, config.batch);
        }
        let skipped: usize = reports.iter().map(|r| r.skipped).sum();
        assert_eq!(skipped, atlas.rollouts_skipped);

        let mut previous = std::time::Duration::ZERO;
        for r in &reports {
            assert!(r.elapsed >= r.generation_elapsed);
            assert!(r.elapsed >= previous + r.generation_elapsed);
            previous = r.elapsed;
        }
    }

    #[test]
    fn slow_observing_the_search_leaves_the_atlas_unchanged() {
        // #529: reporting only. The same-seed search with an observer installed
        // writes the identical atlas to the one written without — the observer
        // draws nothing from the rng and steers nothing.
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let config = QdConfig {
            ensemble_size: 1,
            max_ticks: 20,
            batch: 4,
            generations: 2,
            ..QdConfig::default()
        };
        // The atlas as written — in its canonical order (#536), so compared whole.
        let written = |atlas: &Atlas| serde_json::to_value(atlas).unwrap();

        let mut rng1 = ChaCha8Rng::seed_from_u64(42);
        let unobserved = run_qd(&config, 42, &mut rng1);
        let mut rng2 = ChaCha8Rng::seed_from_u64(42);
        let mut calls = 0;
        let observed = run_qd_observed(&config, 42, &mut rng2, &mut |_: &GenerationReport| {
            calls += 1
        });

        assert_eq!(calls, config.generations + 1);
        assert_eq!(written(&observed), written(&unobserved));
        // The rng is left in the same place, too.
        assert_eq!(rng1.random::<u64>(), rng2.random::<u64>());
    }

    #[test]
    fn a_generation_report_reads_as_one_progress_line() {
        // #529: the CLI prints one line per generation, read at a glance while
        // waiting — index of total, archive state, rollout budget, wall-clock.
        use std::time::Duration;
        let report = GenerationReport {
            generation: 3,
            generations: 10,
            coverage: 41,
            total_cells: 1000,
            qd_score: 12.3456,
            best_fitness: 0.61234,
            rolled_out: 28,
            skipped: 4,
            generation_elapsed: Duration::from_secs(192),
            elapsed: Duration::from_secs(3 * 3600 + 12 * 60 + 5),
            unfinished: 0,
        };
        assert_eq!(
            report.to_string(),
            "gen  3/10  coverage 41/1000 (4.10%)  qd 12.346  best 0.6123  \
             rolled out 28, skipped 4  took 3m12s  elapsed 3h12m05s"
        );
    }

    #[test]
    fn slow_decoder_midpoint_evaluates_live_in_the_archive() {
        // The validation triad's "reproduce a known-good config" check (the #363
        // spike's PASS): the decoder midpoint is the #326 known-viable baseline, so
        // running it through the *same* ensemble path the QD loop uses must land it
        // LIVE (a behaviour cell, not the dead frontier) with positive fitness. This
        // is the regression floor — the search can still find the viable manifold it
        // replaced LHS+BO to map.
        use explorers_genesis::{EnsembleConfig, EvalConfig, RunConfig, run_ensemble};

        let ranges = default_ranges();
        let unit = vec![0.5_f64; ranges.len()];
        let (wp, dist) = decode(&unit, &ranges);
        let ensemble_config = EnsembleConfig {
            ensemble_size: 5,
            run_config: RunConfig {
                max_ticks: 120,
                eval_config: EvalConfig::default(),
                early_stop_crosscheck_fraction: 0.0,
            },
        };
        let result = run_ensemble(&wp, &dist, &ensemble_config, 1000);
        let eval = config_eval_from_ensemble(&result);

        assert!(
            eval.cliff.is_none(),
            "the known-viable decoder midpoint should be LIVE, not on the dead \
             frontier (got cliff {:?})",
            eval.cliff
        );
        assert!(
            eval.median_fitness > 0.0,
            "the known-viable midpoint should score positive fitness, got {}",
            eval.median_fitness
        );
    }

    #[test]
    fn slow_atlas_yields_a_recipe_from_its_best_cell_when_any_live() {
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let config = QdConfig {
            ensemble_size: 1,
            max_ticks: 20,
            batch: 6,
            generations: 1,
            ..QdConfig::default()
        };
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        let atlas = run_qd(&config, 7, &mut rng);

        if let Some(recipe) = atlas.best_recipe(&config.ranges, config.max_ticks) {
            // A recipe drawn from the atlas round-trips through serde and decodes
            // to a non-degenerate world (the decoder's known-viable baseline).
            let json = serde_json::to_string(&recipe).unwrap();
            let recovered: explorers_sim::WorldRecipe = serde_json::from_str(&json).unwrap();
            assert_eq!(recipe, recovered);
            assert!(recipe.parameters.initial_population_size > 0);
            // The projected cell is reachable as a cell-specific recipe: the
            // highest-fitness cell clearing the coexistence floor, argmax fallback
            // when none clears (mirrors `best_recipe`'s floored pick, #401).
            let pick = atlas
                .cells
                .iter()
                .filter(|c| c.coexistence_fraction >= COEXISTENCE_FLOOR)
                .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap())
                .or_else(|| {
                    atlas
                        .cells
                        .iter()
                        .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap())
                })
                .unwrap();
            let cell_recipe = atlas
                .recipe_for_cell(pick.cell, &config.ranges, config.max_ticks)
                .unwrap();
            assert_eq!(recipe, cell_recipe);
        }
    }

    // --- Bifurcation cross-check (#372) -------------------------------------

    /// Build a live eval with explicit predicted coords + observed descriptors,
    /// for cross-check unit tests.
    fn live_with(
        clustering: f32,
        oscillation: f32,
        coexistence_duration: f32,
        predicted_branching: f32,
        predicted_oscillation: f32,
    ) -> ConfigEval {
        let mut ev = live(0.3, descr(oscillation, clustering, 0.1));
        ev.coexistence_duration = coexistence_duration;
        ev.predicted_branching_distance = predicted_branching;
        ev.predicted_oscillation_distance = predicted_oscillation;
        ev
    }

    #[test]
    fn branching_disagreement_in_the_small_n_regime_is_tagged_weak_observable() {
        // #359's known-weak signature: the predicted margin says COEXISTENCE
        // (D > 0) but the observed clustering_strength is 0 while coexistence
        // genuinely persists (coexistence_duration > 0) — the multi-peak test
        // silently zeroes below n=4. The disagreement must surface, tagged
        // WeakObservable (it localises to the observable, not F's reading).
        let eval = live_with(0.0, 0.0, 12.0, 0.5, -0.2);
        let ds = bifurcation_crosscheck(&eval);
        let b = ds
            .iter()
            .find(|d| d.axis == "branching")
            .expect("branching disagreement must surface");
        assert_eq!(b.regime, CrosscheckRegime::WeakObservable);
        assert!((b.predicted - 0.5).abs() < 1e-6);
        assert_eq!(b.observed, 0.0);
    }

    #[test]
    fn branching_disagreement_with_a_live_observable_is_tagged_validated() {
        // Predicted MONOCULTURE (D < 0) but the observed clustering_strength is
        // positive (a multi-peak coexistence reading the observable trusts here):
        // the disagreement implicates F's reading, so it is tagged Validated.
        let eval = live_with(0.4, 0.0, 8.0, -0.3, -0.2);
        let ds = bifurcation_crosscheck(&eval);
        let b = ds
            .iter()
            .find(|d| d.axis == "branching")
            .expect("branching disagreement must surface");
        assert_eq!(b.regime, CrosscheckRegime::Validated);
    }

    #[test]
    fn oscillation_disagreement_is_always_tagged_weak_observable() {
        // #358's verdict: oscillation_strength cannot adjudicate the Hopf crossing
        // at genesis scale, so an oscillation-axis disagreement is constant
        // WeakObservable. Here F predicts a limit cycle (|λ|−1 > 0) but the
        // observable reads frozen (oscillation_strength == 0).
        let eval = live_with(0.0, 0.0, 0.0, -0.1, 0.3);
        let ds = bifurcation_crosscheck(&eval);
        let o = ds
            .iter()
            .find(|d| d.axis == "oscillation")
            .expect("oscillation disagreement must surface");
        assert_eq!(o.regime, CrosscheckRegime::WeakObservable);
        assert!((o.predicted - 0.3).abs() < 1e-6);
    }

    #[test]
    fn crosscheck_is_silent_when_predictions_agree_with_the_observables() {
        // Both signs match the observables (coexistence predicted and observed;
        // oscillation predicted and observed): no disagreement is fabricated.
        let eval = live_with(0.4, 0.5, 10.0, 0.3, 0.2);
        assert!(
            bifurcation_crosscheck(&eval).is_empty(),
            "agreement must produce no disagreement"
        );
    }

    #[test]
    fn guild_fractions_feed_neither_binning_nor_fitness_nor_the_crosscheck() {
        // Authority boundary: the decomposer and consumer guild fractions are
        // reported distributions, never a behaviour axis, fitness term, or
        // cross-check input. Two configs identical in every binning/fitness/
        // descriptor field but with opposite guild fractions must bin to the same
        // cell, yield the same archive fitness, and surface the same bifurcation
        // cross-check.
        let d = descr(0.5, 0.6, 0.2);
        let mut no_guild = live(0.4, d);
        let mut all_guild = live(0.4, d);
        no_guild.decomposer_fraction = 0.0;
        no_guild.consumer_fraction = 0.0;
        all_guild.decomposer_fraction = 1.0;
        all_guild.consumer_fraction = 1.0;
        // Same predicted coords + observed boundary so the only difference is the
        // decomposer fraction.
        for ev in [&mut no_guild, &mut all_guild] {
            ev.predicted_branching_distance = 0.5; // coexistence predicted
            ev.predicted_oscillation_distance = -0.2; // frozen predicted
            ev.coexistence_duration = 5.0;
        }

        // Binning: same cell.
        assert_eq!(
            cell_of(&no_guild.descriptors),
            cell_of(&all_guild.descriptors)
        );

        // Fitness / archive improvement: identical regardless of decomposer fraction.
        let mut a0 = Archive::new(0.0);
        let mut a1 = Archive::new(0.0);
        let imp0 = a0.insert(&vec![0.5; 3], &no_guild);
        let imp1 = a1.insert(&vec![0.5; 3], &all_guild);
        assert_eq!(imp0, imp1);
        assert_eq!(a0.best_fitness(), a1.best_fitness());
        assert_eq!(a0.qd_score(), a1.qd_score());

        // Cross-check: identical disagreements regardless of decomposer fraction.
        let ds0 = bifurcation_crosscheck(&no_guild);
        let ds1 = bifurcation_crosscheck(&all_guild);
        assert_eq!(ds0.len(), ds1.len());
        for (x, y) in ds0.iter().zip(ds1.iter()) {
            assert_eq!(x.axis, y.axis);
            assert_eq!(x.regime, y.regime);
            assert_eq!(x.predicted, y.predicted);
            assert_eq!(x.observed, y.observed);
        }
    }

    // --- Coexistence fraction + robustness-aware projection (#401) -----------

    /// A zeroed [`FitnessBreakdown`] with the two coexistence observables set, for
    /// synthetic per-seed results (no sim).
    fn breakdown(
        clustering: f32,
        coexistence_duration: f32,
    ) -> explorers_genesis::FitnessBreakdown {
        explorers_genesis::FitnessBreakdown {
            fitness: 0.0,
            failure: None,
            oscillation_strength: 0.0,
            clustering_strength: clustering,
            coexistence_duration,
            turnover_score: 0.0,
            trophic_balance_score: 0.0,
            ticks_survived: 0,
            carcass_locked_fraction: 0.0,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        }
    }

    fn run_result(
        fitness: f32,
        failure: Option<FailureMode>,
        clustering: f32,
        coexistence_duration: f32,
    ) -> RunResult {
        RunResult {
            fitness,
            failure,
            termination_tick: 0,
            breakdown: breakdown(clustering, coexistence_duration),
            early_stop: None,
        }
    }

    #[test]
    fn config_eval_computes_coexistence_fraction() {
        // The #401 straddler's independent 8-seed re-evaluation: 4 monoculture
        // failures, 1 lockup failure, 3 alive-and-coexisting. Only the 3 live
        // coexisting seeds count → 3/8. (The #359 small-N disjunction means a live
        // seed coexisting only via coexistence_duration still counts.)
        let run_results = vec![
            run_result(0.0, Some(FailureMode::Monoculture), 0.0, 0.0),
            run_result(0.0, Some(FailureMode::Monoculture), 0.0, 0.0),
            run_result(0.0, Some(FailureMode::Monoculture), 0.0, 0.0),
            run_result(0.0, Some(FailureMode::Monoculture), 0.0, 0.0),
            run_result(0.0, Some(FailureMode::NutrientLockup), 0.0, 0.0),
            run_result(0.6, None, 0.4, 8.0),
            run_result(0.6, None, 0.0, 12.0), // coexists only via duration (small-N)
            run_result(0.6, None, 0.3, 5.0),
        ];
        let result = EnsembleResult {
            median_fitness: 0.0,
            run_results,
            unfinished: 0,
        };
        let eval = config_eval_from_ensemble(&result);
        assert_eq!(eval.coexistence_fraction, 3.0 / 8.0);
    }

    #[test]
    fn guild_aware_floors_count_coexisting_seeds_that_hold_the_guild() {
        // #538 (#494 option 3): a guild-aware floor counts a seed only when it
        // coexists *and* holds the guild. All four fractions come from the same
        // seeds, so no guild-aware fraction can exceed the plain one. A gated
        // seed that holds a guild (#527 carries it) does not coexist, so it
        // counts under no floor.
        let with_guilds = |mut r: RunResult, decomposer: bool, consumer: bool| {
            r.breakdown.has_decomposer_guild = decomposer;
            r.breakdown.has_consumer_guild = consumer;
            r
        };
        let run_results = vec![
            with_guilds(run_result(0.6, None, 0.4, 8.0), false, false),
            with_guilds(run_result(0.6, None, 0.4, 8.0), true, false),
            with_guilds(run_result(0.6, None, 0.4, 8.0), false, true),
            with_guilds(run_result(0.6, None, 0.4, 8.0), true, true),
            with_guilds(
                run_result(0.0, Some(FailureMode::Monoculture), 0.0, 0.0),
                true,
                true,
            ),
        ];
        let fractions = CoexistenceFractions::of_seeds(&run_results);
        assert_eq!(fractions.under(CoexistenceFloor::Plain), 4.0 / 5.0);
        assert_eq!(fractions.under(CoexistenceFloor::Decomposer), 2.0 / 5.0);
        assert_eq!(fractions.under(CoexistenceFloor::Consumer), 2.0 / 5.0);
        assert_eq!(fractions.under(CoexistenceFloor::Either), 3.0 / 5.0);
        // The plain fraction is exactly the in-run `coexistence_fraction`.
        let result = EnsembleResult {
            median_fitness: 0.0,
            run_results,
            unfinished: 0,
        };
        assert_eq!(
            fractions.under(CoexistenceFloor::Plain),
            config_eval_from_ensemble(&result).coexistence_fraction
        );
        assert_eq!(CoexistenceFractions::of_seeds(&[]), Default::default());
    }

    #[test]
    fn early_stop_crosscheck_surfaces_exactly_the_stopped_dead_but_alive_at_horizon_seeds() {
        // The carry-to-T cross-check (#506): of an ensemble where the energy-
        // death gate stopped three seeds, one was carried and recovered by T
        // (alive on the full series) — the one disagreement; one was carried
        // and stayed dead — agreement; one was not carried at all — nothing
        // to compare. Extinct and live seeds are never candidates.
        use explorers_genesis::{EarlyStop, HorizonVerdict};
        let stop = |horizon: Option<HorizonVerdict>| {
            let mut r = run_result(0.0, Some(FailureMode::EnergyDeath), 0.0, 0.0);
            r.termination_tick = 350;
            r.early_stop = Some(EarlyStop {
                failure: FailureMode::EnergyDeath,
                tick: 350,
                horizon,
            });
            r
        };
        let run_results = vec![
            run_result(0.6, None, 0.4, 8.0),
            run_result(0.0, Some(FailureMode::Extinction), 0.0, 0.0),
            stop(Some(HorizonVerdict {
                failure: None,
                fitness: 0.42,
                termination_tick: 2000,
            })),
            stop(Some(HorizonVerdict {
                failure: Some(FailureMode::EnergyDeath),
                fitness: 0.0,
                termination_tick: 2000,
            })),
            stop(None),
        ];
        let result = EnsembleResult {
            median_fitness: 0.0,
            run_results,
            unfinished: 0,
        };
        let unit = vec![0.5; 3];
        let check = early_stop_crosscheck(&result, &unit);
        assert_eq!(check.carried, 2, "two seeds were carried to the horizon");
        assert_eq!(check.disagreements.len(), 1);
        let d = &check.disagreements[0];
        assert_eq!(d.gate, "energy_death");
        assert_eq!(d.stop_tick, 350);
        assert_eq!(d.seed_index, 2);
        assert_eq!(d.observed_fitness, 0.42);
        assert_eq!(d.unit, unit);
    }

    #[test]
    fn config_eval_computes_consumer_fraction_alongside_decomposer_fraction() {
        // Both heterotroph guild fractions (#490) are the share of the ensemble
        // whose seed read the guild, computed the same way — including gated
        // seeds in the denominator.
        let mut run_results: Vec<RunResult> =
            (0..4).map(|_| run_result(0.5, None, 0.5, 5.0)).collect();
        run_results[0].breakdown.has_consumer_guild = true;
        run_results[1].breakdown.has_consumer_guild = true;
        run_results[1].breakdown.has_decomposer_guild = true;
        run_results[3].breakdown.has_consumer_guild = true;
        let result = EnsembleResult {
            median_fitness: 0.5,
            run_results,
            unfinished: 0,
        };
        let eval = config_eval_from_ensemble(&result);
        assert_eq!(eval.consumer_fraction, 3.0 / 4.0);
        assert_eq!(eval.decomposer_fraction, 1.0 / 4.0);
    }

    #[test]
    fn best_recipe_prefers_the_robust_cell_over_a_higher_fitness_straddler() {
        // The #401 fix: a higher-fitness cell that coexists on a minority of seeds
        // (a bifurcation straddler) must NOT be projected over a lower-fitness cell
        // that is robustly sensible (most of its ensemble coexists).
        let ranges = default_ranges();
        let mut straddler = atlas_cell(0.1);
        straddler.cell = cell_of(&descr(0.1, 0.1, 0.1)).into();
        straddler.fitness = 0.67;
        straddler.coexistence_fraction = 0.375;
        straddler.unit = vec![0.2; ranges.len()];
        let mut robust = atlas_cell(0.1);
        robust.cell = cell_of(&descr(0.2, 0.2, 0.1)).into();
        robust.fitness = 0.40;
        robust.coexistence_fraction = 1.0;
        robust.unit = vec![0.8; ranges.len()];

        let atlas = atlas_with(vec![straddler, robust.clone()], 0);
        let recipe = atlas.best_recipe(&ranges, 100).unwrap();
        let expected = recipe_from_unit(&robust.unit, &ranges, 100);
        assert_eq!(recipe, expected);
    }

    #[test]
    fn best_recipe_falls_back_to_argmax_when_no_cell_clears_the_floor() {
        // Projection totality: if no live cell clears the floor the projection still
        // yields a world (the highest-fitness straddler), so the app is never left
        // without a recipe on a live atlas.
        let ranges = default_ranges();
        let mut a = atlas_cell(0.1);
        a.cell = cell_of(&descr(0.1, 0.1, 0.1)).into();
        a.fitness = 0.30;
        a.coexistence_fraction = 0.2;
        a.unit = vec![0.3; ranges.len()];
        let mut b = atlas_cell(0.1);
        b.cell = cell_of(&descr(0.2, 0.2, 0.1)).into();
        b.fitness = 0.50;
        b.coexistence_fraction = 0.4;
        b.unit = vec![0.7; ranges.len()];

        let atlas = atlas_with(vec![a, b.clone()], 0);
        let recipe = atlas.best_recipe(&ranges, 100).unwrap();
        // Fallback is plain argmax-fitness — the higher-fitness sub-floor cell.
        let expected = recipe_from_unit(&b.unit, &ranges, 100);
        assert_eq!(recipe, expected);
    }

    /// A stub refinement read at n = 32 whose seeds coexist at `plain` and hold
    /// no guild — all the plain-floor tests need.
    fn plain_eval(plain: f32, median_fitness: f32) -> RefinedEval {
        RefinedEval {
            fractions: CoexistenceFractions {
                plain,
                ..Default::default()
            },
            median_fitness,
            sample_count: 32,
            unfinished: 0,
            decomposer_fraction: 0.0,
            consumer_fraction: 0.0,
        }
    }

    #[test]
    fn refinement_drops_a_straddler_that_passed_the_in_run_floor() {
        // #404: a higher-fitness cell that clears the floor on its lucky in-run n=5
        // draw (recorded 0.6) but coexists on a minority under the larger
        // independent ensemble (refined 0.2) must be rejected, and the projection
        // must fall to the robustly-coexisting lower-fitness cell (refined 0.9).
        let ranges = default_ranges();
        let mut straddler = atlas_cell(0.1);
        straddler.cell = cell_of(&descr(0.1, 0.1, 0.1)).into();
        straddler.fitness = 0.67;
        straddler.coexistence_fraction = 0.6; // cleared the in-run floor
        straddler.unit = vec![0.2; ranges.len()];
        let mut robust = atlas_cell(0.1);
        robust.cell = cell_of(&descr(0.2, 0.2, 0.1)).into();
        robust.fitness = 0.40;
        robust.coexistence_fraction = 1.0;
        robust.unit = vec![0.8; ranges.len()];

        let atlas = atlas_with(vec![straddler.clone(), robust.clone()], 0);
        // Stub evaluator: rank 0 is the straddler (top fitness), rank 1 the robust
        // cell. The larger ensemble exposes the straddler's true minority coexistence.
        let projection = project_with_refinement(
            &atlas,
            &ranges,
            100,
            REFINE_TOP_K,
            CoexistenceFloor::Plain,
            |rank, _| {
                if rank == 0 {
                    plain_eval(0.2, 0.0)
                } else {
                    plain_eval(0.9, 0.40)
                }
            },
        );

        assert!(
            projection.cleared_floor,
            "the robust cell clears the refined floor"
        );
        assert_eq!(
            projection.recipe.unwrap(),
            recipe_from_unit(&robust.unit, &ranges, 100),
            "the robust cell is projected, not the refined-out straddler"
        );
        assert_eq!(projection.refined.len(), 2);
        assert_eq!(projection.refined[0].cell, straddler.cell);
        assert!(!projection.refined[0].clears_floor);
        assert!(projection.refined[1].clears_floor);

        // Authority boundary: refinement is selection only — the straddler stays a
        // recorded cell with its real fitness and in-run fraction untouched.
        let recorded = atlas
            .cells
            .iter()
            .find(|c| c.cell == straddler.cell)
            .unwrap();
        assert_eq!(recorded.fitness, 0.67);
        assert_eq!(recorded.coexistence_fraction, 0.6);
    }

    #[test]
    fn a_guild_aware_floor_rejects_a_cell_that_coexists_without_the_guild() {
        // #538 (#494 option 3): the top cell coexists robustly but on seeds
        // without a decomposer guild; the next holds one. The plain floor
        // projects the top cell, the decomposer floor the second. Changing the
        // floor changes only `clears_floor` and the pick: the same ranks are
        // refined, the same units evaluated, and every fraction is reported
        // under every floor.
        let ranges = default_ranges();
        let cells: Vec<AtlasCell> = (0..2)
            .map(|i| {
                let mut c = atlas_cell(0.1);
                c.cell = [i, 0, 0];
                c.fitness = 0.9 - 0.1 * i as f32;
                c.unit = vec![0.2 + 0.5 * i as f64; ranges.len()];
                c
            })
            .collect();
        let atlas = atlas_with(cells.clone(), 0);
        let evaluate = |calls: &mut Vec<(usize, Vec<f64>)>, rank: usize, unit: &[f64]| {
            calls.push((rank, unit.to_vec()));
            let decomposer = if rank == 0 { 0.1 } else { 0.8 };
            RefinedEval {
                fractions: CoexistenceFractions {
                    plain: 0.9,
                    decomposer,
                    consumer: 0.0,
                    either: decomposer,
                },
                median_fitness: 0.5,
                sample_count: 32,
                unfinished: 0,
                decomposer_fraction: decomposer,
                consumer_fraction: 0.0,
            }
        };

        let mut plain_calls = Vec::new();
        let plain = project_with_refinement(
            &atlas,
            &ranges,
            100,
            REFINE_TOP_K,
            CoexistenceFloor::Plain,
            |rank, unit| evaluate(&mut plain_calls, rank, unit),
        );
        let mut guild_calls = Vec::new();
        let guild = project_with_refinement(
            &atlas,
            &ranges,
            100,
            REFINE_TOP_K,
            CoexistenceFloor::Decomposer,
            |rank, unit| evaluate(&mut guild_calls, rank, unit),
        );

        assert_eq!(
            plain.recipe.unwrap(),
            recipe_from_unit(&cells[0].unit, &ranges, 100)
        );
        assert_eq!(
            guild.recipe.unwrap(),
            recipe_from_unit(&cells[1].unit, &ranges, 100)
        );
        assert!(plain.cleared_floor && guild.cleared_floor);
        assert_eq!(plain.floor, CoexistenceFloor::Plain);
        assert_eq!(guild.floor, CoexistenceFloor::Decomposer);
        assert_eq!(plain_calls, guild_calls, "same ranks, same units");
        assert_eq!(
            plain
                .refined
                .iter()
                .map(|r| r.clears_floor)
                .collect::<Vec<_>>(),
            vec![true, true]
        );
        assert_eq!(
            guild
                .refined
                .iter()
                .map(|r| r.clears_floor)
                .collect::<Vec<_>>(),
            vec![false, true]
        );
        // The audit carries every floor's fraction and the guild reads, whichever
        // floor picked.
        assert_eq!(plain.refined[0].refined_fractions.decomposer, 0.1);
        assert_eq!(plain.refined[1].refined_decomposer_fraction, 0.8);
        assert_eq!(guild.refined[0].refined_fractions.plain, 0.9);
    }

    #[test]
    fn refinement_falls_back_to_argmax_when_no_refined_cell_clears() {
        // When the larger ensemble drops every top-K cell below the floor, the
        // projection still yields a world (highest recorded fitness) but flags the
        // fallback so the search can warn — mirrors best_recipe's totality.
        let ranges = default_ranges();
        let mut a = atlas_cell(0.1);
        a.cell = cell_of(&descr(0.1, 0.1, 0.1)).into();
        a.fitness = 0.30;
        a.unit = vec![0.3; ranges.len()];
        let mut b = atlas_cell(0.1);
        b.cell = cell_of(&descr(0.2, 0.2, 0.1)).into();
        b.fitness = 0.50;
        b.unit = vec![0.7; ranges.len()];

        let atlas = atlas_with(vec![a.clone(), b.clone()], 0);
        let projection = project_with_refinement(
            &atlas,
            &ranges,
            100,
            REFINE_TOP_K,
            CoexistenceFloor::Plain,
            |_, _| plain_eval(0.1, 0.0),
        );

        assert!(!projection.cleared_floor);
        assert_eq!(
            projection.recipe.unwrap(),
            recipe_from_unit(&b.unit, &ranges, 100),
            "fallback is plain argmax-fitness"
        );
    }

    #[test]
    fn refinement_only_touches_the_top_k_and_discloses_the_rest() {
        // Bounded cost: only the top-K live cells are refined; cells ranked below
        // the cut are reported as unrefined, never silently dropped.
        let ranges = default_ranges();
        let cells: Vec<AtlasCell> = (0..5)
            .map(|i| {
                let mut c = atlas_cell(0.1);
                c.cell = [i, 0, 0];
                c.fitness = 0.9 - 0.1 * i as f32; // strictly descending, distinct
                c.unit = vec![0.1 * (i + 1) as f64; ranges.len()];
                c
            })
            .collect();
        let atlas = atlas_with(cells, 0);

        let mut refined_ranks = Vec::new();
        let projection = project_with_refinement(
            &atlas,
            &ranges,
            100,
            2,
            CoexistenceFloor::Plain,
            |rank, _| {
                refined_ranks.push(rank);
                plain_eval(0.9, 0.5)
            },
        );

        assert_eq!(projection.refined.len(), 2, "only top-2 refined");
        assert_eq!(
            refined_ranks,
            vec![0, 1],
            "evaluator called for the top-2 ranks only"
        );
        assert_eq!(
            projection.unrefined_live_cells, 3,
            "the other 3 live cells disclosed"
        );
    }

    #[test]
    fn refinement_on_empty_atlas_yields_no_recipe() {
        let ranges = default_ranges();
        let atlas = atlas_with(vec![], 0);
        let projection = project_with_refinement(
            &atlas,
            &ranges,
            100,
            REFINE_TOP_K,
            CoexistenceFloor::Plain,
            |_, _| plain_eval(1.0, 1.0),
        );
        assert!(projection.recipe.is_none());
        assert!(!projection.cleared_floor);
        assert!(projection.refined.is_empty());
    }

    fn no_simulation_budget() -> RolloutBudget {
        RolloutBudget {
            simulation: std::time::Duration::ZERO,
            ..RolloutBudget::UNBOUNDED
        }
    }

    #[test]
    fn slow_an_unfinished_rollout_enters_neither_the_archive_nor_the_frontier() {
        // #562: a rollout past its budget is no verdict — not a cell, not a
        // death — only a count.
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let config = QdConfig {
            ensemble_size: 2,
            max_ticks: 20,
            batch: 4,
            generations: 1,
            prefilter_crosscheck_fraction: 0.0,
            rollout_budget: no_simulation_budget(),
            ..QdConfig::default()
        };
        let atlas = run_qd(&config, 42, &mut ChaCha8Rng::seed_from_u64(42));

        let total = config.batch * (config.generations + 1);
        let rolled_out = total - atlas.rollouts_skipped;
        assert!(rolled_out > 0, "some config must clear the prefilter");
        assert_eq!(atlas.coverage, 0);
        assert_eq!(atlas.dead_frontier, atlas.dead_frontier_apriori);
        assert_eq!(
            atlas.rollouts_unfinished,
            rolled_out * config.ensemble_size as usize
        );
    }

    #[test]
    fn slow_an_unhit_budget_leaves_the_atlas_byte_identical() {
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let budgeted = QdConfig {
            ensemble_size: 2,
            max_ticks: 20,
            batch: 4,
            generations: 1,
            ..QdConfig::default()
        };
        let unbudgeted = QdConfig {
            rollout_budget: RolloutBudget::UNBOUNDED,
            ..budgeted.clone()
        };
        let a = run_qd(&budgeted, 42, &mut ChaCha8Rng::seed_from_u64(42));
        let b = run_qd(&unbudgeted, 42, &mut ChaCha8Rng::seed_from_u64(42));
        let (a, b) = (
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap(),
        );
        assert_eq!(a, b);
        // No budget fired, so the atlas carries no trace of one: the same
        // bytes an atlas from before #562 has.
        assert!(!a.contains("rollouts_unfinished"));
    }

    #[test]
    fn slow_refinement_reports_unfinished_seeds_per_cell() {
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let config = crate::search::SearchConfig {
            ensemble_size: 2,
            max_ticks: 15,
            batch: 6,
            generations: 1,
            ..Default::default()
        };
        let atlas = crate::search::run_search(&config, 7, &mut ChaCha8Rng::seed_from_u64(42));
        assert!(!atlas.cells.is_empty(), "need a live cell to refine");

        let rconfig = RefinementConfig {
            top_k: 2,
            ensemble_size: 3,
            max_ticks: config.max_ticks,
            rollout_budget: no_simulation_budget(),
            ..RefinementConfig::default()
        };
        let projection = refined_best_recipe(&atlas, &config.ranges, &rconfig, 7);
        assert!(!projection.refined.is_empty());
        for cell in &projection.refined {
            assert_eq!(cell.refined_unfinished, 3);
            // Excluded from the denominator: nothing finished, nothing counted.
            assert_eq!(cell.refined_sample_count, 0);
            assert!(!cell.clears_floor);
        }
    }

    #[test]
    fn slow_refined_best_recipe_is_deterministic_in_seed() {
        // #404 determinism: a fixed (atlas, config, base_seed) yields a
        // bit-reproducible refinement (same refined fractions) and projection.
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let config = crate::search::SearchConfig {
            ensemble_size: 2,
            max_ticks: 15,
            batch: 6,
            generations: 1,
            ..Default::default()
        };
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let atlas = crate::search::run_search(&config, 7, &mut rng);

        let rconfig = RefinementConfig {
            top_k: 4,
            ensemble_size: 4,
            max_ticks: config.max_ticks,
            floor: CoexistenceFloor::Plain,
            ..RefinementConfig::default()
        };
        let p1 = refined_best_recipe(&atlas, &config.ranges, &rconfig, 7);
        let p2 = refined_best_recipe(&atlas, &config.ranges, &rconfig, 7);

        assert_eq!(p1.recipe, p2.recipe);
        assert_eq!(p1.cleared_floor, p2.cleared_floor);
        assert_eq!(p1.refined.len(), p2.refined.len());
        for (a, b) in p1.refined.iter().zip(p2.refined.iter()) {
            assert_eq!(a.cell, b.cell);
            assert_eq!(a.refined_fractions, b.refined_fractions);
            assert_eq!(a.refined_median_fitness, b.refined_median_fitness);
        }
    }

    #[test]
    fn coexistence_fraction_feeds_neither_binning_nor_fitness() {
        // Authority boundary (mirrors the decomposer-fraction test): the coexistence
        // fraction is a reported per-seed distribution, never a behaviour axis nor a
        // fitness term. Two configs identical in every binning/fitness/descriptor
        // field but with opposite coexistence fractions must bin to the same cell
        // and yield identical archive fitness / qd-score / cross-check.
        let d = descr(0.5, 0.6, 0.2);
        let mut none = live(0.4, d);
        let mut all = live(0.4, d);
        none.coexistence_fraction = 0.0;
        all.coexistence_fraction = 1.0;
        for ev in [&mut none, &mut all] {
            ev.predicted_branching_distance = 0.5;
            ev.predicted_oscillation_distance = -0.2;
            ev.coexistence_duration = 5.0;
        }

        // Binning: same cell.
        assert_eq!(cell_of(&none.descriptors), cell_of(&all.descriptors));

        // Fitness / archive improvement: identical regardless of coexistence fraction.
        let mut a0 = Archive::new(0.0);
        let mut a1 = Archive::new(0.0);
        let imp0 = a0.insert(&vec![0.5; 3], &none);
        let imp1 = a1.insert(&vec![0.5; 3], &all);
        assert_eq!(imp0, imp1);
        assert_eq!(a0.best_fitness(), a1.best_fitness());
        assert_eq!(a0.qd_score(), a1.qd_score());

        // Cross-check: identical disagreements regardless of coexistence fraction.
        let ds0 = bifurcation_crosscheck(&none);
        let ds1 = bifurcation_crosscheck(&all);
        assert_eq!(ds0.len(), ds1.len());
        for (x, y) in ds0.iter().zip(ds1.iter()) {
            assert_eq!(x.axis, y.axis);
            assert_eq!(x.regime, y.regime);
            assert_eq!(x.predicted, y.predicted);
            assert_eq!(x.observed, y.observed);
        }
    }

    #[test]
    fn slow_sweep_populates_predicted_coords_and_regime_tagged_disagreements() {
        // A small multi-seed QD run: every live cell carries finite predicted
        // coords, and any surfaced bifurcation disagreement carries a regime tag.
        // `slow_` — it steps real sims over several seeds.
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let config = QdConfig {
            ensemble_size: 2,
            max_ticks: 60,
            batch: 6,
            generations: 2,
            ..QdConfig::default()
        };

        let mut saw_live = false;
        for &seed in &[5_u64, 17, 29] {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let atlas = run_qd(&config, seed, &mut rng);
            for c in &atlas.cells {
                saw_live = true;
                assert!(
                    c.predicted_oscillation_distance.is_finite(),
                    "live cell oscillation coord must be finite"
                );
                assert!(
                    c.predicted_branching_distance.is_finite(),
                    "live cell branching coord must be finite"
                );
            }
            for d in &atlas.bifurcation_disagreements {
                assert!(
                    d.axis == "branching" || d.axis == "oscillation",
                    "disagreement axis must be one of the two bifurcation axes"
                );
                assert!(matches!(
                    d.regime,
                    CrosscheckRegime::Validated | CrosscheckRegime::WeakObservable
                ));
            }
        }
        assert!(saw_live, "the sweep should populate at least one live cell");
    }
}
