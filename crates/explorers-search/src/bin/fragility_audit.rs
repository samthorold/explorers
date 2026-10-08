//! The fragility audit (#693): for each atlas cell, how often its outcome
//! flips under small perturbations — of the seed and of its parameters.
//!
//! Each cell is evaluated as the search evaluates it (`run_single` with the
//! search's evaluator, `QdConfig::default().eval_config()`: the predictive
//! bloom stop on at `DEFAULT_BLOOM_STOP`, moved by `--bloom-stop TICK:FACTOR`
//! or turned off by `--no-bloom-stop`; the search's horizon 2000,
//! `--horizon`; and the search's per-rollout budget `SEARCH_ROLLOUT_BUDGET`)
//! on an ensemble of `--ensemble` seeds (default 10, from `--seed`, default
//! 1000; the search's own seeds are not recorded per cell),
//! first unperturbed (the baseline, radius 0), then at each of `--radii`
//! (default 0.01, 0.03, 0.1) on `--draws` perturbed copies of its unit
//! vector (default 8): Gaussian noise of standard deviation the radius per
//! dimension of the atlas's own unit box, clamped to `[0, 1]`, deterministic
//! in `--jitter-seed` (`explorers_search::fragility::jitter`). Every row —
//! one per cell × radius × draw — records the jitter vector, each seed's
//! verdict (`live`, a failure mode — `bloom_stop` among them — or an
//! unfinished `timeout` / `eval_timeout`) and fitness, the persisted
//! fraction, the modal verdict, the bloom stop it ran under and the search's
//! a-priori prefilter cliff on the world, if any. The search would record a
//! gated world dead on that cliff without a rollout; the audit rolls it out
//! anyway and the summary tallies the gated rows apart.
//!
//! ## Hyphal uptake (#729)
//!
//! `--hyphal-uptake on|off` and `--contact-distance <d_c>` (positive) pin
//! the hyphal uptake switch and contact distance (world-rules.md flow 2,
//! *Hyphal uptake*) on every audited world, after decoding and jitter, as
//! the census's flags of the same names do (#728). A pinned run writes the
//! world's effective switch and `d_c` into every row (`hyphal_uptake`,
//! `contact_distance`); an unpinned run writes neither, so its rows are
//! the pre-#729 audit's byte for byte and older rows read as switch off. A
//! run refuses an `--out` holding rows at another setting: give each arm
//! its own file. `--draws 0` audits the unperturbed cells alone.
//!
//! ## Network (#741)
//!
//! `--network-connection-cap <n>`, `--network-creation-cost <e>`,
//! `--network-maintenance-cost <e>`, `--network-redistribution-rate <f>` and
//! `--network-transfer-efficiency <f>` pin the network (world-rules.md
//! flow 5) on every audited world, after decoding, jitter and the hyphal
//! pins, named and parsed as the census's flags (#646). When any is pinned,
//! every row carries the world's five effective network parameters as a
//! `network` object; an unpinned run writes none, so its rows are the
//! pre-#741 audit's byte for byte and older rows read as network off. As
//! for the hyphal setting, a run refuses an `--out` holding rows at another
//! network setting.
//!
//! The summary (printed after every run, or alone with `--summary`) reads
//! per cell and radius the **flip rate** — the share of finished (draw,
//! seed) evaluations whose verdict differs from the baseline's on the same
//! seed — and the fitness spread; per cell the **seed-only flip rate**, the
//! share of baseline seeds whose verdict differs from the cell's modal
//! verdict (the noise floor), and the **basin width**, the largest radius
//! whose flip rate, and every smaller radius's, is below 20 %; atlas-wide
//! distributions of both; a **dimension attribution** (Spearman's ρ of
//! |jitter| per parameter against each draw's flip share, within radius);
//! and the flip rate against each cell's behaviour-axis coordinates and
//! fitness.
//!
//! ## Resumable by construction
//!
//! A cell's remaining rows run in parallel (every row × seed rollout at
//! once) and are appended to `--out` (default
//! `target/fragility-audit.jsonl`) in order when the cell finishes; rows
//! already present are skipped, so a killed run loses at most one cell.
//! `--limit N` runs at most `N` further cells; `--configs atlas:0,atlas:5`
//! selects cells. Each rollout carries the search's budgets
//! (`--run-timeout-secs`, `--eval-timeout-secs`, default 600 each); one
//! that exhausts either is recorded unfinished, never dropped.
//!
//! ## Running
//!
//!   scripts/fragility-audit.sh                          # the committed atlas
//!   cargo run --release -p explorers-search --bin fragility_audit -- \
//!     --configs atlas:0,atlas:1 --draws 2 --horizon 300  # a smoke run
//!   ./target/release/fragility_audit --summary

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use explorers_search::config_source::{ConfigSource, NETWORK_FLAGS, parse_selector};
use explorers_search::fragility::{
    AXES, FragilityRow, Rollouts, Summary, evaluate_rows, plan_rows, ran_at_hyphal_setting,
    ran_at_network_setting, read_cell_meta, summarise,
};
use explorers_search::search::{BLOOM_STOP_FLAG, NO_BLOOM_STOP_FLAG, parse_bloom_stop};
use explorers_search::sweep::{
    EVAL_TIMEOUT_FLAG, RUN_TIMEOUT_FLAG, append_row, plan_tasks, read_atlas_units, read_rows,
};

