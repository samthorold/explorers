//! Empirical check of the #433 energy-bound derivation against real genesis runs
//! (issue #438, formal viability B2).
//!
//! ## What it checks
//!
//! `docs/research/433-energy-bound.md` (B1) proved two things and disproved a
//! third. This instrument runs the atlas live cells plus a low-discrepancy sample
//! of the search box through the genesis step loop and, per run, measures:
//!
//! 1. **Lemma 1** — per-tick solar income `P(t) ≤ F·min(N_P(t), m²)` with
//!    `m = ⌊√2·L/r⌋ + 1`. Checked every tick; the run's `lemma1_max_ratio` is the
//!    tightest tick.
//! 2. **The Theorem** — `B·Σ_{t<T} N_s(t) ≤ E_tot(0) + T·P_max` for every prefix
//!    `T` (the transient term is kept, so this is exactly the proven statement at
//!    finite horizon), plus the summed Lemma 2 with *realised* income
//!    `B·Σ N_s ≤ E_tot(0) − E_tot(T) + Σ P(t)` — tighter, and the form a second
//!    energy tap would trip.
//! 3. **The obstruction** — `max_t E_living(t)` against the naive cumulative-solar
//!    envelope `E_tot(0) + t·P_max`, and the share of that peak sitting in the
//!    reproductive allocation (the stock B1 shows is untaxed and uncapped). This
//!    is a *measurement*, not a bound: B1 proved no `E_max` exists.
//!
//! A violation of (1) or (2) is a stepper bug or a proof error — it is reported
//! with the `(source, config_index, seed)` needed to reproduce it and must be
//! filed, never softened. Runs that hit a NaN are recorded separately with the
//! `negative_founder_traits` flag, since `World::new` can seed a negative founder
//! trait under a non-integer maintenance exponent (#444) and NaN-poison the run;
//! those are excluded from the distributions and counted against #444.
//!
//! ## What it does NOT do
//!
//! No change to the stepper, evaluator, or search. No assertion on emergent
//! values beyond the proven bounds. Not a CI gate — run it explicitly.
//!
//! ## Determinism and sourcing
//!
//! Configs: the atlas live-cell `unit` vectors (decoded over the atlas's own
//! search box, exactly as the search evaluated them — #559) plus the same fixed-seed
//! LHS draw `role_emergence.rs` uses (`SAMPLE_SEED = 421`, 200 points), so
//! `sample:i` here is the same config as `sample:i` there. Seeds are a fixed
//! contiguous block per config. Each (config, seed) run is independent, so the
//! rayon collect is order-stable and the artifact is byte-identical across runs.
//!
//! ## Resumable by construction
//!
//! One JSON line per config appended to `--out` (default
//! `target/energy-bound-check.jsonl`); configs already present are skipped;
//! `--limit N` runs at most `N` further configs. Fixed order and seed block,
//! so a loop of short foreground calls produces a file byte-identical to one
//! uninterrupted run (`explorers_search::sweep`). Subset selectors:
//! `--configs atlas:0,sample:12` and `--seeds 2`; `sample@S:i` names a config
//! of the seed-`S` LHS draw (`config_source`).
//!
//! A (config, seed) run's step loop has a wall-clock budget, the
//! **simulation budget** (`--run-timeout-secs`, default 300): a knife-edge
//! world that costs seconds per tick stops where it is and is recorded as
//! mode `timeout`. Its ticks are not read — the summary excludes it from the
//! distributions and the violation list and counts it separately. This bin
//! takes no terminal evaluation (its checks are per-tick reads), so the
//! sweeps' **evaluation budget** (`--eval-timeout-secs`, #523) bounds nothing
//! here: the flag is accepted so every sweep bin shares one command-line
//! shape, and no run here records `eval_timeout`. The summary still treats
//! that mode as unfinished and counts it apart, so a row carrying it can
//! never be read as a checked run.
//!
//! ## Running
//!
//!   cargo build --release -p explorers-search --bin energy_bound_check
//!   ./target/release/energy_bound_check --limit 5     # repeat until 0 run
//!   ./target/release/energy_bound_check --summary
//!
//! Full run: 282 configs × 8 seeds × the search horizon (`SearchConfig::max_ticks`,
//! 2000 ticks since #507); hours of sim time, hence the chunked shape.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use rayon::prelude::*;

use explorers_genesis::EvalConfig;
use explorers_search::config_source::{
    ConfigSource, parse_selector, resolve_config, sampled_units,
};
use explorers_search::search::{SearchConfig, default_ranges};
use explorers_search::sweep::{
    AtlasUnits, DEFAULT_EVAL_TIMEOUT_SECS, EVAL_TIMEOUT_FLAG, EVAL_TIMEOUT_MODE, TIMEOUT_MODE,
    append_row, done_configs, is_unfinished, plan_tasks, read_atlas_units, read_rows,
};
use explorers_sim::{InitialDistribution, World, WorldParameters};

/// Fixed contiguous seed block per config (the `role_emergence` convention).
const N_SEEDS: u64 = 8;
const SEED_BASE: u64 = 1000;

/// The config-only quantities of `docs/research/433-energy-bound.md`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Bounds {
    /// Cells per axis in Lemma 1's tiling: `m = ⌊√2·L/r⌋ + 1`.
    m: u32,
    /// Lemma 1's config-only per-tick solar cap `P_max = F·m²`.
    p_max: f64,
    /// The Theorem's long-run mean survivor ceiling `N̄_max = F·m²/B`.
    n_mean_max: f64,
}

