//! Re-project a recipe from an atlas already on disk (#531).
//!
//! The search CLI writes the atlas *before* the gated elite refinement (#404)
//! runs, so a refinement that dies leaves the atlas intact. [`read_atlas`] and
//! [`reproject`] spend that: they run the refinement and recipe projection
//! against the atlas file alone, without re-running the search — a recovery
//! that costs minutes, not hours, and a way to compare projection settings on
//! fixed data rather than across two searches' draws.
//!
//! The re-projection is exactly the in-run one: the cells carry the `unit`
//! vectors the refinement re-evaluates, and the atlas records the search's
//! seed and horizon ([`AtlasProvenance`]), which is everything else it reads.

use std::path::{Path, PathBuf};

use crate::qd::{Atlas, AtlasProvenance, RefinedProjection, RefinementConfig, refined_best_recipe};
use crate::search::default_ranges;

/// Why an atlas file could not be written, read, or re-projected.
#[derive(Debug)]
pub enum AtlasFileError {
    /// Reading or writing the atlas file failed (a missing file lands here).
    Io {
        path: PathBuf,
        error: std::io::Error,
    },
    /// The file did not parse as an atlas.
    Malformed {
        path: PathBuf,
        error: serde_json::Error,
    },
    /// The atlas records no search seed and horizon (it predates #531), and
    /// they were not given: the refinement cannot be reproduced without them.
    MissingProvenance,
    /// A given seed or horizon contradicts the one the atlas records. Each entry
    /// names the knob and both values.
    ProvenanceMismatch { mismatches: Vec<String> },
}

impl std::fmt::Display for AtlasFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AtlasFileError::Io { path, error } => write!(f, "atlas {}: {error}", path.display()),
            AtlasFileError::Malformed { path, error } => {
                write!(f, "atlas {} is malformed: {error}", path.display())
            }
            AtlasFileError::MissingProvenance => write!(
                f,
                "the atlas does not record the search seed and horizon it was drawn \
                 under (it predates #531); give the original run's --seed and \
                 --max-ticks to re-project it"
            ),
            AtlasFileError::ProvenanceMismatch { mismatches } => write!(
                f,
                "the atlas was drawn under a different search than the one given, \
                 refusing to re-project it: {}",
                mismatches.join("; ")
            ),
        }
    }
}

impl std::error::Error for AtlasFileError {}

/// Write `atlas` to `path` as pretty JSON — the search CLI's atlas output.
pub fn write_atlas(atlas: &Atlas, path: &Path) -> Result<(), AtlasFileError> {
    let json = serde_json::to_string_pretty(atlas).expect("an atlas serialises");
    std::fs::write(path, json).map_err(|error| AtlasFileError::Io {
        path: path.to_path_buf(),
        error,
    })
}

/// Read back an atlas written by [`write_atlas`].
pub fn read_atlas(path: &Path) -> Result<Atlas, AtlasFileError> {
    let text = std::fs::read_to_string(path).map_err(|error| AtlasFileError::Io {
        path: path.to_path_buf(),
        error,
    })?;
    serde_json::from_str(&text).map_err(|error| AtlasFileError::Malformed {
        path: path.to_path_buf(),
        error,
    })
}

/// The projection settings a re-projection runs under.
#[derive(Clone, Debug)]
pub struct ReprojectSettings {
    /// Top live cells to refine ([`RefinementConfig::top_k`]).
    pub top_k: usize,
    /// Refinement ensemble size ([`RefinementConfig::ensemble_size`]).
    pub ensemble_size: u32,
    /// The search seed, for an atlas that does not record it.
    pub seed: Option<u64>,
    /// The search horizon, for an atlas that does not record it.
    pub max_ticks: Option<u64>,
}

/// Refine the atlas's top live cells and project the recipe exactly as the run
/// that drew the atlas does ([`refined_best_recipe`]), under the search seed and
/// horizon it records (or, for an atlas that records none, the ones given).
pub fn reproject(
    atlas: &Atlas,
    settings: &ReprojectSettings,
) -> Result<RefinedProjection, AtlasFileError> {
    let AtlasProvenance { seed, max_ticks } = provenance(atlas, settings)?;
    let refinement = RefinementConfig {
        top_k: settings.top_k,
        ensemble_size: settings.ensemble_size,
        max_ticks,
    };
    Ok(refined_best_recipe(
        atlas,
        &default_ranges(),
        &refinement,
        seed,
    ))
}

