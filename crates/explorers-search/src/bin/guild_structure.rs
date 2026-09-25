//! Guild structure (issue #546): for the LHS seeds that hold a decomposer
//! guild at `T = 2000`, the trait and diet structure that tells a mixotroph
//! monoculture straddling the producer / heterotroph line (reading A) from a
//! real behavioural guild inside a trait monoculture (reading B).
//!
//! Each listed (config, seed) pair ([`GUILD_SEEDS`]) is rolled out on the
//! census's path — `search::decode` over `default_ranges`, horizon 2000,
//! `EvalConfig::default()`, no early-stop carry — through
//! `explorers_genesis::rollout`, which is `run_single` keeping the terminal
//! world and observations. Every read is an evaluator or topology function:
//! the verdict and guild flags are the rollout's own; the monoculture margin
//! is `clustering_strength` against `clustering_threshold`; trophic position
//! is `trophic_coordinates`; roles are `TopologyProjection::trophic_roles_of`
//! (the terminal read the evaluator scores balance with, and the
//! settled-window role snapshots the guild predicate reads); detrital
//! reliance is `TopologyProjection::detrital_reliance`, the quantity the role
//! read cuts at 0.5; the generalist share is `generalist_energy_share`.
//!
//! ## Classification rule
//!
//! Over the terminal decomposers, with "near the line" meaning a
//! heterotrophy share within 0.1 of 0.5:
//!
//! - **A (straddle)** when at least half the decomposers sit near the line;
//! - **B (behavioural guild)** when at most a fifth sit near the line *and*
//!   the decomposers' median detrital reliance is at least 0.75;
//! - **ambiguous** otherwise, or with no terminal decomposers.
//!
//! ## Running
//!
//! Resumable JSON-lines, one row per config, the shared sweep shape:
//!
//!   cargo build --release -p explorers-search --bin guild_structure
//!   ./target/release/guild_structure --limit 3      # repeat until 0 run
//!   ./target/release/guild_structure --summary

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

use explorers_genesis::{Rollout, RunConfig, rollout};
use explorers_search::config_source::{ConfigSource, parse_selector, sampled_units};
use explorers_search::qd::Cliff;
use explorers_search::search::{decode, default_ranges};
use explorers_search::sweep::{append_row, done_configs, plan_tasks, read_rows};
use rayon::prelude::*;

use explorers_genesis::EvalConfig;
use explorers_genesis_eval::guild::RoleSnapshot;
use explorers_genesis_eval::{clustering_strength, generalist_energy_share, trophic_coordinates};
use explorers_sim::TraitVector;
use explorers_sim::topology::TrophicRole;

/// How far from the photo = hetero line (heterotrophy share 0.5) an agent
/// counts as sitting on it.
const NEAR_LINE: f32 = 0.1;
/// Reading A when at least this share of the decomposers sit near the line.
const STRADDLE_SHARE: f32 = 0.5;
/// Reading B needs at most this share of the decomposers near the line…
const CLEAR_SHARE: f32 = 0.2;
/// …and a median detrital reliance at least this far above the 0.5 cut.
const CLEAR_RELIANCE: f32 = 0.75;

/// The reading a seed's decomposer guild supports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum Reading {
    /// (A) A mixotroph monoculture straddling the role boundary.
    Straddle,
    /// (B) A real behavioural guild inside a trait monoculture.
    Guild,
    /// Neither reading's condition holds (or no terminal decomposers).
    Ambiguous,
}

/// Whether a heterotrophy share sits within [`NEAR_LINE`] of 0.5.
fn near_line(share: f32) -> bool {
    (share - 0.5).abs() <= NEAR_LINE
}

