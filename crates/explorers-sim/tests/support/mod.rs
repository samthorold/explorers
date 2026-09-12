//! Shared property-test support for the sim stepper (Workstream C, #430).
//!
//! Provides a `proptest` strategy over `WorldParameters` + `InitialDistribution`
//! drawn from the genesis search's default ranges, so property suites check the
//! stepper's invariants across the same parameter space the search explores.
//!
//! The ranges are copied verbatim from `explorers-search::default_ranges()` /
//! `viable_baseline()` rather than imported: the sim must never depend on the
//! search crate (see docs/agents/architecture.md). Two dimensions are narrowed
//! for test speed — world extent and population — and are documented inline.
//!
//! Reuse from a property suite with `mod support;` and `support::world_case()`.

#![allow(dead_code)]

use explorers_sim::{InitialDistribution, TraitVector, WorldParameters};
use proptest::prelude::*;

/// Small worlds keep 256 cases × several properties well under a minute.
/// Search's `world_extent` range is 20..100; the upper bound is narrowed here.
pub const MAX_WORLD_EXTENT: f32 = 30.0;
/// Search's `initial_population_size` range is 5..50; narrowed to ≤ 40.
pub const MAX_POPULATION: u32 = 40;
/// Upper bound on ticks a generated case runs.
pub const MAX_TICKS: u32 = 20;

/// One reproducible, shrinkable stepper case: a parameterisation, its founding
/// distribution, the world seed, and how many ticks to run.
#[derive(Debug, Clone)]
pub struct WorldCase {
    pub params: WorldParameters,
    pub dist: InitialDistribution,
    pub seed: u64,
    pub ticks: u32,
}