/// The seed and horizon to re-project under: the atlas's own, checked against
/// any given; or, for an atlas that records none, the given ones.
fn provenance(
    atlas: &Atlas,
    settings: &ReprojectSettings,
) -> Result<AtlasProvenance, AtlasFileError> {
    let Some(recorded) = atlas.provenance else {
        return match (settings.seed, settings.max_ticks) {
            (Some(seed), Some(max_ticks)) => Ok(AtlasProvenance { seed, max_ticks }),
            _ => Err(AtlasFileError::MissingProvenance),
        };
    };
    let mut mismatches = Vec::new();
    if let Some(seed) = settings.seed.filter(|&s| s != recorded.seed) {
        mismatches.push(format!("seed: atlas {}, given {seed}", recorded.seed));
    }
    if let Some(max_ticks) = settings.max_ticks.filter(|&t| t != recorded.max_ticks) {
        mismatches.push(format!(
            "max_ticks: atlas {}, given {max_ticks}",
            recorded.max_ticks
        ));
    }
    if mismatches.is_empty() {
        Ok(recorded)
    } else {
        Err(AtlasFileError::ProvenanceMismatch { mismatches })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qd::{QdConfig, RefinementConfig, refined_best_recipe, run_qd};
    use crate::search::default_ranges;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "explorers-atlas-file-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn tiny() -> QdConfig {
        QdConfig {
            ensemble_size: 1,
            max_ticks: 20,
            batch: 4,
            generations: 1,
            ..QdConfig::default()
        }
    }

    fn settings(top_k: usize, ensemble_size: u32) -> ReprojectSettings {
        ReprojectSettings {
            top_k,
            ensemble_size,
            seed: None,
            max_ticks: None,
        }
    }

    /// A small hand-built atlas: one live cell, and the provenance given.
    fn one_cell_atlas(provenance: Option<AtlasProvenance>) -> Atlas {
        let atlas = run_qd(&tiny(), 7, &mut ChaCha8Rng::seed_from_u64(7));
        Atlas {
            provenance,
            cells: atlas.cells.into_iter().take(1).collect(),
            ..atlas
        }
    }

    #[test]
    fn the_same_search_writes_a_byte_identical_atlas_file() {
        // #536: `atlas:N` in every research sweep is a position in this file,
        // and notes cite its SHA-1 — so the same search must write the same
        // bytes, not the same cells in the archive's HashMap order.
        let config = QdConfig {
            batch: 8,
            generations: 2,
            ..tiny()
        };
        let dir = scratch("byte-identical");
        let (first, second) = (dir.join("first.json"), dir.join("second.json"));
        let atlas = run_qd(&config, 42, &mut ChaCha8Rng::seed_from_u64(42));
        assert!(atlas.cells.len() > 1, "order is only visible with >1 cell");
        write_atlas(&atlas, &first).unwrap();
        write_atlas(
            &run_qd(&config, 42, &mut ChaCha8Rng::seed_from_u64(42)),
            &second,
        )
        .unwrap();
        assert_eq!(
            std::fs::read(&first).unwrap(),
            std::fs::read(&second).unwrap()
        );
        let cells: Vec<[usize; 3]> = atlas.cells.iter().map(|c| c.cell).collect();
        assert!(cells.is_sorted(), "cells in cell-index order: {cells:?}");
    }

    #[test]
    fn the_frontier_tallies_are_written_in_label_order() {
        // #536: the two dead-frontier tallies are label-keyed maps; their
        // written key order must not depend on hashing either.
        let labels = [
            "population_explosion",
            "extinction",
            "nutrient_lockup",
            "monoculture",
            "generalist_dominance",
            "energy_death",
        ];
        let tally: std::collections::BTreeMap<String, usize> = labels
            .iter()
            .enumerate()
            .map(|(i, l)| (l.to_string(), i))
            .collect();
        let atlas = Atlas {
            dead_frontier: tally.clone().into_iter().collect(),
            dead_frontier_apriori: tally.into_iter().collect(),
            ..one_cell_atlas(None)
        };
        let written = serde_json::to_string(&atlas).unwrap();
        let mut sorted = labels;
        sorted.sort();
        for field in ["dead_frontier", "dead_frontier_apriori"] {
            let body = &written[written.find(&format!("\"{field}\":{{")).unwrap()..];
            let body = &body[..body.find('}').unwrap()];
            let mut positions: Vec<(usize, &str)> = labels
                .iter()
                .map(|l| (body.find(&format!("\"{l}\"")).unwrap(), *l))
                .collect();
            positions.sort();
            let written_order: Vec<&str> = positions.into_iter().map(|(_, l)| l).collect();
            assert_eq!(written_order, sorted, "{field}");
        }
    }

    #[test]
    fn an_atlas_that_records_no_provenance_needs_the_seed_and_horizon_given() {
        // An atlas written before #531 does not record the search seed and
        // horizon the refinement reads. Re-projecting it on a guessed seed would
        // quietly be a different draw, so it is refused until both are given.
        let legacy = one_cell_atlas(None);
        let err = reproject(&legacy, &settings(1, 1)).unwrap_err();
        assert!(matches!(err, AtlasFileError::MissingProvenance), "{err}");
        let given = ReprojectSettings {
            seed: Some(7),
            max_ticks: Some(20),
            ..settings(1, 1)
        };
        let recorded = one_cell_atlas(Some(AtlasProvenance {
            seed: 7,
            max_ticks: 20,
        }));
        assert_eq!(
            serde_json::to_string(&reproject(&legacy, &given).unwrap().recipe).unwrap(),
            serde_json::to_string(&reproject(&recorded, &settings(1, 1)).unwrap().recipe).unwrap()
        );
    }

    #[test]
    fn a_seed_or_horizon_that_contradicts_the_atlas_is_refused() {
        // Like --resume: a given value that disagrees with what the atlas
        // records is refused (naming both), never silently preferred.
        let atlas = one_cell_atlas(Some(AtlasProvenance {
            seed: 7,
            max_ticks: 20,
        }));
        let agreeing = ReprojectSettings {
            seed: Some(7),
            max_ticks: Some(20),
            ..settings(1, 1)
        };
        assert!(reproject(&atlas, &agreeing).is_ok());
        let contradicting = ReprojectSettings {
            seed: Some(8),
            max_ticks: Some(2000),
            ..settings(1, 1)
        };
        let err = reproject(&atlas, &contradicting).unwrap_err();
        let AtlasFileError::ProvenanceMismatch { mismatches } = &err else {
            panic!("expected a provenance mismatch, got {err}");
        };
        assert_eq!(mismatches.len(), 2, "{err}");
        assert!(err.to_string().contains("seed"), "{err}");
        assert!(err.to_string().contains("max_ticks"), "{err}");
    }

    #[test]
    fn a_missing_or_malformed_atlas_file_is_a_clear_error() {
        let dir = scratch("unreadable");
        let missing = dir.join("no-such-atlas.json");
        let err = read_atlas(&missing).unwrap_err();
        assert!(matches!(err, AtlasFileError::Io { .. }), "{err}");
        assert!(err.to_string().contains("no-such-atlas.json"), "{err}");

        let malformed = dir.join("truncated.json");
        std::fs::write(&malformed, r#"{"cells": [{"cell": [1, 2"#).unwrap();
        let err = read_atlas(&malformed).unwrap_err();
        assert!(matches!(err, AtlasFileError::Malformed { .. }), "{err}");
        assert!(err.to_string().contains("truncated.json"), "{err}");
    }

    #[test]
    fn an_atlas_with_no_live_cells_projects_no_recipe() {
        // All dead frontier: the same "no recipe" outcome the full run gives,
        // not a panic.
        let dead = Atlas {
            cells: Vec::new(),
            coverage: 0,
            ..one_cell_atlas(Some(AtlasProvenance {
                seed: 7,
                max_ticks: 20,
            }))
        };
        let path = scratch("all-dead").join("atlas.json");
        write_atlas(&dead, &path).unwrap();
        let projection = reproject(&read_atlas(&path).unwrap(), &settings(10, 32)).unwrap();
        assert!(projection.recipe.is_none());
        assert!(projection.refined.is_empty());
    }

    #[test]
    fn a_recipe_reprojected_from_the_atlas_file_is_the_one_the_writing_run_projected() {
        // The whole trust of #531: re-projecting an atlas read back off disk
        // yields exactly the recipe (and refinement audit) the run that wrote it
        // projected, under the same projection settings.
        let seed = 7;
        let config = tiny();
        let atlas = run_qd(&config, seed, &mut ChaCha8Rng::seed_from_u64(seed));
        assert!(
            !atlas.cells.is_empty(),
            "the tiny search must yield a live cell"
        );
        let refinement = RefinementConfig {
            top_k: 2,
            ensemble_size: 2,
            max_ticks: config.max_ticks,
        };
        let in_run = refined_best_recipe(&atlas, &default_ranges(), &refinement, seed);

        let path = scratch("equivalence").join("atlas.json");
        write_atlas(&atlas, &path).unwrap();
        let read_back = read_atlas(&path).unwrap();
        let reprojected = reproject(&read_back, &settings(2, 2)).unwrap();

        assert_eq!(
            serde_json::to_string(&reprojected.recipe).unwrap(),
            serde_json::to_string(&in_run.recipe).unwrap()
        );
        assert_eq!(reprojected.cleared_floor, in_run.cleared_floor);
        assert_eq!(
            serde_json::to_string(&reprojected.refined).unwrap(),
            serde_json::to_string(&in_run.refined).unwrap()
        );
        assert_eq!(
            reprojected.unrefined_live_cells,
            in_run.unrefined_live_cells
        );
    }
}
