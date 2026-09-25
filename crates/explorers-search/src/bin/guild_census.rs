//! Guild census (issue #494): the per-cell guild observables the QD atlas
//! reports (#490) — `decomposer_fraction`, `consumer_fraction`, and the
//! coexistence fraction beside them — measured on identical semantics over
//! the atlas's live cells and the 200-point LHS sample of the search box, so
//! the two populations can be compared directly.
//!
//! Each config is decoded (`search::decode` over `default_ranges`), rolled out
//! with `explorers_genesis::run_ensemble` exactly as the QD search rolls out a
//! batch config (`--ensemble` seeds, default 5; horizon `--max-ticks`, default
//! 2000; `EvalConfig::default()`), and reduced with
//! `explorers_search::qd::config_eval_from_ensemble` — the reduction the
//! archive stores on a cell. The early-stop carry is off: it only records a
//! horizon verdict beside a gated seed and never changes the seed's verdict,
//! so the fractions are unaffected.
//!
//! ## Seeds
//!
//! The search seeds config `k` of a run at `base_seed + k·1000`, but the atlas
//! does not record which search config produced a cell's elite, so its exact
//! seed block cannot be recovered. Every config here is instead rolled out on
//! the same block `seed .. seed + ensemble` (`--seed`, default 1000, as the
//! sibling sweeps): an independent draw of the same size, horizon and
//! reduction. An atlas row therefore re-measures a cell's fractions; it
//! reproduces them only up to seed noise at n = 5.
//!
//! ## No wall-clock budget
//!
//! `run_ensemble` carries no simulation or evaluation budget (the sibling
//! sweeps' `--run-timeout-secs` / `--eval-timeout-secs` wrap their own step
//! loops, not the genesis one), so a pathological config runs to its horizon.
//!
//! ## Resumable by construction
//!
//! One JSON line per config appended to `--output` (default
//! `target/guild-census.jsonl`); configs already present are skipped; `--limit
//! N` runs at most `N` further configs, in the fixed sweep order of
//! `explorers_search::sweep` (atlas cells by index, then the sample). Atlas
//! cells are swept only when `--atlas PATH` is given.
//!
//! ## Running
//!
//!   cargo build --release -p explorers-search --bin guild_census
//!   ./target/release/guild_census --limit 5                   # the LHS sample
//!   ./target/release/guild_census --atlas atlas.json --limit 5
//!   ./target/release/guild_census --summary

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

use explorers_genesis::{
    EnsembleConfig, EnsembleResult, EvalConfig, FailureMode, RunConfig, run_ensemble,
};
use explorers_search::config_source::{ConfigSource, parse_selector, resolve_unit, sampled_units};
use explorers_search::qd::{CoexistenceFractions, config_eval_from_ensemble};
use explorers_search::search::{decode, default_ranges};
use explorers_search::sweep::{append_row, done_configs, plan_tasks, read_atlas_units, read_rows};

/// A guild fraction counts as "held" at or above this share of the ensemble.
const HALF: f32 = 0.5;

/// One seed of a config's ensemble.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct SeedGuild {
    seed: u64,
    /// The evaluator's failure mode (the cliff label), or `None` for a live run.
    failure: Option<String>,
    fitness: f32,
    termination_tick: u64,
    decomposer: bool,
    consumer: bool,
    /// In the coexisting regime, as the atlas's plain coexistence fraction
    /// counts it.
    coexisting: bool,
}

/// The coexistence fraction under each guild-aware floor (#538).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct GuildCoexistence {
    decomposer: f32,
    consumer: f32,
    either: f32,
}

/// One config's census row.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct CensusRow {
    source: ConfigSource,
    config_index: usize,
    horizon: u64,
    base_seed: u64,
    sample_count: u32,
    decomposer_fraction: f32,
    consumer_fraction: f32,
    /// The plain coexistence fraction — the atlas cell's `coexistence_fraction`.
    coexistence_fraction: f32,
    guild_coexistence: GuildCoexistence,
    median_fitness: f32,
    /// The representative (median-fitness) seed's cliff, `None` for a live
    /// config — one the atlas would place in a behaviour cell.
    cliff: Option<String>,
    seeds: Vec<SeedGuild>,
}