/// The known-viable baseline every non-searched field inherits — copied from
/// `explorers-search::viable_baseline()` (the example4/example9 template).
pub fn viable_baseline() -> WorldParameters {
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

/// Strategy over the searched `WorldParameters` dimensions (ranges copied from
/// `explorers-search::default_ranges()`, indices 0..=16 and 24..=25, 28..=31).
pub fn world_parameters() -> impl Strategy<Value = WorldParameters> {
    world_parameters_with_exponent(1.5f32..=3.0)
}

/// `world_parameters()` with `maintenance_cost_exponent` restricted to the
/// integers 2 and 3 (both inside the search range). Workaround for #444:
/// `World::new` can seed a negative founder trait, and a negative base under a
/// non-integer `powf` is NaN. Integer exponents keep every case finite so the
/// conservation properties still run in CI; delete this once #444 is fixed.
pub fn world_parameters_integer_exponent() -> impl Strategy<Value = WorldParameters> {
    world_parameters_with_exponent(prop_oneof![Just(2.0f32), Just(3.0f32)])
}

fn world_parameters_with_exponent(
    exponent: impl Strategy<Value = f32>,
) -> impl Strategy<Value = WorldParameters> {
    let searched = (
        (
            1.0f32..=20.0,  // 0  solar_flux_magnitude
            0.1f32..=0.9,   // 1  base_trophic_efficiency
            0.1f32..=5.0,   // 2  trophic_distance_decay
            0.1f32..=0.9,   // 3  reproduction_efficiency
            0.01f32..=0.5,  // 4  base_metabolic_rate
            0.001f32..=0.1, // 5  movement_cost_coefficient
            1.0f32..=30.0,  // 6  sensing_range_coefficient
            5.0f32..=50.0,  // 7  reproduction_energy_threshold
            0.01f32..=0.5,  // 8  mutation_rate
            0.01f32..=0.5,  // 9  mutation_magnitude
        ),
        (
            0.5f32..=5.0,               // 10 contact_range_coefficient
            20.0f32..=MAX_WORLD_EXTENT, // 11 world_extent (narrowed)
            5u32..=MAX_POPULATION,      // 12 initial_population_size (narrowed)
            1.0f32..=20.0,              // 13 light_competition_radius
            0.001f32..=0.1,             // 14 photo_maintenance_cost
            0.001f32..=0.1,             // 15 heterotrophy_maintenance_cost
            0.5f32..=5.0,               // 16 reproductive_compatibility_distance
            0.01f32..=0.5,              // 24 base_nutrient_ratio
            0.01f32..=0.5,              // 25 specification_nutrient_coefficient
        ),
        (
            exponent,      // 28 maintenance_cost_exponent (1.5..=3.0 in search)
            1.0f32..=5.0,  // 29 growth_retention_multiplier
            0.05f32..=0.5, // 30 offspring_structure_fraction
            0.05f32..=1.0, // 31 reserve_mobilisation_rate
        ),
    );
    searched.prop_map(|(a, b, c)| WorldParameters {
        solar_flux_magnitude: a.0,
        base_trophic_efficiency: a.1,
        trophic_distance_decay: a.2,
        reproduction_efficiency: a.3,
        base_metabolic_rate: a.4,
        movement_cost_coefficient: a.5,
        sensing_range_coefficient: a.6,
        reproduction_energy_threshold: a.7,
        mutation_rate: a.8,
        mutation_magnitude: a.9,
        contact_range_coefficient: b.0,
        world_extent: b.1,
        initial_population_size: b.2,
        light_competition_radius: b.3,
        photo_maintenance_cost: b.4,
        heterotrophy_maintenance_cost: b.5,
        reproductive_compatibility_distance: b.6,
        base_nutrient_ratio: b.7,
        specification_nutrient_coefficient: b.8,
        maintenance_cost_exponent: c.0,
        growth_retention_multiplier: c.1,
        offspring_structure_fraction: c.2,
        reserve_mobilisation_rate: c.3,
        ..viable_baseline()
    })
}

/// Strategy over the searched `InitialDistribution` dimensions (ranges copied
/// from `explorers-search::default_ranges()`, indices 17..=23 and 26..=27).
/// Founder fecundity is fixed at the search's template value (0.35), as in
/// `decode`.
pub fn initial_distribution() -> impl Strategy<Value = InitialDistribution> {
    (
        0.0f32..=1.0,  // 17 mean_photosynthetic_absorption
        0.0f32..=1.0,  // 18 mean_heterotrophy
        0.0f32..=1.0,  // 19 mean_mobility
        0.0f32..=1.0,  // 20 mean_kappa
        0.1f32..=1.0,  // 21 trait_covariance
        1u32..=5,      // 22 initial_cluster_count
        1.0f32..=50.0, // 23 initial_energy_per_agent
        0.0f32..=1.0,  // 26 mean_asexual_propensity
        0.0f32..=2.0,  // 27 mean_dispersal
    )
        .prop_map(
            |(photo, hetero, mobility, kappa, cov, clusters, energy, asexual, dispersal)| {
                InitialDistribution {
                    mean_traits: TraitVector {
                        photosynthetic_absorption: photo,
                        heterotrophy: hetero,
                        mobility,
                        kappa,
                        fecundity: 0.35,
                        asexual_propensity: asexual,
                        dispersal,
                    },
                    trait_covariance: cov,
                    initial_cluster_count: clusters,
                    initial_energy_per_agent: energy,
                }
            },
        )
}

/// A complete stepper case: parameters, distribution, world seed and tick count.
/// Seeds come from proptest's RNG so every case is reproducible from the
/// persisted regression seed and shrinks like any other input.
pub fn world_case() -> impl Strategy<Value = WorldCase> {
    world_case_from(world_parameters())
}

/// `world_case()` over `world_parameters_integer_exponent()` — the #444
/// workaround domain. Delete once #444 is fixed.
pub fn world_case_integer_exponent() -> impl Strategy<Value = WorldCase> {
    world_case_from(world_parameters_integer_exponent())
}

fn world_case_from(
    params: impl Strategy<Value = WorldParameters>,
) -> impl Strategy<Value = WorldCase> {
    (
        params,
        initial_distribution(),
        any::<u64>(),
        1u32..=MAX_TICKS,
    )
        .prop_map(|(params, dist, seed, ticks)| WorldCase {
            params,
            dist,
            seed,
            ticks,
        })
}