const DEFAULT_OUT: &str = "target/fragility-audit.jsonl";
const DEFAULT_SEED: u64 = 1000;
const DEFAULT_ENSEMBLE: u64 = 10;
const DEFAULT_DRAWS: usize = 8;
const DEFAULT_RADII: [f64; 3] = [0.01, 0.03, 0.1];
const DEFAULT_JITTER_SEED: u64 = 693;

#[derive(Clone, Debug)]
struct Args {
    atlas: PathBuf,
    out: PathBuf,
    json: Option<PathBuf>,
    configs: Option<HashSet<(ConfigSource, usize)>>,
    limit: Option<usize>,
    /// The horizon, evaluator (bloom stop) and per-rollout budget: the
    /// search's unless a flag moves them.
    rollouts: Rollouts,
    seed: u64,
    ensemble: u64,
    draws: usize,
    radii: Vec<f64>,
    jitter_seed: u64,
    summary_only: bool,
}

fn parse_args<I: IntoIterator<Item = String>>(argv: I) -> Args {
    let mut args = Args {
        atlas: PathBuf::from("atlas.json"),
        out: PathBuf::from(DEFAULT_OUT),
        json: None,
        configs: None,
        limit: None,
        rollouts: Rollouts::default(),
        seed: DEFAULT_SEED,
        ensemble: DEFAULT_ENSEMBLE,
        draws: DEFAULT_DRAWS,
        radii: DEFAULT_RADII.to_vec(),
        jitter_seed: DEFAULT_JITTER_SEED,
        summary_only: false,
    };
    let mut it = argv.into_iter();
    let number = |flag: &str, raw: String| -> u64 {
        raw.parse()
            .unwrap_or_else(|_| panic!("fragility_audit: {flag} {raw:?} is not an integer"))
    };
    while let Some(flag) = it.next() {
        let mut value = |flag: &str| -> String {
            it.next()
                .unwrap_or_else(|| panic!("fragility_audit: {flag} needs a value"))
        };
        match flag.as_str() {
            "--atlas" => args.atlas = PathBuf::from(value("--atlas")),
            "--out" => args.out = PathBuf::from(value("--out")),
            "--json" => args.json = Some(PathBuf::from(value("--json"))),
            "--configs" => {
                let set = parse_selector(&value("--configs"), "--configs", None);
                assert!(
                    set.iter().all(|(s, _)| *s == ConfigSource::Atlas),
                    "fragility_audit: --configs takes atlas cells only (atlas:i)"
                );
                args.configs = Some(set);
            }
            "--limit" => args.limit = Some(number("--limit", value("--limit")) as usize),
            "--horizon" => {
                args.rollouts.horizon = number("--horizon", value("--horizon"));
                assert!(
                    args.rollouts.horizon > 0,
                    "fragility_audit: --horizon must be positive"
                );
            }
            "--seed" => args.seed = number("--seed", value("--seed")),
            "--ensemble" => {
                args.ensemble = number("--ensemble", value("--ensemble"));
                assert!(
                    args.ensemble > 0,
                    "fragility_audit: --ensemble must be positive"
                );
            }
            "--draws" => args.draws = number("--draws", value("--draws")) as usize,
            "--radii" => {
                let raw = value("--radii");
                args.radii = raw
                    .split(',')
                    .map(|r| {
                        r.trim()
                            .parse::<f64>()
                            .ok()
                            .filter(|r| r.is_finite() && *r > 0.0)
                            .unwrap_or_else(|| {
                                panic!("fragility_audit: --radii {raw:?}: each must be > 0")
                            })
                    })
                    .collect();
                args.radii.sort_by(f64::total_cmp);
                args.radii.dedup();
            }
            "--jitter-seed" => args.jitter_seed = number("--jitter-seed", value("--jitter-seed")),
            RUN_TIMEOUT_FLAG => {
                args.rollouts.budget.simulation =
                    Duration::from_secs(number(RUN_TIMEOUT_FLAG, value(RUN_TIMEOUT_FLAG)))
            }
            EVAL_TIMEOUT_FLAG => {
                args.rollouts.budget.evaluation =
                    Duration::from_secs(number(EVAL_TIMEOUT_FLAG, value(EVAL_TIMEOUT_FLAG)))
            }
            BLOOM_STOP_FLAG => {
                args.rollouts.eval_config.bloom_stop = Some(
                    parse_bloom_stop(&value(BLOOM_STOP_FLAG))
                        .unwrap_or_else(|e| panic!("fragility_audit: {e}")),
                )
            }
            NO_BLOOM_STOP_FLAG => args.rollouts.eval_config.bloom_stop = None,
            "--hyphal-uptake" => {
                args.rollouts.hyphal_uptake = Some(match value("--hyphal-uptake").as_str() {
                    "on" => true,
                    "off" => false,
                    v => panic!("fragility_audit: --hyphal-uptake takes on or off, not {v:?}"),
                })
            }
            "--contact-distance" => {
                let raw = value("--contact-distance");
                args.rollouts.contact_distance = Some(
                    raw.parse::<f32>()
                        .ok()
                        .filter(|d| d.is_finite() && *d > 0.0)
                        .unwrap_or_else(|| {
                            panic!(
                                "fragility_audit: --contact-distance {raw:?} must be a positive number"
                            )
                        }),
                )
            }
            f if NETWORK_FLAGS.contains(&f) => args
                .rollouts
                .network
                .set(f, &value(f))
                .unwrap_or_else(|e| panic!("fragility_audit: {e}")),
            "--summary" => args.summary_only = true,
            other => panic!("fragility_audit: unknown argument {other:?}"),
        }
    }
    args
}

