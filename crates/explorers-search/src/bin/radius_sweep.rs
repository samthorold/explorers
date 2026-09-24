//! Single-factor sweep of `light_competition_radius` at a fixed live baseline
//! (issue #462, re-scoped): the observational reading in
//! `docs/research/462-pi-space-explanatory-check.md` — over the LHS sample of
//! the search box the radius is the strongest predictor of a config's live and
//! lockup fractions, monotone across its searched range — was taken with all
//! 32 dimensions varying. This bin holds every other parameter at a baseline
//! that is live at the settled horizon and moves the radius alone through its
//! searched range, so the effect is read as a response curve rather than a
//! correlation.
//!
//! Each level runs `--seeds` rollouts through the genesis step loop exactly as
//! `explorers_genesis::run_single` does (retained event kinds, per-step
//! compaction, the evaluator's `early_stop`, `evaluate_from_log` at the
//! horizon) and records per seed the terminal mode, the peak population, the
//! terminal producer / consumer counts and the settled-window reads the
//! evaluator makes (carcass-locked fraction, coexistence duration, the guild
//! flags). The row also carries the count ceiling's geometric factor
//! `m = ⌊√2·L/r⌋ + 1` (viability.md, *sustained-count ceiling*) at that level,
//! the candidate the note names for the coordinate the permanence groups lack.
//!
//! Resumable in the `explorers_search::sweep` shape: one JSON line per level,
//! appended as soon as its seeds finish; levels already in the file are
//! skipped on start.
//!
//! A (level, seed) run carries two wall-clock budgets (#523). The
//! **simulation budget** (`--run-timeout-secs`, default 300) bounds the step
//! loop; a rollout that exhausts it is recorded as mode `timeout`. The
//! **evaluation budget** (`--eval-timeout-secs`, default 300) bounds the
//! terminal evaluation of a rollout that reached the horizon — on a dense
//! terminal roster the per-snapshot clustering dominates the run — and one
//! that exhausts it is recorded as mode `eval_timeout`, with every
//! settled-window read absent. Both are unfinished: the summary reads its
//! fractions over the finished seeds and counts the two apart.
//!
//! ```text
//! radius_sweep [--baseline recipe|atlas:N|sample:N] [--levels 1,2,3,5,8,12,16,20]
//!              [--seeds 5] [--horizon 2000] [--out target/radius-sweep.jsonl]
//!              [--run-timeout-secs 300] [--eval-timeout-secs 300] [--summary]
//! ```

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rayon::prelude::*;

use explorers_genesis::{EvalConfig, FailureMode};
use explorers_genesis_eval::{
    EVALUATOR_EVENT_KINDS, RolloutObservations, early_stop, sustainable_stock,
};
use explorers_search::config_source::sampled_units;
use explorers_search::search::{SearchConfig, decode, default_ranges};
use explorers_search::sweep::{
    DEFAULT_EVAL_TIMEOUT_SECS, EVAL_TIMEOUT_FLAG, EVAL_TIMEOUT_MODE, TIMEOUT_MODE, append_row,
    evaluate_within_budget, is_unfinished, read_atlas_units, read_rows,
};
use explorers_sim::{InitialDistribution, World, WorldParameters, WorldRecipe};

const DEFAULT_OUT: &str = "target/radius-sweep.jsonl";
/// Eight levels spanning the searched range of `light_competition_radius`
/// (`default_ranges`: 1–20), denser at the small end where the LHS quintiles
/// move fastest.
const DEFAULT_LEVELS: [f32; 8] = [1.0, 2.0, 3.0, 5.0, 8.0, 12.0, 16.0, 20.0];
const DEFAULT_SEEDS: u64 = 5;
const DEFAULT_RUN_TIMEOUT_SECS: u64 = 300;
/// Seeds are drawn from the same base the other research sweeps use.
const SEED_BASE: u64 = 1000;

#[derive(Clone, Debug, PartialEq)]
enum Baseline {
    Recipe(PathBuf),
    Atlas(usize),
    Sample(usize),
}

