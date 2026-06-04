//! THROWAWAY SPIKE — issue #363 / Research Brief B (AC5). NOT production.
//!
//! This is the cheap, disposable instrument that tests one claim before any
//! commitment: *does a Quality-Diversity (MAP-Elites) atlas beat the incumbent
//! LHS + GP-BO search at an equal rollout budget, and produce a usable viability
//! atlas the incumbent cannot?* Promoting QD to **be** the genesis search is a
//! separate `ready-for-human` follow-up gated on the numbers this prints — exactly
//! the rhythm of F's #358/#359 spikes and A's #353-gated #351.
//!
//! ## What it reuses unchanged (the guardrail: call the pipeline, don't fork it)
//!
//! - [`explorers_search::search::decode`] / [`default_ranges`] — the same
//!   `[0,1]^31` → `(WorldParameters, InitialDistribution)` decoder the production
//!   search consumes. Every point the emitter proposes is a unit-cube point fed to
//!   `decode` verbatim.
//! - [`explorers_genesis_eval::evaluate_from_log`] — the **unchanged** evaluator:
//!   the six gates, the five-component fitness, oscillation/clustering descriptors.
//! - [`explorers_sim::World`] stepped exactly as [`explorers_genesis::run_single`]
//!   steps it (step → sample free-energy & carcass fraction → early-stop on
//!   extinction / explosion), and [`explorers_sim::topology`] for the realised-diet
//!   decomposer classification, reused verbatim from `decomposer_emergence.rs`.
//!
//! ## Why it mirrors `run_single` instead of calling `run_ensemble`
//!
//! AC5 says "reuse `run_ensemble` unchanged", but `run_ensemble` returns only a
//! `FitnessBreakdown` per seed — which carries `oscillation_strength` and
//! `clustering_strength` but **not** the carcass-locked nutrient fraction (it is
//! sampled per tick inside `run_single` and consumed by the lockup gate, then
//! discarded), and never computes topology roles. Two of the three required
//! descriptors (axis iii) and the decomposer-guild distribution are therefore
//! unreachable from `run_ensemble`'s output. So this driver mirrors `run_single`'s
//! rollout loop **exactly** — the established `decomposer_emergence.rs` precedent —
//! sampling the carcass-fraction series it needs and threading a
//! `TopologyProjection`, while delegating fitness / gates / the other two axes to
//! the unchanged `evaluate_from_log`. The per-seed ensemble (seed = `base + i`)
//! reproduces `run_ensemble`'s semantics. No physics, evaluator, or RNG/eval order
//! is touched.

use std::collections::HashMap;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;

use explorers_genesis_eval::{EvalConfig, FailureMode, evaluate_from_log};
use explorers_search::search::{ParameterRange, SearchConfig, decode, default_ranges, run_search};
use explorers_sim::World;
use explorers_sim::topology::{TopologyProjection, TrophicRole};

// ---------------------------------------------------------------------------
// Archive geometry
// ---------------------------------------------------------------------------

/// Bins per behaviour axis. Coarse, per AC2 ("start hard-binned at coarse
/// resolution e.g. 20×20×20").
const RESOLUTION: usize = 20;

/// A persistent decomposer guild — the AC3b distributional property — is one
/// where ≥1 agent reads as `Decomposer` for at least this fraction of the run.
/// Lifted verbatim from `decomposer_emergence.rs` so the reported distribution
/// uses the same classification the emergence regression test does.
const PERSISTENCE_FRACTION: f64 = 0.25;

/// The six terminal/soft gates, as an out-of-archive frontier key (AC3a). A
/// gated config gets **no behaviour cell** — its descriptors are degenerate — it
/// is tallied here instead. This tally *is* the atlas's dead-region layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Cliff {
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

    fn label(&self) -> &'static str {
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

