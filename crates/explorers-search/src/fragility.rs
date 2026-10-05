//! The fragility audit (#693): how often an atlas world's outcome flips under
//! small perturbations — of the seed, and of its unit vector in the atlas's
//! own search box.

use explorers_genesis::{
    BloomStop, EvalConfig, RolloutBudget, RunConfig, Unfinished, run_single_within,
};
use std::collections::BTreeMap;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;

use crate::config_source::ConfigSource;
use crate::prefilter::prefilter_cliff;
use crate::qd::QdConfig;
use crate::role_diet::failure_label;
use crate::search::{SearchConfig, decode};
use crate::sweep::{AtlasUnits, EVAL_TIMEOUT_MODE, TIMEOUT_MODE, is_unfinished};

/// The verdict label of a live world (no failure mode).
pub const LIVE: &str = "live";

/// One seed's evaluation of one (possibly perturbed) world.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SeedVerdict {
    pub seed: u64,
    /// [`LIVE`], the evaluator's failure mode (`failure_label`), or an
    /// unfinished mode (`timeout`, `eval_timeout`).
    pub verdict: String,
    /// The evaluator's fitness; absent on an unfinished rollout.
    #[serde(default)]
    pub fitness: Option<f32>,
}

/// How every rollout of the audit runs: the horizon, the evaluator and the
/// per-rollout wall-clock budget. The default is the search's own (#693):
/// `QdConfig::default().eval_config()` (the evaluator with the predictive
/// bloom stop at `DEFAULT_BLOOM_STOP`), `SearchConfig::default().max_ticks`
/// and `SEARCH_ROLLOUT_BUDGET`, so the verdicts are the ones genesis scores.
#[derive(Clone, Debug)]
pub struct Rollouts {
    pub horizon: u64,
    pub eval_config: EvalConfig,
    pub budget: RolloutBudget,
}

impl Default for Rollouts {
    fn default() -> Self {
        let search = SearchConfig::default();
        Rollouts {
            horizon: search.max_ticks,
            eval_config: EvalConfig {
                bloom_stop: search.bloom_stop,
                ..QdConfig::default().eval_config()
            },
            budget: search.rollout_budget,
        }
    }
}

/// Evaluate a world on one seed as the search does: `run_single` under
/// `rollouts`. The early-stop carry-to-horizon cross-check is off: it never
/// changes a seed's verdict, only records a second reading beside it.
pub fn evaluate_seed(
    params: &explorers_genesis::WorldParameters,
    dist: &explorers_genesis::InitialDistribution,
    seed: u64,
    rollouts: &Rollouts,
) -> SeedVerdict {
    let config = RunConfig {
        max_ticks: rollouts.horizon,
        eval_config: rollouts.eval_config.clone(),
        early_stop_crosscheck_fraction: 0.0,
    };
    let budget = rollouts.budget;
    match run_single_within(params, dist, &config, seed, budget) {
        Ok(result) => SeedVerdict {
            seed,
            verdict: result
                .failure
                .as_ref()
                .map_or(LIVE, failure_label)
                .to_string(),
            fitness: Some(result.fitness),
        },
        Err(Unfinished::Simulation) => SeedVerdict {
            seed,
            verdict: TIMEOUT_MODE.to_string(),
            fitness: None,
        },
        Err(Unfinished::Evaluation) => SeedVerdict {
            seed,
            verdict: EVAL_TIMEOUT_MODE.to_string(),
            fitness: None,
        },
    }
}

/// Which perturbation of which cell a row evaluates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowKey {
    pub config_index: usize,
    pub radius: f64,
    pub draw: usize,
}

/// A SplitMix64 step: mixes one word into a running state.
fn mix(state: u64, word: u64) -> u64 {
    let mut z = (state ^ word).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The cell's unit vector perturbed by Gaussian noise of standard deviation
/// `key.radius` per dimension, clamped to `[0, 1]`. Deterministic in
/// `(jitter_seed, key)`: each (cell, radius, draw) has its own stream, so a
/// row is reproducible alone, whatever else a run evaluates. Radius 0 is the
/// unit vector itself, bit for bit.
pub fn jitter(unit: &[f64], key: RowKey, jitter_seed: u64) -> Vec<f64> {
    if key.radius == 0.0 {
        return unit.to_vec();
    }
    let stream = [
        key.config_index as u64,
        key.radius.to_bits(),
        key.draw as u64,
    ]
    .into_iter()
    .fold(mix(0, jitter_seed), mix);
    let mut rng = ChaCha8Rng::seed_from_u64(stream);
    unit.iter()
        .map(|&x| {
            // Box–Muller on (0, 1] × [0, 1).
            let u1 = 1.0 - rng.random::<f64>();
            let u2 = rng.random::<f64>();
            let z = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
            (x + key.radius * z).clamp(0.0, 1.0)
        })
        .collect()
}

/// One row: one cell × radius × draw, evaluated on every seed. Radius 0
/// (draw 0) is the unperturbed cell, the baseline every other row of the
/// cell is read against.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FragilityRow {
    /// Always `atlas`: the audit perturbs atlas cells.
    pub source: ConfigSource,
    pub config_index: usize,
    /// The atlas the row ran on (`AtlasUnits::fingerprint`).
    #[serde(default)]
    pub atlas_fingerprint: Option<u64>,
    pub radius: f64,
    pub draw: usize,
    pub horizon: u64,
    /// The predictive bloom stop the rollouts ran under (`None`: off).
    #[serde(default)]
    pub bloom_stop: Option<BloomStop>,
    /// The perturbed unit vector minus the cell's, after clamping.
    pub jitter: Vec<f64>,
    /// The search's a-priori prefilter cliff on this world
    /// (`prefilter::prefilter_cliff`), if it gates it. The search would not
    /// roll a gated world out but record it dead on that cliff; the audit
    /// rolls it out anyway and keeps both readings.
    #[serde(default)]
    pub prefilter_cliff: Option<String>,
    pub seeds: Vec<SeedVerdict>,
    /// The live share of the finished seeds; `None` when none finished.
    #[serde(default)]
    pub persisted_fraction: Option<f64>,
    /// The commonest verdict over the finished seeds (ties to the
    /// lexicographically first); `None` when none finished.
    #[serde(default)]
    pub modal_verdict: Option<String>,
}

/// The finished seeds' verdicts, unfinished ones (either budget spent)
/// dropped.
fn finished(seeds: &[SeedVerdict]) -> impl Iterator<Item = &SeedVerdict> {
    seeds.iter().filter(|s| !is_unfinished(&s.verdict))
}

