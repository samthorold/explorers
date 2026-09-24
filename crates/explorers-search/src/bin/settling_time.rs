//! Settling-time instrument (issue #504): measures the two transient ticks
//! `docs/system-design/genesis-search.md` defines as ecological constants —
//! the **grace** (founder provisioning transient) and the **horizon `T`**
//! (succession settling time) — across the atlas's live cells and a
//! low-discrepancy sample of the search box. A follow-up issue (#505) drives
//! the sweep and records the values; this bin only produces the measurement.
//!
//! ## What it measures
//!
//! Each (config, seed) runs to the horizon with nothing injected, through the
//! genesis step loop exactly as `explorers_genesis::run_single` does (retained
//! event kinds, per-step compaction, the evaluator's early stops and terminal
//! classification). The instrument observes two per-tick *series*, never the
//! event log:
//!
//! - **provisioning-transient tick** — the first tick at which the living
//!   free-energy stock (`World::free_energy`) exceeds its tick-0 value; `null`
//!   if it never does. The grace is an upper quantile of this over live runs.
//! - **producer-settling tick** — the last tick at which the producer count
//!   (agents with `photosynthetic_absorption >= heterotrophy`, the evaluator's
//!   own producer rule) lies outside the band it holds over the run's final
//!   stretch: `±20 %` of the mean producer count over the final 500 ticks
//!   (`BAND`; `--tail-ticks` overrides the tail). `0` when the series never
//!   leaves the band; `null` when the run did not reach the horizon or the tail
//!   holds no producers. `T/2` is placed above an upper quantile of this.
//!
//! Every seed record also carries the termination tick and the evaluator's
//! terminal mode, so dead runs are separable from settled ones; the summary
//! quantiles (50th / 90th / 95th / max, nearest-rank) read **live runs only**
//! (mode `none` and reached the horizon), split atlas vs sample.
//!
//! A (config, seed) run carries two wall-clock budgets (#523). The
//! **simulation budget** (`--run-timeout-secs`, default 300) bounds the step
//! loop: a knife-edge world that costs seconds per tick stops where it is and
//! is recorded as mode `timeout`. The **evaluation budget**
//! (`--eval-timeout-secs`, default 300) bounds the terminal evaluation of a
//! rollout that reached the horizon: a dense terminal roster whose clustering
//! overruns it is recorded as mode `eval_timeout` — no verdict, never a
//! failure classification. Neither is live; each shows under its own label in
//! the summary's mode breakdown instead of stalling the sweep. Both guards
//! only read a clock, so runs that finish inside them stay byte-identical.
//!
//! ## Resumable by construction
//!
//! Results are appended one JSON line per config to `--out` (default
//! `target/settling-time.jsonl`). On start, configs already present in the file
//! are skipped; `--limit N` runs at most `N` further configs and exits. Config
//! order (atlas cells by index, then the seed-421 LHS sample by index) and the
//! seed block (`SEED_BASE + 0..8`) are fixed, and configs run sequentially with
//! the seeds as the parallel unit, so a sweep driven as a loop of short calls
//! produces a file byte-identical to a single uninterrupted run. Every row
//! carries the horizon and the band it was measured with, and the summary
//! (printed at the end of every invocation over *all* rows present, or alone
//! with `--summary`) restates the procedure — so the recorded values travel
//! with their definition.
//!
//! ## What it does NOT do
//!
//! No change to the stepper, evaluator, or search. Not a CI gate — run it
//! explicitly.
//!
//! ## Running
//!
//! Full sweep (282 configs × 8 seeds × 3000 ticks), as resumable chunks:
//!   cargo build --release -p explorers-search --bin settling_time
//!   ./target/release/settling_time --limit 10     # repeat until it runs 0
//!   ./target/release/settling_time --summary
//!
//! Smoke run (3 atlas cells at 600 ticks, a few seconds):
//!   ./target/release/settling_time --configs atlas:0,atlas:1,atlas:2 \
//!       --horizon 600 --out target/settling-smoke.jsonl

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use rayon::prelude::*;

use explorers_genesis::{EvalConfig, FailureMode};
use explorers_genesis_eval::{
    EVALUATOR_EVENT_KINDS, RolloutObservations, early_stop, sustainable_stock,
};
use explorers_search::config_source::{ConfigSource, parse_selector, sampled_units};
use explorers_search::search::{decode, default_ranges};
use explorers_search::sweep::{
    DEFAULT_EVAL_TIMEOUT_SECS, EVAL_TIMEOUT_FLAG, EVAL_TIMEOUT_MODE, TIMEOUT_MODE, append_row,
    done_configs, evaluate_within_budget, plan_tasks, read_atlas_units, read_rows,
};
use explorers_sim::{InitialDistribution, World, WorldParameters};

