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

/// K has no effect while `k_d` is 0, so `--enzyme-half-saturation` without
/// `--litter-half-life` is refused.
#[test]
fn enzyme_half_saturation_without_litter_half_life_is_refused() {
    let output = Command::new(env!("CARGO_BIN_EXE_reference_mode"))
        .args(["--ticks", "1", "--enzyme-half-saturation", "1"])
        .output()
        .expect("the instrument runs");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--enzyme-half-saturation"), "{stderr}");
    assert!(stderr.contains("--litter-half-life"), "{stderr}");
}
