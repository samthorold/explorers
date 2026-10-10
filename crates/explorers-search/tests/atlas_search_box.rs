//! The unit-vector hazard of #559: a cell's `unit` names a world only together
//! with the search box it was drawn under. The research bins read atlas cells
//! through `sweep::read_atlas_units` and resolve a `source:index` key with
//! `config_source::resolve_config`; these tests pin that an atlas drawn under
//! the narrowed box decodes, in a research bin, to exactly the worlds the
//! search evaluated, while `sample:i` / `sample@S:i` stay draws over the
//! instruments' own box (`sample_box`, the size-blind box since #653). The
//! committed atlas decodes, bit for bit, to the worlds its search evaluated,
//! and the older atlases cut from it (untaxed, taxed, leached, legacy) to the
//! worlds main named before #701 and #716 changed `decode`.

use std::path::PathBuf;

use explorers_search::atlas_file::write_atlas;
use explorers_search::config_source::{
    ConfigSource, resolve_config, sample_box, sample_draw, sample_fixed, sampled_units,
};
use explorers_search::search::{
    FixedParameters, SearchConfig, decode, default_ranges, leached_ranges, narrowed_ranges,
    run_search, size_blind_ranges, taxed_ranges, untaxed_ranges,
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
        let evaluated = decode(&cell.unit, &config.ranges, &config.fixed);
        assert_eq!(
            resolve_config(ConfigSource::Atlas, i, &read, &sampled),
            evaluated,
            "atlas:{i}"
        );
        // The hazard itself: the same unit over the full box is another world.
        assert_ne!(
            decode(&cell.unit, &default_ranges(), &FixedParameters::genesis()),
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
        decode(&sampled[12], &full, &sample_fixed())
    );
    assert_eq!(
        resolve_config(ConfigSource::Sample(9421), 12, &read, &sampled),
        decode(&sample_draw(9421)[12], &full, &sample_fixed())
    );
}

const COMMITTED_ATLAS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../atlas.json");

/// The committed atlas (#687, #719's seed-42 search) was searched under
/// genesis's box from #716 until #773 and records it: the 33-coordinate
/// untaxed box, with no scales recorded, and the leaching rate held outside
/// it at `λ = 0.0025`, which it records as `fixed`. Every `atlas:i` decodes
/// over that recorded box at that rate, with `c_AH = 0`. It predates the
/// bounded box genesis searches since #773, so a reader under that box is
/// refused.
#[test]
fn the_committed_atlas_records_its_33_dimension_box_and_fixed_leaching_rate() {
    let text = std::fs::read_to_string(COMMITTED_ATLAS).unwrap();
    let read = read_atlas_units(std::path::Path::new(COMMITTED_ATLAS));
    assert_eq!(read.search_box(), untaxed_ranges().as_slice());
    assert_eq!(read.fixed(), &FixedParameters::genesis());
    assert!(read.check_search_box(&untaxed_ranges()).is_ok());
    assert!(read.check_search_box(&default_ranges()).is_err());
    assert!(read.check_search_box(&leached_ranges()).is_err());
    assert!(read.check_search_box(&taxed_ranges()).is_err());
    // #719's atlas, the seed-42 one read in `719-fixed-leaching-atlas.md`;
    // its fragility audit's rows record the same fingerprint.
    assert_eq!(format!("{:016x}", read.fingerprint()), "823db6be9c5ec9ba");
    let raw: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(!raw["search_box"].to_string().contains("scale"));
    assert_eq!(raw["fixed"], serde_json::json!({ "leaching_rate": 0.0025 }));
    for i in 0..read.len() {
        let unit: Vec<f64> = serde_json::from_value(raw["cells"][i]["unit"].clone()).unwrap();
        assert_eq!(unit.len(), 33);
        let world = read.decode(i);
        assert_eq!(
            world,
            decode(&unit, &untaxed_ranges(), &FixedParameters::genesis()),
            "atlas:{i}"
        );
        assert_eq!(world.0.cross_trait_cost, 0.0, "atlas:{i}");
        assert_eq!(world.0.leaching_rate, 0.0025, "atlas:{i}");
    }
}

