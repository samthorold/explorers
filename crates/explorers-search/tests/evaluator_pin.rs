//! Pins the evaluator's per-seed [`FitnessBreakdown`] on committed atlas live
//! cells at the 500-tick horizon (issue #502). The horizon is pinned here
//! explicitly (`HORIZON`), not read from `SearchConfig::default()` — the
//! search default moved to 2000 under #507, and these goldens document the
//! evaluator's reads on committed atlas cells at 500 ticks, not the search's
//! horizon.
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
//!
//! Re-captured once more under #506, where the dead-pool gates fire
//! incrementally and stop the rollout where it dies: the one pinned seed
//! the lockup gate catches (cell 52 / seed 1001) now stops at tick 450 — the
//! window boundary at which its series-so-far first reads locked — instead
//! of running to 500. Its mode is unchanged and every seed that reaches the
//! horizon is bit-identical; only that `ticks_survived` moved.
//!
//! Re-captured under #540, a stepper change: an agent whose metabolic charge
//! is capped now dies that tick, instead of surviving on the sub-epsilon
//! reserve its own feeding credits it in the drain pass. Two trajectories
//! moved (cell 6 / seed 1000 and cell 52 / seed 1000); every gate verdict,
//! `ticks_survived` and guild flag is unchanged.
//!
//! Re-captured under #486, an evaluator read changed by design:
//! `trophic_balance_score` buckets each living agent by the topology's
//! trophic-role read instead of each DBSCAN cluster by its mean, and counts
//! noise-labelled agents. Only `trophic_balance_score` and the fitness that
//! sums it moved (the all-noise seeds cell 54 / seed 1001 and cell 64 / seed
//! 1001 went from 0 to 1); every gate verdict, `ticks_survived` and guild
//! flag is unchanged.
//!
//! Re-pinned under #494 on the regenerated settled-horizon atlas: the
//! committed `atlas.json` was replaced, so the cell indices and every golden
//! below are new, chosen afresh for the same spread of verdicts (see `CELLS`).

use explorers_genesis::{EvalConfig, FailureMode, FitnessBreakdown, RunConfig, run_single};
use explorers_search::search::{decode, default_ranges};

const HORIZON: u64 = 500;
/// A handful of committed atlas live cells (indices into `atlas.json`'s
/// `cells`), chosen for spread at `HORIZON` from a scan of every cell (#494):
/// a lockup early stop (cell 11 / seed 1000), a decomposer-guild seed on a
/// high-fitness cell (27 / 1001), a monoculture on both seeds (39), a
/// consumer-guild seed (59 / 1001), a generalist-dominance gate (66 / 1000),
/// an extinction (71 / 1000) and the top-fitness seed (76 / 1001).
const CELLS: [usize; 7] = [11, 27, 39, 59, 66, 71, 76];
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
        early_stop_crosscheck_fraction: 0.0,
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
            cell: 11,
            seed: 1000,
            fitness: 0x0,
            failure: Some(FailureMode::NutrientLockup),
            oscillation_strength: 0x0,
            clustering_strength: 0x0,
            coexistence_duration: 0x0,
            turnover_score: 0x0,
            trophic_balance_score: 0x0,
            ticks_survived: 450,
            carcass_locked_fraction: 0x0,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        },
        Pinned {
            cell: 11,
            seed: 1001,
            fitness: 0x3f30f9bf,
            failure: None,
            oscillation_strength: 0x3e835b12,
            clustering_strength: 0x3f800000,
            coexistence_duration: 0x3e4ccccd,
            turnover_score: 0x3f800000,
            trophic_balance_score: 0x3f800000,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3e968f09,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        },
        Pinned {
            cell: 27,
            seed: 1000,
            fitness: 0x3f037bbf,
            failure: None,
            oscillation_strength: 0x3eb4f8a9,
            clustering_strength: 0x3f800000,
            coexistence_duration: 0x3eb851ec,
            turnover_score: 0x3e178d50,
            trophic_balance_score: 0x3f34e21b,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3d9174a1,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        },
        Pinned {
            cell: 27,
            seed: 1001,
            fitness: 0x3f42562a,
            failure: None,
            oscillation_strength: 0x3ef3adcf,
            clustering_strength: 0x3f800000,
            coexistence_duration: 0x3f800000,
            turnover_score: 0x3f374bc7,
            trophic_balance_score: 0x3f1a8c26,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3e28dfc9,
            has_decomposer_guild: true,
            has_consumer_guild: false,
        },
        Pinned {
            cell: 39,
            seed: 1000,
            fitness: 0x0,
            failure: Some(FailureMode::Monoculture),
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
            cell: 39,
            seed: 1001,
            fitness: 0x0,
            failure: Some(FailureMode::Monoculture),
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
            cell: 59,
            seed: 1000,
            fitness: 0x3f040060,
            failure: None,
            oscillation_strength: 0x3ea3df02,
            clustering_strength: 0x3f800000,
            coexistence_duration: 0x3e23d70a,
            turnover_score: 0x3ea8f5c3,
            trophic_balance_score: 0x3f44a1bc,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3d198c9a,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        },
        Pinned {
            cell: 59,
            seed: 1001,
            fitness: 0x3ee7037e,
            failure: None,
            oscillation_strength: 0x3ea77c1c,
            clustering_strength: 0x3f800000,
            coexistence_duration: 0x0,
            turnover_score: 0x3dac0831,
            trophic_balance_score: 0x3f5849a8,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3d02247d,
            has_decomposer_guild: false,
            has_consumer_guild: true,
        },
        Pinned {
            cell: 66,
            seed: 1000,
            fitness: 0x0,
            failure: Some(FailureMode::GeneralistDominance),
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
            cell: 66,
            seed: 1001,
            fitness: 0x3f0ed3d5,
            failure: None,
            oscillation_strength: 0x3e9392f2,
            clustering_strength: 0x3f333333,
            coexistence_duration: 0x3f6b851f,
            turnover_score: 0x3e73b646,
            trophic_balance_score: 0x3f24b3ce,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3d67632a,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        },
        Pinned {
            cell: 71,
            seed: 1000,
            fitness: 0x0,
            failure: Some(FailureMode::Extinction),
            oscillation_strength: 0x0,
            clustering_strength: 0x0,
            coexistence_duration: 0x0,
            turnover_score: 0x0,
            trophic_balance_score: 0x0,
            ticks_survived: 484,
            carcass_locked_fraction: 0x0,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        },
        Pinned {
            cell: 71,
            seed: 1001,
            fitness: 0x3eadd641,
            failure: None,
            oscillation_strength: 0x3ee21cd5,
            clustering_strength: 0x0,
            coexistence_duration: 0x0,
            turnover_score: 0x3e83126f,
            trophic_balance_score: 0x3f800000,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3c4bc3ad,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        },
        Pinned {
            cell: 76,
            seed: 1000,
            fitness: 0x3e628241,
            failure: None,
            oscillation_strength: 0x0,
            clustering_strength: 0x0,
            coexistence_duration: 0x0,
            turnover_score: 0x3dd91687,
            trophic_balance_score: 0x3f800000,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3caacc83,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        },
        Pinned {
            cell: 76,
            seed: 1001,
            fitness: 0x3f5d56ca,
            failure: None,
            oscillation_strength: 0x3ee2d489,
            clustering_strength: 0x3f800000,
            coexistence_duration: 0x3f6147ae,
            turnover_score: 0x3f800000,
            trophic_balance_score: 0x3f800000,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3d4d850d,
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
            early_stop_crosscheck_fraction: 0.0,
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
