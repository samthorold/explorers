use rand::Rng;

use explorers_genesis::{BloomStop, InitialDistribution, RolloutBudget, WorldParameters};
use explorers_sim::TraitVector;

use std::path::Path;

use rand_chacha::ChaCha8Rng;

use crate::checkpoint::{CheckpointError, resume_qd, run_qd_checkpointed};
use crate::qd::{Atlas, GenerationReport, QdConfig, SEARCH_ROLLOUT_BUDGET, run_qd_observed};

/// The genesis search output is the [`Atlas`] (CONTEXT.md) — the live archive of
/// behaviour cells plus the dead frontier. `SearchResult` is kept as the public
/// alias the consumers name; `best_recipe` / `recipe_for_cell` live on `Atlas`.
pub type SearchResult = Atlas;

/// One coordinate of the search box: the raw field it decodes to (`name`),
/// its bounds, and the scale the unit coordinate is mapped on (#701).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ParameterRange {
    pub name: String,
    pub min: f64,
    pub max: f64,
    /// How the unit coordinate `u` maps onto `[min, max]`. Part of the box:
    /// the same `u` over another scale is another world. Serialised only when
    /// it is not linear, so a range recorded without one (every atlas and
    /// checkpoint from before #701) reads as linear, and a linear box writes
    /// exactly as it did.
    #[serde(default, skip_serializing_if = "Scale::is_linear")]
    pub scale: Scale,
}

/// The scale a [`ParameterRange`] maps its unit coordinate on (#701).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scale {
    /// `min + u · (max − min)`.
    #[default]
    Linear,
    /// `min + u² · (max − min)`: holds `min` exactly, like the linear scale,
    /// and gives a quarter of the coordinate to the bottom sixteenth of the
    /// range. The leaching rate's scale (world-rules.md, *The range*).
    Square,
}

impl Scale {
    pub fn is_linear(&self) -> bool {
        *self == Scale::Linear
    }
}

impl ParameterRange {
    /// The raw value the unit coordinate `u` names over this range.
    pub fn value_at(&self, u: f64) -> f64 {
        match self.scale {
            Scale::Linear => self.min + u * (self.max - self.min),
            Scale::Square => self.min + u * u * (self.max - self.min),
        }
    }
}

/// Configuration for the genesis outer search. Post-#365 the outer loop is the
/// QD (CMA-MAE) illuminator; the knobs are the QD ones. (The LHS/Sobol/GP-BO
/// modules remain in the crate but are no longer on the production path — GP-BO
/// is earmarked for a later refinement role, brief E / #349.)
#[derive(Clone, Debug)]
pub struct SearchConfig {
    /// The search box. Defaults to the full box ([`default_ranges`]); the
    /// narrowed box ([`narrowed_ranges`], #559) is opt-in.
    pub ranges: Vec<ParameterRange>,
    /// The parameters held fixed outside the box (#716). Defaults to
    /// [`FixedParameters::genesis`].
    pub fixed: FixedParameters,
    pub ensemble_size: u32,
    pub max_ticks: u64,
    /// Solutions evaluated per generation (the batch size).
    pub batch: usize,
    /// Adaptation generations after the random bootstrap batch.
    pub generations: usize,
    /// Initial per-dimension emitter deviation (unit-cube coordinates).
    pub sigma: f64,
    /// CMA-MAE archive learning rate (soft per-cell acceptance threshold drag).
    pub archive_learning_rate: f32,
    /// Fraction of prefilter-gated (a priori dead) configs still rolled out as the
    /// agreement cross-check (viability.md). 0 disables it.
    pub prefilter_crosscheck_fraction: f32,
    /// Fraction of rollouts an incremental dead-pool gate stopped that are
    /// carried to the horizon anyway and re-verdicted, the early-stop cross-check
    /// (`early_stop_crosscheck_fraction` in [`QdConfig`], #506). 0 disables it.
    pub early_stop_crosscheck_fraction: f32,
    /// Carcass-directed bootstrap seeds (`carcass_seed_count` in [`QdConfig`]) —
    /// how many guaranteed high-carcass starting points to inject so the atlas's
    /// nutrient-lockup layer is reached by running (it cannot be prefiltered).
    pub carcass_seed_count: usize,
    /// Wall-clock budget on each seed rollout (`rollout_budget` in
    /// [`QdConfig`], #562).
    pub rollout_budget: RolloutBudget,
    /// The predictive bloom stop (`bloom_stop` in [`QdConfig`], #573):
    /// `DEFAULT_BLOOM_STOP` unless turned off.
    pub bloom_stop: Option<BloomStop>,
    /// Adaptive top-up's screen (`top_up_screen` in [`QdConfig`], #699):
    /// `None` rolls out the whole ensemble for every config.
    pub top_up_screen: Option<u32>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            ranges: default_ranges(),
            fixed: FixedParameters::genesis(),
            ensemble_size: crate::qd::SEARCH_ENSEMBLE_SIZE,
            max_ticks: 2000,
            batch: 32,
            generations: 10,
            sigma: 0.15,
            archive_learning_rate: 0.5,
            prefilter_crosscheck_fraction: 0.05,
            early_stop_crosscheck_fraction: 0.05,
            carcass_seed_count: 2,
            rollout_budget: SEARCH_ROLLOUT_BUDGET,
            bloom_stop: Some(crate::qd::DEFAULT_BLOOM_STOP),
            top_up_screen: None,
        }
    }
}

/// The search's flag for the predictive bloom stop (#573), `TICK:FACTOR`.
pub const BLOOM_STOP_FLAG: &str = "--bloom-stop";

/// The search's flag that turns the bloom stop off.
pub const NO_BLOOM_STOP_FLAG: &str = "--no-bloom-stop";

/// Parse a [`BLOOM_STOP_FLAG`] value: `300:5` stops, at tick 300, a rollout
/// whose running peak is at least 5 × founders.
pub fn parse_bloom_stop(value: &str) -> Result<BloomStop, String> {
    let err = || format!("{BLOOM_STOP_FLAG} takes TICK:FACTOR (e.g. 300:5), not {value:?}");
    let (tick, factor) = value.split_once(':').ok_or_else(err)?;
    let tick: u64 = tick.parse().map_err(|_| err())?;
    let factor: f32 = factor.parse().map_err(|_| err())?;
    if tick == 0 || !factor.is_finite() || factor < 0.0 {
        return Err(err());
    }
    Ok(BloomStop { tick, factor })
}

/// The leaching rate `λ` every world genesis searches runs at (#716), held
/// outside the box: `λ_max / 4`, a carcass half-life of about 277 ticks
/// (world-rules.md, *Carcass energy decays only through agents; carcass
/// nutrient leaches*, *The rate*). The stepper's own default stays 0.
pub const GENESIS_LEACHING_RATE: f64 = 0.0025;

/// The world parameters a search holds at a fixed value outside its box
/// (#716), by the name of the field each sets, as a [`ParameterRange`] names
/// its field. A unit vector names a world only together with its box and
/// these, so an atlas records them beside its box and a reader decodes its
/// cells at the values it records. An atlas that records none decodes with
/// every such field at the known-viable baseline's value, which for `λ` is
/// the stepper's 0.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct FixedParameters(pub std::collections::BTreeMap<String, f64>);

impl FixedParameters {
    /// No fixed parameters: what an atlas from before #716 records.
    pub fn none() -> Self {
        FixedParameters::default()
    }

