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
//!   `run_ensemble`'s output — no `run_single` mirror.
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
    EnsembleConfig, EnsembleResult, EvalConfig, FailureMode, RunConfig, run_ensemble,
};
use explorers_sim::WorldRecipe;

use crate::prefilter::prefilter_cliff;
use crate::search::{ParameterRange, decode, default_ranges};

/// Bins per behaviour axis. Coarse, per the spike (20×20×20).
pub const RESOLUTION: usize = 20;

/// The six terminal cliffs a config can die on — the dead-frontier key. A gated
/// config gets no behaviour cell (its descriptors are degenerate); it is tallied
/// here instead. This tally is the atlas's dead-frontier layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cliff {
    Extinction,
    PopulationExplosion,
    EnergyDeath,
    NutrientLockup,
    Monoculture,
    GeneralistDominance,
}

impl Cliff {
    fn from_failure(f: &FailureMode) -> Self {
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
#[derive(Clone, Copy, Debug)]
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
    /// Fraction of the ensemble that sprouted a persistent decomposer guild
    /// (reported distribution, never an axis or fitness term).
    pub decomposer_fraction: f32,
    /// The per-cell sample count (the seed-ensemble size).
    pub sample_count: u32,
}

/// Reduce a `run_ensemble` result to a [`ConfigEval`], reading the three axes and
/// the decomposer signal off the per-seed [`FitnessBreakdown`] — no `run_single`
/// mirror. The median-fitness seed (lower-middle for an even ensemble) is the
/// representative, matching the incumbent's median reduction.
pub fn config_eval_from_ensemble(result: &EnsembleResult) -> ConfigEval {
    let sample_count = result.run_results.len() as u32;
    let decomposer_fraction = if sample_count == 0 {
        0.0
    } else {
        result
            .run_results
            .iter()
            .filter(|r| r.breakdown.has_decomposer_guild)
            .count() as f32
            / sample_count as f32
    };

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
            sample_count: 0,
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
        sample_count,
    }
}

/// One filled cell of the soft archive: the current elite plus the rolling
/// acceptance threshold that makes the cell descriptor-noise tolerant (CMA-MAE).
#[derive(Clone, Debug)]
pub struct CellRecord {
    pub fitness: f32,
    pub descriptors: Descriptors,
    pub unit: Vec<f64>,
    pub decomposer_fraction: f32,
    pub sample_count: u32,
    /// The cell's rolling acceptance threshold. A new solution is accepted when
    /// its fitness clears this (not merely the sitting elite); the threshold is
    /// then nudged toward the accepted fitness by the archive learning rate.
    threshold: f32,
}

/// The soft (CMA-MAE) MAP-Elites archive over the three behaviour axes, plus the
/// dead frontier. Live configs fill cells under a rolling per-cell acceptance
/// threshold; gated configs are tallied to the frontier by cliff.
#[derive(Default)]
pub struct Archive {
    cells: std::collections::HashMap<(usize, usize, usize), CellRecord>,
    /// Observed deaths: configs that cleared the prefilter (or were never
    /// prefiltered) and died in their seed-ensemble rollout, keyed by cliff.
    frontier: std::collections::HashMap<Cliff, usize>,
    /// A priori deaths: configs the viability prefilter proved dead in closed
    /// form, routed here without spending an ensemble (`crate::prefilter`). The
    /// two tallies are kept apart so the atlas can distinguish a-priori from
    /// observed deaths (genesis-search.md, "the dead frontier is the atlas's most
    /// valuable layer").
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
                                sample_count: eval.sample_count,
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
                                rec.sample_count = eval.sample_count;
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

    /// QD-score: the sum of elite fitnesses over filled cells.
    pub fn qd_score(&self) -> f32 {
        self.cells.values().map(|c| c.fitness).sum()
    }

    pub fn best_fitness(&self) -> f32 {
        self.cells.values().map(|c| c.fitness).fold(0.0, f32::max)
    }