/// The classification rule, over the terminal decomposers' heterotrophy
/// shares and detrital reliances.
fn classify(decomposer_hetero_shares: &[f32], decomposer_reliances: &[f32]) -> Reading {
    if decomposer_hetero_shares.is_empty() {
        return Reading::Ambiguous;
    }
    let near = fraction(decomposer_hetero_shares, near_line);
    if near >= STRADDLE_SHARE {
        return Reading::Straddle;
    }
    let reliance = quantile(decomposer_reliances, 0.5).unwrap_or(0.0);
    if near <= CLEAR_SHARE && reliance >= CLEAR_RELIANCE {
        Reading::Guild
    } else {
        Reading::Ambiguous
    }
}

/// The `q`-quantile of `values` (linear interpolation between order
/// statistics); `None` on an empty slice.
fn quantile(values: &[f32], q: f32) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f32::total_cmp);
    let pos = q.clamp(0.0, 1.0) * (sorted.len() - 1) as f32;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    Some(sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo as f32))
}

/// Histogram bins over `[0, 1]`.
const BINS: usize = 10;

/// A summary of a set of values in `[0, 1]` (heterotrophy shares, detrital
/// reliances).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct Distribution {
    n: usize,
    mean: f32,
    min: f32,
    p10: f32,
    p25: f32,
    p50: f32,
    p75: f32,
    p90: f32,
    max: f32,
    /// Share of the values within [`NEAR_LINE`] of 0.5.
    near_line: f32,
    /// Counts over `[0, 0.1), [0.1, 0.2), … [0.9, 1.0]`.
    histogram: [usize; BINS],
}

impl Distribution {
    /// `None` on an empty slice.
    fn of(values: &[f32]) -> Option<Self> {
        let q = |p| quantile(values, p);
        let mut histogram = [0; BINS];
        for &v in values {
            let bin = (v.clamp(0.0, 1.0) * BINS as f32).floor() as usize;
            histogram[bin.min(BINS - 1)] += 1;
        }
        Some(Self {
            n: values.len(),
            mean: values.iter().sum::<f32>() / values.len().max(1) as f32,
            min: q(0.0)?,
            p10: q(0.1)?,
            p25: q(0.25)?,
            p50: q(0.5)?,
            p75: q(0.75)?,
            p90: q(0.9)?,
            max: q(1.0)?,
            near_line: fraction(values, near_line),
            histogram,
        })
    }
}

/// One (config, seed) pair the #494 LHS census read a decomposer guild on,
/// with the verdict it recorded there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GuildSeed {
    /// The LHS sample index.
    config: usize,
    seed: u64,
    /// The census's failure label, `None` for a live seed.
    census_failure: Option<&'static str>,
}

impl GuildSeed {
    /// Whether a re-run reads the census's verdict and still holds a
    /// decomposer guild.
    fn reproduces(&self, failure: Option<&str>, has_decomposer_guild: bool) -> bool {
        failure == self.census_failure && has_decomposer_guild
    }
}

const MONO: Option<&str> = Some("monoculture");
const GD: Option<&str> = Some("generalist_dominance");

const fn seed(config: usize, seed: u64, census_failure: Option<&'static str>) -> GuildSeed {
    GuildSeed {
        config,
        seed,
        census_failure,
    }
}

/// The 21 LHS seeds holding a decomposer guild at `T = 2000` (#494, finding
/// 3): 12 gated `monoculture`, 3 gated `generalist_dominance`, 6 live.
const GUILD_SEEDS: [GuildSeed; 21] = [
    seed(15, 1000, MONO),
    seed(20, 1000, MONO),
    seed(20, 1001, MONO),
    seed(20, 1003, MONO),
    seed(36, 1000, MONO),
    seed(36, 1002, MONO),
    seed(36, 1004, MONO),
    seed(38, 1000, MONO),
    seed(129, 1003, MONO),
    seed(129, 1004, MONO),
    seed(165, 1000, MONO),
    seed(100, 1000, MONO),
    seed(45, 1001, GD),
    seed(96, 1000, GD),
    seed(100, 1002, GD),
    seed(10, 1002, None),
    seed(20, 1004, None),
    seed(127, 1000, None),
    seed(127, 1002, None),
    seed(127, 1004, None),
    seed(188, 1000, None),
];

