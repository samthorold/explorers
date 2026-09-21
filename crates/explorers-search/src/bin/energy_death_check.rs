//! Zero-false-positive check for the history-free energy-death read (issue
//! #508): `explorers_genesis_eval::is_free_energy_dead_sustainable` — the
//! trailing lockup-window peak of the living stock against a fraction of the
//! config's sustainable stock (`sustainable_stock`, the energy form of
//! viability's solar ceiling) — compared against the history-peak stand-in
//! (`is_free_energy_dead`, `COLLAPSE_FRACTION`) on every run that reaches
//! the settled horizon, over the atlas's live cells and the 200-point LHS
//! sample of the search box, 8 seeds each.
//!
//! ## The promotion rule
//!
//! *Zero false positives* (viability.md): the new read is promoted only if
//! no run it calls dead is one the stand-in calls alive **and** that is
//! visibly alive at `T` — population at or above the roster gates' floor
//! (`ROSTER_FLOOR`) and at least one birth in the settled window `(T/2, T]`.
//! A disagreement on a moribund roster (a few bodies, no births) counts as a
//! disagreement but not as a false positive: the read may well be right
//! there and the stand-in wrong.
//!
//! ## How the runs reach `T`
//!
//! The comparison needs the stand-in's full-series read, so the rollout does
//! **not** stop on the incremental energy-death gate: the genesis step loop
//! is run as `explorers_genesis::run_single` does (retained event kinds,
//! per-step compaction, the evaluator's `early_stop`), but an `EnergyDeath`
//! stop is recorded (the tick it first fired) and the rollout carries on to
//! the horizon — the carry-to-`T` cross-check at fraction 1 for this one gate.
//! Extinction, explosion and nutrient lockup stop the rollout as usual; those
//! runs do not reach `T` and are excluded from the comparison. Note that the
//! evaluator reads energy death before lockup, so a run the energy-death gate
//! has flagged is no longer stopped by lockup either; it reaches `T` and is
//! verdicted there on the full series, exactly as a carried run is.
//!
//! Every seed record carries both verdicts, the sustainable stock, the
//! trailing-window peak the new read saw and their ratio, the population at
//! `T` and the settled-window births — so the fraction at which the read
//! *would* have had zero false positives (the smallest ratio over the
//! visibly-alive, stand-in-alive runs) is readable off the rows.
//!
//! A (config, seed) rollout has a wall-clock budget (`--run-timeout-secs`,
//! default 300): a knife-edge world that costs seconds per tick stops where
//! it is and is recorded as mode `timeout`, not reaching the horizon.
//!
//! ## Resumable by construction
//!
//! One JSON line per config appended to `--out` (default
//! `target/energy-death-check.jsonl`); configs already present are skipped;
//! `--limit N` runs at most `N` further configs. Fixed order and seed block,
//! so a loop of short foreground calls produces a file byte-identical to one
//! uninterrupted run (`explorers_search::sweep`).
//!
//! ## Running
//!
//!   cargo build --release -p explorers-search --bin energy_death_check
//!   ./target/release/energy_death_check --limit 5     # repeat until 0 run
//!   ./target/release/energy_death_check --summary

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use rayon::prelude::*;

