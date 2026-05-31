//! Run scenario files through the genesis evaluator and emit the observed
//! outcome as JSON — the **example lens** of the validation triad (#293).
//!
//! "Sensible" is defined by the same expected-properties machinery genesis uses
//! (`evaluate_from_log`), so an example run and a genesis point describe one
//! world rather than two private notions of success. The output is the
//! deterministic *evidence* (failure mode against the six, plus the five
//! sensible-world criterion scores) — not a verdict. Whether a scenario is
//! sensible is read from this evidence against the scenario's declared probed
//! mode and prediction; that read is deliberately left to a human or an agent.
//!
//! Usage:
//!   cargo run -p explorers-genesis-eval --bin eval_scenarios -- [--seed N] FILE...
//!   cargo run -p explorers-genesis-eval --bin eval_scenarios -- scenarios/*.json > scenarios/observed.json
//!
//! Deterministic: a fixed seed (default 1) yields the same evidence every run,
//! so the committed snapshot is regenerable and drift shows up as a diff.

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
    }
}

fn main() {
    let mut seed = 1u64;
    let mut paths: Vec<String> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" => {
                seed = args
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| die("--seed requires a number"));
            }
            other => paths.push(other.to_string()),
        }
    }
    if paths.is_empty() {
        die::<()>("usage: eval_scenarios [--seed N] FILE...");
    }

    let config = EvalConfig::default();
    let mut out = Vec::with_capacity(paths.len());
    for path in &paths {
        out.push(eval_one(path, seed, &config));
    }
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}

/// Run one scenario to its `max_ticks` (terminating early on extinction or
/// population explosion, exactly as the genesis runner does) and package the
/// evaluator's verdict as JSON evidence.
fn eval_one(path: &str, seed: u64, config: &EvalConfig) -> serde_json::Value {
    let contents =
        std::fs::read_to_string(path).unwrap_or_else(|e| die(&format!("read {path}: {e}")));
    let recipe: WorldRecipe =
        serde_json::from_str(&contents).unwrap_or_else(|e| die(&format!("parse {path}: {e}")));

    let mut world = World::from_recipe(&recipe, seed);
    // Accumulate the true per-tick birth/death counts as demographic evidence —
    // the evaluator's own `Reproduced`-event tally double-counts (parent-pair plus
    // per-offspring events), so it is not a usable birth count on its own.
    let mut total_births = 0usize;
    let mut total_deaths = 0usize;
    for _ in 0..recipe.max_ticks {
        world.step();
        total_births += world.last_tick_births();
        total_deaths += world.last_tick_deaths();
        if world.agents().is_empty() || world.agents().len() > config.max_population {
            break;
        }
    }

    let breakdown = evaluate_from_log(&world, config, recipe.max_ticks);
    let name = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string());

    serde_json::json!({
        "scenario": name,
        "seed": seed,
        "max_ticks": recipe.max_ticks,
        "failure_mode": failure_name(&breakdown.failure),
        "ticks_survived": breakdown.ticks_survived,
        "final_population": world.agents().len(),
        "total_births": total_births,
        "total_deaths": total_deaths,
        "scores": {
            "oscillation_strength": breakdown.oscillation_strength,
            "clustering_strength": breakdown.clustering_strength,
            "coexistence_duration": breakdown.coexistence_duration,
            "turnover_score": breakdown.turnover_score,
            "trophic_balance_score": breakdown.trophic_balance_score,
            "fitness": breakdown.fitness,
        },
    })
}

fn die<T>(msg: &str) -> T {
    eprintln!("{msg}");
    std::process::exit(1);
}
