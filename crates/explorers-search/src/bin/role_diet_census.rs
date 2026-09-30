//! Role tag against realised diet (issue #596).
//!
//! For each config: an ensemble rolled out exactly as the genesis rollout
//! runs it ([`explorers_search::role_diet::rollout`], pinned against
//! `explorers_genesis::rollout`), with every agent's lifetime income booked
//! by source. On the evaluator's own second-half role snapshots it reads each
//! agent's trait tag and its diet role (light ≥ half its income → producer by
//! diet), and re-reads the guild rule with diet roles. Per config it reports:
//!
//! - the trait tag × diet role confusion over agent-samples;
//! - the light-share deciles of the agents the tag reads as heterotrophs;
//! - the guild fractions (share of seeds holding each guild) under both
//!   reads;
//! - every death, attributed to trait producers, light-fed trait
//!   heterotrophs and the rest, by infant / grazed / grazed by kin.
//!
//! Configs and seeds as `guild_census`: the atlas cells (`--atlas PATH`),
//! the LHS sample, or `--configs` (`sample@S:i` for another draw); the seed
//! block `--seed ..` (default 1000) of `--ensemble` seeds (default 5, the
//! search's), horizon `--max-ticks` (default 2000), `EvalConfig::default()`.
//! Seeds run in parallel. One JSON line per config appended to `--out`
//! (default `target/role-diet-census.jsonl`); configs already present are
//! skipped; `--limit N` caps a call; `--summary` only prints.
//! `--founder-aggregation A` pins every world's founding placement (#601)
//! and records the pin on each row; `0` is the pre-#601 well-mixed scatter,
//! for comparing against an older tree (#605).
//!
//!   cargo run --release -p explorers-search --bin role_diet_census -- --atlas atlas.json
//!   cargo run --release -p explorers-search --bin role_diet_census -- --configs sample:31,sample:110
//!   cargo run --release -p explorers-search --bin role_diet_census -- --summary

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

use rayon::prelude::*;

use explorers_genesis::EvalConfig;
use explorers_search::config_source::{
    ConfigSource, parse_founder_aggregation, parse_selector, resolve_config, sampled_units,
    with_founder_aggregation,
};
use explorers_search::role_diet::{
    Confusion, DeathCounts, DeathTable, LIGHT_SHARE_BINS, PRODUCER_LIGHT_SHARE, SeedDiet, rollout,
};
use explorers_search::search::default_ranges;
use explorers_search::sweep::{append_row, done_configs, plan_tasks, read_atlas_units, read_rows};

const DEFAULT_HORIZON: u64 = 2000;
const DEFAULT_ENSEMBLE: u64 = 5;
const DEFAULT_SEED: u64 = 1000;
const DEFAULT_OUT: &str = "target/role-diet-census.jsonl";
/// A guild counts as held by a config at or above this share of its seeds.
const HALF: f64 = 0.5;

const ROLES: [&str; 3] = ["producer", "consumer", "decomposer"];

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct Row {
    source: ConfigSource,
    config_index: usize,
    horizon: u64,
    base_seed: u64,
    /// The founder aggregation every world was pinned to
    /// (`--founder-aggregation`, #605); absent when worlds ran as decoded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    founder_aggregation: Option<f32>,
    seeds: Vec<SeedDiet>,
}

impl Row {
    /// Share of seeds holding each guild, under the tag and under diet.
    fn guild_fractions(&self) -> ([f64; 3], [f64; 3]) {
        let n = self.seeds.len().max(1) as f64;
        let frac = |f: &dyn Fn(&SeedDiet) -> [bool; 3]| {
            let mut out = [0.0; 3];
            for s in &self.seeds {
                for (o, held) in out.iter_mut().zip(f(s)) {
                    *o += f64::from(u8::from(held)) / n;
                }
            }
            out
        };
        (frac(&|s| s.tag_guilds), frac(&|s| s.diet_guilds))
    }