/// Build a row from an ensemble rolled out on `base_seed..`.
fn census_row(
    source: ConfigSource,
    config_index: usize,
    horizon: u64,
    base_seed: u64,
    result: &EnsembleResult,
) -> CensusRow {
    let eval = config_eval_from_ensemble(result);
    let fractions = CoexistenceFractions::of_seeds(&result.run_results);
    let seeds = result
        .run_results
        .iter()
        .enumerate()
        .map(|(i, r)| SeedGuild {
            seed: base_seed.wrapping_add(i as u64),
            failure: r.failure.as_ref().map(|f| failure_label(f).to_string()),
            fitness: r.fitness,
            termination_tick: r.termination_tick,
            decomposer: r.breakdown.has_decomposer_guild,
            consumer: r.breakdown.has_consumer_guild,
            // The atlas's own per-seed predicate, read through its public
            // fraction over a one-seed ensemble.
            coexisting: CoexistenceFractions::of_seeds(std::slice::from_ref(r)).plain > 0.0,
        })
        .collect();
    CensusRow {
        source,
        config_index,
        horizon,
        base_seed,
        sample_count: eval.sample_count,
        decomposer_fraction: eval.decomposer_fraction,
        consumer_fraction: eval.consumer_fraction,
        coexistence_fraction: eval.coexistence_fraction,
        guild_coexistence: GuildCoexistence {
            decomposer: fractions.decomposer,
            consumer: fractions.consumer,
            either: fractions.either,
        },
        median_fitness: eval.median_fitness,
        cliff: eval.cliff.map(|c| c.label().to_string()),
        seeds,
    }
}

/// A failure mode as the atlas's cliff label (`Cliff::label`).
fn failure_label(f: &FailureMode) -> &'static str {
    match f {
        FailureMode::Extinction => "extinction",
        FailureMode::PopulationExplosion => "population_explosion",
        FailureMode::EnergyDeath => "energy_death",
        FailureMode::NutrientLockup => "nutrient_lockup",
        FailureMode::Monoculture => "monoculture",
        FailureMode::GeneralistDominance => "generalist_dominance",
    }
}

/// A guild predicate and how many of the configs holding it also coexist
/// (`coexistence_fraction >= 0.5`).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
struct Cross {
    holds: usize,
    holds_and_coexisting: usize,
}

