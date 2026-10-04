//! Research bin (#589): the heterotroph margin across the search box.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use explorers_search::config_source::{
    ConfigSource, SAMPLE_CONFIGS, parse_selector, sampled_units,
};
use explorers_search::heterotroph_margin::{HeterotrophLine, LineRead};
use explorers_search::search::default_ranges;
use explorers_search::search::{ParameterRange, decode};
use explorers_search::sweep::plan_tasks;
use explorers_sim::TraitVector;
use serde_json::Value;

/// The box axes a row records beside its unit vector: the ones the margin
/// reads, and #462's radius.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct Axes {
    base_trophic_efficiency: f32,
    trophic_distance_decay: f32,
    heterotrophy_maintenance_cost: f32,
    base_metabolic_rate: f32,
    maintenance_cost_exponent: f32,
    /// `mean_photosynthetic_absorption + mean_heterotrophy`: the line's length
    /// is `√2 ×` this on a multi-cluster founding.
    trophic_budget: f32,
    light_competition_radius: f32,
    world_extent: f32,
}

/// One config's margin row.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct MarginRow {
    source: ConfigSource,
    config_index: usize,
    unit: Vec<f64>,
    axes: Axes,
    producer: TraitVector,
    line: LineRead,
    /// The atlas cell's own recorded guild fractions (atlas rows only).
    atlas_cell: Option<AtlasCellOutcome>,
    /// The #509 sweep's outcome for this config, when a sweep is joined.
    sweep: Option<SweepOutcome>,
}

/// An atlas cell's recorded seed fractions (`consumer_fraction` is absent
/// from atlases written before #490's consumer read).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct AtlasCellOutcome {
    decomposer_fraction: f64,
    consumer_fraction: Option<f64>,
    coexistence_fraction: f64,
}

/// The margin row for the config `unit` names over `search_box`.
fn margin_row(
    source: ConfigSource,
    config_index: usize,
    unit: &[f64],
    search_box: &[ParameterRange],
) -> MarginRow {
    let (params, dist) = decode(unit, search_box);
    let line = HeterotrophLine::from_founders(&dist);
    let mean = dist.mean_traits;
    MarginRow {
        source,
        config_index,
        unit: unit.to_vec(),
        axes: Axes {
            base_trophic_efficiency: params.base_trophic_efficiency,
            trophic_distance_decay: params.trophic_distance_decay,
            heterotrophy_maintenance_cost: params.heterotrophy_maintenance_cost,
            base_metabolic_rate: params.base_metabolic_rate,
            maintenance_cost_exponent: params.maintenance_cost_exponent,
            trophic_budget: mean.photosynthetic_absorption + mean.heterotrophy,
            light_competition_radius: params.light_competition_radius,
            world_extent: params.world_extent,
        },
        producer: line.producer,
        line: line.read(&params),
        atlas_cell: None,
        sweep: None,
    }
}

/// Attach the sweep's outcomes to `rows` by `(source, config_index)`. The
/// sweep records each config's founder producer centroid; a row whose own
/// differs names a different world (an atlas regenerated since the sweep
/// ran), and the join refuses it.
fn join_sweep(rows: &mut [MarginRow], sweep: &[Value]) -> Result<usize, String> {
    let mut joined = 0;
    for rec in sweep {
        let source: ConfigSource = serde_json::from_value(rec["source"].clone())
            .map_err(|e| format!("sweep row source: {e}"))?;
        let index = rec["config_index"]
            .as_u64()
            .ok_or("sweep row has no config_index")? as usize;
        let Some(row) = rows
            .iter_mut()
            .find(|r| r.source == source && r.config_index == index)
        else {
            continue;
        };
        let producer: TraitVector = serde_json::from_value(rec["producer_centroid"].clone())
            .map_err(|e| format!("{source}:{index} producer_centroid: {e}"))?;
        if producer != row.producer {
            return Err(format!(
                "{source}:{index}: the sweep's founder producer {producer:?} is not this \
                 config's {:?}; the sweep ran on a different world under this key \
                 (pass the atlas the sweep ran on)",
                row.producer
            ));
        }
        row.sweep = Some(sweep_outcome(rec));
        joined += 1;
    }
    Ok(joined)
}

/// An atlas file read loosely enough to take every generation of it: the
/// cells' unit vectors, their recorded fractions and the box they decode over
/// (a legacy atlas, with no box recorded: the full box).
struct LooseAtlas {
    search_box: Vec<ParameterRange>,
    units: Vec<Vec<f64>>,
    outcomes: Vec<AtlasCellOutcome>,
}