/// First tick at which the living free-energy stock exceeds its tick-0 value.
/// `free_energy[t]` is the stock at tick `t` (index 0 is the founder
/// provisioning, before any step). `None` if it never does.
fn provisioning_transient_tick(free_energy: &[f32]) -> Option<u64> {
    let founder = *free_energy.first()?;
    free_energy
        .iter()
        .position(|&e| e > founder)
        .map(|t| t as u64)
}

/// The producer band: `±fraction` of the mean producer count over the run's
/// final `tail_ticks` ticks.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct Band {
    tail_ticks: usize,
    fraction: f64,
}

const BAND: Band = Band {
    tail_ticks: 500,
    fraction: 0.2,
};

/// The producer-settling tick: the last tick at which the producer count is
/// outside the band it holds over the final stretch (`band`). `producers[t]`
/// is the count at tick `t`. `Some(0)` when the series never leaves the band;
/// `None` when the series is shorter than the tail (a run that did not reach
/// the horizon has no final stretch) or the tail mean is zero (no producers
/// to settle).
fn producer_settling_tick(producers: &[usize], band: Band) -> Option<u64> {
    let mean = tail_mean(producers, band).filter(|&m| m > 0.0)?;
    let (lo, hi) = (mean * (1.0 - band.fraction), mean * (1.0 + band.fraction));
    let outside = |&n: &usize| (n as f64) < lo || (n as f64) > hi;
    Some(producers.iter().rposition(outside).map_or(0, |t| t as u64))
}

/// Mean of the final `band.tail_ticks` entries; `None` when the series is
/// shorter than the tail.
fn tail_mean(producers: &[usize], band: Band) -> Option<f64> {
    if band.tail_ticks == 0 || producers.len() < band.tail_ticks {
        return None;
    }
    let tail = &producers[producers.len() - band.tail_ticks..];
    Some(tail.iter().sum::<usize>() as f64 / band.tail_ticks as f64)
}

/// The evaluator's terminal failure mode, as a stable label.
fn mode_label(failure: &Option<FailureMode>) -> &'static str {
    match failure {
        None => "none",
        Some(FailureMode::Extinction) => "extinction",
        Some(FailureMode::PopulationExplosion) => "explosion",
        Some(FailureMode::EnergyDeath) => "energy-death",
        Some(FailureMode::Monoculture) => "monoculture",
        Some(FailureMode::GeneralistDominance) => "generalist-dominance",
        Some(FailureMode::NutrientLockup) => "nutrient-lockup",
    }
}

/// One (config × seed) run.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct SeedRecord {
    seed: u64,
    /// The world's tick when the rollout stopped: the horizon, where a
    /// terminal gate fired (population zero or over the explosion cap, or a
    /// dead-pool gate on the series-so-far), or where the wall-clock budget
    /// ran out.
    termination_tick: u64,
    /// The evaluator's terminal classification (`evaluate_from_log`),
    /// `"timeout"` when the step loop exceeded the simulation budget
    /// (`--run-timeout-secs`), or `"eval_timeout"` when the terminal
    /// evaluation exceeded the evaluation budget (`--eval-timeout-secs`) —
    /// recorded, not evaluated, so the summary's mode breakdown shows them
    /// rather than silently missing them.
    mode: String,
    /// `mode == "none"` and the run reached the horizon: the runs the summary
    /// quantiles read.
    live: bool,
    founder_free_energy: f32,
    provisioning_transient_tick: Option<u64>,
    producer_settling_tick: Option<u64>,
    /// Mean producer count over the band's tail (`None` when the run did not
    /// reach the horizon).
    tail_mean_producers: Option<f64>,
    founder_producers: usize,
    peak_producers: usize,
    terminal_producers: usize,
}

fn producer_count(world: &World) -> usize {
    world
        .agents()
        .iter()
        .filter(|a| a.traits.photosynthetic_absorption >= a.traits.heterotrophy)
        .count()
}

