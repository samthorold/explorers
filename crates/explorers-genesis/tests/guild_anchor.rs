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

/// `(seed, fitness, oscillation, clustering, coexistence, turnover, carcass)` as `f32` bits. First pinned on `main` at 9b51a01; re-captured
/// under #503 (the evaluator reads oscillation and coexistence off the
/// settled window `(T/2, T]`, and the grace became an absolute tick count) —
/// only fitness, oscillation and coexistence moved, and the guild read still
/// touches none of them. Re-captured under #486 (trophic balance scored per
/// agent against the role read, not on cluster means) — only fitness and
/// trophic balance moved. Re-pinned under #494 on the regenerated
/// settled-horizon atlas's recipe: `recipe.json` itself was replaced, so every
/// value is new (a different world, not a changed read). Re-pinned under #600:
/// need-gated consumption changes the physics the recipe runs under, so every
/// reading but the saturated clustering and trophic scores moved (again a
/// different world, not a changed read). Re-pinned under #601: `recipe.json`
/// omits founder aggregation, so it now founds at the aggregated default
/// (`0.8`, one tight patch per cluster) instead of the well-mixed scatter —
/// a different founding, not a changed read. Re-pinned under #602: trophic
/// balance left fitness, so the column is gone and only fitness moved (the
/// mean of four criteria, not a weighted five). Re-pinned under #603:
/// satiation is co-limited with nutrient, so nutrient-starved consumers keep
/// feeding — a physics change, not a changed read (seeds 1 and 2 moved; the
/// carcass-locked fraction fell on both). Re-pinned under #604: consumers
/// spare living targets that resemble them (recognition) — a physics change,
/// not a changed read (every seed moved; seed 1 lost its clustering and
/// seed 2 its coexistence under a recipe found for the kin-blind physics).
/// Re-pinned under #623: satiation is the surplus above the retention
/// buffer, read before growth, at the default sensitivity 33 — a physics
/// change, not a changed read (every seed moved; seed 3 now reads zero on
/// every column, its world failing before it scores).
/// Re-pinned under #652: a consumer retains its nutrient ratio × the energy
/// a bite gains it, not its whole-body demand × that energy — a physics
/// change, not a changed read (seeds 1 and 2 moved; seed 1 regained its
/// clustering; seed 3 still reads zero).
/// Re-capture with
/// `cargo test -p explorers-genesis --test guild_anchor -- --ignored print_golden --nocapture`.
const GOLDEN: [(u64, [u32; 6]); 3] = [
    (
        1,
        [
            1051957666, 1044994138, 1065353216, 0, 1045656764, 1030349268,
        ],
    ),
    (
        2,
        [
            1052341607, 1050015326, 1065353216, 0, 1042267767, 1032447002,
        ],
    ),
    (3, [0, 0, 0, 0, 0, 0]),
];

fn readings(seed: u64) -> [u32; 6] {
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
