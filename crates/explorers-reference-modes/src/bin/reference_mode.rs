//! Reference-mode instrument (#751; `docs/system-design/reference-modes.md`).
//!
//! Mode 1, colonisation overshoot: builds the designed mesocosm
//! ([`explorers_reference_modes::mesocosm::mode1_mesocosm`]) from its
//! committed spec (`mode1.json`, #782) with somatic wear on and standing carcasses at founding (#764), runs it,
//! and reports the centre patch, the 3 × 3 block of cells the perturbation
//! clears, per sampled tick beside the world's decomposers, plus the
//! distribution of producer lifespans over the run, read also over producers
//! past establishment with their senescent share, against which the wear is
//! calibrated (#766). Each run or arm also reports #764's decomposer reading
//! (#772): whether decomposers persist, the heterotroph-dominant agents alive
//! at the end, and the carcass-drain energy split by the drainer's autotrophy
//! at 0.1, over the whole horizon and over the arm.
//!
//! With `--clear-at T` (#752) it reads the perturbation against its paired
//! control: it settles the mesocosm to tick `T`, clones it, clears every agent
//! and carcass from one copy's centre patch (an outflow, booked in each arm's
//! energy and nutrient ledger), and runs both copies `--ticks` further,
//! reporting their centre-patch series side by side. `--no-decomposers` builds
//! the mesocosm from producer founders alone.
//!
//! ```text
//! reference_mode [--spec PATH] [--ticks N] [--seed S] [--wear-rate W]
//!                [--use-wear-rate U] [--repair-rate R] [--senescence-hazard H]
//!                [--sample-every K] [--clear-at T] [--no-decomposers]
//!                [--out PATH]
//! ```
//!
//! Defaults: the committed `crates/explorers-reference-modes/mode1.json`
//! (found from any working directory), 3000 ticks, seed 1, the spec's wear
//! (the four wear flags override its parameters), a sample every 25 ticks, no clearance, with
//! decomposers, artifact `target/reference-mode/mode1.json` (or
//! `mode1-paired.json` with `--clear-at`) with the human-readable summary
//! beside it as `.md` (also printed). Deterministic per seed and arguments.

use std::path::PathBuf;

use explorers_reference_modes::mesocosm::{
    Community, Mode1Spec, mode1_mesocosm, mode1_summary, paired_summary, run_mode1, run_paired,
};

const USAGE: &str = "usage: reference_mode [--spec PATH] [--ticks N] [--seed S] [--wear-rate W] [--use-wear-rate U] [--repair-rate R] [--senescence-hazard H] [--sample-every K] [--clear-at T] [--no-decomposers] [--out PATH]";

struct Cli {
    spec: PathBuf,
    ticks: u64,
    seed: u64,
    /// Overrides of the spec's wear_rate, use_wear_rate, repair_rate and
    /// senescence_hazard, in that order.
    wear: [Option<f32>; 4],
    sample_every: u64,
    clear_at: Option<u64>,
    community: Community,
    out: Option<PathBuf>,
}

impl Cli {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut cli = Cli {
            spec: PathBuf::from(Mode1Spec::COMMITTED_PATH),
            ticks: 3000,
            seed: 1,
            wear: [None; 4],
            sample_every: 25,
            clear_at: None,
            community: Community::ProducersAndDecomposers,
            out: None,
        };
        while let Some(flag) = args.next() {
            if flag == "--no-decomposers" {
                cli.community = Community::ProducersOnly;
                continue;
            }
            let value = args.next().ok_or(format!("{flag} needs a value"))?;
            let bad = |e: &dyn std::fmt::Display| format!("{flag}: cannot parse {value:?}: {e}");
            match flag.as_str() {
                "--spec" => cli.spec = PathBuf::from(&value),
                "--ticks" => cli.ticks = value.parse().map_err(|e| bad(&e))?,
                "--seed" => cli.seed = value.parse().map_err(|e| bad(&e))?,
                "--wear-rate" => cli.wear[0] = Some(value.parse().map_err(|e| bad(&e))?),
                "--use-wear-rate" => cli.wear[1] = Some(value.parse().map_err(|e| bad(&e))?),
                "--repair-rate" => cli.wear[2] = Some(value.parse().map_err(|e| bad(&e))?),
                "--senescence-hazard" => cli.wear[3] = Some(value.parse().map_err(|e| bad(&e))?),
                "--sample-every" => cli.sample_every = value.parse().map_err(|e| bad(&e))?,
                "--clear-at" => cli.clear_at = Some(value.parse().map_err(|e| bad(&e))?),
                "--out" => cli.out = Some(PathBuf::from(&value)),
                _ => return Err(format!("unknown flag {flag}")),
            }
        }
        if cli.sample_every == 0 {
            return Err("--sample-every must be at least 1".into());
        }
        Ok(cli)
    }
}

fn main() {
    let cli = Cli::parse(std::env::args().skip(1)).unwrap_or_else(|e| {
        eprintln!("reference_mode: {e}\n{USAGE}");
        std::process::exit(2);
    });
    let contents = std::fs::read_to_string(&cli.spec)
        .unwrap_or_else(|e| panic!("read {}: {e}", cli.spec.display()));
    let spec: Mode1Spec = serde_json::from_str(&contents)
        .unwrap_or_else(|e| panic!("parse {}: {e}", cli.spec.display()));
    let mut wear = spec.wear();
    for (field, value) in [
        &mut wear.wear_rate,
        &mut wear.use_wear_rate,
        &mut wear.repair_rate,
        &mut wear.senescence_hazard,
    ]
    .into_iter()
    .zip(cli.wear)
    {
        if let Some(value) = value {
            *field = value;
        }
    }

    let mut world = mode1_mesocosm(&spec, wear, cli.seed, cli.community);
    let community = match cli.community {
        Community::ProducersAndDecomposers => "producers and decomposers",
        Community::ProducersOnly => "producers only",
    };
    let header = format!(
        "spec {}, seed {}, {} ticks, wear rate {}, use-wear rate {}, repair rate {}, senescence hazard {}, sample every {}, {community}",
        cli.spec.display(),
        cli.seed,
        cli.ticks,
        wear.wear_rate,
        wear.use_wear_rate,
        wear.repair_rate,
        wear.senescence_hazard,
        cli.sample_every
    );
    let (summary, report, default_out) = match cli.clear_at {
        None => {
            let report = run_mode1(&mut world, cli.ticks, cli.sample_every);
            let summary = mode1_summary(&header, &report);
            (summary, serde_json::to_value(report), "mode1.json")
        }
        Some(settle) => {
            let report = run_paired(world, settle, cli.ticks, cli.sample_every);
            let header = format!("{header}, settled to tick {settle} then cleared");
            let summary = paired_summary(&header, &report);
            (summary, serde_json::to_value(report), "mode1-paired.json")
        }
    };
    print!("{summary}");

    let out = cli
        .out
        .unwrap_or_else(|| PathBuf::from("target/reference-mode").join(default_out));
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).expect("create the artifact directory");
    }
    let artifact = serde_json::json!({
        "spec": cli.spec,
        "seed": cli.seed,
        "ticks": cli.ticks,
        "wear": wear,
        "sample_every": cli.sample_every,
        "clear_at": cli.clear_at,
        "community": cli.community,
        "report": report.expect("the report serialises"),
    });
    std::fs::write(&out, serde_json::to_string(&artifact).unwrap()).expect("write the artifact");
    std::fs::write(out.with_extension("md"), &summary).expect("write the summary");
    eprintln!("wrote {} and its .md summary", out.display());
}