fn opt(v: Option<f64>) -> String {
    v.map_or("-".to_string(), |v| format!("{v:.3}"))
}

fn print_summary(s: &Summary, names_len: usize, atlas_fingerprint: u64) {
    println!("# Fragility audit (#693)");
    println!(
        "# horizon(s) {:?}; bloom stop(s) {:?}; radii {:?}; {} cells read; {} parameters",
        s.horizons,
        s.bloom_stops,
        s.radii,
        s.cells.len(),
        names_len
    );
    if s.atlas_fingerprints.iter().any(|&f| f != atlas_fingerprint) {
        println!(
            "# WARNING: rows ran on atlas fingerprint(s) {:?}, not this atlas's {atlas_fingerprint}",
            s.atlas_fingerprints
        );
    }
    println!(
        "# flip rate: share of finished (draw, seed) evaluations whose verdict differs from the \
         unperturbed cell's on the same seed; vs-modal: from the cell's modal verdict; seed-only: \
         unperturbed seeds off the modal verdict (the noise floor); basin: largest radius with it \
         and every smaller radius below 20 %."
    );

    println!("\n## Per cell: seed-only flip rate and flip rate by radius");
    let header: Vec<String> = s.radii.iter().map(|r| format!("r={r}")).collect();
    println!(
        "  cell  modal                 persist  seed-flip  {}  basin   (each radius: flip / vs-modal / fitness sd)",
        header.join("  ")
    );
    for c in &s.cells {
        let by: Vec<String> = c
            .by_radius
            .iter()
            .map(|r| {
                format!(
                    "{}/{}/{}",
                    opt(r.flip_rate),
                    opt(r.flip_rate_vs_modal),
                    opt(r.fitness_sd)
                )
            })
            .collect();
        println!(
            "  {:>4}  {:<20}  {:>7}  {:>9}  {}  {}",
            c.config_index,
            c.modal_verdict.as_deref().unwrap_or("-"),
            opt(c.persisted_fraction),
            opt(c.seed_flip_rate),
            by.join("  "),
            c.basin_width
                .map_or("unread".to_string(), |b| format!("{b}"))
        );
    }

    println!("\n## Atlas-wide distribution");
    let q = |q: &Option<explorers_search::fragility::Quantiles>| {
        q.as_ref().map_or("-".to_string(), |q| {
            format!(
                "n {} min {:.3} p25 {:.3} median {:.3} p75 {:.3} max {:.3} mean {:.3}",
                q.n, q.min, q.p25, q.median, q.p75, q.max, q.mean
            )
        })
    };
    println!("  seed-only flip rate         {}", q(&s.seed_flip_rate));
    println!(
        "  baseline fitness sd         {}",
        q(&s.baseline_fitness_sd)
    );
    for d in &s.flip_rate_by_radius {
        println!("  r={:<6} flip rate          {}", d.radius, q(&d.flip_rate));
        println!(
            "  r={:<6} flip rate vs modal {}",
            d.radius,
            q(&d.flip_rate_vs_modal)
        );
        println!(
            "  r={:<6} fitness sd         {}",
            d.radius,
            q(&d.fitness_sd)
        );
    }

    println!("\n## Basin width (largest radius with flip rate < 20 %)");
    for (w, n) in &s.basin_widths {
        println!("  {w:>8}: {n} cells");
    }

    println!(
        "\n## Dimension attribution (Spearman ρ of |jitter|/radius vs draw flip share, within radius)"
    );
    for a in &s.attribution {
        println!(
            "  {:>2} {:<40} ρ {:>7}  ({} draws)",
            a.index,
            a.name,
            opt(a.rho),
            a.draws
        );
    }

    println!("\n## Fragility vs behaviour axes and fitness (Spearman ρ over cells)");
    println!(
        "  rate               {:>12} {:>12} {:>12} {:>12}",
        AXES[0], AXES[1], AXES[2], "fitness"
    );
    let row = |label: String, r: &explorers_search::fragility::Regional| {
        println!(
            "  {:<18} {:>12} {:>12} {:>12} {:>12}",
            label,
            opt(r.vs_axis[0]),
            opt(r.vs_axis[1]),
            opt(r.vs_axis[2]),
            opt(r.vs_fitness)
        );
    };
    row("seed-only".to_string(), &s.regional_seed);
    for r in &s.regional {
        row(format!("flip r={}", r.radius), r);
    }

    println!(
        "\n## Prefilter (worlds the search would gate a priori, recorded dead without a rollout)"
    );
    let p = &s.prefilter;
    println!(
        "  {} gated rows; {} finished seeds on them, {} live (prefilter disagreements)",
        p.gated_rows, p.gated_finished_seeds, p.gated_live_seeds
    );
    for (cliff, n) in &p.by_cliff {
        println!("  {cliff}: {n} rows");
    }

    println!("\n## Unfinished evaluations (recorded, unread)");
    if s.unfinished.is_empty() {
        println!("  none");
    }
    for (mode, n) in &s.unfinished {
        println!("  {mode}: {n}");
    }
}

