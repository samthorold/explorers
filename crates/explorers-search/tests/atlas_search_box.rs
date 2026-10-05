//! The unit-vector hazard of #559: a cell's `unit` names a world only together
//! with the search box it was drawn under. The research bins read atlas cells
//! through `sweep::read_atlas_units` and resolve a `source:index` key with
//! `config_source::resolve_config`; these tests pin that an atlas drawn under
//! the narrowed box decodes, in a research bin, to exactly the worlds the
//! search evaluated, while `sample:i` / `sample@S:i` stay draws over the
//! instruments' own box (`sample_box`, the size-blind box since #653).

use std::path::PathBuf;

use explorers_search::atlas_file::write_atlas;
use explorers_search::config_source::{
    ConfigSource, resolve_config, sample_box, sample_draw, sampled_units,
};
use explorers_search::search::{
    SearchConfig, decode, default_ranges, narrowed_ranges, run_search, size_blind_ranges,
    untaxed_ranges,
};
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
    let sampled = sampled_units();
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
fn sample_keys_stay_draws_over_the_sample_box() {
    let path = scratch("samples").join("atlas.json");
    let atlas = run_search(&tiny_search(), 7, &mut ChaCha8Rng::seed_from_u64(7));
    write_atlas(&atlas, &path).unwrap();
    let read = read_atlas_units(&path);
    let full = sample_box();
    let sampled = sampled_units();
    assert_eq!(
        resolve_config(ConfigSource::SAMPLE, 12, &read, &sampled),
        decode(&sampled[12], &full)
    );
    assert_eq!(
        resolve_config(ConfigSource::Sample(9421), 12, &read, &sampled),
        decode(&sample_draw(9421)[12], &full)
    );
}

const COMMITTED_ATLAS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../atlas.json");

/// The committed atlas (#663) was searched with `b` in the box and records
/// it: the 33-dimension box, the untaxed box since `c_AH` joined the full box
/// (#669). So every `atlas:i` decodes with its own `b` and with `c_AH = 0`.
#[test]
fn the_committed_atlas_records_its_33_dimension_box() {
    let text = std::fs::read_to_string(COMMITTED_ATLAS).unwrap();
    let read = read_atlas_units(std::path::Path::new(COMMITTED_ATLAS));
    assert_eq!(read.search_box(), untaxed_ranges().as_slice());
    assert_eq!(read.search_box().len(), 33);
    // #663's atlas, the one #656 searched and read (`656-fresh-atlas-verdict.md`).
    assert_eq!(format!("{:016x}", read.fingerprint()), "9c79856550a0151e");
    let raw: serde_json::Value = serde_json::from_str(&text).unwrap();
    for i in 0..read.len() {
        let unit: Vec<f64> = serde_json::from_value(raw["cells"][i]["unit"].clone()).unwrap();
        assert_eq!(unit.len(), 33);
        let world = read.decode(i);
        assert_eq!(world, decode(&unit, &untaxed_ranges()), "atlas:{i}");
        assert_eq!(world.0.cross_trait_cost, 0.0, "atlas:{i}");
    }
}

/// A legacy atlas (before #559, as the committed atlas was until #663): the
/// committed atlas with its box, its 33rd coordinate and its heterotroph
/// shares (#602) stripped, written to scratch.
fn legacy_atlas(name: &str) -> PathBuf {
    let text = std::fs::read_to_string(COMMITTED_ATLAS).unwrap();
    let mut raw: serde_json::Value = serde_json::from_str(&text).unwrap();
    raw.as_object_mut().unwrap().remove("search_box");
    for cell in raw["cells"].as_array_mut().unwrap() {
        cell["unit"].as_array_mut().unwrap().truncate(32);
        cell.as_object_mut().unwrap().remove("heterotroph_shares");
    }
    let path = scratch(name).join("atlas.json");
    std::fs::write(&path, raw.to_string()).unwrap();
    path
}

/// An atlas that records no box reads as the box it was searched under — the
/// full box before #653, the size-blind box — so every `atlas:i` of a legacy
/// atlas names the world it always named, with `b = 0`.
#[test]
fn a_legacy_atlas_reads_as_the_size_blind_box() {
    let path = legacy_atlas("legacy-box");
    let text = std::fs::read_to_string(&path).unwrap();
    let read = read_atlas_units(&path);
    assert_eq!(read.search_box(), size_blind_ranges().as_slice());
    let raw: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(!read.is_empty());
    for i in 0..read.len() {
        let unit: Vec<f64> = serde_json::from_value(raw["cells"][i]["unit"].clone()).unwrap();
        assert_eq!(unit.len(), 32);
        let world = read.decode(i);
        assert_eq!(world, decode(&unit, &size_blind_ranges()));
        assert_eq!(world.0.uptake_structure_exponent, 0.0, "atlas:{i}");
        assert_eq!(world.0.cross_trait_cost, 0.0, "atlas:{i}");
    }
}

/// An atlas from before #602 records no heterotroph shares. It still reads
/// back, every cell with an empty share distribution.
#[test]
fn a_legacy_atlas_reads_back_with_no_heterotroph_shares() {
    let path = legacy_atlas("legacy-shares");
    let atlas = explorers_search::atlas_file::read_atlas(&path).unwrap();
    assert!(!atlas.cells.is_empty());
    assert!(atlas.cells.iter().all(|c| c.heterotroph_shares.is_empty()));
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