use explorers_genesis::{EvalConfig, FailureMode};
use explorers_genesis_eval::{
    EVALUATOR_EVENT_KINDS, ROSTER_FLOOR, RolloutObservations, SUSTAINABLE_FRACTION, early_stop,
    evaluate_from_log, is_free_energy_dead, is_free_energy_dead_sustainable, sustainable_stock,
};
use explorers_search::config_source::{ConfigSource, parse_selector, sampled_units};
use explorers_search::search::{decode, default_ranges};
use explorers_search::sweep::{append_row, done_configs, plan_tasks, read_atlas_units, read_rows};
use explorers_sim::{InitialDistribution, World, WorldParameters};

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
    /// The world's tick when the rollout stopped: the horizon, where
    /// extinction / explosion / lockup stopped it, or where the wall-clock
    /// budget ran out.
    termination_tick: u64,
    /// The evaluator's terminal classification at the horizon
    /// (`evaluate_from_log`), the early stop's mode, or `"timeout"`.
    mode: String,
    /// The run got to `T`: the only runs the comparison reads.
    reached_horizon: bool,
    /// The stand-in (`is_free_energy_dead`, post-grace history peak) on the
    /// full series at the horizon.
    standin_dead: bool,
    /// First tick the incremental energy-death gate fired, if it did (the
    /// rollout carried on regardless).
    standin_first_fire_tick: Option<u64>,
    /// The new read (`is_free_energy_dead_sustainable`) at the horizon.
    sustainable_dead: bool,
    /// The config's sustainable stock, `F·m²·τ`.
    sustainable_stock: f32,
    /// The trailing lockup-window peak of the post-grace living stock the new
    /// read compared against it.
    window_peak_stock: f32,
    /// `window_peak_stock / sustainable_stock`.
    stock_ratio: f64,
    population_at_horizon: usize,
    /// Births (`Born` events) in the settled window `(T/2, T]`.
    settled_births: usize,
    /// Reached the horizon with population `>= ROSTER_FLOOR` and at least one
    /// settled-window birth.
    visibly_alive: bool,
}

/// Drive one (config, seed) through the genesis step loop as `run_single`
/// does, but carrying an energy-death stop to the horizon (see the module
/// doc), and read both energy-death verdicts on the full series.
fn run_seed(
    params: &WorldParameters,
    dist: &InitialDistribution,
    seed: u64,
    horizon: u64,
    run_timeout: Duration,
) -> SeedRecord {
    let eval_config = EvalConfig::default();
    let started = Instant::now();
    let mut timed_out = false;
    let mut stopped: Option<FailureMode> = None;
    let mut standin_first_fire_tick: Option<u64> = None;
    let mut world = World::new(params.clone(), dist.clone(), seed);
    world.retain_event_kinds(EVALUATOR_EVENT_KINDS);
    let mut observations = RolloutObservations::with_capacity(horizon as usize);
    for _ in 0..horizon {
        world.step();
        observations.observe(&world, eval_config.coexistence_sample_interval);
        world.compact_event_log_before(observations.consumed_events());
        match early_stop(world.agents().len(), &observations, &eval_config) {
            None => {}
            Some(FailureMode::EnergyDeath) => {
                standin_first_fire_tick.get_or_insert(world.tick());
            }
            Some(failure) => {
                stopped = Some(failure);
                break;
            }
        }
        if started.elapsed() > run_timeout {
            timed_out = true;
            break;
        }
    }
    let termination_tick = world.tick();
    let reached_horizon = !timed_out && stopped.is_none() && termination_tick == horizon;
    let mode = if timed_out {
        "timeout".to_string()
    } else if let Some(failure) = &stopped {
        mode_label(&Some(failure.clone())).to_string()
    } else {
        let breakdown = evaluate_from_log(&world, &observations, &eval_config, horizon);
        mode_label(&breakdown.failure).to_string()
    };
    let grace = eval_config.grace_ticks as usize;
    let window = eval_config.energy_death_window;
    let post_grace = observations.free_energy.get(grace..).unwrap_or(&[]);
    let stock = sustainable_stock(params);
    let window_peak_stock = post_grace[post_grace.len().saturating_sub(window)..]
        .iter()
        .copied()
        .fold(0.0_f32, f32::max);
    let population_at_horizon = world.agents().len();
    let settled_births = observations
        .born
        .iter()
        .filter(|b| b.tick > horizon / 2)
        .count();
    SeedRecord {
        seed,
        termination_tick,
        mode,
        reached_horizon,
        standin_dead: is_free_energy_dead(post_grace, window),
        standin_first_fire_tick,
        sustainable_dead: is_free_energy_dead_sustainable(post_grace, window, stock),
        sustainable_stock: stock,
        window_peak_stock,
        stock_ratio: f64::from(window_peak_stock / stock),
        population_at_horizon,
        settled_births,
        visibly_alive: reached_horizon
            && population_at_horizon >= ROSTER_FLOOR
            && settled_births > 0,
    }
}

/// One config: one JSON line in the output.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct ConfigRow {
    source: ConfigSource,
    config_index: usize,
    horizon: u64,
    seeds: Vec<SeedRecord>,
}