impl Baseline {
    fn label(&self) -> String {
        match self {
            Baseline::Recipe(p) => format!("recipe:{}", p.display()),
            Baseline::Atlas(i) => format!("atlas:{i}"),
            Baseline::Sample(i) => format!("sample:{i}"),
        }
    }

    fn load(&self, atlas: &Path) -> (WorldParameters, InitialDistribution) {
        match self {
            Baseline::Recipe(path) => {
                let contents = std::fs::read_to_string(path)
                    .unwrap_or_else(|e| panic!("radius_sweep: read {}: {e}", path.display()));
                let recipe: WorldRecipe = serde_json::from_str(&contents)
                    .unwrap_or_else(|e| panic!("radius_sweep: parse {}: {e}", path.display()));
                let dist = recipe.initial_distribution.unwrap_or_else(|| {
                    panic!(
                        "radius_sweep: {} has no initial_distribution",
                        path.display()
                    )
                });
                (recipe.parameters, dist)
            }
            Baseline::Atlas(i) => decode(&read_atlas_units(atlas)[*i], &default_ranges()),
            Baseline::Sample(i) => {
                let ranges = default_ranges();
                decode(&sampled_units(ranges.len())[*i], &ranges)
            }
        }
    }
}

struct Args {
    baseline: Baseline,
    levels: Vec<f32>,
    seeds: u64,
    horizon: u64,
    out: PathBuf,
    atlas: PathBuf,
    run_timeout: Duration,
    eval_timeout: Duration,
    summary_only: bool,
}

fn mode_label(failure: &Option<FailureMode>) -> &'static str {
    match failure {
        None => "none",
        Some(FailureMode::Extinction) => "extinction",
        Some(FailureMode::PopulationExplosion) => "population-explosion",
        Some(FailureMode::Monoculture) => "monoculture",
        Some(FailureMode::GeneralistDominance) => "generalist-dominance",
        Some(FailureMode::EnergyDeath) => "energy-death",
        Some(FailureMode::NutrientLockup) => "nutrient-lockup",
    }
}

fn producer_consumer_counts(world: &World) -> (usize, usize) {
    world.agents().iter().fold((0, 0), |(p, c), a| {
        if a.traits.photosynthetic_absorption >= a.traits.heterotrophy {
            (p + 1, c)
        } else {
            (p, c + 1)
        }
    })
}

/// One (level × seed) rollout.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct SeedRecord {
    seed: u64,
    /// `evaluate_from_log`'s failure at the horizon, the gate that stopped the
    /// rollout, `"timeout"` (simulation budget spent) or `"eval_timeout"`
    /// (evaluation budget spent at the horizon).
    mode: String,
    termination_tick: u64,
    live: bool,
    founders: usize,
    peak_population: usize,
    terminal_producers: usize,
    terminal_consumers: usize,
    /// The evaluator's settled-window reads, present only when the rollout
    /// reached the horizon and was verdicted there.
    carcass_locked_fraction: Option<f32>,
    coexistence_duration: Option<f32>,
    has_decomposer_guild: Option<bool>,
    has_consumer_guild: Option<bool>,
}

fn run_seed(
    params: &WorldParameters,
    dist: &InitialDistribution,
    seed: u64,
    horizon: u64,
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
    let founders = world.agents().len();
    let mut peak_population = founders;
    for _ in 0..horizon {
        world.step();
        observations.observe(&world, eval_config.coexistence_sample_interval);
        world.compact_event_log_before(observations.consumed_events());
        peak_population = peak_population.max(world.agents().len());
        stopped = early_stop(world.agents().len(), &observations, &eval_config, stock);
        if stopped.is_some() {
            break;
        }
        if started.elapsed() > run_timeout {
            timed_out = true;
            break;
        }
    }
    let evaluated = !timed_out && stopped.is_none();
    let breakdown = if evaluated {
        evaluate_within_budget(&world, &observations, &eval_config, horizon, eval_timeout)
    } else {
        None
    };
    let mode = if timed_out {
        TIMEOUT_MODE.to_string()
    } else if let Some(failure) = stopped {
        mode_label(&Some(failure)).to_string()
    } else {
        match &breakdown {
            Some(breakdown) => mode_label(&breakdown.failure).to_string(),
            None => EVAL_TIMEOUT_MODE.to_string(),
        }
    };
    let termination_tick = world.tick();
    let (terminal_producers, terminal_consumers) = producer_consumer_counts(&world);
    SeedRecord {
        seed,
        live: mode == "none" && termination_tick == horizon,
        mode,
        termination_tick,
        founders,
        peak_population,
        terminal_producers,
        terminal_consumers,
        carcass_locked_fraction: breakdown.as_ref().map(|b| b.carcass_locked_fraction),
        coexistence_duration: breakdown.as_ref().map(|b| b.coexistence_duration),
        has_decomposer_guild: breakdown.as_ref().map(|b| b.has_decomposer_guild),
        has_consumer_guild: breakdown.as_ref().map(|b| b.has_consumer_guild),
    }
}