/// The commonest finished verdict, ties to the lexicographically first.
pub fn modal_verdict(seeds: &[SeedVerdict]) -> Option<String> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for s in finished(seeds) {
        *counts.entry(&s.verdict).or_default() += 1;
    }
    // BTreeMap iterates in label order; `max_by_key` keeps the last maximum,
    // so iterate in reverse to keep the first.
    counts
        .into_iter()
        .rev()
        .max_by_key(|&(_, n)| n)
        .map(|(v, _)| v.to_string())
}

/// The live share of the finished seeds.
pub fn persisted_fraction(seeds: &[SeedVerdict]) -> Option<f64> {
    let n = finished(seeds).count();
    (n > 0).then(|| finished(seeds).filter(|s| s.verdict == LIVE).count() as f64 / n as f64)
}

impl FragilityRow {
    pub fn new(
        key: RowKey,
        horizon: u64,
        jitter: Vec<f64>,
        seeds: Vec<SeedVerdict>,
        atlas_fingerprint: u64,
    ) -> Self {
        FragilityRow {
            source: ConfigSource::Atlas,
            config_index: key.config_index,
            atlas_fingerprint: Some(atlas_fingerprint),
            radius: key.radius,
            draw: key.draw,
            horizon,
            bloom_stop: None,
            jitter,
            prefilter_cliff: None,
            persisted_fraction: persisted_fraction(&seeds),
            modal_verdict: modal_verdict(&seeds),
            seeds,
        }
    }

    pub fn key(&self) -> RowKey {
        RowKey {
            config_index: self.config_index,
            radius: self.radius,
            draw: self.draw,
        }
    }
}

/// The perturbed unit vector a key names, and its displacement from the cell.
fn perturbed(atlas: &AtlasUnits, key: RowKey, jitter_seed: u64) -> (Vec<f64>, Vec<f64>) {
    let unit = &atlas.units()[key.config_index];
    let moved = jitter(unit, key, jitter_seed);
    let delta = moved.iter().zip(unit).map(|(p, u)| p - u).collect();
    (moved, delta)
}

/// Evaluate one row: the cell's unit vector perturbed by the key's draw at
/// its radius, decoded over the atlas's box, on each of `seeds`.
pub fn evaluate_row(
    atlas: &AtlasUnits,
    key: RowKey,
    seeds: &[u64],
    rollouts: &Rollouts,
    jitter_seed: u64,
) -> FragilityRow {
    let (moved, delta) = perturbed(atlas, key, jitter_seed);
    let (params, dist) = decode(&moved, atlas.search_box());
    let seeds = seeds
        .iter()
        .map(|&s| evaluate_seed(&params, &dist, s, rollouts))
        .collect();
    FragilityRow {
        prefilter_cliff: prefilter_label(&params),
        bloom_stop: rollouts.eval_config.bloom_stop,
        ..FragilityRow::new(key, rollouts.horizon, delta, seeds, atlas.fingerprint())
    }
}

/// The search's prefilter cliff on a world, as its label.
fn prefilter_label(params: &explorers_genesis::WorldParameters) -> Option<String> {
    prefilter_cliff(params).map(|c| c.label().to_string())
}

/// [`evaluate_row`] for each of `keys`, every (row, seed) rollout in
/// parallel. Rayon's indexed collect keeps order, so the rows are exactly
/// the sequential ones, in key order.
pub fn evaluate_rows(
    atlas: &AtlasUnits,
    keys: &[RowKey],
    seeds: &[u64],
    rollouts: &Rollouts,
    jitter_seed: u64,
) -> Vec<FragilityRow> {
    let worlds: Vec<_> = keys
        .iter()
        .map(|&key| {
            let (moved, delta) = perturbed(atlas, key, jitter_seed);
            (key, delta, decode(&moved, atlas.search_box()))
        })
        .collect();
    let verdicts: Vec<SeedVerdict> = (0..worlds.len() * seeds.len())
        .into_par_iter()
        .map(|i| {
            let (_, _, (params, dist)) = &worlds[i / seeds.len()];
            evaluate_seed(params, dist, seeds[i % seeds.len()], rollouts)
        })
        .collect();
    let fingerprint = atlas.fingerprint();
    worlds
        .into_iter()
        .zip(verdicts.chunks(seeds.len().max(1)))
        .map(|((key, delta, (params, _)), v)| FragilityRow {
            prefilter_cliff: prefilter_label(&params),
            bloom_stop: rollouts.eval_config.bloom_stop,
            ..FragilityRow::new(key, rollouts.horizon, delta, v.to_vec(), fingerprint)
        })
        .collect()
}

/// The flip rate below which a radius is inside a cell's basin.
pub const BASIN_THRESHOLD: f64 = 0.2;

/// One cell at one radius, over every draw and seed.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct RadiusFragility {
    pub radius: f64,
    pub draws: usize,
    /// (draw, seed) evaluations finished both here and at the baseline on
    /// the same seed: the flip rate's denominator.
    pub evaluations: usize,
    /// Those whose verdict differs from the baseline's on the same seed.
    pub flips: usize,
    pub flip_rate: Option<f64>,
    /// The share of finished (draw, seed) evaluations whose verdict differs
    /// from the cell's modal verdict: the like-for-like reading against the
    /// seed-only flip rate.
    pub flip_rate_vs_modal: Option<f64>,
    /// Population standard deviation, min and max of the finished fitness.
    pub fitness_sd: Option<f64>,
    pub fitness_min: Option<f64>,
    pub fitness_max: Option<f64>,
    /// Evaluations that exhausted a budget, unread.
    pub unfinished: usize,
}

/// One cell's fragility: its seed noise floor and its flip rate by radius.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct CellFragility {
    pub config_index: usize,
    /// The unperturbed cell's modal verdict, over its finished seeds.
    pub modal_verdict: Option<String>,
    pub persisted_fraction: Option<f64>,
    /// The share of the unperturbed cell's finished seeds whose verdict
    /// differs from its modal verdict: the noise floor.
    pub seed_flip_rate: Option<f64>,
    pub baseline_fitness_sd: Option<f64>,
    pub by_radius: Vec<RadiusFragility>,
    /// The largest radius whose flip rate, and every smaller radius's, is
    /// below [`BASIN_THRESHOLD`]; 0 when the smallest is not; `None` when
    /// the smallest is unread.
    pub basin_width: Option<f64>,
}

