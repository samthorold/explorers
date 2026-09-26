use rand::Rng;

use explorers_genesis::{InitialDistribution, RolloutBudget, WorldParameters};
use explorers_sim::TraitVector;

use std::path::Path;

use rand_chacha::ChaCha8Rng;

use crate::checkpoint::{CheckpointError, resume_qd, run_qd_checkpointed};
use crate::qd::{Atlas, GenerationReport, QdConfig, SEARCH_ROLLOUT_BUDGET, run_qd_observed};

/// The genesis search output is the [`Atlas`] (CONTEXT.md) — the live archive of
/// behaviour cells plus the dead frontier. `SearchResult` is kept as the public
/// alias the consumers name; `best_recipe` / `recipe_for_cell` live on `Atlas`.
pub type SearchResult = Atlas;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ParameterRange {
    pub name: String,
    pub min: f64,
    pub max: f64,
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
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            ranges: default_ranges(),
            ensemble_size: 5,
            max_ticks: 2000,
            batch: 32,
            generations: 10,
            sigma: 0.15,
            archive_learning_rate: 0.5,
            prefilter_crosscheck_fraction: 0.05,
            early_stop_crosscheck_fraction: 0.05,
            carcass_seed_count: 2,
            rollout_budget: SEARCH_ROLLOUT_BUDGET,
        }
    }
}

pub fn default_ranges() -> Vec<ParameterRange> {
    vec![
        ParameterRange {
            name: "solar_flux_magnitude".into(),
            min: 1.0,
            max: 20.0,
        }, // 0
        ParameterRange {
            name: "base_trophic_efficiency".into(),
            min: 0.1,
            max: 0.9,
        }, // 1
        ParameterRange {
            name: "trophic_distance_decay".into(),
            min: 0.1,
            max: 5.0,
        }, // 2
        ParameterRange {
            name: "reproduction_efficiency".into(),
            min: 0.1,
            max: 0.9,
        }, // 3
        ParameterRange {
            name: "base_metabolic_rate".into(),
            min: 0.01,
            max: 0.5,
        }, // 4
        ParameterRange {
            name: "movement_cost_coefficient".into(),
            min: 0.001,
            max: 0.1,
        }, // 5
        ParameterRange {
            name: "sensing_range_coefficient".into(),
            min: 1.0,
            max: 30.0,
        }, // 6
        ParameterRange {
            name: "reproduction_energy_threshold".into(),
            min: 5.0,
            max: 50.0,
        }, // 7
        ParameterRange {
            name: "mutation_rate".into(),
            min: 0.01,
            max: 0.5,
        }, // 8
        ParameterRange {
            name: "mutation_magnitude".into(),
            min: 0.01,
            max: 0.5,
        }, // 9
        ParameterRange {
            name: "contact_range_coefficient".into(),
            min: 0.5,
            max: 5.0,
        }, // 10
        ParameterRange {
            name: "world_extent".into(),
            min: 20.0,
            max: 100.0,
        }, // 11
        ParameterRange {
            name: "initial_population_size".into(),
            min: 5.0,
            max: 50.0,
        }, // 12
        ParameterRange {
            name: "light_competition_radius".into(),
            min: 1.0,
            max: 20.0,
        }, // 13
        ParameterRange {
            name: "photo_maintenance_cost".into(),
            min: 0.001,
            max: 0.1,
        }, // 14
        ParameterRange {
            name: "heterotrophy_maintenance_cost".into(),
            min: 0.001,
            max: 0.1,
        }, // 15
        ParameterRange {
            name: "reproductive_compatibility_distance".into(),
            min: 0.5,
            max: 5.0,
        }, // 16
        ParameterRange {
            name: "mean_photosynthetic_absorption".into(),
            min: 0.0,
            max: 1.0,
        }, // 17
        ParameterRange {
            name: "mean_heterotrophy".into(),
            min: 0.0,
            max: 1.0,
        }, // 18
        ParameterRange {
            name: "mean_mobility".into(),
            min: 0.0,
            max: 1.0,
        }, // 19
        ParameterRange {
            name: "mean_kappa".into(),
            min: 0.0,
            max: 1.0,
        }, // 20
        ParameterRange {
            name: "trait_covariance".into(),
            min: 0.1,
            max: 1.0,
        }, // 21
        ParameterRange {
            name: "initial_cluster_count".into(),
            min: 1.0,
            max: 5.0,
        }, // 22
        ParameterRange {
            name: "initial_energy_per_agent".into(),
            min: 1.0,
            max: 50.0,
        }, // 23
        ParameterRange {
            name: "base_nutrient_ratio".into(),
            min: 0.01,
            max: 0.5,
        }, // 24
        ParameterRange {
            name: "specification_nutrient_coefficient".into(),
            min: 0.01,
            max: 0.5,
        }, // 25
        ParameterRange {
            name: "mean_asexual_propensity".into(),
            min: 0.0,
            max: 1.0,
        }, // 26
        ParameterRange {
            name: "mean_dispersal".into(),
            min: 0.0,
            max: 2.0,
        }, // 27
        ParameterRange {
            name: "maintenance_cost_exponent".into(),
            min: 1.5,
            max: 3.0,
        }, // 28
        ParameterRange {
            name: "growth_retention_multiplier".into(),
            min: 1.0,
            max: 5.0,
        }, // 29
        ParameterRange {
            name: "offspring_structure_fraction".into(),
            min: 0.05,
            max: 0.5,
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
        }, // 31
    ]
}

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
/// The raw32 `decode` coordinates are kept (the held-out check rejects a
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
                    "{}: recorded [{}, {}] vs reader {} [{}, {}]",
                    a.name, a.min, a.max, b.name, b.min, b.max
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
    }
}

