//! Authority-boundary anchor for the heterotroph guild read (#490).
//!
//! The guild observables (`has_decomposer_guild`, `has_consumer_guild`) are
//! reported only — never a behaviour axis, never a fitness term. This suite
//! pins the fitness, every descriptor and the coexistence duration of the
//! atlas's projected recipe (`recipe.json`) over three seeds to the exact
//! bits they read *before* the guild read landed, so the read can never leak
//! into what the search optimises or bins on.

use explorers_genesis::{EvalConfig, RunConfig, run_single};
use explorers_sim::WorldRecipe;

fn recipe() -> WorldRecipe {
    let path = format!("{}/../../recipe.json", env!("CARGO_MANIFEST_DIR"));
    let contents = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_json::from_str(&contents).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

/// `(seed, fitness, oscillation, clustering, coexistence, turnover, trophic,
/// carcass)` as `f32` bits. First pinned on `main` at 9b51a01; re-captured
/// under #503 (the evaluator reads oscillation and coexistence off the
/// settled window `(T/2, T]`, and the grace became an absolute tick count) —
/// only fitness, oscillation and coexistence moved, and the guild read still
/// touches none of them. Re-captured under #486 (trophic balance scored per
/// agent against the role read, not on cluster means) — only fitness and
/// trophic balance moved. Re-pinned under #494 on the regenerated
/// settled-horizon atlas's recipe: `recipe.json` itself was replaced, so every
/// value is new (a different world, not a changed read). Re-capture with
/// `cargo test -p explorers-genesis --test guild_anchor -- --ignored print_golden --nocapture`.
const GOLDEN: [(u64, [u32; 7]); 3] = [
    (
        1,
        [
            1057418803, 1049594728, 1065353216, 0, 1052099215, 1065353216, 1047125560,
        ],
    ),
    (
        2,
        [
            1057277051, 1051129998, 1065353216, 0, 1049146425, 1065353216, 1046411327,
        ],
    ),
    (
        3,
        [
            1057622103, 1053154461, 1065353216, 0, 1050572489, 1065353216, 1047379013,
        ],
    ),
];

fn readings(seed: u64) -> [u32; 7] {
    let recipe = recipe();
    let run_config = RunConfig {
        max_ticks: recipe.max_ticks,
        eval_config: EvalConfig::default(),
        early_stop_crosscheck_fraction: 0.0,
    };
    let dist = recipe
        .initial_distribution
        .clone()
        .expect("recipe carries a distribution");
    let r = run_single(&recipe.parameters, &dist, &run_config, seed);
    let b = &r.breakdown;
    [
        b.fitness.to_bits(),
        b.oscillation_strength.to_bits(),
        b.clustering_strength.to_bits(),
        b.coexistence_duration.to_bits(),
        b.turnover_score.to_bits(),
        b.trophic_balance_score.to_bits(),
        b.carcass_locked_fraction.to_bits(),
    ]
}

/// Prints the golden table above in source form. Ignored: run by hand when
/// the evaluator's reads change *by design* and the pin must be re-captured.
#[test]
#[ignore]
fn print_golden() {
    for (seed, _) in GOLDEN {
        println!("    ({seed}, {:?}),", readings(seed));
    }
}

#[test]
fn guild_read_leaves_fitness_and_every_descriptor_byte_identical() {
    for (seed, expected) in GOLDEN {
        let got = readings(seed);
        assert_eq!(
            got, expected,
            "seed {seed}: readings {got:?} != golden {expected:?}"
        );
    }
}