/// One radius level: one JSON line.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct LevelRow {
    baseline: String,
    /// The baseline's own radius, for reference.
    baseline_radius: f32,
    light_competition_radius: f32,
    world_extent: f32,
    /// The count ceiling's geometric factor at this level,
    /// `m = ⌊√2·L/r⌋ + 1`.
    ceiling_m: u64,
    horizon: u64,
    seeds: Vec<SeedRecord>,
}

fn ceiling_m(world_extent: f32, radius: f32) -> u64 {
    ((2.0_f64).sqrt() * world_extent as f64 / radius as f64).floor() as u64 + 1
}

fn run_level(args: &Args, base: &(WorldParameters, InitialDistribution), radius: f32) -> LevelRow {
    let (base_params, dist) = base;
    let params = WorldParameters {
        light_competition_radius: radius,
        ..base_params.clone()
    };
    let seeds: Vec<SeedRecord> = (0..args.seeds)
        .into_par_iter()
        .map(|s| {
            run_seed(
                &params,
                dist,
                SEED_BASE + s,
                args.horizon,
                args.run_timeout,
                args.eval_timeout,
            )
        })
        .collect();
    LevelRow {
        baseline: args.baseline.label(),
        baseline_radius: base_params.light_competition_radius,
        light_competition_radius: radius,
        world_extent: params.world_extent,
        ceiling_m: ceiling_m(params.world_extent, radius),
        horizon: args.horizon,
        seeds,
    }
}

fn parse_args<I: IntoIterator<Item = String>>(argv: I) -> Args {
    let mut args = Args {
        baseline: Baseline::Recipe(PathBuf::from("recipe.json")),
        levels: DEFAULT_LEVELS.to_vec(),
        seeds: DEFAULT_SEEDS,
        horizon: SearchConfig::default().max_ticks,
        out: PathBuf::from(DEFAULT_OUT),
        atlas: PathBuf::from("atlas.json"),
        run_timeout: Duration::from_secs(DEFAULT_RUN_TIMEOUT_SECS),
        eval_timeout: Duration::from_secs(DEFAULT_EVAL_TIMEOUT_SECS),
        summary_only: false,
    };
    let mut it = argv.into_iter();
    let value = |flag: &str, it: &mut I::IntoIter| -> String {
        it.next()
            .unwrap_or_else(|| panic!("radius_sweep: {flag} needs a value"))
    };
    let number = |flag: &str, raw: &str| -> u64 {
        raw.parse()
            .unwrap_or_else(|_| panic!("radius_sweep: {flag} {raw:?} is not an integer"))
    };
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--baseline" => {
                let raw = value("--baseline", &mut it);
                args.baseline = match raw.split_once(':') {
                    Some(("atlas", i)) => Baseline::Atlas(number("--baseline", i) as usize),
                    Some(("sample", i)) => Baseline::Sample(number("--baseline", i) as usize),
                    Some(("recipe", p)) => Baseline::Recipe(PathBuf::from(p)),
                    None if raw == "recipe" => Baseline::Recipe(PathBuf::from("recipe.json")),
                    _ => panic!(
                        "radius_sweep: --baseline must be recipe|recipe:PATH|atlas:N|sample:N"
                    ),
                };
            }
            "--levels" => {
                let raw = value("--levels", &mut it);
                args.levels = raw
                    .split(',')
                    .map(|t| {
                        t.trim().parse().unwrap_or_else(|_| {
                            panic!("radius_sweep: --levels entry {t:?} is not a number")
                        })
                    })
                    .collect();
                assert!(!args.levels.is_empty(), "radius_sweep: --levels is empty");
            }
            "--seeds" => args.seeds = number("--seeds", &value("--seeds", &mut it)).max(1),
            "--horizon" => {
                args.horizon = number("--horizon", &value("--horizon", &mut it));
                assert!(args.horizon > 0, "radius_sweep: --horizon must be positive");
            }
            "--out" => args.out = PathBuf::from(value("--out", &mut it)),
            "--atlas" => args.atlas = PathBuf::from(value("--atlas", &mut it)),
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
            other => panic!("radius_sweep: unknown argument {other:?}"),
        }
    }
    args
}