    /// What genesis's searches hold fixed (#716): `λ` at
    /// [`GENESIS_LEACHING_RATE`].
    pub fn genesis() -> Self {
        FixedParameters(
            [("leaching_rate".to_string(), GENESIS_LEACHING_RATE)]
                .into_iter()
                .collect(),
        )
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The value `name` is held at, if it is held.
    pub fn get(&self, name: &str) -> Option<f64> {
        self.0.get(name).copied()
    }
}

/// The full box (#716): the 33-coordinate untaxed box ([`untaxed_ranges`]).
/// Neither the cross-trait cost `c_AH` (trade-off #5) nor the leaching rate
/// `λ` is in it: genesis selects on neither, and a dimension the search does
/// not select on only adds noise. `c_AH` keeps its default 0; `λ` is held at
/// [`GENESIS_LEACHING_RATE`] outside the box, which the search records as one
/// of its [`FixedParameters`] (world-rules.md, *Carcass energy decays only
/// through agents; carcass nutrient leaches*, *The rate*).
pub fn default_ranges() -> Vec<ParameterRange> {
    untaxed_ranges()
}

/// The box #686's and #711's atlases were searched under (#701): the untaxed
/// box ([`untaxed_ranges`]), then the leaching rate `λ` over `[0, 0.01]` on a
/// square scale, `λ = 0.01 · u²`. The range holds `λ = 0` exactly, which a log
/// scale cannot; the square gives a quarter of the coordinate to
/// `λ < 0.01 / 16`, where nutrient lockup still bites.
pub fn leached_ranges() -> Vec<ParameterRange> {
    let mut ranges = untaxed_ranges();
    ranges.push(ParameterRange {
        name: "leaching_rate".into(),
        min: 0.0,
        max: 0.01,
        scale: Scale::Square,
    });
    ranges
}

/// The box the committed atlas (#677) was searched under: the untaxed box
/// ([`untaxed_ranges`]) with the cross-trait cost `c_AH` as its 34th
/// coordinate, linear over `[0, 0.14]`, and no leaching (`λ = 0`, which
/// [`decode`] gives any box without `λ`'s coordinate when no fixed rate is
/// recorded, as the committed atlas records none). It is the box `c_AH` is
/// searched over if it comes out of reserve.
pub fn taxed_ranges() -> Vec<ParameterRange> {
    let mut ranges = untaxed_ranges();
    ranges.push(ParameterRange {
        name: "cross_trait_cost".into(),
        // Cross-trait cost `c_AH` (trade-off #5, #669), linear over
        // [0, 0.14]: the range must hold `c_AH = 0` exactly — the latent
        // default every world before #669 ran — so a log scale is out.
        // The top is measured, not copied from the per-trait costs: the
        // `c_AH` at which a typical light-fed mixotroph that drains pays
        // about twice its median drain income
        // (`docs/research/668-cross-trait-calibration.md`).
        min: 0.0,
        max: 0.14,
        scale: Scale::Linear,
    });
    ranges
}

/// The full box as it stood before the cross-trait cost joined it (#669),
/// and again since #716: 33 coordinates, with mixotrophy untaxed
/// (`c_AH = 0`). Its leaching rate is the one held fixed outside it:
/// [`GENESIS_LEACHING_RATE`] for a search since #716, and the stepper's
/// `λ = 0` for #663's atlas (committed until #677), which was searched under
/// this box and records no fixed rate.
pub fn untaxed_ranges() -> Vec<ParameterRange> {
    vec![
        ParameterRange {
            name: "solar_flux_magnitude".into(),
            min: 1.0,
            max: 20.0,
            scale: Scale::Linear,
        }, // 0
        ParameterRange {
            name: "base_trophic_efficiency".into(),
            min: 0.1,
            max: 0.9,
            scale: Scale::Linear,
        }, // 1
        ParameterRange {
            name: "trophic_distance_decay".into(),
            min: 0.1,
            max: 5.0,
            scale: Scale::Linear,
        }, // 2
        ParameterRange {
            name: "reproduction_efficiency".into(),
            min: 0.1,
            max: 0.9,
            scale: Scale::Linear,
        }, // 3
        ParameterRange {
            name: "base_metabolic_rate".into(),
            min: 0.01,
            max: 0.5,
            scale: Scale::Linear,
        }, // 4
        ParameterRange {
            name: "movement_cost_coefficient".into(),
            min: 0.001,
            max: 0.1,
            scale: Scale::Linear,
        }, // 5
        ParameterRange {
            name: "sensing_range_coefficient".into(),
            min: 1.0,
            max: 30.0,
            scale: Scale::Linear,
        }, // 6
        ParameterRange {
            name: "reproduction_energy_threshold".into(),
            min: 5.0,
            max: 50.0,
            scale: Scale::Linear,
        }, // 7
        ParameterRange {
            name: "mutation_rate".into(),
            min: 0.01,
            max: 0.5,
            scale: Scale::Linear,
        }, // 8
        ParameterRange {
            name: "mutation_magnitude".into(),
            min: 0.01,
            max: 0.5,
            scale: Scale::Linear,
        }, // 9
        ParameterRange {
            name: "contact_range_coefficient".into(),
            min: 0.5,
            max: 5.0,
            scale: Scale::Linear,
        }, // 10
        ParameterRange {
            name: "world_extent".into(),
            min: 20.0,
            max: 100.0,
            scale: Scale::Linear,
        }, // 11
        ParameterRange {
            name: "initial_population_size".into(),
            min: 5.0,
            max: 50.0,
            scale: Scale::Linear,
        }, // 12
        ParameterRange {
            name: "light_competition_radius".into(),
            min: 1.0,
            max: 20.0,
            scale: Scale::Linear,
        }, // 13
        ParameterRange {
            name: "photo_maintenance_cost".into(),
            min: 0.001,
            max: 0.1,
            scale: Scale::Linear,
        }, // 14
        ParameterRange {
            name: "heterotrophy_maintenance_cost".into(),
            min: 0.001,
            max: 0.1,
            scale: Scale::Linear,
        }, // 15
        ParameterRange {
            name: "reproductive_compatibility_distance".into(),
            min: 0.5,
            max: 5.0,
            scale: Scale::Linear,
        }, // 16
        ParameterRange {
            name: "mean_photosynthetic_absorption".into(),
            min: 0.0,
            max: 1.0,
            scale: Scale::Linear,
        }, // 17
        ParameterRange {
            name: "mean_heterotrophy".into(),
            min: 0.0,
            max: 1.0,
            scale: Scale::Linear,
        }, // 18
        ParameterRange {
            name: "mean_mobility".into(),
            min: 0.0,
            max: 1.0,
            scale: Scale::Linear,
        }, // 19
        ParameterRange {
            name: "mean_kappa".into(),
            min: 0.0,
            max: 1.0,
            scale: Scale::Linear,
        }, // 20
        ParameterRange {
            name: "trait_covariance".into(),
            min: 0.1,
            max: 1.0,
            scale: Scale::Linear,
        }, // 21
        ParameterRange {
            name: "initial_cluster_count".into(),
            min: 1.0,
            max: 5.0,
            scale: Scale::Linear,
        }, // 22
        ParameterRange {
            name: "initial_energy_per_agent".into(),
            min: 1.0,
            max: 50.0,
            scale: Scale::Linear,
        }, // 23
        ParameterRange {
            name: "base_nutrient_ratio".into(),
            min: 0.01,
            max: 0.5,
            scale: Scale::Linear,
        }, // 24
        ParameterRange {
            name: "specification_nutrient_coefficient".into(),
            min: 0.01,
            max: 0.5,
            scale: Scale::Linear,
        }, // 25
        ParameterRange {
            name: "mean_asexual_propensity".into(),
            min: 0.0,
            max: 1.0,
            scale: Scale::Linear,
        }, // 26
        ParameterRange {
            name: "mean_dispersal".into(),
            min: 0.0,
            max: 2.0,
            scale: Scale::Linear,
        }, // 27
        ParameterRange {
            name: "maintenance_cost_exponent".into(),
            min: 1.5,
            max: 3.0,
            scale: Scale::Linear,
        }, // 28
        ParameterRange {
            name: "growth_retention_multiplier".into(),
            min: 1.0,
            max: 5.0,
            scale: Scale::Linear,
        }, // 29
        ParameterRange {
            name: "offspring_structure_fraction".into(),
            min: 0.05,
            max: 0.5,
            scale: Scale::Linear,
        }, // 30
        ParameterRange {
            name: "reserve_mobilisation_rate".into(),
            // Reserve mobilisation rate `f` (flow 9). The lower bound stays above
            // zero so reserve always drains *some* surplus per tick (rate 0 would
            // freeze growth and reproduction entirely); 1.0 is the historical
            // one-tick liquidation. Search explores `f < 1` for the buffering that
            // lets discrete-meal consumers survive between meals.
            min: 0.05,
            max: 1.0,
            scale: Scale::Linear,
        }, // 31
        ParameterRange {
            name: "uptake_structure_exponent".into(),
            // Uptake structure exponent `b` (flow 2, #653), linear over [0, 1]:
            // the range must hold `b = 0` exactly — the size-blind rule every
            // world before #653 ran — so a log scale is out. Above 1 a large
            // body would take up more per unit structure than a small one,
            // which no domain evidence supports. The reference structure
            // `s_ref` stays out of the box at its default, 100.
            min: 0.0,
            max: 1.0,
            scale: Scale::Linear,
        }, // 32
    ]
}

/// The full box as it stood before the uptake structure exponent joined it
/// (#653): the first 32 [`untaxed_ranges`] dims, with uptake size-blind
/// (`b = 0`, which [`decode`] gives any box without `b`'s coordinate). An
/// atlas from before #559 records no box and was searched under this one.
pub fn size_blind_ranges() -> Vec<ParameterRange> {
    let mut ranges = untaxed_ranges();
    ranges.truncate(SIZE_BLIND_DIMS);
    ranges
}

/// The raw coordinates of [`size_blind_ranges`].
const SIZE_BLIND_DIMS: usize = 32;

/// The dims the narrowed search box keeps at their full [`default_ranges`]
/// width (#559). The first eight are the raw core both LHS draws select on
/// their own (`docs/research/462-held-out-check.md`); `trait_covariance` and
/// `mean_heterotrophy` stay wide for the bloom-onset margin.
pub const FULL_WIDTH_DIMS: [&str; 10] = [
    "light_competition_radius",
    "world_extent",
    "solar_flux_magnitude",
    "initial_population_size",
    "contact_range_coefficient",
    "mean_kappa",
    "mean_mobility",
    "reproduction_energy_threshold",
    "trait_covariance",
    "mean_heterotrophy",
];

/// The fraction of a dim's full [`default_ranges`] span the narrowed search
/// box keeps for every dim outside [`FULL_WIDTH_DIMS`] (#559). A judgement
/// call the held-out data does not settle — undetectable at n ≈ 200 is not
/// zero, so the band keeps some width — and so it lives here, in one place.
pub const NARROWED_BAND_FRACTION: f64 = 0.25;

/// An opt-in narrowed search box (#559): the
/// [`default_ranges`] dims, in the same order, with every dim outside
/// [`FULL_WIDTH_DIMS`] shrunk to a band of [`NARROWED_BAND_FRACTION`] of its
/// full span around its [`band_centre`]. A band that would cross a full-range
/// bound is slid back inside it, so every band keeps its full width, lies
/// within the full range, and contains its centre.
///
/// The raw `decode` coordinates are kept (the held-out check rejects a
/// reduced decode); only the box they span changes. A unit vector therefore
/// names a world only together with the box it is decoded over — which is why
/// the atlas records its box ([`crate::qd::Atlas::search_box`]).
pub fn narrowed_ranges() -> Vec<ParameterRange> {
    default_ranges()
        .into_iter()
        .map(|r| {
            if FULL_WIDTH_DIMS.contains(&r.name.as_str()) {
                return r;
            }
            let width = NARROWED_BAND_FRACTION * (r.max - r.min);
            let lo = (band_centre(&r) - width / 2.0).clamp(r.min, r.max - width);
            ParameterRange {
                min: lo,
                max: lo + width,
                ..r
            }
        })
        .collect()
}

/// The centre of a shrunk dim's band in [`narrowed_ranges`] (#559): the value
/// `decode` inherits for that field from the known-viable baseline — the
/// "obvious centre" of `462-held-out-check.md`:
///
/// | dim | centre |
/// |---|---|
/// | `base_trophic_efficiency` | 0.8 |
/// | `trophic_distance_decay` | 1.0 |
/// | `reproduction_efficiency` | 0.7 |
/// | `base_metabolic_rate` | 0.3 |
/// | `movement_cost_coefficient` | 0.05 |
/// | `sensing_range_coefficient` | 10.0 |
/// | `mutation_rate` | 0.1 |
/// | `mutation_magnitude` | 0.05 |
/// | `photo_maintenance_cost` | 0.01 |
/// | `heterotrophy_maintenance_cost` | 0.01 |
/// | `reproductive_compatibility_distance` | 2.0 |
/// | `base_nutrient_ratio` | 0.1 |
/// | `specification_nutrient_coefficient` | 0.2 |
/// | `maintenance_cost_exponent` | 2.0 |
/// | `growth_retention_multiplier` | 2.0 |
/// | `offspring_structure_fraction` | 0.2 |
/// | `reserve_mobilisation_rate` | 1.0 (the full range's top, so the band is its top quarter) |
/// | `uptake_structure_exponent` | 0.0 (size-blind uptake, the range's bottom, so the band is its bottom quarter) |
/// | `cross_trait_cost` | 0.0 (untaxed mixotrophy, the range's bottom, so the band is its bottom quarter; only in a narrowing of [`taxed_ranges`]) |
/// | `leaching_rate` | 0.0 (no leaching, the range's bottom, so the band is its bottom quarter, `[0, 0.0025]`, still on the square scale; only in a narrowing of [`leached_ranges`]) |
///
/// The founder-distribution dims have no inherited value — `decode` sets the
/// whole `InitialDistribution` from the unit vector — so their centre is the
/// midpoint of the full range: `mean_photosynthetic_absorption` 0.5,
/// `initial_cluster_count` 3, `initial_energy_per_agent` 25.5,
/// `mean_asexual_propensity` 0.5, `mean_dispersal` 1.0. (Any other dim,
/// including the full-width core, also reads as its midpoint; the core is
/// never banded.)
pub fn band_centre(range: &ParameterRange) -> f64 {
    let b = viable_baseline();
    let inherited = match range.name.as_str() {
        "base_trophic_efficiency" => b.base_trophic_efficiency,
        "trophic_distance_decay" => b.trophic_distance_decay,
        "reproduction_efficiency" => b.reproduction_efficiency,
        "base_metabolic_rate" => b.base_metabolic_rate,
        "movement_cost_coefficient" => b.movement_cost_coefficient,
        "sensing_range_coefficient" => b.sensing_range_coefficient,
        "mutation_rate" => b.mutation_rate,
        "mutation_magnitude" => b.mutation_magnitude,
        "photo_maintenance_cost" => b.photo_maintenance_cost,
        "heterotrophy_maintenance_cost" => b.heterotrophy_maintenance_cost,
        "reproductive_compatibility_distance" => b.reproductive_compatibility_distance,
        "base_nutrient_ratio" => b.base_nutrient_ratio,
        "specification_nutrient_coefficient" => b.specification_nutrient_coefficient,
        "maintenance_cost_exponent" => b.maintenance_cost_exponent,
        "growth_retention_multiplier" => b.growth_retention_multiplier,
        "offspring_structure_fraction" => b.offspring_structure_fraction,
        "reserve_mobilisation_rate" => b.reserve_mobilisation_rate,
        "uptake_structure_exponent" => b.uptake_structure_exponent,
        "cross_trait_cost" => b.cross_trait_cost,
        "leaching_rate" => b.leaching_rate,
        _ => return (range.min + range.max) / 2.0,
    };
    // The baseline holds f32; read it back at the decimal it was written as,
    // so a centre of 0.1 is 0.1 and not 0.10000000149.
    format!("{inherited}")
        .parse()
        .expect("an f32 prints as an f64")
}

/// A reader asked to decode unit vectors over a box other than the one they
/// were drawn under (#559). Under a different box the same unit names a
/// different world, so this is refused rather than decoded.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchBoxMismatch {
    /// Each dim that differs, as `name: recorded [min, max] vs reader [min, max]`,
    /// or the two dimension counts when they differ.
    pub differences: Vec<String>,
}

