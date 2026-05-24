use std::fs;
use std::path::PathBuf;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use explorers_search::search::{SearchConfig, default_ranges, run_search};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut lhs_samples = 50;
    let mut ensemble_size = 5;
    let mut max_ticks = 500;
    let mut bayesopt_iterations = 10;
    let mut seed = 42u64;
    let mut output_path = PathBuf::from("search_results.json");
    let mut recipe_output_path = PathBuf::from("recipe.json");

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--samples" => { i += 1; lhs_samples = args[i].parse().unwrap(); }
            "--ensemble" => { i += 1; ensemble_size = args[i].parse().unwrap(); }
            "--max-ticks" => { i += 1; max_ticks = args[i].parse().unwrap(); }
            "--bayesopt-iterations" => { i += 1; bayesopt_iterations = args[i].parse().unwrap(); }
            "--seed" => { i += 1; seed = args[i].parse().unwrap(); }
            "--output" => { i += 1; output_path = PathBuf::from(&args[i]); }
            "--recipe-output" => { i += 1; recipe_output_path = PathBuf::from(&args[i]); }
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

    let recipe = result.best_recipe(&default_ranges(), config.max_ticks);
    let recipe_json = serde_json::to_string_pretty(&recipe).unwrap();
    fs::write(&recipe_output_path, &recipe_json).unwrap();

    eprintln!("Results written to {}", output_path.display());
    eprintln!("Recipe written to {}", recipe_output_path.display());
    eprintln!("Top parameterisations by fitness:");
    for (i, p) in result.parameterisations.iter().take(5).enumerate() {
        eprintln!("  {}. fitness = {:.4}", i + 1, p.median_fitness);
    }

    // Diagnostic: summarise failure modes and component scores
    let total_runs: usize = result.parameterisations.iter()
        .map(|p| p.run_breakdowns.len())
        .sum();
    let mut failures: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut surviving_runs = Vec::new();
    for p in &result.parameterisations {
        for r in &p.run_breakdowns {
            if let Some(f) = &r.failure {
                *failures.entry(f.clone()).or_default() += 1;
            } else {
                surviving_runs.push(r);
            }
        }
    }
    eprintln!("\nDiagnostics ({total_runs} total runs):");
    eprintln!("  Failures:");
    if failures.is_empty() {
        eprintln!("    (none)");
    }
    let mut failure_vec: Vec<_> = failures.into_iter().collect();
    failure_vec.sort_by(|a, b| b.1.cmp(&a.1));
    for (mode, count) in &failure_vec {
        eprintln!("    {mode}: {count}");
    }
    eprintln!("  Survived: {}", surviving_runs.len());

    if !surviving_runs.is_empty() {
        let mut os_zero = 0;
        let mut cs_zero = 0;
        let mut cd_zero = 0;
        let mut ts_zero = 0;
        let mut tb_zero = 0;
        let mut os_sum = 0.0_f32;
        let mut cs_sum = 0.0_f32;
        let mut cd_sum = 0.0_f32;
        let mut ts_sum = 0.0_f32;
        let mut tb_sum = 0.0_f32;
        for r in &surviving_runs {
            if r.oscillation_strength == 0.0 { os_zero += 1; }
            if r.clustering_strength == 0.0 { cs_zero += 1; }
            if r.coexistence_duration == 0.0 { cd_zero += 1; }
            if r.turnover_score == 0.0 { ts_zero += 1; }
            if r.trophic_balance_score == 0.0 { tb_zero += 1; }
            os_sum += r.oscillation_strength;
            cs_sum += r.clustering_strength;
            cd_sum += r.coexistence_duration;
            ts_sum += r.turnover_score;
            tb_sum += r.trophic_balance_score;
        }
        let n = surviving_runs.len() as f32;
        eprintln!("  Component scores (surviving runs — zero count / mean):");
        eprintln!("    oscillation:  {os_zero} zero, mean {:.4}", os_sum / n);
        eprintln!("    clustering:   {cs_zero} zero, mean {:.4}", cs_sum / n);
        eprintln!("    coexistence:  {cd_zero} zero, mean {:.4}", cd_sum / n);
        eprintln!("    turnover:     {ts_zero} zero, mean {:.4}", ts_sum / n);
        eprintln!("    trophic_bal:  {tb_zero} zero, mean {:.4}", tb_sum / n);
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
    eprintln!("  --samples N              LHS sample count (default: 50)");
    eprintln!("  --ensemble N             Ensemble size per parameterisation (default: 5)");
    eprintln!("  --max-ticks N            Max simulation ticks per run (default: 500)");
    eprintln!("  --bayesopt-iterations N  Bayesian optimisation iterations (default: 10)");
    eprintln!("  --seed N                 Random seed (default: 42)");
    eprintln!("  --output PATH            Output JSON path (default: search_results.json)");
    eprintln!("  --recipe-output PATH     Recipe JSON path (default: recipe.json)");
    eprintln!("  --help, -h               Show this help");
}
