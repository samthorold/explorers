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
//! `--litter-half-life L` (#805; world rules, *Carcass decomposition
//! saturates in its decomposers*) turns saturating decomposition on, with
//! the decay constant `k_d = ln 2 / (L × T_p)` at which a carcass under full
//! saturation halves in L producer lifespans, `T_p` the median established
//! producer lifespan at the committed wear (229 ticks).
//! `--enzyme-half-saturation K` sets the half-saturation constant to K
//! decomposer founder drain capabilities (effective heterotrophy × `u_H` at
//! founding, read from the world the run builds); without it, the world's
//! default. K needs L, since it has no effect while `k_d` is 0, and needs
//! decomposer founders. The summary and artifact record L, `T_p`, `k_d`, the
//! founders' capability and K in founder capabilities and absolute units.
//! Without the flags nothing is recorded and the world is unchanged.
//!
//! `--size-scaled-base-rate` (#807; world rules, flow 8, *The base rate
//! scales with structure*) makes the base metabolic cost `b · S`, with
//! `b = base_metabolic_rate / S_p + structure_maintenance_coefficient`, `S_p`
//! the established producers' median structure #804 read (21.6), so a
//! producer of that size pays what it did. The summary and artifact record
//! `b`, `S_p`, and the decomposer founder's and the median producer
//! founder's upkeep at founding under both rules. Without the flag nothing is
//! recorded and the world is unchanged. `--established-producer-structure
//! S_p` (#814) gives `S_p` explicitly, for a reading that settles it under
//! the rules it runs with; it needs `--size-scaled-base-rate`. Without it
//! `S_p` is 21.6.
//!
//! `--living-resistance` (#813; world rules, *Consumption is incremental →
//! Living tissue resists drains*) sets the living-tissue resistance `ρ` to 1,
//! the design's value for mode 1's consumers: no heterotroph drains a living
//! target, and carcass drains are unchanged. The summary and artifact record
//! `ρ`, and the guild readout's living-versus-carcass split is the check.
//! Without the flag `ρ` stays 0 and nothing is recorded.
//!
//! `--starvation-draws-earmark` and `--starvation-shrinks` (#821; world
//! rules, flow 8, *When reserve cannot pay: the starvation cascade*) turn on
//! the cascade's two steps: upkeep reserve cannot pay is drawn from the
//! energy earmark, then paid by burning structure. Each works alone. The
//! summary and artifact record both switches. Either option, or
//! `--starvation-readout` alone (for a control with both off), also writes
//! the founder starvation readout per run or arm: for each decomposer
//! founder, the tick it died, whether it reproduced before its unallocated
//! reserve first ran out, the earmark it held then, and the ticks it lived
//! on the earmark and on structure, summarised as medians and counts.
//! Without any of them nothing is recorded and the world is unchanged.
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
//!                [--litter-x N] [--body-reach-pieces K]
//!                [--litter-half-life L] [--enzyme-half-saturation K]
//!                [--size-scaled-base-rate] [--established-producer-structure S_p]
//!                [--living-resistance] [--starvation-draws-earmark]
//!                [--starvation-shrinks] [--starvation-readout] [--out PATH]
//! ```
//!
//! Defaults: the committed `crates/explorers-reference-modes/mode1.json`
//! (found from any working directory), 3000 ticks, seed 1, the spec's wear
//! (the four wear flags override its parameters), a sample every 25 ticks, no clearance, with
//! decomposers, 1× litter, artifact `target/reference-mode/mode1.json` (or
//! `mode1-paired.json` with `--clear-at`) with the human-readable summary
//! beside it as `.md` (also printed). Deterministic per seed and arguments.

use std::path::PathBuf;

use explorers_reference_modes::mesocosm::{
    Community, ESTABLISHED_PRODUCER_LIFESPAN_TICKS, Mode1Spec, body_reach_section,
    carcass_decay_constant_for_half_life, decomposition_kinetics_section,
    founder_starvation_section, living_resistance_section, mode1_mesocosm_with_litter,
    mode1_summary, paired_summary, run_mode1, run_paired, set_body_reach_pieces,
    set_decomposition_kinetics, set_living_resistance, set_size_scaled_base_rate,
    set_size_scaled_base_rate_at, set_starvation_cascade, size_scaled_base_rate_section,
    starvation_cascade_section,
};

