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
/// trophic balance moved. Re-capture with
/// `cargo test -p explorers-genesis --test guild_anchor -- --ignored print_golden --nocapture`.
const GOLDEN: [(u64, [u32; 7]); 3] = [
    (
        1,
        [
            1060910181, 1054493816, 1065301997, 1065353216, 1052535423, 1063416466, 1026108230,
        ],
    ),
    (
        2,
        [
            1061134438, 1054501779, 1065353216, 1065353216, 1057937687, 1061294880, 1040666870,
        ],
    ),
    (
        3,
        [
            1060063743, 1058105126, 1065353216, 1065353216, 1047099605, 1059105949, 1031849468,
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