/// A run the new read calls dead, the stand-in calls alive, and that is
/// visibly alive at the horizon — the promotion rule's disqualifier.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
struct FalsePositive {
    source: ConfigSource,
    config_index: usize,
    seed: u64,
    stock_ratio: f64,
    population_at_horizon: usize,
    settled_births: usize,
}

/// The comparison counts over one population of runs.
#[derive(Clone, Debug, Default, serde::Serialize)]
struct Counts {
    configs: usize,
    runs: usize,
    /// Runs that got to `T` — the denominator of everything below.
    reached_horizon: usize,
    standin_dead: usize,
    sustainable_dead: usize,
    both_dead: usize,
    /// New read dead, stand-in alive (any roster).
    new_dead_standin_alive: usize,
    /// Stand-in dead, new read alive.
    standin_dead_new_alive: usize,
    visibly_alive: usize,
    /// `new_dead_standin_alive` restricted to visibly-alive runs.
    false_positives: usize,
    /// The smallest stock ratio over visibly-alive runs the stand-in calls
    /// alive: the fraction at which the new read would have zero false
    /// positives on this population.
    min_alive_stock_ratio: Option<f64>,
    /// Quantiles (p05 / p50 / p95) of the stock ratio over those same runs.
    alive_stock_ratio_p05: Option<f64>,
    alive_stock_ratio_p50: Option<f64>,
    alive_stock_ratio_p95: Option<f64>,
    /// Runs not reaching the horizon, by mode.
    modes: Vec<(String, usize)>,
}

#[derive(Clone, Debug, serde::Serialize)]
struct Summary {
    procedure: &'static str,
    horizons: Vec<u64>,
    sustainable_fraction: f32,
    atlas: Counts,
    sample: Counts,
    pooled: Counts,
    false_positive_cells: Vec<FalsePositive>,
    /// The promotion rule: zero false positives.
    passes: bool,
}

const PROCEDURE: &str = "Each (config, seed) runs to the horizon through the genesis step loop; \
extinction, explosion and nutrient lockup stop it, an energy-death stop is recorded and carried on. \
On runs that reach the horizon: standin_dead = is_free_energy_dead on the post-grace series \
(trailing-window peak below COLLAPSE_FRACTION of the post-grace history peak); sustainable_dead = \
is_free_energy_dead_sustainable (trailing-window peak below SUSTAINABLE_FRACTION of the config's \
sustainable stock F*m^2). visibly_alive = population at T >= ROSTER_FLOOR and >= 1 Born event in (T/2, T]. \
A false positive is sustainable_dead && !standin_dead && visibly_alive; the read is promoted only at zero.";

fn quantile(sorted: &[f64], p: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let rank = (p * sorted.len() as f64).ceil() as usize;
    Some(sorted[rank.clamp(1, sorted.len()) - 1])
}

fn count(rows: &[&ConfigRow]) -> Counts {
    let runs: Vec<&SeedRecord> = rows.iter().flat_map(|r| r.seeds.iter()).collect();
    let at_t: Vec<&SeedRecord> = runs.iter().copied().filter(|s| s.reached_horizon).collect();
    let mut modes: BTreeMap<String, usize> = BTreeMap::new();
    for s in runs.iter().filter(|s| !s.reached_horizon) {
        *modes.entry(s.mode.clone()).or_default() += 1;
    }
    let mut alive_ratios: Vec<f64> = at_t
        .iter()
        .filter(|s| s.visibly_alive && !s.standin_dead)
        .map(|s| s.stock_ratio)
        .collect();
    alive_ratios.sort_by(f64::total_cmp);
    Counts {
        configs: rows.len(),
        runs: runs.len(),
        reached_horizon: at_t.len(),
        standin_dead: at_t.iter().filter(|s| s.standin_dead).count(),
        sustainable_dead: at_t.iter().filter(|s| s.sustainable_dead).count(),
        both_dead: at_t
            .iter()
            .filter(|s| s.standin_dead && s.sustainable_dead)
            .count(),
        new_dead_standin_alive: at_t
            .iter()
            .filter(|s| s.sustainable_dead && !s.standin_dead)
            .count(),
        standin_dead_new_alive: at_t
            .iter()
            .filter(|s| s.standin_dead && !s.sustainable_dead)
            .count(),
        visibly_alive: at_t.iter().filter(|s| s.visibly_alive).count(),
        false_positives: at_t
            .iter()
            .filter(|s| s.sustainable_dead && !s.standin_dead && s.visibly_alive)
            .count(),
        min_alive_stock_ratio: alive_ratios.first().copied(),
        alive_stock_ratio_p05: quantile(&alive_ratios, 0.05),
        alive_stock_ratio_p50: quantile(&alive_ratios, 0.5),
        alive_stock_ratio_p95: quantile(&alive_ratios, 0.95),
        modes: modes.into_iter().collect(),
    }
}