/// A stable FNV-1a digest of every decoded world of an atlas, read off the
/// worlds' `Debug` form (which prints each `f32` round-trip exactly), so two
/// decodes share it exactly when they name bit-identical worlds.
///
/// Fields added to `WorldParameters` after a digest was read are asserted at
/// the latent default every atlas decodes them to and dropped from the
/// `Debug` form, so the digests stay the ones read before them: hyphal
/// uptake off at the default contact distance (#727). The wear law (#763)
/// replaced `repair_decay`, which every atlas decoded to 1.0, with the repair
/// rate and senescence hazard at their placeholder defaults; the digest reads
/// the old field back in their place.
fn decoded_worlds_digest(read: &explorers_search::sweep::AtlasUnits) -> u64 {
    const LATER_FIELDS: &str = ", hyphal_uptake: false, contact_distance: 0.1";
    const WEAR_LAW_FIELDS: &str = ", repair_rate: 1.0, senescence_hazard: 0.01";
    const REPAIR_DECAY_FIELD: &str = ", repair_decay: 1.0";
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for i in 0..read.len() {
        let world = read.decode(i);
        assert!(!world.0.hyphal_uptake, "atlas:{i}");
        assert_eq!(
            world.0.contact_distance,
            explorers_sim::DEFAULT_CONTACT_DISTANCE,
            "atlas:{i}"
        );
        assert_eq!(world.0.repair_rate, 1.0, "atlas:{i}");
        assert_eq!(world.0.senescence_hazard, 0.01, "atlas:{i}");
        let debug = format!("{world:?}");
        assert_eq!(debug.matches(LATER_FIELDS).count(), 1, "atlas:{i}");
        assert_eq!(debug.matches(WEAR_LAW_FIELDS).count(), 1, "atlas:{i}");
        let debug =
            debug
                .replacen(LATER_FIELDS, "", 1)
                .replacen(WEAR_LAW_FIELDS, REPAIR_DECAY_FIELD, 1);
        for b in debug.bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    h
}

/// The committed atlas (#687) names exactly the worlds #719's search
/// evaluated. The digest was read on this tree, whose `decode` is the one
/// the search ran (main `1ff1fdd`).
#[test]
fn the_committed_atlas_decodes_to_the_same_worlds_bit_for_bit() {
    let read = read_atlas_units(std::path::Path::new(COMMITTED_ATLAS));
    assert_eq!(
        format!("{:016x}", decoded_worlds_digest(&read)),
        "316a7b03c3fa1af1"
    );
}

/// The committed atlas with its `fixed` record dropped, as an atlas from
/// before #716 records none, written to scratch as JSON for the fixtures
/// below to cut.
fn committed_atlas_without_fixed_rate() -> serde_json::Value {
    let text = std::fs::read_to_string(COMMITTED_ATLAS).unwrap();
    let mut raw: serde_json::Value = serde_json::from_str(&text).unwrap();
    raw.as_object_mut().unwrap().remove("fixed");
    raw
}

/// An atlas searched under the untaxed box with no fixed rate (#663's,
/// committed until #677): the committed atlas, which records that box, with
/// its fixed rate dropped, written to scratch.
fn untaxed_atlas(name: &str) -> PathBuf {
    let raw = committed_atlas_without_fixed_rate();
    let path = scratch(name).join("atlas.json");
    std::fs::write(&path, raw.to_string()).unwrap();
    path
}

/// An atlas that records the untaxed box decodes over it: every `atlas:i`
/// names the world it was searched as, with its own `b` and `c_AH = 0`.
#[test]
fn an_untaxed_atlas_reads_as_its_33_dimension_box_with_c_ah_zero() {
    let path = untaxed_atlas("untaxed-box");
    let text = std::fs::read_to_string(&path).unwrap();
    let read = read_atlas_units(&path);
    assert_eq!(read.search_box(), untaxed_ranges().as_slice());
    // Read on main before #701 changed `decode`.
    assert_eq!(
        format!("{:016x}", decoded_worlds_digest(&read)),
        "db732abd6e4d010d"
    );
    let raw: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(!read.is_empty());
    for i in 0..read.len() {
        let unit: Vec<f64> = serde_json::from_value(raw["cells"][i]["unit"].clone()).unwrap();
        assert_eq!(unit.len(), 33);
        let world = read.decode(i);
        assert_eq!(
            world,
            decode(&unit, &untaxed_ranges(), &FixedParameters::none()),
            "atlas:{i}"
        );
        assert_eq!(world.0.cross_trait_cost, 0.0, "atlas:{i}");
        assert_eq!(world.0.leaching_rate, 0.0, "atlas:{i}");
    }
}

/// A legacy atlas (before #559, as the committed atlas was until #663): the
/// committed atlas with its box, its fixed rate, its coordinates past the
/// 32nd and its heterotroph shares (#602) stripped, written to scratch.
fn legacy_atlas(name: &str) -> PathBuf {
    let mut raw = committed_atlas_without_fixed_rate();
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
    // Read on main before #701 changed `decode`.
    assert_eq!(
        format!("{:016x}", decoded_worlds_digest(&read)),
        "28ebd538d2879ee9"
    );
    let raw: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(!read.is_empty());
    for i in 0..read.len() {
        let unit: Vec<f64> = serde_json::from_value(raw["cells"][i]["unit"].clone()).unwrap();
        assert_eq!(unit.len(), 32);
        let world = read.decode(i);
        assert_eq!(
            world,
            decode(&unit, &size_blind_ranges(), &FixedParameters::none())
        );
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

/// #716: a search under genesis's box records the leaching rate its worlds
/// ran at beside the box, and every reader decodes its cells at that rate:
/// the atlas, the research bins' `atlas:i` and the exported recipe alike.
#[test]
fn an_atlas_records_the_fixed_leaching_rate_and_decodes_at_it() {
    let config = SearchConfig {
        ranges: default_ranges(),
        ..tiny_search()
    };
    assert_eq!(config.fixed, FixedParameters::genesis());
    let atlas = run_search(&config, 7, &mut ChaCha8Rng::seed_from_u64(7));
    assert!(
        !atlas.cells.is_empty(),
        "the tiny search yields a live cell"
    );
    let path = scratch("fixed-rate").join("atlas.json");
    write_atlas(&atlas, &path).unwrap();

    let raw: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(raw["fixed"], serde_json::json!({ "leaching_rate": 0.0025 }));
    let back = explorers_search::atlas_file::read_atlas(&path).unwrap();
    assert_eq!(back.fixed, FixedParameters::genesis());

    let read = read_atlas_units(&path);
    assert_eq!(read.fixed(), &FixedParameters::genesis());
    let sampled = sampled_units();
    for (i, cell) in atlas.cells.iter().enumerate() {
        let world = resolve_config(ConfigSource::Atlas, i, &read, &sampled);
        assert_eq!(world, decode(&cell.unit, &config.ranges, &config.fixed));
        assert_eq!(world.0.leaching_rate, 0.0025, "atlas:{i}");
        let recipe = back
            .recipe_for_cell(cell.cell, &back.search_box(), 20)
            .unwrap();
        assert_eq!(recipe.parameters.leaching_rate, 0.0025, "atlas:{i}");
    }
}

/// #716: an atlas that records neither a `λ` coordinate nor a fixed rate
/// writes no `fixed` record and decodes at the stepper's `λ = 0`.
#[test]
fn an_atlas_without_a_fixed_rate_writes_none_and_decodes_at_zero() {
    let config = SearchConfig {
        fixed: FixedParameters::none(),
        ..tiny_search()
    };
    let atlas = run_search(&config, 7, &mut ChaCha8Rng::seed_from_u64(7));
    let path = scratch("no-fixed-rate").join("atlas.json");
    write_atlas(&atlas, &path).unwrap();
    let raw: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert!(raw.get("fixed").is_none());
    let read = read_atlas_units(&path);
    assert!(!read.is_empty());
    for i in 0..read.len() {
        assert_eq!(read.decode(i).0.leaching_rate, 0.0, "atlas:{i}");
    }
}

/// An atlas searched with `λ` in the box (#686's, #711's): the committed
/// atlas, whose box is the untaxed 33 coordinates, with `λ` appended on its
/// square scale, each cell's 34th coordinate a spread of `u`, and its fixed
/// rate dropped, as those atlases record none; written to scratch.
fn leached_atlas(name: &str) -> PathBuf {
    let mut raw = committed_atlas_without_fixed_rate();
    raw["search_box"] = serde_json::to_value(leached_ranges()).unwrap();
    for (i, cell) in raw["cells"].as_array_mut().unwrap().iter_mut().enumerate() {
        let unit = cell["unit"].as_array_mut().unwrap();
        assert_eq!(unit.len(), 33);
        unit.push(serde_json::json!((i as f64 * 0.137) % 1.0));
    }
    let path = scratch(name).join("atlas.json");
    std::fs::write(&path, raw.to_string()).unwrap();
    path
}

/// #716: an atlas searched with `λ` as a box coordinate (#686's, #711's)
/// decodes every cell at its own coordinate's `λ = 0.01 · u²`, not at
/// genesis's fixed rate, and names the worlds it named before #716.
#[test]
fn a_leached_atlas_decodes_with_its_own_coordinate_lambda() {
    let path = leached_atlas("leached-box");
    let read = read_atlas_units(&path);
    assert_eq!(read.search_box(), leached_ranges().as_slice());
    assert!(read.fixed().is_empty());
    assert!(read.check_search_box(&default_ranges()).is_err());
    // Read on main before #716.
    assert_eq!(
        format!("{:016x}", decoded_worlds_digest(&read)),
        "4fc466933a23bd54"
    );
    let mut leaching = 0;
    for i in 0..read.len() {
        let u = read.units()[i][33];
        let world = read.decode(i);
        assert_eq!(world.0.leaching_rate, (0.01 * u * u) as f32, "atlas:{i}");
        assert_eq!(world.0.cross_trait_cost, 0.0, "atlas:{i}");
        leaching += usize::from(world.0.leaching_rate != 0.0025);
    }
    assert!(leaching > 0, "the cells keep their own rates");
}

/// An atlas searched with `c_AH` in the box (#677's, committed until #687):
/// the committed atlas with `c_AH` appended as its 34th coordinate, each
/// cell's a spread of `u`, and its fixed rate dropped, as #677's records
/// none; written to scratch.
fn taxed_atlas(name: &str) -> PathBuf {
    let mut raw = committed_atlas_without_fixed_rate();
    raw["search_box"] = serde_json::to_value(taxed_ranges()).unwrap();
    for (i, cell) in raw["cells"].as_array_mut().unwrap().iter_mut().enumerate() {
        let unit = cell["unit"].as_array_mut().unwrap();
        assert_eq!(unit.len(), 33);
        unit.push(serde_json::json!((i as f64 * 0.137) % 1.0));
    }
    let path = scratch(name).join("atlas.json");
    std::fs::write(&path, raw.to_string()).unwrap();
    path
}

/// An atlas that records the 34-coordinate taxed box (#677's) is as long as
/// a leached one, so it must decode by name: every `atlas:i` keeps its own
/// `c_AH`, read linearly off the 34th coordinate, and `λ = 0`, since it
/// records no fixed rate (#716).
#[test]
fn a_taxed_atlas_decodes_its_own_c_ah_by_name() {
    let path = taxed_atlas("taxed-box");
    let read = read_atlas_units(&path);
    assert_eq!(read.search_box(), taxed_ranges().as_slice());
    assert!(read.fixed().is_empty(), "it records no fixed rate");
    assert_eq!(read.search_box().len(), leached_ranges().len());
    assert!(read.check_search_box(&leached_ranges()).is_err());
    assert!(read.check_search_box(&default_ranges()).is_err());
    // Read on main before #701 changed `decode`.
    assert_eq!(
        format!("{:016x}", decoded_worlds_digest(&read)),
        "994dcb89ca16176e"
    );
    let mut taxed = 0;
    for i in 0..read.len() {
        let unit = &read.units()[i];
        let world = read.decode(i);
        assert_eq!(
            world,
            decode(unit, &taxed_ranges(), &FixedParameters::none()),
            "atlas:{i}"
        );
        assert_eq!(
            world.0.cross_trait_cost,
            (0.0 + unit[33] * (0.14 - 0.0)) as f32,
            "atlas:{i}"
        );
        assert_eq!(world.0.leaching_rate, 0.0, "atlas:{i}");
        taxed += usize::from(world.0.cross_trait_cost > 0.0);
    }
    assert!(taxed > 0, "the cells keep their own c_AH");
}