fn bounds(params: &WorldParameters) -> Bounds {
    let l = f64::from(params.world_extent);
    let r = f64::from(params.light_competition_radius);
    let f = f64::from(params.solar_flux_magnitude);
    let b = f64::from(params.base_metabolic_rate);
    let m = (std::f64::consts::SQRT_2 * l / r).floor() as u32 + 1;
    let p_max = f * f64::from(m * m);
    Bounds {
        m,
        p_max,
        n_mean_max: p_max / b,
    }
}

/// What one tick of a run contributes to the checks. `solar_income`,
/// `producers_at_start` and `survivors` are the tick's own `P(t)`, `N_P(t)` and
/// `N_s(t)`; the `*_after` stocks are read at the tick's end, i.e. at `t+1`.
#[derive(Clone, Copy, Debug, Default)]
struct TickSample {
    solar_income: f64,
    producers_at_start: usize,
    survivors: usize,
    e_living_after: f64,
    e_alloc_after: f64,
    e_tot_after: f64,
}

/// The per-run results of the three checks.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
struct RunChecks {
    /// Lemma 1: `max_t P(t) / (F·min(N_P(t), m²))` over ticks with a positive cap.
    lemma1_max_ratio: f64,
    /// Ticks on which `P(t)` exceeded the Lemma 1 cap beyond `REL_TOLERANCE`.
    lemma1_violations: usize,
    /// Theorem (i): `max_T  B·Σ_{t<T} N_s(t) / (E_tot(0) + T·P_max)`.
    theorem_max_ratio: f64,
    /// Ticks (prefix lengths `T`) on which the Theorem's inequality failed.
    theorem_violations: usize,
    /// Lemma 2 summed with the *realised* income (the first inequality of the
    /// Theorem's proof): `max_T  B·Σ_{t<T} N_s(t) / (E_tot(0) − E_tot(T) + Σ_{t<T} P(t))`.
    /// Tighter than the Theorem's config-only form, and the one a second energy
    /// tap (#444's O3) would trip. A non-positive denominator with a positive
    /// numerator is reported as `+∞`.
    lemma2_max_ratio: f64,
    lemma2_violations: usize,
    /// Tightness of the asymptotic ceiling: the whole-run mean of `N_s(t)` over
    /// `N̄_max = F·m²/B`. Not a bound at finite `T` (the transient term is
    /// dropped); reported so the slack of `N̄_max` is visible.
    mean_survivors_over_n_max: f64,
    /// Obstruction measure: `max_t E_living(t)`, the tick it occurs at (1-based,
    /// after that tick's step), and that peak over the naive cumulative-solar
    /// envelope `E_tot(0) + t·P_max`.
    peak_e_living: f64,
    peak_tick: u64,
    envelope_ratio_at_peak: f64,
    /// Share of the peak `E_living` held in the reproductive allocation — the
    /// stock B1 identifies as untaxed and uncapped (O1).
    alloc_share_at_peak: f64,
}

/// Relative slack for f32 summation of per-tick flows (matches
/// `prop_energy_bound.rs`).
const REL_TOLERANCE: f64 = 1e-4;

fn check_run(bounds: &Bounds, b: f64, e_tot0: f64, ticks: &[TickSample]) -> RunChecks {
    let mut out = RunChecks::default();
    let cells = f64::from(bounds.m * bounds.m);
    let f = bounds.p_max / cells;
    let mut cum_survivors = 0.0_f64;
    let mut cum_solar = 0.0_f64;
    for (i, s) in ticks.iter().enumerate() {
        let t_after = i as u64 + 1;
        // Lemma 1.
        let cap = f * (s.producers_at_start as f64).min(cells);
        if cap > 0.0 {
            let ratio = s.solar_income / cap;
            out.lemma1_max_ratio = out.lemma1_max_ratio.max(ratio);
            if ratio > 1.0 + REL_TOLERANCE {
                out.lemma1_violations += 1;
            }
        } else if s.solar_income > 0.0 {
            out.lemma1_max_ratio = f64::INFINITY;
            out.lemma1_violations += 1;
        }
        // Theorem (i), at every prefix length T = i + 1.
        cum_survivors += s.survivors as f64;
        let theorem_bound = e_tot0 + t_after as f64 * bounds.p_max;
        let ratio = b * cum_survivors / theorem_bound;
        out.theorem_max_ratio = out.theorem_max_ratio.max(ratio);
        if ratio > 1.0 + REL_TOLERANCE {
            out.theorem_violations += 1;
        }
        // Lemma 2 summed against realised income.
        cum_solar += s.solar_income;
        let lemma2_bound = e_tot0 - s.e_tot_after + cum_solar;
        let paid = b * cum_survivors;
        let ratio = if lemma2_bound > 0.0 {
            paid / lemma2_bound
        } else if paid > 0.0 {
            f64::INFINITY
        } else {
            0.0
        };
        out.lemma2_max_ratio = out.lemma2_max_ratio.max(ratio);
        // Slack scales with the gross throughput, not the (possibly tiny) net
        // denominator — the same convention as `prop_energy_bound.rs`.
        let slack = (e_tot0 + cum_solar).abs().max(1.0) * REL_TOLERANCE;
        if paid > lemma2_bound + slack {
            out.lemma2_violations += 1;
        }
        // Obstruction: peak living energy and its envelope ratio.
        if s.e_living_after > out.peak_e_living {
            out.peak_e_living = s.e_living_after;
            out.peak_tick = t_after;
            out.envelope_ratio_at_peak =
                s.e_living_after / (e_tot0 + t_after as f64 * bounds.p_max);
            out.alloc_share_at_peak = if s.e_living_after > 0.0 {
                s.e_alloc_after / s.e_living_after
            } else {
                0.0
            };
        }
    }
    if !ticks.is_empty() {
        out.mean_survivors_over_n_max = cum_survivors / ticks.len() as f64 / bounds.n_mean_max;
    }
    out
}