    /// Whether the config is live as the atlas reads one: most seeds live.
    fn live(&self) -> bool {
        let live = self.seeds.iter().filter(|s| s.failure.is_none()).count();
        live * 2 > self.seeds.len()
    }
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
struct Pool {
    configs: usize,
    seeds: usize,
    confusion: Confusion,
    light_share: [u64; LIGHT_SHARE_BINS],
    deaths: DeathTable,
    /// Configs holding each guild at ≥ half their seeds: [tag & diet, tag
    /// only, diet only] per role.
    guild_cross: [[usize; 3]; 3],
    /// Live seeds' terminal trophic balance, (tag read, diet read).
    balance: Vec<(f32, f32)>,
}

fn pool<'a>(rows: impl Iterator<Item = &'a Row>) -> Pool {
    let mut p = Pool::default();
    for r in rows {
        p.configs += 1;
        let (tag, diet) = r.guild_fractions();
        for role in 0..3 {
            let (t, d) = (tag[role] >= HALF, diet[role] >= HALF);
            let col = match (t, d) {
                (true, true) => Some(0),
                (true, false) => Some(1),
                (false, true) => Some(2),
                (false, false) => None,
            };
            if let Some(c) = col {
                p.guild_cross[role][c] += 1;
            }
        }
        for s in &r.seeds {
            p.seeds += 1;
            p.confusion.merge(&s.confusion);
            for (a, b) in p.light_share.iter_mut().zip(s.heterotroph_light_share) {
                *a += b;
            }
            p.deaths.merge(&s.deaths);
            if let (None, Some(t), Some(d)) =
                (&s.failure, s.trophic_balance_tag, s.trophic_balance_diet)
            {
                p.balance.push((t, d));
            }
        }
    }
    p
}

#[derive(Clone, Debug, PartialEq)]
struct Args {
    limit: Option<usize>,
    horizon: u64,
    ensemble: u64,
    seed: u64,
    out: PathBuf,
    atlas: Option<PathBuf>,
    configs: Option<HashSet<(ConfigSource, usize)>>,
    summary_only: bool,
    /// Pin every resolved world's founder aggregation (#601) to this value;
    /// `None` runs worlds as decoded. `0` is the pre-#601 well-mixed scatter,
    /// for comparing against an older tree (#605).
    founder_aggregation: Option<f32>,
}

