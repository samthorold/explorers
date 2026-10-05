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
//! - every grazed death's (grazer, victim) pairs, by kinship, trait
//!   distance and the grazer's drain-time satiation
//!   ([`explorers_search::grazer_hunger`], #606), and every birth's trait
//!   distance to its parents.
//! - every second-half sampled agent's surplus satiation, read after
//!   metabolism and before growth, by recent-income role, with the light-fed
//!   mixotrophs apart, and the default `satiation_sensitivity = 1 / s₂₅` it
//!   proposes (#622).
//! - the intake-ceiling window (#629): light-fed mixotrophs' light and
//!   heterotrophs-by-diet's realised intake and drain potential at the start
//!   of drain resolution, in ticks of maintenance in both currencies
//!   ([`explorers_search::intake_ceiling`]), with the bounds
//!   `k ≤ 2 × p25(light / m)` and `k ≥ p75(intake / m)` and the default `k`
//!   at the window's geometric midpoint. `--intake-ceiling-k K` also bands
//!   the killing grazers by their predicted intake-gate expression at `K`.
//! - per-sample gate records (#634): every kin-killing pair whose killer is a
//!   light-fed mixotroph, and every heterotroph-by-diet sample, each with
//!   what the intake gate reads. `--region` (with or without `--summary`)
//!   evaluates the ceiling `C = m × (k_a + k_h × h_eff)` over a log grid
//!   (k_a, k_h each over [0.25, 20] at ≥ 8 points per decade, k_h also 0)
//!   and prints both outcomes per cell, the feasible-cell count, and the
//!   largest-margin default, or "region EMPTY"
//!   ([`explorers_search::intake_ceiling::region_report`]).
//! - the fullness-gate region (#637): with `--fullness`, every seed keeps
//!   each agent's fullness bank online (one fullness per currency per
//!   clearance time `τ` on a fixed grid) and accumulates, per `(n, k, τ)`,
//!   the sated light-fed mixotroph kin kills and the production heterotrophs
//!   by diet keep ([`explorers_search::fullness`]). `--fullness-region`
//!   (with or without `--summary`) pools the counters and prints, per
//!   exponent n ∈ {2, 4, 8}, both outcomes over the `(k, τ)` grid, the
//!   feasible region and the largest-margin default, or "region EMPTY"; the
//!   energy-alone sated split; and the per-agent lifetime mean intake / m of
//!   both populations ([`explorers_search::fullness::fullness_report`]).
//!   Its header names the uptake scaling the rows were pinned to (#656).
//!   Each row stores its grid's axes, so rows from before #656's downward
//!   `k` extension read back on their own 17-point grid; pooling rows of
//!   different grids is refused, never misaligned.
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
//! `--satiation-sensitivity C` and `--recognition-distance D` pin every
//! world's need-gate and recognition scales (#619), recorded likewise.
//! `--uptake-structure-exponent b` and `--uptake-reference-structure s_ref`
//! pin every world's size-scaled uptake (#644; for re-reading the region at
//! b = 1, #655), recorded likewise. `--cross-trait-cost c` pins every
//! world's autotrophy × heterotrophy cross-trait cost `c_AH` (#667, #668),
//! recorded likewise.
//!
//!   cargo run --release -p explorers-search --bin role_diet_census -- --atlas atlas.json
//!   cargo run --release -p explorers-search --bin role_diet_census -- --configs sample:31,sample:110
//!   cargo run --release -p explorers-search --bin role_diet_census -- --summary
//!   cargo run --release -p explorers-search --bin role_diet_census -- --summary --region
//!   cargo run --release -p explorers-search --bin role_diet_census -- --atlas atlas.json --fullness
//!   cargo run --release -p explorers-search --bin role_diet_census -- --summary --fullness-region

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

use rayon::prelude::*;

