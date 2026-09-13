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
/// carcass)` as `f32` bits, pinned on `main` at 9b51a01.
const GOLDEN: [(u64, [u32; 7]); 3] = [
    (
        1,
        [
            1061316563, 1057502092, 1065301997, 1063675494, 1052535423, 1065353216, 1026108230,
        ],
    ),
    (
        2,
        [
            1061487686, 1049917588, 1065353216, 1065353216, 1057937687, 1065353216, 1040666870,
        ],
    ),
    (
        3,
        [
            1059199711, 1050693708, 1065353216, 1059061760, 1047099605, 1065353216, 1031849468,
        ],
    ),
];

fn readings(seed: u64) -> [u32; 7] {
    let recipe = recipe();
    let run_config = RunConfig {
        max_ticks: recipe.max_ticks,
        eval_config: EvalConfig::default(),
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
