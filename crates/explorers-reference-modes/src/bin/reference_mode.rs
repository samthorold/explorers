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
//! at 0.1, over the whole horizon and over the arm. And it reports the
//! founder readout (#781): the decomposer founders, tracked by id, alive at
//! ticks 20, 50 and 100 and at the end, and their mean income over ticks
//! 5–20 as energy gained (drained structure × the trophic efficiency
//! applied).
//!
//! Each run or arm also reports mode 1's reading (#794;
//! `reference-modes.md`, mode 1, *Feeding reach → The reading*) over its
//! second half: whether the heterotroph-dominant decomposer guild holds (the
//! committed *heterotroph guild* predicate over agents with the decomposer
//! role whose heterotrophy exceeds their autotrophy), its fewest members, its
//! share of the carcass drain, its drain split between living targets and
//! carcasses, whether the centre patch's producers persist, and the pass:
//! the guild holds and the producers persist.
//!
//! `--litter-x N` (#781) stands N times the settled litter at founding: N
//! times the carcasses per cell, so N times their energy and nutrient. It is
//! a diagnostic instrument setting; the committed mesocosm is 1×.
//!
//! `--decomposer-founders N` (#829; `reference-modes.md`, mode 1,
//! *Decomposer founders*, *Founder density*) seeds N decomposer founders, at
//! least 1, at uniform random positions like every founder, in place of one
//! a cell (81). The producers and the founding litter stand where they do
//! without it. It needs decomposer founders, so it refuses
//! `--no-decomposers`. The summary's header and the artifact record N;
//! without the flag nothing is recorded and the world is unchanged.
//!
//! `--body-reach-pieces k` (#795; `reference-modes.md`, mode 1, *Feeding
//! reach*) sets the body-reach coefficient so that a decomposer founder's
//! feeding reach at founding spans k litter pieces, from the founders' effective
//! heterotrophy and co-limited structure and the litter the run builds (so it
//! follows `--litter-x`). Where the contact-only reach already spans k, the
//! coefficient clamps to 0, the unit-restricted case. The summary and
//! artifact record k, the coefficient, the founders' reach and the clamp.
//! Without the flag, or at 0, the coefficient is the spec's and nothing is
//! recorded.
//!
//! Saturating decomposition runs in every world (#864; world rules, *Carcass
//! decomposition saturates in its decomposers*), at the world's own decay
//! constant, a litter half-life of one producer lifespan.
//! `--litter-half-life L` (#805) sets the decay constant in its place, to
//! `k_d = ln 2 / (L × T_p)` at which a carcass under full saturation halves
//! in L producer lifespans, `T_p` the median established producer lifespan
//! at the committed wear (229 ticks).
//! `--enzyme-half-saturation K` sets the half-saturation constant to K
//! decomposer founder drain capabilities (effective heterotrophy × `u_H` at
//! founding, read from the world the run builds); without it, the world's
//! default. K needs decomposer founders; without L it runs at the world's
//! own `k_d`, recorded as L 1. The summary and artifact record L, `T_p`,
//! `k_d`, the founders' capability and K in founder capabilities and absolute
//! units. Without the flags nothing is recorded and the world keeps its own
//! kinetics.
//!
//! The base metabolic cost is `b · S` in every world (#862; world rules,
//! flow 8, *The base rate scales with structure*), with `b =
//! base_metabolic_rate / S_p + structure_maintenance_coefficient` at the
//! world's base-rate reference structure `S_p`. `--established-producer-structure
//! S_p` (#814) sets `S_p` in place of the world's default 6.937, for a reading
//! that settles it under the rules it runs with. The summary and artifact
//! (`base_rate_reference_structure`) then record `b`, `S_p`, and the
//! decomposer founder's and the median producer founder's upkeep at founding.
//! Without it nothing is recorded and the world keeps its default.
//! `--size-scaled-base-rate`, the switch the rule had before #862, is refused
//! as unknown.
//!
//! Living tissue resists every drain in every world (#813, #865; world rules,
//! *Consumption is incremental → Living tissue resists drains*), at the
//! world's own `ρ = 1`: no heterotroph drains a living target, and carcass
//! drains are unchanged. `--living-resistance`, the switch it had before
//! #865, is refused as unknown.
//!
//! `--decomposer-mesocosm` (#884; `end-state.md`, *The decomposer guild*,
//! *Its setup*) runs the decomposer mesocosm, hypothesis G's setup: the
//! rules above (`--litter-half-life`, `--enzyme-half-saturation`) at the
//! values
//! [`explorers_reference_modes::mesocosm::DecomposerMesocosm::DECLARED`]
//! declares, with the size-scaled base rate at the world's `S_p`, so G's
//! rungs read the mesocosm the crate declares rather than restating it. It
//! refuses those flags and `--established-producer-structure`, which would
//! set a rule twice or move its `S_p`, and `--no-decomposers`. The summary
//! carries the rules' sections, and the artifact records the declaration
//! (`decomposer_mesocosm`) beside them.
//! Without it nothing is recorded and the world is unchanged.
//!
//! The starvation cascade runs in every world (#821, #863; world rules, flow
//! 8, *When reserve cannot pay: the starvation cascade*): upkeep reserve
//! cannot pay is drawn from the energy earmark, then paid by burning
//! structure. `--starvation-readout` writes the founder starvation readout
//! per run or arm: for each decomposer founder, the tick it died, whether it
//! reproduced before its unallocated reserve first ran out, the earmark it
//! held then, and the ticks it lived on the earmark and on structure,
//! summarised as medians and counts. Without it nothing is recorded and the
//! world is unchanged.
//!
//! Reproductive isolation follows ancestry in every world (#837, #861; world
//! rules, flow 4, *Reproductive isolation follows ancestry, through a neutral
//! marker*): mode 1's producers found cluster 0 and its decomposer founders
//! cluster 1, 10 s apart in the marker, and a pair mates only past a draw
//! against its compatibility. Each run or arm writes the marker readout: per
//! sample, the founder line's living members and the share of them below
//! autotrophy 0.1, and at the end the mean and lowest pairwise compatibility
//! within the line and within the producers. `--compatibility-scale S` sets
//! `s` in place of the default, and the summary and artifact record it.
//!
//! It reads by the ladder (#857; `end-state.md`, *The decomposer guild*,
//! *Its ladder*), so the expensive rungs fail fast:
//!
//! - `--early-window C` reads rung 2 over the run, whose length is
//!   `--ticks` (about 100 for the guild): the decomposer founders alive at
//!   tick C (50 for the guild) and the founders' line's births, and whether
//!   both conditions hold. One run with founders, not paired.
//! - `--abort-on-line-extinction` stops a run, or a paired settle or arm, in
//!   the step the founders' line has no living member, and records the tick
//!   (`aborted_at`): the run fails, since no later tick can rescue the guild.
//! - `--seeds N --passes-needed K` runs seeds from `--seed` on, one at a
//!   time, under "at least K of N", and stops as soon as the outcome is
//!   decided either way, writing the outcome (passed, failed, or with
//!   `--failures-undecided` undecided at the failure bound), the seeds run
//!   and each run's artifact and summary. A seed passes by the early window
//!   when asked for, else by the guild: the run's, or a paired run's control
//!   arm's (`passes`: the guild holds and the producers persist). The
//!   outcome is the one running all N would give.
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
//!                [--litter-x N] [--decomposer-founders N] [--body-reach-pieces K]
//!                [--litter-half-life L] [--enzyme-half-saturation K]
//!                [--established-producer-structure S_p]
//!                [--decomposer-mesocosm]
//!                [--starvation-readout]
//!                [--compatibility-scale S]
//!                [--early-window C]
//!                [--abort-on-line-extinction]
//!                [--seeds N --passes-needed K [--failures-undecided]]
//!                [--out PATH]
//! ```
//!
//! Defaults: the committed `crates/explorers-reference-modes/mode1.json`
//! (found from any working directory), 3000 ticks, seed 1, the spec's wear
//! (the four wear flags override its parameters), a sample every 25 ticks, no clearance, with
//! decomposers, one decomposer founder a cell, 1× litter, artifact `target/reference-mode/mode1.json` (or
//! `mode1-paired.json` with `--clear-at`, `mode1-sequence.json` with
//! `--seeds`) with the human-readable summary
//! beside it as `.md` (also printed). Deterministic per seed and arguments.

