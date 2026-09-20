//! Pins the evaluator's per-seed [`FitnessBreakdown`] on committed atlas live
//! cells at the 500-tick horizon (issue #502).
//!
//! The rollout now keeps only the event kinds the evaluator reads and drops
//! history once read (`explorers_genesis_eval::EVALUATOR_EVENT_KINDS`). That
//! is an observer-side change — no trajectory changes — so every read must
//! come out byte-identical to the full-log evaluator. The golden values below
//! were first captured from the full-log evaluator (main at ec79277) with
//! `cargo test -p explorers-search --test evaluator_pin -- --ignored
//! print_golden --nocapture`, and are compared bit-for-bit.
//!
//! Re-captured under #503, where the evaluator's reads changed *by design*:
//! `oscillation_strength` and `coexistence_duration` read the settled window
//! `(T/2, T]` (here `(250, 500]`) instead of the post-grace series, and the
//! grace became an absolute tick count (100 — the same 100 ticks the old
//! 0.2 fraction gave at this horizon, so every gate verdict, `ticks_survived`,
//! `clustering_strength`, `turnover_score`, `trophic_balance_score` and
//! `carcass_locked_fraction` came out bit-identical to the ec79277 capture;
//! only the two windowed reads and the fitness that sums them moved).

use explorers_genesis::{EvalConfig, FailureMode, FitnessBreakdown, RunConfig, run_single};
use explorers_search::search::{decode, default_ranges};

const HORIZON: u64 = 500;
/// A handful of committed atlas live cells (indices into `atlas.json`'s
/// `cells`), chosen for spread: a consumer-guild seed (cell 6 / seed 1000),
/// high-fitness coexisting cells, a lockup and an extinction seed, and
/// low-fitness cells on the frontier's edge.
const CELLS: [usize; 6] = [0, 6, 9, 52, 54, 64];
const SEEDS: [u64; 2] = [1000, 1001];

fn atlas_units() -> Vec<Vec<f64>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../atlas.json");
    let text = std::fs::read_to_string(path).expect("committed atlas.json at the repo root");
    let atlas: serde_json::Value = serde_json::from_str(&text).expect("atlas.json parses");
    atlas["cells"]
        .as_array()
        .expect("atlas has a `cells` array")
        .iter()
        .map(|cell| {
            cell["unit"]
                .as_array()
                .expect("cell has a `unit` vector")
                .iter()
                .map(|v| v.as_f64().expect("unit coordinate is a number"))
                .collect()
        })
        .collect()
}

fn rollout(unit: &[f64], seed: u64) -> FitnessBreakdown {
    let (params, dist) = decode(unit, &default_ranges());
    let config = RunConfig {
        max_ticks: HORIZON,
        eval_config: EvalConfig::default(),
    };
    run_single(&params, &dist, &config, seed).breakdown
}

/// One pinned read, with every `f32` held as its bit pattern.
#[derive(Debug, PartialEq)]
struct Pinned {
    cell: usize,
    seed: u64,
    fitness: u32,
    failure: Option<FailureMode>,
    oscillation_strength: u32,
    clustering_strength: u32,
    coexistence_duration: u32,
    turnover_score: u32,
    trophic_balance_score: u32,
    ticks_survived: u64,
    carcass_locked_fraction: u32,
    has_decomposer_guild: bool,
    has_consumer_guild: bool,
}

fn pin(cell: usize, seed: u64, b: &FitnessBreakdown) -> Pinned {
    Pinned {
        cell,
        seed,
        fitness: b.fitness.to_bits(),
        failure: b.failure.clone(),
        oscillation_strength: b.oscillation_strength.to_bits(),
        clustering_strength: b.clustering_strength.to_bits(),
        coexistence_duration: b.coexistence_duration.to_bits(),
        turnover_score: b.turnover_score.to_bits(),
        trophic_balance_score: b.trophic_balance_score.to_bits(),
        ticks_survived: b.ticks_survived,
        carcass_locked_fraction: b.carcass_locked_fraction.to_bits(),
        has_decomposer_guild: b.has_decomposer_guild,
        has_consumer_guild: b.has_consumer_guild,
    }
}