/// Counts over one set of rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
struct Tally {
    configs: usize,
    coexisting: usize,
    decomposer_half: Cross,
    consumer_half: Cross,
    either_half: Cross,
    both_half: Cross,
    either_full: Cross,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
struct SourceSummary {
    all: Tally,
    /// Rows whose representative seed is live (`cliff == None`).
    live: Tally,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
struct Summary {
    atlas: SourceSummary,
    sample: SourceSummary,
}

impl Cross {
    fn count(&mut self, holds: bool, coexisting: bool) {
        if holds {
            self.holds += 1;
            if coexisting {
                self.holds_and_coexisting += 1;
            }
        }
    }
}

fn tally<'a>(rows: impl Iterator<Item = &'a CensusRow>) -> Tally {
    let mut t = Tally::default();
    for r in rows {
        let coexisting = r.coexistence_fraction >= HALF;
        let decomposer = r.decomposer_fraction >= HALF;
        let consumer = r.consumer_fraction >= HALF;
        t.configs += 1;
        t.coexisting += usize::from(coexisting);
        t.decomposer_half.count(decomposer, coexisting);
        t.consumer_half.count(consumer, coexisting);
        t.either_half.count(decomposer || consumer, coexisting);
        t.both_half.count(decomposer && consumer, coexisting);
        t.either_full.count(
            r.decomposer_fraction >= 1.0 || r.consumer_fraction >= 1.0,
            coexisting,
        );
    }
    t
}

fn summarise(rows: &[CensusRow]) -> Summary {
    let source = |source: ConfigSource| {
        let of = || {
            rows.iter()
                .filter(move |r| r.source.is_sample() == source.is_sample())
        };
        SourceSummary {
            all: tally(of()),
            live: tally(of().filter(|r| r.cliff.is_none())),
        }
    };
    Summary {
        atlas: source(ConfigSource::Atlas),
        sample: source(ConfigSource::SAMPLE),
    }
}

/// Command line: `--limit N`, `--max-ticks T` (alias `--horizon`),
/// `--ensemble N`, `--seed S`, `--output PATH` (alias `--out`), `--atlas
/// PATH`, `--configs atlas:0,sample:12` (`sample@S:i`: the seed-`S` LHS
/// draw), `--summary`.
#[derive(Clone, Debug, PartialEq)]
struct Args {
    limit: Option<usize>,
    horizon: u64,
    ensemble: u32,
    seed: u64,
    out: PathBuf,
    atlas: Option<PathBuf>,
    configs: Option<HashSet<(ConfigSource, usize)>>,
    summary_only: bool,
}

/// The search's default horizon (`--max-ticks`).
const DEFAULT_HORIZON: u64 = 2000;
/// The search's default ensemble size (`--ensemble`).
const DEFAULT_ENSEMBLE: u32 = 5;
/// The sibling sweeps' seed block base.
const DEFAULT_SEED: u64 = 1000;
const DEFAULT_OUT: &str = "target/guild-census.jsonl";

fn parse_args<I: IntoIterator<Item = String>>(argv: I) -> Args {
    let mut args = Args {
        limit: None,
        horizon: DEFAULT_HORIZON,
        ensemble: DEFAULT_ENSEMBLE,
        seed: DEFAULT_SEED,
        out: PathBuf::from(DEFAULT_OUT),
        atlas: None,
        configs: None,
        summary_only: false,
    };
    let mut it = argv.into_iter();
    let value = |flag: &str, it: &mut I::IntoIter| -> String {
        it.next()
            .unwrap_or_else(|| panic!("guild_census: {flag} needs a value"))
    };
    let number = |flag: &str, raw: &str| -> u64 {
        raw.parse()
            .unwrap_or_else(|_| panic!("guild_census: {flag} {raw:?} is not an integer"))
    };
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--limit" => args.limit = Some(number("--limit", &value("--limit", &mut it)) as usize),
            "--max-ticks" | "--horizon" => {
                args.horizon = number(&flag, &value(&flag, &mut it));
                assert!(args.horizon > 0, "guild_census: {flag} must be positive");
            }
            "--ensemble" => {
                args.ensemble = number("--ensemble", &value("--ensemble", &mut it)) as u32;
                assert!(
                    args.ensemble > 0,
                    "guild_census: --ensemble must be positive"
                );
            }
            "--seed" => args.seed = number("--seed", &value("--seed", &mut it)),
            "--output" | "--out" => args.out = PathBuf::from(value(&flag, &mut it)),
            "--atlas" => args.atlas = Some(PathBuf::from(value("--atlas", &mut it))),
            "--configs" => {
                args.configs = Some(parse_selector(
                    &value("--configs", &mut it),
                    "--configs",
                    None,
                ))
            }
            "--summary" => args.summary_only = true,
            other => panic!("guild_census: unknown argument {other:?}"),
        }
    }
    args
}

fn run_config(source: ConfigSource, config_index: usize, unit: &[f64], args: &Args) -> CensusRow {
    let (params, dist) = decode(unit, &default_ranges());
    let ensemble_config = EnsembleConfig {
        ensemble_size: args.ensemble,
        run_config: RunConfig {
            max_ticks: args.horizon,
            eval_config: EvalConfig::default(),
            early_stop_crosscheck_fraction: 0.0,
        },
    };
    let result = run_ensemble(&params, &dist, &ensemble_config, args.seed);
    census_row(source, config_index, args.horizon, args.seed, &result)
}

fn print_tally(name: &str, t: &Tally) {
    println!(
        "## {name}: {} configs, {} coexisting (coexistence_fraction >= 0.5)",
        t.configs, t.coexisting
    );
    println!(
        "  {:<22} {:>6} {:>12} {:>12} {:>12} {:>12}",
        "predicate", "holds", "hold+coex", "hold+!coex", "!hold+coex", "!hold+!coex"
    );
    for (label, c) in [
        ("decomposer >= 0.5", t.decomposer_half),
        ("consumer >= 0.5", t.consumer_half),
        ("either >= 0.5", t.either_half),
        ("both >= 0.5", t.both_half),
        ("either == 1.0", t.either_full),
    ] {
        let not_holds_coex = t.coexisting - c.holds_and_coexisting;
        println!(
            "  {:<22} {:>6} {:>12} {:>12} {:>12} {:>12}",
            label,
            c.holds,
            c.holds_and_coexisting,
            c.holds - c.holds_and_coexisting,
            not_holds_coex,
            t.configs - c.holds - not_holds_coex
        );
    }
}

fn print_summary(summary: &Summary) {
    println!("\n# Guild census (issue #494)\n");
    print_tally("atlas, all rows", &summary.atlas.all);
    print_tally("atlas, live rows (cliff == null)", &summary.atlas.live);
    print_tally("sample, all rows", &summary.sample.all);
    print_tally("sample, live rows (cliff == null)", &summary.sample.live);
    println!(
        "\n{}",
        serde_json::to_string_pretty(summary).expect("serialise summary")
    );
}