use std::path::PathBuf;

use explorers_reference_modes::ladder::{
    OnFailure, SeedRule, read_in_sequence, sequential_reading_section,
};
use explorers_reference_modes::mesocosm::{
    Community, CompatibilityScale, DecomposerMesocosm, ESTABLISHED_PRODUCER_LIFESPAN_TICKS,
    EarlyWindow, Mode1Spec, RunOptions, Wear, base_rate_reference_structure_section,
    body_reach_section, carcass_decay_constant_for_half_life, compatibility_scale_section,
    decomposition_kinetics_section, early_window_section, founder_starvation_section,
    line_extinction_section, marker_readout_section, mode1_mesocosm_with_founders, mode1_summary,
    paired_summary, run_mode1_with, run_paired_with, set_base_rate_reference_structure,
    set_body_reach_pieces, set_compatibility_scale, set_decomposition_kinetics,
};

const USAGE: &str = "usage: reference_mode [--spec PATH] [--ticks N] [--seed S] [--wear-rate W] [--use-wear-rate U] [--repair-rate R] [--senescence-hazard H] [--sample-every K] [--clear-at T] [--no-decomposers] [--litter-x N] [--decomposer-founders N] [--body-reach-pieces K] [--litter-half-life L] [--enzyme-half-saturation K] [--established-producer-structure S_p] [--decomposer-mesocosm] [--starvation-readout] [--compatibility-scale S] [--early-window C] [--abort-on-line-extinction] [--seeds N --passes-needed K [--failures-undecided]] [--out PATH]";

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
    /// The diagnostic litter multiplier (#781): N times the settled litter
    /// stock at founding. 1 is the committed mesocosm.
    litter_x: usize,
    /// The decomposer founders' count (#829); `None` is one a cell.
    decomposer_founders: Option<usize>,
    /// The litter pieces a decomposer founder's reach spans at founding
    /// (#795); `None`, as at 0, keeps the spec's coefficient.
    body_reach_pieces: Option<f32>,
    /// The litter half-life under full saturation, in producer lifespans
    /// (#805); `None` keeps carcass drains additive.
    litter_half_life: Option<f32>,
    /// The half-saturation constant in decomposer founder drain capabilities
    /// (#805); `None` keeps the world's default.
    enzyme_half_saturation: Option<f32>,
    /// `S_p` for the size-scaled base rate (#814, #862); `None` keeps the
    /// world's default.
    established_producer_structure: Option<f32>,
    /// The decomposer mesocosm (#884), whose declared values set the rules
    /// above.
    decomposer_mesocosm: Option<DecomposerMesocosm>,
    /// Whether to write the founder starvation readout (#821).
    starvation_readout: bool,
    /// A compatibility scale `s` in place of the default (#837, #861).
    compatibility_scale: Option<f32>,
    /// The early window's founder tick (#857): read G's rung 2 over the run.
    early_window: Option<u64>,
    /// Whether a run stops, as a failure, once the founders' line is
    /// extinct (#857).
    abort_on_line_extinction: bool,
    /// Seeds in sequence under "at least k of n" (#857).
    sequence: Option<SeedRule>,
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
            litter_x: 1,
            decomposer_founders: None,
            body_reach_pieces: None,
            litter_half_life: None,
            enzyme_half_saturation: None,
            established_producer_structure: None,
            decomposer_mesocosm: None,
            starvation_readout: false,
            compatibility_scale: None,
            early_window: None,
            abort_on_line_extinction: false,
            sequence: None,
            out: None,
        };
        let (mut seeds, mut passes_needed, mut failures_undecided) = (None, None, false);
        while let Some(flag) = args.next() {
            if flag == "--no-decomposers" {
                cli.community = Community::ProducersOnly;
                continue;
            }
            if flag == "--decomposer-mesocosm" {
                cli.decomposer_mesocosm = Some(DecomposerMesocosm::DECLARED);
                continue;
            }
            if flag == "--starvation-readout" {
                cli.starvation_readout = true;
                continue;
            }
            if flag == "--abort-on-line-extinction" {
                cli.abort_on_line_extinction = true;
                continue;
            }
            if flag == "--failures-undecided" {
                failures_undecided = true;
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
                "--litter-x" => cli.litter_x = value.parse().map_err(|e| bad(&e))?,
                "--decomposer-founders" => {
                    let n: usize = value.parse().map_err(|e| bad(&e))?;
                    if n == 0 {
                        return Err("--decomposer-founders must be at least 1".into());
                    }
                    cli.decomposer_founders = Some(n);
                }
                "--body-reach-pieces" => {
                    let k: f32 = value.parse().map_err(|e| bad(&e))?;
                    if !(k >= 0.0 && k.is_finite()) {
                        return Err("--body-reach-pieces must be a finite k >= 0".into());
                    }
                    cli.body_reach_pieces = (k > 0.0).then_some(k);
                }
                "--litter-half-life" => {
                    let l: f32 = value.parse().map_err(|e| bad(&e))?;
                    if !(l > 0.0 && l.is_finite()) {
                        return Err("--litter-half-life must be a finite L > 0".into());
                    }
                    cli.litter_half_life = Some(l);
                }
                "--enzyme-half-saturation" => {
                    let k: f32 = value.parse().map_err(|e| bad(&e))?;
                    if !(k > 0.0 && k.is_finite()) {
                        return Err("--enzyme-half-saturation must be a finite K > 0".into());
                    }
                    cli.enzyme_half_saturation = Some(k);
                }
                "--established-producer-structure" => {
                    let s_p: f32 = value.parse().map_err(|e| bad(&e))?;
                    if !(s_p > 0.0 && s_p.is_finite()) {
                        return Err(
                            "--established-producer-structure must be a finite S_p > 0".into()
                        );
                    }
                    cli.established_producer_structure = Some(s_p);
                }
                "--compatibility-scale" => {
                    let s: f32 = value.parse().map_err(|e| bad(&e))?;
                    if !(s > 0.0 && s.is_finite()) {
                        return Err("--compatibility-scale must be a finite s > 0".into());
                    }
                    cli.compatibility_scale = Some(s);
                }
                "--early-window" => cli.early_window = Some(value.parse().map_err(|e| bad(&e))?),
                "--seeds" => seeds = Some(value.parse::<usize>().map_err(|e| bad(&e))?),
                "--passes-needed" => {
                    passes_needed = Some(value.parse::<usize>().map_err(|e| bad(&e))?)
                }
                "--clear-at" => cli.clear_at = Some(value.parse().map_err(|e| bad(&e))?),
                "--out" => cli.out = Some(PathBuf::from(&value)),
                _ => return Err(format!("unknown flag {flag}")),
            }
        }
        if cli.litter_x == 0 {
            return Err("--litter-x must be at least 1".into());
        }
        if let Some(d) = cli.decomposer_mesocosm {
            if cli.established_producer_structure.is_some()
                || cli.litter_half_life.is_some()
                || cli.enzyme_half_saturation.is_some()
            {
                return Err(
                    "--decomposer-mesocosm sets its rules at their declared values: drop --established-producer-structure, --litter-half-life and --enzyme-half-saturation"
                        .into(),
                );
            }
            if cli.community == Community::ProducersOnly {
                return Err("--decomposer-mesocosm needs decomposer founders".into());
            }
            cli.litter_half_life = Some(d.litter_half_life);
            cli.enzyme_half_saturation = Some(d.enzyme_half_saturation);
        }
        if cli.decomposer_founders.is_some() && cli.community == Community::ProducersOnly {
            return Err("--decomposer-founders needs decomposer founders".into());
        }
        if cli.body_reach_pieces.is_some() && cli.community == Community::ProducersOnly {
            return Err("--body-reach-pieces needs decomposer founders".into());
        }
        if cli.enzyme_half_saturation.is_some() {
            if cli.community == Community::ProducersOnly {
                return Err("--enzyme-half-saturation needs decomposer founders".into());
            }
            // Saturating decomposition is in every world (#864): K alone
            // runs at the world's own k_d, a litter half-life of one
            // producer lifespan, recorded as such.
            cli.litter_half_life = cli
                .litter_half_life
                .or(Some(explorers_sim::REFERENCE_LITTER_HALF_LIFE));
        }
        if let Some(l) = cli.litter_half_life {
            let k_d = carcass_decay_constant_for_half_life(l, ESTABLISHED_PRODUCER_LIFESPAN_TICKS);
            if k_d > 1.0 {
                return Err(format!(
                    "--litter-half-life {l} gives k_d {k_d} per tick, above 1"
                ));
            }
        }
        if let Some(c) = cli.early_window {
            if c > cli.ticks {
                return Err(format!(
                    "--early-window {c}: the founders are counted within the run's {} ticks",
                    cli.ticks
                ));
            }
            if cli.clear_at.is_some() {
                return Err(
                    "--early-window reads one run, not a paired one: drop --clear-at".into(),
                );
            }
            if cli.community == Community::ProducersOnly {
                return Err("--early-window needs decomposer founders".into());
            }
        }
        cli.sequence = match (seeds, passes_needed) {
            (Some(n), Some(k)) if (1..=n).contains(&k) => Some(SeedRule::at_least(
                k,
                n,
                if failures_undecided {
                    OnFailure::Undecided
                } else {
                    OnFailure::Fails
                },
            )),
            (Some(n), Some(k)) => {
                return Err(format!(
                    "--passes-needed {k} of --seeds {n}: needs between 1 and {n}"
                ));
            }
            (None, None) if !failures_undecided => None,
            _ => {
                return Err(
                    "--seeds and --passes-needed go together, and --failures-undecided needs them"
                        .into(),
                );
            }
        };
        if cli.abort_on_line_extinction && cli.community == Community::ProducersOnly {
            return Err(
                "--abort-on-line-extinction needs decomposer founders: without them there is no line"
                    .into(),
            );
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
    let mut spec: Mode1Spec = serde_json::from_str(&contents)
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

    // The compatibility scale acts at world creation, so it is set on the
    // spec.
    let compatibility_scale = cli
        .compatibility_scale
        .map(|s| set_compatibility_scale(&mut spec, s));
    let run = |seed| run_seed(&cli, &spec, wear, compatibility_scale, seed);

    let (summary, artifact, default_out) = match cli.sequence {
        None => {
            let r = run(cli.seed);
            (r.summary, r.artifact, r.default_out)
        }
        // Seeds in sequence (#857): run from `--seed` on until the rule is
        // decided, keeping each run's summary and artifact.
        Some(rule) => {
            let mut runs = Vec::new();
            let reading = read_in_sequence(rule, cli.seed.., |seed| {
                let r = run(seed);
                let passed = r.passed;
                runs.push(r);
                passed
            });
            let mut summary: String = (runs.iter())
                .map(|r| format!("# Seed {}\n\n{}\n", r.seed, r.summary))
                .collect();
            summary.push_str(&sequential_reading_section(&reading));
            let artifact = serde_json::json!({
                "sequence": reading,
                "runs": runs.into_iter().map(|r| r.artifact).collect::<Vec<_>>(),
            });
            (summary, artifact, "mode1-sequence.json")
        }
    };
    print!("{summary}");
    let out = cli
        .out
        .clone()
        .unwrap_or_else(|| PathBuf::from("target/reference-mode").join(default_out));
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).expect("create the artifact directory");
    }
    std::fs::write(&out, serde_json::to_string(&artifact).unwrap()).expect("write the artifact");
    std::fs::write(out.with_extension("md"), &summary).expect("write the summary");
    eprintln!("wrote {} and its .md summary", out.display());
}

