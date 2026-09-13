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
//! Deterministic: a fixed base seed (default 1) and size (default 32) yield the
//! same evidence every run, so the committed snapshot is regenerable and drift
//! shows up as a diff. The default ensemble is 32 because a unanimous `32/32`
//! read is a 95 % Clopper–Pearson lower bound of `p ≥ 0.89` on the modal
//! failure mode's true per-seed rate, where `8/8` bounded only `p ≥ 0.63`
//! (#434, `docs/research/434-ensemble-confidence.md`).

use explorers_genesis_eval::ensemble::{ScenarioAggregate, SeedObservation, SeedScores, aggregate};
use explorers_genesis_eval::{EvalConfig, FailureMode, RolloutObservations, evaluate_from_log};
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

/// The parsed command line: the deterministic seed block and the scenario files.
struct Cli {
    base_seed: u64,
    seeds: u64,
    paths: Vec<String>,
}

/// Parse `[--seed N] [--seeds N] FILE...`. Defaults: base seed 1, 32 seeds —
/// the size #434 recommends for the suite (`32/32 ⇒ p ≥ 0.89`, versus
/// `8/8 ⇒ p ≥ 0.63`). Returns a usage error rather than exiting so it is
/// testable.
fn parse_args(args: impl Iterator<Item = String>) -> Result<Cli, String> {
    let mut cli = Cli {
        base_seed: 1,
        seeds: 32,
        paths: Vec::new(),
    };
    let mut args = args;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" => {
                cli.base_seed = args
                    .next()
                    .and_then(|s| s.parse().ok())
                    .ok_or("--seed requires a number")?;
            }
            "--seeds" => {
                cli.seeds = args
                    .next()
                    .and_then(|s| s.parse().ok())
                    .filter(|&n| n > 0)
                    .ok_or("--seeds requires a positive number")?;
            }
            other => cli.paths.push(other.to_string()),
        }
    }
    if cli.paths.is_empty() {
        return Err("usage: eval_scenarios [--seed N] [--seeds N] FILE...".to_string());
    }
    Ok(cli)
}

fn main() {
    let Cli {
        base_seed,
        seeds,
        paths,
    } = parse_args(std::env::args().skip(1)).unwrap_or_else(|e| die(&e));

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
    // Per-tick series the rollout observes so the evaluator can read each trend:
    // free energy (issue #302), carcass fraction (#342) and producer share (#392)
    // every tick, plus a coarse-interval trait-vector snapshot for coexistence
    // (#394). This binary observes the same rollout the search does, so it samples
    // the same way and bundles the series into the same `RolloutObservations`.
    let mut observations = RolloutObservations::with_capacity(recipe.max_ticks as usize);
    for _ in 0..recipe.max_ticks {
        world.step();
        total_births += world.last_tick_births();
        total_deaths += world.last_tick_deaths();
        observations.observe(&world, config.coexistence_sample_interval);
        if world.agents().is_empty() || world.agents().len() > config.max_population {
            break;
        }
    }

    let breakdown = evaluate_from_log(&world, &observations, config, recipe.max_ticks);

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

#[cfg(test)]
mod tests {
    use super::parse_args;

    #[test]
    fn default_ensemble_is_32_seeds_from_base_seed_1() {
        // #434: a unanimous 32/32 read bounds the modal rate at p >= 0.89 (95 %),
        // versus p >= 0.63 at 8/8 — the suite's default is the recommended size.
        let cli = parse_args(["scenarios/example4.json".to_string()].into_iter()).unwrap();
        assert_eq!(cli.seeds, 32);
        assert_eq!(cli.base_seed, 1);
        assert_eq!(cli.paths, vec!["scenarios/example4.json".to_string()]);
    }
}