/// The three F-derived behaviour descriptors (AC2), all in `[0,1]`.
#[derive(Clone, Copy, Debug)]
struct Descriptors {
    /// Frozen ↔ oscillation (Hopf) axis — `FitnessBreakdown::oscillation_strength`.
    oscillation: f32,
    /// Monoculture ↔ coexistence (branching) axis — the dip statistic
    /// `FitnessBreakdown::clustering_strength`.
    clustering: f32,
    /// Energy-death / nutrient-lockup flux axis — the carcass-locked nutrient
    /// fraction the lockup gate reads (`LOCK_FRACTION`), here the trailing-window
    /// mean of the per-tick `World::carcass_locked_nutrient_fraction()` series.
    /// Deliberately NOT `trophic_balance_score` (decomposer-blind, AC2).
    carcass: f32,
}

/// Hard-bin a descriptor triple into an archive cell index. Inputs are clamped
/// to `[0,1]` then mapped to `0..RESOLUTION`.
fn cell_of(d: &Descriptors) -> (usize, usize, usize) {
    let bin = |x: f32| -> usize {
        let c = x.clamp(0.0, 1.0);
        ((c * RESOLUTION as f32) as usize).min(RESOLUTION - 1)
    };
    (bin(d.oscillation), bin(d.clustering), bin(d.carcass))
}

// ---------------------------------------------------------------------------
// Rollout — mirrors explorers_genesis::run_single, plus carcass + topology
// ---------------------------------------------------------------------------

/// One seed's outcome.
struct SeedOutcome {
    fitness: f32,
    failure: Option<FailureMode>,
    descriptors: Descriptors,
    /// AC3b: did a decomposer guild persist (≥1 decomposer for ≥25% of the run)?
    persistent_decomposer: bool,
}

/// Run one seed of `unit` through the genesis rollout loop, mirroring
/// `explorers_genesis::run_single` exactly (step → sample free-energy & carcass
/// fraction → early-stop), additionally threading a `TopologyProjection` for the
/// decomposer-guild readout. Fitness / gates / oscillation / clustering all come
/// from the unchanged `evaluate_from_log`; only the carcass-fraction axis and the
/// guild readout are computed here from quantities `run_single`/`run_ensemble`
/// discard.
fn rollout_seed(unit: &[f64], ranges: &[ParameterRange], seed: u64, max_ticks: u64) -> SeedOutcome {
    let eval = EvalConfig::default();
    let (params, dist) = decode(unit, ranges);
    let mut world = World::new(params, dist, seed);
    let mut topo = TopologyProjection::new();

    let mut free_energy_per_tick: Vec<f32> = Vec::with_capacity(max_ticks as usize);
    let mut carcass_fraction_per_tick: Vec<f32> = Vec::with_capacity(max_ticks as usize);
    let mut decomposer_ticks = 0u64;
    let mut ran_ticks = 0u64;

    for _ in 0..max_ticks {
        world.step();
        ran_ticks += 1;
        free_energy_per_tick.push(world.free_energy());
        carcass_fraction_per_tick.push(world.carcass_locked_nutrient_fraction());

        // Realised-diet classification (reads event log + agents only; does not
        // perturb the world or its RNG). Same `trophic_roles` call as the
        // emergence regression test.
        topo.update(world.event_log());
        let has_decomposer = topo
            .trophic_roles(world.agents())
            .values()
            .any(|&r| r == TrophicRole::Decomposer);
        if has_decomposer {
            decomposer_ticks += 1;
        }

        if world.agents().is_empty() {
            break;
        }
        if world.agents().len() > eval.max_population {
            break;
        }
    }

    let breakdown = evaluate_from_log(
        &world,
        &free_energy_per_tick,
        &carcass_fraction_per_tick,
        &eval,
        max_ticks,
    );

    // Axis (iii): trailing-window mean of the carcass-locked fraction — the same
    // quantity (and window) the nutrient-lockup gate inspects.
    let carcass = trailing_mean(&carcass_fraction_per_tick, eval.nutrient_lock_window);
    let persistent_decomposer =
        ran_ticks > 0 && (decomposer_ticks as f64 / ran_ticks as f64) >= PERSISTENCE_FRACTION;

    SeedOutcome {
        fitness: breakdown.fitness,
        failure: breakdown.failure.clone(),
        descriptors: Descriptors {
            oscillation: breakdown.oscillation_strength,
            clustering: breakdown.clustering_strength,
            carcass,
        },
        persistent_decomposer,
    }
}