/// One seed's run: its summary, its artifact, whether it passed (the early
/// window when asked for, else the guild: the run's, or a paired run's
/// control arm's) and the artifact's default name.
struct SeedRun {
    seed: u64,
    summary: String,
    artifact: serde_json::Value,
    passed: bool,
    default_out: &'static str,
}

fn run_seed(
    cli: &Cli,
    spec: &Mode1Spec,
    wear: Wear,
    compatibility_scale: Option<CompatibilityScale>,
    seed: u64,
) -> SeedRun {
    let mut world = mode1_mesocosm_with_founders(
        spec,
        wear,
        seed,
        cli.community,
        cli.litter_x,
        cli.decomposer_founders,
    );
    let body_reach = cli.body_reach_pieces.map(|k| {
        set_body_reach_pieces(&mut world, k).expect("the mesocosm has decomposer founders")
    });
    let kinetics = cli.litter_half_life.map(|l| {
        set_decomposition_kinetics(&mut world, l, cli.enzyme_half_saturation)
            .expect("K is given only with decomposer founders")
    });
    let base_rate = cli
        .established_producer_structure
        .map(|s_p| set_base_rate_reference_structure(&mut world, s_p));
    let starvation_readout = cli.starvation_readout;
    let community = match cli.community {
        Community::ProducersAndDecomposers => "producers and decomposers",
        Community::ProducersOnly => "producers only",
    };
    let header = format!(
        "spec {}, seed {}, {} ticks, wear rate {}, use-wear rate {}, repair rate {}, senescence hazard {}, sample every {}, {community}, litter x{}",
        cli.spec.display(),
        seed,
        cli.ticks,
        wear.wear_rate,
        wear.use_wear_rate,
        wear.repair_rate,
        wear.senescence_hazard,
        cli.sample_every,
        cli.litter_x
    );
    // A founder count is recorded only when given, so a default run's
    // summary is unchanged.
    let header = match cli.decomposer_founders {
        Some(n) => format!("{header}, {n} decomposer founders"),
        None => header,
    };
    // The feeding reach, kinetics and base rate set are run
    // settings, so they follow the header.
    let with_reach = |header: String| {
        let header = match &body_reach {
            Some(r) => format!("{header}\n\n{}", body_reach_section(r)),
            None => header,
        };
        let header = match &kinetics {
            Some(k) => format!("{header}\n\n{}", decomposition_kinetics_section(k)),
            None => header,
        };
        let header = match &base_rate {
            Some(r) => format!("{header}\n\n{}", base_rate_reference_structure_section(r)),
            None => header,
        };
        match &compatibility_scale {
            Some(m) => format!("{header}\n\n{}", compatibility_scale_section(m)),
            None => header,
        }
    };
    // The early window (#857), when asked for.
    let mut early_window = None;
    // The founder starvation readout (#821), when asked for: its summary
    // section and its artifact entry.
    let options = RunOptions {
        abort_on_line_extinction: cli.abort_on_line_extinction,
    };
    let (summary, report, starvation, passed, default_out) = match cli.clear_at {
        None => {
            let report = run_mode1_with(&mut world, cli.ticks, cli.sample_every, options);
            let mut summary = mode1_summary(&with_reach(header), &report);
            if let Some(s) = line_extinction_section(&[("run", &report)]) {
                summary.push_str(&format!("\n{s}"));
            }
            let readout = &report.founders.starvation;
            let starvation = starvation_readout.then(|| {
                summary.push_str(&format!(
                    "\n{}",
                    founder_starvation_section(&[("run", readout)])
                ));
                serde_json::to_value(readout).expect("the starvation readout serialises")
            });
            if let Some(m) = &report.marker {
                summary.push_str(&format!("\n{}", marker_readout_section(&[("run", m)])));
            }
            early_window = cli.early_window.map(|c| {
                let w = EarlyWindow::read(&report, c);
                summary.push_str(&format!("\n{}\n", early_window_section(&w)));
                w
            });
            let passed = early_window.map_or(report.guild.passes, |w| w.holds);
            (
                summary,
                serde_json::to_value(report),
                starvation,
                passed,
                "mode1.json",
            )
        }
        Some(settle) => {
            let report = run_paired_with(world, settle, cli.ticks, cli.sample_every, options);
            let header = with_reach(format!("{header}, settled to tick {settle} then cleared"));
            let mut summary = paired_summary(&header, &report);
            let arms = [
                ("perturbed", &report.perturbed),
                ("control", &report.control),
            ];
            if let Some(s) = line_extinction_section(&arms) {
                summary.push_str(&format!("\n{s}"));
            }
            let (perturbed, control) = (
                &report.perturbed.founders.starvation,
                &report.control.founders.starvation,
            );
            let starvation = starvation_readout.then(|| {
                summary.push_str(&format!(
                    "\n{}",
                    founder_starvation_section(&[("perturbed", perturbed), ("control", control)])
                ));
                serde_json::json!({ "perturbed": perturbed, "control": control })
            });
            if let (Some(p), Some(c)) = (&report.perturbed.marker, &report.control.marker) {
                summary.push_str(&format!(
                    "\n{}",
                    marker_readout_section(&[("perturbed", p), ("control", c)])
                ));
            }
            // The full reading's seed passes on the paired control's guild,
            // which holds only with its producers persisting.
            let passed = report.control.guild.passes;
            (
                summary,
                serde_json::to_value(report),
                starvation,
                passed,
                "mode1-paired.json",
            )
        }
    };
    let mut artifact = serde_json::json!({
        "spec": cli.spec,
        "seed": seed,
        "ticks": cli.ticks,
        "wear": wear,
        "sample_every": cli.sample_every,
        "clear_at": cli.clear_at,
        "community": cli.community,
        "litter_x": cli.litter_x,
        "report": report.expect("the report serialises"),
    });
    if let Some(n) = cli.decomposer_founders {
        artifact["decomposer_founders"] = serde_json::json!(n);
    }
    if let Some(r) = body_reach {
        artifact["body_reach"] = serde_json::to_value(r).expect("the body reach serialises");
    }
    if let Some(k) = kinetics {
        artifact["decomposition_kinetics"] =
            serde_json::to_value(k).expect("the decomposition kinetics serialise");
    }
    if let Some(r) = base_rate {
        artifact["base_rate_reference_structure"] =
            serde_json::to_value(r).expect("the base rate serialises");
    }
    if let Some(d) = cli.decomposer_mesocosm {
        artifact["decomposer_mesocosm"] =
            serde_json::to_value(d).expect("the decomposer mesocosm serialises");
    }
    if let Some(m) = compatibility_scale {
        artifact["compatibility_scale"] = serde_json::json!(m.compatibility_scale);
    }
    if let Some(readout) = starvation {
        artifact["founder_starvation"] = readout;
    }
    if let Some(w) = early_window {
        artifact["early_window"] = serde_json::to_value(w).expect("the early window serialises");
    }
    if cli.abort_on_line_extinction {
        artifact["abort_on_line_extinction"] = serde_json::json!(true);
    }
    SeedRun {
        seed,
        summary,
        artifact,
        passed,
        default_out,
    }
}