/// A role's count over the settled-window samples.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct CountRange {
    min: usize,
    median: f32,
    max: usize,
}

impl CountRange {
    fn of(counts: &[usize]) -> Self {
        let as_f32: Vec<f32> = counts.iter().map(|&c| c as f32).collect();
        Self {
            min: counts.iter().copied().min().unwrap_or(0),
            median: quantile(&as_f32, 0.5).unwrap_or(0.0),
            max: counts.iter().copied().max().unwrap_or(0),
        }
    }
}

/// Role counts over the role snapshots in the settled window `(T/2, T]` —
/// the samples, at the evaluator's cadence, the guild predicate reads.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct WindowRoleCounts {
    samples: usize,
    producer: CountRange,
    consumer: CountRange,
    decomposer: CountRange,
}

impl WindowRoleCounts {
    fn of(role_snapshots: &[RoleSnapshot], max_ticks: u64) -> Self {
        let window: Vec<_> = role_snapshots
            .iter()
            .filter(|(tick, _)| *tick > max_ticks / 2)
            .collect();
        let range = |role: TrophicRole| {
            let counts: Vec<usize> = window
                .iter()
                .map(|(_, roles)| roles.values().filter(|r| **r == role).count())
                .collect();
            CountRange::of(&counts)
        };
        Self {
            samples: window.len(),
            producer: range(TrophicRole::Producer),
            consumer: range(TrophicRole::Consumer),
            decomposer: range(TrophicRole::Decomposer),
        }
    }
}

/// The terminal roster read the way the evaluator reads it at `T`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct TerminalStructure {
    roster: usize,
    /// The monoculture gate's read (`clustering_strength`) and its threshold;
    /// a negative margin is a monoculture.
    clustering_strength: f32,
    clustering_threshold: f32,
    clustering_margin: f32,
    /// Heterotrophy share (`trophic_coordinates`) over the whole roster.
    hetero_share: Option<Distribution>,
    /// Terminal decomposers (the topology's role read at `T`).
    decomposers: usize,
    decomposer_hetero_share: Option<Distribution>,
    /// Detrital reliance (decomposed ÷ consumed) of the terminal decomposers.
    decomposer_reliance: Option<Distribution>,
    /// The generalist-dominance gate's read and its fraction.
    generalist_energy_share: f32,
    generalist_dominance_fraction: f32,
    reading: Reading,
}

impl TerminalStructure {
    /// `roster` is `(id, traits, energy)` per living agent; `roles` the
    /// topology's trophic-role read of it; `reliance` the topology's detrital
    /// reliance per agent.
    fn of(
        roster: &[(u64, TraitVector, f32)],
        roles: &HashMap<u64, TrophicRole>,
        reliance: impl Fn(u64) -> Option<f32>,
        config: &EvalConfig,
    ) -> Self {
        let vectors: Vec<TraitVector> = roster.iter().map(|a| a.1).collect();
        let energies: Vec<f32> = roster.iter().map(|a| a.2).collect();
        let share = |t: &TraitVector| trophic_coordinates(t).1;
        let decomposers: Vec<_> = roster
            .iter()
            .filter(|a| roles.get(&a.0) == Some(&TrophicRole::Decomposer))
            .collect();
        let dec_shares: Vec<f32> = decomposers.iter().map(|a| share(&a.1)).collect();
        let dec_reliance: Vec<f32> = decomposers.iter().filter_map(|a| reliance(a.0)).collect();
        let strength = clustering_strength(&vectors);
        Self {
            roster: roster.len(),
            clustering_strength: strength,
            clustering_threshold: config.clustering_threshold,
            clustering_margin: strength - config.clustering_threshold,
            hetero_share: Distribution::of(&vectors.iter().map(share).collect::<Vec<_>>()),
            decomposers: decomposers.len(),
            decomposer_hetero_share: Distribution::of(&dec_shares),
            decomposer_reliance: Distribution::of(&dec_reliance),
            generalist_energy_share: generalist_energy_share(
                &vectors,
                &energies,
                config.generalist_threshold,
            ),
            generalist_dominance_fraction: config.generalist_dominance_fraction,
            reading: classify(&dec_shares, &dec_reliance),
        }
    }
}

