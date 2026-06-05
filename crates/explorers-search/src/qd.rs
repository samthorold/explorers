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
    frontier: std::collections::HashMap<Cliff, usize>,
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
            archive_learning_rate: archive_learning_rate.clamp(0.0, 1.0),
        }
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
    /// on each cliff. The atlas's negative-space layer.
    pub fn dead_frontier(&self) -> std::collections::HashMap<String, usize> {
        self.frontier
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

/// The genesis [`Atlas`](../../CONTEXT.md): the live archive (cells binned on the
/// three behaviour axes) paired with the dead frontier (keyed by cliff), plus the
/// coverage / QD-score summary. This is the search's output — a map onto
/// behaviour, not a ranked list.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Atlas {
    pub cells: Vec<AtlasCell>,
    /// Dead-frontier tally: how many configs died on each cliff (by label).
    pub dead_frontier: std::collections::HashMap<String, usize>,
    /// Filled-cell count.
    pub coverage: usize,
    /// Total cells in the binning (`RESOLUTION³`).
    pub total_cells: usize,
    /// Σ elite fitness over filled cells.
    pub qd_score: f32,
    /// The best elite fitness found (the recipe projection's fitness).
    pub best_fitness: f32,
}

impl Atlas {
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

    // Generation 0: a random bootstrap batch over the cube (the QD analogue of the
    // incumbent's LHS stage), drawn from the emitter's initial wide Gaussian.
    let mut batch: Vec<Vec<f64>> = (0..config.batch).map(|_| emitter.sample(rng)).collect();

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

        // Evaluate the batch. Each config fans out over its seed ensemble inside
        // `run_ensemble` (rayon), reused unchanged.
        let evals: Vec<ConfigEval> = batch
            .iter()
            .zip(seeds.iter())
            .map(|(unit, &seed)| {
                let (wp, dist) = decode(unit, &config.ranges);
                let result = run_ensemble(&wp, &dist, &ensemble_config, seed);
                config_eval_from_ensemble(&result)
            })
            .collect();

        let improvements: Vec<f32> = batch
            .iter()
            .zip(evals.iter())
            .map(|(unit, eval)| archive.insert(unit, eval))
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