use explorers_genesis::EvalConfig;
use explorers_search::config_source::{
    ConfigSource, parse_founder_aggregation, parse_non_negative, parse_positive, parse_selector,
    resolve_config, sampled_units, with_consumption_scales, with_cross_trait_cost,
    with_founder_aggregation, with_uptake_scaling,
};
use explorers_search::fullness::{FullnessGrid, fullness_report};
use explorers_search::grazer_hunger::SurplusDistribution;
use explorers_search::grazer_hunger::{
    DISTANCE_BANDS, DISTANCE_EDGES, EXPRESSION_TABLE_HEADER, GrazerHunger, HALF_EXPRESSION_BAND,
    HUNGRY_BANDS, RECOGNITION_BANDS, SATIATION_BANDS, SATIATION_EDGES, TraitDistances,
};
use explorers_search::intake_ceiling::{GateSamples, Region, log_grid, region_report, window_line};
use explorers_search::role_diet::{
    Confusion, DeathCounts, DeathTable, GroupReproduction, IntakeCensus, LIGHT_SHARE_BINS,
    PRODUCER_LIGHT_SHARE, ReproductionTable, SeedDiet, SurplusByRole, rollout_with_fullness,
};
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
    /// The satiation sensitivity every world was pinned to
    /// (`--satiation-sensitivity`, #619); absent when worlds ran as decoded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    satiation_sensitivity: Option<f32>,
    /// The recognition distance every world was pinned to
    /// (`--recognition-distance`, #619); absent when worlds ran as decoded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    recognition_distance: Option<f32>,
    /// The intake-ceiling multiple the killing grazers' predicted intake-gate
    /// expression was read at (`--intake-ceiling-k`, #629); absent when not
    /// read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    intake_ceiling_k: Option<f32>,
    /// The size-scaled uptake every world was pinned to
    /// (`--uptake-structure-exponent`, `--uptake-reference-structure`, #644,
    /// #655); absent when worlds ran as decoded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    uptake_structure_exponent: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    uptake_reference_structure: Option<f32>,
    /// The cross-trait cost `c_AH` every world was pinned to
    /// (`--cross-trait-cost`, #668); absent when worlds ran as decoded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cross_trait_cost: Option<f32>,
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
    /// Killing grazers by kinship, distance and drain-time satiation (#606).
    kills: GrazerHunger,
    /// Parent–offspring trait distances (#606).
    births: TraitDistances,
    /// Surplus satiation before growth, by income role (#622).
    surplus: SurplusByRole,
    /// Births and earmark fill by diet group (#624).
    reproduction: ReproductionTable,
    /// The intake-ceiling window's populations (#629).
    intake: IntakeCensus,
    /// The distinct `--intake-ceiling-k` values the pooled rows read kills at.
    intake_ks: Vec<f32>,
    /// Per-sample gate records for the `(k_a, k_h)` region (#634).
    gates: GateSamples,
    /// The fullness-gate region's counters, summed (#637).
    fullness: FullnessGrid,
}