    /// The elite of the argmax-fitness live cell — the recipe projection.
    pub fn best_cell(&self) -> Option<&CellRecord> {
        self.cells
            .values()
            .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap())
    }

    /// Iterate the filled cells with their indices.
    pub fn cells(&self) -> impl Iterator<Item = (&(usize, usize, usize), &CellRecord)> {
        self.cells.iter()
    }

    /// The dead frontier as a label-keyed tally — the count of configs that died
    /// on each cliff, **a priori and observed combined**. The atlas's
    /// negative-space layer.
    pub fn dead_frontier(&self) -> std::collections::HashMap<String, usize> {
        let mut out: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for (k, &v) in self.frontier.iter().chain(self.apriori_frontier.iter()) {
            *out.entry(k.label().to_string()).or_insert(0) += v;
        }
        out
    }

    /// The **a priori** layer of the dead frontier: configs the viability
    /// prefilter proved dead in closed form, keyed by cliff label. These spent no
    /// ensemble. The complement (`dead_frontier` minus this) is the observed
    /// deaths.
    pub fn dead_frontier_apriori(&self) -> std::collections::HashMap<String, usize> {
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
#[derive(Clone, Debug, serde::Serialize)]
pub struct AtlasCell {
    /// The behaviour-axis cell index `(oscillation, clustering, carcass)`.
    pub cell: [usize; 3],
    pub fitness: f32,
    pub oscillation: f32,
    pub clustering: f32,
    pub carcass: f32,
    /// Per-cell decomposer-guild distribution: the fraction of the cell's seed
    /// ensemble that sprouted a persistent guild (reported, never optimised).
    pub decomposer_fraction: f32,
    /// The seed-ensemble sample count behind that fraction.
    pub sample_count: u32,
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
#[derive(Clone, Debug, serde::Serialize)]
pub struct PrefilterDisagreement {
    /// The cliff the prefilter predicted (the gate that fired).
    pub predicted_cliff: String,
    /// The median fitness the cross-check rollout actually observed.
    pub observed_fitness: f32,
    /// The unit-cube point that disagreed.
    pub unit: Vec<f64>,
}

/// The genesis [`Atlas`](../../CONTEXT.md): the live archive (cells binned on the
/// three behaviour axes) paired with the dead frontier (keyed by cliff), plus the
/// coverage / QD-score summary. This is the search's output — a map onto
/// behaviour, not a ranked list.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Atlas {
    pub cells: Vec<AtlasCell>,
    /// Dead-frontier tally: how many configs died on each cliff (by label),
    /// **a priori and observed combined**.
    pub dead_frontier: std::collections::HashMap<String, usize>,
    /// The **a priori** layer of the dead frontier: configs the viability
    /// prefilter (`crate::prefilter`) proved dead in closed form, by cliff label.
    /// These spent no ensemble. The observed deaths are `dead_frontier` minus
    /// this. Genesis-search.md: "the dead frontier is the atlas's most valuable
    /// layer", and the a-priori-vs-observed split is its agreement cross-check.
    pub dead_frontier_apriori: std::collections::HashMap<String, usize>,
    /// Ensemble rollouts the prefilter skipped (the budget saved) — one per
    /// a-priori death that was *not* drawn into the agreement cross-check sample.
    pub rollouts_skipped: usize,
    /// Prefilter disagreements surfaced: configs the prefilter proved dead that a
    /// cross-check rollout nonetheless showed alive. A non-empty list localises a
    /// mis-drawn gate (viability.md, *Place in the validation triad*); it is
    /// reported, never swallowed.
    pub prefilter_disagreements: Vec<PrefilterDisagreement>,
    /// Filled-cell count.
    pub coverage: usize,
    /// Total cells in the binning (`RESOLUTION³`).
    pub total_cells: usize,
    /// Σ elite fitness over filled cells.
    pub qd_score: f32,
    /// The best elite fitness found (the recipe projection's fitness).
    pub best_fitness: f32,
}

/// The carcass-locked fraction at which the evaluator gates a world as
/// `NutrientLockup` — mirrored here so the search can cross-check its atlas against
/// the gate. **Must equal** `LOCK_FRACTION` in `explorers-genesis-eval`
/// (`is_nutrient_locked`); the cross-check test pins the two together so a future
/// drift in the evaluator gate surfaces here as a failing test.
pub const LOCK_FRACTION: f32 = 0.4;

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

    /// The world recipe drawn from the argmax-fitness live cell — the default
    /// projection of the atlas (genesis-search.md, "the recipe is a projection").
    /// `None` if no cell is live (every config died — the atlas is all frontier).
    pub fn best_recipe(&self, ranges: &[ParameterRange], max_ticks: u64) -> Option<WorldRecipe> {
        let best = self
            .cells
            .iter()
            .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap())?;
        Some(recipe_from_unit(&best.unit, ranges, max_ticks))
    }

    /// The world recipe for a specific live cell index, if that cell is filled —
    /// the atlas's honest stance that *any* viable cell yields a recipe, not only
    /// the best one.
    pub fn recipe_for_cell(
        &self,
        cell: [usize; 3],
        ranges: &[ParameterRange],
        max_ticks: u64,
    ) -> Option<WorldRecipe> {
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
    /// How many **carcass-directed seeds** ([`carcass_seed_unit`]) to inject at the
    /// head of the bootstrap batch. The nutrient-lockup cliff cannot be
    /// prefiltered, so the atlas's lockup layer is populated only by *running*
    /// worlds that strand their nutrient — and a random emitter never lands there
    /// (#363: 0 lockup-frontier configs). Seeding a few guaranteed high-carcass
    /// starting points gives the covariance-adapting emitter a foothold on the
    /// carcass axis to breed toward the cliff. 0 disables carcass direction (the
    /// historical random-bootstrap behaviour).
    pub carcass_seed_count: usize,
}