impl std::fmt::Display for SearchBoxMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the unit vectors were drawn under a different search box than the one \
             they would be decoded over, refusing to decode them: {}",
            self.differences.join("; ")
        )
    }
}

impl std::error::Error for SearchBoxMismatch {}

/// Check that `reader`, the box a reader would decode over, is exactly the
/// box `recorded` the unit vectors were drawn under (#559).
pub fn check_search_box(
    recorded: &[ParameterRange],
    reader: &[ParameterRange],
) -> Result<(), SearchBoxMismatch> {
    let differences: Vec<String> = if recorded.len() != reader.len() {
        vec![format!(
            "recorded box has {} dims, reader's {}",
            recorded.len(),
            reader.len()
        )]
    } else {
        recorded
            .iter()
            .zip(reader)
            .filter(|(a, b)| a != b)
            .map(|(a, b)| {
                format!(
                    "{}: recorded [{}, {}] {:?} vs reader {} [{}, {}] {:?}",
                    a.name, a.min, a.max, a.scale, b.name, b.min, b.max, b.scale
                )
            })
            .collect()
    };
    if differences.is_empty() {
        Ok(())
    } else {
        Err(SearchBoxMismatch { differences })
    }
}

/// A single named known-viable baseline `WorldParameters`, taken verbatim from
/// the committed example4/example9 scenario template (the fully-specified,
/// post-#309 set). Every field is given a sane non-zero value where the template
/// is non-zero, so `decode()` can start from this and override only the searched
/// dimensions — any parameter not in `default_ranges` inherits a viable value
/// rather than the mechanically-fatal inline zeros it used to get (issue #326).
fn viable_baseline() -> WorldParameters {
    WorldParameters {
        solar_flux_magnitude: 10.0,
        base_trophic_efficiency: 0.8,
        trophic_distance_decay: 1.0,
        reproduction_efficiency: 0.7,
        base_metabolic_rate: 0.3,
        movement_cost_coefficient: 0.05,
        sensing_range_coefficient: 10.0,
        reproduction_energy_threshold: 15.0,
        reproduction_nutrient_threshold: 1.0,
        mutation_rate: 0.1,
        mutation_magnitude: 0.05,
        contact_range_coefficient: 3.0,
        world_extent: 100.0,
        initial_population_size: 0,
        light_competition_radius: 8.0,
        photo_maintenance_cost: 0.01,
        heterotrophy_maintenance_cost: 0.01,
        initial_nutrient_pool: 50000.0,
        growth_efficiency: 0.3,
        wear_rate: 0.0,
        wear_degradation_steepness: 1.0,
        somatic_maintenance_cost_coefficient: 0.1,
        use_wear_rate: 0.0,
        structure_maintenance_coefficient: 0.01,
        repair_decay: 1.0,
        base_nutrient_ratio: 0.1,
        specification_nutrient_coefficient: 0.2,
        reproductive_compatibility_distance: 2.0,
        mobility_maintenance_cost: 0.0,
        maintenance_cost_exponent: 2.0,
        nutrient_grid_cell_size: 10.0,
        growth_retention_multiplier: 2.0,
        reserve_mobilisation_rate: 1.0,
        offspring_structure_fraction: 0.2,
        asexual_propensity_maintenance_cost: 0.01,
        dispersal_propagule_cost_coefficient: 0.0,
        dispersal_propagule_cost_exponent: 2.0,
        dispersal_reach_coefficient: 10.0,
        body_reach_coefficient: 0.0,
        network_connection_cap: 0,
        network_creation_cost: 0.0,
        network_maintenance_cost: 0.0,
        network_redistribution_rate: 0.0,
        network_transfer_efficiency: 0.0,
        uptake_structure_exponent: 0.0,
        uptake_reference_structure: explorers_sim::DEFAULT_UPTAKE_REFERENCE_STRUCTURE,
        recognition_distance: 0.5,
        cross_trait_cost: 0.0,
        leaching_rate: 0.0,
    }
}

