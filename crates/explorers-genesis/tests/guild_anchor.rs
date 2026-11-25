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
/// Re-pinned under #663 on the recipe of the atlas searched under ratio
/// retention with `b` in the box: `recipe.json` itself was replaced (atlas:39,
/// cell [5, 19, 7], `b` ≈ 0.67), so every value is new — a different world,
/// not a changed read. Seeds 1 and 3 lock up (ticks 1600 and 1800) and read
/// zero; seed 2 reaches the horizon and carries the pin.
/// Re-pinned under #666: retention adds the consumer's nutrient deficit to
/// its ratio × energy gained, so light-fed mixotrophs on poor ground keep
/// the drained nutrient their surplus waits on — a physics change, not a
/// changed read. Seed 1 no longer locks up and now reaches the horizon;
/// seed 2 moved (it now scores coexistence); seed 3 still reads zero.
/// Re-pinned under #677 on the recipe of the atlas searched under the
/// deficit rule with `c_AH` in the box: `recipe.json` itself was replaced
/// (atlas:90, cell [9, 19, 6], `b` ≈ 0.72, `c_AH` ≈ 0.139), so every value is
/// new — a different world, not a changed read. Seeds 1 and 3 reach the
/// horizon; seed 2 locks up (tick 1650) and reads zero.
/// Re-pinned under #684: the surplus gate is removed (expression ungated)
/// and recognition restraint is 1 — a physics change, not a changed read.
/// The recipe was found under the gate. Seed 1 moved (fitness 0.108 → 0.177);
/// seed 2 no longer reads zero (fitness 0.104), and seed 3 now does (was 0.320).
/// Re-pinned under #687 on the recipe of #719's atlas, searched on genesis's
/// box at the fixed leaching rate: `recipe.json` itself was replaced
/// (atlas:31, cell [5, 19, 1], `λ = 0.0025`, `c_AH = 0`), so every value is
/// new — a different world, not a changed read. All three seeds now score,
/// at fitness 0.744, 0.725 and 0.720 (refined 0.704 at n = 32), with full
/// clustering and coexistence 1.0, 0.99 and 1.0.
/// Re-pinned under #780: foraging movement follows what a resource releases
/// (the cue rule), and light competition and nutrient uptake sum in stable id
/// order, as does carcass leaching — a physics change, not a changed read.
/// The summing order alone moved only last bits; the movement rule moved
/// every seed, all still
/// scoring with full clustering: fitness 0.718, 0.764 and 0.764, coexistence
/// 1.0, 1.0 and 0.99.
/// Re-pinned under #901: no draw takes a store below zero, so the last share
/// of a cell's uptake split, or of an exhausted carcass's nutrient, is capped
/// at what the store still holds. That is a physics change of a few ulps,
/// where a split used to overdraw, not a changed read. The ulps grow over the
/// run. Seed 2 moved only in the last bits of oscillation. Seed 1's fitness
/// went from 0.718 to 0.706, and seed 3's from 0.764 to 0.783, its
/// coexistence from 0.99 to 1.0. All three seeds keep full clustering.
/// Re-pinned under #859: every living agent sheds tissue each tick into
/// necromass piles that consumers drain (tissue turnover) — a physics change,
/// not a changed read. Seed 1's fitness fell from 0.706 to 0.388 (clustering
/// 1.0 to 0.571, coexistence 1.0 to 0.14); seed 2's rose from 0.764 to 0.783
/// and seed 3's moved from 0.783 to 0.776. The carcass-locked fraction rose
/// on every seed (0.129, 0.092, 0.114 to 0.198, 0.166, 0.223).
/// Re-pinned under #860: hyphal uptake in every world, so still heterotrophs
/// and mixotrophs draw pool nutrient through their heterotrophy — a physics
/// change, not a changed read. Seed 1's fitness rose from 0.388 to 0.704
/// (clustering 0.571 to 0.944, coexistence 0.14 to 0.76); seed 2 held at
/// 0.782 (coexistence 0.91 to 0.93); seed 3's fell from 0.776 to 0.534
/// (clustering 1.0 to 0.798, coexistence 0.99 to 0.09). The carcass-locked
/// fraction rose on seed 1 (0.198 to 0.266) and barely moved on seeds 2 and 3.
/// Re-pinned under #861: reproductive isolation follows ancestry in every
/// world, so the recipe's two founding clusters (producers and heterotrophs)
/// never cross and mates are chosen by marker distance, not trait distance —
/// a physics change, not a changed read. Fitness fell on every seed (0.704,
/// 0.782, 0.534 to 0.579, 0.727, 0.497); seed 1's coexistence fell from 0.76
/// to 0, seed 2's rose from 0.93 to 0.98, seed 3's held at 0.09. The
/// carcass-locked fraction fell on every seed (0.266, 0.164, 0.221 to 0.170,
/// 0.134, 0.109).
/// Re-pinned under #862: the base rate is `b · S` in every world, at the
/// rate that leaves a producer of the established structure 6.937 paying what
/// it did — a physics change, not a changed read. Every seed now reads zero
/// on every column: the recipe, found under the per-agent constant, fails
/// before it scores (seed 1 by nutrient lockup at tick 1950; on all three the
/// agents of autotrophy 0.3 or less fall to a handful or none by ticks
/// 1800–1900, and carcasses pile up). So until
/// the recipe is found again under the full physics (after #870), this
/// anchor pins only that the guild read does not turn a failed world into a
/// scoring one.
/// Since #865 living tissue resists every drain at the world's own `ρ = 1`.
/// The recipe then still reads zero on every column, but its three seeds
/// took 523 s in the workspace test run, against 89 s at `ρ = 0`. So the
/// anchor runs the recipe in the controlled variant without resistance
/// (`ρ = 0`), the world it ran before #865. Re-pinning it under the full physics waits on the
/// recipe being found again (#870).
/// Re-capture with
/// `cargo test -p explorers-genesis --test guild_anchor -- --ignored print_golden --nocapture`.
const GOLDEN: [(u64, [u32; 6]); 3] = [
    (1, [0, 0, 0, 0, 0, 0]),
    (2, [0, 0, 0, 0, 0, 0]),
    (3, [0, 0, 0, 0, 0, 0]),
];

fn readings(seed: u64) -> [u32; 6] {
    let mut recipe = recipe();
    recipe.parameters.living_resistance = 0.0;
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