/// The fraction of `values` for which `pred` holds; 0 on an empty slice.
fn fraction(values: &[f32], pred: impl Fn(f32) -> bool) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().filter(|&&v| pred(v)).count() as f32 / values.len() as f32
}

/// Verdict counts for one census group.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
struct GroupTally {
    straddle: usize,
    guild: usize,
    ambiguous: usize,
    /// Seeds whose census verdict or guild did not reproduce.
    excluded: usize,
}

/// Verdict counts by census group.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
struct VerdictTally {
    monoculture: GroupTally,
    generalist_dominance: GroupTally,
    live: GroupTally,
}

impl VerdictTally {
    /// Over `(census failure, reproduces, reading)` per seed.
    fn of<'a>(seeds: impl IntoIterator<Item = (Option<&'a str>, bool, Reading)>) -> Self {
        let mut t = Self::default();
        for (census_failure, reproduces, reading) in seeds {
            let group = match census_failure {
                Some("monoculture") => &mut t.monoculture,
                Some("generalist_dominance") => &mut t.generalist_dominance,
                None => &mut t.live,
                Some(other) => panic!("guild_structure: no census group {other:?}"),
            };
            match (reproduces, reading) {
                (false, _) => group.excluded += 1,
                (true, Reading::Straddle) => group.straddle += 1,
                (true, Reading::Guild) => group.guild += 1,
                (true, Reading::Ambiguous) => group.ambiguous += 1,
            }
        }
        t
    }
}

/// One seed's measurement.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct SeedStructure {
    seed: u64,
    /// The verdict the #494 census recorded.
    census_failure: Option<String>,
    /// The verdict this re-run reads.
    failure: Option<String>,
    fitness: f32,
    termination_tick: u64,
    has_decomposer_guild: bool,
    has_consumer_guild: bool,
    /// Census verdict and decomposer guild both reproduce.
    reproduces: bool,
    window_roles: WindowRoleCounts,
    terminal: TerminalStructure,
}

/// One config's row: the listed seeds of it.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct StructureRow {
    source: ConfigSource,
    config_index: usize,
    horizon: u64,
    seeds: Vec<SeedStructure>,
}

/// Roll one listed seed out on the census's path (`decode` over
/// `default_ranges`, `EvalConfig::default()`, horizon `horizon`, no early-stop
/// carry) and read its structure.
fn measure(unit: &[f64], guild_seed: &GuildSeed, horizon: u64) -> SeedStructure {
    let (params, dist) = decode(unit, &default_ranges());
    let run_config = RunConfig {
        max_ticks: horizon,
        eval_config: EvalConfig::default(),
        early_stop_crosscheck_fraction: 0.0,
    };
    let Rollout {
        result,
        world,
        observations,
    } = rollout(&params, &dist, &run_config, guild_seed.seed);
    let failure = result
        .failure
        .as_ref()
        .map(|f| Cliff::from_failure(f).label());
    let agents = world.agents();
    let topology = observations.topology();
    // The evaluator's own terminal read: the projection's trophic roles over
    // the living roster, traits and `energy()` per agent.
    let roles = topology.trophic_roles_of(agents.iter().map(|a| (a.id, &a.traits)));
    let roster: Vec<(u64, TraitVector, f32)> = agents
        .iter()
        .map(|a| (a.id, a.traits, a.energy()))
        .collect();
    let terminal = TerminalStructure::of(
        &roster,
        &roles,
        |id| topology.detrital_reliance(id),
        &run_config.eval_config,
    );
    SeedStructure {
        seed: guild_seed.seed,
        census_failure: guild_seed.census_failure.map(String::from),
        failure: failure.map(String::from),
        fitness: result.fitness,
        termination_tick: result.termination_tick,
        has_decomposer_guild: result.breakdown.has_decomposer_guild,
        has_consumer_guild: result.breakdown.has_consumer_guild,
        reproduces: guild_seed.reproduces(failure, result.breakdown.has_decomposer_guild),
        window_roles: WindowRoleCounts::of(&observations.role_snapshots, horizon),
        terminal,
    }
}