/// The world a unit vector `values` names over the box `ranges`, with the
/// parameters `fixed` holds outside it (#716).
///
/// Each coordinate is read **by its range's name**, never by its position
/// (#701): boxes of the same length can hold different fields at the same
/// index (the committed atlas's 34th coordinate is `c_AH`, the leached box's
/// is `λ`), so a box is identified by the names it records, not its length. A
/// field the box has no coordinate for keeps the known-viable baseline's
/// value, which for the fields later boxes added (`b`, `c_AH`, `λ`) is the
/// latent default 0 every world before them ran, unless `fixed` holds it at
/// a value of its own. The 32 fields every box has carried since the
/// size-blind box must be present.
pub fn decode(
    values: &[f64],
    ranges: &[ParameterRange],
    fixed: &FixedParameters,
) -> (WorldParameters, InitialDistribution) {
    let field = |name: &str| -> Option<f64> {
        ranges
            .iter()
            .position(|r| r.name == name)
            .map(|i| ranges[i].value_at(values[i]))
            .or_else(|| fixed.get(name))
    };
    let v = |name: &str| -> f64 {
        field(name).unwrap_or_else(|| panic!("the search box has no `{name}` coordinate"))
    };

    // Start from the known-viable baseline and override only the searched
    // dimensions, so non-searched fields inherit sane values rather than zero.
    let params = WorldParameters {
        solar_flux_magnitude: v("solar_flux_magnitude") as f32,
        base_trophic_efficiency: v("base_trophic_efficiency") as f32,
        trophic_distance_decay: v("trophic_distance_decay") as f32,
        reproduction_efficiency: v("reproduction_efficiency") as f32,
        base_metabolic_rate: v("base_metabolic_rate") as f32,
        movement_cost_coefficient: v("movement_cost_coefficient") as f32,
        sensing_range_coefficient: v("sensing_range_coefficient") as f32,
        reproduction_energy_threshold: v("reproduction_energy_threshold") as f32,
        mutation_rate: v("mutation_rate") as f32,
        mutation_magnitude: v("mutation_magnitude") as f32,
        contact_range_coefficient: v("contact_range_coefficient") as f32,
        world_extent: v("world_extent") as f32,
        initial_population_size: v("initial_population_size").round() as u32,
        light_competition_radius: v("light_competition_radius") as f32,
        photo_maintenance_cost: v("photo_maintenance_cost") as f32,
        heterotrophy_maintenance_cost: v("heterotrophy_maintenance_cost") as f32,
        reproductive_compatibility_distance: v("reproductive_compatibility_distance") as f32,
        base_nutrient_ratio: v("base_nutrient_ratio") as f32,
        specification_nutrient_coefficient: v("specification_nutrient_coefficient") as f32,
        maintenance_cost_exponent: v("maintenance_cost_exponent") as f32,
        growth_retention_multiplier: v("growth_retention_multiplier") as f32,
        offspring_structure_fraction: v("offspring_structure_fraction") as f32,
        reserve_mobilisation_rate: v("reserve_mobilisation_rate") as f32,
        // A box from before #653 has no coordinate for `b`: its worlds keep
        // the baseline's size-blind uptake, `b = 0`.
        uptake_structure_exponent: field("uptake_structure_exponent").map_or(0.0, |x| x as f32),
        // Only the committed atlas's box (#677, `taxed_ranges`) has a
        // coordinate for `c_AH`; every other box's worlds keep the baseline's
        // untaxed mixotrophy, `c_AH = 0`.
        cross_trait_cost: field("cross_trait_cost").map_or(0.0, |x| x as f32),
        // #686's and #711's boxes have a coordinate for `λ`; genesis's box
        // since #716 holds it fixed. A box with neither (every atlas from
        // before #701, the committed one included) keeps the baseline's
        // `λ = 0`, no leaching.
        leaching_rate: field("leaching_rate").map_or(0.0, |x| x as f32),
        ..viable_baseline()
    };

    let dist = InitialDistribution {
        mean_traits: TraitVector {
            photosynthetic_absorption: v("mean_photosynthetic_absorption") as f32,
            heterotrophy: v("mean_heterotrophy") as f32,
            mobility: v("mean_mobility") as f32,
            kappa: v("mean_kappa") as f32,
            // Founder fecundity inherits the known-viable template value; the
            // search does not vary it, so it must not default to sterile (0.0).
            fecundity: 0.35,
            asexual_propensity: v("mean_asexual_propensity") as f32,
            dispersal: v("mean_dispersal") as f32,
        },
        trait_covariance: v("trait_covariance") as f32,
        initial_cluster_count: v("initial_cluster_count").round() as u32,
        initial_energy_per_agent: v("initial_energy_per_agent") as f32,
        // Founder aggregation is not yet searched (#607): decoded worlds found
        // at the aggregated design default.
        founder_aggregation: explorers_sim::DEFAULT_FOUNDER_AGGREGATION,
    };

    (params, dist)
}