pub fn decode(values: &[f64], ranges: &[ParameterRange]) -> (WorldParameters, InitialDistribution) {
    let v = |i: usize| -> f64 {
        let r = &ranges[i];
        r.min + values[i] * (r.max - r.min)
    };

    // Start from the known-viable baseline and override only the searched
    // dimensions, so non-searched fields inherit sane values rather than zero.
    let params = WorldParameters {
        solar_flux_magnitude: v(0) as f32,
        base_trophic_efficiency: v(1) as f32,
        trophic_distance_decay: v(2) as f32,
        reproduction_efficiency: v(3) as f32,
        base_metabolic_rate: v(4) as f32,
        movement_cost_coefficient: v(5) as f32,
        sensing_range_coefficient: v(6) as f32,
        reproduction_energy_threshold: v(7) as f32,
        mutation_rate: v(8) as f32,
        mutation_magnitude: v(9) as f32,
        contact_range_coefficient: v(10) as f32,
        world_extent: v(11) as f32,
        initial_population_size: v(12).round() as u32,
        light_competition_radius: v(13) as f32,
        photo_maintenance_cost: v(14) as f32,
        heterotrophy_maintenance_cost: v(15) as f32,
        reproductive_compatibility_distance: v(16) as f32,
        base_nutrient_ratio: v(24) as f32,
        specification_nutrient_coefficient: v(25) as f32,
        maintenance_cost_exponent: v(28) as f32,
        growth_retention_multiplier: v(29) as f32,
        offspring_structure_fraction: v(30) as f32,
        reserve_mobilisation_rate: v(31) as f32,
        ..viable_baseline()
    };

    let dist = InitialDistribution {
        mean_traits: TraitVector {
            photosynthetic_absorption: v(17) as f32,
            heterotrophy: v(18) as f32,
            mobility: v(19) as f32,
            kappa: v(20) as f32,
            // Founder fecundity inherits the known-viable template value; the
            // search does not vary it, so it must not default to sterile (0.0).
            fecundity: 0.35,
            asexual_propensity: v(26) as f32,
            dispersal: v(27) as f32,
        },
        trait_covariance: v(21) as f32,
        initial_cluster_count: v(22).round() as u32,
        initial_energy_per_agent: v(23) as f32,
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #559: the narrowed box names the same 32 dims in the same order (so a
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
        assert_eq!(shrunk.len(), 22);
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
        let (p, d) = decode(&unit, &full);
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

    #[test]
    fn decode_maps_unit_interval_to_parameter_ranges() {
        let ranges = vec![
            ParameterRange {
                name: "a".into(),
                min: 10.0,
                max: 20.0,
            },
            ParameterRange {
                name: "b".into(),
                min: 0.0,
                max: 1.0,
            },
        ];

        let values_at_zero = vec![0.0, 0.0];
        let values_at_one = vec![1.0, 1.0];
        let values_at_half = vec![0.5, 0.5];

        let r = &ranges;
        let decode_val = |vals: &[f64], i: usize| -> f64 {
            let range = &r[i];
            range.min + vals[i] * (range.max - range.min)
        };

        assert!((decode_val(&values_at_zero, 0) - 10.0).abs() < 1e-10);
        assert!((decode_val(&values_at_one, 0) - 20.0).abs() < 1e-10);
        assert!((decode_val(&values_at_half, 1) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn decode_never_zeros_load_bearing_fields() {
        // The baseline must give viable values to the load-bearing fields at
        // every unit-vector input, so the decoder can never silently seed a
        // mechanically-fatal world (zero growth, zero nutrient, sterile founders).
        let ranges = default_ranges();
        for &u in &[0.0, 0.5, 1.0] {
            let unit = vec![u; ranges.len()];
            let (params, dist) = decode(&unit, &ranges);
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
        let (params, dist) = decode(&unit, &ranges);

        assert!(params.solar_flux_magnitude > 0.0);
        assert!(params.initial_population_size > 0);
        assert!(dist.initial_cluster_count > 0);
        assert!(dist.initial_energy_per_agent > 0.0);
    }

    #[test]
    fn decode_at_boundaries() {
        let ranges = default_ranges();

        let zeros = vec![0.0; ranges.len()];
        let (params_lo, _) = decode(&zeros, &ranges);
        assert!((params_lo.solar_flux_magnitude - 1.0).abs() < 1e-5);

        let ones = vec![1.0; ranges.len()];
        let (params_hi, _) = decode(&ones, &ranges);
        assert!((params_hi.solar_flux_magnitude - 20.0).abs() < 1e-5);
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
        let (params, _) = decode(&unit, &ranges);
        assert!(
            params.reserve_mobilisation_rate < 1.0,
            "search must be able to reach f < 1 (got {})",
            params.reserve_mobilisation_rate
        );
        // And the top of the range reproduces the historical no-op exactly.
        unit[idx] = 1.0;
        let (params_hi, _) = decode(&unit, &ranges);
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
        let (params, _) = decode(&unit, &ranges);
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