/// Mean, population standard deviation, min and max.
fn spread(xs: &[f64]) -> (Option<f64>, Option<f64>, Option<f64>) {
    if xs.is_empty() {
        return (None, None, None);
    }
    let n = xs.len() as f64;
    let mean = xs.iter().sum::<f64>() / n;
    let sd = (xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n).sqrt();
    let min = xs.iter().copied().fold(f64::INFINITY, f64::min);
    let max = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (Some(sd), Some(min), Some(max))
}

fn ratio(num: usize, den: usize) -> Option<f64> {
    (den > 0).then(|| num as f64 / den as f64)
}

/// The basin width of `(radius, flip rate)` pairs in ascending radius: see
/// [`CellFragility::basin_width`]. The walk stops at an unread radius.
pub fn basin_width(rates: &[(f64, Option<f64>)]) -> Option<f64> {
    let mut basin = None;
    for &(radius, rate) in rates {
        match rate {
            None => break,
            Some(r) if r >= BASIN_THRESHOLD => return Some(basin.unwrap_or(0.0)),
            Some(_) => basin = Some(radius),
        }
    }
    basin
}

/// Whether a row is a cell's unperturbed baseline.
fn is_baseline(row: &FragilityRow) -> bool {
    row.radius == 0.0
}

/// Each cell with a baseline row, by index, read at each of `radii`.
pub fn cell_fragility(rows: &[FragilityRow], radii: &[f64]) -> Vec<CellFragility> {
    let mut by_cell: BTreeMap<usize, Vec<&FragilityRow>> = BTreeMap::new();
    for r in rows {
        by_cell.entry(r.config_index).or_default().push(r);
    }
    by_cell
        .into_iter()
        .filter_map(|(config_index, rows)| {
            let base = rows.iter().find(|r| is_baseline(r))?;
            let modal = modal_verdict(&base.seeds);
            let base_on: BTreeMap<u64, &str> = finished(&base.seeds)
                .map(|s| (s.seed, s.verdict.as_str()))
                .collect();
            let base_fitness: Vec<f64> = finished(&base.seeds)
                .filter_map(|s| s.fitness.map(f64::from))
                .collect();
            let n_base = finished(&base.seeds).count();
            let seed_flip_rate = modal.as_ref().and_then(|m| {
                ratio(
                    finished(&base.seeds).filter(|s| &s.verdict != m).count(),
                    n_base,
                )
            });
            let by_radius: Vec<RadiusFragility> = radii
                .iter()
                .map(|&radius| {
                    let at: Vec<&&FragilityRow> =
                        rows.iter().filter(|r| r.radius == radius).collect();
                    let all: Vec<&SeedVerdict> = at.iter().flat_map(|r| &r.seeds).collect();
                    let done: Vec<&SeedVerdict> = all
                        .iter()
                        .copied()
                        .filter(|s| !is_unfinished(&s.verdict))
                        .collect();
                    let paired: Vec<bool> = done
                        .iter()
                        .filter_map(|s| base_on.get(&s.seed).map(|b| *b != s.verdict))
                        .collect();
                    let fitness: Vec<f64> = done
                        .iter()
                        .filter_map(|s| s.fitness.map(f64::from))
                        .collect();
                    let (fitness_sd, fitness_min, fitness_max) = spread(&fitness);
                    let flips = paired.iter().filter(|&&f| f).count();
                    RadiusFragility {
                        radius,
                        draws: at.len(),
                        evaluations: paired.len(),
                        flips,
                        flip_rate: ratio(flips, paired.len()),
                        flip_rate_vs_modal: modal.as_ref().and_then(|m| {
                            ratio(done.iter().filter(|s| &s.verdict != m).count(), done.len())
                        }),
                        fitness_sd,
                        fitness_min,
                        fitness_max,
                        unfinished: all.len() - done.len(),
                    }
                })
                .collect();
            let basin = basin_width(
                &by_radius
                    .iter()
                    .map(|r| (r.radius, r.flip_rate))
                    .collect::<Vec<_>>(),
            );
            Some(CellFragility {
                config_index,
                persisted_fraction: persisted_fraction(&base.seeds),
                modal_verdict: modal,
                seed_flip_rate,
                baseline_fitness_sd: spread(&base_fitness).0,
                by_radius,
                basin_width: basin,
            })
        })
        .collect()
}

/// Midranks of `v` (ties share the mean of their ranks).
fn midranks(v: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..v.len()).collect();
    order.sort_by(|&a, &b| v[a].total_cmp(&v[b]));
    let mut ranks = vec![0.0; v.len()];
    let mut i = 0;
    while i < order.len() {
        let mut j = i + 1;
        while j < order.len() && v[order[j]] == v[order[i]] {
            j += 1;
        }
        let mid = (i + 1 + j) as f64 / 2.0;
        for &k in &order[i..j] {
            ranks[k] = mid;
        }
        i = j;
    }
    ranks
}

/// Spearman's ρ of `(x, y)` pairs: Pearson's correlation of the midranks.
/// `None` below three pairs or when either side is constant.
pub fn spearman(pairs: &[(f64, f64)]) -> Option<f64> {
    if pairs.len() < 3 {
        return None;
    }
    let x = midranks(&pairs.iter().map(|p| p.0).collect::<Vec<_>>());
    let y = midranks(&pairs.iter().map(|p| p.1).collect::<Vec<_>>());
    let mean = (pairs.len() as f64 + 1.0) / 2.0;
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (a, b) in x.iter().zip(&y) {
        let (dx, dy) = (a - mean, b - mean);
        sxy += dx * dy;
        sxx += dx * dx;
        syy += dy * dy;
    }
    (sxx > 0.0 && syy > 0.0).then(|| sxy / (sxx * syy).sqrt())
}

/// How strongly one parameter's displacement predicts a flip.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct DimensionAttribution {
    pub index: usize,
    pub name: String,
    /// Spearman's ρ of |jitter| on this dimension against the draw's flip
    /// share, taken within each radius and averaged weighted by the draws
    /// that defined it. Positive: moving this parameter flips outcomes.
    pub rho: Option<f64>,
    /// Draws behind `rho`.
    pub draws: usize,
}

/// Per draw at radius > 0 with a paired finished seed: its flip share
/// against the cell's baseline on the same seeds.
fn draw_flip_shares(rows: &[FragilityRow]) -> Vec<(&FragilityRow, f64)> {
    let mut base: BTreeMap<usize, BTreeMap<u64, &str>> = BTreeMap::new();
    for r in rows.iter().filter(|r| is_baseline(r)) {
        base.insert(
            r.config_index,
            finished(&r.seeds)
                .map(|s| (s.seed, s.verdict.as_str()))
                .collect(),
        );
    }
    rows.iter()
        .filter(|r| !is_baseline(r))
        .filter_map(|r| {
            let b = base.get(&r.config_index)?;
            let paired: Vec<bool> = finished(&r.seeds)
                .filter_map(|s| b.get(&s.seed).map(|v| *v != s.verdict))
                .collect();
            ratio(paired.iter().filter(|&&f| f).count(), paired.len()).map(|share| (r, share))
        })
        .collect()
}

