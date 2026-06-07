//! Run scenario files through the genesis evaluator and emit the observed
//! outcome as JSON — the **example lens** of the validation triad (#293).
//!
//! "Sensible" is defined by the same expected-properties machinery genesis uses
//! (`evaluate_from_log`), so an example run and a genesis point describe one
//! world rather than two private notions of success. The output is the
//! deterministic *evidence* — not a verdict. Whether a scenario is sensible is
//! read from this evidence against the scenario's declared probed mode and
//! prediction; that read is deliberately left to a human or an agent.
//!
//! **Ensemble, not a single seed (#314).** Each scenario is run over a
//! deterministic seed set `base_seed .. base_seed + N` (mirroring genesis's
//! `run_ensemble`, which medians over `base_seed.wrapping_add(i)`), and the
//! emitted evidence is a *distribution*: a per-scenario aggregate (failure-mode
//! distribution + modal mode, median/spread of every score) plus a `per_seed`
//! breakdown. Regime-sensitive scenarios (example6) flip between regimes on
//! small changes, so a single draw can hinge the verdict; the distribution is
//! the robust read. The binary stays prediction-agnostic — it does not read
//! `metadata.prediction` or apply a pass/fail threshold; the
//! majority/supermajority read against the prediction lives in `verdicts.md`.
//!
//! Usage:
//!   cargo run -p explorers-genesis-eval --bin eval_scenarios -- [--seed N] [--seeds N] FILE...
//!   cargo run -p explorers-genesis-eval --bin eval_scenarios -- scenarios/example*.json > scenarios/observed.json
//!
//! Deterministic: a fixed base seed (default 1) and size (default 8) yield the
//! same evidence every run, so the committed snapshot is regenerable and drift
//! shows up as a diff.

use explorers_genesis_eval::ensemble::{ScenarioAggregate, SeedObservation, SeedScores, aggregate};
use explorers_genesis_eval::{EvalConfig, FailureMode, evaluate_from_log};
use explorers_sim::{World, WorldRecipe};

/// Name the failure mode for the JSON output. `None` is "none" — the run did not
/// trip any of the degenerate-configuration detectors.
fn failure_name(failure: &Option<FailureMode>) -> &'static str {
    match failure {
        None => "none",
        Some(FailureMode::Extinction) => "extinction",
        Some(FailureMode::PopulationExplosion) => "population_explosion",
        Some(FailureMode::EnergyDeath) => "energy_death",
        Some(FailureMode::Monoculture) => "monoculture",
        Some(FailureMode::GeneralistDominance) => "generalist_dominance",
        Some(FailureMode::NutrientLockup) => "nutrient_lockup",
    }
}

fn main() {
    let mut base_seed = 1u64;
    let mut seeds = 8u64;
    let mut paths: Vec<String> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" => {
                base_seed = args
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| die("--seed requires a number"));
            }
            "--seeds" => {
                seeds = args
                    .next()
                    .and_then(|s| s.parse().ok())
                    .filter(|&n| n > 0)
                    .unwrap_or_else(|| die("--seeds requires a positive number"));
            }
            other => paths.push(other.to_string()),
        }
    }
    if paths.is_empty() {
        die::<()>("usage: eval_scenarios [--seed N] [--seeds N] FILE...");
    }

    let config = EvalConfig::default();
    let mut out = Vec::with_capacity(paths.len());
    for path in &paths {
        out.push(eval_scenario(path, base_seed, seeds, &config));
    }
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}

/// Run one scenario across the deterministic seed ensemble
/// `base_seed .. base_seed + seeds` (mirroring genesis's
/// `base_seed.wrapping_add(i)`) and aggregate the per-seed evidence into its
/// ensemble distribution.
fn eval_scenario(path: &str, base_seed: u64, seeds: u64, config: &EvalConfig) -> ScenarioAggregate {
    let contents =
        std::fs::read_to_string(path).unwrap_or_else(|e| die(&format!("read {path}: {e}")));
    let recipe: WorldRecipe =
        serde_json::from_str(&contents).unwrap_or_else(|e| die(&format!("parse {path}: {e}")));
    let name = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string());

    let per_seed: Vec<SeedObservation> = (0..seeds)
        .map(|i| eval_one(&recipe, base_seed.wrapping_add(i), config))
        .collect();

    aggregate(&name, recipe.max_ticks, base_seed, per_seed)
}

/// Run one scenario at one seed to its `max_ticks` (terminating early on
/// extinction or population explosion, exactly as the genesis runner does) and
/// package the evaluator's verdict as one seed's evidence row.
fn eval_one(recipe: &WorldRecipe, seed: u64, config: &EvalConfig) -> SeedObservation {
    let mut world = World::from_recipe(recipe, seed);
    // Accumulate the true per-tick birth/death counts as demographic evidence —
    // the evaluator's own `Reproduced`-event tally double-counts (parent-pair plus
    // per-offspring events), so it is not a usable birth count on its own.
    let mut total_births = 0usize;
    let mut total_deaths = 0usize;
    // Sample the free (non-carcass-locked) energy stock each tick so the
    // evaluator can read its trend — energy death is this stock collapsing into
    // carcasses (issue #302). The world stays history-free; the series lives here.
    let mut free_energy_per_tick: Vec<f32> = Vec::with_capacity(recipe.max_ticks as usize);
    // Sample the dead pool's share of system nutrient each tick so the evaluator
    // can read its trend — nutrient lockup is this fraction sequestering high and
    // staying there as carcasses out-accumulate the decomposers (issue #342).
    let mut carcass_fraction_per_tick: Vec<f32> = Vec::with_capacity(recipe.max_ticks as usize);
    // Sample the producer (autotroph) share of living energy each tick so the
    // evaluator can read the producer↔consumer rhythm — the oscillation descriptor
    // is the anti-correlation depth of this detrended series (issue #392).
    let mut producer_share_per_tick: Vec<f32> = Vec::with_capacity(recipe.max_ticks as usize);
    for _ in 0..recipe.max_ticks {
        world.step();
        total_births += world.last_tick_births();
        total_deaths += world.last_tick_deaths();
        free_energy_per_tick.push(world.free_energy());
        carcass_fraction_per_tick.push(world.carcass_locked_nutrient_fraction());
        producer_share_per_tick.push(world.producer_energy_share());
        if world.agents().is_empty() || world.agents().len() > config.max_population {
            break;
        }
    }

    let breakdown = evaluate_from_log(
        &world,
        &free_energy_per_tick,
        &carcass_fraction_per_tick,
        &producer_share_per_tick,
        config,
        recipe.max_ticks,
    );

    SeedObservation {
        seed,
        failure_mode: failure_name(&breakdown.failure).to_string(),
        ticks_survived: breakdown.ticks_survived,
        final_population: world.agents().len(),
        total_births,
        total_deaths,
        scores: SeedScores {
            oscillation_strength: breakdown.oscillation_strength,
            clustering_strength: breakdown.clustering_strength,
            coexistence_duration: breakdown.coexistence_duration,
            turnover_score: breakdown.turnover_score,
            trophic_balance_score: breakdown.trophic_balance_score,
            fitness: breakdown.fitness,
        },
    }
}

fn die<T>(msg: &str) -> T {
    eprintln!("{msg}");
    std::process::exit(1);
}
