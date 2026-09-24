//! The resumable JSON-lines sweep shape the research bins share
//! (`settling_time`, `energy_death_check`, `energy_bound_check`,
//! `permanence_crosscheck`): one row per config appended to an
//! output file as soon as it is complete, configs already present skipped on
//! start, a fixed sweep order (atlas cells by index, then the LHS sample by
//! index) and a `--limit` cap — so a sweep driven as a loop of short
//! foreground calls produces a file byte-identical to one uninterrupted run.
//!
//! A (config, seed) run in these sweeps carries two wall-clock budgets
//! (#523): the **simulation budget** (`--run-timeout-secs`) bounds the step
//! loop, and a rollout that exhausts it is recorded as [`TIMEOUT_MODE`]; the
//! **evaluation budget** ([`EVAL_TIMEOUT_FLAG`]) bounds the terminal
//! evaluation of a rollout that finished simulating, and one that exhausts it
//! is recorded as [`EVAL_TIMEOUT_MODE`]. The two stay distinct because a
//! rollout that reached the horizon but cannot be evaluated in time (a dense
//! terminal roster) is a different fact about a config from one that could
//! not be simulated. Neither is a verdict: both are unfinished
//! ([`is_unfinished`]) and excluded wherever a sweep reads finished runs.

use std::collections::HashSet;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

use explorers_genesis::{EvalConfig, FitnessBreakdown};
use explorers_genesis_eval::{RolloutObservations, evaluate_from_log_within};

use crate::config_source::ConfigSource;

/// The mode a run records when its step loop exhausted the simulation budget
/// (`--run-timeout-secs`).
pub const TIMEOUT_MODE: &str = "timeout";

/// The mode a run records when it finished simulating but its terminal
/// evaluation exhausted the evaluation budget: no verdict was reached, so it
/// is neither a failure classification nor [`TIMEOUT_MODE`].
pub const EVAL_TIMEOUT_MODE: &str = "eval_timeout";

/// The command-line flag that sets the evaluation budget, in seconds, in
/// every bin that has `--run-timeout-secs`.
pub const EVAL_TIMEOUT_FLAG: &str = "--eval-timeout-secs";

/// The evaluation budget when [`EVAL_TIMEOUT_FLAG`] is not given.
pub const DEFAULT_EVAL_TIMEOUT_SECS: u64 = 300;

/// A run that reached no verdict — it exhausted either budget — and so is
/// excluded wherever a sweep reads finished runs.
pub fn is_unfinished(mode: &str) -> bool {
    mode == TIMEOUT_MODE || mode == EVAL_TIMEOUT_MODE
}

/// The evaluator's terminal verdict on a rollout that finished simulating,
/// under an evaluation `budget` of wall clock starting now: `None` when the
/// budget is spent first (record [`EVAL_TIMEOUT_MODE`]). The evaluation
/// abandons cooperatively on the calling thread, so nothing keeps running;
/// a verdict that is reached is bit-identical to an unbudgeted one. A budget
/// too large to be a deadline (`Duration::MAX`) is no deadline.
pub fn evaluate_within_budget(
    world: &explorers_sim::World,
    observations: &RolloutObservations,
    config: &EvalConfig,
    max_ticks: u64,
    budget: Duration,
) -> Option<FitnessBreakdown> {
    match Instant::now().checked_add(budget) {
        Some(deadline) => {
            evaluate_from_log_within(world, observations, config, max_ticks, deadline)
        }
        None => Some(explorers_genesis_eval::evaluate_from_log(
            world,
            observations,
            config,
            max_ticks,
        )),
    }
}

/// The atlas file's shape, as far as the sweeps read it: the unit vector of
/// each live cell.
#[derive(serde::Deserialize)]
pub struct AtlasFile {
    pub cells: Vec<AtlasCellUnit>,
}

#[derive(serde::Deserialize)]
pub struct AtlasCellUnit {
    pub unit: Vec<f64>,
}

/// The atlas's live-cell unit vectors, in file order.
pub fn read_atlas_units(path: &Path) -> Vec<Vec<f64>> {
    let contents =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let atlas: AtlasFile =
        serde_json::from_str(&contents).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    atlas.cells.into_iter().map(|c| c.unit).collect()
}

/// The `(source, config_index)` keys already present in a JSON-lines output
/// file — the configs a resumed sweep skips. A missing file is an empty set.
pub fn done_configs(path: &Path) -> HashSet<(ConfigSource, usize)> {
    #[derive(serde::Deserialize)]
    struct Key {
        source: ConfigSource,
        config_index: usize,
    }
    let Ok(contents) = std::fs::read_to_string(path) else {
        return HashSet::new();
    };
    contents
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let key: Key = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("{}: unparseable row {line:?}: {e}", path.display()));
            (key.source, key.config_index)
        })
        .collect()
}

/// The configs an invocation runs, in the fixed sweep order (atlas cells
/// first, then the LHS sample, each by index), minus `done`, restricted to
/// `filter` when one is given, and capped at `limit`.
pub fn plan_tasks(
    atlas_len: usize,
    sample_len: usize,
    filter: Option<&HashSet<(ConfigSource, usize)>>,
    done: &HashSet<(ConfigSource, usize)>,
    limit: Option<usize>,
) -> Vec<(ConfigSource, usize)> {
    let atlas = (0..atlas_len).map(|i| (ConfigSource::Atlas, i));
    let sample = (0..sample_len).map(|i| (ConfigSource::Sample, i));
    atlas
        .chain(sample)
        .filter(|key| filter.is_none_or(|f| f.contains(key)))
        .filter(|key| !done.contains(key))
        .take(limit.unwrap_or(usize::MAX))
        .collect()
}

/// Every row of a JSON-lines file; a missing file is no rows.
pub fn read_rows<R: serde::de::DeserializeOwned>(path: &Path) -> Vec<R> {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    contents
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("{}: unparseable row {line:?}: {e}", path.display()))
        })
        .collect()
}

/// Append one row as one JSON line, creating the file and its directory.
pub fn append_row<R: serde::Serialize>(path: &Path, row: &R) {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap_or_else(|e| panic!("open {}: {e}", path.display()));
    let mut line = serde_json::to_string(row).expect("serialise row");
    line.push('\n');
    file.write_all(line.as_bytes())
        .unwrap_or_else(|e| panic!("append {}: {e}", path.display()));
}