fn summarise(rows: &[ConfigRow]) -> Summary {
    let mut horizons: Vec<u64> = rows.iter().map(|r| r.horizon).collect();
    horizons.sort_unstable();
    horizons.dedup();
    let of = |source: ConfigSource| -> Vec<&ConfigRow> {
        rows.iter().filter(|r| r.source == source).collect()
    };
    let all: Vec<&ConfigRow> = rows.iter().collect();
    let false_positive_cells: Vec<FalsePositive> = rows
        .iter()
        .flat_map(|r| {
            r.seeds
                .iter()
                .filter(|s| {
                    s.reached_horizon && s.sustainable_dead && !s.standin_dead && s.visibly_alive
                })
                .map(move |s| FalsePositive {
                    source: r.source,
                    config_index: r.config_index,
                    seed: s.seed,
                    stock_ratio: s.stock_ratio,
                    population_at_horizon: s.population_at_horizon,
                    settled_births: s.settled_births,
                })
        })
        .collect();
    Summary {
        procedure: PROCEDURE,
        horizons,
        sustainable_fraction: SUSTAINABLE_FRACTION,
        atlas: count(&of(ConfigSource::Atlas)),
        sample: count(&of(ConfigSource::Sample)),
        pooled: count(&all),
        passes: false_positive_cells.is_empty(),
        false_positive_cells,
    }
}

/// Command line: `--limit N`, `--horizon T`, `--out PATH`, `--atlas PATH`,
/// `--seeds N` (1..=8), `--configs atlas:0,sample:12`,
/// `--run-timeout-secs N`, `--summary`.
#[derive(Clone, Debug, PartialEq)]
struct Args {
    limit: Option<usize>,
    horizon: u64,
    out: PathBuf,
    atlas: PathBuf,
    seeds: u64,
    configs: Option<HashSet<(ConfigSource, usize)>>,
    run_timeout: Duration,
    summary_only: bool,
}

/// The settled horizon: `SearchConfig::max_ticks`'s default.
const DEFAULT_HORIZON: u64 = 2000;
const DEFAULT_RUN_TIMEOUT_SECS: u64 = 300;
const DEFAULT_OUT: &str = "target/energy-death-check.jsonl";
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
        run_timeout: Duration::from_secs(DEFAULT_RUN_TIMEOUT_SECS),
        summary_only: false,
    };
    let mut it = argv.into_iter();
    let value = |flag: &str, it: &mut I::IntoIter| -> String {
        it.next()
            .unwrap_or_else(|| panic!("energy_death_check: {flag} needs a value"))
    };
    let number = |flag: &str, raw: &str| -> u64 {
        raw.parse()
            .unwrap_or_else(|_| panic!("energy_death_check: {flag} {raw:?} is not an integer"))
    };
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--limit" => args.limit = Some(number("--limit", &value("--limit", &mut it)) as usize),
            "--horizon" => {
                args.horizon = number("--horizon", &value("--horizon", &mut it));
                assert!(
                    args.horizon > 0,
                    "energy_death_check: --horizon must be positive"
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
            "--run-timeout-secs" => {
                args.run_timeout = Duration::from_secs(number(
                    "--run-timeout-secs",
                    &value("--run-timeout-secs", &mut it),
                ))
            }
            "--summary" => args.summary_only = true,
            other => panic!("energy_death_check: unknown argument {other:?}"),
        }
    }
    args
}