/// Every dimension's attribution, strongest first (undefined last). Each
/// radius is its own stratum, |jitter| read in units of it: a pooled
/// correlation would mostly read that larger radii move every parameter
/// further and flip more.
pub fn dimension_attribution(rows: &[FragilityRow], names: &[String]) -> Vec<DimensionAttribution> {
    let shares = draw_flip_shares(rows);
    let mut radii: Vec<f64> = shares.iter().map(|(r, _)| r.radius).collect();
    radii.sort_by(f64::total_cmp);
    radii.dedup();
    let mut out: Vec<DimensionAttribution> = names
        .iter()
        .enumerate()
        .map(|(d, name)| {
            let (mut weighted, mut draws) = (0.0, 0);
            for &radius in &radii {
                let pairs: Vec<(f64, f64)> = shares
                    .iter()
                    .filter(|(r, _)| r.radius == radius)
                    .filter_map(|(r, share)| r.jitter.get(d).map(|j| (j.abs() / radius, *share)))
                    .collect();
                if let Some(rho) = spearman(&pairs) {
                    weighted += rho * pairs.len() as f64;
                    draws += pairs.len();
                }
            }
            DimensionAttribution {
                index: d,
                name: name.clone(),
                rho: (draws > 0).then(|| weighted / draws as f64),
                draws,
            }
        })
        .collect();
    out.sort_by(|a, b| match (a.rho, b.rho) {
        (Some(x), Some(y)) => y.total_cmp(&x),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.index.cmp(&b.index),
    });
    out
}

/// Nearest-rank quantiles and the mean of a distribution.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Quantiles {
    pub n: usize,
    pub min: f64,
    pub p25: f64,
    pub median: f64,
    pub p75: f64,
    pub max: f64,
    pub mean: f64,
}

impl Quantiles {
    pub fn of(xs: &[f64]) -> Option<Quantiles> {
        if xs.is_empty() {
            return None;
        }
        let mut v = xs.to_vec();
        v.sort_by(f64::total_cmp);
        let at = |p: f64| v[((p * v.len() as f64).ceil() as usize).clamp(1, v.len()) - 1];
        Some(Quantiles {
            n: v.len(),
            min: v[0],
            p25: at(0.25),
            median: at(0.5),
            p75: at(0.75),
            max: v[v.len() - 1],
            mean: v.iter().sum::<f64>() / v.len() as f64,
        })
    }
}

/// What the atlas records of a cell beside its unit vector: its
/// behaviour-axis cell `(oscillation, clustering, carcass)` and fitness.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct CellMeta {
    pub cell: [usize; 3],
    pub fitness: f64,
}

/// The atlas's cells' [`CellMeta`], in file order.
pub fn read_cell_meta(path: &std::path::Path) -> Vec<CellMeta> {
    #[derive(serde::Deserialize)]
    struct File {
        cells: Vec<CellMeta>,
    }
    let contents =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let file: File =
        serde_json::from_str(&contents).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    file.cells
}

/// The behaviour axes, in `CellMeta::cell` order.
pub const AXES: [&str; 3] = ["oscillation", "clustering", "carcass"];

/// The atlas-wide distribution of the cells' flip rates at one radius.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct RadiusDistribution {
    pub radius: f64,
    pub flip_rate: Option<Quantiles>,
    pub flip_rate_vs_modal: Option<Quantiles>,
    pub fitness_sd: Option<Quantiles>,
}

/// Whether fragility is everywhere or regional, at one radius: Spearman's ρ
/// over cells of the flip rate against each behaviour-axis coordinate and
/// against the cell's atlas fitness.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Regional {
    pub radius: f64,
    pub vs_axis: [Option<f64>; 3],
    pub vs_fitness: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Summary {
    pub radii: Vec<f64>,
    pub horizons: Vec<u64>,
    pub atlas_fingerprints: Vec<u64>,
    /// The distinct bloom stops the rows ran under, as `TICK:FACTOR` or
    /// `off`.
    pub bloom_stops: Vec<String>,
    pub cells: Vec<CellFragility>,
    /// The cells' seed-only flip rates: the noise floor.
    pub seed_flip_rate: Option<Quantiles>,
    pub baseline_fitness_sd: Option<Quantiles>,
    pub flip_rate_by_radius: Vec<RadiusDistribution>,
    /// Cells per basin width (`unread` when the smallest radius is), in
    /// ascending width.
    pub basin_widths: Vec<(String, usize)>,
    pub attribution: Vec<DimensionAttribution>,
    /// The seed-only flip rate's regional reading, then each radius's.
    pub regional_seed: Regional,
    pub regional: Vec<Regional>,
    pub prefilter: PrefilterTally,
    /// Unfinished evaluations by mode, over every row.
    pub unfinished: Vec<(String, usize)>,
}

/// The rows whose world the search's a-priori prefilter gates: the search
/// would have recorded them dead on the cliff without a rollout.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct PrefilterTally {
    pub gated_rows: usize,
    pub gated_finished_seeds: usize,
    /// Finished seeds of gated rows the rollout found live: prefilter
    /// disagreements.
    pub gated_live_seeds: usize,
    pub by_cliff: Vec<(String, usize)>,
}

fn prefilter_tally(rows: &[FragilityRow]) -> PrefilterTally {
    let gated: Vec<&FragilityRow> = rows
        .iter()
        .filter(|r| r.prefilter_cliff.is_some())
        .collect();
    let mut by_cliff: BTreeMap<String, usize> = BTreeMap::new();
    for r in &gated {
        *by_cliff
            .entry(r.prefilter_cliff.clone().unwrap_or_default())
            .or_default() += 1;
    }
    PrefilterTally {
        gated_rows: gated.len(),
        gated_finished_seeds: gated.iter().map(|r| finished(&r.seeds).count()).sum(),
        gated_live_seeds: gated
            .iter()
            .map(|r| finished(&r.seeds).filter(|s| s.verdict == LIVE).count())
            .sum(),
        by_cliff: by_cliff.into_iter().collect(),
    }
}