fn parse_args<I: IntoIterator<Item = String>>(argv: I) -> Result<Args, String> {
    let mut args = Args {
        limit: None,
        horizon: DEFAULT_HORIZON,
        ensemble: DEFAULT_ENSEMBLE,
        seed: DEFAULT_SEED,
        out: PathBuf::from(DEFAULT_OUT),
        atlas: None,
        configs: None,
        summary_only: false,
        founder_aggregation: None,
    };
    let mut it = argv.into_iter();
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{flag} needs a value"));
        let number = |raw: String| -> Result<u64, String> {
            raw.parse()
                .map_err(|_| format!("{flag} {raw:?} is not an integer"))
        };
        match flag.as_str() {
            "--limit" => args.limit = Some(number(value()?)? as usize),
            "--max-ticks" | "--horizon" => args.horizon = number(value()?)?,
            "--ensemble" => args.ensemble = number(value()?)?,
            "--seed" => args.seed = number(value()?)?,
            "--out" | "--output" => args.out = PathBuf::from(value()?),
            "--atlas" => args.atlas = Some(PathBuf::from(value()?)),
            "--configs" => args.configs = Some(parse_selector(&value()?, "--configs", None)),
            "--summary" => args.summary_only = true,
            "--founder-aggregation" => {
                args.founder_aggregation = Some(parse_founder_aggregation(&value()?)?)
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    if args.horizon == 0 || args.ensemble == 0 {
        return Err("--max-ticks and --ensemble must be positive".to_string());
    }
    Ok(args)
}

fn run_row(
    source: ConfigSource,
    config_index: usize,
    config: &(
        explorers_sim::WorldParameters,
        explorers_sim::InitialDistribution,
    ),
    args: &Args,
) -> Row {
    let eval = EvalConfig::default();
    let config = with_founder_aggregation(config.clone(), args.founder_aggregation);
    let seeds = (0..args.ensemble)
        .into_par_iter()
        .map(|i| rollout(&config.0, &config.1, args.seed + i, args.horizon, &eval))
        .collect();
    Row {
        source,
        config_index,
        horizon: args.horizon,
        base_seed: args.seed,
        founder_aggregation: args.founder_aggregation,
        seeds,
    }
}

fn main() {
    let args = parse_args(std::env::args().skip(1)).unwrap_or_else(|e| {
        eprintln!("role_diet_census: {e}");
        std::process::exit(2)
    });
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
            "role_diet_census: {} configs to run × {} seeds (base {}), horizon {}; {} done in {}",
            tasks.len(),
            args.ensemble,
            args.seed,
            args.horizon,
            done.len(),
            args.out.display()
        );
        let start = Instant::now();
        let total = tasks.len();
        for (n, (source, idx)) in tasks.into_iter().enumerate() {
            let config = resolve_config(source, idx, &atlas_units, &sampled);
            let t = Instant::now();
            let row = run_row(source, idx, &config, &args);
            append_row(&args.out, &row);
            let (tag, diet) = row.guild_fractions();
            eprintln!(
                "  {source}:{idx} ({}/{total}): consumer guild tag {:.2} diet {:.2}, decomposer tag {:.2} diet {:.2} ({:.1}s; {:.0}s elapsed)",
                n + 1,
                tag[1],
                diet[1],
                tag[2],
                diet[2],
                t.elapsed().as_secs_f64(),
                start.elapsed().as_secs_f64()
            );
        }
    }
    let rows: Vec<Row> = read_rows(&args.out);
    print_summary(&rows);
}

fn pct(n: u64, d: u64) -> String {
    if d == 0 {
        "–".to_string()
    } else {
        format!("{:.1}", 100.0 * n as f64 / d as f64)
    }
}

fn print_deaths(label: &str, c: &DeathCounts) {
    println!(
        "| {label} | {} | {} ({}%) | {} ({}%) | {} / {} | {} ({}% of grazed) |",
        c.deaths,
        c.infant,
        pct(c.infant, c.deaths),
        c.grazed,
        pct(c.grazed, c.deaths),
        c.infant_grazed,
        c.infant_grazed_by_kin,
        c.grazed_by_kin,
        pct(c.grazed_by_kin, c.grazed),
    );
}