/// One config: the seed ensemble run in parallel (rayon's indexed collect
/// keeps seed order, so the row is bit-identical to the sequential map).
fn run_config(
    source: ConfigSource,
    config_index: usize,
    unit: &[f64],
    seeds: u64,
    horizon: u64,
    run_timeout: Duration,
) -> ConfigRow {
    let (params, dist) = decode(unit, &default_ranges());
    let seeds: Vec<SeedRecord> = (0..seeds)
        .into_par_iter()
        .map(|s| run_seed(&params, &dist, SEED_BASE + s, horizon, run_timeout))
        .collect();
    ConfigRow {
        source,
        config_index,
        horizon,
        seeds,
    }
}

fn print_counts(name: &str, c: &Counts) {
    let f = |v: Option<f64>| v.map_or("-".to_string(), |v| format!("{v:.4}"));
    println!(
        "## {name}: {} configs, {} runs, {} reached T, {} visibly alive",
        c.configs, c.runs, c.reached_horizon, c.visibly_alive
    );
    println!(
        "  stand-in dead {}  new read dead {}  both {}  new-dead/stand-in-alive {}  stand-in-dead/new-alive {}",
        c.standin_dead,
        c.sustainable_dead,
        c.both_dead,
        c.new_dead_standin_alive,
        c.standin_dead_new_alive
    );
    println!(
        "  FALSE POSITIVES {}   stock ratio over visibly-alive, stand-in-alive runs: min {} p05 {} p50 {} p95 {}",
        c.false_positives,
        f(c.min_alive_stock_ratio),
        f(c.alive_stock_ratio_p05),
        f(c.alive_stock_ratio_p50),
        f(c.alive_stock_ratio_p95)
    );
    if !c.modes.is_empty() {
        let modes: Vec<String> = c.modes.iter().map(|(m, n)| format!("{m} {n}")).collect();
        println!("  runs not reaching T by mode: {}", modes.join(", "));
    }
}

fn print_summary(summary: &Summary) {
    println!("\n# Energy-death zero-false-positive check (issue #508)");
    println!(
        "# horizon(s) {:?}; SUSTAINABLE_FRACTION {}",
        summary.horizons, summary.sustainable_fraction
    );
    println!("# {}\n", summary.procedure);
    print_counts("atlas", &summary.atlas);
    print_counts("sample", &summary.sample);
    print_counts("pooled", &summary.pooled);
    println!(
        "\n## promotion rule: {}",
        if summary.passes {
            "PASSES (zero false positives)"
        } else {
            "FAILS"
        }
    );
    for fp in &summary.false_positive_cells {
        println!(
            "  {:?}:{} seed {}  ratio {:.4}  population {}  settled births {}",
            fp.source,
            fp.config_index,
            fp.seed,
            fp.stock_ratio,
            fp.population_at_horizon,
            fp.settled_births
        );
    }
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
            "energy_death_check: {} atlas + {} sampled configs × {} seeds, horizon {} ticks; {} done in {}, running {} now",
            atlas_units.len(),
            sampled.len(),
            args.seeds,
            args.horizon,
            done.len(),
            args.out.display(),
            tasks.len()
        );
        let start = Instant::now();
        let total = tasks.len();
        for (n, (source, idx)) in tasks.into_iter().enumerate() {
            let unit = match source {
                ConfigSource::Atlas => &atlas_units[idx],
                ConfigSource::Sample => &sampled[idx],
            };
            let row = run_config(
                source,
                idx,
                unit,
                args.seeds,
                args.horizon,
                args.run_timeout,
            );
            append_row(&args.out, &row);
            eprintln!(
                "  {:?}:{idx} done ({}/{total}, {} at T, {} FP, {:.0}s elapsed)",
                source,
                n + 1,
                row.seeds.iter().filter(|s| s.reached_horizon).count(),
                row.seeds
                    .iter()
                    .filter(|s| s.sustainable_dead && !s.standin_dead && s.visibly_alive)
                    .count(),
                start.elapsed().as_secs_f64()
            );
        }
        eprintln!(
            "energy_death_check: {total} configs run in {:.0}s; appended to {}",
            start.elapsed().as_secs_f64(),
            args.out.display()
        );
    }
    let rows: Vec<ConfigRow> = read_rows(&args.out);
    eprintln!(
        "energy_death_check: summarising {} rows from {}",
        rows.len(),
        args.out.display()
    );
    print_summary(&summarise(&rows));
}

