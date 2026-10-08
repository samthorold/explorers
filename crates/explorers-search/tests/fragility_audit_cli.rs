//! The fragility audit's resume guard (#729, #741): a run refuses an `--out`
//! holding rows written at another hyphal or network setting, so two arms
//! never mix in one file.

use std::path::PathBuf;
use std::process::{Command, Output};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "explorers-fragility-audit-cli-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The audit over the repo's atlas, cell 0 unperturbed, writing no rollouts
/// (`--limit 0`): only the resume guard and the summary run.
fn audit(out: &std::path::Path, extra: &[&str]) -> Output {
    let atlas = concat!(env!("CARGO_MANIFEST_DIR"), "/../../atlas.json");
    Command::new(env!("CARGO_BIN_EXE_fragility_audit"))
        .args(["--atlas", atlas, "--configs", "atlas:0", "--draws", "0"])
        .args(["--limit", "0", "--out"])
        .arg(out)
        .args(extra)
        .output()
        .expect("the fragility audit runs")
}

/// A network-off row (no `network` field), as an unpinned run writes it.
const OFF_ROW: &str = r#"{"source":"atlas","config_index":0,"atlas_fingerprint":9384858128293480890,"radius":0.0,"draw":0,"horizon":2000,"bloom_stop":{"tick":300,"factor":10.0},"jitter":[0.0],"prefilter_cliff":null,"seeds":[{"seed":1000,"verdict":"live","fitness":0.3}],"persisted_fraction":1.0,"modal_verdict":"live"}"#;

/// #741: resuming a network-on run into a file of network-off rows is
/// refused, naming the network; the off arm resumes into it as before.
#[test]
fn resuming_into_rows_at_another_network_setting_is_refused() {
    let dir = scratch("network");
    let out = dir.join("rows.jsonl");
    std::fs::write(&out, format!("{OFF_ROW}\n")).unwrap();

    let off = audit(&out, &[]);
    assert!(
        off.status.success(),
        "{}",
        String::from_utf8_lossy(&off.stderr)
    );

    let on = audit(&out, &["--network-connection-cap", "4"]);
    assert!(!on.status.success());
    let stderr = String::from_utf8_lossy(&on.stderr);
    assert!(
        stderr.contains("holds rows at another network setting"),
        "{stderr}"
    );
}
