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
//!
//! Re-captured under #599, an evaluator read changed by design: every
//! agent's trophic role is read from its recent realised income, not its
//! trait vector, for both the guild read and `trophic_balance_score` (an
//! agent with no income yet has no role and is left out). No trajectory
//! changed: every gate verdict and `ticks_survived` is bit-identical. On
//! every live seed `trophic_balance_score` went to 1 — the trait-tagged
//! "heterotrophs" whose energy it counted against producers live on light
//! (#596) — and the fitness that sums it moved with it. Two guild flags fell:
//! cell 27 / seed 1001's decomposer guild and cell 59 / seed 1001's consumer
//! guild were trait-read guilds of agents living on light.
//!
//! Re-captured under #600, a stepper change: consumption is need-gated — a
//! consumer expresses `1 / (1 + satiation_sensitivity × s)` of its
//! heterotrophic capability, `s` its reserve in ticks of its own maintenance.
//! Every trajectory that feeds moved. Four verdicts moved with them: cell 11's
//! lockup stop moved from seed 1000 to seed 1001 (still at tick 450), and
//! cell 39 / seed 1001 (monoculture), cell 66 / seed 1000 (generalist
//! dominance) and cell 71 / seed 1000 (extinction at 484) now reach the
//! horizon ungated. The `CELLS` spread below was chosen under the old physics;
//! re-choosing it belongs with the atlas regeneration (#607).
//!
//! Re-captured under #601, a founding change: the decoded worlds now found at
//! the aggregated default (`founder_aggregation = 0.8`, one tight patch per
//! cluster) instead of the well-mixed scatter. Every trajectory moved. Three
//! verdicts moved with them: cell 11 / seed 1000 now locks up at tick 350
//! (was live to the horizon) and cell 11 / seed 1001 locks up at 400 (was
//! 450); cell 39 / seed 1001 is now a monoculture on both seeds (was live).
//! The atlas itself was searched under well-mixed founding; re-choosing
//! `CELLS` still belongs with the atlas regeneration (#607).
//!
//! Re-captured under #602, an evaluator read changed by design: trophic
//! balance left fitness, which is now the mean of the four remaining
//! criteria, and the pin carries the heterotroph shares (energy and income,
//! consumer and decomposer) in its place. No trajectory changed: every gate
//! verdict, `ticks_survived`, criterion and guild flag is bit-identical; only
//! `fitness` moved, on every live seed. The shares read `None` on the lockup
//! early stops, which have no terminal verdict to read them on.
//!
//! Re-captured under #603, a stepper change: satiation is co-limited — the
//! lesser of energy satiation and nutrient satiation (free nutrient against
//! what the reserve would bind into structure) — so a consumer short of free
//! nutrient feeds near full capability however full its reserve. Every
//! trajectory that feeds moved. One cell's verdict moved with it: cell 71 is
//! extinct again on both seeds (tick 43 / seed 1000, tick 48 / seed 1001; it
//! was live to the horizon under the energy-only gate, and dies at tick 13 on
//! seed 1000 ungated). It founds four agents whose free nutrient growth binds
//! almost as fast as they take it up, so the nutrient side keeps the gate
//! open. Every other verdict, `ticks_survived` and guild flag is unchanged.

use std::path::Path;

use explorers_genesis::{
    EvalConfig, FailureMode, FitnessBreakdown, InitialDistribution, RunConfig, WorldParameters,
    run_single,
};
use explorers_search::search::{decode, default_ranges};
use explorers_search::sweep::{AtlasUnits, read_atlas_units};

const HORIZON: u64 = 500;
/// A handful of committed atlas live cells (indices into `atlas.json`'s
/// `cells`), chosen for spread at `HORIZON` from a scan of every cell (#494):
/// a lockup early stop (cell 11 / seed 1000), a decomposer-guild seed on a
/// high-fitness cell (27 / 1001), a monoculture on both seeds (39), a
/// consumer-guild seed (59 / 1001), a generalist-dominance gate (66 / 1000),
/// an extinction (71 / 1000) and the top-fitness seed (76 / 1001). The two
/// guild seeds were guilds by the trait read; by income (#599) they hold none.
/// Under need-gated consumption (#600) four of these verdicts moved (see the
/// module note), so the spread is now narrower than described; under
/// co-limited satiation (#603) cell 71 is an extinction again.
const CELLS: [usize; 7] = [11, 27, 39, 59, 66, 71, 76];
const SEEDS: [u64; 2] = [1000, 1001];