fn read_loose_atlas(path: &Path) -> LooseAtlas {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let atlas: Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    let search_box = match atlas.get("search_box") {
        Some(b) if !b.is_null() => serde_json::from_value(b.clone()).expect("a search box"),
        _ => explorers_search::search::size_blind_ranges(),
    };
    let cells = atlas["cells"].as_array().expect("an atlas has cells");
    let units = cells
        .iter()
        .map(|c| serde_json::from_value(c["unit"].clone()).expect("a cell has a unit"))
        .collect();
    let outcomes = cells
        .iter()
        .map(|c| AtlasCellOutcome {
            decomposer_fraction: c["decomposer_fraction"]
                .as_f64()
                .expect("decomposer_fraction"),
            consumer_fraction: c["consumer_fraction"].as_f64(),
            coexistence_fraction: c["coexistence_fraction"]
                .as_f64()
                .expect("coexistence_fraction"),
        })
        .collect();
    LooseAtlas {
        search_box,
        units,
        outcomes,
    }
}

/// Every config's row in the shared sweep order (atlas cells, then the
/// seed-421 sample), restricted to `filter` when one is given.
fn all_rows(atlas_path: &Path, filter: Option<&HashSet<(ConfigSource, usize)>>) -> Vec<MarginRow> {
    let atlas = read_loose_atlas(atlas_path);
    let full = explorers_search::config_source::sample_box();
    let sampled = sampled_units();
    plan_tasks(
        atlas.units.len(),
        SAMPLE_CONFIGS,
        filter,
        &HashSet::new(),
        None,
    )
    .into_iter()
    .map(|(source, index)| match source {
        ConfigSource::Atlas => MarginRow {
            atlas_cell: Some(atlas.outcomes[index]),
            ..margin_row(source, index, &atlas.units[index], &atlas.search_box)
        },
        ConfigSource::SAMPLE => margin_row(source, index, &sampled[index], &full),
        ConfigSource::Sample(seed) => margin_row(
            source,
            index,
            &explorers_search::config_source::sample_draw(seed)[index],
            &full,
        ),
    })
    .collect()
}

/// The best margin over a two-axis grid of the full box, every other axis
/// at its median. `margins[j][i]` is at `y_values[j]`, `x_values[i]`.
struct Slice {
    x_axis: String,
    y_axis: String,
    x_values: Vec<f64>,
    y_values: Vec<f64>,
    margins: Vec<Vec<f32>>,
}

fn slice(x_axis: &str, y_axis: &str, n: usize) -> Slice {
    let box_ = default_ranges();
    let at = |name: &str| {
        box_.iter()
            .position(|r| r.name == name)
            .unwrap_or_else(|| panic!("{name} is not a search-box axis"))
    };
    let (xi, yi) = (at(x_axis), at(y_axis));
    let steps: Vec<f64> = (0..n).map(|i| i as f64 / (n - 1) as f64).collect();
    let value = |axis: usize, u: f64| box_[axis].min + u * (box_[axis].max - box_[axis].min);
    let margins = steps
        .iter()
        .map(|&v| {
            steps
                .iter()
                .map(|&u| {
                    let mut unit = vec![0.5; box_.len()];
                    unit[xi] = u;
                    unit[yi] = v;
                    margin_row(ConfigSource::SAMPLE, 0, &unit, &box_)
                        .line
                        .best
                        .margin
                })
                .collect()
        })
        .collect();
    Slice {
        x_axis: x_axis.to_string(),
        y_axis: y_axis.to_string(),
        x_values: steps.iter().map(|&u| value(xi, u)).collect(),
        y_values: steps.iter().map(|&v| value(yi, v)).collect(),
        margins,
    }
}