/// Drive one (config, seed) through the genesis step loop exactly as
/// `explorers_genesis::run_single` does (retained event kinds, per-step
/// compaction, same early stops, same `evaluate_from_log`), reading the two
/// per-tick series the transients are defined on: the living free-energy
/// stock and the producer count, both indexed by tick from tick 0.
///
/// A rollout that exceeds `run_timeout` of wall clock stops where it is and
/// reads as mode `"timeout"` (not live); one that reaches the horizon but
/// whose evaluation exceeds `eval_timeout` reads as `"eval_timeout"` (not
/// live). The guards only compare a clock against the budgets, so a run that
/// exhausts neither is byte-identical to one run without budgets.
fn run_seed(
    params: &WorldParameters,
    dist: &InitialDistribution,
    seed: u64,
    horizon: u64,
    band: Band,
    run_timeout: Duration,
    eval_timeout: Duration,
) -> SeedRecord {
    let eval_config = EvalConfig::default();
    let stock = sustainable_stock(params);
    let started = Instant::now();
    let mut timed_out = false;
    let mut stopped: Option<FailureMode> = None;
    let mut world = World::new(params.clone(), dist.clone(), seed);
    world.retain_event_kinds(EVALUATOR_EVENT_KINDS);
    let mut observations = RolloutObservations::with_capacity(horizon as usize);
    let mut free_energy: Vec<f32> = Vec::with_capacity(horizon as usize + 1);
    let mut producers: Vec<usize> = Vec::with_capacity(horizon as usize + 1);
    free_energy.push(world.free_energy());
    producers.push(producer_count(&world));
    for _ in 0..horizon {
        world.step();
        observations.observe(&world, eval_config.coexistence_sample_interval);
        world.compact_event_log_before(observations.consumed_events());
        free_energy.push(world.free_energy());
        producers.push(producer_count(&world));
        // The evaluator's incremental terminal check — extinction, explosion,
        // and the dead-pool gates on the series-so-far (#506) — stops the
        // rollout where it dies, as `run_single` does.
        stopped = early_stop(world.agents().len(), &observations, &eval_config, stock);
        if stopped.is_some() {
            break;
        }
        if started.elapsed() > run_timeout {
            timed_out = true;
            break;
        }
    }
    let mode = if timed_out {
        TIMEOUT_MODE
    } else if let Some(failure) = stopped {
        mode_label(&Some(failure))
    } else {
        match evaluate_within_budget(&world, &observations, &eval_config, horizon, eval_timeout) {
            Some(breakdown) => mode_label(&breakdown.failure),
            None => EVAL_TIMEOUT_MODE,
        }
    };
    let termination_tick = world.tick();
    let reached_horizon = !timed_out && termination_tick == horizon;
    let tail_mean_producers = reached_horizon
        .then(|| tail_mean(&producers, band))
        .flatten();
    SeedRecord {
        seed,
        termination_tick,
        mode: mode.to_string(),
        live: mode == "none" && reached_horizon,
        founder_free_energy: free_energy[0],
        provisioning_transient_tick: provisioning_transient_tick(&free_energy),
        producer_settling_tick: if reached_horizon {
            producer_settling_tick(&producers, band)
        } else {
            None
        },
        tail_mean_producers,
        founder_producers: producers[0],
        peak_producers: producers.iter().copied().max().unwrap_or(0),
        terminal_producers: *producers.last().unwrap_or(&0),
    }
}

/// One config: one JSON line in the output. Every row carries the procedure
/// that produced it (horizon, band, producer rule) so the recorded values
/// travel with their definition.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct ConfigRow {
    source: ConfigSource,
    config_index: usize,
    horizon: u64,
    band: Band,
    seeds: Vec<SeedRecord>,
}

/// Upper quantiles of a tick sample: 50th / 90th / 95th / max, nearest-rank.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
struct Quantiles {
    n: usize,
    p50: Option<u64>,
    p90: Option<u64>,
    p95: Option<u64>,
    max: Option<u64>,
}

fn quantiles(values: &[u64]) -> Quantiles {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let n = sorted.len();
    let at = |p: f64| -> Option<u64> {
        if n == 0 {
            return None;
        }
        let rank = (p * n as f64).ceil() as usize;
        Some(sorted[rank.clamp(1, n) - 1])
    };
    Quantiles {
        n,
        p50: at(0.5),
        p90: at(0.9),
        p95: at(0.95),
        max: sorted.last().copied(),
    }
}

/// The two transient ticks over one source's live runs.
#[derive(Clone, Debug, Default, serde::Serialize)]
struct SourceSummary {
    configs: usize,
    runs: usize,
    live_runs: usize,
    /// Live runs whose stock never re-exceeded the founder budget.
    live_runs_without_provisioning_transient: usize,
    /// Live runs with no producers over the tail (no settling tick).
    live_runs_without_settling_tick: usize,
    /// Dead / non-live runs by terminal mode, sorted by label.
    modes: Vec<(String, usize)>,
    provisioning_transient_tick: Quantiles,
    producer_settling_tick: Quantiles,
}

#[derive(Clone, Debug, serde::Serialize)]
struct Summary {
    /// How the ticks are defined, so the recorded values carry their procedure.
    procedure: &'static str,
    horizons: Vec<u64>,
    bands: Vec<Band>,
    atlas: SourceSummary,
    sample: SourceSummary,
}

