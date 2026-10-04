//! Export a config the search or the research sweeps name as a
//! [`WorldRecipe`] the app opens (#581) — the library half of the
//! `export_recipe` bin.
//!
//! A [`RecipeRef`] names an atlas live cell (`cell:I,J,K`, or `atlas:N` by its
//! index in the atlas's `cells` list) or a research config (`sample:i`,
//! `sample@S:i`), in the selector grammar the research bins share
//! ([`parse_config_key`]). [`export_recipe`] decodes it the way those bins do
//! — atlas cells over the atlas's own box through [`Atlas::recipe_for_cell`],
//! sample draws over the full box — so decoding stays in this crate and the
//! app reads only the recipe.

use std::path::Path;

use explorers_sim::WorldRecipe;

use crate::config_source::{ConfigSource, parse_config_key, sample_box, sample_draw};
use crate::qd::Atlas;
use crate::search::{SearchConfig, decode};

/// A reference to the config a recipe is exported for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecipeRef {
    /// An atlas live cell by its behaviour-axis coordinate `[i, j, k]`.
    Cell([usize; 3]),
    /// A research config key, as the research bins' selectors name it.
    Config(ConfigSource, usize),
}

impl std::str::FromStr for RecipeRef {
    type Err = String;

    fn from_str(raw: &str) -> Result<Self, String> {
        let raw = raw.trim();
        if let Some(coords) = raw.strip_prefix("cell:") {
            let coords: Vec<usize> = coords
                .split(',')
                .map(|c| c.trim().parse())
                .collect::<Result<_, _>>()
                .map_err(|_| format!("{raw:?}: a cell is cell:I,J,K"))?;
            let cell = coords
                .try_into()
                .map_err(|_| format!("{raw:?}: a cell is cell:I,J,K"))?;
            return Ok(RecipeRef::Cell(cell));
        }
        let (source, index) = parse_config_key(raw).map_err(|e| format!("{raw:?}: {e}"))?;
        Ok(RecipeRef::Config(source, index))
    }
}

impl RecipeRef {
    /// Whether the reference names an atlas live cell, so the atlas file
    /// must be read to export it.
    pub fn names_atlas_cell(&self) -> bool {
        matches!(
            self,
            RecipeRef::Cell(_) | RecipeRef::Config(ConfigSource::Atlas, _)
        )
    }
}

impl std::fmt::Display for RecipeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecipeRef::Cell([i, j, k]) => write!(f, "cell:{i},{j},{k}"),
            RecipeRef::Config(source, index) => write!(f, "{source}:{index}"),
        }
    }
}

/// The recipe `reference` names, at `max_ticks` — by default the horizon the
/// world was judged at: the one `atlas` records for its cells, else the
/// search's `T = 2000`. A reference that names no config (a cell the atlas did
/// not fill, an index past its list, an atlas reference with no atlas) is an
/// error naming the reference.
pub fn export_recipe(
    reference: &RecipeRef,
    atlas: Option<&Atlas>,
    max_ticks: Option<u64>,
) -> Result<WorldRecipe, String> {
    let search_horizon = SearchConfig::default().max_ticks;
    let fail = |why: String| format!("{reference}: {why}");
    let atlas = || atlas.ok_or_else(|| fail("an atlas reference needs an atlas".into()));
    let cell =
        match *reference {
            RecipeRef::Cell(cell) => cell,
            RecipeRef::Config(ConfigSource::Atlas, index) => {
                let atlas = atlas()?;
                atlas.cells.get(index).map(|c| c.cell).ok_or_else(|| {
                    fail(format!("the atlas has {} live cells", atlas.cells.len()))
                })?
            }
            RecipeRef::Config(ConfigSource::Sample(seed), index) => {
                let full = sample_box();
                let units = sample_draw(seed);
                let unit = units
                    .get(index)
                    .ok_or_else(|| fail(format!("a sample draw has {} configs", units.len())))?;
                let (parameters, initial) = decode(unit, &full);
                return Ok(WorldRecipe {
                    parameters,
                    initial_distribution: Some(initial),
                    agents: None,
                    carcasses: None,
                    max_ticks: max_ticks.unwrap_or(search_horizon),
                });
            }
        };
    let atlas = atlas()?;
    let horizon = atlas.provenance.map_or(search_horizon, |p| p.max_ticks);
    atlas
        .recipe_for_cell(cell, &atlas.search_box(), max_ticks.unwrap_or(horizon))
        .ok_or_else(|| fail("no live cell at that coordinate in the atlas".into()))
}