fn print_slice(s: &Slice) {
    println!(
        "\n{} (rows) × {} (columns), best margin E/tick:",
        s.y_axis, s.x_axis
    );
    let header: Vec<String> = s.x_values.iter().map(|x| format!("{x:.3}")).collect();
    println!("| | {} |", header.join(" | "));
    println!("|---|{}", "---:|".repeat(s.x_values.len()));
    for (y, row) in s.y_values.iter().zip(&s.margins) {
        let cells: Vec<String> = row.iter().map(|m| format!("{m:+.3}")).collect();
        println!("| {y:.3} | {} |", cells.join(" | "));
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Args {
    atlas: PathBuf,
    sweep: Option<PathBuf>,
    out: PathBuf,
    configs: Option<HashSet<(ConfigSource, usize)>>,
    /// `--slice X,Y`: print the two-axis slice instead of the rows.
    slice: Option<(String, String)>,
}

/// Grid points per axis in a printed slice.
const SLICE_POINTS: usize = 9;

fn parse_args<I: IntoIterator<Item = String>>(argv: I) -> Result<Args, String> {
    let mut args = Args {
        atlas: PathBuf::from("atlas.json"),
        sweep: None,
        out: PathBuf::from("target/heterotroph-margin.jsonl"),
        configs: None,
        slice: None,
    };
    let mut it = argv.into_iter();
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--atlas" => args.atlas = PathBuf::from(value()?),
            "--sweep" => args.sweep = Some(PathBuf::from(value()?)),
            "--out" => args.out = PathBuf::from(value()?),
            "--configs" => args.configs = Some(parse_selector(&value()?, "--configs", None)),
            "--slice" => {
                let raw = value()?;
                let (x, y) = raw.split_once(',').ok_or("--slice takes X,Y")?;
                args.slice = Some((x.to_string(), y.to_string()));
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    Ok(args)
}

/// A config's recorded outcome in the #509 sweep (`permanence_crosscheck`'s
/// JSON-lines rows): seed fractions over its finished seeds, as #462 read
/// them. `None` fractions when no seed finished.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct SweepOutcome {
    finished: usize,
    live: Option<f64>,
    lockup: Option<f64>,
    /// Live with a consumer at `T`.
    consumers: Option<f64>,
    /// A decomposer guild read at `T`.
    guild: Option<f64>,
}

fn sweep_outcome(row: &Value) -> SweepOutcome {
    let finished: Vec<&Value> = row["seeds"]
        .as_array()
        .expect("a sweep row has seeds")
        .iter()
        .filter(|s| !matches!(s["mode"].as_str(), Some("timeout" | "eval_timeout")))
        .collect();
    let n = finished.len();
    let fraction = |pred: &dyn Fn(&Value) -> bool| {
        (n > 0).then(|| finished.iter().filter(|s| pred(s)).count() as f64 / n as f64)
    };
    let live = |s: &Value| s["mode"] == "none";
    SweepOutcome {
        finished: n,
        live: fraction(&live),
        lockup: fraction(&|s| s["mode"] == "nutrient-lockup"),
        consumers: fraction(&|s| live(s) && s["terminal_consumers"].as_u64().unwrap_or(0) > 0),
        guild: fraction(&|s| s["decomposer_guild"] == true),
    }
}

fn main() {
    let args =
        parse_args(std::env::args().skip(1)).unwrap_or_else(|e| panic!("heterotroph_margin: {e}"));
    if let Some((x, y)) = &args.slice {
        print_slice(&slice(x, y, SLICE_POINTS));
        return;
    }
    let mut rows = all_rows(&args.atlas, args.configs.as_ref());
    if let Some(path) = &args.sweep {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let sweep: Vec<Value> = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).expect("a sweep row"))
            .collect();
        let joined =
            join_sweep(&mut rows, &sweep).unwrap_or_else(|e| panic!("heterotroph_margin: {e}"));
        eprintln!(
            "heterotroph_margin: joined {joined} of {} sweep rows",
            sweep.len()
        );
    }
    let out: String = rows
        .iter()
        .map(|r| serde_json::to_string(r).expect("a row serialises") + "\n")
        .collect();
    std::fs::write(&args.out, out).unwrap_or_else(|e| panic!("write {}: {e}", args.out.display()));
    eprintln!(
        "heterotroph_margin: {} rows written to {}",
        rows.len(),
        args.out.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed(mode: &str, consumers: u64, guild: bool) -> Value {
        serde_json::json!({
            "mode": mode, "terminal_consumers": consumers, "decomposer_guild": guild
        })
    }

    fn sample_row(index: usize) -> MarginRow {
        let units = explorers_search::config_source::sampled_units();
        margin_row(
            ConfigSource::SAMPLE,
            index,
            &units[index],
            &explorers_search::config_source::sample_box(),
        )
    }

    fn sweep_row(row: &MarginRow, producer: TraitVector) -> Value {
        serde_json::json!({
            "source": row.source, "config_index": row.config_index,
            "producer_centroid": producer,
            "seeds": [seed("nutrient-lockup", 0, false)],
        })
    }

    /// The join attaches the sweep's outcome to the row it names, and refuses
    /// a sweep row whose founder producer is not the row's: that sweep ran on
    /// a different world under the same key.
    #[test]
    fn join_attaches_outcomes_and_refuses_a_different_world_under_the_same_key() {
        let mut rows = vec![sample_row(31), sample_row(12)];
        let producer = rows[0].producer;
        let sweep = [sweep_row(&rows[0], producer)];
        let joined = join_sweep(&mut rows, &sweep).unwrap();
        assert_eq!(joined, 1);
        assert_eq!(rows[0].sweep.unwrap().lockup, Some(1.0));
        assert_eq!(rows[1].sweep, None);

        let moved = TraitVector {
            kappa: producer.kappa + 0.1,
            ..producer
        };
        let mut rows = vec![sample_row(31)];
        let sweep = [sweep_row(&rows[0], moved)];
        assert!(join_sweep(&mut rows, &sweep).is_err());
    }

    fn legacy_atlas(dir: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("atlas.json");
        let unit = explorers_search::config_source::sampled_units()[7].clone();
        let atlas = serde_json::json!({ "cells": [{
            "decomposer_fraction": 0.25, "coexistence_fraction": 0.5, "unit": unit
        }]});
        std::fs::write(&path, atlas.to_string()).unwrap();
        path
    }

    /// Atlas rows come first, decoded over the atlas's own box (a legacy
    /// atlas: the full box) and carrying the cell's recorded fractions, then
    /// the seed-421 sample, restricted to the selector; the rows are the
    /// same on every run.
    #[test]
    fn rows_cover_the_atlas_then_the_sample_and_are_deterministic() {
        let atlas = legacy_atlas("hm-rows");
        let filter = parse_selector("atlas:0,sample:31", "--configs", None);
        let rows = all_rows(&atlas, Some(&filter));
        assert_eq!(rows.len(), 2);
        assert_eq!(
            (rows[0].source, rows[0].config_index),
            (ConfigSource::Atlas, 0)
        );
        assert_eq!(rows[0], sample_row_as_atlas(7));
        let cell = rows[0].atlas_cell.unwrap();
        assert_eq!(cell.decomposer_fraction, 0.25);
        assert_eq!(cell.consumer_fraction, None);
        assert_eq!(rows[1], sample_row(31));
        assert_eq!(rows, all_rows(&atlas, Some(&filter)));
    }

    fn sample_row_as_atlas(index: usize) -> MarginRow {
        MarginRow {
            source: ConfigSource::Atlas,
            config_index: 0,
            atlas_cell: Some(AtlasCellOutcome {
                decomposer_fraction: 0.25,
                consumer_fraction: None,
                coexistence_fraction: 0.5,
            }),
            ..sample_row(index)
        }
    }

    #[test]
    fn cli_defaults_to_the_atlas_and_no_sweep_and_takes_every_flag() {
        let d = parse_args(Vec::<String>::new()).unwrap();
        assert_eq!(d.atlas, std::path::PathBuf::from("atlas.json"));
        assert_eq!(d.sweep, None);
        assert_eq!(
            d.out,
            std::path::PathBuf::from("target/heterotroph-margin.jsonl")
        );
        let a = parse_args(
            "--atlas a.json --sweep s.jsonl --out o.jsonl --configs sample:31"
                .split(' ')
                .map(String::from),
        )
        .unwrap();
        assert_eq!(a.atlas, std::path::PathBuf::from("a.json"));
        assert_eq!(a.sweep, Some(std::path::PathBuf::from("s.jsonl")));
        assert_eq!(a.out, std::path::PathBuf::from("o.jsonl"));
        assert_eq!(a.configs.unwrap().len(), 1);
        let s = parse_args(["--slice", "a,b"].map(String::from)).unwrap();
        assert_eq!(s.slice, Some(("a".to_string(), "b".to_string())));
        assert!(parse_args(["--bogus".to_string()]).is_err());
    }

    /// A slice holds every other axis at the box median (unit 0.5) and moves
    /// two named axes over an `n × n` grid; each entry is that config's best
    /// margin.
    #[test]
    fn slice_is_the_best_margin_over_a_two_axis_grid_at_the_box_medians() {
        let s = slice("base_trophic_efficiency", "trophic_distance_decay", 5);
        assert_eq!(s.x_values.len(), 5);
        assert_eq!(s.margins.len(), 5);
        assert!(s.margins.iter().all(|row| row.len() == 5));
        let box_ = default_ranges();
        let mut unit = vec![0.5; box_.len()];
        unit[1] = 1.0; // base_trophic_efficiency at its max
        unit[2] = 0.0; // trophic_distance_decay at its min
        let row = margin_row(ConfigSource::SAMPLE, 0, &unit, &box_);
        assert_eq!(s.margins[0][4], row.line.best.margin);
        assert_eq!(s.x_values[4], 0.9);
        assert_eq!(s.y_values[0], 0.1);
    }

    /// Fractions are over finished seeds (a timed-out seed is dropped), as
    /// #462 read the sweep.
    #[test]
    fn sweep_outcome_reads_seed_fractions_over_finished_seeds() {
        let row = serde_json::json!({ "seeds": [
            seed("none", 2, true),
            seed("none", 0, false),
            seed("nutrient-lockup", 0, false),
            seed("timeout", 0, false),
        ]});
        let o = sweep_outcome(&row);
        assert_eq!(o.finished, 3);
        assert_eq!(o.live, Some(2.0 / 3.0));
        assert_eq!(o.lockup, Some(1.0 / 3.0));
        assert_eq!(o.consumers, Some(1.0 / 3.0));
        assert_eq!(o.guild, Some(1.0 / 3.0));
        let none = sweep_outcome(&serde_json::json!({ "seeds": [seed("timeout", 0, false)] }));
        assert_eq!(none.finished, 0);
        assert_eq!(none.lockup, None);
    }
}