/// The genesis outer search: QD (CMA-MAE) illumination over world-parameter
/// space, returning the [`Atlas`]. Delegates to [`run_qd`](crate::qd::run_qd), reusing [`decode`]
/// and `run_ensemble` unchanged (issue #365). The LHS/Sobol/GP-BO path is retired
/// from production — its modules stay in the crate, dormant.
pub fn run_search(config: &SearchConfig, base_seed: u64, rng: &mut impl Rng) -> SearchResult {
    run_search_observed(config, base_seed, rng, &mut |_: &GenerationReport| {})
}

/// [`run_search`], calling `observer` as each QD generation completes (#529) —
/// see [`run_qd_observed`]. Reporting only: the atlas is unchanged.
pub fn run_search_observed(
    config: &SearchConfig,
    base_seed: u64,
    rng: &mut impl Rng,
    observer: &mut impl FnMut(&GenerationReport),
) -> SearchResult {
    run_qd_observed(&config.qd(), base_seed, rng, observer)
}

/// [`run_search_observed`], writing a checkpoint to `checkpoint` at every
/// generation boundary (#530) — see [`run_qd_checkpointed`]. The atlas is the
/// uncheckpointed search's.
pub fn run_search_checkpointed(
    config: &SearchConfig,
    base_seed: u64,
    rng: ChaCha8Rng,
    checkpoint: &Path,
    observer: &mut impl FnMut(&GenerationReport),
) -> Result<SearchResult, CheckpointError> {
    run_qd_checkpointed(&config.qd(), base_seed, rng, checkpoint, observer)
}

/// Resume a checkpointed search (#530) — see [`resume_qd`]. Refused unless
/// `(config, base_seed)` match the ones the checkpoint was written under.
pub fn resume_search(
    config: &SearchConfig,
    base_seed: u64,
    checkpoint: &Path,
    observer: &mut impl FnMut(&GenerationReport),
) -> Result<SearchResult, CheckpointError> {
    resume_qd(&config.qd(), base_seed, checkpoint, observer)
}