#[cfg(test)]
mod tests {
    use super::*;
    use explorers_search::sweep::AtlasFile;
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::time::Duration;

    use explorers_search::config_source::ConfigSource;
    use explorers_search::search::{decode, default_ranges};
    use explorers_sim::{InitialDistribution, WorldParameters};

    fn seed(
        seed: u64,
        reached_horizon: bool,
        standin_dead: bool,
        sustainable_dead: bool,
        population: usize,
        settled_births: usize,
        stock_ratio: f64,
    ) -> SeedRecord {
        SeedRecord {
            seed,
            termination_tick: if reached_horizon { 100 } else { 50 },
            mode: if reached_horizon {
                "none"
            } else {
                "extinction"
            }
            .to_string(),
            reached_horizon,
            standin_dead,
            standin_first_fire_tick: standin_dead.then_some(80),
            sustainable_dead,
            sustainable_stock: 100.0,
            window_peak_stock: (stock_ratio * 100.0) as f32,
            stock_ratio,
            population_at_horizon: population,
            settled_births,
            visibly_alive: reached_horizon && population >= ROSTER_FLOOR && settled_births > 0,
        }
    }

    #[test]
    fn summary_counts_disagreements_and_false_positives_on_runs_that_reach_the_horizon() {
        let rows = vec![
            ConfigRow {
                source: ConfigSource::Atlas,
                config_index: 3,
                horizon: 100,
                seeds: vec![
                    // agree alive
                    seed(0, true, false, false, 30, 5, 0.5),
                    // new read dead, stand-in alive, visibly alive: FALSE POSITIVE
                    seed(1, true, false, true, 25, 2, 0.05),
                    // new read dead, stand-in alive, but a moribund roster: a
                    // disagreement, not a false positive
                    seed(2, true, false, true, 3, 0, 0.02),
                    // both dead
                    seed(3, true, true, true, 40, 9, 0.01),
                    // did not reach the horizon: excluded from every count
                    seed(4, false, false, true, 0, 0, 0.0),
                ],
            },
            ConfigRow {
                source: ConfigSource::Sample,
                config_index: 7,
                horizon: 100,
                seeds: vec![
                    // stand-in dead, new read alive: the other direction
                    seed(0, true, true, false, 25, 1, 0.3),
                    seed(1, true, false, false, 25, 1, 0.2),
                ],
            },
        ];
        let s = summarise(&rows);
        assert_eq!(s.atlas.runs, 5);
        assert_eq!(s.atlas.reached_horizon, 4);
        assert_eq!(s.atlas.standin_dead, 1);
        assert_eq!(s.atlas.sustainable_dead, 3);
        assert_eq!(s.atlas.new_dead_standin_alive, 2);
        assert_eq!(s.atlas.false_positives, 1);
        assert_eq!(s.atlas.standin_dead_new_alive, 0);
        assert_eq!(s.atlas.visibly_alive, 3);
        assert_eq!(s.sample.standin_dead_new_alive, 1);
        assert_eq!(s.sample.false_positives, 0);
        assert_eq!(s.pooled.false_positives, 1);
        assert_eq!(s.pooled.reached_horizon, 6);
        assert_eq!(
            s.false_positive_cells,
            vec![FalsePositive {
                source: ConfigSource::Atlas,
                config_index: 3,
                seed: 1,
                stock_ratio: 0.05,
                population_at_horizon: 25,
                settled_births: 2,
            }]
        );
        assert!(!s.passes);
        // The smallest stock ratio among visibly-alive runs the stand-in calls
        // alive is the fraction at which the read would have zero false
        // positives: 0.05 here.
        assert!((s.pooled.min_alive_stock_ratio.unwrap() - 0.05).abs() < 1e-9);
        // Removing the false positive makes it pass.
        let mut rows = rows;
        rows[0].seeds.remove(1);
        assert!(summarise(&rows).passes);
    }