/// Prints the golden table below in source form. Ignored: run by hand when
/// the evaluator's reads change *by design* and the pin must be re-captured.
#[test]
#[ignore]
fn print_golden() {
    let units = atlas_units();
    for &cell in &CELLS {
        for &seed in &SEEDS {
            let p = pin(cell, seed, &rollout(&units[cell], seed));
            println!(
                "    Pinned {{ cell: {}, seed: {}, fitness: {:#x}, failure: {:?}, oscillation_strength: {:#x}, clustering_strength: {:#x}, coexistence_duration: {:#x}, turnover_score: {:#x}, trophic_balance_score: {:#x}, ticks_survived: {}, carcass_locked_fraction: {:#x}, has_decomposer_guild: {}, has_consumer_guild: {} }},",
                p.cell,
                p.seed,
                p.fitness,
                p.failure,
                p.oscillation_strength,
                p.clustering_strength,
                p.coexistence_duration,
                p.turnover_score,
                p.trophic_balance_score,
                p.ticks_survived,
                p.carcass_locked_fraction,
                p.has_decomposer_guild,
                p.has_consumer_guild,
            );
        }
    }
}

fn golden() -> Vec<Pinned> {
    vec![
        Pinned {
            cell: 0,
            seed: 1000,
            fitness: 0x3ef148ee,
            failure: None,
            oscillation_strength: 0x3e98ba78,
            clustering_strength: 0x3f800000,
            coexistence_duration: 0x0,
            turnover_score: 0x3d6d9168,
            trophic_balance_score: 0x3f800000,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3bef15ac,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        },
        Pinned {
            cell: 0,
            seed: 1001,
            fitness: 0x3f3125c6,
            failure: None,
            oscillation_strength: 0x3e95759d,
            clustering_strength: 0x3f800000,
            coexistence_duration: 0x3f800000,
            turnover_score: 0x3e2c0831,
            trophic_balance_score: 0x3f800000,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3d0b6893,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        },
        Pinned {
            cell: 6,
            seed: 1000,
            fitness: 0x3f1b08c4,
            failure: None,
            oscillation_strength: 0x3e891db3,
            clustering_strength: 0x3f800000,
            coexistence_duration: 0x3f0f5c29,
            turnover_score: 0x3ed81062,
            trophic_balance_score: 0x3f47389f,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3d81bba8,
            has_decomposer_guild: false,
            has_consumer_guild: true,
        },
        Pinned {
            cell: 6,
            seed: 1001,
            fitness: 0x3f091b91,
            failure: None,
            oscillation_strength: 0x3ea8e69f,
            clustering_strength: 0x3f800000,
            coexistence_duration: 0x0,
            turnover_score: 0x3eb22d0e,
            trophic_balance_score: 0x3f800000,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3cf4c52e,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        },
        Pinned {
            cell: 9,
            seed: 1000,
            fitness: 0x3f543afe,
            failure: None,
            oscillation_strength: 0x3eb1ba75,
            clustering_strength: 0x3f800000,
            coexistence_duration: 0x3f800000,
            turnover_score: 0x3f4c49ba,
            trophic_balance_score: 0x3f800000,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3e77f7f2,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        },
        Pinned {
            cell: 9,
            seed: 1001,
            fitness: 0x3f60e5cc,
            failure: None,
            oscillation_strength: 0x3ec8f9f8,
            clustering_strength: 0x3f800000,
            coexistence_duration: 0x3f800000,
            turnover_score: 0x3f800000,
            trophic_balance_score: 0x3f800000,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3e6b834d,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        },
        Pinned {
            cell: 52,
            seed: 1000,
            fitness: 0x3f58ba06,
            failure: None,
            oscillation_strength: 0x3e6e8881,
            clustering_strength: 0x3f800000,
            coexistence_duration: 0x3f800000,
            turnover_score: 0x3f800000,
            trophic_balance_score: 0x3f800000,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3edf6c8f,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        },
        Pinned {
            cell: 52,
            seed: 1001,
            fitness: 0x0,
            failure: Some(FailureMode::NutrientLockup),
            oscillation_strength: 0x0,
            clustering_strength: 0x0,
            coexistence_duration: 0x0,
            turnover_score: 0x0,
            trophic_balance_score: 0x0,
            ticks_survived: 500,
            carcass_locked_fraction: 0x0,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        },
        Pinned {
            cell: 54,
            seed: 1000,
            fitness: 0x3f00a7e5,
            failure: None,
            oscillation_strength: 0x3e727e21,
            clustering_strength: 0x3f800000,
            coexistence_duration: 0x0,
            turnover_score: 0x3e8d4fdf,
            trophic_balance_score: 0x3f800000,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3d35856f,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        },
        Pinned {
            cell: 54,
            seed: 1001,
            fitness: 0x3db3afb9,
            failure: None,
            oscillation_strength: 0x3eb07ae3,
            clustering_strength: 0x0,
            coexistence_duration: 0x0,
            turnover_score: 0x3dc08312,
            trophic_balance_score: 0x0,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3d05d3d6,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        },
        Pinned {
            cell: 64,
            seed: 1000,
            fitness: 0x0,
            failure: Some(FailureMode::Extinction),
            oscillation_strength: 0x0,
            clustering_strength: 0x0,
            coexistence_duration: 0x0,
            turnover_score: 0x0,
            trophic_balance_score: 0x0,
            ticks_survived: 456,
            carcass_locked_fraction: 0x0,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        },
        Pinned {
            cell: 64,
            seed: 1001,
            fitness: 0x3d03126f,
            failure: None,
            oscillation_strength: 0x0,
            clustering_strength: 0x0,
            coexistence_duration: 0x0,
            turnover_score: 0x3e23d70a,
            trophic_balance_score: 0x0,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3d3cc3c0,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        },
    ]
}

