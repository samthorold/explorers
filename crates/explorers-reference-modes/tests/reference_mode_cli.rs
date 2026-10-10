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