const PROCEDURE: &str = "Each (config, seed) runs to the horizon with nothing injected. \
provisioning_transient_tick: first tick at which the living free-energy stock (World::free_energy) \
exceeds its tick-0 value; null if never. producer_settling_tick: the last tick at which the producer \
count (agents with photosynthetic_absorption >= heterotrophy) lies outside the band mean*(1 +/- band.fraction), \
where mean is the mean producer count over the run's final band.tail_ticks ticks; 0 if never outside; \
null if the run did not reach the horizon or the tail mean is zero. Quantiles are nearest-rank over live \
runs (mode none, reached the horizon), split by source.";

fn summarise_source(rows: &[ConfigRow], source: ConfigSource) -> SourceSummary {
    let rows: Vec<&ConfigRow> = rows.iter().filter(|r| r.source == source).collect();
    let runs: Vec<&SeedRecord> = rows.iter().flat_map(|r| r.seeds.iter()).collect();
    let live: Vec<&SeedRecord> = runs.iter().copied().filter(|s| s.live).collect();
    let mut modes: BTreeMap<String, usize> = BTreeMap::new();
    for s in runs.iter().filter(|s| !s.live) {
        *modes.entry(s.mode.clone()).or_default() += 1;
    }
    let provisioning: Vec<u64> = live
        .iter()
        .filter_map(|s| s.provisioning_transient_tick)
        .collect();
    let settling: Vec<u64> = live
        .iter()
        .filter_map(|s| s.producer_settling_tick)
        .collect();
    SourceSummary {
        configs: rows.len(),
        runs: runs.len(),
        live_runs: live.len(),
        live_runs_without_provisioning_transient: live.len() - provisioning.len(),
        live_runs_without_settling_tick: live.len() - settling.len(),
        modes: modes.into_iter().collect(),
        provisioning_transient_tick: quantiles(&provisioning),
        producer_settling_tick: quantiles(&settling),
    }
}

fn summarise(rows: &[ConfigRow]) -> Summary {
    let mut horizons: Vec<u64> = rows.iter().map(|r| r.horizon).collect();
    horizons.sort_unstable();
    horizons.dedup();
    let mut bands: Vec<Band> = Vec::new();
    for r in rows {
        if !bands.contains(&r.band) {
            bands.push(r.band);
        }
    }
    Summary {
        procedure: PROCEDURE,
        horizons,
        bands,
        atlas: summarise_source(rows, ConfigSource::Atlas),
        sample: summarise_source(rows, ConfigSource::Sample),
    }
}

/// Command line: `--limit N`, `--horizon T`, `--out PATH`, `--atlas PATH`,
/// `--seeds N` (1..=8), `--configs atlas:0,sample:12`, `--tail-ticks N`,
/// `--run-timeout-secs N` (per (config, seed) simulation budget),
/// `--eval-timeout-secs N` (per (config, seed) evaluation budget),
/// `--summary` (no runs; summarise the rows already in `--out`).
#[derive(Clone, Debug, PartialEq)]
struct Args {
    limit: Option<usize>,
    horizon: u64,
    out: PathBuf,
    atlas: PathBuf,
    seeds: u64,
    configs: Option<HashSet<(ConfigSource, usize)>>,
    band: Band,
    run_timeout: Duration,
    eval_timeout: Duration,
    summary_only: bool,
}

const DEFAULT_HORIZON: u64 = 3000;
const DEFAULT_RUN_TIMEOUT_SECS: u64 = 300;
const DEFAULT_OUT: &str = "target/settling-time.jsonl";
const N_SEEDS: u64 = 8;
const SEED_BASE: u64 = 1000;