fn main() {
    let args = parse_args(std::env::args().skip(1));
    if !args.summary_only {
        let atlas_units = args
            .atlas
            .as_deref()
            .map(read_atlas_units)
            .unwrap_or_default();
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
            "guild_census: {} atlas + {} sampled configs × {} seeds (base {}), horizon {} ticks; {} done in {}, running {} now",
            atlas_units.len(),
            sampled.len(),
            args.ensemble,
            args.seed,
            args.horizon,
            done.len(),
            args.out.display(),
            tasks.len()
        );
        let start = Instant::now();
        let total = tasks.len();
        for (n, (source, idx)) in tasks.into_iter().enumerate() {
            let unit = resolve_unit(source, idx, &atlas_units, &sampled);
            let config_start = Instant::now();
            let row = run_config(source, idx, &unit, &args);
            append_row(&args.out, &row);
            eprintln!(
                "  {}:{idx} done ({}/{total}): decomposer {:.2} consumer {:.2} coexistence {:.2} cliff {} ({:.1}s; {:.0}s elapsed)",
                source,
                n + 1,
                row.decomposer_fraction,
                row.consumer_fraction,
                row.coexistence_fraction,
                row.cliff.as_deref().unwrap_or("-"),
                config_start.elapsed().as_secs_f64(),
                start.elapsed().as_secs_f64()
            );
        }
        eprintln!(
            "guild_census: {total} configs run in {:.0}s; appended to {}",
            start.elapsed().as_secs_f64(),
            args.out.display()
        );
    }
    let rows: Vec<CensusRow> = read_rows(&args.out);
    eprintln!(
        "guild_census: summarising {} rows from {}",
        rows.len(),
        args.out.display()
    );
    print_summary(&summarise(&rows));
}

#[cfg(test)]
mod tests {
    use super::*;
    use explorers_genesis::{FitnessBreakdown, RunResult};

    fn row(
        source: ConfigSource,
        config_index: usize,
        decomposer: f32,
        consumer: f32,
        coexistence: f32,
        cliff: Option<&str>,
    ) -> CensusRow {
        CensusRow {
            source,
            config_index,
            horizon: 2000,
            base_seed: 1000,
            sample_count: 5,
            decomposer_fraction: decomposer,
            consumer_fraction: consumer,
            coexistence_fraction: coexistence,
            guild_coexistence: GuildCoexistence {
                decomposer: 0.0,
                consumer: 0.0,
                either: 0.0,
            },
            median_fitness: 0.0,
            cliff: cliff.map(String::from),
            seeds: Vec::new(),
        }
    }

    /// A synthetic per-seed result (no sim).
    fn run(
        fitness: f32,
        failure: Option<FailureMode>,
        coexistence_duration: f32,
        decomposer: bool,
        consumer: bool,
    ) -> RunResult {
        RunResult {
            fitness,
            failure: failure.clone(),
            termination_tick: 2000,
            breakdown: FitnessBreakdown {
                fitness,
                failure,
                oscillation_strength: 0.0,
                clustering_strength: 0.0,
                coexistence_duration,
                turnover_score: 0.0,
                trophic_balance_score: 0.0,
                ticks_survived: 2000,
                carcass_locked_fraction: 0.0,
                has_decomposer_guild: decomposer,
                has_consumer_guild: consumer,
            },
            early_stop: None,
        }
    }

