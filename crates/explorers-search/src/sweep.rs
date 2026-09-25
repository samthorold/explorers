//! The resumable JSON-lines sweep shape the research bins share
//! (`settling_time`, `energy_death_check`, `energy_bound_check`,
//! `permanence_crosscheck`): one row per config appended to an
//! output file as soon as it is complete, configs already present skipped on
//! start, a fixed sweep order (atlas cells by index, then the seed-421 LHS
//! sample by index, then any config of another draw a `sample@S:i` selector
//! names, by seed and index) and a `--limit` cap — so a sweep driven as a loop of short
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

use explorers_genesis::{InitialDistribution, WorldParameters};

use crate::config_source::ConfigSource;
use crate::search::{ParameterRange, SearchBoxMismatch, check_search_box, decode, default_ranges};

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

/// The atlas file's shape, as far as the sweeps read it: the search box it
/// was drawn under (absent on an atlas from before #559) and the unit vector
/// of each live cell.
#[derive(serde::Deserialize)]
pub struct AtlasFile {
    #[serde(default)]
    pub search_box: Option<Vec<ParameterRange>>,
    pub cells: Vec<AtlasCellUnit>,
}

#[derive(serde::Deserialize)]
pub struct AtlasCellUnit {
    pub unit: Vec<f64>,
}

/// An atlas's live cells as the research sweeps read them (#559): each
/// cell's unit vector together with the search box it decodes over. A unit
/// names a world only with its box, so the cells are decoded here, over the
/// atlas's own box, and nowhere else.
#[derive(Clone, Debug)]
pub struct AtlasUnits {
    search_box: Vec<ParameterRange>,
    units: Vec<Vec<f64>>,
}

impl Default for AtlasUnits {
    /// No atlas: no cells, over the full box.
    fn default() -> Self {
        AtlasUnits {
            search_box: default_ranges(),
            units: Vec::new(),
        }
    }
}

impl AtlasUnits {
    /// Cells `units` drawn under `search_box`. Panics if a unit vector does
    /// not span the box.
    pub fn new(search_box: Vec<ParameterRange>, units: Vec<Vec<f64>>) -> Self {
        for (i, unit) in units.iter().enumerate() {
            assert_eq!(
                unit.len(),
                search_box.len(),
                "atlas:{i} has a {}-dim unit vector but the atlas's search box has {} dims",
                unit.len(),
                search_box.len()
            );
        }
        AtlasUnits { search_box, units }
    }

    /// The number of live cells.
    pub fn len(&self) -> usize {
        self.units.len()
    }

    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }

    /// The search box the cells decode over: the one the atlas records, or
    /// the full box `default_ranges()` for an atlas from before #559.
    pub fn search_box(&self) -> &[ParameterRange] {
        &self.search_box
    }

    /// The world live cell `index` names, decoded over the atlas's own box.
    pub fn decode(&self, index: usize) -> (WorldParameters, InitialDistribution) {
        decode(&self.units[index], &self.search_box)
    }

    /// Check that `ranges`, the box a reader would decode the cells over, is
    /// the atlas's own (#559).
    pub fn check_search_box(&self, ranges: &[ParameterRange]) -> Result<(), SearchBoxMismatch> {
        check_search_box(&self.search_box, ranges)
    }
}

/// The atlas's live cells, in file order, with the search box they decode
/// over (legacy atlas: the full box). Panics if it cannot be read or parsed,
/// or if a cell's unit vector does not span the box.
pub fn read_atlas_units(path: &Path) -> AtlasUnits {
    let contents =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let atlas: AtlasFile =
        serde_json::from_str(&contents).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    AtlasUnits::new(
        atlas.search_box.unwrap_or_else(default_ranges),
        atlas.cells.into_iter().map(|c| c.unit).collect(),
    )
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
/// first, then the seed-421 LHS sample, each by index, then any config of
/// another draw that `filter` names, by draw seed and index), minus `done`,
/// restricted to `filter` when one is given, and capped at `limit`. Every
/// draw has `sample_len` configs; an index past it is never planned.
pub fn plan_tasks(
    atlas_len: usize,
    sample_len: usize,
    filter: Option<&HashSet<(ConfigSource, usize)>>,
    done: &HashSet<(ConfigSource, usize)>,
    limit: Option<usize>,
) -> Vec<(ConfigSource, usize)> {
    let atlas = (0..atlas_len).map(|i| (ConfigSource::Atlas, i));
    let sample = (0..sample_len).map(|i| (ConfigSource::SAMPLE, i));
    let mut other_draws: Vec<(u64, usize)> = filter
        .into_iter()
        .flatten()
        .filter_map(|&(source, i)| match source {
            ConfigSource::Sample(seed) if source != ConfigSource::SAMPLE && i < sample_len => {
                Some((seed, i))
            }
            _ => None,
        })
        .collect();
    other_draws.sort_unstable();
    let other_draws = other_draws
        .into_iter()
        .map(|(seed, i)| (ConfigSource::Sample(seed), i));
    atlas
        .chain(sample)
        .filter(|key| filter.is_none_or(|f| f.contains(key)))
        .chain(other_draws)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A filter may name configs of another LHS draw (`sample@S:i`): they
    /// are planned after the default order, by draw and index, and are
    /// skipped once done like any other config.
    #[test]
    fn a_filter_naming_another_draw_plans_its_configs_after_the_default_order() {
        use ConfigSource::{Atlas, Sample};
        let filter: HashSet<_> = [
            (Sample(9421), 3),
            (Sample(9421), 0),
            (Sample(77), 1),
            (ConfigSource::SAMPLE, 1),
            (Atlas, 0),
            (Sample(9421), 5), // past the draw's length: dropped, as for sample:5
        ]
        .into_iter()
        .collect();
        let none = HashSet::new();
        assert_eq!(
            plan_tasks(2, 4, Some(&filter), &none, None),
            vec![
                (Atlas, 0),
                (ConfigSource::SAMPLE, 1),
                (Sample(77), 1),
                (Sample(9421), 0),
                (Sample(9421), 3),
            ]
        );
        let done: HashSet<_> = [(Sample(9421), 0)].into_iter().collect();
        assert_eq!(
            plan_tasks(2, 4, Some(&filter), &done, Some(4)),
            vec![
                (Atlas, 0),
                (ConfigSource::SAMPLE, 1),
                (Sample(77), 1),
                (Sample(9421), 3),
            ]
        );
    }
}