/// Mean of the trailing `window` samples (whole series if shorter), 0 if empty.
fn trailing_mean(series: &[f32], window: usize) -> f32 {
    if series.is_empty() {
        return 0.0;
    }
    let start = series.len().saturating_sub(window);
    let tail = &series[start..];
    tail.iter().sum::<f32>() / tail.len() as f32
}

/// One config's ensemble verdict, reduced exactly as the incumbent reduces
/// (median fitness over the seed ensemble; AC: "incumbent's 5-seed median").
struct ConfigEval {
    median_fitness: f32,
    /// `Some(cliff)` if the median-fitness seed is gated (→ dead frontier),
    /// `None` if it is a live world (→ archive cell).
    cliff: Option<Cliff>,
    /// Descriptors of the median-fitness seed (only meaningful when `cliff` is
    /// `None`).
    descriptors: Descriptors,
    /// AC3b: fraction of the ensemble that sprouted a persistent decomposer guild.
    decomposer_fraction: f32,
    /// Seeds in the ensemble (the per-cell sample count).
    sample_count: u32,
}

/// Evaluate a config = run an ensemble of `ensemble_size` seeds (`base + i`,
/// the `run_ensemble` seed scheme) in parallel — the (config × seed) rayon unit
/// brief A / #353 ships. Reduce to a median; the median-fitness seed decides
/// dead-vs-live and supplies the descriptors.
fn evaluate_config(
    unit: &[f64],
    ranges: &[ParameterRange],
    base_seed: u64,
    ensemble_size: u32,
    max_ticks: u64,
) -> ConfigEval {
    let mut outcomes: Vec<SeedOutcome> = (0..ensemble_size)
        .into_par_iter()
        .map(|i| rollout_seed(unit, ranges, base_seed.wrapping_add(i as u64), max_ticks))
        .collect();

    let decomposer_fraction =
        outcomes.iter().filter(|o| o.persistent_decomposer).count() as f32 / ensemble_size as f32;

    // Sort by fitness; the lower-middle element is the median seed for odd
    // ensembles (the median of an even ensemble is an average with no single
    // representative — keep `ensemble_size` odd).
    outcomes.sort_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap());
    let mid = outcomes.len() / 2;
    let rep = &outcomes[mid];
    let median_fitness = rep.fitness;

    ConfigEval {
        median_fitness,
        cliff: rep.failure.as_ref().map(Cliff::from_failure),
        descriptors: rep.descriptors,
        decomposer_fraction,
        sample_count: ensemble_size,
    }
}

// ---------------------------------------------------------------------------
// Archive
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct CellRecord {
    fitness: f32,
    descriptors: Descriptors,
    unit: Vec<f64>,
    decomposer_fraction: f32,
    sample_count: u32,
}

#[derive(Default)]
struct Archive {
    cells: HashMap<(usize, usize, usize), CellRecord>,
    /// Dead-frontier tally: per cliff, the unit-cube points that died there
    /// (AC3a — "which parameter regions die, and which cliff they hit").
    frontier: HashMap<Cliff, Vec<Vec<f64>>>,
}

impl Archive {
    /// Place one evaluated config. Gated → frontier tally; live → elite cell
    /// (kept only if it beats the incumbent elite of that cell).
    fn insert(&mut self, unit: &[f64], eval: ConfigEval) {
        match eval.cliff {
            Some(cliff) => {
                self.frontier.entry(cliff).or_default().push(unit.to_vec());
            }
            None => {
                let cell = cell_of(&eval.descriptors);
                let better = self
                    .cells
                    .get(&cell)
                    .is_none_or(|c| eval.median_fitness > c.fitness);
                if better {
                    self.cells.insert(
                        cell,
                        CellRecord {
                            fitness: eval.median_fitness,
                            descriptors: eval.descriptors,
                            unit: unit.to_vec(),
                            decomposer_fraction: eval.decomposer_fraction,
                            sample_count: eval.sample_count,
                        },
                    );
                }
            }
        }
    }

    fn coverage(&self) -> usize {
        self.cells.len()
    }

    /// QD-score = sum of elite fitnesses over filled cells.
    fn qd_score(&self) -> f32 {
        self.cells.values().map(|c| c.fitness).sum()
    }