/// One level's reads over its finished seeds — those that exhausted neither
/// budget. The unfinished are counted apart, by budget, and enter no fraction.
#[derive(Clone, Debug, PartialEq)]
struct LevelSummary {
    /// Finished seeds: the denominator of every fraction.
    n: usize,
    /// Seeds that exhausted the simulation budget.
    timed_out: usize,
    /// Seeds that exhausted the evaluation budget.
    eval_timed_out: usize,
    live: f64,
    lockup: f64,
    extinct: f64,
    /// Non-live verdicts other than lockup and extinction.
    other: f64,
    consumers_at_t: f64,
    median_peak: Option<f64>,
    median_carcass_locked: Option<f64>,
    decomposer_guilds: usize,
    consumer_guilds: usize,
}

fn summarise_level(row: &LevelRow) -> LevelSummary {
    let finished: Vec<&SeedRecord> = row
        .seeds
        .iter()
        .filter(|s| !is_unfinished(&s.mode))
        .collect();
    let n = finished.len();
    let frac = |pred: &dyn Fn(&SeedRecord) -> bool| -> f64 {
        if n == 0 {
            f64::NAN
        } else {
            finished.iter().filter(|s| pred(s)).count() as f64 / n as f64
        }
    };
    let median = |mut v: Vec<f64>| -> Option<f64> {
        if v.is_empty() {
            return None;
        }
        v.sort_by(|a, b| a.total_cmp(b));
        Some(v[v.len() / 2])
    };
    let mode_count = |mode: &str| row.seeds.iter().filter(|s| s.mode == mode).count();
    LevelSummary {
        n,
        timed_out: mode_count(TIMEOUT_MODE),
        eval_timed_out: mode_count(EVAL_TIMEOUT_MODE),
        live: frac(&|s| s.live),
        lockup: frac(&|s| s.mode == "nutrient-lockup"),
        extinct: frac(&|s| s.mode == "extinction"),
        other: frac(&|s| !s.live && s.mode != "nutrient-lockup" && s.mode != "extinction"),
        consumers_at_t: frac(&|s| s.live && s.terminal_consumers > 0),
        median_peak: median(finished.iter().map(|s| s.peak_population as f64).collect()),
        median_carcass_locked: median(
            finished
                .iter()
                .filter_map(|s| s.carcass_locked_fraction.map(|c| c as f64))
                .collect(),
        ),
        decomposer_guilds: finished
            .iter()
            .filter(|s| s.has_decomposer_guild == Some(true))
            .count(),
        consumer_guilds: finished
            .iter()
            .filter(|s| s.has_consumer_guild == Some(true))
            .count(),
    }
}