fn pool<'a>(rows: impl Iterator<Item = &'a Row>) -> Pool {
    let mut p = Pool::default();
    for r in rows {
        p.configs += 1;
        if let Some(k) = r.intake_ceiling_k
            && !p.intake_ks.contains(&k)
        {
            p.intake_ks.push(k);
        }
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
            p.kills.merge(&s.kills);
            p.births.merge(&s.births);
            p.surplus.merge(&s.surplus);
            p.reproduction.merge(&s.reproduction);
            p.intake.merge(&s.intake);
            p.gates.merge(&s.gates);
            if let Some(f) = &s.fullness {
                p.fullness.merge(f);
            }
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
    /// Pin every world's satiation sensitivity (#619); `None` as decoded.
    satiation_sensitivity: Option<f32>,
    /// Pin every world's recognition distance (#619); `None` as decoded.
    recognition_distance: Option<f32>,
    /// Read the killing grazers' predicted intake-gate expression at this
    /// ceiling multiple (#629); `None` skips it.
    intake_ceiling_k: Option<f32>,
    /// Pin every world's uptake structure exponent `b` and reference
    /// structure `s_ref` (#644, #655); `None` as decoded.
    uptake_structure_exponent: Option<f32>,
    uptake_reference_structure: Option<f32>,
    /// Pin every world's cross-trait cost `c_AH` (#668); `None` as decoded.
    cross_trait_cost: Option<f32>,
    /// Evaluate the `(k_a, k_h)` ceiling region over the rows' gate samples
    /// and print it after the summary (#634).
    region: bool,
    /// Run the fullness readout into every seed (#637).
    fullness: bool,
    /// Evaluate the fullness-gate region over the rows' counters and print
    /// it after the summary (#637).
    fullness_region: bool,
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
        satiation_sensitivity: None,
        recognition_distance: None,
        intake_ceiling_k: None,
        uptake_structure_exponent: None,
        uptake_reference_structure: None,
        cross_trait_cost: None,
        region: false,
        fullness: false,
        fullness_region: false,
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
            "--region" => args.region = true,
            "--fullness" => args.fullness = true,
            "--fullness-region" => args.fullness_region = true,
            "--founder-aggregation" => {
                args.founder_aggregation = Some(parse_founder_aggregation(&value()?)?)
            }
            "--satiation-sensitivity" => {
                args.satiation_sensitivity = Some(parse_non_negative(&flag, &value()?)?)
            }
            "--recognition-distance" => {
                args.recognition_distance = Some(parse_non_negative(&flag, &value()?)?)
            }
            "--intake-ceiling-k" => {
                args.intake_ceiling_k = Some(parse_non_negative(&flag, &value()?)?)
            }
            "--uptake-structure-exponent" => {
                args.uptake_structure_exponent = Some(parse_non_negative(&flag, &value()?)?)
            }
            "--uptake-reference-structure" => {
                args.uptake_reference_structure = Some(parse_positive(&flag, &value()?)?)
            }
            "--cross-trait-cost" => {
                args.cross_trait_cost = Some(parse_non_negative(&flag, &value()?)?)
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
    let config = with_consumption_scales(
        config,
        args.satiation_sensitivity,
        args.recognition_distance,
    );
    let config = with_uptake_scaling(
        config,
        args.uptake_structure_exponent,
        args.uptake_reference_structure,
    );
    let config = with_cross_trait_cost(config, args.cross_trait_cost);
    let seeds = (0..args.ensemble)
        .into_par_iter()
        .map(|i| {
            rollout_with_fullness(
                &config.0,
                &config.1,
                args.seed + i,
                args.horizon,
                &eval,
                args.intake_ceiling_k,
                args.fullness,
            )
        })
        .collect();
    Row {
        source,
        config_index,
        horizon: args.horizon,
        base_seed: args.seed,
        founder_aggregation: args.founder_aggregation,
        satiation_sensitivity: args.satiation_sensitivity,
        recognition_distance: args.recognition_distance,
        intake_ceiling_k: args.intake_ceiling_k,
        uptake_structure_exponent: args.uptake_structure_exponent,
        uptake_reference_structure: args.uptake_reference_structure,
        cross_trait_cost: args.cross_trait_cost,
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
        let sampled = sampled_units();
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
    if args.region {
        print!("{}", region_text(&rows));
    }
    if args.fullness_region {
        print!("{}", fullness_text(&rows));
    }
}

/// The fullness-gate region over each source's pooled counters, all
/// configs (#637).
fn fullness_text(rows: &[Row]) -> String {
    let mut out = String::new();
    for (name, sample) in [("atlas", false), ("sample", true)] {
        let of = || rows.iter().filter(move |r| r.source.is_sample() == sample);
        if of().next().is_none() {
            continue;
        }
        let p = pool(of());
        out.push_str(&format!(
            "\n## Fullness-gate region (#637): {name}, all configs\n\n{}\n\n",
            uptake_line(of())
        ));
        if p.fullness.k.is_empty() {
            out.push_str("No fullness counters on these rows (run with --fullness).\n");
            continue;
        }
        out.push_str(&fullness_report(&p.fullness));
    }
    out
}

/// The uptake scaling the rows ran at (#656): the pinned `(b, s_ref)` when
/// every row shares one, else that the worlds ran as decoded (each config
/// its own) or under mixed pins.
fn uptake_line<'a>(rows: impl Iterator<Item = &'a Row>) -> String {
    let pins: HashSet<(Option<u32>, Option<u32>)> = rows
        .map(|r| {
            (
                r.uptake_structure_exponent.map(f32::to_bits),
                r.uptake_reference_structure.map(f32::to_bits),
            )
        })
        .collect();
    let show = |v: Option<u32>| v.map_or("decoded".to_string(), |b| f32::from_bits(b).to_string());
    match pins.into_iter().collect::<Vec<_>>()[..] {
        [(None, None)] => {
            "Uptake scaling (#644): as decoded, each config its own b and s_ref (not pinned)."
                .into()
        }
        [(b, s)] => format!(
            "Uptake scaling (#644): pinned, b = {}, s_ref = {}.",
            show(b),
            show(s)
        ),
        _ => "Uptake scaling (#644): MIXED pins across these rows.".into(),
    }
}

/// The `(k_a, k_h)` grid the region is read on (#634): each over
/// [0.25, 20] on a log grid at ≥ 8 points per decade, `k_h` also at 0.
fn region_grid() -> (Vec<f32>, Vec<f32>) {
    let k_a = log_grid(0.25, 20.0, 8.0);
    let k_h = std::iter::once(0.0).chain(k_a.iter().copied()).collect();
    (k_a, k_h)
}

/// The ceiling region over each source's pooled gate samples, all configs
/// (#634).
fn region_text(rows: &[Row]) -> String {
    let (k_a, k_h) = region_grid();
    let mut out = String::new();
    for (name, sample) in [("atlas", false), ("sample", true)] {
        let of = || rows.iter().filter(move |r| r.source.is_sample() == sample);
        if of().next().is_none() {
            continue;
        }
        let p = pool(of());
        out.push_str(&format!(
            "\n## Ceiling region (#634): {name}, all configs\n\nIntake ceiling C = m × (k_a + k_h × h_eff), read on the ungated world.\n"
        ));
        out.push_str(&region_report(&Region::evaluate(&p.gates, &k_a, &k_h)));
    }
    out
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
    println!(
        "\nReproduction by diet group (#624): births over whole runs, booked to each parent's group at the birth (a two-parent birth counts for each parent); earmark fill = energy the grow phase moved into the reproductive earmark in the step before each second-half agent-sample (the accounting's earmark fill), mean per agent-sample.\n"
    );
    println!("| agents | births | agent-samples | mean earmark fill |");
    println!("|---|---:|---:|---:|");
    let r = &p.reproduction;
    print_reproduction("trait producers", &r.trait_producer);
    print_reproduction(
        "trait heterotrophs, light-fed",
        &r.trait_heterotroph_light_fed,
    );
    print_reproduction(
        "trait heterotrophs, heterotroph by diet",
        &r.trait_heterotroph_diet_fed,
    );
    print_reproduction(
        "trait heterotrophs, no income yet",
        &r.trait_heterotroph_no_income,
    );
    print_kills(&p.kills, &p.births);
    print_surplus(&p.surplus);
    print_intake(&p.intake, &p.kills, &p.intake_ks);
}

/// The intake-ceiling window (#629): each population's supply or intake in
/// ticks of maintenance at the start of drain resolution, in both
/// currencies, with both bounds, the window and its default `k`; and the
/// killing grazers' predicted intake-gate expression at `--intake-ceiling-k`.
fn print_intake(ic: &IntakeCensus, kills: &GrazerHunger, ks: &[f32]) {
    println!(
        "\nIntake at the start of drain resolution (#629), over second-half agent-samples, in ticks of maintenance m (drain-time metabolic cost). Energy side: light / m, intake = (light + drain income as energy received) / m, P_E / m (drain potential: capability toward every target in reach before co-feeders split it, as energy received). Nutrient side, matched through growth: uptake / (η·ratio) / m, (uptake + bound nutrient drained) / (η·ratio) / m, P_N / (η·ratio) / m. Light-fed mixotrophs: income producers (#599) with heterotrophy > 0; heterotrophs by diet: the deaths table's group.\n"
    );
    println!(
        "| population | reading | samples | p25 | median | p75 | nutrient samples | nutrient p25 | nutrient median | nutrient p75 |"
    );
    println!("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|");
    let fmt = |v: Option<f64>| v.map_or("–".to_string(), |x| format!("{x:.3}"));
    let quartiles = |d: &SurplusDistribution| {
        format!(
            "{} | {} | {} | {}",
            d.count(),
            fmt(d.percentile(0.25)),
            fmt(d.percentile(0.5)),
            fmt(d.percentile(0.75))
        )
    };
    let rows: [(&str, &str, &SurplusDistribution, &SurplusDistribution); 3] = [
        (
            "light-fed mixotrophs",
            "light (uptake)",
            &ic.mixotroph_light,
            &ic.mixotroph_uptake,
        ),
        (
            "heterotrophs by diet",
            "intake",
            &ic.heterotroph_intake,
            &ic.heterotroph_intake_nutrient,
        ),
        (
            "heterotrophs by diet",
            "drain potential P",
            &ic.heterotroph_potential,
            &ic.heterotroph_potential_nutrient,
        ),
    ];
    for (population, reading, energy, nutrient) in rows {
        println!(
            "| {population} | {reading} | {} | {} |",
            quartiles(energy),
            quartiles(nutrient)
        );
    }
    println!(
        "\nIntake-ceiling window (#629): {}",
        window_line(ic.energy_window(), "light", "intake")
    );
    println!(
        "Intake-ceiling window, nutrient side (#629): {}",
        window_line(ic.nutrient_window(), "uptake", "nutrient intake")
    );
    let k = if ks.is_empty() {
        "–".to_string()
    } else {
        ks.iter()
            .map(|k| format!("{k}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    println!(
        "\nKilling grazers at the intake gate (#629): the same pairs by the grazer's predicted expression E = max(room_E/(room_E+P_E), room_N/(room_N+P_N)) at k = {k} (--intake-ceiling-k), read at the start of drain resolution. Bands: E < 0.1, 0.1–0.5, 0.5–0.9, ≥ 0.9.\n"
    );
    println!("| pairs | read | E < 0.1 | E 0.1–0.5 | E 0.5–0.9 | E ≥ 0.9 | E ≥ 0.5 |");
    println!("|---|---:|---:|---:|---:|---:|---:|");
    for (label, kin) in [("kin", true), ("non-kin", false)] {
        let bands = kills.intake_expression[usize::from(kin)];
        let read: u64 = bands.iter().sum();
        if read == 0 {
            println!("| {label} | – | – | – | – | – | – |");
            continue;
        }
        let half: u64 = bands[HALF_EXPRESSION_BAND..].iter().sum();
        println!(
            "| {label} | {read} | {} | {} | {} | {} | {} ({}%) |",
            bands[0],
            bands[1],
            bands[2],
            bands[3],
            half,
            pct(half, read)
        );
    }
}

fn print_reproduction(label: &str, g: &GroupReproduction) {
    println!(
        "| {label} | {} | {} | {} |",
        g.births,
        g.samples,
        g.mean_earmark_fill()
            .map_or("–".to_string(), |m| format!("{m:.4}")),
    );
}

/// Surplus satiation by income role (#622): ticks of maintenance above the
/// retention buffer, co-limited by free nutrient, read before growth.
fn print_surplus(sp: &SurplusByRole) {
    println!(
        "\nSurplus satiation (#622): s = max(0, min(reserve − buffer, N / (η·ratio))) / m, read after metabolism and before growth, over second-half agent-samples by recent-income role (#599). Light-fed mixotrophs: income producers with heterotrophy > 0. Proposed default c = 1 / s₂₅ over the light-fed mixotrophs with positive surplus (#622).\n"
    );
    println!(
        "| agents | samples | at s = 0 | of which no free nutrient | nutrient-limited | s₂₅ | median | s₇₅ | 1 / s₂₅ | 1 / median | s₂₅ (s > 0) | 1 / s₂₅ (s > 0) |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    let mut heterotrophs = sp.roles[1].clone();
    heterotrophs.merge(&sp.roles[2]);
    let rows: [(&str, &SurplusDistribution); 7] = [
        ("light-fed mixotrophs", &sp.light_fed_mixotrophs),
        (
            "light-fed mixotrophs, energy side alone",
            &sp.light_fed_mixotrophs_energy,
        ),
        ("producers by income", &sp.roles[0]),
        ("consumers by income", &sp.roles[1]),
        ("decomposers by income", &sp.roles[2]),
        ("heterotrophs by income (both)", &heterotrophs),
        ("no income role", &sp.roles[3]),
    ];
    let fmt = |v: Option<f64>| v.map_or("–".to_string(), |x| format!("{x:.3}"));
    let inv = |v: Option<f64>| fmt(v.filter(|&x| x > 0.0).map(|x| 1.0 / x));
    for (label, d) in rows {
        let (s25, s50) = (d.percentile(0.25), d.percentile(0.5));
        let p25 = d.positive_percentile(0.25);
        println!(
            "| {label} | {} | {}% | {}% | {}% | {} | {} | {} | {} | {} | {} | {} |",
            d.count(),
            pct(d.zero, d.count()),
            pct(d.zero_nutrient_limited, d.zero),
            pct(d.nutrient_limited, d.count()),
            fmt(s25),
            fmt(s50),
            fmt(d.percentile(0.75)),
            inv(s25),
            inv(s50),
            fmt(p25),
            inv(p25),
        );
    }
    println!(
        "\nProposed default satiation_sensitivity c = 1 / s₂₅ (light-fed mixotrophs, s > 0): {}",
        fmt(sp.proposed_sensitivity())
    );
}

/// Band labels from upper edges: `[0, e0)`, …, `≥ eN`.
fn band_labels(edges: &[f32]) -> Vec<String> {
    let mut out = Vec::with_capacity(edges.len() + 1);
    let mut lo = 0.0;
    for e in edges {
        out.push(format!("{lo}–{e}"));
        lo = *e;
    }
    out.push(format!("≥ {lo}"));
    out
}

/// The killing grazers' hunger (#606): (grazer, victim) pairs of grazed
/// deaths, by kinship and trait distance, against the grazer's satiation in
/// the drain pass (ticks of maintenance in its scarcer currency).
fn print_kills(k: &GrazerHunger, births: &TraitDistances) {
    let sat = band_labels(&SATIATION_EDGES);
    let dist = band_labels(&DISTANCE_EDGES);
    println!(
        "\nKilling grazers (#606): (grazer, victim) pairs of grazed deaths by the grazer's drain-time satiation, in ticks of maintenance in its scarcer currency (< 10 = under half expression at c = 0.1: hungry).\n"
    );
    println!(
        "| pairs | total | hungry (< 10) | within d < 0.5 | {} | nutrient-limited |",
        sat.iter()
            .map(|l| format!("s {l}"))
            .collect::<Vec<_>>()
            .join(" | ")
    );
    println!(
        "|---|---:|---:|---:|{}---:|",
        "---:|".repeat(SATIATION_BANDS)
    );
    for (label, kin) in [("kin", true), ("non-kin", false)] {
        let total = k.total(kin);
        let by = k.by_satiation(kin);
        let nl: u64 = k.nutrient_limited[usize::from(kin)].iter().sum();
        println!(
            "| {label} | {total} | {} ({}%) | {} ({}%) | {} | {} ({}%) |",
            k.hungry(kin),
            pct(k.hungry(kin), total),
            k.within(kin, RECOGNITION_BANDS),
            pct(k.within(kin, RECOGNITION_BANDS), total),
            by.iter()
                .map(|n| format!("{n}"))
                .collect::<Vec<_>>()
                .join(" | "),
            nl,
            pct(nl, total),
        );
    }
    println!("\n| pairs by trait distance | {} |", dist.join(" | "));
    println!("|---|{}", "---:|".repeat(DISTANCE_BANDS));
    for (label, kin) in [("kin", true), ("non-kin", false)] {
        let by = k.by_distance(kin);
        let hungry: Vec<String> = k.pairs[usize::from(kin)]
            .iter()
            .zip(by)
            .map(|(row, n)| {
                let h: u64 = row[..HUNGRY_BANDS].iter().sum();
                format!("{n} ({}% hungry)", pct(h, n))
            })
            .collect();
        println!("| {label} | {} |", hungry.join(" | "));
    }
    println!(
        "\nKilling grazers at the pre-growth surplus read (#624): the same pairs by the grazer's surplus s (reserve above the retention buffer, co-limited by free nutrient, in ticks of maintenance) read after metabolism and before growth, and its expression E = 1/(1 + c·s) at each run's c (pinned or as decoded). E ≥ 0.5 = at most half gated (hungry side); E < 0.5 = past half expression (sated side); s = 0 = no surplus (E = 1). Bands: E < 0.1, 0.1–0.5, 0.5–0.9, ≥ 0.9.\n"
    );
    println!("| pairs | {EXPRESSION_TABLE_HEADER} |");
    println!(
        "|---|{}",
        "---:|".repeat(EXPRESSION_TABLE_HEADER.matches('|').count() + 1)
    );
    println!("| all | {} |", k.expression_cells());
    println!(
        "\nParent–offspring trait distance over {} births: mean {}, rms {}; by band {}.",
        births.count(),
        births.mean().map_or("–".into(), |m| format!("{m:.4}")),
        births.rms().map_or("–".into(), |m| format!("{m:.4}")),
        dist.iter()
            .zip(births.bands)
            .map(|(l, n)| format!("{l}: {n}"))
            .collect::<Vec<_>>()
            .join(", ")
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
    use explorers_search::grazer_hunger::{KillerReading, Satiation, Surplus};
    use explorers_search::intake_ceiling::GateSample;
    use explorers_sim::TraitVector;

    fn seed(tag: [bool; 3], diet: [bool; 3], failure: Option<&str>) -> SeedDiet {
        let mut confusion = Confusion::default();
        confusion.0[1][0] = 3;
        confusion.0[1][1] = 1;
        let mut kills = GrazerHunger::default();
        kills.record(
            true,
            0.05,
            KillerReading::at(
                Satiation {
                    energy: 2.0,
                    nutrient: 0.5,
                },
                Surplus {
                    energy: 0.0,
                    nutrient: 0.5,
                },
                33.0,
            ),
        );
        let mut births = TraitDistances::default();
        let t = TraitVector {
            photosynthetic_absorption: 1.0,
            heterotrophy: 0.0,
            mobility: 0.0,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 0.5,
            dispersal: 0.3,
        };
        births.record(&t, &t);
        let mut surplus = SurplusByRole::default();
        surplus.record(
            Some(explorers_sim::topology::TrophicRole::Producer),
            0.2,
            &Surplus {
                energy: 4.0,
                nutrient: 9.0,
            },
        );
        surplus.record(
            None,
            0.0,
            &Surplus {
                energy: 3.0,
                nutrient: 0.0,
            },
        );
        let mut intake = IntakeCensus::default();
        intake.mixotroph_light.record(4.0);
        intake.heterotroph_intake.record(1.0);
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
            kills,
            births,
            surplus,
            reproduction: ReproductionTable::default(),
            intake,
            fullness: None,
            gates: GateSamples {
                mixotroph_kin_kills: vec![GateSample {
                    h_eff: 0.05,
                    maintenance: 1.0,
                    light: 2.0,
                    potential_energy: 0.2,
                    intake: 2.2,
                    ..Default::default()
                }],
                diet_fed: vec![GateSample {
                    h_eff: 1.0,
                    maintenance: 1.0,
                    potential_energy: 4.0,
                    intake: 4.0,
                    ..Default::default()
                }],
            },
        }
    }

    /// `--region` evaluates the `(k_a, k_h)` grid (#634) over every row's
    /// pooled gate samples: k_a and k_h on a log grid over [0.25, 20] at
    /// ≥ 8 points per decade, k_h also at 0 (the body-only ceiling of #629).
    #[test]
    fn region_evaluates_the_grid_over_pooled_gate_samples() {
        let parse = |a: &[&str]| parse_args(a.iter().map(|s| s.to_string()));
        assert!(!parse(&[]).unwrap().region);
        assert!(parse(&["--summary", "--region"]).unwrap().region);
        let row = Row {
            source: ConfigSource::Atlas,
            config_index: 0,
            horizon: 2000,
            base_seed: 1000,
            founder_aggregation: None,
            satiation_sensitivity: None,
            recognition_distance: None,
            intake_ceiling_k: None,
            uptake_structure_exponent: None,
            uptake_reference_structure: None,
            cross_trait_cost: None,
            seeds: vec![
                seed([true; 3], [true; 3], None),
                seed([true; 3], [true; 3], None),
            ],
        };
        let p = pool(std::iter::once(&row));
        assert_eq!(p.gates.mixotroph_kin_kills.len(), 2, "gates pooled");
        let text = region_text(&[row]);
        assert!(
            text.contains("## Ceiling region (#634): atlas, all configs"),
            "{text}"
        );
        assert!(text.contains("n = 2 kin-kill pairs") && text.contains("n = 2 samples"));
        assert!(text.contains("| k_a \\ k_h | 0 | 0.25 |"), "{text}");
        assert!(text.contains("| 20 | "), "{text}");
        assert!(
            text.contains("k_h = 0 (the body-only ceiling of #629"),
            "{text}"
        );
    }

    /// `--fullness` runs the fullness readout (#637) into every seed;
    /// `--fullness-region` pools the rows' counters (a sum) and prints the
    /// region per exponent, the energy-alone split and the per-agent
    /// separation. Rows without the readout pool as empty.
    #[test]
    fn fullness_runs_the_readout_and_its_region_pools_the_rows() {
        let parse = |a: &[&str]| parse_args(a.iter().map(|s| s.to_string()));
        let a = parse(&[]).unwrap();
        assert!(!a.fullness && !a.fullness_region);
        let args = parse(&["--fullness", "--max-ticks", "120", "--ensemble", "2"]).unwrap();
        assert!(args.fullness);
        assert!(
            parse(&["--summary", "--fullness-region"])
                .unwrap()
                .fullness_region
        );
        let sampled = sampled_units();
        let decoded = resolve_config(ConfigSource::SAMPLE, 31, &Default::default(), &sampled);
        let row = run_row(ConfigSource::SAMPLE, 31, &decoded, &args);
        let grids: Vec<_> = row
            .seeds
            .iter()
            .map(|s| s.fullness.clone().unwrap())
            .collect();
        let mut old = row.clone();
        old.seeds.iter_mut().for_each(|s| s.fullness = None);
        let p = pool([&row, &old].into_iter());
        assert_eq!(p.fullness.kills, grids[0].kills + grids[1].kills);
        assert_eq!(
            p.fullness.consumer_ticks,
            grids[0].consumer_ticks + grids[1].consumer_ticks
        );
        let text = fullness_text(&[row, old]);
        assert!(
            text.contains("## Fullness-gate region (#637): sample, all configs"),
            "{text}"
        );
        assert!(text.contains("### n = 4"), "{text}");
        assert!(text.contains("heterotrophs by diet |"), "{text}");
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
            satiation_sensitivity: None,
            recognition_distance: None,
            intake_ceiling_k: None,
            uptake_structure_exponent: None,
            uptake_reference_structure: None,
            cross_trait_cost: None,
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
        assert_eq!(p.kills.pairs[1][0][0], 3, "kills summed over seeds");
        assert_eq!(p.kills.nutrient_limited[1][0], 3);
        assert_eq!(p.births.count(), 3);
        assert_eq!(p.surplus.light_fed_mixotrophs.count(), 3, "surplus summed");
        assert_eq!(p.surplus.roles[3].zero, 3);
        assert_eq!(p.surplus.roles[3].zero_nutrient_limited, 3);
        assert_eq!(p.intake.mixotroph_light.count(), 3, "intake summed");
        assert_eq!(p.intake.heterotroph_intake.count(), 3);
    }

    /// `--intake-ceiling-k K` reads the killing grazers' predicted
    /// intake-gate expression at `K` (#629) and records it on the row; rows
    /// without it read and write as before.
    #[test]
    fn an_intake_ceiling_k_reaches_the_kills_and_the_row() {
        let parse = |a: &[&str]| parse_args(a.iter().map(|s| s.to_string()));
        assert_eq!(parse(&[]).unwrap().intake_ceiling_k, None);
        assert!(parse(&["--intake-ceiling-k", "-1"]).is_err());
        let args = parse(&[
            "--intake-ceiling-k",
            "4",
            "--max-ticks",
            "300",
            "--ensemble",
            "1",
        ])
        .unwrap();
        assert_eq!(args.intake_ceiling_k, Some(4.0));
        let sampled = sampled_units();
        let decoded = resolve_config(ConfigSource::SAMPLE, 31, &Default::default(), &sampled);
        let row = run_row(ConfigSource::SAMPLE, 31, &decoded, &args);
        assert_eq!(row.intake_ceiling_k, Some(4.0));
        let kills = &row.seeds[0].kills;
        let banded: u64 = kills.intake_expression.iter().flatten().sum();
        assert_eq!(banded, kills.total(true) + kills.total(false));
        let back: Row = serde_json::from_str(&serde_json::to_string(&row).unwrap()).unwrap();
        assert_eq!(back, row);
        let mut json: serde_json::Value = serde_json::to_value(&row).unwrap();
        json.as_object_mut().unwrap().remove("intake_ceiling_k");
        let old: Row = serde_json::from_value(json).unwrap();
        assert_eq!(old.intake_ceiling_k, None);
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

        let sampled = sampled_units();
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

    /// `--satiation-sensitivity C` and `--recognition-distance D` pin every
    /// resolved world's need-gate and recognition scales (#619), and each row
    /// records them; unpinned rows read and write as before.
    #[test]
    fn consumption_scale_pins_reach_the_world_and_the_row() {
        let parse = |a: &[&str]| parse_args(a.iter().map(|s| s.to_string()));
        let a = parse(&[]).unwrap();
        assert_eq!(
            (a.satiation_sensitivity, a.recognition_distance),
            (None, None)
        );
        assert!(parse(&["--satiation-sensitivity", "-1"]).is_err());
        assert!(parse(&["--recognition-distance", "x"]).is_err());
        let args = parse(&[
            "--satiation-sensitivity",
            "3",
            "--recognition-distance",
            "1.5",
            "--founder-aggregation",
            "0",
            "--max-ticks",
            "60",
            "--ensemble",
            "1",
        ])
        .unwrap();
        assert_eq!(
            (args.satiation_sensitivity, args.recognition_distance),
            (Some(3.0), Some(1.5))
        );

        let sampled = sampled_units();
        let decoded = resolve_config(ConfigSource::SAMPLE, 31, &Default::default(), &sampled);
        let pinned = run_row(ConfigSource::SAMPLE, 31, &decoded, &args);
        assert_eq!(
            (pinned.satiation_sensitivity, pinned.recognition_distance),
            (Some(3.0), Some(1.5))
        );
        let unpinned_args = parse(&[
            "--founder-aggregation",
            "0",
            "--max-ticks",
            "60",
            "--ensemble",
            "1",
        ])
        .unwrap();
        let unpinned = run_row(ConfigSource::SAMPLE, 31, &decoded, &unpinned_args);
        let json = serde_json::to_string(&unpinned).unwrap();
        assert!(!json.contains("satiation_sensitivity") && !json.contains("recognition_distance"));
        assert_ne!(pinned.seeds, unpinned.seeds, "the pins change the rollout");
        let back: Row = serde_json::from_str(&serde_json::to_string(&pinned).unwrap()).unwrap();
        assert_eq!(back, pinned);
    }

    /// `--uptake-structure-exponent b` and `--uptake-reference-structure
    /// s_ref` pin every resolved world's size-scaled uptake (#644, #655), and
    /// each row records them; unpinned rows read and write as before.
    #[test]
    fn uptake_scaling_pins_reach_the_world_and_the_row() {
        let parse = |a: &[&str]| parse_args(a.iter().map(|s| s.to_string()));
        let a = parse(&[]).unwrap();
        assert_eq!(
            (a.uptake_structure_exponent, a.uptake_reference_structure),
            (None, None)
        );
        assert!(parse(&["--uptake-structure-exponent", "-1"]).is_err());
        assert!(parse(&["--uptake-reference-structure", "0"]).is_err());
        let base = ["--max-ticks", "60", "--ensemble", "1"];
        let pinned_flags = [
            "--uptake-structure-exponent",
            "1",
            "--uptake-reference-structure",
            "100",
        ];
        let args = parse(&[&pinned_flags[..], &base[..]].concat()).unwrap();
        assert_eq!(
            (
                args.uptake_structure_exponent,
                args.uptake_reference_structure
            ),
            (Some(1.0), Some(100.0))
        );

        let sampled = sampled_units();
        let decoded = resolve_config(ConfigSource::SAMPLE, 31, &Default::default(), &sampled);
        let pinned = run_row(ConfigSource::SAMPLE, 31, &decoded, &args);
        assert_eq!(
            (
                pinned.uptake_structure_exponent,
                pinned.uptake_reference_structure
            ),
            (Some(1.0), Some(100.0))
        );
        let unpinned = run_row(ConfigSource::SAMPLE, 31, &decoded, &parse(&base).unwrap());
        let json = serde_json::to_string(&unpinned).unwrap();
        assert!(!json.contains("uptake_structure_exponent"));
        assert!(!json.contains("uptake_reference_structure"));
        assert_ne!(pinned.seeds, unpinned.seeds, "the pins change the rollout");
        let back: Row = serde_json::from_str(&serde_json::to_string(&pinned).unwrap()).unwrap();
        assert_eq!(back, pinned);
    }

    /// `--cross-trait-cost c` pins every resolved world's `c_AH` (#668), and
    /// each row records it; unpinned rows read and write as before.
    #[test]
    fn a_cross_trait_cost_pin_reaches_the_world_and_the_row() {
        let parse = |a: &[&str]| parse_args(a.iter().map(|s| s.to_string()));
        assert_eq!(parse(&[]).unwrap().cross_trait_cost, None);
        assert!(parse(&["--cross-trait-cost", "-1"]).is_err());
        let base = ["--max-ticks", "60", "--ensemble", "1"];
        let args = parse(&[&["--cross-trait-cost", "5"][..], &base[..]].concat()).unwrap();
        assert_eq!(args.cross_trait_cost, Some(5.0));

        let sampled = sampled_units();
        let decoded = resolve_config(ConfigSource::SAMPLE, 31, &Default::default(), &sampled);
        let pinned = run_row(ConfigSource::SAMPLE, 31, &decoded, &args);
        assert_eq!(pinned.cross_trait_cost, Some(5.0));
        let unpinned = run_row(ConfigSource::SAMPLE, 31, &decoded, &parse(&base).unwrap());
        let json = serde_json::to_string(&unpinned).unwrap();
        assert!(!json.contains("cross_trait_cost"));
        assert_ne!(pinned.seeds, unpinned.seeds, "the pin changes the rollout");
        let back: Row = serde_json::from_str(&serde_json::to_string(&pinned).unwrap()).unwrap();
        assert_eq!(back, pinned);
    }

    /// #656: the fullness region's header names the uptake scaling, so a
    /// report says which `b` it was read at without its filename.
    #[test]
    fn the_region_header_names_the_uptake_scaling() {
        let row = |b: Option<f32>, s: Option<f32>| Row {
            source: ConfigSource::Atlas,
            config_index: 0,
            horizon: 60,
            base_seed: 1000,
            founder_aggregation: None,
            satiation_sensitivity: None,
            recognition_distance: None,
            intake_ceiling_k: None,
            uptake_structure_exponent: b,
            uptake_reference_structure: s,
            cross_trait_cost: None,
            seeds: Vec::new(),
        };
        let pinned = [row(Some(1.0), Some(100.0)), row(Some(1.0), Some(100.0))];
        assert_eq!(
            uptake_line(pinned.iter()),
            "Uptake scaling (#644): pinned, b = 1, s_ref = 100."
        );
        let decoded = [row(None, None)];
        assert!(uptake_line(decoded.iter()).contains("as decoded"));
        let mixed = [row(None, None), row(Some(1.0), Some(100.0))];
        assert!(uptake_line(mixed.iter()).contains("MIXED"));
        assert!(fullness_text(&pinned).contains("pinned, b = 1, s_ref = 100"));
    }
}