/// Five-number summary of a set of observed/bound ratios.
#[derive(Clone, Copy, Debug, serde::Serialize)]
struct Distribution {
    n: usize,
    min: f64,
    q1: f64,
    median: f64,
    q3: f64,
    max: f64,
}

/// Linear-interpolated percentile of a sorted, non-empty slice (`q` in [0,1]).
fn percentile(sorted: &[f64], q: f64) -> f64 {
    let rank = q * (sorted.len() - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    let frac = rank - lo as f64;
    sorted[lo] + frac * (sorted[hi] - sorted[lo])
}

fn distribution(values: &[f64]) -> Option<Distribution> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    Some(Distribution {
        n: sorted.len(),
        min: sorted[0],
        q1: percentile(&sorted, 0.25),
        median: percentile(&sorted, 0.5),
        q3: percentile(&sorted, 0.75),
        max: sorted[sorted.len() - 1],
    })
}

/// One (config × seed) run: the run's terminal state and the three checks.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct SeedRecord {
    seed: u64,
    /// `E_tot(0)`: the endowment booked at world creation.
    e_tot0: f64,
    /// Ticks actually run (`== horizon` unless terminated early).
    ran_ticks: u64,
    /// `extinction`, `explosion`, `survived` (mirrors `run_single`'s stops),
    /// or `timeout` when the simulation budget ran out.
    terminal_mode: String,
    peak_population: usize,
    /// Founders seeded by `World::new`, and how many of them carry a negative
    /// *metabolic* trait (photosynthetic_absorption, heterotrophy, mobility,
    /// asexual_propensity — the four `powf`'d in `metabolise`; #444). Under the
    /// search box's non-integer exponent those founders' reserve reads NaN after
    /// their first metabolise and `World::step`'s final `retain(reserve > 0)`
    /// silently drops them at tick 1 — no `Died` event, no carcass — so the run
    /// proceeds on the non-negative trait domain from tick 1 with `E_tot`
    /// *reduced* by their endowment (a leak in the safe direction for every
    /// bound here).
    founders: usize,
    negative_founders: usize,
    /// `negative_founders > 0`.
    negative_founder_traits: bool,
    /// First tick at which solar income or a living stock read NaN; the checks
    /// stop there. Attributable to #444 whenever `negative_founder_traits`.
    nan_tick: Option<u64>,
    #[serde(flatten)]
    checks: RunChecks,
}

/// One config: one JSON line in the output — the config-only bounds and the
/// seed ensemble.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct ConfigRow {
    source: ConfigSource,
    config_index: usize,
    horizon: u64,
    /// Config-only inputs to the bounds (decoded `WorldParameters`).
    solar_flux_magnitude: f32,
    world_extent: f32,
    light_competition_radius: f32,
    base_metabolic_rate: f32,
    maintenance_cost_exponent: f32,
    m: u32,
    p_max: f64,
    n_mean_max: f64,
    seeds: Vec<SeedRecord>,
}

fn living_energy(world: &World) -> (f64, f64) {
    let mut living = 0.0_f64;
    let mut alloc = 0.0_f64;
    for a in world.agents() {
        living += f64::from(a.energy());
        alloc += f64::from(a.repro_reserve);
    }
    (living, alloc)
}

fn carcass_energy(world: &World) -> f64 {
    world.carcasses().iter().map(|c| f64::from(c.energy)).sum()
}

fn negative_metabolic_founders(world: &World) -> usize {
    world
        .agents()
        .iter()
        .filter(|a| {
            let t = &a.traits;
            t.photosynthetic_absorption < 0.0
                || t.heterotrophy < 0.0
                || t.mobility < 0.0
                || t.asexual_propensity < 0.0
        })
        .count()
}

/// Drive one (config, seed) through the genesis step loop to the search horizon
/// with `run_single`'s early stops, sampling the per-tick facts the checks need.
/// A run past `run_timeout` of wall clock stops where it is as mode `timeout`.
fn run_seed(
    params: &WorldParameters,
    dist: &InitialDistribution,
    seed: u64,
    horizon: u64,
    run_timeout: Duration,
) -> SeedRecord {
    let b = bounds(params);
    let base_rate = f64::from(params.base_metabolic_rate);
    let max_pop = EvalConfig::default().max_population;
    let started = Instant::now();
    let mut world = World::new(params.clone(), dist.clone(), seed);
    // The checks read agents, carcasses and the per-tick ledger, never the
    // event log; keep none of it, so a dense world at the settled horizon
    // fits in memory (observer-side — no trajectory changes).
    world.retain_event_kinds(&[]);
    let founders = world.agents().len();
    let negative_founders = negative_metabolic_founders(&world);
    let (living0, _) = living_energy(&world);
    let e_tot0 = living0 + carcass_energy(&world);

    let mut ticks: Vec<TickSample> = Vec::with_capacity(horizon as usize);
    let mut ran_ticks = 0u64;
    let mut terminal_mode = "survived";
    let mut peak_population = world.agents().len();
    let mut nan_tick = None;
    let mut before_ids: HashSet<u64> = HashSet::new();

    for _ in 0..horizon {
        let producers_at_start = world
            .agents()
            .iter()
            .filter(|a| a.traits.photosynthetic_absorption > 0.0 && a.structure > 0.0)
            .count();
        before_ids.clear();
        before_ids.extend(world.agents().iter().map(|a| a.id));

        world.step();
        ran_ticks += 1;
        peak_population = peak_population.max(world.agents().len());

        // The tick's own `P(t)`: the energy ledger is rebuilt every tick from
        // that tick's `Photosynthesized` events. (Differencing the world's
        // cumulative f32 `total_solar_input` instead loses the tick to the
        // counter's ULP once the counter is large — at the 2000-tick horizon
        // that read exceeded the cap by 1e-4 on a run the ledger reads at
        // exactly 1.0; #509.)
        let solar_income = f64::from(world.energy_ledger().total_solar_input());
        let survivors = world
            .agents()
            .iter()
            .filter(|a| before_ids.contains(&a.id))
            .count();
        let (e_living_after, e_alloc_after) = living_energy(&world);
        let e_tot_after = e_living_after + carcass_energy(&world);
        if solar_income.is_nan() || e_living_after.is_nan() || e_tot_after.is_nan() {
            nan_tick = Some(ran_ticks);
            break;
        }
        ticks.push(TickSample {
            solar_income,
            producers_at_start,
            survivors,
            e_living_after,
            e_alloc_after,
            e_tot_after,
        });

        if world.agents().is_empty() {
            terminal_mode = "extinction";
            break;
        }
        if world.agents().len() > max_pop {
            terminal_mode = "explosion";
            break;
        }
        if started.elapsed() > run_timeout {
            terminal_mode = TIMEOUT_MODE;
            break;
        }
    }

    let checks = check_run(&b, base_rate, e_tot0, &ticks);
    SeedRecord {
        seed,
        e_tot0,
        ran_ticks,
        terminal_mode: terminal_mode.to_string(),
        peak_population,
        founders,
        negative_founders,
        negative_founder_traits: negative_founders > 0,
        nan_tick,
        checks,
    }
}

