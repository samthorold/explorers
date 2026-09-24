use std::fs;
use std::path::PathBuf;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use explorers_search::checkpoint::inspect;
use explorers_search::qd::{
    COEXISTENCE_FLOOR, REFINE_ENSEMBLE_SIZE, REFINE_TOP_K, RefinementConfig, refined_best_recipe,
};
use explorers_search::search::{
    SearchConfig, default_ranges, resume_search, run_search_checkpointed, run_search_observed,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut ensemble_size = 5;
    let mut max_ticks = 2000;
    let mut batch = 32;
    let mut generations = 10;
    let mut seed = 42u64;
    let mut output_path = PathBuf::from("atlas.json");
    let mut recipe_output_path = PathBuf::from("recipe.json");
    let mut refine_top_k = REFINE_TOP_K;
    let mut refine_ensemble = REFINE_ENSEMBLE_SIZE;
    let mut checkpoint_path: Option<PathBuf> = None;
    let mut resume_path: Option<PathBuf> = None;

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
            "--refine-top-k" => {
                i += 1;
                refine_top_k = args[i].parse().unwrap();
            }
            "--refine-ensemble" => {
                i += 1;
                refine_ensemble = args[i].parse().unwrap();
            }
            "--checkpoint" => {
                i += 1;
                checkpoint_path = Some(PathBuf::from(&args[i]));
            }
            "--resume" => {
                i += 1;
                resume_path = Some(PathBuf::from(&args[i]));
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

    // One line per completed generation (#529): a multi-hour regeneration is
    // otherwise silent between the header above and the summary below. With a
    // checkpoint (#530), each line also means that generation is on disk.
    let mut report = |report: &explorers_search::qd::GenerationReport| {
        eprintln!("  {report}");
    };
    let searched = match (checkpoint_path, resume_path) {
        (Some(_), Some(_)) => {
            eprintln!(
                "--checkpoint and --resume are exclusive: --resume PATH keeps checkpointing \
                 to PATH"
            );
            std::process::exit(1);
        }
        (None, None) => Ok(run_search_observed(&config, seed, &mut rng, &mut report)),
        (Some(path), None) => {
            // Never overwrite a checkpoint silently: it may be hours of search.
            if path.exists() {
                eprintln!(
                    "Checkpoint {} already exists. Resume it with --resume {}, or remove it \
                     to start a fresh search.",
                    path.display(),
                    path.display()
                );
                std::process::exit(1);
            }
            eprintln!("  Checkpoint: {} (every generation)", path.display());
            run_search_checkpointed(&config, seed, rng, &path, &mut report)
        }
        (None, Some(path)) => match inspect(&path) {
            Ok(summary) => {
                if summary.next_generation > generations {
                    eprintln!(
                        "  Resuming from {}: its search is finished, reading back its atlas",
                        path.display()
                    );
                } else {
                    eprintln!(
                        "  Resuming from {} at generation {} of {}",
                        path.display(),
                        summary.next_generation,
                        generations
                    );
                }
                resume_search(&config, seed, &path, &mut report)
            }
            Err(e) => Err(e),
        },
    };
    let atlas = match searched {
        Ok(atlas) => atlas,
        Err(e) => {
            eprintln!("Search stopped: {e}");
            std::process::exit(1);
        }
    };

    let json = serde_json::to_string_pretty(&atlas).unwrap();
    fs::write(&output_path, &json).unwrap();

    // Gated elite refinement (#404): re-evaluate the top-K live cells at a larger,
    // independent-seeded ensemble before projecting, so the high-variance in-run n=5
    // estimate that both fitness and the coexistence floor depend on is hardened.
    // Selection only — the atlas map (binning, per-cell fitness) is untouched.
    let refinement = RefinementConfig {
        top_k: refine_top_k,
        ensemble_size: refine_ensemble,
        max_ticks: config.max_ticks,
    };
    eprintln!(
        "\nRefining top-{} live cells at ensemble n={} (independent seeds)...",
        refine_top_k, refine_ensemble
    );
    let projection = refined_best_recipe(&atlas, &default_ranges(), &refinement, seed);

    if !projection.refined.is_empty() {
        eprintln!("Refined cells (recorded → refined coexistence fraction):");
        for r in &projection.refined {
            eprintln!(
                "  cell {:?}: fitness={:.4} coexist {:.2} → {:.2} (n={}) refined_fit={:.4}{}",
                r.cell,
                r.recorded_fitness,
                r.recorded_coexistence_fraction,
                r.refined_coexistence_fraction,
                r.refined_sample_count,
                r.refined_median_fitness,
                if r.clears_floor { " ✓" } else { "" },
            );
        }
        if projection.unrefined_live_cells > 0 {
            eprintln!(
                "  ({} lower-fitness live cell(s) below the top-{} cut were not refined)",
                projection.unrefined_live_cells, refine_top_k
            );
        }
    }

    match projection.recipe {
        Some(recipe) => {
            let recipe_json = serde_json::to_string_pretty(&recipe).unwrap();
            fs::write(&recipe_output_path, &recipe_json).unwrap();
            eprintln!(
                "Recipe (best refined-robust live cell) written to {}",
                recipe_output_path.display()
            );
            // Warn when the refinement had to fall back below the floor: no top-K
            // cell stays robustly sensible under the larger ensemble, so the recipe
            // is a bifurcation straddler — most of its ensemble does NOT coexist
            // (#401, #404).
            if !projection.cleared_floor {
                eprintln!(
                    "  WARNING: no refined top-{} cell clears the coexistence floor ({:.2}); the \
                     recipe is the argmax-fitness straddler (a lucky-draw world). Try a larger \
                     budget, ensemble, or --refine-top-k.",
                    refine_top_k, COEXISTENCE_FLOOR
                );
            }
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
             decomposer_frac={:.2} consumer_frac={:.2} coexist_frac={:.2} (n={})",
            cell.cell,
            cell.fitness,
            cell.oscillation,
            cell.clustering,
            cell.carcass,
            cell.decomposer_fraction,
            cell.consumer_fraction,
            cell.coexistence_fraction,
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

    // Predicted bifurcation coordinates (#372): the closed-form distance-to-
    // bifurcation readings F validated (#358 Hopf, #359 branching), wired in as
    // per-cell *descriptors* plus a predicted-vs-observed cross-check — never a
    // fitness term, never a binning axis. Report the spread over live cells and
    // the disagreement count split by regime.
    eprintln!("\nPredicted bifurcation coordinates (live-cell spread):");
    if atlas.cells.is_empty() {
        eprintln!("  (none — no live cells)");
    } else {
        let spread = |get: &dyn Fn(&explorers_search::qd::AtlasCell) -> f32| {
            let vals: Vec<f32> = atlas.cells.iter().map(get).collect();
            let n = vals.len() as f32;
            let min = vals.iter().copied().fold(f32::INFINITY, f32::min);
            let max = vals.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let mean = vals.iter().sum::<f32>() / n;
            (min, mean, max)
        };
        let (omin, omean, omax) = spread(&|c| c.predicted_oscillation_distance);
        let (bmin, bmean, bmax) = spread(&|c| c.predicted_branching_distance);
        eprintln!("  oscillation |lambda|-1  min={omin:+.4} mean={omean:+.4} max={omax:+.4}");
        eprintln!("  branching D             min={bmin:+.4} mean={bmean:+.4} max={bmax:+.4}");
    }

    let (mut n_validated, mut n_weak) = (0usize, 0usize);
    for d in &atlas.bifurcation_disagreements {
        match d.regime {
            explorers_search::qd::CrosscheckRegime::Validated => n_validated += 1,
            explorers_search::qd::CrosscheckRegime::WeakObservable => n_weak += 1,
        }
    }
    eprintln!(
        "Bifurcation cross-check disagreements: {} total ({} validated-regime, {} weak-observable)",
        atlas.bifurcation_disagreements.len(),
        n_validated,
        n_weak
    );
    eprintln!(
        "  (a weak-observable disagreement localises to the genesis observable, not F's reading; \
         objective-promotion stays gated on observable-hardening — #358/#359.)"
    );

    // Early-stop cross-check (#506): rollouts the incremental energy-death /
    // lockup gates stopped where they died, but which the carry took to the
    // horizon anyway. A stopped-dead / alive-at-T disagreement is a gate that
    // fired on a reversible collapse — surfaced here, never swallowed.
    eprintln!(
        "Early-stop cross-check disagreements: {} of {} stopped rollout(s) carried to the \
         horizon read alive on the full series (carry fraction {:.2})",
        atlas.early_stop_disagreements.len(),
        atlas.early_stop_crosschecks,
        config.early_stop_crosscheck_fraction,
    );
    for d in &atlas.early_stop_disagreements {
        eprintln!(
            "  {:<16} stopped at tick {:<6} seed #{:<2} horizon fitness={:.4}",
            d.gate, d.stop_tick, d.seed_index, d.observed_fitness
        );
    }
}

fn print_usage() {
    eprintln!("Usage: explorers-search [OPTIONS]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --batch N           Solutions evaluated per generation (default: 32)");
    eprintln!("  --generations N     Adaptation generations after bootstrap (default: 10)");
    eprintln!("  --ensemble N        Ensemble size per parameterisation (default: 5)");
    eprintln!("  --max-ticks N       Max simulation ticks per run (default: 2000)");
    eprintln!("  --seed N            Random seed (default: 42)");
    eprintln!("  --output PATH       Atlas JSON path (default: atlas.json)");
    eprintln!("  --recipe-output PATH  Recipe JSON path (default: recipe.json)");
    eprintln!("  --refine-top-k N    Top live cells to refine before projecting (default: 10)");
    eprintln!("  --refine-ensemble N Refinement ensemble size, independent seeds (default: 32)");
    eprintln!("  --checkpoint PATH   Write the search state to PATH at every generation");
    eprintln!("                      boundary (atomically), so an interrupted search can be");
    eprintln!("                      resumed. Refuses to overwrite an existing PATH.");
    eprintln!("  --resume PATH       Resume the search checkpointed at PATH and keep");
    eprintln!("                      checkpointing there. The resumed atlas is exactly the");
    eprintln!("                      uninterrupted one. Pass the same search options and");
    eprintln!("                      --seed as the original run; a mismatch is refused.");
    eprintln!("  --help, -h          Show this help");
}