fn main() {
    let args = parse_args(std::env::args().skip(1));
    let atlas = read_atlas_units(&args.atlas);
    if !args.summary_only {
        let cells: Vec<usize> =
            plan_tasks(atlas.len(), 0, args.configs.as_ref(), &HashSet::new(), None)
                .into_iter()
                .map(|(_, i)| i)
                .collect();
        let done: Vec<FragilityRow> = read_rows(&args.out);
        if let Some(other) = done
            .iter()
            .find(|r| !ran_at_hyphal_setting(r, &args.rollouts))
        {
            panic!(
                "fragility_audit: {} holds rows at another hyphal setting (atlas:{} ran at \
                 hyphal_uptake {:?}, contact_distance {:?}; this run pins {:?}, {:?}): \
                 give each setting its own --out",
                args.out.display(),
                other.config_index,
                other.hyphal_uptake,
                other.contact_distance,
                args.rollouts.hyphal_uptake,
                args.rollouts.contact_distance
            );
        }
        if let Some(other) = done
            .iter()
            .find(|r| !ran_at_network_setting(r, &args.rollouts))
        {
            panic!(
                "fragility_audit: {} holds rows at another network setting (atlas:{} ran at \
                 network {:?}; this run pins {:?}): give each setting its own --out",
                args.out.display(),
                other.config_index,
                other.network,
                args.rollouts.network
            );
        }
        let mut plan = plan_rows(&cells, &args.radii, args.draws, &done);
        plan.truncate(args.limit.unwrap_or(usize::MAX));
        let seeds: Vec<u64> = (0..args.ensemble).map(|i| args.seed + i).collect();
        eprintln!(
            "fragility_audit: {} cells selected, {} rows done in {}; {} seeds × (1 + {} radii × {} draws), horizon {}, bloom stop {:?}, budget {:?}, hyphal uptake {:?} d_c {:?}, network {:?}; running {} cells now",
            cells.len(),
            done.len(),
            args.out.display(),
            seeds.len(),
            args.radii.len(),
            args.draws,
            args.rollouts.horizon,
            args.rollouts.eval_config.bloom_stop,
            args.rollouts.budget,
            args.rollouts.hyphal_uptake,
            args.rollouts.contact_distance,
            args.rollouts.network,
            plan.len()
        );
        let start = Instant::now();
        let total = plan.len();
        for (n, (cell, keys)) in plan.into_iter().enumerate() {
            let t0 = Instant::now();
            let rows = evaluate_rows(&atlas, &keys, &seeds, &args.rollouts, args.jitter_seed);
            for row in &rows {
                append_row(&args.out, row);
            }
            eprintln!(
                "  atlas:{cell} done ({}/{total}): {} rows in {:.0}s, {:.0}s elapsed",
                n + 1,
                rows.len(),
                t0.elapsed().as_secs_f64(),
                start.elapsed().as_secs_f64()
            );
        }
    }
    let rows: Vec<FragilityRow> = read_rows(&args.out);
    let mut radii: Vec<f64> = rows.iter().map(|r| r.radius).filter(|&r| r > 0.0).collect();
    radii.sort_by(f64::total_cmp);
    radii.dedup();
    let names: Vec<String> = atlas.search_box().iter().map(|r| r.name.clone()).collect();
    let meta = read_cell_meta(&args.atlas);
    let summary = summarise(&rows, &radii, &names, &meta);
    eprintln!(
        "fragility_audit: summarising {} rows from {}",
        rows.len(),
        args.out.display()
    );
    print_summary(&summary, names.len(), atlas.fingerprint());
    if let Some(path) = &args.json {
        std::fs::write(
            path,
            serde_json::to_string_pretty(&summary).expect("serialise summary"),
        )
        .unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use explorers_genesis::BloomStop;
    use explorers_search::config_source::NetworkPins;
    use explorers_search::qd::{DEFAULT_BLOOM_STOP, SEARCH_ROLLOUT_BUDGET};
    use explorers_search::search::SearchConfig;

    /// #693: by default the audit rolls out as the search does — its
    /// bloom stop, horizon and per-rollout budget — and the search's bloom
    /// stop flags move or turn off the stop.
    #[test]
    fn the_default_audit_carries_the_searchs_bloom_stop_and_budget() {
        let search = SearchConfig::default();
        let a = parse_args(std::iter::empty());
        assert_eq!(a.rollouts.eval_config.bloom_stop, Some(DEFAULT_BLOOM_STOP));
        assert_eq!(a.rollouts.eval_config.bloom_stop, search.bloom_stop);
        assert_eq!(a.rollouts.horizon, search.max_ticks);
        assert_eq!(a.rollouts.budget, SEARCH_ROLLOUT_BUDGET);
        let a = parse_args([NO_BLOOM_STOP_FLAG.to_string()]);
        assert_eq!(a.rollouts.eval_config.bloom_stop, None);
        let a = parse_args([BLOOM_STOP_FLAG.to_string(), "500:5".to_string()]);
        assert_eq!(
            a.rollouts.eval_config.bloom_stop,
            Some(BloomStop {
                tick: 500,
                factor: 5.0
            })
        );
    }

    #[test]
    fn args_default_to_the_full_audit_and_accept_each_flag() {
        let a = parse_args(std::iter::empty());
        assert_eq!(a.out, PathBuf::from(DEFAULT_OUT));
        assert_eq!(a.rollouts.horizon, 2000);
        assert_eq!(a.ensemble, 10);
        assert_eq!(a.draws, 8);
        assert_eq!(a.radii, vec![0.01, 0.03, 0.1]);
        assert_eq!(a.configs, None);
        assert_eq!(a.limit, None);
        assert!(!a.summary_only);
        let a = parse_args(
            "--atlas a.json --out o.jsonl --json s.json --configs atlas:0,atlas:2 --limit 1 --horizon 300 --seed 5 --ensemble 3 --draws 2 --radii 0.1,0.01 --jitter-seed 9 --run-timeout-secs 7 --eval-timeout-secs 11 --summary"
                .split(' ')
                .map(String::from),
        );
        assert_eq!(a.atlas, PathBuf::from("a.json"));
        assert_eq!(a.json, Some(PathBuf::from("s.json")));
        assert_eq!(a.configs.map(|c| c.len()), Some(2));
        assert_eq!(a.limit, Some(1));
        assert_eq!(
            (a.rollouts.horizon, a.seed, a.ensemble, a.draws),
            (300, 5, 3, 2)
        );
        assert_eq!(a.radii, vec![0.01, 0.1]);
        assert_eq!(a.jitter_seed, 9);
        assert_eq!(a.rollouts.budget.simulation, Duration::from_secs(7));
        assert_eq!(a.rollouts.budget.evaluation, Duration::from_secs(11));
        assert!(a.summary_only);
    }

    /// #729: `--hyphal-uptake on|off` and `--contact-distance <d_c>` pin
    /// the switch and `d_c` on every audited world, as the census's flags
    /// do (#728); unset, the worlds keep their decoded values.
    #[test]
    fn the_hyphal_uptake_flags_pin_the_rollouts() {
        let a = parse_args(std::iter::empty());
        assert_eq!(
            (a.rollouts.hyphal_uptake, a.rollouts.contact_distance),
            (None, None)
        );
        let a = parse_args(
            "--hyphal-uptake on --contact-distance 0.05"
                .split(' ')
                .map(String::from),
        );
        assert_eq!(
            (a.rollouts.hyphal_uptake, a.rollouts.contact_distance),
            (Some(true), Some(0.05))
        );
        let a = parse_args(["--hyphal-uptake".to_string(), "off".to_string()]);
        assert_eq!(a.rollouts.hyphal_uptake, Some(false));
    }

    #[test]
    #[should_panic(expected = "--hyphal-uptake takes on or off")]
    fn the_hyphal_uptake_flag_takes_on_or_off() {
        parse_args(["--hyphal-uptake".to_string(), "yes".to_string()]);
    }

    #[test]
    #[should_panic(expected = "--contact-distance \"0\" must be a positive number")]
    fn the_contact_distance_must_be_positive() {
        parse_args(["--contact-distance".to_string(), "0".to_string()]);
    }

    /// #741: the `--network-*` flags pin the network on every audited
    /// world, named and parsed as the census's (#646); unset, the worlds keep
    /// their decoded (off) network.
    #[test]
    fn the_network_flags_pin_the_rollouts() {
        let a = parse_args(std::iter::empty());
        assert_eq!(a.rollouts.network, NetworkPins::default());
        let a = parse_args(
            "--network-connection-cap 4 --network-creation-cost 0.1 --network-maintenance-cost 0.01 --network-redistribution-rate 0.2 --network-transfer-efficiency 0.8"
                .split(' ')
                .map(String::from),
        );
        assert_eq!(
            a.rollouts.network,
            NetworkPins {
                connection_cap: Some(4),
                creation_cost: Some(0.1),
                maintenance_cost: Some(0.01),
                redistribution_rate: Some(0.2),
                transfer_efficiency: Some(0.8),
            }
        );
    }

    #[test]
    #[should_panic(expected = "--network-transfer-efficiency \"1.5\" must be a number in [0, 1]")]
    fn the_network_flags_reject_what_the_census_rejects() {
        parse_args(
            ["--network-transfer-efficiency", "1.5"]
                .into_iter()
                .map(String::from),
        );
    }
}