fn regional(radius: f64, rates: &[(usize, Option<f64>)], meta: &[CellMeta]) -> Regional {
    let pairs = |x: &dyn Fn(&CellMeta) -> f64| -> Vec<(f64, f64)> {
        rates
            .iter()
            .filter_map(|&(i, rate)| Some((x(meta.get(i)?), rate?)))
            .collect()
    };
    Regional {
        radius,
        vs_axis: std::array::from_fn(|a| spearman(&pairs(&|m| m.cell[a] as f64))),
        vs_fitness: spearman(&pairs(&|m| m.fitness)),
    }
}

/// The audit's summary over `rows`, read at `radii` (> 0), with the box's
/// parameter `names` and the atlas's `meta` by cell index.
pub fn summarise(
    rows: &[FragilityRow],
    radii: &[f64],
    names: &[String],
    meta: &[CellMeta],
) -> Summary {
    let cells = cell_fragility(rows, radii);
    let mut horizons: Vec<u64> = rows.iter().map(|r| r.horizon).collect();
    horizons.sort_unstable();
    horizons.dedup();
    let mut atlas_fingerprints: Vec<u64> =
        rows.iter().filter_map(|r| r.atlas_fingerprint).collect();
    atlas_fingerprints.sort_unstable();
    atlas_fingerprints.dedup();
    let of = |f: &dyn Fn(&CellFragility) -> Option<f64>| -> Option<Quantiles> {
        Quantiles::of(&cells.iter().filter_map(f).collect::<Vec<_>>())
    };
    let flip_rate_by_radius = (0..radii.len())
        .map(|k| RadiusDistribution {
            radius: radii[k],
            flip_rate: of(&|c| c.by_radius[k].flip_rate),
            flip_rate_vs_modal: of(&|c| c.by_radius[k].flip_rate_vs_modal),
            fitness_sd: of(&|c| c.by_radius[k].fitness_sd),
        })
        .collect();
    let mut basins: BTreeMap<Option<u64>, usize> = BTreeMap::new();
    for c in &cells {
        // Basin widths are non-negative, so their bits order as they do.
        *basins.entry(c.basin_width.map(f64::to_bits)).or_default() += 1;
    }
    let basin_widths = basins
        .into_iter()
        .map(|(w, n)| {
            (
                w.map_or("unread".to_string(), |b| format!("{}", f64::from_bits(b))),
                n,
            )
        })
        .collect();
    let regional_seed = regional(
        0.0,
        &cells
            .iter()
            .map(|c| (c.config_index, c.seed_flip_rate))
            .collect::<Vec<_>>(),
        meta,
    );
    let regional_by_radius = (0..radii.len())
        .map(|k| {
            regional(
                radii[k],
                &cells
                    .iter()
                    .map(|c| (c.config_index, c.by_radius[k].flip_rate))
                    .collect::<Vec<_>>(),
                meta,
            )
        })
        .collect();
    let mut unfinished: BTreeMap<String, usize> = BTreeMap::new();
    for s in rows.iter().flat_map(|r| &r.seeds) {
        if is_unfinished(&s.verdict) {
            *unfinished.entry(s.verdict.clone()).or_default() += 1;
        }
    }
    let mut bloom_stops: Vec<String> = rows
        .iter()
        .map(|r| {
            r.bloom_stop
                .map_or("off".to_string(), |b| format!("{}:{}", b.tick, b.factor))
        })
        .collect();
    bloom_stops.sort();
    bloom_stops.dedup();
    Summary {
        bloom_stops,
        radii: radii.to_vec(),
        horizons,
        atlas_fingerprints,
        seed_flip_rate: of(&|c| c.seed_flip_rate),
        baseline_fitness_sd: of(&|c| c.baseline_fitness_sd),
        flip_rate_by_radius,
        basin_widths,
        attribution: dimension_attribution(rows, names),
        regional_seed,
        regional: regional_by_radius,
        prefilter: prefilter_tally(rows),
        unfinished: unfinished.into_iter().collect(),
        cells,
    }
}