fn parse_args<I: IntoIterator<Item = String>>(argv: I) -> Args {
    let mut args = Args {
        limit: None,
        horizon: DEFAULT_HORIZON,
        out: PathBuf::from(DEFAULT_OUT),
        atlas: PathBuf::from("atlas.json"),
        seeds: N_SEEDS,
        configs: None,
        band: BAND,
        run_timeout: Duration::from_secs(DEFAULT_RUN_TIMEOUT_SECS),
        eval_timeout: Duration::from_secs(DEFAULT_EVAL_TIMEOUT_SECS),
        summary_only: false,
    };
    let mut it = argv.into_iter();
    let value = |flag: &str, it: &mut I::IntoIter| -> String {
        it.next()
            .unwrap_or_else(|| panic!("settling_time: {flag} needs a value"))
    };
    let number = |flag: &str, raw: &str| -> u64 {
        raw.parse()
            .unwrap_or_else(|_| panic!("settling_time: {flag} {raw:?} is not an integer"))
    };
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--limit" => args.limit = Some(number("--limit", &value("--limit", &mut it)) as usize),
            "--horizon" => {
                args.horizon = number("--horizon", &value("--horizon", &mut it));
                assert!(
                    args.horizon > 0,
                    "settling_time: --horizon must be positive"
                );
            }
            "--out" => args.out = PathBuf::from(value("--out", &mut it)),
            "--atlas" => args.atlas = PathBuf::from(value("--atlas", &mut it)),
            "--seeds" => {
                args.seeds = number("--seeds", &value("--seeds", &mut it)).clamp(1, N_SEEDS)
            }
            "--configs" => {
                args.configs = Some(parse_selector(
                    &value("--configs", &mut it),
                    "--configs",
                    None,
                ))
            }
            "--tail-ticks" => {
                args.band.tail_ticks =
                    number("--tail-ticks", &value("--tail-ticks", &mut it)) as usize;
                assert!(
                    args.band.tail_ticks > 0,
                    "settling_time: --tail-ticks must be positive"
                );
            }
            "--run-timeout-secs" => {
                args.run_timeout = Duration::from_secs(number(
                    "--run-timeout-secs",
                    &value("--run-timeout-secs", &mut it),
                ))
            }
            EVAL_TIMEOUT_FLAG => {
                args.eval_timeout = Duration::from_secs(number(
                    EVAL_TIMEOUT_FLAG,
                    &value(EVAL_TIMEOUT_FLAG, &mut it),
                ))
            }
            "--summary" => args.summary_only = true,
            other => panic!("settling_time: unknown argument {other:?}"),
        }
    }
    assert!(
        args.horizon >= args.band.tail_ticks as u64,
        "settling_time: --horizon ({}) must cover the band tail ({})",
        args.horizon,
        args.band.tail_ticks
    );
    args
}

/// One config: the seed ensemble run in parallel (rayon's indexed collect
/// keeps seed order, so the row is bit-identical to the sequential map).
fn run_config(source: ConfigSource, config_index: usize, unit: &[f64], args: &Args) -> ConfigRow {
    let (params, dist) = decode(unit, &default_ranges());
    let (horizon, band) = (args.horizon, args.band);
    let seeds: Vec<SeedRecord> = (0..args.seeds)
        .into_par_iter()
        .map(|s| {
            run_seed(
                &params,
                &dist,
                SEED_BASE + s,
                horizon,
                band,
                args.run_timeout,
                args.eval_timeout,
            )
        })
        .collect();
    ConfigRow {
        source,
        config_index,
        horizon,
        band,
        seeds,
    }
}

fn print_source(name: &str, s: &SourceSummary) {
    println!(
        "## {name}: {} configs, {} runs, {} live",
        s.configs, s.runs, s.live_runs
    );
    let q = |q: &Quantiles| {
        let f = |v: Option<u64>| v.map_or("-".to_string(), |v| v.to_string());
        format!(
            "n {:>4}  p50 {:>5}  p90 {:>5}  p95 {:>5}  max {:>5}",
            q.n,
            f(q.p50),
            f(q.p90),
            f(q.p95),
            f(q.max)
        )
    };
    println!(
        "  provisioning transient tick  {}  (never: {})",
        q(&s.provisioning_transient_tick),
        s.live_runs_without_provisioning_transient
    );
    println!(
        "  producer settling tick       {}  (undefined: {})",
        q(&s.producer_settling_tick),
        s.live_runs_without_settling_tick
    );
    if !s.modes.is_empty() {
        let modes: Vec<String> = s.modes.iter().map(|(m, n)| format!("{m} {n}")).collect();
        println!("  non-live runs by mode: {}", modes.join(", "));
    }
}

fn print_summary(summary: &Summary) {
    println!("\n# Settling-time instrument (issue #504)");
    println!(
        "# horizon(s) {:?}; band(s) {:?}",
        summary.horizons, summary.bands
    );
    println!("# {}\n", summary.procedure);
    print_source("atlas", &summary.atlas);
    print_source("sample", &summary.sample);
    println!(
        "\n{}",
        serde_json::to_string_pretty(summary).expect("serialise summary")
    );
}