/// One config: the seed ensemble run in parallel (rayon's indexed collect
/// keeps seed order, so the row is bit-identical to the sequential map).
fn run_config(
    source: ConfigSource,
    config_index: usize,
    (params, dist): &(WorldParameters, InitialDistribution),
    seeds: u64,
    horizon: u64,
    run_timeout: Duration,
) -> ConfigRow {
    let b = bounds(params);
    let seeds: Vec<SeedRecord> = (0..seeds)
        .into_par_iter()
        .map(|s| run_seed(params, dist, SEED_BASE + s, horizon, run_timeout))
        .collect();
    ConfigRow {
        source,
        config_index,
        horizon,
        solar_flux_magnitude: params.solar_flux_magnitude,
        world_extent: params.world_extent,
        light_competition_radius: params.light_competition_radius,
        base_metabolic_rate: params.base_metabolic_rate,
        maintenance_cost_exponent: params.maintenance_cost_exponent,
        m: b.m,
        p_max: b.p_max,
        n_mean_max: b.n_mean_max,
        seeds,
    }
}

/// A run that broke a proven bound, with what is needed to reproduce it.
#[derive(Clone, Debug, serde::Serialize)]
struct Violation {
    source: ConfigSource,
    config_index: usize,
    seed: u64,
    lemma1_violations: usize,
    theorem_violations: usize,
    lemma2_violations: usize,
    negative_founder_traits: bool,
}

#[derive(serde::Serialize)]
struct Summary {
    /// The horizon(s) the rows were run at (one, unless files were mixed).
    horizons: Vec<u64>,
    atlas_configs: usize,
    sampled_configs: usize,
    total_runs: usize,
    runs_survived: usize,
    runs_extinct: usize,
    runs_exploded: usize,
    /// Runs stopped by the simulation budget: excluded from every
    /// distribution and from the violation list, whatever their ticks read.
    runs_timed_out: usize,
    /// Runs recorded under the evaluation budget's mode (`eval_timeout`):
    /// excluded as the timeouts are, counted apart from them.
    runs_eval_timed_out: usize,
    /// Runs that hit a NaN and were excluded from the distributions, and how
    /// many of those had a negative founder trait (#444).
    runs_nan: usize,
    runs_nan_with_negative_founder: usize,
    /// Runs with ≥1 negative-metabolic-trait founder (#444) that stayed finite
    /// (the founder is culled at tick 1, see `RunRecord::founders`), and the
    /// total founders so culled over all founders seeded.
    runs_with_negative_founders: usize,
    founders_total: usize,
    negative_founders_total: usize,
    /// Runs the distributions are over: finite (no NaN) and not timed out.
    runs_checked: usize,
    violations: Vec<Violation>,
    /// Check 1 — Lemma 1 tightness: `max_t P(t) / (F·min(N_P, m²))`.
    lemma1_max_ratio: Option<Distribution>,
    /// Check 2 — Theorem tightness: `max_T B·ΣN_s / (E_tot(0) + T·P_max)`.
    theorem_max_ratio: Option<Distribution>,
    /// Check 2 — summed Lemma 2 with realised income.
    lemma2_max_ratio: Option<Distribution>,
    /// Check 2 — whole-run mean survivors over the asymptotic `N̄_max`.
    mean_survivors_over_n_max: Option<Distribution>,
    /// Check 3 — `max_t E_living(t)` over the naive envelope at that tick.
    envelope_ratio_at_peak: Option<Distribution>,
    /// Check 3 — allocation share of `E_living` at its peak.
    alloc_share_at_peak: Option<Distribution>,
    /// Check 3 — `max_t E_living(t) / E_tot(0)`: how far living energy grows
    /// past the endowment.
    peak_over_endowment: Option<Distribution>,
    /// Check 3 restricted to runs that reached the horizon.
    envelope_ratio_at_peak_survivors: Option<Distribution>,
    alloc_share_at_peak_survivors: Option<Distribution>,
}