/// Write `recipe` to `path` as pretty JSON — the file the app's `--recipe`
/// loads.
pub fn write_recipe(recipe: &WorldRecipe, path: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(recipe).expect("a recipe serialises");
    std::fs::write(path, json).map_err(|e| format!("write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_source::{parse_selector, resolve_config, sampled_units};
    use crate::qd::{AtlasCell, AtlasProvenance};
    use crate::search::narrowed_ranges;
    use crate::sweep::AtlasUnits;
    use explorers_sim::World;

    fn cell(cell: [usize; 3], x: f64) -> AtlasCell {
        AtlasCell {
            cell,
            fitness: 0.5,
            oscillation: 0.0,
            clustering: 0.0,
            carcass: 0.0,
            decomposer_fraction: 0.0,
            consumer_fraction: 0.0,
            coexistence_fraction: 1.0,
            heterotroph_shares: Vec::new(),
            sample_count: 8,
            predicted_oscillation_distance: 0.0,
            predicted_branching_distance: 0.0,
            unit: vec![x; narrowed_ranges().len()],
        }
    }

    /// A hand-built atlas of two live cells drawn under the narrowed box.
    fn atlas() -> Atlas {
        Atlas {
            provenance: None,
            search_box: Some(narrowed_ranges()),
            cells: vec![cell([0, 1, 2], 0.25), cell([3, 4, 5], 0.75)],
            dead_frontier: Default::default(),
            dead_frontier_apriori: Default::default(),
            rollouts_skipped: 0,
            rollouts_unfinished: 0,
            prefilter_disagreements: Vec::new(),
            bifurcation_disagreements: Vec::new(),
            early_stop_crosschecks: 0,
            early_stop_disagreements: Vec::new(),
            coverage: 2,
            total_cells: 1000,
            qd_score: 1.0,
            best_fitness: 0.5,
        }
    }

    /// `atlas:N` is the N-th live cell of `atlas.json`'s `cells` list,
    /// decoded over the atlas's own box — the world the research bins run
    /// for `atlas:N`, and the recipe `recipe_for_cell` gives for its cell.
    #[test]
    fn an_atlas_index_exports_that_cell_over_the_atlas_box() {
        let atlas = atlas();
        let recipe = export_recipe(&"atlas:1".parse().unwrap(), Some(&atlas), None).unwrap();

        let units = AtlasUnits::new(
            narrowed_ranges(),
            atlas.cells.iter().map(|c| c.unit.clone()).collect(),
        );
        let (parameters, initial) = resolve_config(ConfigSource::Atlas, 1, &units, &[]);
        assert_eq!(recipe.parameters, parameters);
        assert_eq!(recipe.initial_distribution, Some(initial));
        assert_eq!(
            Some(recipe),
            atlas.recipe_for_cell([3, 4, 5], &narrowed_ranges(), 2000)
        );
    }

    /// `sample@S:i` exports the world `permanence_crosscheck` runs for the
    /// same reference: its selector parse, resolved as that bin resolves it.
    #[test]
    fn a_sample_reference_exports_the_world_permanence_crosscheck_runs() {
        let reference = "sample@9421:12";
        let recipe = export_recipe(&reference.parse().unwrap(), None, None).unwrap();

        let keys = parse_selector(reference, "--configs", None);
        let &(source, index) = keys.iter().next().unwrap();
        let sampled = sampled_units();
        let (parameters, initial) = resolve_config(source, index, &AtlasUnits::default(), &sampled);
        assert_eq!(recipe.parameters, parameters);
        assert_eq!(recipe.initial_distribution, Some(initial));
    }

    /// `cell:I,J,K` names a live cell by its behaviour-axis coordinate — the
    /// same world as its index in the `cells` list.
    #[test]
    fn a_cell_coordinate_exports_the_same_world_as_its_index() {
        let atlas = atlas();
        let export = |r: &str| export_recipe(&r.parse().unwrap(), Some(&atlas), None).unwrap();
        assert_eq!(export("cell:3,4,5"), export("atlas:1"));
        assert_eq!(export("cell:0,1,2"), export("atlas:0"));
        assert_ne!(export("cell:0,1,2"), export("atlas:1"));
    }

    /// A reference that names no config fails with a message naming it —
    /// never a panic: a cell the atlas did not fill, an index past its live
    /// cells or past a sample draw, and an atlas reference with no atlas.
    #[test]
    fn a_reference_to_no_config_fails_naming_the_reference() {
        let atlas = atlas();
        for reference in ["cell:9,9,9", "atlas:2", "sample:200", "sample@9421:500"] {
            let err = export_recipe(&reference.parse().unwrap(), Some(&atlas), None).unwrap_err();
            assert!(err.contains(reference), "{reference}: {err}");
        }
        let err = export_recipe(&"atlas:0".parse().unwrap(), None, None).unwrap_err();
        assert!(err.contains("atlas:0"), "{err}");
    }

    /// `max_ticks` is the horizon the world was judged at unless overridden:
    /// the one an atlas records for its cells, the search's `T = 2000` for an
    /// atlas that records none and for the sample draws.
    #[test]
    fn max_ticks_defaults_to_the_horizon_the_world_was_judged_at() {
        let ticks = |r: &str, atlas: &Atlas, over| {
            export_recipe(&r.parse().unwrap(), Some(atlas), over)
                .unwrap()
                .max_ticks
        };
        let legacy = atlas();
        let recorded = Atlas {
            provenance: Some(AtlasProvenance {
                seed: 7,
                max_ticks: 1500,
                bloom_stop: None,
            }),
            ..atlas()
        };
        assert_eq!(ticks("atlas:0", &legacy, None), 2000);
        assert_eq!(ticks("atlas:0", &recorded, None), 1500);
        assert_eq!(ticks("cell:0,1,2", &recorded, None), 1500);
        assert_eq!(ticks("sample:3", &recorded, None), 2000);
        assert_eq!(ticks("atlas:0", &recorded, Some(600)), 600);
        assert_eq!(ticks("sample:3", &recorded, Some(600)), 600);
    }

    /// An exported recipe, written to disk, reads back as the `WorldRecipe`
    /// the app's `--recipe` loads, and seeds the same world every time.
    #[test]
    fn an_exported_recipe_round_trips_to_a_deterministic_world() {
        let recipe = export_recipe(&"sample@9421:12".parse().unwrap(), None, None).unwrap();
        let path = std::env::temp_dir().join(format!(
            "explorers-recipe-export-{}.json",
            std::process::id()
        ));
        write_recipe(&recipe, &path).unwrap();
        let back: WorldRecipe =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(back, recipe);

        let run = || {
            let mut world = World::from_recipe(&back, 7);
            for _ in 0..20 {
                world.step();
            }
            format!("{:?}", world.agents())
        };
        let first = run();
        assert!(first.len() > 2, "the world has agents");
        assert_eq!(first, run());
    }
}