fn main() {
    let args = parse_args(std::env::args().skip(1));
    if !args.summary_only {
        let atlas_units = read_atlas_units(&args.atlas);
        let sampled = sampled_units(default_ranges().len());
        let done = done_configs(&args.out);
        let tasks = plan_tasks(
            atlas_units.len(),
            sampled.len(),
            args.configs.as_ref(),
            &done,
            args.limit,
        );
        eprintln!(
            "settling_time: {} atlas + {} sampled configs × {} seeds, horizon {} ticks, band ±{} of the final {} ticks; {} done in {}, running {} now",
            atlas_units.len(),
            sampled.len(),
            args.seeds,
            args.horizon,
            args.band.fraction,
            args.band.tail_ticks,
            done.len(),
            args.out.display(),
            tasks.len()
        );
        if args.configs.is_some() || args.seeds != N_SEEDS || args.horizon != DEFAULT_HORIZON {
            eprintln!(
                "settling_time: SUBSET MODE — configs={:?}, seeds={}, horizon={} (summary is partial)",
                args.configs.as_ref().map(|f| f.len()),
                args.seeds,
                args.horizon
            );
        }
        let start = Instant::now();
        let total = tasks.len();
        // Configs run sequentially (the seeds inside are the parallel unit)
        // so each row is appended as soon as it is complete and a killed
        // sweep loses at most one config.
        for (n, (source, idx)) in tasks.into_iter().enumerate() {
            let unit = match source {
                ConfigSource::Atlas => &atlas_units[idx],
                ConfigSource::Sample => &sampled[idx],
            };
            let row = run_config(source, idx, unit, &args);
            append_row(&args.out, &row);
            eprintln!(
                "  {:?}:{idx} done ({}/{total}, {} live of {}, {:.0}s elapsed)",
                source,
                n + 1,
                row.seeds.iter().filter(|s| s.live).count(),
                row.seeds.len(),
                start.elapsed().as_secs_f64()
            );
        }
        eprintln!(
            "settling_time: {total} configs run in {:.0}s; appended to {}",
            start.elapsed().as_secs_f64(),
            args.out.display()
        );
    }
    let rows: Vec<ConfigRow> = read_rows(&args.out);
    eprintln!(
        "settling_time: summarising {} rows from {}",
        rows.len(),
        args.out.display()
    );
    print_summary(&summarise(&rows));
}

#[cfg(test)]
mod tests {
    use super::*;
    use explorers_search::sweep::AtlasFile;

    #[test]
    fn args_default_to_the_full_sweep_and_accept_each_flag() {
        let args = parse_args(std::iter::empty());
        assert_eq!(args.limit, None);
        assert_eq!(args.horizon, DEFAULT_HORIZON);
        assert_eq!(args.out, PathBuf::from(DEFAULT_OUT));
        assert_eq!(args.atlas, PathBuf::from("atlas.json"));
        assert_eq!(args.seeds, N_SEEDS);
        assert_eq!(args.configs, None);
        assert_eq!(args.band, BAND);
        assert_eq!(args.run_timeout, Duration::from_secs(300));
        assert_eq!(args.eval_timeout, Duration::from_secs(300));
        assert!(!args.summary_only);
        let args = parse_args(
            "--limit 3 --horizon 600 --out target/x.jsonl --atlas a.json --seeds 2 --configs atlas:0,atlas:1 --tail-ticks 100 --run-timeout-secs 7 --eval-timeout-secs 11 --summary"
                .split(' ')
                .map(String::from),
        );
        assert_eq!(args.limit, Some(3));
        assert_eq!(args.horizon, 600);
        assert_eq!(args.out, PathBuf::from("target/x.jsonl"));
        assert_eq!(args.atlas, PathBuf::from("a.json"));
        assert_eq!(args.seeds, 2);
        assert_eq!(args.configs.as_ref().map(|c| c.len()), Some(2));
        assert_eq!(args.band.tail_ticks, 100);
        assert_eq!(args.run_timeout, Duration::from_secs(7));
        assert_eq!(args.eval_timeout, Duration::from_secs(11));
        assert!(args.summary_only);
    }

    fn seed(
        seed: u64,
        mode: &str,
        live: bool,
        prov: Option<u64>,
        settle: Option<u64>,
    ) -> SeedRecord {
        SeedRecord {
            seed,
            termination_tick: 100,
            mode: mode.to_string(),
            live,
            founder_free_energy: 1.0,
            provisioning_transient_tick: prov,
            producer_settling_tick: settle,
            tail_mean_producers: None,
            founder_producers: 1,
            peak_producers: 1,
            terminal_producers: 1,
        }
    }