fn print_pool(name: &str, p: &Pool) {
    println!("\n## {name}: {} configs, {} seeds\n", p.configs, p.seeds);
    println!(
        "Trait tag (rows) × diet role (columns), agent-samples over the second half; diet producer = light ≥ {PRODUCER_LIGHT_SHARE} of lifetime income (heterotrophic income as energy drained).\n"
    );
    println!(
        "| tag \\ diet | producer | consumer | decomposer | no income | total | diet agrees |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|");
    for (i, row) in p.confusion.0.iter().enumerate() {
        let total: u64 = row.iter().sum();
        println!(
            "| {} | {} | {} | {} | {} | {} | {}% |",
            ROLES[i],
            row[0],
            row[1],
            row[2],
            row[3],
            total,
            pct(row[i], total)
        );
    }
    let het: u64 = p.light_share.iter().sum();
    println!(
        "\nLight share of income, trait-tagged heterotroph agent-samples with income ({het}):\n"
    );
    println!("| light share | agent-samples | % |");
    println!("|---|---:|---:|");
    for (i, n) in p.light_share.iter().enumerate() {
        println!(
            "| {:.1}–{:.1} | {n} | {} |",
            i as f64 / LIGHT_SHARE_BINS as f64,
            (i + 1) as f64 / LIGHT_SHARE_BINS as f64,
            pct(*n, het)
        );
    }
    println!("\nConfigs holding a guild on ≥ half their seeds:\n");
    println!("| guild | tag and diet | tag only | diet only |");
    println!("|---|---:|---:|---:|");
    for (i, c) in p.guild_cross.iter().enumerate() {
        println!("| {} | {} | {} | {} |", ROLES[i], c[0], c[1], c[2]);
    }
    let median = |mut v: Vec<f64>| -> f64 {
        v.sort_by(f64::total_cmp);
        explorers_search::invasion::median(&v)
    };
    let b = &p.balance;
    println!(
        "\nTrophic balance (the producer share of living energy, the evaluator's fitness term until #602) at the horizon, {} live seeds: median by tag {:.3}, by diet {:.3}; median diet − tag {:+.3}; {} seeds move by more than 0.05.",
        b.len(),
        median(b.iter().map(|x| x.0 as f64).collect()),
        median(b.iter().map(|x| x.1 as f64).collect()),
        median(b.iter().map(|x| (x.1 - x.0) as f64).collect()),
        b.iter().filter(|x| (x.1 - x.0).abs() > 0.05).count(),
    );
    println!(
        "\nDeaths over whole runs (infant ≤ {} ticks; grazed = drained alive in its death tick; kin = parent, offspring or sibling):\n",
        explorers_search::role_diet::INFANT_AGE
    );
    println!("| agents | deaths | infant | grazed | infant grazed / by kin | grazed by kin |");
    println!("|---|---:|---:|---:|---:|---:|");
    print_deaths("trait producers", &p.deaths.trait_producer);
    print_deaths(
        "trait heterotrophs, light-fed",
        &p.deaths.trait_heterotroph_light_fed,
    );
    print_deaths(
        "trait heterotrophs, heterotroph by diet",
        &p.deaths.trait_heterotroph_diet_fed,
    );
    print_deaths(
        "trait heterotrophs, no income yet",
        &p.deaths.trait_heterotroph_no_income,
    );
}

