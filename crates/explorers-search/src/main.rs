use std::fs;
use std::path::PathBuf;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use explorers_search::search::{SearchConfig, run_search};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut lhs_samples = 20;
    let mut ensemble_size = 3;
    let mut max_ticks = 100;
    let mut bayesopt_iterations = 10;
    let mut seed = 42u64;
    let mut output_path = PathBuf::from("search_results.json");

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--samples" => { i += 1; lhs_samples = args[i].parse().unwrap(); }
            "--ensemble" => { i += 1; ensemble_size = args[i].parse().unwrap(); }
            "--max-ticks" => { i += 1; max_ticks = args[i].parse().unwrap(); }
            "--bayesopt-iterations" => { i += 1; bayesopt_iterations = args[i].parse().unwrap(); }
            "--seed" => { i += 1; seed = args[i].parse().unwrap(); }
            "--output" => { i += 1; output_path = PathBuf::from(&args[i]); }
            "--help" | "-h" => {
                print_usage();
                return;
            }
            other => {
                eprintln!("Unknown argument: {other}");
                print_usage();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let config = SearchConfig {
        lhs_samples,
        ensemble_size,
        max_ticks,
        bayesopt_iterations,
        ..Default::default()
    };

    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    eprintln!("Running parameter search...");
    eprintln!("  LHS samples: {lhs_samples}");
    eprintln!("  Ensemble size: {ensemble_size}");
    eprintln!("  Max ticks: {max_ticks}");
    eprintln!("  Bayesian opt iterations: {bayesopt_iterations}");
    eprintln!("  Seed: {seed}");

    let result = run_search(&config, seed, &mut rng);

    let json = serde_json::to_string_pretty(&result).unwrap();
    fs::write(&output_path, &json).unwrap();

    eprintln!("Results written to {}", output_path.display());
    eprintln!("Top parameterisations by fitness:");
    for (i, p) in result.parameterisations.iter().take(5).enumerate() {
        eprintln!("  {}. fitness = {:.4}", i + 1, p.median_fitness);
    }

    if !result.sensitivity.rankings.is_empty() {
        eprintln!("\nSensitivity ranking (by total-effect):");
        for entry in result.sensitivity.rankings.iter().take(10) {
            eprintln!(
                "  {} — S1: {:.3}, ST: {:.3}",
                entry.name, entry.first_order, entry.total_effect
            );
        }
    }
}

fn print_usage() {
    eprintln!("Usage: explorers-search [OPTIONS]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --samples N              LHS sample count (default: 20)");
    eprintln!("  --ensemble N             Ensemble size per parameterisation (default: 3)");
    eprintln!("  --max-ticks N            Max simulation ticks per run (default: 100)");
    eprintln!("  --bayesopt-iterations N  Bayesian optimisation iterations (default: 10)");
    eprintln!("  --seed N                 Random seed (default: 42)");
    eprintln!("  --output PATH            Output JSON path (default: search_results.json)");
    eprintln!("  --help, -h               Show this help");
}