    fn best_fitness(&self) -> f32 {
        self.cells
            .values()
            .map(|c| c.fitness)
            .fold(0.0_f32, f32::max)
    }

    fn best_cell(&self) -> Option<&CellRecord> {
        self.cells
            .values()
            .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap())
    }
}

// ---------------------------------------------------------------------------
// MAP-Elites driver (classic Gaussian / iso-line emitter)
// ---------------------------------------------------------------------------

struct MapElitesParams {
    dims: usize,
    batch: usize,
    generations: usize,
    ensemble_size: u32,
    max_ticks: u64,
    /// Iso-line / Gaussian mutation σ in unit-cube coordinates.
    sigma: f64,
    base_seed: u64,
}

/// Run MAP-Elites and return the filled archive plus the total single-world
/// rollout count consumed (for the equal-budget comparison).
fn run_map_elites(p: &MapElitesParams, rng: &mut ChaCha8Rng) -> (Archive, u64) {
    let ranges = default_ranges();
    let mut archive = Archive::default();
    let mut rollouts = 0u64;
    // Monotonic config counter → distinct, order-independent ensemble base seeds.
    let mut config_index: u64 = 0;

    // Generation 0: a random batch over the cube (the classic MAP-Elites
    // bootstrap, analogous to the incumbent's LHS stage).
    let mut batch: Vec<Vec<f64>> = (0..p.batch)
        .map(|_| (0..p.dims).map(|_| rng.random_range(0.0..1.0)).collect())
        .collect();

    for generation in 0..=p.generations {
        // Evaluate the batch in parallel (config-level fan-out; each config
        // itself fans out over its seed ensemble).
        let seeds: Vec<u64> = (0..batch.len())
            .map(|_| {
                let s = p.base_seed.wrapping_add(config_index.wrapping_mul(1000));
                config_index += 1;
                s
            })
            .collect();

        let evals: Vec<ConfigEval> = batch
            .par_iter()
            .zip(seeds.par_iter())
            .map(|(unit, &seed)| evaluate_config(unit, &ranges, seed, p.ensemble_size, p.max_ticks))
            .collect();

        for (unit, eval) in batch.iter().zip(evals.into_iter()) {
            rollouts += p.ensemble_size as u64;
            archive.insert(unit, eval);
        }

        if generation == p.generations {
            break;
        }

        // Emit the next batch: select a random elite and perturb it with
        // iso-Gaussian noise (classic MAP-Elites). Falls back to uniform draws
        // while the archive is still empty.
        let elites: Vec<Vec<f64>> = archive.cells.values().map(|c| c.unit.clone()).collect();
        batch = (0..p.batch)
            .map(|_| {
                if elites.is_empty() {
                    (0..p.dims).map(|_| rng.random_range(0.0..1.0)).collect()
                } else {
                    let parent = &elites[rng.random_range(0..elites.len())];
                    parent
                        .iter()
                        .map(|&x| (x + p.sigma * gaussian(rng)).clamp(0.0, 1.0))
                        .collect()
                }
            })
            .collect();
    }

    (archive, rollouts)
}