    #[test]
    fn args_default_to_the_full_sweep_and_accept_each_flag() {
        let args = parse_args(std::iter::empty());
        assert_eq!(args.limit, None);
        assert_eq!(args.horizon, DEFAULT_HORIZON);
        assert_eq!(args.out, PathBuf::from(DEFAULT_OUT));
        assert_eq!(args.seeds, N_SEEDS);
        assert_eq!(args.configs, None);
        assert_eq!(args.run_timeout, Duration::from_secs(300));
        assert!(!args.summary_only);
        let args = parse_args(
            "--limit 3 --horizon 600 --out target/x.jsonl --atlas a.json --seeds 2 --configs atlas:0,atlas:1 --run-timeout-secs 7 --summary"
                .split(' ')
                .map(String::from),
        );
        assert_eq!(args.limit, Some(3));
        assert_eq!(args.horizon, 600);
        assert_eq!(args.out, PathBuf::from("target/x.jsonl"));
        assert_eq!(args.atlas, PathBuf::from("a.json"));
        assert_eq!(args.seeds, 2);
        assert_eq!(args.configs.as_ref().map(|c| c.len()), Some(2));
        assert_eq!(args.run_timeout, Duration::from_secs(7));
        assert!(args.summary_only);
    }

    fn atlas_cell(index: usize) -> (WorldParameters, InitialDistribution) {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../atlas.json");
        let atlas: AtlasFile =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        decode(&atlas.cells[index].unit, &default_ranges())
    }

    #[test]
    fn a_rollout_yields_both_reads_and_the_visibly_alive_facts_at_the_horizon() {
        let (params, dist) = atlas_cell(0);
        let record = run_seed(&params, &dist, 1000, 40, Duration::MAX);
        assert_eq!(record.seed, 1000);
        assert!(record.termination_tick >= 1 && record.termination_tick <= 40);
        assert_eq!(
            record.reached_horizon,
            record.termination_tick == 40 && record.mode != "timeout"
        );
        assert!(record.sustainable_stock > 0.0);
        assert!(
            (record.stock_ratio - f64::from(record.window_peak_stock / record.sustainable_stock))
                .abs()
                < 1e-6
        );
        assert_eq!(
            record.visibly_alive,
            record.reached_horizon
                && record.population_at_horizon >= ROSTER_FLOOR
                && record.settled_births > 0
        );
        // Determinism: the same seed reproduces the record byte for byte.
        let again = run_seed(&params, &dist, 1000, 40, Duration::MAX);
        assert_eq!(
            serde_json::to_string(&record).unwrap(),
            serde_json::to_string(&again).unwrap()
        );
    }

    #[test]
    fn a_rollout_past_its_wall_clock_budget_stops_and_reads_as_a_timeout() {
        let (params, dist) = atlas_cell(0);
        let record = run_seed(&params, &dist, 1000, 40, Duration::ZERO);
        assert_eq!(record.mode, "timeout");
        assert!(!record.reached_horizon);
        assert!(!record.visibly_alive);
        let row = ConfigRow {
            source: ConfigSource::Atlas,
            config_index: 0,
            horizon: 40,
            seeds: vec![record],
        };
        let summary = summarise(&[row]);
        assert_eq!(summary.atlas.reached_horizon, 0);
        assert_eq!(summary.atlas.modes, vec![("timeout".to_string(), 1)]);
    }

    #[test]
    fn resume_skips_configs_already_in_the_output_and_limit_caps_the_rest() {
        let dir = std::env::temp_dir().join(format!("energy-death-check-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("out.jsonl");
        assert!(done_configs(&out).is_empty());
        std::fs::write(
            &out,
            "{\"source\":\"atlas\",\"config_index\":0,\"seeds\":[]}\n{\"source\":\"sample\",\"config_index\":0,\"seeds\":[]}\n",
        )
        .unwrap();
        let done = done_configs(&out);
        assert_eq!(done.len(), 2);
        assert_eq!(
            plan_tasks(2, 2, None, &done, None),
            vec![(ConfigSource::Atlas, 1), (ConfigSource::Sample, 1)]
        );
        assert_eq!(
            plan_tasks(2, 2, None, &done, Some(1)),
            vec![(ConfigSource::Atlas, 1)]
        );
        let filter: HashSet<_> = [(ConfigSource::Sample, 1)].into_iter().collect();
        assert_eq!(
            plan_tasks(2, 2, Some(&filter), &done, None),
            vec![(ConfigSource::Sample, 1)]
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
