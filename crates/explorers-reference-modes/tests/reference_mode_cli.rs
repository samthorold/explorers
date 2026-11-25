//! The reference-mode instrument (#751): one command builds the mode-1
//! mesocosm from its committed spec (#782), runs it, and writes a machine-readable
//! artifact and a human-readable summary, identically for the same seed and
//! arguments.

use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "explorers-reference-mode-cli-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(out: &Path, seed: &str) -> (String, String) {
    let spec = concat!(env!("CARGO_MANIFEST_DIR"), "/mode1.json");
    let output = Command::new(env!("CARGO_BIN_EXE_reference_mode"))
        .args(["--spec", spec, "--ticks", "40", "--sample-every", "10"])
        .args(["--seed", seed, "--out"])
        .arg(out)
        .output()
        .expect("the instrument runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (
        std::fs::read_to_string(out).unwrap(),
        std::fs::read_to_string(out.with_extension("md")).unwrap(),
    )
}

#[test]
fn the_same_seed_and_arguments_write_identical_artifacts() {
    let dir = scratch("determinism");
    let (json_a, md_a) = run(&dir.join("a.json"), "3");
    let (json_b, md_b) = run(&dir.join("b.json"), "3");
    let (json_c, _) = run(&dir.join("c.json"), "4");

    let artifact: serde_json::Value = serde_json::from_str(&json_a).unwrap();
    assert_eq!(artifact["report"]["samples"].as_array().unwrap().len(), 5);
    assert!(md_a.contains("Producer lifespans"));
    assert_eq!(json_a, json_b);
    assert_eq!(md_a, md_b);
    assert_ne!(json_a, json_c);
}