impl SearchConfig {
    /// The QD knobs this configuration drives.
    fn qd(&self) -> QdConfig {
        QdConfig {
            ranges: self.ranges.clone(),
            fixed: self.fixed.clone(),
            ensemble_size: self.ensemble_size,
            max_ticks: self.max_ticks,
            batch: self.batch,
            generations: self.generations,
            sigma: self.sigma,
            archive_learning_rate: self.archive_learning_rate,
            prefilter_crosscheck_fraction: self.prefilter_crosscheck_fraction,
            early_stop_crosscheck_fraction: self.early_stop_crosscheck_fraction,
            carcass_seed_count: self.carcass_seed_count,
            rollout_budget: self.rollout_budget,
            bloom_stop: self.bloom_stop,
            top_up_screen: self.top_up_screen,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #573: the search stops a rollout that has bloomed to 10x its founders
    /// by tick 300, by default.
    #[test]
    fn the_search_applies_the_bloom_stop_at_300_by_10_by_default() {
        let rule = Some(BloomStop {
            tick: 300,
            factor: 10.0,
        });
        assert_eq!(SearchConfig::default().bloom_stop, rule);
        assert_eq!(SearchConfig::default().qd().bloom_stop, rule);
        assert_eq!(crate::qd::QdConfig::default().bloom_stop, rule);
    }

    /// #582: the evaluator the search runs by default is the shared
    /// `EvalConfig::search()` the app's verdict panel reads with, so the two
    /// cannot drift apart.
    #[test]
    fn the_default_search_evaluates_with_the_shared_search_eval_config() {
        let shared = explorers_genesis::EvalConfig::search();
        assert_eq!(
            format!("{:?}", SearchConfig::default().qd().eval_config()),
            format!("{shared:?}")
        );
    }

    #[test]
    fn the_bloom_stop_flag_reads_tick_colon_factor_and_refuses_anything_else() {
        assert_eq!(
            parse_bloom_stop("300:5"),
            Ok(BloomStop {
                tick: 300,
                factor: 5.0
            })
        );
        assert_eq!(parse_bloom_stop("500:2.5").map(|b| b.factor), Ok(2.5));
        for bad in [
            "300", "300:", ":5", "x:5", "300:y", "0:5", "300:-1", "300:inf",
        ] {
            assert!(parse_bloom_stop(bad).is_err(), "{bad}");
        }
    }

    /// #559: the narrowed box names the same dims (34 since #669) in the same order (so a
    /// unit vector has the same length and axis meaning under either box), and
    /// keeps the ten core dims at exactly their full-box bounds.
    #[test]
    fn the_narrowed_box_keeps_the_core_at_full_width() {
        let full = default_ranges();
        let narrowed = narrowed_ranges();
        let names = |rs: &[ParameterRange]| rs.iter().map(|r| r.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&narrowed), names(&full));
        let core = [
            "light_competition_radius",
            "world_extent",
            "solar_flux_magnitude",
            "initial_population_size",
            "contact_range_coefficient",
            "mean_kappa",
            "mean_mobility",
            "reproduction_energy_threshold",
            "trait_covariance",
            "mean_heterotrophy",
        ];
        assert_eq!(FULL_WIDTH_DIMS, core);
        for (n, f) in narrowed.iter().zip(&full) {
            if core.contains(&f.name.as_str()) {
                assert_eq!((n.min, n.max), (f.min, f.max), "{}", f.name);
            }
        }
    }

    /// The search runs over the full box by default. The narrowed box (#559)
    /// lost QD coverage and clustering-axis diversity against the full box on
    /// two search seeds, so it is opt-in via `ranges: narrowed_ranges()`.
    #[test]
    fn the_search_defaults_to_the_full_box() {
        let ranges = SearchConfig::default().ranges;
        let bounds = |rs: &[ParameterRange]| rs.iter().map(|r| (r.min, r.max)).collect::<Vec<_>>();
        assert_eq!(bounds(&ranges), bounds(&default_ranges()));
        assert_ne!(bounds(&ranges), bounds(&narrowed_ranges()));
    }

    /// #559: every other dim is a band of `NARROWED_BAND_FRACTION` of its
    /// full span, inside its full bounds, containing its centre.
    #[test]
    fn every_other_dim_is_a_band_around_its_centre() {
        let full = default_ranges();
        let narrowed = narrowed_ranges();
        let shrunk: Vec<_> = narrowed
            .iter()
            .zip(&full)
            .filter(|(n, _)| !FULL_WIDTH_DIMS.contains(&n.name.as_str()))
            .collect();
        // 22 dims at #559, plus the uptake structure exponent (#653).
        assert_eq!(shrunk.len(), 23);
        for (n, f) in shrunk {
            let centre = band_centre(f);
            assert!(f.min <= n.min && n.max <= f.max, "{} outside", f.name);
            assert!(
                n.min <= centre && centre <= n.max,
                "{} misses its centre",
                f.name
            );
            let width = NARROWED_BAND_FRACTION * (f.max - f.min);
            assert!(
                ((n.max - n.min) - width).abs() < 1e-12,
                "{}: width {} not {width}",
                f.name,
                n.max - n.min
            );
        }
    }

    /// #559: a shrunk dim's centre is the value `decode` inherits from the
    /// known-viable baseline for that field — decoding every dim at its centre
    /// gives back the baseline for every searched `WorldParameters` field the
    /// box shrinks. The founder-distribution dims have no inherited value, so
    /// their centre is the midpoint of the full range.
    #[test]
    fn a_band_centre_is_the_value_decode_inherits() {
        let full = default_ranges();
        let unit: Vec<f64> = full
            .iter()
            .map(|r| (band_centre(r) - r.min) / (r.max - r.min))
            .collect();
        let (p, d) = decode(&unit, &full, &FixedParameters::genesis());
        let b = viable_baseline();
        let close = |got: f32, want: f32, name: &str| {
            assert!(
                (got - want).abs() <= 1e-6 * want.abs().max(1.0),
                "{name}: {got} vs {want}"
            )
        };
        close(p.base_trophic_efficiency, b.base_trophic_efficiency, "bte");
        close(p.trophic_distance_decay, b.trophic_distance_decay, "tdd");
        close(p.reproduction_efficiency, b.reproduction_efficiency, "re");
        close(p.base_metabolic_rate, b.base_metabolic_rate, "bmr");
        close(
            p.movement_cost_coefficient,
            b.movement_cost_coefficient,
            "mcc",
        );
        close(
            p.sensing_range_coefficient,
            b.sensing_range_coefficient,
            "src",
        );
        close(p.mutation_rate, b.mutation_rate, "mr");
        close(p.mutation_magnitude, b.mutation_magnitude, "mm");
        close(
            p.uptake_structure_exponent,
            b.uptake_structure_exponent,
            "use",
        );
        close(p.photo_maintenance_cost, b.photo_maintenance_cost, "pmc");
        close(
            p.heterotrophy_maintenance_cost,
            b.heterotrophy_maintenance_cost,
            "hmc",
        );
        close(
            p.reproductive_compatibility_distance,
            b.reproductive_compatibility_distance,
            "rcd",
        );
        close(p.base_nutrient_ratio, b.base_nutrient_ratio, "bnr");
        close(
            p.specification_nutrient_coefficient,
            b.specification_nutrient_coefficient,
            "snc",
        );
        close(
            p.maintenance_cost_exponent,
            b.maintenance_cost_exponent,
            "mce",
        );
        close(
            p.growth_retention_multiplier,
            b.growth_retention_multiplier,
            "grm",
        );
        close(
            p.offspring_structure_fraction,
            b.offspring_structure_fraction,
            "osf",
        );
        close(
            p.reserve_mobilisation_rate,
            b.reserve_mobilisation_rate,
            "rmr",
        );
        // No inherited value: the midpoint of the full range.
        close(d.mean_traits.photosynthetic_absorption, 0.5, "mpa");
        close(d.mean_traits.asexual_propensity, 0.5, "map");
        close(d.mean_traits.dispersal, 1.0, "md");
        close(d.initial_energy_per_agent, 25.5, "iepa");
        assert_eq!(d.initial_cluster_count, 3);
    }

    /// A range maps its unit coordinate onto `[min, max]` on its scale:
    /// linearly, or as `min + u² · (max − min)` on the square scale (#701).
    #[test]
    fn a_range_maps_the_unit_interval_on_its_scale() {
        let linear = ParameterRange {
            name: "a".into(),
            min: 10.0,
            max: 20.0,
            scale: Scale::Linear,
        };
        assert_eq!(linear.value_at(0.0), 10.0);
        assert_eq!(linear.value_at(0.5), 15.0);
        assert_eq!(linear.value_at(1.0), 20.0);
        let square = ParameterRange {
            scale: Scale::Square,
            ..linear
        };
        assert_eq!(square.value_at(0.0), 10.0);
        assert_eq!(square.value_at(0.5), 12.5);
        assert_eq!(square.value_at(1.0), 20.0);
    }

    #[test]
    fn decode_never_zeros_load_bearing_fields() {
        // The baseline must give viable values to the load-bearing fields at
        // every unit-vector input, so the decoder can never silently seed a
        // mechanically-fatal world (zero growth, zero nutrient, sterile founders).
        let ranges = default_ranges();
        for &u in &[0.0, 0.5, 1.0] {
            let unit = vec![u; ranges.len()];
            let (params, dist) = decode(&unit, &ranges, &FixedParameters::genesis());
            assert!(
                params.growth_efficiency > 0.0,
                "growth_efficiency must be > 0 at unit input {u}, got {}",
                params.growth_efficiency
            );
            assert!(
                params.initial_nutrient_pool > 0.0,
                "initial_nutrient_pool must be > 0 at unit input {u}, got {}",
                params.initial_nutrient_pool
            );
            assert!(
                dist.mean_traits.fecundity > 0.0,
                "founder fecundity must be > 0 at unit input {u}, got {}",
                dist.mean_traits.fecundity
            );
        }
    }

    #[test]
    fn decode_produces_valid_world_parameters() {
        let ranges = default_ranges();
        let unit = vec![0.5; ranges.len()];
        let (params, dist) = decode(&unit, &ranges, &FixedParameters::genesis());

        assert!(params.solar_flux_magnitude > 0.0);
        assert!(params.initial_population_size > 0);
        assert!(dist.initial_cluster_count > 0);
        assert!(dist.initial_energy_per_agent > 0.0);
    }

    #[test]
    fn decode_at_boundaries() {
        let ranges = default_ranges();

        let zeros = vec![0.0; ranges.len()];
        let (params_lo, _) = decode(&zeros, &ranges, &FixedParameters::genesis());
        assert!((params_lo.solar_flux_magnitude - 1.0).abs() < 1e-5);

        let ones = vec![1.0; ranges.len()];
        let (params_hi, _) = decode(&ones, &ranges, &FixedParameters::genesis());
        assert!((params_hi.solar_flux_magnitude - 20.0).abs() < 1e-5);
    }

    /// #653: the uptake structure exponent `b` is searched linearly over
    /// `[0, 1]` (the range must hold `b = 0` exactly, so every size-blind
    /// world stays reachable), while the uptake reference structure is not
    /// searched and decodes to 100 wherever the raw coordinate sits.
    #[test]
    fn decode_spans_the_uptake_structure_exponent_over_zero_to_one() {
        let ranges = default_ranges();
        let idx = ranges
            .iter()
            .position(|r| r.name == "uptake_structure_exponent")
            .expect("uptake_structure_exponent must be a searched parameter");
        assert!(
            !ranges
                .iter()
                .any(|r| r.name == "uptake_reference_structure"),
            "s_ref stays out of the box"
        );
        let mut unit = vec![0.5; ranges.len()];
        for (u, want) in [(0.0, 0.0), (0.25, 0.25), (0.5, 0.5), (1.0, 1.0)] {
            unit[idx] = u;
            let (params, _) = decode(&unit, &ranges, &FixedParameters::genesis());
            assert_eq!(params.uptake_structure_exponent, want, "unit {u}");
            assert_eq!(params.uptake_reference_structure, 100.0, "unit {u}");
        }
    }

    /// #716: the default box is the 33-coordinate untaxed box again: neither
    /// the cross-trait cost nor the leaching rate is a coordinate of it.
    #[test]
    fn the_default_box_is_the_untaxed_box_without_a_leaching_rate() {
        let ranges = default_ranges();
        assert_eq!(ranges, untaxed_ranges());
        assert_eq!(ranges.len(), 33);
        assert!(!ranges.iter().any(|r| r.name == "leaching_rate"));
        assert!(!ranges.iter().any(|r| r.name == "cross_trait_cost"));
    }

    /// #716: every world genesis decodes from the default box runs at the
    /// fixed leaching rate 0.0025, which the search holds outside the box.
    #[test]
    fn a_world_decoded_from_the_default_box_leaches_at_the_genesis_rate() {
        assert_eq!(GENESIS_LEACHING_RATE, 0.0025);
        let config = SearchConfig::default();
        assert_eq!(config.fixed, FixedParameters::genesis());
        for u in [0.0, 0.5, 1.0] {
            let unit = vec![u; config.ranges.len()];
            let (params, _) = decode(&unit, &config.ranges, &config.fixed);
            assert_eq!(params.leaching_rate, 0.0025, "unit {u}");
            assert_eq!(params.cross_trait_cost, 0.0, "unit {u}");
        }
    }

    /// #701: the box #686's and #711's atlases were searched under ends in
    /// `λ` over `[0, 0.01]` on a square scale, after the untaxed box.
    #[test]
    fn the_leached_box_ends_in_the_leaching_rate_on_a_square_scale() {
        let ranges = leached_ranges();
        assert_eq!(ranges.len(), 34);
        let last = ranges.last().unwrap();
        assert_eq!(last.name, "leaching_rate");
        assert_eq!((last.min, last.max), (0.0, 0.01));
        assert_eq!(last.scale, Scale::Square);
        assert_eq!(ranges[..33], untaxed_ranges()[..]);
    }

    /// #701: `λ = 0.01 · u²` for the coordinate `u`, holding 0 exactly; and
    /// with `c_AH` out of the box every searched world keeps `c_AH = 0`.
    #[test]
    fn decode_reads_the_leaching_rate_as_its_top_times_u_squared() {
        let ranges = leached_ranges();
        let idx = ranges.len() - 1;
        let mut unit = vec![0.5; ranges.len()];
        for (u, want) in [(0.0, 0.0), (0.5, 0.0025), (1.0, 0.01)] {
            unit[idx] = u;
            let (params, _) = decode(&unit, &ranges, &FixedParameters::none());
            assert_eq!(params.leaching_rate, want as f32, "unit {u}");
            assert_eq!(params.cross_trait_cost, 0.0, "unit {u}");
        }
    }

    /// #653: a unit vector drawn under the size-blind box (the full box before
    /// `b` joined it, 32 raw coordinates) still decodes to the world it named,
    /// with `b = 0` — the same world as the untaxed box (#669) at `b`'s raw
    /// coordinate 0.
    #[test]
    fn a_size_blind_box_still_decodes_its_worlds_with_b_zero() {
        let old = size_blind_ranges();
        let full = untaxed_ranges();
        assert_eq!(old.len(), 32);
        assert_eq!(old[..], full[..32]);
        for u in [0.0, 0.3, 1.0] {
            let unit: Vec<f64> = (0..32).map(|i| (u + i as f64 * 0.017) % 1.0).collect();
            let (params, dist) = decode(&unit, &old, &FixedParameters::none());
            assert_eq!(params.uptake_structure_exponent, 0.0);
            assert_eq!(params.uptake_reference_structure, 100.0);
            let mut extended = unit.clone();
            extended.push(0.0);
            let (p33, d33) = decode(&extended, &full, &FixedParameters::none());
            assert_eq!(format!("{params:?}"), format!("{p33:?}"));
            assert_eq!(format!("{dist:?}"), format!("{d33:?}"));
        }
    }

    /// #669, #701: a unit vector drawn under the untaxed box (33 raw
    /// coordinates, #663's atlas's) still decodes to the world it named, with
    /// `c_AH = 0` and `λ = 0`: the same world as the leached box at `λ`'s
    /// coordinate 0 and as the taxed box at `c_AH`'s.
    #[test]
    fn an_untaxed_box_still_decodes_its_worlds_with_c_ah_and_lambda_zero() {
        let old = untaxed_ranges();
        assert_eq!(old.len(), 33);
        for u in [0.0, 0.3, 1.0] {
            let unit: Vec<f64> = (0..33).map(|i| (u + i as f64 * 0.017) % 1.0).collect();
            let (params, dist) = decode(&unit, &old, &FixedParameters::none());
            assert_eq!(params.cross_trait_cost, 0.0);
            assert_eq!(params.leaching_rate, 0.0);
            let mut extended = unit.clone();
            extended.push(0.0);
            for full in [leached_ranges(), taxed_ranges()] {
                assert_eq!(old[..], full[..33]);
                let (p34, d34) = decode(&extended, &full, &FixedParameters::none());
                assert_eq!(format!("{params:?}"), format!("{p34:?}"));
                assert_eq!(format!("{dist:?}"), format!("{d34:?}"));
            }
        }
    }

    /// #701: the committed atlas's box (#677) has as many coordinates as the
    /// leached box (#686's and #711's), and its 34th is `c_AH`, not `λ`.
    /// `decode` tells them apart by name: over the taxed box the 34th
    /// coordinate is `c_AH`, linear over `[0, 0.14]`, with `λ = 0`; the same
    /// unit over the leached box is `λ`, with `c_AH = 0`.
    #[test]
    fn the_taxed_box_decodes_its_last_coordinate_as_c_ah_with_lambda_zero() {
        let taxed = taxed_ranges();
        assert_eq!(taxed.len(), leached_ranges().len());
        let last = taxed.last().unwrap();
        assert_eq!(last.name, "cross_trait_cost");
        assert_eq!((last.min, last.max), (0.0, 0.14));
        assert_eq!(last.scale, Scale::Linear);
        let mut unit = vec![0.5; taxed.len()];
        for (u, want) in [(0.0, 0.0), (0.5, 0.07), (1.0, 0.14)] {
            unit[33] = u;
            let (params, _) = decode(&unit, &taxed, &FixedParameters::none());
            assert_eq!(params.cross_trait_cost, want as f32, "unit {u}");
            assert_eq!(params.leaching_rate, 0.0, "unit {u}");
            let (full, _) = decode(&unit, &leached_ranges(), &FixedParameters::none());
            assert_eq!(full.cross_trait_cost, 0.0, "unit {u}");
        }
    }

    /// #701: a range serialised without a scale (every box recorded before
    /// the scale existed) reads as linear, and a linear range writes no scale,
    /// so a linear box is written exactly as before. A square range writes its
    /// scale and reads back as written.
    #[test]
    fn a_range_without_a_scale_reads_as_linear_and_a_square_one_round_trips() {
        let legacy: ParameterRange =
            serde_json::from_str(r#"{"name":"cross_trait_cost","min":0.0,"max":0.14}"#).unwrap();
        assert_eq!(legacy, taxed_ranges()[33]);
        assert_eq!(
            serde_json::to_string(&legacy).unwrap(),
            r#"{"name":"cross_trait_cost","min":0.0,"max":0.14}"#
        );
        let lambda = leached_ranges().pop().unwrap();
        let text = serde_json::to_string(&lambda).unwrap();
        assert_eq!(
            text,
            r#"{"name":"leaching_rate","min":0.0,"max":0.01,"scale":"square"}"#
        );
        assert_eq!(
            serde_json::from_str::<ParameterRange>(&text).unwrap(),
            lambda
        );
    }

    #[test]
    fn decode_explores_reserve_mobilisation_rate_below_one() {
        // Issue #384: the reserve mobilisation rate `f` must be a searchable
        // dimension so genesis can find the buffering regime `f < 1`. At unit input
        // 0.0 the decoder must produce a rate strictly below 1.0 (the lower end of
        // its range), and the dimension must round-trip through its own index — not
        // inherit the baseline's no-op 1.0.
        let ranges = default_ranges();
        let mut unit = vec![0.5; ranges.len()];
        let idx = ranges
            .iter()
            .position(|r| r.name == "reserve_mobilisation_rate")
            .expect("reserve_mobilisation_rate must be a searched parameter");
        unit[idx] = 0.0; // minimum of the range
        let (params, _) = decode(&unit, &ranges, &FixedParameters::genesis());
        assert!(
            params.reserve_mobilisation_rate < 1.0,
            "search must be able to reach f < 1 (got {})",
            params.reserve_mobilisation_rate
        );
        // And the top of the range reproduces the historical no-op exactly.
        unit[idx] = 1.0;
        let (params_hi, _) = decode(&unit, &ranges, &FixedParameters::genesis());
        assert!(
            (params_hi.reserve_mobilisation_rate - 1.0).abs() < 1e-5,
            "f at the top of its range must be 1.0 (historical no-op), got {}",
            params_hi.reserve_mobilisation_rate
        );
    }

    #[test]
    fn slow_run_search_returns_an_atlas() {
        // run_search is now the QD outer loop: its output is the Atlas (live cells
        // + dead frontier + coverage / QD-score), not a ranked list. Every config
        // routes to a cell or the frontier; the binning is RESOLUTION³.
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let config = SearchConfig {
            ensemble_size: 1,
            max_ticks: 10,
            batch: 4,
            generations: 1,
            ..Default::default()
        };

        let atlas = run_search(&config, 42, &mut rng);

        assert_eq!(atlas.total_cells, crate::qd::RESOLUTION.pow(3));
        let dead: usize = atlas.dead_frontier.values().sum();
        assert!(atlas.coverage + dead >= 1);
        // QD-score is the sum of elite fitnesses over filled cells.
        assert!(atlas.qd_score >= 0.0);
        // Live cells carry the per-cell decomposer, consumer + coexistence
        // distributions and sample count.
        for cell in &atlas.cells {
            assert!(cell.decomposer_fraction >= 0.0 && cell.decomposer_fraction <= 1.0);
            assert!(cell.consumer_fraction >= 0.0 && cell.consumer_fraction <= 1.0);
            assert!(cell.coexistence_fraction >= 0.0 && cell.coexistence_fraction <= 1.0);
            assert_eq!(cell.sample_count, config.ensemble_size);
        }
    }

    #[test]
    fn slow_atlas_best_recipe_is_the_robust_cell_projection() {
        // The default projection is the highest-fitness live cell that clears the
        // coexistence floor (argmax fallback when none clears) — #401. Either way
        // the recipe round-trips through serde and decodes to a non-degenerate
        // world; that totality (a live atlas always yields a recipe) is the claim.
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let config = SearchConfig {
            ensemble_size: 1,
            max_ticks: 10,
            batch: 6,
            generations: 1,
            ..Default::default()
        };

        let atlas = run_search(&config, 42, &mut rng);
        if let Some(recipe) = atlas.best_recipe(&config.ranges, config.max_ticks) {
            let json = serde_json::to_string_pretty(&recipe).unwrap();
            let recovered: explorers_sim::WorldRecipe = serde_json::from_str(&json).unwrap();
            assert_eq!(recipe, recovered);
            assert!(recipe.parameters.solar_flux_magnitude > 0.0);
            assert!(recipe.parameters.initial_population_size > 0);
        }
    }

    #[test]
    fn slow_run_search_is_reproducible() {
        // Issue #350 + #365: the QD outer loop must be bit-reproducible for a
        // fixed (config, base_seed, rng) — same coverage, dead frontier, and best
        // fitness across two runs.
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let config = SearchConfig {
            ensemble_size: 2,
            max_ticks: 15,
            batch: 4,
            generations: 1,
            ..Default::default()
        };

        let mut rng1 = ChaCha8Rng::seed_from_u64(42);
        let atlas1 = run_search(&config, 7, &mut rng1);
        let mut rng2 = ChaCha8Rng::seed_from_u64(42);
        let atlas2 = run_search(&config, 7, &mut rng2);

        assert_eq!(atlas1.coverage, atlas2.coverage);
        assert_eq!(atlas1.dead_frontier, atlas2.dead_frontier);
        assert_eq!(atlas1.best_fitness, atlas2.best_fitness);
        assert_eq!(atlas1.qd_score, atlas2.qd_score);
    }

    #[test]
    fn slow_run_search_observed_reports_every_generation_to_the_caller() {
        // #529: the CLI's entry point carries the per-generation observer
        // through to the QD loop, and the atlas matches the unobserved search.
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let config = SearchConfig {
            ensemble_size: 1,
            max_ticks: 15,
            batch: 4,
            generations: 2,
            ..Default::default()
        };

        let mut seen: Vec<usize> = Vec::new();
        let mut rng1 = ChaCha8Rng::seed_from_u64(42);
        let observed = run_search_observed(&config, 7, &mut rng1, &mut |r: &GenerationReport| {
            seen.push(r.generation)
        });
        let mut rng2 = ChaCha8Rng::seed_from_u64(42);
        let unobserved = run_search(&config, 7, &mut rng2);

        assert_eq!(seen, vec![0, 1, 2]);
        assert_eq!(observed.coverage, unobserved.coverage);
        assert_eq!(observed.qd_score, unobserved.qd_score);
    }

    #[test]
    fn slow_the_search_entry_checkpoints_and_resumes_to_the_same_atlas() {
        // #530: the CLI's entry point carries checkpointing through to the QD
        // loop — a checkpointed search, and a resume from its checkpoint after
        // the bootstrap, both write the plain search's atlas.
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let config = SearchConfig {
            ensemble_size: 1,
            max_ticks: 15,
            batch: 4,
            generations: 2,
            ..Default::default()
        };
        let dir = std::env::temp_dir().join(format!(
            "explorers-search-entry-checkpoint-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let live = dir.join("search.checkpoint.json");
        let after_bootstrap = dir.join("after-bootstrap.json");

        let plain = run_search(&config, 7, &mut ChaCha8Rng::seed_from_u64(42));
        let checkpointed = run_search_checkpointed(
            &config,
            7,
            ChaCha8Rng::seed_from_u64(42),
            &live,
            &mut |r: &GenerationReport| {
                if r.generation == 0 {
                    std::fs::copy(&live, &after_bootstrap).unwrap();
                }
            },
        )
        .unwrap();
        let resumed =
            resume_search(&config, 7, &after_bootstrap, &mut |_: &GenerationReport| {}).unwrap();

        let written = |atlas: &SearchResult| serde_json::to_value(atlas).unwrap();
        assert_eq!(written(&checkpointed), written(&plain));
        assert_eq!(written(&resumed), written(&plain));
    }

    #[test]
    fn base_metabolic_rate_range_capped_at_half() {
        let ranges = default_ranges();
        let bmr = ranges
            .iter()
            .find(|r| r.name == "base_metabolic_rate")
            .unwrap();
        assert!((bmr.min - 0.01).abs() < 1e-10);
        assert!((bmr.max - 0.5).abs() < 1e-10);
    }

    #[test]
    fn trait_covariance_range_widened() {
        let ranges = default_ranges();
        let tc = ranges
            .iter()
            .find(|r| r.name == "trait_covariance")
            .unwrap();
        assert!((tc.min - 0.1).abs() < 1e-10);
        assert!((tc.max - 1.0).abs() < 1e-10);
    }

    #[test]
    fn network_parameters_are_not_searched() {
        // Flow 5 slice 1 (#409): the network mechanism is default-disabled and has
        // no genesis criterion this epic, so its parameters must NOT be in the
        // search ranges — genesis never explores them, keeping the atlas and
        // existing recipes bit-unchanged.
        let ranges = default_ranges();
        for r in &ranges {
            assert!(
                !r.name.starts_with("network_"),
                "network parameter {} must not be searched by genesis",
                r.name
            );
        }
    }

    #[test]
    fn stoichiometric_parameters_in_search_ranges() {
        let ranges = default_ranges();
        let bnr = ranges
            .iter()
            .find(|r| r.name == "base_nutrient_ratio")
            .expect("base_nutrient_ratio should be in search ranges");
        assert!(bnr.min >= 0.0);
        assert!(bnr.max > bnr.min);

        let snc = ranges
            .iter()
            .find(|r| r.name == "specification_nutrient_coefficient")
            .expect("specification_nutrient_coefficient should be in search ranges");
        assert!(snc.min >= 0.0);
        assert!(snc.max > snc.min);

        // decode at midpoint should produce non-zero values
        let unit = vec![0.5; ranges.len()];
        let (params, _) = decode(&unit, &ranges, &FixedParameters::genesis());
        assert!(
            params.base_nutrient_ratio > 0.0,
            "decoded base_nutrient_ratio should be positive"
        );
        assert!(
            params.specification_nutrient_coefficient > 0.0,
            "decoded specification_nutrient_coefficient should be positive"
        );
    }
}