const USAGE: &str = "usage: reference_mode [--spec PATH] [--ticks N] [--seed S] [--wear-rate W] [--use-wear-rate U] [--repair-rate R] [--senescence-hazard H] [--sample-every K] [--clear-at T] [--no-decomposers] [--litter-x N] [--body-reach-pieces K] [--litter-half-life L] [--enzyme-half-saturation K] [--size-scaled-base-rate] [--established-producer-structure S_p] [--living-resistance] [--starvation-draws-earmark] [--starvation-shrinks] [--starvation-readout] [--out PATH]";

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
    /// The litter pieces a decomposer founder's reach spans at founding
    /// (#795); `None`, as at 0, keeps the spec's coefficient.
    body_reach_pieces: Option<f32>,
    /// The litter half-life under full saturation, in producer lifespans
    /// (#805); `None` keeps carcass drains additive.
    litter_half_life: Option<f32>,
    /// The half-saturation constant in decomposer founder drain capabilities
    /// (#805); `None` keeps the world's default.
    enzyme_half_saturation: Option<f32>,
    /// Whether the base metabolic rate scales with structure (#807).
    size_scaled_base_rate: bool,
    /// `S_p` for the size-scaled base rate (#814); `None` keeps #804's 21.6.
    established_producer_structure: Option<f32>,
    /// Whether living tissue resists drains, at mode 1's `ρ` = 1 (#813).
    living_resistance: bool,
    /// The starvation cascade's steps (#821): the earmark pays, then
    /// structure.
    starvation_draws_earmark: bool,
    starvation_shrinks: bool,
    /// Whether to write the founder starvation readout with both steps off.
    starvation_readout: bool,
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
            body_reach_pieces: None,
            litter_half_life: None,
            enzyme_half_saturation: None,
            size_scaled_base_rate: false,
            established_producer_structure: None,
            living_resistance: false,
            starvation_draws_earmark: false,
            starvation_shrinks: false,
            starvation_readout: false,
            out: None,
        };
        while let Some(flag) = args.next() {
            if flag == "--no-decomposers" {
                cli.community = Community::ProducersOnly;
                continue;
            }
            if flag == "--size-scaled-base-rate" {
                cli.size_scaled_base_rate = true;
                continue;
            }
            if flag == "--living-resistance" {
                cli.living_resistance = true;
                continue;
            }
            if flag == "--starvation-draws-earmark" {
                cli.starvation_draws_earmark = true;
                continue;
            }
            if flag == "--starvation-shrinks" {
                cli.starvation_shrinks = true;
                continue;
            }
            if flag == "--starvation-readout" {
                cli.starvation_readout = true;
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
                "--clear-at" => cli.clear_at = Some(value.parse().map_err(|e| bad(&e))?),
                "--out" => cli.out = Some(PathBuf::from(&value)),
                _ => return Err(format!("unknown flag {flag}")),
            }
        }
        if cli.litter_x == 0 {
            return Err("--litter-x must be at least 1".into());
        }
        if cli.body_reach_pieces.is_some() && cli.community == Community::ProducersOnly {
            return Err("--body-reach-pieces needs decomposer founders".into());
        }
        if cli.enzyme_half_saturation.is_some() {
            if cli.litter_half_life.is_none() {
                return Err(
                    "--enzyme-half-saturation needs --litter-half-life: K has no effect while k_d is 0"
                        .into(),
                );
            }
            if cli.community == Community::ProducersOnly {
                return Err("--enzyme-half-saturation needs decomposer founders".into());
            }
        }
        if let Some(l) = cli.litter_half_life {
            let k_d = carcass_decay_constant_for_half_life(l, ESTABLISHED_PRODUCER_LIFESPAN_TICKS);
            if k_d > 1.0 {
                return Err(format!(
                    "--litter-half-life {l} gives k_d {k_d} per tick, above 1"
                ));
            }
        }
        if cli.established_producer_structure.is_some() && !cli.size_scaled_base_rate {
            return Err(
                "--established-producer-structure needs --size-scaled-base-rate: S_p only sets b"
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

    let mut world = mode1_mesocosm_with_litter(&spec, wear, cli.seed, cli.community, cli.litter_x);
    let body_reach = cli.body_reach_pieces.map(|k| {
        set_body_reach_pieces(&mut world, k).expect("the mesocosm has decomposer founders")
    });
    let kinetics = cli.litter_half_life.map(|l| {
        set_decomposition_kinetics(&mut world, l, cli.enzyme_half_saturation)
            .expect("K is given only with decomposer founders")
    });
    let size_scaled = cli
        .size_scaled_base_rate
        .then(|| match cli.established_producer_structure {
            Some(s_p) => set_size_scaled_base_rate_at(&mut world, s_p),
            None => set_size_scaled_base_rate(&mut world),
        });
    let living_resistance = cli
        .living_resistance
        .then(|| set_living_resistance(&mut world));
    let cascade = (cli.starvation_draws_earmark || cli.starvation_shrinks).then(|| {
        set_starvation_cascade(
            &mut world,
            cli.starvation_draws_earmark,
            cli.starvation_shrinks,
        )
    });
    let starvation_readout = cli.starvation_readout || cascade.is_some();
    let community = match cli.community {
        Community::ProducersAndDecomposers => "producers and decomposers",
        Community::ProducersOnly => "producers only",
    };
    let header = format!(
        "spec {}, seed {}, {} ticks, wear rate {}, use-wear rate {}, repair rate {}, senescence hazard {}, sample every {}, {community}, litter x{}",
        cli.spec.display(),
        cli.seed,
        cli.ticks,
        wear.wear_rate,
        wear.use_wear_rate,
        wear.repair_rate,
        wear.senescence_hazard,
        cli.sample_every,
        cli.litter_x
    );
    // The feeding reach, kinetics, base rate and resistance set are run
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
        let header = match &size_scaled {
            Some(r) => format!("{header}\n\n{}", size_scaled_base_rate_section(r)),
            None => header,
        };
        let header = match living_resistance {
            Some(rho) => format!("{header}\n\n{}", living_resistance_section(rho)),
            None => header,
        };
        match &cascade {
            Some(c) => format!("{header}\n\n{}", starvation_cascade_section(c)),
            None => header,
        }
    };
    // The founder starvation readout (#821), when asked for: its summary
    // section and its artifact entry.
    let (summary, report, starvation, default_out) = match cli.clear_at {
        None => {
            let report = run_mode1(&mut world, cli.ticks, cli.sample_every);
            let mut summary = mode1_summary(&with_reach(header), &report);
            let readout = &report.founders.starvation;
            let starvation = starvation_readout.then(|| {
                summary.push_str(&format!(
                    "\n{}",
                    founder_starvation_section(&[("run", readout)])
                ));
                serde_json::to_value(readout).expect("the starvation readout serialises")
            });
            (
                summary,
                serde_json::to_value(report),
                starvation,
                "mode1.json",
            )
        }
        Some(settle) => {
            let report = run_paired(world, settle, cli.ticks, cli.sample_every);
            let header = with_reach(format!("{header}, settled to tick {settle} then cleared"));
            let mut summary = paired_summary(&header, &report);
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
            (
                summary,
                serde_json::to_value(report),
                starvation,
                "mode1-paired.json",
            )
        }
    };
    print!("{summary}");

    let out = cli
        .out
        .unwrap_or_else(|| PathBuf::from("target/reference-mode").join(default_out));
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).expect("create the artifact directory");
    }
    let mut artifact = serde_json::json!({
        "spec": cli.spec,
        "seed": cli.seed,
        "ticks": cli.ticks,
        "wear": wear,
        "sample_every": cli.sample_every,
        "clear_at": cli.clear_at,
        "community": cli.community,
        "litter_x": cli.litter_x,
        "report": report.expect("the report serialises"),
    });
    if let Some(r) = body_reach {
        artifact["body_reach"] = serde_json::to_value(r).expect("the body reach serialises");
    }
    if let Some(k) = kinetics {
        artifact["decomposition_kinetics"] =
            serde_json::to_value(k).expect("the decomposition kinetics serialise");
    }
    if let Some(r) = size_scaled {
        artifact["size_scaled_base_rate"] =
            serde_json::to_value(r).expect("the size-scaled base rate serialises");
    }
    if let Some(rho) = living_resistance {
        artifact["living_resistance"] = serde_json::json!(rho);
    }
    if let Some(c) = cascade {
        artifact["starvation_cascade"] =
            serde_json::to_value(c).expect("the starvation cascade serialises");
    }
    if let Some(readout) = starvation {
        artifact["founder_starvation"] = readout;
    }
    std::fs::write(&out, serde_json::to_string(&artifact).unwrap()).expect("write the artifact");
    std::fs::write(out.with_extension("md"), &summary).expect("write the summary");
    eprintln!("wrote {} and its .md summary", out.display());
}
