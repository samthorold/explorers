//! The resumable JSON-lines sweep shape the research bins share
//! (`settling_time`, `energy_death_check`): one row per config appended to an
//! output file as soon as it is complete, configs already present skipped on
//! start, a fixed sweep order (atlas cells by index, then the LHS sample by
//! index) and a `--limit` cap — so a sweep driven as a loop of short
//! foreground calls produces a file byte-identical to one uninterrupted run.

use std::collections::HashSet;
use std::io::Write;
use std::path::Path;

use crate::config_source::ConfigSource;

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
