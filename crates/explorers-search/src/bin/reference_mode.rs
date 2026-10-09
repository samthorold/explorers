//! Reference-mode instrument (#751; `docs/system-design/reference-modes.md`).
//!
//! Mode 1, colonisation overshoot: builds the designed mesocosm
//! ([`explorers_search::mesocosm::mode1_mesocosm`]) from the committed recipe
//! with somatic wear on, runs it, and reports the centre cell, the patch the
//! perturbation will clear, per sampled tick, plus the distribution of
//! producer lifespans over the run, against which the wear rate is
//! calibrated.
//!
//! ```text
//! reference_mode [--recipe PATH] [--ticks N] [--seed S] [--wear-rate W]
//!                [--sample-every K] [--out PATH]
//! ```
//!
//! Defaults: `recipe.json`, 3000 ticks, seed 1, wear rate
//! [`PROVISIONAL_WEAR_RATE`], a sample every 25 ticks, artifact
//! `target/reference-mode/mode1.json` with the human-readable summary beside
//! it as `.md` (also printed). Deterministic per seed and arguments.

use std::path::PathBuf;

use explorers_search::mesocosm::{PROVISIONAL_WEAR_RATE, mode1_mesocosm, mode1_summary, run_mode1};
use explorers_sim::WorldRecipe;

const USAGE: &str = "usage: reference_mode [--recipe PATH] [--ticks N] [--seed S] [--wear-rate W] [--sample-every K] [--out PATH]";

struct Cli {
    recipe: PathBuf,
    ticks: u64,
    seed: u64,
    wear_rate: f32,
    sample_every: u64,
    out: PathBuf,
}

impl Cli {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut cli = Cli {
            recipe: PathBuf::from("recipe.json"),
            ticks: 3000,
            seed: 1,
            wear_rate: PROVISIONAL_WEAR_RATE,
            sample_every: 25,
            out: PathBuf::from("target/reference-mode/mode1.json"),
        };
        while let Some(flag) = args.next() {
            let value = args.next().ok_or(format!("{flag} needs a value"))?;
            let bad = |e: &dyn std::fmt::Display| format!("{flag}: cannot parse {value:?}: {e}");
            match flag.as_str() {
                "--recipe" => cli.recipe = PathBuf::from(&value),
                "--ticks" => cli.ticks = value.parse().map_err(|e| bad(&e))?,
                "--seed" => cli.seed = value.parse().map_err(|e| bad(&e))?,
                "--wear-rate" => cli.wear_rate = value.parse().map_err(|e| bad(&e))?,
                "--sample-every" => cli.sample_every = value.parse().map_err(|e| bad(&e))?,
                "--out" => cli.out = PathBuf::from(&value),
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
    let contents = std::fs::read_to_string(&cli.recipe)
        .unwrap_or_else(|e| panic!("read {}: {e}", cli.recipe.display()));
    let recipe: WorldRecipe = serde_json::from_str(&contents)
        .unwrap_or_else(|e| panic!("parse {}: {e}", cli.recipe.display()));

    let mut world = mode1_mesocosm(&recipe, cli.wear_rate, cli.seed);
    let report = run_mode1(&mut world, cli.ticks, cli.sample_every);

    let header = format!(
        "recipe {}, seed {}, {} ticks, wear rate {}, sample every {}",
        cli.recipe.display(),
        cli.seed,
        cli.ticks,
        cli.wear_rate,
        cli.sample_every
    );
    let summary = mode1_summary(&header, &report);
    print!("{summary}");

    if let Some(dir) = cli.out.parent() {
        std::fs::create_dir_all(dir).expect("create the artifact directory");
    }
    let artifact = serde_json::json!({
        "recipe": cli.recipe,
        "seed": cli.seed,
        "ticks": cli.ticks,
        "wear_rate": cli.wear_rate,
        "sample_every": cli.sample_every,
        "report": report,
    });
    std::fs::write(&cli.out, serde_json::to_string(&artifact).unwrap())
        .expect("write the artifact");
    std::fs::write(cli.out.with_extension("md"), &summary).expect("write the summary");
    eprintln!("wrote {} and its .md summary", cli.out.display());
}