/// Command line: `--limit N`, `--horizon T`, `--out PATH`, `--atlas PATH`,
/// `--seeds N` (1..=8), `--configs atlas:0,sample:12`,
/// `--run-timeout-secs N` (simulation budget), `--eval-timeout-secs N`
/// (accepted for the shared sweep shape; this bin takes no evaluation),
/// `--summary`.
#[derive(Clone, Debug, PartialEq)]
struct Args {
    limit: Option<usize>,
    horizon: u64,
    out: PathBuf,
    atlas: PathBuf,
    seeds: u64,
    configs: Option<HashSet<(ConfigSource, usize)>>,
    run_timeout: Duration,
    /// Parsed for the shared sweep command line; unread, since this bin
    /// takes no terminal evaluation (see the module doc).
    eval_timeout: Duration,
    summary_only: bool,
}

const DEFAULT_RUN_TIMEOUT_SECS: u64 = 300;
const DEFAULT_OUT: &str = "target/energy-bound-check.jsonl";

fn parse_args<I: IntoIterator<Item = String>>(argv: I) -> Args {
    let mut args = Args {
        limit: None,
        horizon: SearchConfig::default().max_ticks,
        out: PathBuf::from(DEFAULT_OUT),
        atlas: PathBuf::from("atlas.json"),
        seeds: N_SEEDS,
        configs: None,
        run_timeout: Duration::from_secs(DEFAULT_RUN_TIMEOUT_SECS),
        eval_timeout: Duration::from_secs(DEFAULT_EVAL_TIMEOUT_SECS),
        summary_only: false,
    };
    let mut it = argv.into_iter();
    let value = |flag: &str, it: &mut I::IntoIter| -> String {
        it.next()
            .unwrap_or_else(|| panic!("energy_bound_check: {flag} needs a value"))
    };
    let number = |flag: &str, raw: &str| -> u64 {
        raw.parse()
            .unwrap_or_else(|_| panic!("energy_bound_check: {flag} {raw:?} is not an integer"))
    };
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--limit" => args.limit = Some(number("--limit", &value("--limit", &mut it)) as usize),
            "--horizon" => {
                args.horizon = number("--horizon", &value("--horizon", &mut it));
                assert!(
                    args.horizon > 0,
                    "energy_bound_check: --horizon must be positive"
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
            EVAL_TIMEOUT_FLAG => {
                args.eval_timeout = Duration::from_secs(number(
                    EVAL_TIMEOUT_FLAG,
                    &value(EVAL_TIMEOUT_FLAG, &mut it),
                ))
            }
            "--summary" => args.summary_only = true,
            other => panic!("energy_bound_check: unknown argument {other:?}"),
        }
    }
    args
}