#[test]
fn fitness_breakdown_is_byte_identical_on_atlas_live_cells_at_the_500_tick_horizon() {
    let units = atlas_units();
    let golden = golden();
    assert_eq!(golden.len(), CELLS.len() * SEEDS.len());
    for expected in &golden {
        let actual = pin(
            expected.cell,
            expected.seed,
            &rollout(&units[expected.cell], expected.seed),
        );
        assert_eq!(
            &actual, expected,
            "atlas cell {} seed {}",
            expected.cell, expected.seed
        );
    }
}

/// The memory criterion of #502, run by hand under `/usr/bin/time -l` on a
/// release build: an 8-seed `run_ensemble` of the `sample:55` LHS config
/// (`443-invasion-growth.md` §6.3, the densest known guild config) at the
/// working settled-community horizon `T = 2000`, under full rayon parallelism.
///
/// ```sh
/// /usr/bin/time -l cargo test --release -p explorers-search --test evaluator_pin \
///     -- --ignored horizon_ensemble_sample_55 --nocapture
/// ```
#[test]
#[ignore]
fn horizon_ensemble_sample_55() {
    use explorers_genesis::{EnsembleConfig, run_ensemble};
    use explorers_search::config_source::sampled_units;

    let ranges = default_ranges();
    let unit = &sampled_units(ranges.len())[55];
    let (params, dist) = decode(unit, &ranges);
    let config = EnsembleConfig {
        ensemble_size: 8,
        run_config: RunConfig {
            max_ticks: 2000,
            eval_config: EvalConfig::default(),
        },
    };
    let start = std::time::Instant::now();
    let result = run_ensemble(&params, &dist, &config, 1000);
    println!(
        "sample:55 × 8 seeds @ 2000 ticks in {:.1?}",
        start.elapsed()
    );
    for (i, r) in result.run_results.iter().enumerate() {
        println!(
            "  seed {i}: tick {} failure {:?} fitness {:.3} guilds C={} D={}",
            r.termination_tick,
            r.failure,
            r.fitness,
            r.breakdown.has_consumer_guild,
            r.breakdown.has_decomposer_guild
        );
    }
}