/// The committed atlas's live cells, each decoded over the atlas's own search
/// box (#559) — the full box for this atlas, which records none.
fn atlas_units() -> AtlasUnits {
    read_atlas_units(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../atlas.json"
    )))
}

fn rollout((params, dist): &(WorldParameters, InitialDistribution), seed: u64) -> FitnessBreakdown {
    let config = RunConfig {
        max_ticks: HORIZON,
        eval_config: EvalConfig::default(),
        early_stop_crosscheck_fraction: 0.0,
    };
    run_single(params, dist, &config, seed).breakdown
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
    ticks_survived: u64,
    carcass_locked_fraction: u32,
    has_decomposer_guild: bool,
    has_consumer_guild: bool,
    /// Energy consumer, energy decomposer, income consumer, income
    /// decomposer (#602).
    heterotroph_shares: Option<[u32; 4]>,
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
        ticks_survived: b.ticks_survived,
        carcass_locked_fraction: b.carcass_locked_fraction.to_bits(),
        has_decomposer_guild: b.has_decomposer_guild,
        has_consumer_guild: b.has_consumer_guild,
        heterotroph_shares: b.heterotroph_shares.map(|s| {
            [
                s.energy.consumer.to_bits(),
                s.energy.decomposer.to_bits(),
                s.income.consumer.to_bits(),
                s.income.decomposer.to_bits(),
            ]
        }),
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
            let p = pin(cell, seed, &rollout(&units.decode(cell), seed));
            println!(
                "    Pinned {{ cell: {}, seed: {}, fitness: {:#x}, failure: {:?}, oscillation_strength: {:#x}, clustering_strength: {:#x}, coexistence_duration: {:#x}, turnover_score: {:#x}, ticks_survived: {}, carcass_locked_fraction: {:#x}, has_decomposer_guild: {}, has_consumer_guild: {}, heterotroph_shares: {:?} }},",
                p.cell,
                p.seed,
                p.fitness,
                p.failure,
                p.oscillation_strength,
                p.clustering_strength,
                p.coexistence_duration,
                p.turnover_score,
                p.ticks_survived,
                p.carcass_locked_fraction,
                p.has_decomposer_guild,
                p.has_consumer_guild,
                p.heterotroph_shares,
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
            ticks_survived: 350,
            carcass_locked_fraction: 0x0,
            has_decomposer_guild: false,
            has_consumer_guild: false,
            heterotroph_shares: None,
        },
        Pinned {
            cell: 11,
            seed: 1001,
            fitness: 0x0,
            failure: Some(FailureMode::NutrientLockup),
            oscillation_strength: 0x0,
            clustering_strength: 0x0,
            coexistence_duration: 0x0,
            turnover_score: 0x0,
            ticks_survived: 400,
            carcass_locked_fraction: 0x0,
            has_decomposer_guild: false,
            has_consumer_guild: false,
            heterotroph_shares: None,
        },
        Pinned {
            cell: 27,
            seed: 1000,
            fitness: 0x3eb50e5a,
            failure: None,
            oscillation_strength: 0x3e8b8532,
            clustering_strength: 0x3f800000,
            coexistence_duration: 0x3d23d70a,
            turnover_score: 0x3dd0e560,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3d83c40d,
            has_decomposer_guild: false,
            has_consumer_guild: false,
            heterotroph_shares: Some([0, 0, 0, 0]),
        },
        Pinned {
            cell: 27,
            seed: 1001,
            fitness: 0x3f4d2379,
            failure: None,
            oscillation_strength: 0x3eb2d629,
            clustering_strength: 0x3f800000,
            coexistence_duration: 0x3f800000,
            turnover_score: 0x3f5b22d1,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3ea6fa7b,
            has_decomposer_guild: false,
            has_consumer_guild: false,
            heterotroph_shares: Some([0, 0, 0, 0]),
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
            ticks_survived: 500,
            carcass_locked_fraction: 0x0,
            has_decomposer_guild: false,
            has_consumer_guild: false,
            heterotroph_shares: Some([0, 0, 0, 0]),
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
            ticks_survived: 500,
            carcass_locked_fraction: 0x0,
            has_decomposer_guild: false,
            has_consumer_guild: false,
            heterotroph_shares: Some([0, 0, 0, 0]),
        },
        Pinned {
            cell: 59,
            seed: 1000,
            fitness: 0x3bd4fdf4,
            failure: None,
            oscillation_strength: 0x0,
            clustering_strength: 0x0,
            coexistence_duration: 0x0,
            turnover_score: 0x3cd4fdf4,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3c5b3cba,
            has_decomposer_guild: false,
            has_consumer_guild: false,
            heterotroph_shares: Some([0, 0, 0, 0]),
        },
        Pinned {
            cell: 59,
            seed: 1001,
            fitness: 0x3d7f589f,
            failure: None,
            oscillation_strength: 0x3e2b60d0,
            clustering_strength: 0x0,
            coexistence_duration: 0x0,
            turnover_score: 0x3da7ef9e,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3cc89d0e,
            has_decomposer_guild: false,
            has_consumer_guild: false,
            heterotroph_shares: Some([0, 0, 0, 0]),
        },
        Pinned {
            cell: 66,
            seed: 1000,
            fitness: 0x3eac8108,
            failure: None,
            oscillation_strength: 0x3e7b958d,
            clustering_strength: 0x3f800000,
            coexistence_duration: 0x0,
            turnover_score: 0x3dd0e560,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3c1f11a4,
            has_decomposer_guild: false,
            has_consumer_guild: false,
            heterotroph_shares: Some([0, 0, 0, 0]),
        },
        Pinned {
            cell: 66,
            seed: 1001,
            fitness: 0x3efcaf24,
            failure: None,
            oscillation_strength: 0x3f1ebe8a,
            clustering_strength: 0x3f800000,
            coexistence_duration: 0x3e23d70a,
            turnover_score: 0x3e46a7f0,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3d8594c9,
            has_decomposer_guild: false,
            has_consumer_guild: false,
            heterotroph_shares: Some([0, 0, 0, 0]),
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
            ticks_survived: 43,
            carcass_locked_fraction: 0x0,
            has_decomposer_guild: false,
            has_consumer_guild: false,
            heterotroph_shares: None,
        },
        Pinned {
            cell: 71,
            seed: 1001,
            fitness: 0x0,
            failure: Some(FailureMode::Extinction),
            oscillation_strength: 0x0,
            clustering_strength: 0x0,
            coexistence_duration: 0x0,
            turnover_score: 0x0,
            ticks_survived: 48,
            carcass_locked_fraction: 0x0,
            has_decomposer_guild: false,
            has_consumer_guild: false,
            heterotroph_shares: None,
        },
        Pinned {
            cell: 76,
            seed: 1000,
            fitness: 0x3e91a9fc,
            failure: None,
            oscillation_strength: 0x0,
            clustering_strength: 0x3f800000,
            coexistence_duration: 0x0,
            turnover_score: 0x3e0d4fdf,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3cf1f611,
            has_decomposer_guild: false,
            has_consumer_guild: false,
            heterotroph_shares: Some([990218257, 0, 956928237, 0]),
        },
        Pinned {
            cell: 76,
            seed: 1001,
            fitness: 0x3f28b1f2,
            failure: None,
            oscillation_strength: 0x3e8e956f,
            clustering_strength: 0x3f0eb043,
            coexistence_duration: 0x3f4ccccd,
            turnover_score: 0x3f800000,
            ticks_survived: 500,
            carcass_locked_fraction: 0x3e387081,
            has_decomposer_guild: false,
            has_consumer_guild: false,
            heterotroph_shares: Some([0, 999985983, 0, 999987341]),
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
            &rollout(&units.decode(expected.cell), expected.seed),
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