/// Run the configs not yet in `args.out` (in sweep order, up to `args.limit`),
/// appending one row each as it completes. Returns how many were run.
fn sweep(args: &Args, atlas_units: &AtlasUnits, sampled: &[Vec<f64>]) -> usize {
    let done = done_configs(&args.out);
    let tasks = plan_tasks(
        atlas_units.len(),
        sampled.len(),
        args.configs.as_ref(),
        &done,
        args.limit,
    );
    eprintln!(
        "energy_bound_check: {} atlas + {} sampled configs × {} seeds, horizon {} ticks; {} done in {}, running {} now",
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
    for (n, (source, idx)) in tasks.iter().copied().enumerate() {
        let world = resolve_config(source, idx, atlas_units, sampled);
        let row = run_config(
            source,
            idx,
            &world,
            args.seeds,
            args.horizon,
            args.run_timeout,
        );
        append_row(&args.out, &row);
        let violating = row
            .seeds
            .iter()
            .filter(|s| !is_unfinished(&s.terminal_mode) && s.nan_tick.is_none())
            .filter(|s| {
                s.checks.lemma1_violations
                    + s.checks.theorem_violations
                    + s.checks.lemma2_violations
                    > 0
            })
            .count();
        eprintln!(
            "  {}:{idx} done ({}/{total}, {} survived, {} timed out, {} violating, {:.0}s elapsed)",
            source,
            n + 1,
            row.seeds
                .iter()
                .filter(|s| s.terminal_mode == "survived")
                .count(),
            row.seeds
                .iter()
                .filter(|s| s.terminal_mode == TIMEOUT_MODE)
                .count(),
            violating,
            start.elapsed().as_secs_f64()
        );
    }
    eprintln!(
        "energy_bound_check: {total} configs run in {:.0}s; appended to {}",
        start.elapsed().as_secs_f64(),
        args.out.display()
    );
    total
}

fn main() {
    let args = parse_args(std::env::args().skip(1));
    if !args.summary_only {
        let atlas_units = read_atlas_units(&args.atlas);
        let sampled = sampled_units(default_ranges().len());
        sweep(&args, &atlas_units, &sampled);
    }
    let rows: Vec<ConfigRow> = read_rows(&args.out);
    eprintln!(
        "energy_bound_check: summarising {} rows from {}",
        rows.len(),
        args.out.display()
    );
    print_summary(&summarise(&rows));
}

fn summarise(rows: &[ConfigRow]) -> Summary {
    let mut horizons: Vec<u64> = rows.iter().map(|r| r.horizon).collect();
    horizons.sort_unstable();
    horizons.dedup();
    let runs: Vec<(&ConfigRow, &SeedRecord)> = rows
        .iter()
        .flat_map(|r| r.seeds.iter().map(move |s| (r, s)))
        .collect();
    let mode = |m: &str| runs.iter().filter(|(_, s)| s.terminal_mode == m).count();
    let checked: Vec<(&ConfigRow, &SeedRecord)> = runs
        .iter()
        .copied()
        .filter(|(_, s)| s.nan_tick.is_none() && !is_unfinished(&s.terminal_mode))
        .collect();
    let survivors: Vec<(&ConfigRow, &SeedRecord)> = checked
        .iter()
        .copied()
        .filter(|(_, s)| s.terminal_mode == "survived")
        .collect();
    let dist =
        |rs: &[(&ConfigRow, &SeedRecord)], f: fn(&RunChecks) -> f64| -> Option<Distribution> {
            let v: Vec<f64> = rs.iter().map(|(_, s)| f(&s.checks)).collect();
            distribution(&v)
        };
    let violations = checked
        .iter()
        .filter(|(_, s)| {
            s.checks.lemma1_violations + s.checks.theorem_violations + s.checks.lemma2_violations
                > 0
        })
        .map(|(r, s)| Violation {
            source: r.source,
            config_index: r.config_index,
            seed: s.seed,
            lemma1_violations: s.checks.lemma1_violations,
            theorem_violations: s.checks.theorem_violations,
            lemma2_violations: s.checks.lemma2_violations,
            negative_founder_traits: s.negative_founder_traits,
        })
        .collect();
    Summary {
        horizons,
        atlas_configs: rows
            .iter()
            .filter(|r| r.source == ConfigSource::Atlas)
            .count(),
        sampled_configs: rows.iter().filter(|r| r.source.is_sample()).count(),
        total_runs: runs.len(),
        runs_survived: mode("survived"),
        runs_extinct: mode("extinction"),
        runs_exploded: mode("explosion"),
        runs_timed_out: mode(TIMEOUT_MODE),
        runs_eval_timed_out: mode(EVAL_TIMEOUT_MODE),
        runs_nan: runs.iter().filter(|(_, s)| s.nan_tick.is_some()).count(),
        runs_nan_with_negative_founder: runs
            .iter()
            .filter(|(_, s)| s.nan_tick.is_some() && s.negative_founder_traits)
            .count(),
        runs_with_negative_founders: checked
            .iter()
            .filter(|(_, s)| s.negative_founder_traits)
            .count(),
        founders_total: runs.iter().map(|(_, s)| s.founders).sum(),
        negative_founders_total: runs.iter().map(|(_, s)| s.negative_founders).sum(),
        runs_checked: checked.len(),
        violations,
        lemma1_max_ratio: dist(&checked, |c| c.lemma1_max_ratio),
        theorem_max_ratio: dist(&checked, |c| c.theorem_max_ratio),
        lemma2_max_ratio: dist(&checked, |c| c.lemma2_max_ratio),
        mean_survivors_over_n_max: dist(&checked, |c| c.mean_survivors_over_n_max),
        envelope_ratio_at_peak: dist(&checked, |c| c.envelope_ratio_at_peak),
        alloc_share_at_peak: dist(&checked, |c| c.alloc_share_at_peak),
        peak_over_endowment: {
            let v: Vec<f64> = checked
                .iter()
                .filter(|(_, s)| s.e_tot0 > 0.0)
                .map(|(_, s)| s.checks.peak_e_living / s.e_tot0)
                .collect();
            distribution(&v)
        },
        envelope_ratio_at_peak_survivors: dist(&survivors, |c| c.envelope_ratio_at_peak),
        alloc_share_at_peak_survivors: dist(&survivors, |c| c.alloc_share_at_peak),
    }
}

fn fmt_dist(d: &Option<Distribution>) -> String {
    match d {
        None => "—".to_string(),
        Some(d) => format!(
            "n={:<5} min={:<10.4} q1={:<10.4} median={:<10.4} q3={:<10.4} max={:.4}",
            d.n, d.min, d.q1, d.median, d.q3, d.max
        ),
    }
}

fn print_summary(s: &Summary) {
    println!("\n# Energy-bound empirical check (issue #438, against #433)");
    println!(
        "# {} configs ({} atlas + {} sampled) = {} runs, horizon(s) {:?} ticks",
        s.atlas_configs + s.sampled_configs,
        s.atlas_configs,
        s.sampled_configs,
        s.total_runs,
        s.horizons
    );
    println!(
        "# terminal: {} survived, {} extinct, {} exploded, {} timed out, {} eval timed out (both excluded)",
        s.runs_survived, s.runs_extinct, s.runs_exploded, s.runs_timed_out, s.runs_eval_timed_out
    );
    println!(
        "# NaN runs excluded: {} ({} with a negative founder trait, #444); {} runs checked",
        s.runs_nan, s.runs_nan_with_negative_founder, s.runs_checked
    );
    println!(
        "# #444 exposure: {} checked runs seeded a negative-metabolic-trait founder; {} of {} founders culled at tick 1\n",
        s.runs_with_negative_founders, s.negative_founders_total, s.founders_total
    );

    println!("## Violations of a proven bound: {}", s.violations.len());
    for v in &s.violations {
        println!(
            "  {:?}:{} seed {} — lemma1 {} ticks, theorem {} prefixes, lemma2 {} prefixes (negative founder: {})",
            v.source,
            v.config_index,
            v.seed,
            v.lemma1_violations,
            v.theorem_violations,
            v.lemma2_violations,
            v.negative_founder_traits
        );
    }
    println!();
    println!("## Check 1 — Lemma 1, max_t P(t) / (F·min(N_P, m²))");
    println!("  {}", fmt_dist(&s.lemma1_max_ratio));
    println!("## Check 2 — Theorem, max_T B·ΣN_s / (E_tot(0) + T·P_max)");
    println!("  {}", fmt_dist(&s.theorem_max_ratio));
    println!("## Check 2 — Lemma 2 summed, max_T B·ΣN_s / (E_tot(0) − E_tot(T) + ΣP)");
    println!("  {}", fmt_dist(&s.lemma2_max_ratio));
    println!("## Check 2 — mean N_s over the run / N̄_max");
    println!("  {}", fmt_dist(&s.mean_survivors_over_n_max));
    println!("## Check 3 — max_t E_living / (E_tot(0) + t·P_max) at the peak");
    println!("  all:       {}", fmt_dist(&s.envelope_ratio_at_peak));
    println!(
        "  survivors: {}",
        fmt_dist(&s.envelope_ratio_at_peak_survivors)
    );
    println!("## Check 3 — allocation share of E_living at the peak");
    println!("  all:       {}", fmt_dist(&s.alloc_share_at_peak));
    println!(
        "  survivors: {}",
        fmt_dist(&s.alloc_share_at_peak_survivors)
    );
    println!("## Check 3 — max_t E_living / E_tot(0)");
    println!("  {}", fmt_dist(&s.peak_over_endowment));
    println!();
    println!(
        "{}",
        serde_json::to_string_pretty(s).expect("serialise summary")
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use explorers_search::search::{decode, default_ranges};

    /// A search-domain config (unit-cube midpoint) with the four fields the bound
    /// reads pinned to the viable baseline of the B1 note.
    fn baseline() -> WorldParameters {
        let ranges = default_ranges();
        let (params, _) = decode(&vec![0.5; ranges.len()], &ranges);
        WorldParameters {
            world_extent: 100.0,
            light_competition_radius: 8.0,
            solar_flux_magnitude: 10.0,
            base_metabolic_rate: 0.3,
            ..params
        }
    }

    #[test]
    fn bounds_match_the_b1_worked_numbers() {
        // lib.rs default geometry: r = 1000 > L = 100 → one light neighbourhood,
        // so P_max = F and N̄_max = F/B.
        let one_cell = WorldParameters {
            light_competition_radius: 1000.0,
            ..baseline()
        };
        let b = bounds(&one_cell);
        assert_eq!(b.m, 1);
        assert_eq!(b.p_max, 10.0);
        assert!((b.n_mean_max / (10.0 / 0.3) - 1.0).abs() < 1e-6);

        // Viable baseline (L = 100, r = 8, F = 10, B = 0.3): m = 18, P_max = 3240.
        let baseline = baseline();
        let b = bounds(&baseline);
        assert_eq!(b.m, 18);
        assert_eq!(b.p_max, 3240.0);
        assert!((b.n_mean_max / (3240.0 / 0.3) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn check_run_folds_a_tick_series_into_the_three_ratios() {
        // One light cell, F = 10, B = 1 → P_max = 10, N̄_max = 10.
        let bounds = Bounds {
            m: 1,
            p_max: 10.0,
            n_mean_max: 10.0,
        };
        let e_tot0 = 100.0;
        let tick = |solar, producers, survivors, living, alloc, tot| TickSample {
            solar_income: solar,
            producers_at_start: producers,
            survivors,
            e_living_after: living,
            e_alloc_after: alloc,
            e_tot_after: tot,
        };
        let ticks = [
            tick(5.0, 1, 2, 100.0, 20.0, 100.0),
            // Two producers but one cell: cap stays F, income exactly at it.
            tick(10.0, 2, 4, 120.0, 60.0, 130.0),
            // Over the cap: a Lemma 1 violation.
            tick(12.0, 3, 3, 110.0, 60.0, 125.0),
        ];
        let c = check_run(&bounds, 1.0, e_tot0, &ticks);

        assert!((c.lemma1_max_ratio - 1.2).abs() < 1e-12);
        assert_eq!(c.lemma1_violations, 1);

        // Prefix ratios: 2/110, 6/120, 9/130 → the last is the max.
        assert!((c.theorem_max_ratio - 9.0 / 130.0).abs() < 1e-12);
        assert_eq!(c.theorem_violations, 0);
        // Mean survivors 3 over N̄_max 10.
        assert!((c.mean_survivors_over_n_max - 0.3).abs() < 1e-12);

        // Peak living energy 120 after tick 2: envelope 100 + 2·10 = 120.
        assert_eq!(c.peak_e_living, 120.0);
        assert_eq!(c.peak_tick, 2);
        assert!((c.envelope_ratio_at_peak - 1.0).abs() < 1e-12);
        assert!((c.alloc_share_at_peak - 0.5).abs() < 1e-12);
    }

    #[test]
    fn check_run_sums_lemma_2_against_realised_income() {
        let bounds = Bounds {
            m: 1,
            p_max: 10.0,
            n_mean_max: 10.0,
        };
        let tick = |solar, survivors, tot| TickSample {
            solar_income: solar,
            producers_at_start: 1,
            survivors,
            e_living_after: tot,
            e_alloc_after: 0.0,
            e_tot_after: tot,
        };
        // Prefix ratios B·ΣN_s / (E_tot(0) − E_tot(T) + ΣP):
        // 2/(100−100+5) = 0.4, 6/(100−95+15) = 0.3, 9/(100−80+27) ≈ 0.19.
        let ok = [
            tick(5.0, 2, 100.0),
            tick(10.0, 4, 95.0),
            tick(12.0, 3, 80.0),
        ];
        let c = check_run(&bounds, 1.0, 100.0, &ok);
        assert!((c.lemma2_max_ratio - 0.4).abs() < 1e-12);
        assert_eq!(c.lemma2_violations, 0);

        // Energy appearing from nowhere: E_tot rises by more than the income
        // net of the survivors' base charge — the shape a second tap (#444) has.
        let tap = [tick(5.0, 2, 104.0)];
        let c = check_run(&bounds, 1.0, 100.0, &tap);
        assert_eq!(c.lemma2_violations, 1);
        assert!(c.lemma2_max_ratio > 1.0);
    }

    #[test]
    fn distribution_reports_quartiles_and_extremes() {
        let d = distribution(&[4.0, 1.0, 3.0, 2.0, 5.0]).expect("non-empty");
        assert_eq!(d.n, 5);
        assert_eq!(d.min, 1.0);
        assert_eq!(d.q1, 2.0);
        assert_eq!(d.median, 3.0);
        assert_eq!(d.q3, 4.0);
        assert_eq!(d.max, 5.0);
        // Linear interpolation between order statistics.
        let d = distribution(&[10.0, 20.0, 30.0, 40.0]).expect("non-empty");
        assert_eq!(d.median, 25.0);
        assert_eq!(d.q1, 17.5);
        assert!(distribution(&[]).is_none());
    }
}

#[cfg(test)]
mod resume_tests {
    use super::*;
    use explorers_search::search::decode;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("energy-bound-check-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn args_default_to_the_full_sweep_at_the_search_horizon_and_accept_each_flag() {
        let args = parse_args(std::iter::empty());
        assert_eq!(args.limit, None);
        assert_eq!(args.horizon, SearchConfig::default().max_ticks);
        assert_eq!(args.out, PathBuf::from(DEFAULT_OUT));
        assert_eq!(args.seeds, N_SEEDS);
        assert_eq!(args.configs, None);
        assert_eq!(args.run_timeout, Duration::from_secs(300));
        assert_eq!(args.eval_timeout, Duration::from_secs(300));
        assert!(!args.summary_only);
        let args = parse_args(
            "--limit 3 --horizon 600 --out target/x.jsonl --atlas a.json --seeds 2 --configs atlas:0,atlas:1 --run-timeout-secs 7 --eval-timeout-secs 11 --summary"
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
        assert_eq!(args.eval_timeout, Duration::from_secs(11));
        assert!(args.summary_only);
    }

    /// The resumable contract: a sweep split into `--limit 1` calls appends
    /// the same rows, in the same order, as one uninterrupted call.
    #[test]
    fn a_sweep_split_in_two_produces_the_same_file_as_one_run() {
        let dims = default_ranges().len();
        let atlas = AtlasUnits::new(default_ranges(), vec![vec![0.5; dims], vec![0.4; dims]]);
        let sample = vec![vec![0.6; dims]];
        let base = Args {
            limit: None,
            horizon: 20,
            out: tmp("one-shot.jsonl"),
            atlas: PathBuf::new(),
            seeds: 2,
            configs: None,
            run_timeout: Duration::MAX,
            eval_timeout: Duration::MAX,
            summary_only: false,
        };
        assert_eq!(sweep(&base, &atlas, &sample), 3);
        assert_eq!(sweep(&base, &atlas, &sample), 0, "nothing left to run");

        let split = Args {
            limit: Some(2),
            out: tmp("split.jsonl"),
            ..base.clone()
        };
        assert_eq!(sweep(&split, &atlas, &sample), 2);
        assert_eq!(sweep(&split, &atlas, &sample), 1);
        assert_eq!(sweep(&split, &atlas, &sample), 0);

        let one = std::fs::read(&base.out).unwrap();
        let two = std::fs::read(&split.out).unwrap();
        assert!(!one.is_empty());
        assert_eq!(one, two);
        let rows: Vec<ConfigRow> = read_rows(&base.out);
        assert_eq!(
            rows.iter()
                .map(|r| (r.source, r.config_index))
                .collect::<Vec<_>>(),
            vec![
                (ConfigSource::Atlas, 0),
                (ConfigSource::Atlas, 1),
                (ConfigSource::SAMPLE, 0)
            ]
        );
        assert!(rows.iter().all(|r| r.seeds.len() == 2 && r.horizon == 20));
        std::fs::remove_dir_all(base.out.parent().unwrap()).ok();
    }

    #[test]
    fn a_run_past_its_wall_clock_budget_reads_as_a_timeout_and_is_not_checked() {
        let dims = default_ranges().len();
        let unit = vec![0.5; dims];
        let row = run_config(
            ConfigSource::SAMPLE,
            0,
            &decode(&unit, &default_ranges()),
            1,
            20,
            Duration::ZERO,
        );
        assert_eq!(row.seeds[0].terminal_mode, "timeout");
        let summary = summarise(&[row]);
        assert_eq!(summary.runs_timed_out, 1);
        assert_eq!(summary.runs_checked, 0);
        assert!(summary.violations.is_empty());
    }

    #[test]
    fn an_eval_timed_out_run_is_unfinished_counted_apart_and_not_checked() {
        let dims = default_ranges().len();
        let unit = vec![0.5; dims];
        let mut row = run_config(
            ConfigSource::SAMPLE,
            0,
            &decode(&unit, &default_ranges()),
            1,
            20,
            Duration::MAX,
        );
        // A row recorded under the evaluation budget's mode, carrying a
        // violation that would be listed were the run read as finished.
        row.seeds[0].terminal_mode = EVAL_TIMEOUT_MODE.to_string();
        row.seeds[0].checks.lemma1_violations = 1;
        let summary = summarise(&[row]);
        assert_eq!(summary.runs_eval_timed_out, 1);
        assert_eq!(summary.runs_timed_out, 0, "not folded into the timeouts");
        assert_eq!(summary.runs_checked, 0);
        assert!(summary.violations.is_empty());
    }
}