    #[test]
    fn a_row_carries_the_atlas_reduction_and_the_per_seed_guild_flags() {
        let result = EnsembleResult {
            median_fitness: 0.0,
            run_results: vec![
                run(0.9, None, 5.0, true, true),
                run(0.1, Some(FailureMode::Monoculture), 0.0, true, false),
                run(0.5, None, 3.0, false, true),
                run(0.7, None, 0.0, true, false),
            ],
        };
        let r = census_row(ConfigSource::SAMPLE, 7, 2000, 1000, &result);
        let eval = config_eval_from_ensemble(&result);
        assert_eq!(r.source, ConfigSource::SAMPLE);
        assert_eq!(r.config_index, 7);
        assert_eq!(r.sample_count, 4);
        assert_eq!(r.decomposer_fraction, eval.decomposer_fraction);
        assert_eq!(r.decomposer_fraction, 0.75);
        assert_eq!(r.consumer_fraction, 0.5);
        assert_eq!(r.coexistence_fraction, 0.5);
        assert_eq!(
            r.guild_coexistence,
            GuildCoexistence {
                decomposer: 0.25,
                consumer: 0.5,
                either: 0.5
            }
        );
        // Lower-middle of the fitness order (0.1, 0.5, 0.7, 0.9) is 0.7: live.
        assert_eq!(r.median_fitness, 0.7);
        assert_eq!(r.cliff, None);
        assert_eq!(r.seeds.len(), 4);
        assert_eq!(
            r.seeds[1],
            SeedGuild {
                seed: 1001,
                failure: Some("monoculture".to_string()),
                fitness: 0.1,
                termination_tick: 2000,
                decomposer: true,
                consumer: false,
                coexisting: false,
            }
        );
        assert!(r.seeds[0].coexisting && r.seeds[2].coexisting && !r.seeds[3].coexisting);
        assert_eq!(r.seeds[3].seed, 1003);
    }

    #[test]
    fn args_default_to_the_search_ensemble_and_accept_each_flag() {
        let args = parse_args(std::iter::empty());
        assert_eq!(args.limit, None);
        assert_eq!(args.horizon, 2000);
        assert_eq!(args.ensemble, 5);
        assert_eq!(args.seed, DEFAULT_SEED);
        assert_eq!(args.out, PathBuf::from(DEFAULT_OUT));
        assert_eq!(args.atlas, None);
        assert_eq!(args.configs, None);
        assert!(!args.summary_only);
        let args = parse_args(
            "--limit 3 --max-ticks 600 --ensemble 8 --seed 7 --output target/x.jsonl --atlas a.json --configs atlas:0,sample:1 --summary"
                .split(' ')
                .map(String::from),
        );
        assert_eq!(args.limit, Some(3));
        assert_eq!(args.horizon, 600);
        assert_eq!(args.ensemble, 8);
        assert_eq!(args.seed, 7);
        assert_eq!(args.out, PathBuf::from("target/x.jsonl"));
        assert_eq!(args.atlas, Some(PathBuf::from("a.json")));
        assert_eq!(args.configs.as_ref().map(|c| c.len()), Some(2));
        assert!(args.summary_only);
    }

    #[test]
    fn summary_counts_guild_thresholds_against_coexistence_per_source_and_liveness() {
        use ConfigSource::{Atlas, Sample};
        let rows = vec![
            // decomposer only, coexisting, live
            row(Atlas, 0, 0.6, 0.0, 0.8, None),
            // both guilds, consumer at 1.0, not coexisting, live
            row(Atlas, 1, 0.5, 1.0, 0.2, None),
            // no guild, coexisting, dead representative
            row(Atlas, 2, 0.4, 0.4, 0.6, Some("monoculture")),
            // decomposer at 1.0, coexisting, dead
            row(Atlas, 3, 1.0, 0.2, 0.5, Some("energy_death")),
            // a sample row (of any draw): consumer only, coexisting, live
            row(Sample(9421), 0, 0.0, 0.6, 1.0, None),
        ];
        let s = summarise(&rows);

        let all = s.atlas.all;
        assert_eq!(all.configs, 4);
        assert_eq!(all.coexisting, 3);
        assert_eq!(
            all.decomposer_half,
            Cross {
                holds: 3,
                holds_and_coexisting: 2
            }
        );
        assert_eq!(
            all.consumer_half,
            Cross {
                holds: 1,
                holds_and_coexisting: 0
            }
        );
        assert_eq!(
            all.either_half,
            Cross {
                holds: 3,
                holds_and_coexisting: 2
            }
        );
        assert_eq!(
            all.both_half,
            Cross {
                holds: 1,
                holds_and_coexisting: 0
            }
        );
        assert_eq!(
            all.either_full,
            Cross {
                holds: 2,
                holds_and_coexisting: 1
            }
        );

        let live = s.atlas.live;
        assert_eq!(live.configs, 2);
        assert_eq!(live.coexisting, 1);
        assert_eq!(live.decomposer_half.holds, 2);
        assert_eq!(live.either_full.holds, 1);
        assert_eq!(live.either_full.holds_and_coexisting, 0);

        assert_eq!(s.sample.all.configs, 1);
        assert_eq!(s.sample.live.consumer_half.holds_and_coexisting, 1);
        assert_eq!(s.sample.all.decomposer_half.holds, 0);
    }
}
