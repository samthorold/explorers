use std::fs;
use std::path::PathBuf;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use explorers_search::search::{SearchConfig, default_ranges, run_search};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut ensemble_size = 5;
    let mut max_ticks = 500;
    let mut batch = 32;
    let mut generations = 10;
    let mut seed = 42u64;
    let mut output_path = PathBuf::from("atlas.json");
    let mut recipe_output_path = PathBuf::from("recipe.json");

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--batch" => {
                i += 1;
                batch = args[i].parse().unwrap();
            }
            "--generations" => {
                i += 1;
                generations = args[i].parse().unwrap();
            }
            "--ensemble" => {
                i += 1;
                ensemble_size = args[i].parse().unwrap();
            }
            "--max-ticks" => {
                i += 1;
                max_ticks = args[i].parse().unwrap();
            }
            "--seed" => {
                i += 1;
                seed = args[i].parse().unwrap();
            }
            "--output" => {
                i += 1;
                output_path = PathBuf::from(&args[i]);
            }
            "--recipe-output" => {
                i += 1;
                recipe_output_path = PathBuf::from(&args[i]);
            }
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
        ensemble_size,
        max_ticks,
        batch,
        generations,
        ..Default::default()
    };

    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    eprintln!("Running QD genesis search (CMA-MAE atlas)...");
    eprintln!("  Batch size: {batch}");
    eprintln!("  Generations: {generations}");
    eprintln!("  Ensemble size: {ensemble_size}");
    eprintln!("  Max ticks: {max_ticks}");
    eprintln!("  Seed: {seed}");

    let atlas = run_search(&config, seed, &mut rng);

    let json = serde_json::to_string_pretty(&atlas).unwrap();
    fs::write(&output_path, &json).unwrap();

    match atlas.best_recipe(&default_ranges(), config.max_ticks) {
        Some(recipe) => {
            let recipe_json = serde_json::to_string_pretty(&recipe).unwrap();
            fs::write(&recipe_output_path, &recipe_json).unwrap();
            eprintln!(
                "Recipe (best live cell) written to {}",
                recipe_output_path.display()
            );
        }
        None => {
            eprintln!(
                "No live cell found — the atlas is all dead frontier; no recipe written. \
                 Try a larger budget (more generations / batch)."
            );
        }
    }

    eprintln!("Atlas written to {}", output_path.display());

    // Atlas summary: coverage / QD-score, the top live cells, and the dead
    // frontier (the failure tally) — the search's output is a map, not a ranked
    // list.
    eprintln!(
        "\nCoverage: {} / {} cells ({:.3}%)",
        atlas.coverage,
        atlas.total_cells,
        100.0 * atlas.coverage as f64 / atlas.total_cells as f64
    );
    eprintln!("QD-score (Σ elite fitness): {:.3}", atlas.qd_score);
    eprintln!("Best fitness: {:.4}", atlas.best_fitness);

    let mut top: Vec<_> = atlas.cells.iter().collect();
    top.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());
    eprintln!("\nTop live cells (by fitness):");
    if top.is_empty() {
        eprintln!("  (none — all configs died)");
    }
    for cell in top.iter().take(10) {
        eprintln!(
            "  cell {:?}: fitness={:.4} osc={:.3} clus={:.3} carcass={:.3} \
             decomposer_frac={:.2} (n={})",
            cell.cell,
            cell.fitness,
            cell.oscillation,
            cell.clustering,
            cell.carcass,
            cell.decomposer_fraction,
            cell.sample_count,
        );
    }

    eprintln!("\nDead frontier (configs by cliff):");
    let mut frontier: Vec<_> = atlas.dead_frontier.iter().collect();
    frontier.sort_by(|a, b| b.1.cmp(a.1));
    if frontier.is_empty() {
        eprintln!("  (none)");
    }
    let dead_total: usize = frontier.iter().map(|(_, n)| **n).sum();
    for (cliff, n) in &frontier {
        eprintln!("  {cliff:<22} {n}");
    }
    eprintln!("  {:<22} {dead_total}", "(total dead configs)");
}

fn print_usage() {
    eprintln!("Usage: explorers-search [OPTIONS]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --batch N           Solutions evaluated per generation (default: 32)");
    eprintln!("  --generations N     Adaptation generations after bootstrap (default: 10)");
    eprintln!("  --ensemble N        Ensemble size per parameterisation (default: 5)");
    eprintln!("  --max-ticks N       Max simulation ticks per run (default: 500)");
    eprintln!("  --seed N            Random seed (default: 42)");
    eprintln!("  --output PATH       Atlas JSON path (default: atlas.json)");
    eprintln!("  --recipe-output PATH  Recipe JSON path (default: recipe.json)");
    eprintln!("  --help, -h          Show this help");
}