/// The rows each of `cells` still needs, in order — the baseline (radius
/// 0), then `draws` draws at each of `radii` — minus those in `done`. Cells
/// with nothing left are dropped.
pub fn plan_rows(
    cells: &[usize],
    radii: &[f64],
    draws: usize,
    done: &[FragilityRow],
) -> Vec<(usize, Vec<RowKey>)> {
    let written: std::collections::HashSet<(usize, u64, usize)> = done
        .iter()
        .map(|r| (r.config_index, r.radius.to_bits(), r.draw))
        .collect();
    cells
        .iter()
        .map(|&config_index| {
            let baseline = std::iter::once(RowKey {
                config_index,
                radius: 0.0,
                draw: 0,
            });
            let jittered = radii.iter().flat_map(move |&radius| {
                (0..draws).map(move |draw| RowKey {
                    config_index,
                    radius,
                    draw,
                })
            });
            let keys = baseline
                .chain(jittered)
                .filter(|k| !written.contains(&(k.config_index, k.radius.to_bits(), k.draw)))
                .collect::<Vec<_>>();
            (config_index, keys)
        })
        .filter(|(_, keys)| !keys.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sweep::read_atlas_units;

    /// The search's rollouts at a short horizon, unbudgeted.
    fn quick(horizon: u64) -> Rollouts {
        Rollouts {
            horizon,
            budget: RolloutBudget::UNBOUNDED,
            ..Rollouts::default()
        }
    }

    fn atlas() -> AtlasUnits {
        read_atlas_units(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../atlas.json"
        )))
    }

    fn jittered_key(config_index: usize, radius: f64, draw: usize) -> RowKey {
        RowKey {
            config_index,
            radius,
            draw,
        }
    }

    /// Jitter is a pure function of (jitter seed, cell, radius, draw), stays
    /// in the unit box, is zero at radius 0, and moves the cell at radius > 0.
    #[test]
    fn jitter_is_deterministic_for_a_seed_and_stays_in_the_unit_box() {
        let unit: Vec<f64> = (0..34).map(|i| i as f64 / 33.0).collect();
        let a = jitter(&unit, jittered_key(3, 0.1, 2), 7);
        assert_eq!(a, jitter(&unit, jittered_key(3, 0.1, 2), 7));
        assert_ne!(a, jitter(&unit, jittered_key(3, 0.1, 2), 8));
        assert_ne!(a, jitter(&unit, jittered_key(3, 0.1, 3), 7));
        assert_ne!(a, jitter(&unit, jittered_key(4, 0.1, 2), 7));
        assert_eq!(a.len(), unit.len());
        assert!(a.iter().all(|x| (0.0..=1.0).contains(x)));
        assert!(a.iter().zip(&unit).any(|(p, u)| p != u));
        // The ends of the box clamp: a large radius pins many coordinates
        // to 0 or 1, never past them.
        let wide = jitter(&unit, jittered_key(3, 5.0, 0), 7);
        assert!(wide.iter().all(|x| (0.0..=1.0).contains(x)));
        assert!(wide.iter().any(|&x| x == 0.0 || x == 1.0));
        // Radius 0 is the cell itself, bit for bit.
        let zero = jitter(&unit, jittered_key(3, 0.0, 0), 7);
        assert_eq!(
            zero.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            unit.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
        );
    }

    /// Per-dimension noise is Gaussian with standard deviation the radius:
    /// at a small radius from the box's centre, nothing clamps, and the
    /// sample standard deviation over many draws is near the radius.
    #[test]
    fn jitter_noise_has_the_radius_as_its_standard_deviation() {
        let unit = vec![0.5; 34];
        let deltas: Vec<f64> = (0..200)
            .flat_map(|d| {
                jitter(&unit, jittered_key(0, 0.01, d), 1)
                    .into_iter()
                    .map(|x| x - 0.5)
            })
            .collect();
        let n = deltas.len() as f64;
        let mean = deltas.iter().sum::<f64>() / n;
        let sd = (deltas.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / n).sqrt();
        assert!(mean.abs() < 0.001, "mean {mean}");
        assert!((sd - 0.01).abs() < 0.0005, "sd {sd}");
    }

    /// A jittered row records its displacement and evaluates the perturbed
    /// world, not the cell's.
    #[test]
    fn a_jittered_row_records_its_displacement() {
        let atlas = atlas();
        let key = jittered_key(0, 0.1, 1);
        let row = evaluate_row(&atlas, key, &[1000], &quick(5), 7);
        let moved = jitter(&atlas.units()[0], key, 7);
        let expected: Vec<f64> = moved
            .iter()
            .zip(&atlas.units()[0])
            .map(|(p, u)| p - u)
            .collect();
        assert_eq!(row.jitter, expected);
        assert_eq!(row.bloom_stop, Some(crate::qd::DEFAULT_BLOOM_STOP));
        assert!(row.jitter.iter().any(|&d| d != 0.0));
    }

    pub(crate) fn sv(seed: u64, verdict: &str, fitness: f32) -> SeedVerdict {
        SeedVerdict {
            seed,
            verdict: verdict.to_string(),
            fitness: (!crate::sweep::is_unfinished(verdict)).then_some(fitness),
        }
    }

    /// The persisted fraction is the live share of the finished seeds; the
    /// modal verdict is the commonest finished verdict, ties to the
    /// lexicographically first; unfinished seeds count toward neither.
    #[test]
    fn a_row_reads_its_persisted_fraction_and_modal_verdict_off_finished_seeds() {
        let seeds = vec![
            sv(0, LIVE, 0.4),
            sv(1, "monoculture", 0.0),
            sv(2, LIVE, 0.3),
            sv(3, "timeout", 0.0),
            sv(4, "monoculture", 0.0),
            sv(5, "extinction", 0.0),
        ];
        let row = FragilityRow::new(jittered_key(2, 0.03, 1), 300, vec![0.0; 3], seeds, 9);
        assert_eq!(row.persisted_fraction, Some(2.0 / 5.0));
        // live 2, monoculture 2: the tie goes to "live" < "monoculture".
        assert_eq!(row.modal_verdict.as_deref(), Some(LIVE));
        assert_eq!(row.atlas_fingerprint, Some(9));
        let none = FragilityRow::new(
            jittered_key(2, 0.03, 1),
            300,
            vec![],
            vec![sv(0, "eval_timeout", 0.0)],
            9,
        );
        assert_eq!(none.persisted_fraction, None);
        assert_eq!(none.modal_verdict, None);
    }

    /// A row as an earlier schema wrote it — no fingerprint, no derived
    /// fields, no fitness on a seed — still parses.
    #[test]
    fn old_rows_parse() {
        let line = r#"{"source":"atlas","config_index":4,"radius":0.01,"draw":2,"horizon":2000,"jitter":[0.0,0.001],"seeds":[{"seed":1000,"verdict":"live"}]}"#;
        let row: FragilityRow = serde_json::from_str(line).unwrap();
        assert_eq!(row.config_index, 4);
        assert_eq!(row.atlas_fingerprint, None);
        assert_eq!(row.seeds[0].fitness, None);
        assert_eq!(row.modal_verdict, None);
        // And a row written now round-trips.
        let row = FragilityRow::new(
            jittered_key(1, 0.1, 0),
            300,
            vec![0.5],
            vec![sv(0, LIVE, 0.2)],
            3,
        );
        let back: FragilityRow =
            serde_json::from_str(&serde_json::to_string(&row).unwrap()).unwrap();
        assert_eq!(back, row);
    }

    /// A cell's rows evaluated together (in parallel) are each the row
    /// evaluated alone, in key order.
    #[test]
    fn a_cell_evaluated_at_once_is_its_rows_evaluated_alone() {
        let atlas = atlas();
        let keys = [
            jittered_key(1, 0.0, 0),
            jittered_key(1, 0.03, 0),
            jittered_key(1, 0.03, 1),
        ];
        let seeds = [1000, 1001];
        let together = evaluate_rows(&atlas, &keys, &seeds, &quick(5), 7);
        let alone: Vec<FragilityRow> = keys
            .iter()
            .map(|&k| evaluate_row(&atlas, k, &seeds, &quick(5), 7))
            .collect();
        assert_eq!(together, alone);
        assert_eq!(together[2].atlas_fingerprint, Some(atlas.fingerprint()));
    }

    fn row(config_index: usize, radius: f64, draw: usize, verdicts: &[&str]) -> FragilityRow {
        let seeds = verdicts
            .iter()
            .enumerate()
            .map(|(i, v)| {
                sv(
                    1000 + i as u64,
                    v,
                    if *v == LIVE {
                        0.1 * (i + 1) as f32
                    } else {
                        0.0
                    },
                )
            })
            .collect();
        FragilityRow::new(
            jittered_key(config_index, radius, draw),
            300,
            vec![0.0; 2],
            seeds,
            1,
        )
    }

    /// The flip rate pairs each (draw, seed) with the unperturbed cell on
    /// the same seed; the seed-only rate reads the cell's own seeds against
    /// its modal verdict; the basin is the largest radius below 20 % with
    /// every smaller radius below it too.
    #[test]
    fn flip_rate_and_basin_width_on_a_hand_made_fixture() {
        use std::collections::BTreeMap;
        let m = "monoculture";
        let rows = vec![
            // Cell 0: live on every seed but the last.
            row(0, 0.0, 0, &[LIVE, LIVE, LIVE, LIVE, m]),
            // r = 0.01: one flip in ten (seed 1001 dies).
            row(0, 0.01, 0, &[LIVE, m, LIVE, LIVE, m]),
            row(0, 0.01, 1, &[LIVE, LIVE, LIVE, LIVE, m]),
            // r = 0.03: three flips in nine finished (a timeout is unread).
            row(0, 0.03, 0, &[m, m, LIVE, LIVE, LIVE]),
            row(0, 0.03, 1, &[LIVE, LIVE, LIVE, "timeout", m]),
            // r = 0.1: everything dies — 8 flips in 10.
            row(0, 0.1, 0, &[m, m, m, m, m]),
            row(0, 0.1, 1, &[m, m, m, m, m]),
            // Cell 1: its smallest radius already flips 50 %: basin 0.
            row(1, 0.0, 0, &[LIVE, LIVE]),
            row(1, 0.01, 0, &[m, LIVE]),
        ];
        let cells = cell_fragility(&rows, &[0.01, 0.03, 0.1]);
        assert_eq!(cells.len(), 2);
        let c0 = &cells[0];
        assert_eq!(c0.config_index, 0);
        assert_eq!(c0.modal_verdict.as_deref(), Some(LIVE));
        assert_eq!(c0.seed_flip_rate, Some(0.2));
        let rates: BTreeMap<String, (usize, usize, Option<f64>)> = c0
            .by_radius
            .iter()
            .map(|r| {
                (
                    format!("{}", r.radius),
                    (r.flips, r.evaluations, r.flip_rate),
                )
            })
            .collect();
        assert_eq!(rates["0.01"], (1, 10, Some(0.1)));
        assert_eq!(rates["0.03"], (3, 9, Some(3.0 / 9.0)));
        assert_eq!(rates["0.1"], (8, 10, Some(0.8)));
        assert_eq!(c0.by_radius[1].unfinished, 1);
        // Against the modal verdict ("live"), r = 0.01 has 3 non-live of 10.
        assert_eq!(c0.by_radius[0].flip_rate_vs_modal, Some(0.3));
        // 0.01 is below 20 %, 0.03 is not: the basin is 0.01.
        assert_eq!(c0.basin_width, Some(0.01));
        // Fitness spread at r = 0.1: all zero.
        assert_eq!(c0.by_radius[2].fitness_sd, Some(0.0));
        let c1 = &cells[1];
        assert_eq!(c1.seed_flip_rate, Some(0.0));
        assert_eq!(c1.by_radius[0].flip_rate, Some(0.5));
        assert_eq!(c1.basin_width, Some(0.0));
        // Radii with no rows are unread, and stop the basin walk no further.
        assert_eq!(c1.by_radius[1].flip_rate, None);
    }

    /// A basin walks the radii in order and stops at the first unread or
    /// fragile one: a cell robust at every radius has the largest.
    #[test]
    fn the_basin_is_the_largest_radius_with_every_smaller_one_below_the_threshold() {
        assert_eq!(
            basin_width(&[(0.01, Some(0.0)), (0.03, Some(0.1)), (0.1, Some(0.19))]),
            Some(0.1)
        );
        assert_eq!(
            basin_width(&[(0.01, Some(0.0)), (0.03, Some(0.25)), (0.1, Some(0.1))]),
            Some(0.01)
        );
        assert_eq!(
            basin_width(&[(0.01, Some(0.2)), (0.03, Some(0.0))]),
            Some(0.0)
        );
        assert_eq!(
            basin_width(&[(0.01, Some(0.0)), (0.03, None), (0.1, Some(0.0))]),
            Some(0.01)
        );
        assert_eq!(basin_width(&[(0.01, None)]), None);
    }

    #[test]
    fn spearman_reads_monotone_trends_and_is_undefined_on_constants() {
        assert_eq!(spearman(&[(1.0, 1.0), (2.0, 4.0), (3.0, 9.0)]), Some(1.0));
        assert_eq!(spearman(&[(1.0, 3.0), (2.0, 2.0), (3.0, 1.0)]), Some(-1.0));
        assert_eq!(spearman(&[(1.0, 0.0), (2.0, 0.0), (3.0, 0.0)]), None);
        assert_eq!(spearman(&[(1.0, 0.0), (2.0, 1.0)]), None);
    }

    /// Attribution names the dimension whose displacement predicts the flip:
    /// here flips follow |jitter| on dimension 0, while dimension 1 moves
    /// against them. Displacements are read in units of the radius, so the
    /// radii pool.
    #[test]
    fn attribution_names_the_dimension_whose_displacement_predicts_flipping() {
        let m = "monoculture";
        let mut rows = vec![row(0, 0.0, 0, &[LIVE, LIVE])];
        for (draw, (d0, d1, verdicts)) in [
            (0.001, 0.02, [LIVE, LIVE]),
            (0.005, 0.015, [LIVE, LIVE]),
            (0.01, 0.01, [m, LIVE]),
            (0.02, 0.001, [m, m]),
        ]
        .into_iter()
        .enumerate()
        {
            let mut r = row(0, 0.01, draw, &verdicts);
            r.jitter = vec![d0, -d1];
            rows.push(r);
            // The same pattern ten times larger at r = 0.1.
            let mut r = row(0, 0.1, draw, &verdicts);
            r.jitter = vec![-10.0 * d0, 10.0 * d1];
            rows.push(r);
        }
        let names = vec!["a".to_string(), "b".to_string()];
        let attribution = dimension_attribution(&rows, &names);
        assert_eq!(attribution[0].name, "a");
        assert_eq!(attribution[0].draws, 8);
        assert!(attribution[0].rho.unwrap() > 0.9);
        assert_eq!(attribution[1].name, "b");
        assert!(attribution[1].rho.unwrap() < -0.9);
    }

    #[test]
    fn quantiles_are_nearest_rank_with_a_mean() {
        let q = Quantiles::of(&[0.4, 0.0, 0.1, 0.2, 0.3]).unwrap();
        assert_eq!(
            (q.n, q.min, q.p25, q.median, q.p75, q.max),
            (5, 0.0, 0.1, 0.2, 0.3, 0.4)
        );
        assert!((q.mean - 0.2).abs() < 1e-12);
        assert_eq!(Quantiles::of(&[]), None);
    }

    /// The summary reads the atlas-wide distribution of flip rates by radius
    /// and of the seed-only rate, tallies basin widths, and correlates
    /// per-cell fragility with the behaviour axes and fitness.
    #[test]
    fn the_summary_reads_the_atlas_wide_distribution_and_where_fragility_lies() {
        let m = "monoculture";
        // Three cells, fragility rising with the first axis and with
        // fitness; at r = 0.01 cell 0 flips 0/2, cell 1 1/2, cell 2 2/2.
        let rows = vec![
            row(0, 0.0, 0, &[LIVE, LIVE]),
            row(0, 0.01, 0, &[LIVE, LIVE]),
            row(1, 0.0, 0, &[LIVE, LIVE]),
            row(1, 0.01, 0, &[m, LIVE]),
            row(2, 0.0, 0, &[LIVE, m]),
            row(2, 0.01, 0, &[m, LIVE]),
        ];
        let meta = vec![
            CellMeta {
                cell: [0, 5, 9],
                fitness: 0.1,
            },
            CellMeta {
                cell: [3, 5, 1],
                fitness: 0.2,
            },
            CellMeta {
                cell: [7, 5, 4],
                fitness: 0.3,
            },
        ];
        let s = summarise(&rows, &[0.01], &["a".to_string(), "b".to_string()], &meta);
        assert_eq!(s.cells.len(), 3);
        let at = &s.flip_rate_by_radius[0];
        assert_eq!(at.radius, 0.01);
        let q = at.flip_rate.as_ref().unwrap();
        assert_eq!((q.n, q.min, q.median, q.max), (3, 0.0, 0.5, 1.0));
        let seed = s.seed_flip_rate.as_ref().unwrap();
        assert_eq!((seed.n, seed.max), (3, 0.5));
        // Basins: cells 0 is robust at 0.01, cells 1 and 2 are not.
        assert_eq!(
            s.basin_widths,
            vec![("0".to_string(), 2), ("0.01".to_string(), 1)]
        );
        let regional = &s.regional[0];
        assert_eq!(regional.radius, 0.01);
        assert_eq!(regional.vs_axis[0], Some(1.0));
        assert_eq!(regional.vs_axis[1], None);
        assert_eq!(regional.vs_axis[2], Some(-0.5));
        assert_eq!(regional.vs_fitness, Some(1.0));
        assert_eq!(s.unfinished, vec![]);
    }

    /// Each cell plans its baseline then every radius's draws; rows already
    /// written are skipped, and a cell with nothing left is not planned.
    #[test]
    fn planning_skips_rows_already_written() {
        let done = vec![
            row(0, 0.0, 0, &[LIVE]),
            row(0, 0.01, 0, &[LIVE]),
            row(1, 0.0, 0, &[LIVE]),
            row(1, 0.01, 0, &[LIVE]),
            row(1, 0.01, 1, &[LIVE]),
        ];
        let plan = plan_rows(&[0, 1, 2], &[0.01], 2, &done);
        assert_eq!(
            plan,
            vec![
                (0, vec![jittered_key(0, 0.01, 1)]),
                (
                    2,
                    vec![
                        jittered_key(2, 0.0, 0),
                        jittered_key(2, 0.01, 0),
                        jittered_key(2, 0.01, 1)
                    ]
                ),
            ]
        );
    }

    /// The audit evaluates as the search does by default (#693): the
    /// search's evaluator, with its predictive bloom stop, its horizon and
    /// its per-rollout budget.
    #[test]
    fn the_default_rollouts_are_the_searchs() {
        let search = crate::search::SearchConfig::default();
        let r = Rollouts::default();
        assert_eq!(
            r.eval_config.bloom_stop,
            Some(crate::qd::DEFAULT_BLOOM_STOP)
        );
        assert_eq!(r.eval_config.bloom_stop, search.bloom_stop);
        assert_eq!(
            format!("{:?}", r.eval_config),
            format!("{:?}", crate::qd::QdConfig::default().eval_config())
        );
        assert_eq!(r.horizon, search.max_ticks);
        assert_eq!(r.budget, search.rollout_budget);
    }

    /// A row records the search's a-priori prefilter cliff on its world (the
    /// search would not have rolled a gated world out); the summary tallies
    /// gated rows and how many of their finished seeds the rollout found live.
    #[test]
    fn rows_record_the_prefilter_and_the_summary_tallies_its_gated_rows() {
        let atlas = atlas();
        let row0 = evaluate_row(&atlas, jittered_key(0, 0.1, 0), &[1000], &quick(3), 7);
        let (moved, _) = perturbed(&atlas, jittered_key(0, 0.1, 0), 7);
        let (params, _) = decode(&moved, atlas.search_box());
        assert_eq!(
            row0.prefilter_cliff.as_deref(),
            crate::prefilter::prefilter_cliff(&params).map(|c| c.label())
        );
        let mut gated = row(0, 0.1, 0, &[LIVE, "energy_death", "timeout"]);
        gated.prefilter_cliff = Some("energy_death".to_string());
        let rows = vec![
            row(0, 0.0, 0, &[LIVE, LIVE, LIVE]),
            row(0, 0.1, 1, &[LIVE, LIVE, LIVE]),
            gated,
        ];
        let s = summarise(&rows, &[0.1], &[], &[]);
        assert_eq!(s.prefilter.gated_rows, 1);
        assert_eq!(s.prefilter.gated_finished_seeds, 2);
        assert_eq!(s.prefilter.gated_live_seeds, 1);
        assert_eq!(s.prefilter.by_cliff, vec![("energy_death".to_string(), 1)]);
    }

    /// The tracer: one cell at radius 0 on one seed is the search's own
    /// rollout of the decoded cell (its evaluator, bloom stop included, past
    /// the bloom stop's tick), verdict and fitness bit for bit.
    #[test]
    fn radius_zero_reproduces_the_unperturbed_evaluation_bit_for_bit() {
        let atlas = atlas();
        let key = RowKey {
            config_index: 0,
            radius: 0.0,
            draw: 0,
        };
        let row = evaluate_row(&atlas, key, &[1000], &quick(320), 7);
        let (params, dist) = atlas.decode(0);
        let config = RunConfig {
            max_ticks: 320,
            eval_config: crate::qd::QdConfig::default().eval_config(),
            early_stop_crosscheck_fraction: 0.0,
        };
        let direct = explorers_genesis::run_single(&params, &dist, &config, 1000);
        assert_eq!(row.seeds.len(), 1);
        assert_eq!(
            row.seeds[0].verdict,
            direct.failure.as_ref().map_or(LIVE, failure_label)
        );
        assert_eq!(
            row.seeds[0].fitness.map(f32::to_bits),
            Some(direct.fitness.to_bits())
        );
        assert!(row.jitter.iter().all(|&d| d == 0.0));
    }
}