fn print_summary(rows: &[Row]) {
    println!("\n# Role tag against realised diet (issue #596)");
    for (name, sample) in [("atlas", false), ("sample", true)] {
        let of = || rows.iter().filter(move |r| r.source.is_sample() == sample);
        if of().next().is_none() {
            continue;
        }
        print_pool(&format!("{name}, all configs"), &pool(of()));
        print_pool(
            &format!("{name}, live configs"),
            &pool(of().filter(|r| r.live())),
        );
    }
    println!("\n## Per config\n");
    println!(
        "| config | live seeds | consumer guild tag / diet | decomposer guild tag / diet | tagged consumer samples light-fed |"
    );
    println!("|---|---:|---:|---:|---:|");
    for r in rows {
        let (tag, diet) = r.guild_fractions();
        let mut c = Confusion::default();
        for s in &r.seeds {
            c.merge(&s.confusion);
        }
        let consumers: u64 = c.0[1].iter().sum();
        println!(
            "| {}:{} | {}/{} | {:.2} / {:.2} | {:.2} / {:.2} | {}% |",
            r.source,
            r.config_index,
            r.seeds.iter().filter(|s| s.failure.is_none()).count(),
            r.seeds.len(),
            tag[1],
            diet[1],
            tag[2],
            diet[2],
            pct(c.0[1][0], consumers)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed(tag: [bool; 3], diet: [bool; 3], failure: Option<&str>) -> SeedDiet {
        let mut confusion = Confusion::default();
        confusion.0[1][0] = 3;
        confusion.0[1][1] = 1;
        SeedDiet {
            seed: 0,
            failure: failure.map(String::from),
            termination_tick: 2000,
            tag_guilds: tag,
            diet_guilds: diet,
            confusion,
            heterotroph_light_share: [0, 0, 0, 0, 0, 0, 0, 0, 1, 3],
            deaths: DeathTable::default(),
            trophic_balance_tag: Some(0.5),
            trophic_balance_diet: Some(0.9),
        }
    }

    /// A config holds a guild at ≥ half its seeds; the pool crosses the tag
    /// read with the diet read per role and sums the per-seed tables.
    #[test]
    fn pool_crosses_guild_reads_and_sums_the_tables() {
        let consumer_tag_only = Row {
            source: ConfigSource::SAMPLE,
            config_index: 1,
            horizon: 2000,
            base_seed: 1000,
            founder_aggregation: None,
            seeds: vec![
                seed([true, true, false], [true, false, false], None),
                seed([true, true, false], [true, false, false], None),
                seed(
                    [true, false, false],
                    [true, false, false],
                    Some("monoculture"),
                ),
            ],
        };
        assert!(consumer_tag_only.live());
        let (tag, diet) = consumer_tag_only.guild_fractions();
        assert!((tag[1] - 2.0 / 3.0).abs() < 1e-12 && diet[1] == 0.0);
        let p = pool(std::iter::once(&consumer_tag_only));
        assert_eq!(p.guild_cross[0], [1, 0, 0], "producer: both reads");
        assert_eq!(p.guild_cross[1], [0, 1, 0], "consumer: tag only");
        assert_eq!(p.guild_cross[2], [0, 0, 0]);
        assert_eq!(p.confusion.0[1], [9, 3, 0, 0]);
        assert_eq!(p.light_share[9], 9);
        assert_eq!((p.configs, p.seeds), (1, 3));
        assert_eq!(p.balance, vec![(0.5, 0.9), (0.5, 0.9)], "live seeds only");
    }

    #[test]
    fn args_default_to_the_search_ensemble_and_horizon() {
        let parse = |a: &[&str]| parse_args(a.iter().map(|s| s.to_string()));
        let a = parse(&[]).unwrap();
        assert_eq!((a.horizon, a.ensemble, a.seed), (2000, 5, 1000));
        assert_eq!(a.out, PathBuf::from(DEFAULT_OUT));
        let a = parse(&["--configs", "sample:31", "--limit", "2"]).unwrap();
        assert_eq!(a.limit, Some(2));
        assert!(a.configs.unwrap().contains(&(ConfigSource::SAMPLE, 31)));
        assert!(parse(&["--bogus"]).is_err());
        assert!(parse(&["--ensemble", "0"]).is_err());
    }

    /// `--founder-aggregation A` pins every resolved world's founding
    /// placement (#605: `0` compares against a pre-#601 tree), and each row
    /// records the pin; unpinned rows read and write as before.
    #[test]
    fn a_founder_aggregation_pin_reaches_the_world_and_the_row() {
        let parse = |a: &[&str]| parse_args(a.iter().map(|s| s.to_string()));
        assert_eq!(parse(&[]).unwrap().founder_aggregation, None);
        let args = parse(&[
            "--founder-aggregation",
            "0",
            "--max-ticks",
            "20",
            "--ensemble",
            "1",
        ])
        .unwrap();
        assert_eq!(args.founder_aggregation, Some(0.0));
        assert!(parse(&["--founder-aggregation", "1.2"]).is_err());

        let sampled = sampled_units(default_ranges().len());
        let decoded = resolve_config(ConfigSource::SAMPLE, 31, &Default::default(), &sampled);
        let pinned = run_row(ConfigSource::SAMPLE, 31, &decoded, &args);
        assert_eq!(pinned.founder_aggregation, Some(0.0));
        let unpinned_args = parse(&["--max-ticks", "20", "--ensemble", "1"]).unwrap();
        let unpinned = run_row(ConfigSource::SAMPLE, 31, &decoded, &unpinned_args);
        assert_eq!(unpinned.founder_aggregation, None);
        assert!(
            !serde_json::to_string(&unpinned)
                .unwrap()
                .contains("founder_aggregation")
        );
        assert_ne!(
            pinned.seeds, unpinned.seeds,
            "the pin changes the founding placement the rollout sees"
        );
        let back: Row = serde_json::from_str(&serde_json::to_string(&pinned).unwrap()).unwrap();
        assert_eq!(back, pinned);
    }
}