/// With no `--spec`, the instrument reads the committed spec, from any
/// working directory, and records which spec it read.
#[test]
fn the_committed_spec_is_the_default_from_any_working_directory() {
    let dir = scratch("default-spec");
    let out = dir.join("default.json");
    let output = Command::new(env!("CARGO_BIN_EXE_reference_mode"))
        .current_dir(&dir)
        .args(["--ticks", "10", "--sample-every", "10", "--out"])
        .arg(&out)
        .output()
        .expect("the instrument runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let artifact: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert!(artifact.get("recipe").is_none());
    let spec = artifact["spec"].as_str().unwrap();
    assert!(spec.ends_with("mode1.json"), "{spec}");
}

/// The usage names `--spec`, which replaces `--recipe`.
#[test]
fn the_usage_names_spec_and_refuses_recipe() {
    let output = Command::new(env!("CARGO_BIN_EXE_reference_mode"))
        .args(["--recipe", "recipe.json"])
        .output()
        .expect("the instrument runs");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--spec PATH"), "{stderr}");
    assert!(!stderr.contains("--recipe PATH"), "{stderr}");
}

fn run_with(out: &Path, extra: &[&str]) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_reference_mode"))
        .args(["--ticks", "10", "--sample-every", "5"])
        .args(extra)
        .arg("--out")
        .arg(out)
        .output()
        .expect("the instrument runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_str(&std::fs::read_to_string(out).unwrap()).unwrap()
}

/// `--litter-x N` (#781) stands N times the settled litter, and records it;
/// without the flag, or at 1, the run is the committed mesocosm's.
#[test]
fn litter_x_scales_the_founding_litter_and_leaves_the_default_run_unchanged() {
    let dir = scratch("litter-x");
    let default = run_with(&dir.join("default.json"), &[]);
    let one = run_with(&dir.join("one.json"), &["--litter-x", "1"]);
    let ten = run_with(&dir.join("ten.json"), &["--litter-x", "10"]);

    assert_eq!(default["litter_x"], 1);
    assert_eq!(ten["litter_x"], 10);
    assert_eq!(default["report"], one["report"]);
    let carcass_n = |a: &serde_json::Value| {
        a["report"]["samples"][0]["carcass_nutrient"]
            .as_f64()
            .unwrap()
    };
    let ratio = carcass_n(&ten) / carcass_n(&default);
    assert!((ratio - 10.0).abs() < 1e-3, "{ratio}");
}

#[test]
fn litter_x_refuses_zero() {
    let output = Command::new(env!("CARGO_BIN_EXE_reference_mode"))
        .args(["--litter-x", "0"])
        .output()
        .expect("the instrument runs");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--litter-x"), "{stderr}");
}

/// `--decomposer-founders N` (#829) seeds N decomposer founders, and records
/// N in the artifact and the summary; without it the run is the committed
/// mesocosm's, byte for byte, and at one a cell (81) its report is.
#[test]
fn decomposer_founders_seeds_and_records_the_count_and_the_default_is_unchanged() {
    let dir = scratch("decomposer-founders");
    let default = run_with(&dir.join("default.json"), &[]);
    let ten = run_with(&dir.join("ten.json"), &["--decomposer-founders", "10"]);
    let all = run_with(&dir.join("all.json"), &["--decomposer-founders", "81"]);

    assert!(default.get("decomposer_founders").is_none());
    assert_eq!(default["report"]["founders"]["founders"], 81);
    assert_eq!(ten["decomposer_founders"], 10);
    assert_eq!(ten["report"]["founders"]["founders"], 10);
    assert_eq!(all["report"], default["report"]);
    let md = std::fs::read_to_string(dir.join("ten.md")).unwrap();
    assert!(md.contains("10 decomposer founders"), "{md}");
    let md = std::fs::read_to_string(dir.join("default.md")).unwrap();
    assert!(!md.contains("decomposer founders,"), "{md}");
}

#[test]
fn decomposer_founders_refuses_zero_and_needs_decomposers() {
    for (args, why) in [
        (&["--decomposer-founders", "0"][..], "at least 1"),
        (
            &["--no-decomposers", "--decomposer-founders", "10"][..],
            "needs decomposer founders",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_reference_mode"))
            .args(["--ticks", "1"])
            .args(args)
            .output()
            .expect("the instrument runs");
        assert!(!output.status.success(), "{args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("--decomposer-founders"), "{stderr}");
        assert!(stderr.contains(why), "{stderr}");
        assert!(stderr.contains("usage:"), "{stderr}");
    }
}

/// `--body-reach-pieces k` (#795) sets the body-reach coefficient from k and
/// records k, the coefficient, the founders' reach and the clamp in the
/// artifact and summary; without it, or at 0, the run is the committed
/// mesocosm's, byte for byte.
#[test]
fn body_reach_pieces_sets_and_records_the_coefficient_and_zero_is_absent() {
    let dir = scratch("body-reach-pieces");
    let read = |name: &str| {
        let out = dir.join(name);
        (
            std::fs::read_to_string(&out).unwrap(),
            std::fs::read_to_string(out.with_extension("md")).unwrap(),
        )
    };
    let default = run_with(&dir.join("default.json"), &[]);
    run_with(&dir.join("zero.json"), &["--body-reach-pieces", "0"]);
    let ten = run_with(&dir.join("ten.json"), &["--body-reach-pieces", "10"]);
    let one = run_with(&dir.join("one.json"), &["--body-reach-pieces", "1"]);

    assert_eq!(read("default.json"), read("zero.json"));
    assert!(default.get("body_reach").is_none());
    let reach = &ten["body_reach"];
    assert_eq!(reach["pieces"], 10.0);
    assert!(reach["coefficient"].as_f64().unwrap() > 0.0, "{reach}");
    assert!(reach["founder_reach"].as_f64().unwrap() > 1.37, "{reach}");
    assert_eq!(reach["clamped"], false);
    assert_ne!(default["report"], ten["report"]);
    let (_, md) = read("ten.json");
    assert!(md.contains("## Feeding reach"), "{md}");
    assert!(md.contains("clamped to 0: no"), "{md}");
    // About one piece is the contact-only reach at 1x: the clamp applies and
    // the world is the committed one.
    assert_eq!(one["body_reach"]["clamped"], true);
    assert_eq!(one["body_reach"]["coefficient"], 0.0);
    assert_eq!(default["report"], one["report"]);
}

#[test]
fn body_reach_pieces_needs_decomposer_founders() {
    let output = Command::new(env!("CARGO_BIN_EXE_reference_mode"))
        .args([
            "--ticks",
            "1",
            "--no-decomposers",
            "--body-reach-pieces",
            "10",
        ])
        .output()
        .expect("the instrument runs");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--body-reach-pieces"), "{stderr}");
}

/// `--litter-half-life L` and `--enzyme-half-saturation K` (#805) set the
/// saturating-decomposition kinetics in domain units and record L, `T_p`,
/// `k_d` and K in founder capabilities and absolute units in the artifact and
/// summary; without them nothing is recorded and the run is unchanged.
#[test]
fn kinetics_options_set_and_record_the_decomposition_parameters() {
    let dir = scratch("kinetics");
    let default = run_with(&dir.join("default.json"), &[]);
    let set = run_with(
        &dir.join("set.json"),
        &["--litter-half-life", "1", "--enzyme-half-saturation", "2"],
    );
    assert!(default.get("decomposition_kinetics").is_none());
    let k = &set["decomposition_kinetics"];
    assert_eq!(k["litter_half_life"], 1.0);
    assert_eq!(k["producer_lifespan"], 229.0);
    let k_d = k["decay_constant"].as_f64().unwrap();
    assert!((k_d - std::f64::consts::LN_2 / 229.0).abs() < 1e-6, "{k}");
    assert_eq!(k["half_saturation_founder_capabilities"], 2.0);
    let capability = k["founder_drain_capability"].as_f64().unwrap();
    let absolute = k["half_saturation"].as_f64().unwrap();
    assert!((absolute - 2.0 * capability).abs() < 1e-5, "{k}");
    assert_ne!(default["report"], set["report"]);
    let md = std::fs::read_to_string(dir.join("set.md")).unwrap();
    assert!(md.contains("## Decomposition kinetics"), "{md}");
}

/// `--decomposer-mesocosm` (#884) runs the decomposer mesocosm: its rules at
/// the values the mesocosm crate declares, the run the explicit flags at L 1
/// and K 1 give, state for state, with the size-scaled base rate at the
/// world's `S_p` (#862), and it records the declaration.
#[test]
fn decomposer_mesocosm_runs_the_declared_rules() {
    let dir = scratch("decomposer-mesocosm");
    let explicit = run_with(
        &dir.join("explicit.json"),
        &["--litter-half-life", "1", "--enzyme-half-saturation", "1"],
    );
    let declared = run_with(&dir.join("declared.json"), &["--decomposer-mesocosm"]);
    for key in ["report", "decomposition_kinetics"] {
        assert!(explicit.get(key).is_some(), "{key}");
        assert_eq!(declared[key], explicit[key], "{key}");
    }
    assert!(explicit.get("decomposer_mesocosm").is_none());
    let d = &declared["decomposer_mesocosm"];
    assert!(d.get("established_producer_structure").is_none(), "{d}");
    assert_eq!(d["litter_half_life"], 1.0);
    assert_eq!(d["enzyme_half_saturation"], 1.0);
    assert_eq!(d["founding_litter_ratio"], 170.0);
    let md = std::fs::read_to_string(dir.join("declared.md")).unwrap();
    assert!(md.contains("## Decomposition kinetics"), "{md}");
}

/// `--decomposer-mesocosm` sets its rules itself, so it refuses a flag that
/// would set one of them again or move `S_p`, and it needs decomposer
/// founders.
#[test]
fn decomposer_mesocosm_refuses_its_rules_flags_and_no_decomposers() {
    for extra in [
        &["--established-producer-structure", "4"][..],
        &["--litter-half-life", "2"][..],
        &["--no-decomposers"][..],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_reference_mode"))
            .args(["--ticks", "1", "--decomposer-mesocosm"])
            .args(extra)
            .output()
            .expect("the instrument runs");
        assert!(!output.status.success(), "{extra:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("--decomposer-mesocosm"), "{stderr}");
    }
}

/// Saturating decomposition is in every world since #864, so
/// `--enzyme-half-saturation` alone sets K at the world's own `k_d`, a litter
/// half-life of one producer lifespan, and records it.
#[test]
fn enzyme_half_saturation_alone_runs_at_the_worlds_own_decay_constant() {
    let dir = scratch("enzyme-alone");
    let alone = run_with(&dir.join("alone.json"), &["--enzyme-half-saturation", "2"]);
    let k = &alone["decomposition_kinetics"];
    assert_eq!(k["litter_half_life"], 1.0);
    let k_d = k["decay_constant"].as_f64().unwrap();
    assert_eq!(
        k_d as f32,
        explorers_sim::DEFAULT_CARCASS_DECAY_CONSTANT,
        "{k}"
    );
    assert_eq!(k["half_saturation_founder_capabilities"], 2.0);
}

/// The size-scaled base rate has no switch since #862: the retired
/// `--size-scaled-base-rate` is refused as unknown.
#[test]
fn the_retired_size_scaled_base_rate_flag_is_refused() {
    let output = Command::new(env!("CARGO_BIN_EXE_reference_mode"))
        .args(["--ticks", "1", "--size-scaled-base-rate"])
        .output()
        .expect("the instrument runs");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--size-scaled-base-rate"), "{stderr}");
}

/// `--established-producer-structure S_p` (#814) sets the world's base-rate
/// reference structure in place of its default and records `S_p`, `b` and
/// the founders' upkeep; at the default 6.937 the run is the default run,
/// and without the flag nothing is recorded. It needs a finite `S_p` above
/// 0.
#[test]
fn established_producer_structure_sets_the_worlds_s_p() {
    let dir = scratch("established-producer-structure");
    let default = run_with(&dir.join("default.json"), &[]);
    let same = run_with(
        &dir.join("same.json"),
        &["--established-producer-structure", "6.937"],
    );
    let set = run_with(
        &dir.join("set.json"),
        &["--established-producer-structure", "11.9"],
    );
    assert!(default.get("base_rate_reference_structure").is_none());
    assert_eq!(default["report"], same["report"]);
    let r = &set["base_rate_reference_structure"];
    let s_p = r["base_rate_reference_structure"].as_f64().unwrap();
    assert!((s_p - 11.9).abs() < 1e-5, "{r}");
    let b = r["mass_specific_base_rate"].as_f64().unwrap();
    let expected = r["base_metabolic_rate"].as_f64().unwrap() / s_p
        + r["structure_maintenance_coefficient"].as_f64().unwrap();
    assert!((b - expected).abs() < 1e-6, "{r}");
    assert!(
        r["decomposer_founder"]["upkeep"]["total"].is_number(),
        "{r}"
    );
    assert!(r["median_producer"]["upkeep"]["total"].is_number(), "{r}");
    assert_ne!(default["report"], set["report"]);
    let md = std::fs::read_to_string(dir.join("set.md")).unwrap();
    assert!(md.contains("/ 11.9 +"), "{md}");

    for args in [
        &["--established-producer-structure", "0"][..],
        &["--established-producer-structure", "inf"][..],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_reference_mode"))
            .args(["--ticks", "1"])
            .args(args)
            .output()
            .expect("the instrument runs");
        assert!(!output.status.success(), "{args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("--established-producer-structure"),
            "{stderr}"
        );
    }
}

/// Living tissue resists every drain in every world since #865, at the
/// world's own `ρ = 1` (world rules, *Living tissue resists drains*): with
/// body reach exposing the producers, the guild drains no living target, and
/// nothing about it is recorded. The retired `--living-resistance` is
/// refused as unknown.
#[test]
fn living_tissue_is_closed_to_the_guild_and_the_retired_flag_is_refused() {
    let dir = scratch("living-resistance");
    let reached = run_with(&dir.join("reached.json"), &["--body-reach-pieces", "10"]);
    assert!(reached.get("living_resistance").is_none());
    let living = reached["report"]["guild"]["drain"]["living"]
        .as_f64()
        .unwrap();
    assert_eq!(living, 0.0, "{}", reached["report"]["guild"]);
    let md = std::fs::read_to_string(dir.join("reached.md")).unwrap();
    assert!(!md.contains("## Living-tissue resistance"), "{md}");

    let output = Command::new(env!("CARGO_BIN_EXE_reference_mode"))
        .args(["--ticks", "1", "--living-resistance"])
        .output()
        .expect("the instrument runs");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--living-resistance"), "{stderr}");
}

/// `--starvation-readout` (#821) writes the founder starvation readout, the
/// starvation cascade being physics in every world (#863); without it nothing
/// is recorded, and the readout leaves the report itself as it was. The
/// retired switches `--starvation-draws-earmark` and `--starvation-shrinks`
/// are refused as unknown.
#[test]
fn the_starvation_readout_follows_the_founders_through_the_cascade() {
    let dir = scratch("starvation-cascade");
    let plain = run_with(&dir.join("plain.json"), &["--ticks", "40"]);
    assert!(plain.get("starvation_cascade").is_none());
    assert!(plain.get("founder_starvation").is_none());

    let read = run_with(
        &dir.join("read.json"),
        &["--ticks", "40", "--starvation-readout"],
    );
    let founders = read["founder_starvation"]["founders"].as_array().unwrap();
    assert_eq!(founders.len(), 81, "one entry per decomposer founder");
    assert!(
        founders
            .iter()
            .any(|f| f["ticks_on_earmark"].as_u64().unwrap() > 0),
        "some founder lives on its earmark within 40 ticks"
    );
    let md = std::fs::read_to_string(dir.join("read.md")).unwrap();
    assert!(md.contains("## Founder starvation readout"), "{md}");
    assert_eq!(read["report"], plain["report"]);

    for flag in ["--starvation-draws-earmark", "--starvation-shrinks"] {
        assert!(refused(&["--ticks", "1", flag]).contains(flag));
    }

    let paired = run_with(
        &dir.join("paired.json"),
        &["--clear-at", "5", "--starvation-readout"],
    );
    for arm in ["perturbed", "control"] {
        assert_eq!(
            paired["founder_starvation"][arm]["founders"]
                .as_array()
                .unwrap()
                .len(),
            81,
            "{arm}"
        );
    }
    let md = std::fs::read_to_string(dir.join("paired.md")).unwrap();
    assert!(
        md.contains("| perturbed |") && md.contains("| control |"),
        "{md}"
    );
}

/// Every run or arm writes the marker readout (#837, #861: marker isolation
/// is physics). `--compatibility-scale S` sets the scale in place of the
/// default and records it in the artifact and summary; without it nothing is
/// recorded. The retired `--marker-isolation` switch is refused as unknown.
#[test]
fn every_run_writes_the_marker_readout_and_a_set_scale_is_recorded() {
    let dir = scratch("marker-isolation");
    let plain = run_with(&dir.join("plain.json"), &["--decomposer-founders", "10"]);
    assert!(plain.get("compatibility_scale").is_none());
    let marker = &plain["report"]["marker"];
    assert_eq!(marker["samples"][0]["line_living"], 10);
    assert_eq!(marker["samples"].as_array().unwrap().len(), 3);
    assert!(marker["producers"]["mean"].as_f64().is_some(), "{marker}");
    let md = std::fs::read_to_string(dir.join("plain.md")).unwrap();
    assert!(!md.contains("## Compatibility scale"), "{md}");
    assert!(md.contains("## Marker readout"), "{md}");

    let set = run_with(
        &dir.join("set.json"),
        &["--decomposer-founders", "10", "--compatibility-scale", "50"],
    );
    assert_eq!(set["compatibility_scale"].as_f64(), Some(50.0));
    let md = std::fs::read_to_string(dir.join("set.md")).unwrap();
    assert!(md.contains("## Compatibility scale"), "{md}");

    let paired = run_with(
        &dir.join("paired.json"),
        &["--decomposer-founders", "10", "--clear-at", "5"],
    );
    for arm in ["perturbed", "control"] {
        assert!(paired["report"][arm]["marker"]["line"].is_object(), "{arm}");
    }

    assert!(refused(&["--compatibility-scale", "0"]).contains("--compatibility-scale"));
    assert!(refused(&["--marker-isolation", "1"]).contains("unknown flag --marker-isolation"));
}

fn refused(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_reference_mode"))
        .args(args)
        .output()
        .expect("the instrument runs");
    assert!(!output.status.success(), "{args:?}");
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// `--early-window C` (#857) reads G's rung 2 over the run (`--ticks` is the
/// window): the decomposer founders alive at tick C and the founders' line's
/// births, and whether both conditions hold. It is recorded in the artifact
/// and summary; without it nothing is. It reads one run with founders, at a
/// tick within it.
#[test]
fn early_window_reports_rung_2s_conditions() {
    let dir = scratch("early-window");
    let plain = run_with(&dir.join("plain.json"), &[]);
    assert!(plain.get("early_window").is_none());

    let window = run_with(
        &dir.join("window.json"),
        &["--ticks", "30", "--early-window", "5"],
    );
    let w = &window["early_window"];
    assert_eq!(w["founder_tick"], 5);
    assert_eq!(w["window_end"], 30);
    let alive = w["founders_alive"].as_u64().unwrap();
    let births = window["report"]["recruitment"]["births"]
        .as_array()
        .unwrap()
        .len() as u64;
    assert!(alive > 0, "{w}");
    assert_eq!(w["line_births"].as_u64(), Some(births));
    assert_eq!(w["holds"].as_bool(), Some(alive > 0 && births > 0));
    let md = std::fs::read_to_string(dir.join("window.md")).unwrap();
    assert!(md.contains("## Early window"), "{md}");

    for (args, why) in [
        (&["--ticks", "10", "--early-window", "11"][..], "within"),
        (
            &["--ticks", "10", "--early-window", "5", "--clear-at", "5"][..],
            "--clear-at",
        ),
        (
            &["--ticks", "10", "--early-window", "5", "--no-decomposers"][..],
            "decomposer founders",
        ),
    ] {
        let stderr = refused(args);
        assert!(stderr.contains("--early-window"), "{stderr}");
        assert!(stderr.contains(why), "{stderr}");
    }
}

/// `--seeds N --passes-needed K` (#857) runs seeds from `--seed` on, one at
/// a time, under "at least K of N", and stops as soon as the outcome is
/// decided: it records the outcome, the seeds run and each run's artifact,
/// which is the single run of its seed. A seed passes by the early window
/// when one is asked for. `--failures-undecided` makes the failure bound
/// leave the reading undecided rather than failed.
#[test]
fn seeds_in_sequence_stop_once_the_rule_is_decided() {
    let dir = scratch("sequence");
    let window = ["--ticks", "10", "--early-window", "5"];
    let rule = ["--seeds", "3", "--passes-needed", "2"];
    let seq = run_with(&dir.join("seq.json"), &[&window[..], &rule[..]].concat());
    let reading = &seq["sequence"];
    let runs = seq["runs"].as_array().unwrap();
    let seeds = reading["seeds"].as_array().unwrap();
    assert_eq!(seeds.len(), runs.len());
    for (i, run) in runs.iter().enumerate() {
        let seed = i as u64 + 1;
        assert_eq!(run["seed"].as_u64(), Some(seed));
        let holds = run["early_window"]["holds"].as_bool().unwrap();
        assert_eq!(seeds[i], serde_json::json!([seed, holds]));
    }
    let passes = reading["passes"].as_u64().unwrap();
    let failures = reading["failures"].as_u64().unwrap();
    let outcome = reading["outcome"].as_str().unwrap();
    match outcome {
        "passed" => assert_eq!(passes, 2),
        "failed" => assert_eq!(failures, 2),
        _ => panic!("{reading}"),
    }
    assert_eq!(passes + failures, runs.len() as u64);
    let md = std::fs::read_to_string(dir.join("seq.md")).unwrap();
    assert!(md.contains("## Seeds in sequence"), "{md}");

    let single = run_with(
        &dir.join("one.json"),
        &[&window[..], &["--seed", "1"]].concat(),
    );
    assert_eq!(
        runs[0], single,
        "a run in the sequence is its seed's single run"
    );

    let open = run_with(
        &dir.join("open.json"),
        &[&window[..], &rule[..], &["--failures-undecided"]].concat(),
    );
    let expected = if outcome == "failed" {
        "undecided"
    } else {
        outcome
    };
    assert_eq!(open["sequence"]["outcome"].as_str(), Some(expected));
    assert_eq!(open["sequence"]["seeds"], reading["seeds"]);

    for args in [
        &["--seeds", "3", "--passes-needed", "4"][..],
        &["--seeds", "3", "--passes-needed", "0"][..],
        &["--passes-needed", "2"][..],
        &["--seeds", "3"][..],
        &["--failures-undecided"][..],
    ] {
        let stderr = refused(&[&["--ticks", "1"][..], args].concat());
        assert!(stderr.contains("--passes-needed"), "{args:?}: {stderr}");
    }
}

/// `--abort-on-line-extinction` (#857) stops a run in the step the
/// decomposer founders' line has no living member, and records the tick in
/// the report and the summary; a run whose line lives is unchanged. It needs
/// decomposer founders.
#[test]
fn abort_on_line_extinction_stops_the_run_and_reports_the_tick() {
    let dir = scratch("abort");
    let plain = run_with(&dir.join("plain.json"), &[]);
    let asked = run_with(&dir.join("asked.json"), &["--abort-on-line-extinction"]);
    assert_eq!(asked["abort_on_line_extinction"].as_bool(), Some(true));
    assert!(plain.get("abort_on_line_extinction").is_none());
    assert_eq!(
        asked["report"], plain["report"],
        "the line lives to tick 10"
    );

    let lone = ["--decomposer-founders", "1", "--ticks", "300"];
    let aborted = run_with(
        &dir.join("aborted.json"),
        &[&lone[..], &["--abort-on-line-extinction"]].concat(),
    );
    let at = aborted["report"]["aborted_at"]
        .as_u64()
        .expect("a lone founder's line dies within 300 ticks");
    assert!(at < 300);
    assert_eq!(aborted["report"]["founders"]["end_tick"].as_u64(), Some(at));
    assert_eq!(aborted["report"]["guild"]["passes"].as_bool(), Some(false));
    let md = std::fs::read_to_string(dir.join("aborted.md")).unwrap();
    assert!(
        md.contains(&format!(
            "aborted at tick {at}: the founders' line is extinct"
        )),
        "{md}"
    );

    let stderr = refused(&["--no-decomposers", "--abort-on-line-extinction"]);
    assert!(stderr.contains("--abort-on-line-extinction"), "{stderr}");
    assert!(stderr.contains("decomposer founders"), "{stderr}");
}

/// Growth dilutes wear in every world (#858), so #840's
/// `--growth-dilutes-wear` is gone: a command that still passes it is
/// refused, rather than run as though it chose something.
#[test]
fn the_retired_growth_dilution_flag_is_refused() {
    let stderr = refused(&["--growth-dilutes-wear"]);
    assert!(stderr.contains("--growth-dilutes-wear"), "{stderr}");
}