    #[test]
    fn summary_reads_upper_quantiles_over_live_runs_split_by_source() {
        let band = BAND;
        let rows = vec![
            ConfigRow {
                source: ConfigSource::Atlas,
                config_index: 0,
                horizon: 100,
                band,
                seeds: vec![
                    seed(0, "none", true, Some(10), Some(50)),
                    seed(1, "none", true, Some(30), Some(70)),
                    seed(2, "extinction", false, Some(90), None),
                ],
            },
            ConfigRow {
                source: ConfigSource::Atlas,
                config_index: 1,
                horizon: 100,
                band,
                seeds: vec![
                    seed(0, "none", true, None, Some(60)),
                    seed(1, "none", true, Some(20), None),
                ],
            },
            ConfigRow {
                source: ConfigSource::Sample,
                config_index: 0,
                horizon: 100,
                band,
                seeds: vec![seed(0, "monoculture", false, Some(5), Some(5))],
            },
        ];
        let s = summarise(&rows);
        assert_eq!(s.horizons, vec![100]);
        assert_eq!(s.bands, vec![band]);
        assert_eq!(s.atlas.configs, 2);
        assert_eq!(s.atlas.runs, 5);
        assert_eq!(s.atlas.live_runs, 4);
        assert_eq!(s.atlas.live_runs_without_provisioning_transient, 1);
        assert_eq!(s.atlas.live_runs_without_settling_tick, 1);
        assert_eq!(s.atlas.modes, vec![("extinction".to_string(), 1)]);
        // Live provisioning ticks {10, 30, 20}: p50 20, p90/p95/max 30.
        assert_eq!(
            s.atlas.provisioning_transient_tick,
            Quantiles {
                n: 3,
                p50: Some(20),
                p90: Some(30),
                p95: Some(30),
                max: Some(30)
            }
        );
        // Live settling ticks {50, 70, 60}: p50 60, max 70.
        assert_eq!(s.atlas.producer_settling_tick.p50, Some(60));
        assert_eq!(s.atlas.producer_settling_tick.max, Some(70));
        // The dead sample run contributes to no quantile.
        assert_eq!(s.sample.live_runs, 0);
        assert_eq!(s.sample.provisioning_transient_tick, Quantiles::default());
        assert_eq!(s.sample.modes, vec![("monoculture".to_string(), 1)]);
    }

    #[test]
    fn quantiles_are_nearest_rank() {
        let q = quantiles(&[5, 1, 4, 2, 3, 6, 7, 8, 9, 10]);
        assert_eq!(q.n, 10);
        assert_eq!(q.p50, Some(5));
        assert_eq!(q.p90, Some(9));
        assert_eq!(q.p95, Some(10));
        assert_eq!(q.max, Some(10));
        assert_eq!(quantiles(&[]), Quantiles::default());
    }

    fn atlas_cell(index: usize) -> (WorldParameters, InitialDistribution) {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../atlas.json");
        let atlas: AtlasFile =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        decode(&atlas.cells[index].unit, &default_ranges())
    }

    #[test]
    fn a_rollout_yields_a_record_with_termination_and_the_two_transient_ticks() {
        let (params, dist) = atlas_cell(0);
        let band = Band {
            tail_ticks: 10,
            fraction: 0.2,
        };
        let record = run_seed(&params, &dist, 1000, 40, band, Duration::MAX, Duration::MAX);
        assert_eq!(record.seed, 1000);
        assert!(record.termination_tick >= 1 && record.termination_tick <= 40);
        assert!(record.founder_free_energy > 0.0);
        assert!(record.founder_producers > 0);
        assert_eq!(
            record.live,
            record.mode == "none" && record.termination_tick == 40
        );
        if let Some(t) = record.provisioning_transient_tick {
            assert!(t >= 1 && t <= record.termination_tick);
        }
        if record.termination_tick == 40 {
            assert!(record.tail_mean_producers.is_some());
            assert!(record.producer_settling_tick.is_some_and(|t| t <= 40));
        }
        // Determinism: the same seed reproduces the record byte for byte.
        let again = run_seed(&params, &dist, 1000, 40, band, Duration::MAX, Duration::MAX);
        assert_eq!(
            serde_json::to_string(&record).unwrap(),
            serde_json::to_string(&again).unwrap()
        );
    }

    #[test]
    fn a_rollout_past_its_wall_clock_budget_stops_and_reads_as_a_timeout() {
        let (params, dist) = atlas_cell(0);
        let band = Band {
            tail_ticks: 10,
            fraction: 0.2,
        };
        // A zero budget is exceeded on the first step, so a run that would
        // otherwise continue to the horizon stops early and is not live.
        let record = run_seed(
            &params,
            &dist,
            1000,
            40,
            band,
            Duration::ZERO,
            Duration::MAX,
        );
        assert_eq!(record.mode, "timeout");
        assert!(!record.live);
        assert!(record.termination_tick >= 1 && record.termination_tick < 40);
        assert_eq!(record.producer_settling_tick, None);
        assert_eq!(record.tail_mean_producers, None);
        // The timeout is visible in the summary's mode breakdown.
        let row = ConfigRow {
            source: ConfigSource::Atlas,
            config_index: 0,
            horizon: 40,
            band,
            seeds: vec![record],
        };
        let summary = summarise(&[row]);
        assert_eq!(summary.atlas.live_runs, 0);
        assert_eq!(summary.atlas.modes, vec![("timeout".to_string(), 1)]);
    }

