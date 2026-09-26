//! The unit-vector hazard of #559: a cell's `unit` names a world only together
//! with the search box it was drawn under. The research bins read atlas cells
//! through `sweep::read_atlas_units` and resolve a `source:index` key with
//! `config_source::resolve_config`; these tests pin that an atlas drawn under
//! the narrowed box decodes, in a research bin, to exactly the worlds the
//! search evaluated, while `sample:i` / `sample@S:i` stay draws over the full
//! box.

use std::path::PathBuf;

use explorers_search::atlas_file::write_atlas;
use explorers_search::config_source::{ConfigSource, resolve_config, sample_draw, sampled_units};
use explorers_search::search::{SearchConfig, decode, default_ranges, narrowed_ranges, run_search};
use explorers_search::sweep::read_atlas_units;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "explorers-atlas-search-box-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tiny_search() -> SearchConfig {
    SearchConfig {
        ensemble_size: 1,
        max_ticks: 20,
        batch: 4,
        generations: 1,
        ranges: narrowed_ranges(),
        ..SearchConfig::default()
    }
}

#[test]
fn a_research_bin_decodes_a_narrowed_atlas_to_the_worlds_the_search_evaluated() {
    let config = tiny_search();
    let atlas = run_search(&config, 7, &mut ChaCha8Rng::seed_from_u64(7));
    assert!(
        !atlas.cells.is_empty(),
        "the tiny search yields a live cell"
    );
    let path = scratch("round-trip").join("atlas.json");
    write_atlas(&atlas, &path).unwrap();

    let read = read_atlas_units(&path);
    let sampled = sampled_units(default_ranges().len());
    assert_eq!(read.len(), atlas.cells.len());
    for (i, cell) in atlas.cells.iter().enumerate() {
        let evaluated = decode(&cell.unit, &config.ranges);
        assert_eq!(
            resolve_config(ConfigSource::Atlas, i, &read, &sampled),
            evaluated,
            "atlas:{i}"
        );
        // The hazard itself: the same unit over the full box is another world.
        assert_ne!(
            decode(&cell.unit, &default_ranges()),
            evaluated,
            "atlas:{i}"
        );
    }
}

#[test]
fn sample_keys_stay_draws_over_the_full_box() {
    let path = scratch("samples").join("atlas.json");
    let atlas = run_search(&tiny_search(), 7, &mut ChaCha8Rng::seed_from_u64(7));
    write_atlas(&atlas, &path).unwrap();
    let read = read_atlas_units(&path);
    let full = default_ranges();
    let sampled = sampled_units(full.len());
    assert_eq!(
        resolve_config(ConfigSource::SAMPLE, 12, &read, &sampled),
        decode(&sampled[12], &full)
    );
    assert_eq!(
        resolve_config(ConfigSource::Sample(9421), 12, &read, &sampled),
        decode(&sample_draw(9421, full.len())[12], &full)
    );
}

/// The committed atlas predates #559 and records no box: it reads as the full
/// box it was searched under, so every existing `atlas:i` names the world it
/// always named.
#[test]
fn a_legacy_atlas_reads_as_the_full_box() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../atlas.json");
    let text = std::fs::read_to_string(path).unwrap();
    assert!(
        !text.contains("search_box"),
        "the committed atlas is legacy"
    );
    let read = read_atlas_units(std::path::Path::new(path));
    assert_eq!(read.search_box(), default_ranges().as_slice());
    let raw: serde_json::Value = serde_json::from_str(&text).unwrap();
    let unit: Vec<f64> = serde_json::from_value(raw["cells"][0]["unit"].clone()).unwrap();
    assert_eq!(read.decode(0), decode(&unit, &default_ranges()));
}

/// A research reader that decodes over a box of its own is refused when the
/// atlas records another.
#[test]
fn a_reader_with_another_box_is_refused() {
    let config = tiny_search();
    let path = scratch("mismatch").join("atlas.json");
    write_atlas(
        &run_search(&config, 7, &mut ChaCha8Rng::seed_from_u64(7)),
        &path,
    )
    .unwrap();
    let read = read_atlas_units(&path);
    assert!(read.check_search_box(&config.ranges).is_ok());
    let err = read.check_search_box(&default_ranges()).unwrap_err();
    assert!(err.to_string().contains("search box"), "{err}");
}