fn print_summary(rows: &[LevelRow]) {
    println!("# radius_sweep — light_competition_radius response at a fixed baseline");
    println!();
    println!(
        "| baseline | r | m | live | lockup | extinct | other | consumers at T | median peak | median carcass-locked | guild (dec/cons) | n | timeout | eval_timeout |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    let mut sorted: Vec<&LevelRow> = rows.iter().collect();
    sorted.sort_by(|a, b| {
        a.baseline.cmp(&b.baseline).then(
            a.light_competition_radius
                .total_cmp(&b.light_competition_radius),
        )
    });
    for row in sorted {
        let s = summarise_level(row);
        println!(
            "| {} | {} | {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {} | {} | {}/{} | {} of {} | {} | {} |",
            row.baseline,
            row.light_competition_radius,
            row.ceiling_m,
            s.live,
            s.lockup,
            s.extinct,
            s.other,
            s.consumers_at_t,
            s.median_peak.map_or("—".to_string(), |p| format!("{p:.0}")),
            s.median_carcass_locked
                .map_or("—".to_string(), |c| format!("{c:.3}")),
            s.decomposer_guilds,
            s.consumer_guilds,
            s.n,
            row.seeds.len(),
            s.timed_out,
            s.eval_timed_out
        );
    }
}

fn main() {
    let args = parse_args(std::env::args().skip(1));
    if !args.summary_only {
        let base = args.baseline.load(&args.atlas);
        let done: HashSet<(String, u32)> = read_rows::<LevelRow>(&args.out)
            .into_iter()
            .map(|r| (r.baseline, r.light_competition_radius.to_bits()))
            .collect();
        let label = args.baseline.label();
        let todo: Vec<f32> = args
            .levels
            .iter()
            .copied()
            .filter(|r| !done.contains(&(label.clone(), r.to_bits())))
            .collect();
        eprintln!(
            "radius_sweep: baseline {label} (its own radius {}, world_extent {}), {} levels × {} seeds, horizon {}; {} levels done in {}, running {} now",
            base.0.light_competition_radius,
            base.0.world_extent,
            args.levels.len(),
            args.seeds,
            args.horizon,
            args.levels.len() - todo.len(),
            args.out.display(),
            todo.len()
        );
        let start = Instant::now();
        for (n, radius) in todo.iter().enumerate() {
            let row = run_level(&args, &base, *radius);
            append_row(&args.out, &row);
            eprintln!(
                "  r = {radius} (m = {}) done ({}/{}, {} live of {}, modes {:?}, {:.0}s elapsed)",
                row.ceiling_m,
                n + 1,
                todo.len(),
                row.seeds.iter().filter(|s| s.live).count(),
                row.seeds.len(),
                row.seeds
                    .iter()
                    .map(|s| s.mode.as_str())
                    .collect::<Vec<_>>(),
                start.elapsed().as_secs_f64()
            );
        }
    }
    let rows: Vec<LevelRow> = read_rows(&args.out);
    eprintln!(
        "radius_sweep: summarising {} rows from {}",
        rows.len(),
        args.out.display()
    );
    print_summary(&rows);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ceiling_m_matches_viability_formula() {
        // L = 100, r = 8: ⌊√2·12.5⌋ + 1 = ⌊17.68⌋ + 1 = 18.
        assert_eq!(ceiling_m(100.0, 8.0), 18);
        // r = L: ⌊√2⌋ + 1 = 2.
        assert_eq!(ceiling_m(50.0, 50.0), 2);
    }

    #[test]
    fn parse_args_defaults_and_overrides() {
        let a = parse_args(Vec::<String>::new());
        assert_eq!(a.run_timeout, Duration::from_secs(DEFAULT_RUN_TIMEOUT_SECS));
        assert_eq!(
            a.eval_timeout,
            Duration::from_secs(DEFAULT_EVAL_TIMEOUT_SECS)
        );
        assert_eq!(a.baseline, Baseline::Recipe(PathBuf::from("recipe.json")));
        assert_eq!(a.levels, DEFAULT_LEVELS.to_vec());
        assert_eq!(a.horizon, SearchConfig::default().max_ticks);
        let a = parse_args(
            [
                "--baseline",
                "sample:7",
                "--levels",
                "1, 4",
                "--seeds",
                "3",
                "--run-timeout-secs",
                "7",
                "--eval-timeout-secs",
                "11",
            ]
            .map(String::from),
        );
        assert_eq!(a.baseline, Baseline::Sample(7));
        assert_eq!(a.levels, vec![1.0, 4.0]);
        assert_eq!(a.seeds, 3);
        assert_eq!(a.run_timeout, Duration::from_secs(7));
        assert_eq!(a.eval_timeout, Duration::from_secs(11));
    }

    #[test]
    fn short_sweep_is_resumable_and_byte_identical() {
        let dir = std::env::temp_dir().join(format!("radius-sweep-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let whole = dir.join("whole.jsonl");
        let split = dir.join("split.jsonl");
        let base = Baseline::Sample(0).load(&PathBuf::from("atlas.json"));
        let mk = |out: &PathBuf, levels: Vec<f32>| Args {
            baseline: Baseline::Sample(0),
            levels,
            seeds: 2,
            horizon: 40,
            out: out.clone(),
            atlas: PathBuf::from("atlas.json"),
            run_timeout: Duration::from_secs(60),
            eval_timeout: Duration::from_secs(60),
            summary_only: false,
        };
        for r in [2.0, 10.0] {
            append_row(&whole, &run_level(&mk(&whole, vec![2.0, 10.0]), &base, r));
        }
        append_row(&split, &run_level(&mk(&split, vec![2.0]), &base, 2.0));
        append_row(&split, &run_level(&mk(&split, vec![10.0]), &base, 10.0));
        assert_eq!(
            std::fs::read_to_string(&whole).unwrap(),
            std::fs::read_to_string(&split).unwrap()
        );
        let rows: Vec<LevelRow> = read_rows(&whole);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].ceiling_m, ceiling_m(rows[0].world_extent, 2.0));
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn sample_baseline() -> (WorldParameters, InitialDistribution) {
        Baseline::Sample(0).load(&PathBuf::from("atlas.json"))
    }

    #[test]
    fn a_rollout_whose_evaluation_overruns_its_budget_reads_as_an_eval_timeout() {
        let (params, dist) = sample_baseline();
        let record = run_seed(&params, &dist, SEED_BASE, 40, Duration::MAX, Duration::ZERO);
        assert_eq!(
            record.termination_tick, 40,
            "the rollout reached the horizon"
        );
        assert_eq!(record.mode, EVAL_TIMEOUT_MODE);
        assert!(!record.live);
        // No breakdown, so every breakdown-derived read is the not-read value.
        assert_eq!(record.carcass_locked_fraction, None);
        assert_eq!(record.coexistence_duration, None);
        assert_eq!(record.has_decomposer_guild, None);
        assert_eq!(record.has_consumer_guild, None);
    }

    #[test]
    fn a_level_reads_its_fractions_over_finished_seeds_and_counts_each_budget_apart() {
        let (params, dist) = sample_baseline();
        let finished = run_seed(&params, &dist, SEED_BASE, 40, Duration::MAX, Duration::MAX);
        assert!(!is_unfinished(&finished.mode));
        let unevaluated = SeedRecord {
            mode: EVAL_TIMEOUT_MODE.to_string(),
            live: false,
            carcass_locked_fraction: None,
            coexistence_duration: None,
            has_decomposer_guild: None,
            has_consumer_guild: None,
            ..finished.clone()
        };
        let timed_out = SeedRecord {
            mode: TIMEOUT_MODE.to_string(),
            ..unevaluated.clone()
        };
        let row = LevelRow {
            baseline: "sample:0".to_string(),
            baseline_radius: 1.0,
            light_competition_radius: 1.0,
            world_extent: 1.0,
            ceiling_m: 2,
            horizon: 40,
            seeds: vec![finished.clone(), unevaluated, timed_out],
        };
        let summary = summarise_level(&row);
        assert_eq!(summary.n, 1, "only the finished seed is read");
        assert_eq!(summary.timed_out, 1);
        assert_eq!(summary.eval_timed_out, 1);
        // The unfinished seeds are not "other" (a non-live verdict).
        let other = if finished.live { 0.0 } else { 1.0 };
        assert_eq!(summary.other, other);
    }
}