    #[test]
    fn a_rollout_that_simulates_but_cannot_be_evaluated_in_budget_reads_as_an_eval_timeout() {
        let (params, dist) = atlas_cell(0);
        let band = Band {
            tail_ticks: 10,
            fraction: 0.2,
        };
        // The simulation is unbounded and reaches the horizon; a zero
        // evaluation budget is spent before the verdict.
        let record = run_seed(
            &params,
            &dist,
            1000,
            40,
            band,
            Duration::MAX,
            Duration::ZERO,
        );
        assert_eq!(
            record.termination_tick, 40,
            "the rollout reached the horizon"
        );
        assert_eq!(record.mode, EVAL_TIMEOUT_MODE);
        assert!(!record.live, "no verdict, so not live");
        // Counted under its own mode, not folded into the timeouts.
        let row = ConfigRow {
            source: ConfigSource::Atlas,
            config_index: 0,
            horizon: 40,
            band,
            seeds: vec![record],
        };
        let summary = summarise(&[row]);
        assert_eq!(summary.atlas.live_runs, 0);
        assert_eq!(
            summary.atlas.modes,
            vec![(EVAL_TIMEOUT_MODE.to_string(), 1)]
        );
    }

    #[test]
    fn resume_skips_configs_already_in_the_output_and_limit_caps_the_rest() {
        let dir = std::env::temp_dir().join(format!("settling-time-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("out.jsonl");
        assert!(
            done_configs(&out).is_empty(),
            "missing file is an empty set"
        );
        std::fs::write(
            &out,
            "{\"source\":\"atlas\",\"config_index\":0,\"seeds\":[]}\n{\"source\":\"atlas\",\"config_index\":1,\"seeds\":[]}\n{\"source\":\"sample\",\"config_index\":0,\"seeds\":[]}\n",
        )
        .unwrap();
        let done = done_configs(&out);
        assert_eq!(done.len(), 3);
        let plan = plan_tasks(3, 2, None, &done, None);
        assert_eq!(
            plan,
            vec![(ConfigSource::Atlas, 2), (ConfigSource::Sample, 1)],
            "sweep order is atlas then sample, done configs skipped"
        );
        assert_eq!(
            plan_tasks(3, 2, None, &done, Some(1)),
            vec![(ConfigSource::Atlas, 2)]
        );
        let filter: HashSet<_> = [(ConfigSource::Sample, 1), (ConfigSource::Atlas, 0)]
            .into_iter()
            .collect();
        assert_eq!(
            plan_tasks(3, 2, Some(&filter), &done, None),
            vec![(ConfigSource::Sample, 1)]
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn settling_tick_is_last_tick_outside_the_tail_band() {
        let band = Band {
            tail_ticks: 4,
            fraction: 0.2,
        };
        // Bloom to 30, fall back, settle around 10 (band [8, 12]); the last
        // out-of-band tick is 5 (count 7), then the series stays in band.
        let series = [5, 20, 30, 25, 15, 7, 9, 11, 10, 10, 9, 11];
        assert_eq!(producer_settling_tick(&series, band), Some(5));
        // A brief late excursion re-opens the settling tick.
        let series = [5, 20, 30, 10, 10, 10, 13, 10, 10, 10, 10, 10];
        assert_eq!(producer_settling_tick(&series, band), Some(6));
        // Never leaves the band: settled from tick 0.
        assert_eq!(producer_settling_tick(&[10, 10, 10, 10, 10], band), Some(0));
        // Shorter than the tail, or no producers in the tail: undefined.
        assert_eq!(producer_settling_tick(&[10, 10, 10], band), None);
        assert_eq!(producer_settling_tick(&[3, 2, 1, 0, 0, 0, 0], band), None);
    }

    #[test]
    fn provisioning_transient_is_first_tick_stock_exceeds_founder_budget() {
        // Founder budget 10; the stock decays while the cohort lives off the
        // provisioning, then photosynthetic income overtakes it at tick 4.
        let series = [10.0, 9.0, 8.5, 10.0, 10.5, 12.0];
        assert_eq!(provisioning_transient_tick(&series), Some(4));
        // Never recovers the founder stock.
        assert_eq!(provisioning_transient_tick(&[10.0, 9.0, 8.0, 7.0]), None);
        assert_eq!(provisioning_transient_tick(&[]), None);
    }
}
