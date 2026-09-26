//! The search CLI's re-projection path (#531): `--reproject ATLAS` runs the
//! refinement and recipe projection against an atlas file, without the search.

use std::path::PathBuf;
use std::process::{Command, Output};

use explorers_search::atlas_file::write_atlas;
use explorers_search::qd::{
    AtlasProvenance, CoexistenceFloor, QdConfig, RefinementConfig, refined_best_recipe, run_qd,
};
use explorers_search::search::default_ranges;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "explorers-reproject-cli-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn search_cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_explorers-search"))
        .args(args)
        .output()
        .expect("the search CLI runs")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

const SEED: u64 = 11;

fn tiny() -> QdConfig {
    QdConfig {
        ensemble_size: 1,
        max_ticks: 20,
        batch: 4,
        generations: 1,
        ..QdConfig::default()
    }
}

#[test]
fn reproject_writes_the_recipe_the_writing_run_projected() {
    let config = tiny();
    let atlas = run_qd(&config, SEED, &mut ChaCha8Rng::seed_from_u64(SEED));
    let in_run = refined_best_recipe(
        &atlas,
        &default_ranges(),
        &RefinementConfig {
            top_k: 2,
            ensemble_size: 2,
            max_ticks: config.max_ticks,
            floor: CoexistenceFloor::Plain,
            ..RefinementConfig::default()
        },
        SEED,
    );
    let dir = scratch("equivalence");
    let atlas_path = dir.join("atlas.json");
    let recipe_path = dir.join("recipe.json");
    write_atlas(&atlas, &atlas_path).unwrap();

    let output = search_cli(&[
        "--reproject",
        atlas_path.to_str().unwrap(),
        "--recipe-output",
        recipe_path.to_str().unwrap(),
        "--refine-top-k",
        "2",
        "--refine-ensemble",
        "2",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        !stderr(&output).contains("Running QD genesis search"),
        "the search must not run: {}",
        stderr(&output)
    );

    // Byte-for-byte: the recipe file the full run would have written.
    assert_eq!(
        std::fs::read_to_string(&recipe_path).unwrap(),
        serde_json::to_string_pretty(in_run.recipe.as_ref().unwrap()).unwrap()
    );
}

#[test]
fn reproject_keeps_the_fallback_warning_when_no_refined_cell_clears_the_floor() {
    // One live cell whose elite (mid-cube, one knob at its floor) does not
    // coexist when refined at a 100-tick horizon: the projection falls back to
    // the argmax-fitness straddler — and must still say so on this path.
    let horizon = 100;
    let mut atlas = run_qd(&tiny(), SEED, &mut ChaCha8Rng::seed_from_u64(SEED));
    atlas.provenance = Some(AtlasProvenance {
        seed: SEED,
        max_ticks: horizon,
    });
    atlas.cells.truncate(1);
    let dims = atlas.cells[0].unit.len();
    atlas.cells[0].unit = vec![0.5; dims];
    atlas.cells[0].unit[11] = 0.0;
    let in_run = refined_best_recipe(
        &atlas,
        &default_ranges(),
        &RefinementConfig {
            top_k: 1,
            ensemble_size: 1,
            max_ticks: horizon,
            floor: CoexistenceFloor::Plain,
            ..RefinementConfig::default()
        },
        SEED,
    );
    assert!(in_run.recipe.is_some() && !in_run.cleared_floor);
    let dir = scratch("fallback");
    let atlas_path = dir.join("atlas.json");
    write_atlas(&atlas, &atlas_path).unwrap();

    let output = search_cli(&[
        "--reproject",
        atlas_path.to_str().unwrap(),
        "--recipe-output",
        dir.join("recipe.json").to_str().unwrap(),
        "--refine-top-k",
        "1",
        "--refine-ensemble",
        "1",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stderr(&output)
            .contains("WARNING: no refined top-1 cell clears the plain coexistence floor"),
        "{}",
        stderr(&output)
    );
    assert!(dir.join("recipe.json").exists());
}

#[test]
fn reproject_projects_under_the_coexistence_floor_given() {
    // #538: --coexistence-floor reaches the re-projection, which writes the
    // recipe the in-run projection picks under that floor and names the floor.
    let config = tiny();
    let atlas = run_qd(&config, SEED, &mut ChaCha8Rng::seed_from_u64(SEED));
    let in_run = refined_best_recipe(
        &atlas,
        &default_ranges(),
        &RefinementConfig {
            top_k: 2,
            ensemble_size: 2,
            max_ticks: config.max_ticks,
            floor: CoexistenceFloor::Either,
            ..RefinementConfig::default()
        },
        SEED,
    );
    let dir = scratch("floor");
    let atlas_path = dir.join("atlas.json");
    let recipe_path = dir.join("recipe.json");
    write_atlas(&atlas, &atlas_path).unwrap();

    let output = search_cli(&[
        "--reproject",
        atlas_path.to_str().unwrap(),
        "--recipe-output",
        recipe_path.to_str().unwrap(),
        "--refine-top-k",
        "2",
        "--refine-ensemble",
        "2",
        "--coexistence-floor",
        "either",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("Floor: either"),
        "{}",
        stderr(&output)
    );
    assert_eq!(
        std::fs::read_to_string(&recipe_path).unwrap(),
        serde_json::to_string_pretty(in_run.recipe.as_ref().unwrap()).unwrap()
    );
}

#[test]
fn an_unknown_coexistence_floor_is_refused() {
    let dir = scratch("bad-floor");
    let output = search_cli(&[
        "--reproject",
        dir.join("atlas.json").to_str().unwrap(),
        "--coexistence-floor",
        "guild",
    ]);
    assert!(!output.status.success());
    let err = stderr(&output);
    assert!(err.contains("--coexistence-floor"), "{err}");
    assert!(!err.contains("panicked"), "{err}");
}

#[test]
fn reproject_of_a_missing_atlas_fails_clearly_without_panicking() {
    let dir = scratch("missing");
    let output = search_cli(&[
        "--reproject",
        dir.join("no-such-atlas.json").to_str().unwrap(),
        "--recipe-output",
        dir.join("recipe.json").to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    let err = stderr(&output);
    assert!(err.contains("no-such-atlas.json"), "{err}");
    assert!(!err.contains("panicked"), "{err}");
    assert!(!dir.join("recipe.json").exists());
}

#[test]
fn reproject_refuses_the_flags_that_configure_a_search() {
    // --reproject runs no search: a search knob given with it is a mistake
    // worth naming, not something to ignore silently.
    let dir = scratch("search-flags");
    let output = search_cli(&[
        "--reproject",
        dir.join("atlas.json").to_str().unwrap(),
        "--generations",
        "3",
    ]);
    assert!(!output.status.success());
    let err = stderr(&output);
    assert!(err.contains("--generations"), "{err}");
    assert!(!err.contains("panicked"), "{err}");
}

#[test]
fn usage_documents_reproject() {
    let output = search_cli(&["--help"]);
    assert!(
        stderr(&output).contains("--reproject PATH"),
        "{}",
        stderr(&output)
    );
}