/// Command line: `--configs sample:20,sample:36` (default: every listed
/// config), `--limit N`, `--output PATH` (alias `--out`), `--summary`.
struct Args {
    configs: Option<HashSet<(ConfigSource, usize)>>,
    limit: Option<usize>,
    out: PathBuf,
    summary_only: bool,
}

/// The census horizon.
const HORIZON: u64 = 2000;
const DEFAULT_OUT: &str = "target/546/guild-structure.jsonl";

fn parse_args(argv: impl IntoIterator<Item = String>) -> Args {
    let mut args = Args {
        configs: None,
        limit: None,
        out: PathBuf::from(DEFAULT_OUT),
        summary_only: false,
    };
    let mut it = argv.into_iter();
    while let Some(flag) = it.next() {
        let mut value = || {
            it.next()
                .unwrap_or_else(|| panic!("guild_structure: {flag} needs a value"))
        };
        match flag.as_str() {
            "--configs" => args.configs = Some(parse_selector(&value(), "--configs", None)),
            "--limit" => {
                let raw = value();
                args.limit = Some(raw.parse().unwrap_or_else(|_| {
                    panic!("guild_structure: --limit {raw:?} is not an integer")
                }))
            }
            "--output" | "--out" => args.out = PathBuf::from(value()),
            "--summary" => args.summary_only = true,
            other => panic!("guild_structure: unknown argument {other:?}"),
        }
    }
    args
}

fn main() {
    let args = parse_args(std::env::args().skip(1));
    if !args.summary_only {
        let listed: HashSet<(ConfigSource, usize)> = GUILD_SEEDS
            .iter()
            .map(|s| (ConfigSource::SAMPLE, s.config))
            .filter(|key| args.configs.as_ref().is_none_or(|f| f.contains(key)))
            .collect();
        let sampled = sampled_units(default_ranges().len());
        let done = done_configs(&args.out);
        let tasks = plan_tasks(0, sampled.len(), Some(&listed), &done, args.limit);
        eprintln!(
            "guild_structure: {} listed configs, {} done in {}, running {} now (horizon {HORIZON})",
            listed.len(),
            done.len(),
            args.out.display(),
            tasks.len()
        );
        let start = Instant::now();
        for (source, idx) in tasks {
            let config_start = Instant::now();
            let seeds: Vec<GuildSeed> = GUILD_SEEDS
                .iter()
                .filter(|s| s.config == idx)
                .copied()
                .collect();
            let unit = &sampled[idx];
            let measured: Vec<SeedStructure> = seeds
                .par_iter()
                .map(|s| measure(unit, s, HORIZON))
                .collect();
            let row = StructureRow {
                source,
                config_index: idx,
                horizon: HORIZON,
                seeds: measured,
            };
            append_row(&args.out, &row);
            eprintln!(
                "  sample:{idx} done: {} seeds ({:.1}s; {:.0}s elapsed)",
                row.seeds.len(),
                config_start.elapsed().as_secs_f64(),
                start.elapsed().as_secs_f64()
            );
        }
        eprintln!(
            "guild_structure: run in {:.0}s",
            start.elapsed().as_secs_f64()
        );
    }
    let rows: Vec<StructureRow> = read_rows(&args.out);
    print_summary(&rows);
}