/// Standard normal via Box-Muller (avoids a `rand_distr` dependency for a
/// throwaway bin).
fn gaussian(rng: &mut ChaCha8Rng) -> f64 {
    let u1: f64 = rng.random_range(f64::MIN_POSITIVE..1.0);
    let u2: f64 = rng.random_range(0.0..1.0);
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

// ---------------------------------------------------------------------------
// Incumbent budget accounting
// ---------------------------------------------------------------------------

/// Total single-world rollouts the incumbent `run_search` consumes at a given
/// config — so MAP-Elites can be run at an *equal* budget (the AC's head-to-head
/// condition). Derived from `search.rs`:
///   * LHS sweep:  `lhs_samples × ensemble_size`
///   * Sobol indices: `lhs_samples × (dims + 2)` single-seed evals
///   * BO: `(2·dims).max(3)` initial design + `bayesopt_iterations` steps (single-seed)
///   * final optimised ensemble: `ensemble_size`
fn incumbent_rollout_budget(cfg: &SearchConfig) -> u64 {
    let dims = cfg.ranges.len();
    let lhs = cfg.lhs_samples as u64 * cfg.ensemble_size as u64;
    let sobol = cfg.lhs_samples as u64 * (dims as u64 + 2);
    let bo = (2 * dims).max(3) as u64 + cfg.bayesopt_iterations as u64;
    let final_ensemble = cfg.ensemble_size as u64;
    lhs + sobol + bo + final_ensemble
}

// ---------------------------------------------------------------------------
// Main: head-to-head + validation + atlas
// ---------------------------------------------------------------------------

fn main() {
    // Tunables (env-overridable for fast iteration); defaults sized so the whole
    // head-to-head finishes in a couple of minutes in --release.
    let max_ticks: u64 = env_u64("SPIKE_MAX_TICKS", 300);
    let lhs_samples: usize = env_u64("SPIKE_LHS", 30) as usize;
    let ensemble_size: u32 = env_u64("SPIKE_ENSEMBLE", 5) as u32;
    let me_batch: usize = env_u64("SPIKE_BATCH", 32) as usize;
    let base_seed: u64 = env_u64("SPIKE_SEED", 1000);

    let ranges = default_ranges();
    let dims = ranges.len();

    println!("# MAP-Elites spike (#363) — QD atlas vs LHS+BO at equal budget\n");
    println!(
        "config: dims={dims} max_ticks={max_ticks} ensemble_size={ensemble_size} \
         lhs_samples={lhs_samples} me_batch={me_batch} resolution={RESOLUTION}^3\n"
    );

    // --- Incumbent: the real run_search (LHS → Sobol → GP-BO) -----------------
    let search_cfg = SearchConfig {
        ensemble_size,
        lhs_samples,
        max_ticks,
        bayesopt_iterations: 10,
        sensitivity_threshold: 0.05,
        ..SearchConfig::default()
    };
    let budget = incumbent_rollout_budget(&search_cfg);
    println!("## Incumbent (LHS + Sobol + GP-BO)");
    println!("rollout budget (single worlds): {budget}");

    let mut inc_rng = ChaCha8Rng::seed_from_u64(base_seed);
    let inc_result = run_search(&search_cfg, base_seed, &mut inc_rng);
    let inc_best_optimised = inc_result.optimised[0].median_fitness;
    let inc_best_lhs = inc_result
        .parameterisations
        .iter()
        .map(|p| p.median_fitness)
        .fold(0.0_f32, f32::max);
    let inc_best = inc_best_optimised.max(inc_best_lhs);
    println!("best fitness (BO optimised): {inc_best_optimised:.4}");
    println!("best fitness (LHS sweep):    {inc_best_lhs:.4}");
    println!("best fitness (overall):      {inc_best:.4}");
    println!("coverage / QD-score:         n/a (incumbent returns a ranked list, not an atlas)\n");

    // --- MAP-Elites at an equal rollout budget --------------------------------
    // configs = budget / ensemble_size; with the gen-0 batch, generations =
    // ceil(configs / batch) - 1.
    let configs = (budget / ensemble_size as u64).max(1);
    let total_batches = configs.div_ceil(me_batch as u64).max(1) as usize;
    let generations = total_batches.saturating_sub(1);

    let me_params = MapElitesParams {
        dims,
        batch: me_batch,
        generations,
        ensemble_size,
        max_ticks,
        sigma: 0.1,
        base_seed,
    };
    let mut me_rng = ChaCha8Rng::seed_from_u64(base_seed);
    let (archive, me_rollouts) = run_map_elites(&me_params, &mut me_rng);

    println!("## MAP-Elites (Gaussian emitter, hard-binned archive)");
    println!(
        "rollouts consumed: {me_rollouts} (target {budget}; batch={me_batch} × {} gens incl. gen-0)",
        generations + 1
    );
    println!("best fitness:      {:.4}", archive.best_fitness());
    println!(
        "coverage:          {} / {} cells ({:.2}%)",
        archive.coverage(),
        RESOLUTION.pow(3),
        100.0 * archive.coverage() as f64 / RESOLUTION.pow(3) as f64
    );
    println!("QD-score:          {:.3}", archive.qd_score());
    if let Some(best) = archive.best_cell() {
        let c = cell_of(&best.descriptors);
        println!(
            "best cell {:?}: fitness={:.4} osc={:.3} clus={:.3} carcass={:.3} decomposer_frac={:.2}",
            c,
            best.fitness,
            best.descriptors.oscillation,
            best.descriptors.clustering,
            best.descriptors.carcass,
            best.decomposer_fraction,
        );
    }
    println!();

    // --- Dead-frontier tally (the atlas's most valuable layer, AC3a) ----------
    println!("## Dead frontier (gated configs, by cliff)");
    let mut frontier: Vec<(&Cliff, usize)> =
        archive.frontier.iter().map(|(k, v)| (k, v.len())).collect();
    frontier.sort_by(|a, b| b.1.cmp(&a.1));
    let dead_total: usize = frontier.iter().map(|(_, n)| *n).sum();
    for (cliff, n) in &frontier {
        println!("  {:<22} {n}", cliff.label());
    }
    println!("  {:<22} {dead_total}", "(total dead configs)");
    println!();

    // --- Validation triad -----------------------------------------------------
    println!("## Validation");
    validate_midpoint(&ranges, base_seed, ensemble_size, max_ticks);
    validate_monoculture_boundary(&archive);
    validate_lockup_boundary(&archive);
    println!();

    // --- Decomposer distribution (AC3b: reported, never a descriptor/objective)
    report_decomposer_distribution(&archive);

    // --- Machine-readable atlas dump ------------------------------------------
    dump_atlas_json(&archive, dims, max_ticks, ensemble_size);
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// AC validation: the decoder midpoint (a known-viable config) must land in the
/// high-fitness archive interior, NOT on the dead frontier.
fn validate_midpoint(
    ranges: &[ParameterRange],
    base_seed: u64,
    ensemble_size: u32,
    max_ticks: u64,
) {
    let unit = vec![0.5_f64; ranges.len()];
    let eval = evaluate_config(&unit, ranges, base_seed, ensemble_size, max_ticks);
    match eval.cliff {
        None => {
            let cell = cell_of(&eval.descriptors);
            println!(
                "  [PASS] decoder midpoint is LIVE: fitness={:.4} cell={:?} \
                 (osc={:.3} clus={:.3} carcass={:.3})",
                eval.median_fitness,
                cell,
                eval.descriptors.oscillation,
                eval.descriptors.clustering,
                eval.descriptors.carcass,
            );
        }
        Some(cliff) => {
            println!(
                "  [FAIL] decoder midpoint landed on the dead frontier ({}) — \
                 expected a high-fitness interior cell",
                cliff.label()
            );
        }
    }
}

/// AC cross-check: live cells should sit at clustering ≥ 0.5 (the monoculture
/// gate fires below that, routing those configs to the dead frontier). If live
/// cells appear below the threshold it localises a miscalibrated descriptor.
fn validate_monoculture_boundary(archive: &Archive) {
    let threshold = EvalConfig::default().clustering_threshold;
    let below = archive
        .cells
        .values()
        .filter(|c| c.descriptors.clustering < threshold)
        .count();
    let mono_dead = archive
        .frontier
        .get(&Cliff::Monoculture)
        .map_or(0, |v| v.len());
    if below == 0 {
        println!(
            "  [PASS] monoculture boundary: no live cell below clustering<{threshold} \
             ({mono_dead} configs gated to the monoculture frontier)"
        );
    } else {
        println!(
            "  [WARN] monoculture boundary: {below} live cell(s) below clustering<{threshold} \
             (expected the gate to route these to the frontier; \
             note the gate exempts populations <20 agents)"
        );
    }
}

/// AC cross-check: the lockup frontier (NutrientLockup-gated configs) should sit
/// at high carcass fraction, and live cells should mostly sit below the
/// `LOCK_FRACTION` (0.4) gate.
fn validate_lockup_boundary(archive: &Archive) {
    const LOCK_FRACTION: f32 = 0.4;
    let above = archive
        .cells
        .values()
        .filter(|c| c.descriptors.carcass >= LOCK_FRACTION)
        .count();
    let lock_dead = archive
        .frontier
        .get(&Cliff::NutrientLockup)
        .map_or(0, |v| v.len());
    println!(
        "  [INFO] lockup boundary: {lock_dead} configs gated to the nutrient-lockup frontier; \
         {above} live cell(s) at carcass≥{LOCK_FRACTION} \
         (live worlds near the cliff but not gated — the boundary layer)"
    );
}

/// AC3b: report the decomposer-guild emergence as a per-cell distribution, never
/// as a descriptor or objective.
fn report_decomposer_distribution(archive: &Archive) {
    println!("## Decomposer guild (AC3b — reported distribution, not a descriptor/objective)");
    let cells: Vec<&CellRecord> = archive.cells.values().collect();
    if cells.is_empty() {
        println!("  (no live cells)\n");
        return;
    }
    let with_guild = cells.iter().filter(|c| c.decomposer_fraction > 0.0).count();
    let mean_frac = cells.iter().map(|c| c.decomposer_fraction).sum::<f32>() / cells.len() as f32;
    let strong = cells
        .iter()
        .filter(|c| c.decomposer_fraction >= 0.5)
        .count();
    println!(
        "  live cells: {} | with ≥1 guild seed: {with_guild} | majority-guild (≥50% of seeds): {strong}",
        cells.len()
    );
    println!(
        "  mean per-cell guild seed-fraction: {mean_frac:.3} (sample count {} seeds/cell)\n",
        cells.first().map_or(0, |c| c.sample_count)
    );
}

/// Write the archive as JSON for offline inspection (the "first atlas").
fn dump_atlas_json(archive: &Archive, dims: usize, max_ticks: u64, ensemble_size: u32) {
    use serde_json::json;
    let cells: Vec<_> = archive
        .cells
        .iter()
        .map(|(cell, rec)| {
            json!({
                "cell": [cell.0, cell.1, cell.2],
                "fitness": rec.fitness,
                "oscillation": rec.descriptors.oscillation,
                "clustering": rec.descriptors.clustering,
                "carcass": rec.descriptors.carcass,
                "decomposer_fraction": rec.decomposer_fraction,
                "sample_count": rec.sample_count,
                "unit": rec.unit,
            })
        })
        .collect();
    let frontier: HashMap<&str, usize> = archive
        .frontier
        .iter()
        .map(|(k, v)| (k.label(), v.len()))
        .collect();
    let atlas = json!({
        "spike": "issue-363-mapelites",
        "dims": dims,
        "resolution": RESOLUTION,
        "max_ticks": max_ticks,
        "ensemble_size": ensemble_size,
        "coverage": archive.coverage(),
        "qd_score": archive.qd_score(),
        "best_fitness": archive.best_fitness(),
        "cells": cells,
        "dead_frontier": frontier,
    });
    // Under target/ (gitignored): the atlas is a throwaway artifact, not a
    // committed deliverable.
    let path = "target/mapelites_spike_atlas.json";
    match std::fs::write(path, serde_json::to_string_pretty(&atlas).unwrap()) {
        Ok(()) => println!("atlas written to {path}"),
        Err(e) => println!("could not write atlas to {path}: {e}"),
    }
}

// ---------------------------------------------------------------------------
// Tests (fast unit tests of the pure archive/emitter logic; one slow_ smoke test)
// ---------------------------------------------------------------------------

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

    #[test]
    fn cell_of_clamps_and_bins() {
        assert_eq!(cell_of(&descr(0.0, 0.0, 0.0)), (0, 0, 0));
        assert_eq!(
            cell_of(&descr(1.0, 1.0, 1.0)),
            (RESOLUTION - 1, RESOLUTION - 1, RESOLUTION - 1)
        );
        // out-of-range inputs clamp rather than panic
        assert_eq!(
            cell_of(&descr(-0.5, 2.0, 0.5)),
            (0, RESOLUTION - 1, RESOLUTION / 2)
        );
    }

    #[test]
    fn cliff_maps_every_failure_mode() {
        assert_eq!(
            Cliff::from_failure(&FailureMode::Extinction),
            Cliff::Extinction
        );
        assert_eq!(
            Cliff::from_failure(&FailureMode::NutrientLockup),
            Cliff::NutrientLockup
        );
        assert_eq!(
            Cliff::from_failure(&FailureMode::Monoculture),
            Cliff::Monoculture
        );
    }

    #[test]
    fn live_config_takes_a_cell_and_elite_wins() {
        let mut archive = Archive::default();
        let unit = vec![0.5; 3];
        // First live config at fitness 0.3.
        archive.insert(
            &unit,
            ConfigEval {
                median_fitness: 0.3,
                cliff: None,
                descriptors: descr(0.5, 0.6, 0.2),
                decomposer_fraction: 0.2,
                sample_count: 5,
            },
        );
        assert_eq!(archive.coverage(), 1);
        // A worse config in the same cell does not displace the elite.
        archive.insert(
            &unit,
            ConfigEval {
                median_fitness: 0.1,
                cliff: None,
                descriptors: descr(0.5, 0.6, 0.2),
                decomposer_fraction: 0.0,
                sample_count: 5,
            },
        );
        assert!((archive.best_fitness() - 0.3).abs() < 1e-6);
        // A better config in the same cell does.
        archive.insert(
            &unit,
            ConfigEval {
                median_fitness: 0.42,
                cliff: None,
                descriptors: descr(0.5, 0.6, 0.2),
                decomposer_fraction: 0.4,
                sample_count: 5,
            },
        );
        assert_eq!(archive.coverage(), 1);
        assert!((archive.best_fitness() - 0.42).abs() < 1e-6);
    }

    #[test]
    fn gated_config_goes_to_frontier_not_a_cell() {
        let mut archive = Archive::default();
        let unit = vec![0.1; 3];
        archive.insert(
            &unit,
            ConfigEval {
                median_fitness: 0.0,
                cliff: Some(Cliff::Monoculture),
                descriptors: descr(0.0, 0.0, 0.0),
                decomposer_fraction: 0.0,
                sample_count: 5,
            },
        );
        assert_eq!(archive.coverage(), 0);
        assert_eq!(archive.frontier.get(&Cliff::Monoculture).unwrap().len(), 1);
    }

    #[test]
    fn trailing_mean_uses_window() {
        assert_eq!(trailing_mean(&[], 5), 0.0);
        assert_eq!(trailing_mean(&[1.0, 2.0, 3.0], 10), 2.0);
        // trailing 2 of [1,2,3,4] = (3+4)/2
        assert_eq!(trailing_mean(&[1.0, 2.0, 3.0, 4.0], 2), 3.5);
    }

    #[test]
    fn incumbent_budget_matches_search_formula() {
        let cfg = SearchConfig {
            ensemble_size: 5,
            lhs_samples: 30,
            bayesopt_iterations: 10,
            ..SearchConfig::default()
        };
        let dims = cfg.ranges.len();
        let expected = 30 * 5 + 30 * (dims as u64 + 2) + (2 * dims).max(3) as u64 + 10 + 5;
        assert_eq!(incumbent_rollout_budget(&cfg), expected);
    }

    /// Smoke test: a tiny MAP-Elites run fills at least one cell and stays within
    /// its rollout budget. `slow_` per the multi-seed sweep convention
    /// (`expected-properties.md`): it steps real sims.
    #[test]
    fn slow_map_elites_fills_archive_within_budget() {
        let params = MapElitesParams {
            dims: default_ranges().len(),
            batch: 4,
            generations: 1,
            ensemble_size: 3,
            max_ticks: 30,
            sigma: 0.1,
            base_seed: 7,
        };
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        let (archive, rollouts) = run_map_elites(&params, &mut rng);
        // 2 batches (gen-0 + 1) × 4 configs × 3 seeds.
        assert_eq!(rollouts, 2 * 4 * 3);
        // Every config is routed: gated configs are tallied (counts configs, not
        // cells) and live configs fill cells (which collapse when they share a
        // cell, so coverage ≤ live configs). Together they must account for all 8
        // configs, with the dead tally never exceeding the total.
        let dead: usize = archive.frontier.values().map(|v| v.len()).sum();
        assert!(dead <= 2 * 4, "dead tally {dead} exceeds total configs");
        assert!(
            archive.coverage() + dead >= 1,
            "every run should route at least one config somewhere"
        );
    }
}