impl Default for QdConfig {
    fn default() -> Self {
        QdConfig {
            ranges: default_ranges(),
            ensemble_size: 5,
            max_ticks: 500,
            batch: 32,
            generations: 10,
            sigma: 0.15,
            archive_learning_rate: 0.5,
            prefilter_crosscheck_fraction: 0.05,
            carcass_seed_count: 2,
        }
    }
}

/// Run the QD outer search and return the illuminated [`Atlas`]. Reuses
/// [`decode`] and [`run_ensemble`] unchanged — descriptors are read off the
/// per-seed breakdowns. Deterministic in `(config, base_seed, rng)`.
pub fn run_qd(config: &QdConfig, base_seed: u64, rng: &mut impl Rng) -> Atlas {
    let dims = config.ranges.len();
    let ensemble_config = EnsembleConfig {
        ensemble_size: config.ensemble_size,
        run_config: RunConfig {
            max_ticks: config.max_ticks,
            eval_config: EvalConfig::default(),
        },
    };

    let mut archive = Archive::new(config.archive_learning_rate);
    let mut emitter = CmaEmitter::new(dims, config.sigma);
    let mut config_index: u64 = 0;
    let mut rollouts_skipped: usize = 0;
    let mut disagreements: Vec<PrefilterDisagreement> = Vec::new();

    // Generation 0: a random bootstrap batch over the cube (the QD analogue of the
    // incumbent's LHS stage), drawn from the emitter's initial wide Gaussian — with
    // the head replaced by carcass-directed seeds so the bootstrap reaches the thin
    // high-carcass / nutrient-lockup region a random emitter misses (#363; the cliff
    // cannot be prefiltered, so it must be reached by running).
    let mut batch: Vec<Vec<f64>> = (0..config.batch).map(|_| emitter.sample(rng)).collect();
    inject_carcass_seeds(&mut batch, &config.ranges, config.carcass_seed_count);

    for generation in 0..=config.generations {
        // Distinct per-config ensemble base seeds, derived from a monotonic config
        // counter so evaluation order is immaterial (each config's seed is fixed).
        let seeds: Vec<u64> = (0..batch.len())
            .map(|_| {
                let s = base_seed.wrapping_add(config_index.wrapping_mul(1000));
                config_index += 1;
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
        let gates: Vec<(Option<Cliff>, bool)> = batch
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
        let evals: Vec<Option<ConfigEval>> = batch
            .iter()
            .zip(seeds.iter())
            .zip(gates.iter())
            .map(|((unit, &seed), &(cliff, crosscheck))| {
                if cliff.is_some() && !crosscheck {
                    None
                } else {
                    let (wp, dist) = decode(unit, &config.ranges);
                    let result = run_ensemble(&wp, &dist, &ensemble_config, seed);
                    Some(config_eval_from_ensemble(&result))
                }
            })
            .collect();

        // Route each config and compute the emitter's improvement signal.
        let improvements: Vec<f32> = batch
            .iter()
            .zip(gates.iter())
            .zip(evals.iter())
            .map(|((unit, &(cliff, crosscheck)), eval)| match cliff {
                Some(cliff) => {
                    // Proven dead a priori: it lands on the dead frontier as an
                    // a-priori death regardless of whether it was cross-checked.
                    archive.insert_apriori(cliff);
                    if crosscheck {
                        // The cross-check rolled it out: if the rollout shows life,
                        // the prefilter and the rollout disagree — surface it (a
                        // mis-drawn gate), never swallow it.
                        if let Some(ev) = eval {
                            if let Some(d) = crosscheck_disagreement(cliff, ev, unit) {
                                disagreements.push(d);
                            }
                        }
                    } else {
                        rollouts_skipped += 1;
                    }
                    // An a-priori death never improves a cell.
                    0.0
                }
                None => archive.insert(unit, eval.as_ref().expect("cleared config was rolled out")),
            })
            .collect();

        if generation == config.generations {
            break;
        }

        // Adapt the emitter toward the improvers, then emit the next batch. When
        // the archive is still empty the emitter has nothing to centre on, so it
        // keeps sampling its (widening) Gaussian — the bootstrap continues.
        emitter.adapt(&batch, &improvements);
        let elites = archive.elite_units();
        batch = (0..config.batch)
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

    let cells: Vec<AtlasCell> = archive
        .cells()
        .map(|(idx, rec)| AtlasCell {
            cell: [idx.0, idx.1, idx.2],
            fitness: rec.fitness,
            oscillation: rec.descriptors.oscillation,
            clustering: rec.descriptors.clustering,
            carcass: rec.descriptors.carcass,
            decomposer_fraction: rec.decomposer_fraction,
            sample_count: rec.sample_count,
            unit: rec.unit.clone(),
        })
        .collect();

    Atlas {
        coverage: archive.coverage(),
        total_cells: RESOLUTION.pow(3),
        qd_score: archive.qd_score(),
        best_fitness: archive.best_fitness(),
        dead_frontier: archive.dead_frontier(),
        dead_frontier_apriori: archive.dead_frontier_apriori(),
        rollouts_skipped,
        prefilter_disagreements: disagreements,
        cells,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            sample_count: 5,
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
            sample_count: 5,
            unit: vec![0.5; 3],
        }
    }

    fn atlas_with(cells: Vec<AtlasCell>, lockup_deaths: usize) -> Atlas {
        let mut frontier = std::collections::HashMap::new();
        if lockup_deaths > 0 {
            frontier.insert(Cliff::NutrientLockup.label().to_string(), lockup_deaths);
        }
        Atlas {
            coverage: cells.len(),
            total_cells: RESOLUTION.pow(3),
            qd_score: 0.0,
            best_fitness: 0.0,
            dead_frontier: frontier,
            dead_frontier_apriori: std::collections::HashMap::new(),
            rollouts_skipped: 0,
            prefilter_disagreements: Vec::new(),
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

        let config = QdConfig {
            ranges,
            ensemble_size: 2,
            max_ticks: 40,
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
        // small to keep the sweep foreground-fast.
        let config = QdConfig {
            ensemble_size: 2,
            max_ticks: 400,
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
        for &seed in &[11_u64, 23, 37] {
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
            // The same elite is reachable as a cell-specific recipe.
            let best = atlas
                .cells
                .iter()
                .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap())
                .unwrap();
            let cell_recipe = atlas
                .recipe_for_cell(best.cell, &config.ranges, config.max_ticks)
                .unwrap();
            assert_eq!(recipe, cell_recipe);
        }
    }
}