fn print_summary(rows: &[StructureRow]) {
    let mut seeds: Vec<(usize, &SeedStructure)> = rows
        .iter()
        .flat_map(|r| r.seeds.iter().map(move |s| (r.config_index, s)))
        .collect();
    seeds.sort_by_key(|(c, s)| {
        (
            s.census_failure.is_none(),
            s.census_failure.clone(),
            *c,
            s.seed,
        )
    });
    let opt = |d: &Option<Distribution>, f: fn(&Distribution) -> f32| {
        d.as_ref()
            .map_or("-".to_string(), |d| format!("{:.2}", f(d)))
    };
    println!("\n# Guild structure (issue #546)\n");
    println!(
        "| config/seed | census | re-run | repro | cs − thr | roster | het p10/p50/p90 | near line | P / C / D (window median) | dec n | dec het mean | dec near | dec rel p10/p50/p90 | gen share | reading |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for (config, s) in &seeds {
        let t = &s.terminal;
        let w = &s.window_roles;
        println!(
            "| sample:{config}/{} | {} | {} | {} | {:+.3} | {} | {}/{}/{} | {} | {:.0} / {:.0} / {:.0} (D {}–{}) | {} | {} | {} | {}/{}/{} | {:.2} | {:?} |",
            s.seed,
            s.census_failure.as_deref().unwrap_or("live"),
            s.failure.as_deref().unwrap_or("live"),
            if s.reproduces { "yes" } else { "NO" },
            t.clustering_margin,
            t.roster,
            opt(&t.hetero_share, |d| d.p10),
            opt(&t.hetero_share, |d| d.p50),
            opt(&t.hetero_share, |d| d.p90),
            opt(&t.hetero_share, |d| d.near_line),
            w.producer.median,
            w.consumer.median,
            w.decomposer.median,
            w.decomposer.min,
            w.decomposer.max,
            t.decomposers,
            opt(&t.decomposer_hetero_share, |d| d.mean),
            opt(&t.decomposer_hetero_share, |d| d.near_line),
            opt(&t.decomposer_reliance, |d| d.p10),
            opt(&t.decomposer_reliance, |d| d.p50),
            opt(&t.decomposer_reliance, |d| d.p90),
            t.generalist_energy_share,
            t.reading,
        );
    }
    let tally = VerdictTally::of(seeds.iter().map(|(_, s)| {
        (
            s.census_failure.as_deref(),
            s.reproduces,
            s.terminal.reading,
        )
    }));
    println!(
        "\n{}",
        serde_json::to_string_pretty(&tally).expect("serialise tally")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decomposers_mostly_near_the_photo_hetero_line_read_as_a_straddle() {
        // Three of four decomposers sit within 0.1 of share 0.5.
        let shares = [0.52, 0.55, 0.58, 0.9];
        let reliance = [0.9, 0.95, 1.0, 1.0];
        assert_eq!(classify(&shares, &reliance), Reading::Straddle);
    }

    #[test]
    fn decomposers_clearly_heterotroph_and_carcass_fed_read_as_a_behavioural_guild() {
        // One of five near the line (0.2), median reliance 0.9.
        let shares = [0.58, 0.8, 0.85, 0.9, 0.95];
        let reliance = [0.6, 0.8, 0.9, 1.0, 1.0];
        assert_eq!(classify(&shares, &reliance), Reading::Guild);
    }

    #[test]
    fn anything_between_the_two_readings_is_ambiguous() {
        // Clearly heterotroph, but reliance clustered at the 0.5 cut.
        let clear = [0.8, 0.85, 0.9, 0.95];
        assert_eq!(classify(&clear, &[0.5, 0.55, 0.6, 0.7]), Reading::Ambiguous);
        // Carcass-fed, but a third of the decomposers on the line.
        assert_eq!(
            classify(&[0.52, 0.8, 0.9], &[1.0, 1.0, 1.0]),
            Reading::Ambiguous
        );
        // No terminal decomposers to read.
        assert_eq!(classify(&[], &[]), Reading::Ambiguous);
    }

    #[test]
    fn a_distribution_carries_quantiles_a_ten_bin_histogram_and_the_near_line_share() {
        let values = [0.0, 0.1, 0.45, 0.5, 0.55, 0.72, 0.95, 1.0];
        let d = Distribution::of(&values).expect("non-empty");
        assert_eq!(d.n, 8);
        assert_eq!(d.min, 0.0);
        assert_eq!(d.max, 1.0);
        // Median between the 4th and 5th order statistics (0.5, 0.55).
        assert!((d.p50 - 0.525).abs() < 1e-6);
        assert!(d.p10 <= d.p25 && d.p25 <= d.p50 && d.p50 <= d.p75 && d.p75 <= d.p90);
        assert!((d.mean - 4.27 / 8.0).abs() < 1e-6);
        // 0.45, 0.5, 0.55 sit within 0.1 of the line.
        assert_eq!(d.near_line, 3.0 / 8.0);
        // Bins [0, 0.1), [0.1, 0.2) … [0.9, 1.0]; 1.0 lands in the last.
        assert_eq!(d.histogram, [1, 1, 0, 0, 1, 2, 0, 1, 0, 2]);
        assert_eq!(Distribution::of(&[]), None);
    }

    #[test]
    fn role_counts_span_only_the_settled_window_samples() {
        use TrophicRole::{Consumer, Decomposer, Producer};
        let sample = |tick: u64, roles: &[TrophicRole]| -> RoleSnapshot {
            (
                tick,
                roles
                    .iter()
                    .enumerate()
                    .map(|(i, r)| (i as u64, *r))
                    .collect(),
            )
        };
        let snapshots = vec![
            // Tick 50 is T/2 — outside (T/2, T].
            sample(50, &[Decomposer; 9]),
            sample(60, &[Producer, Producer, Decomposer]),
            sample(80, &[Producer, Decomposer, Decomposer, Consumer]),
            sample(100, &[Producer, Producer, Producer, Decomposer]),
        ];
        let c = WindowRoleCounts::of(&snapshots, 100);
        assert_eq!(c.samples, 3);
        assert_eq!(
            c.producer,
            CountRange {
                min: 1,
                median: 2.0,
                max: 3
            }
        );
        assert_eq!(
            c.consumer,
            CountRange {
                min: 0,
                median: 0.0,
                max: 1
            }
        );
        assert_eq!(
            c.decomposer,
            CountRange {
                min: 1,
                median: 1.0,
                max: 2
            }
        );
    }

    #[test]
    fn the_seed_list_is_the_census_guild_seeds_with_their_census_verdicts() {
        let count = |failure: Option<&str>| {
            GUILD_SEEDS
                .iter()
                .filter(|s| s.census_failure == failure)
                .count()
        };
        assert_eq!(GUILD_SEEDS.len(), 21);
        assert_eq!(count(Some("monoculture")), 12);
        assert_eq!(count(Some("generalist_dominance")), 3);
        assert_eq!(count(None), 6);
        let configs: std::collections::BTreeSet<usize> =
            GUILD_SEEDS.iter().map(|s| s.config).collect();
        assert_eq!(
            configs.into_iter().collect::<Vec<_>>(),
            [10, 15, 20, 36, 38, 45, 96, 100, 127, 129, 165, 188]
        );
    }

    #[test]
    fn a_seed_reproduces_when_its_verdict_and_decomposer_guild_match_the_census() {
        let gated = GuildSeed {
            config: 20,
            seed: 1000,
            census_failure: Some("monoculture"),
        };
        assert!(gated.reproduces(Some("monoculture"), true));
        assert!(!gated.reproduces(Some("monoculture"), false));
        assert!(!gated.reproduces(None, true));
        assert!(!gated.reproduces(Some("generalist_dominance"), true));
        let live = GuildSeed {
            config: 20,
            seed: 1004,
            census_failure: None,
        };
        assert!(live.reproduces(None, true));
        assert!(!live.reproduces(Some("monoculture"), true));
    }

    fn traits(photo: f32, hetero: f32) -> TraitVector {
        TraitVector {
            photosynthetic_absorption: photo,
            heterotrophy: hetero,
            mobility: 0.0,
            kappa: 0.5,
            fecundity: 0.0,
            asexual_propensity: 0.0,
            dispersal: 0.0,
        }
    }

    #[test]
    fn terminal_structure_reads_the_roster_through_the_evaluator_functions() {
        use TrophicRole::{Consumer, Decomposer, Producer};
        // id, traits, energy, role
        let agents = [
            (1, traits(0.5, 0.5), 10.0, Producer), // generalist, on the line
            (2, traits(1.0, 0.0), 2.0, Producer),
            (3, traits(0.1, 0.9), 2.0, Decomposer),
            (4, traits(0.2, 0.8), 2.0, Decomposer),
            (5, traits(0.0, 1.0), 2.0, Decomposer),
            (6, traits(0.0, 1.0), 2.0, Consumer),
        ];
        let roster: Vec<(u64, TraitVector, f32)> =
            agents.iter().map(|&(id, t, e, _)| (id, t, e)).collect();
        let roles: HashMap<u64, TrophicRole> =
            agents.iter().map(|&(id, _, _, r)| (id, r)).collect();
        let reliance = |id: u64| match id {
            3 => Some(0.8),
            4 => Some(0.9),
            5 => Some(1.0),
            6 => Some(0.1),
            _ => None,
        };
        let config = EvalConfig::default();
        let s = TerminalStructure::of(&roster, &roles, reliance, &config);

        let vectors: Vec<TraitVector> = roster.iter().map(|a| a.1).collect();
        assert_eq!(s.roster, 6);
        assert_eq!(s.clustering_strength, clustering_strength(&vectors));
        assert_eq!(s.clustering_threshold, config.clustering_threshold);
        assert_eq!(
            s.clustering_margin,
            s.clustering_strength - config.clustering_threshold
        );
        assert_eq!(s.generalist_energy_share, 0.5);
        assert_eq!(
            s.generalist_dominance_fraction,
            config.generalist_dominance_fraction
        );
        let hetero = s.hetero_share.expect("roster is non-empty");
        assert_eq!(hetero.n, 6);
        assert!((hetero.near_line - 1.0 / 6.0).abs() < 1e-6);
        assert_eq!(s.decomposers, 3);
        let dec_hetero = s.decomposer_hetero_share.expect("three decomposers");
        assert!((dec_hetero.mean - 0.9).abs() < 1e-6);
        let dec_reliance = s.decomposer_reliance.expect("three decomposers");
        assert_eq!(dec_reliance.n, 3);
        assert!((dec_reliance.p50 - 0.9).abs() < 1e-6);
        assert_eq!(s.reading, Reading::Guild);
    }

    #[test]
    fn verdicts_are_tallied_by_census_group_and_non_reproducing_seeds_are_excluded() {
        let seeds = [
            (Some("monoculture"), true, Reading::Straddle),
            (Some("monoculture"), true, Reading::Straddle),
            (Some("monoculture"), true, Reading::Ambiguous),
            (Some("monoculture"), false, Reading::Guild),
            (Some("generalist_dominance"), true, Reading::Guild),
            (None, true, Reading::Guild),
            (None, false, Reading::Straddle),
        ];
        let t = VerdictTally::of(seeds.iter().map(|&(f, r, v)| (f, r, v)));
        assert_eq!(
            t.monoculture,
            GroupTally {
                straddle: 2,
                guild: 0,
                ambiguous: 1,
                excluded: 1
            }
        );
        assert_eq!(
            t.generalist_dominance,
            GroupTally {
                straddle: 0,
                guild: 1,
                ambiguous: 0,
                excluded: 0
            }
        );
        assert_eq!(
            t.live,
            GroupTally {
                straddle: 0,
                guild: 1,
                ambiguous: 0,
                excluded: 1
            }
        );
    }
}
