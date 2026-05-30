use rand::Rng;
use serde::Serialize;

use explorers_genesis::{
    EnsembleConfig, EnsembleResult, RunConfig, WorldParameters,
    InitialDistribution, EvalConfig,
};
use explorers_sim::{TraitVector, WorldRecipe};

use crate::bayesopt::BayesianOptimiser;
use crate::lhs;
use crate::sobol;

#[derive(Clone, Debug)]
pub struct ParameterRange {
    pub name: String,
    pub min: f64,
    pub max: f64,
}

#[derive(Clone, Debug)]
pub struct SearchConfig {
    pub ranges: Vec<ParameterRange>,
    pub ensemble_size: u32,
    pub lhs_samples: usize,
    pub max_ticks: u64,
    pub bayesopt_iterations: usize,
    pub sensitivity_threshold: f64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            ranges: default_ranges(),
            ensemble_size: 5,
            lhs_samples: 50,
            max_ticks: 500,
            bayesopt_iterations: 10,
            sensitivity_threshold: 0.05,
        }
    }
}

#[derive(Serialize)]
pub struct EvaluatedParameterisation {
    pub parameters: Vec<f64>,
    pub parameter_names: Vec<String>,
    pub median_fitness: f32,
    pub run_breakdowns: Vec<RunBreakdown>,
}

#[derive(Serialize)]
pub struct RunBreakdown {
    pub fitness: f32,
    pub failure: Option<String>,
    pub termination_tick: u64,
    pub oscillation_strength: f32,
    pub clustering_strength: f32,
    pub coexistence_duration: f32,
    pub turnover_score: f32,
    pub trophic_balance_score: f32,
}

#[derive(Serialize)]
pub struct SensitivityReport {
    pub rankings: Vec<SensitivityEntry>,
}

#[derive(Serialize)]
pub struct SensitivityEntry {
    pub name: String,
    pub first_order: f64,
    pub total_effect: f64,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub parameterisations: Vec<EvaluatedParameterisation>,
    pub sensitivity: SensitivityReport,
    pub optimised: Vec<EvaluatedParameterisation>,
}

impl SearchResult {
    pub fn best_recipe(&self, ranges: &[ParameterRange], max_ticks: u64) -> WorldRecipe {
        let best = &self.optimised[0];
        let unit_values: Vec<f64> = best
            .parameters
            .iter()
            .zip(ranges.iter())
            .map(|(&actual, range)| (actual - range.min) / (range.max - range.min))
            .collect();
        let (parameters, initial_distribution) = decode(&unit_values, ranges);
        WorldRecipe {
            parameters,
            initial_distribution: Some(initial_distribution),
            agents: None,
            max_ticks,
        }
    }
}

pub fn default_ranges() -> Vec<ParameterRange> {
    vec![
        ParameterRange { name: "solar_flux_magnitude".into(), min: 1.0, max: 20.0 },           // 0
        ParameterRange { name: "base_trophic_efficiency".into(), min: 0.1, max: 0.9 },         // 1
        ParameterRange { name: "trophic_distance_decay".into(), min: 0.1, max: 5.0 },          // 2
        ParameterRange { name: "reproduction_efficiency".into(), min: 0.1, max: 0.9 },         // 3
        ParameterRange { name: "base_metabolic_rate".into(), min: 0.01, max: 0.5 },            // 4
        ParameterRange { name: "movement_cost_coefficient".into(), min: 0.001, max: 0.1 },     // 5
        ParameterRange { name: "sensing_range_coefficient".into(), min: 1.0, max: 30.0 },      // 6
        ParameterRange { name: "reproduction_energy_threshold".into(), min: 5.0, max: 50.0 },  // 7
        ParameterRange { name: "mutation_rate".into(), min: 0.01, max: 0.5 },                  // 8
        ParameterRange { name: "mutation_magnitude".into(), min: 0.01, max: 0.5 },             // 9
        ParameterRange { name: "contact_range_coefficient".into(), min: 0.5, max: 5.0 },         // 10
        ParameterRange { name: "world_extent".into(), min: 20.0, max: 100.0 },                 // 11
        ParameterRange { name: "initial_population_size".into(), min: 5.0, max: 50.0 },        // 12
        ParameterRange { name: "light_competition_radius".into(), min: 1.0, max: 20.0 },       // 13
        ParameterRange { name: "photo_maintenance_cost".into(), min: 0.001, max: 0.1 },        // 14
        ParameterRange { name: "heterotrophy_maintenance_cost".into(), min: 0.001, max: 0.1 },  // 15
        ParameterRange { name: "reproductive_compatibility_distance".into(), min: 0.5, max: 5.0 }, // 16
        ParameterRange { name: "mean_photosynthetic_absorption".into(), min: 0.0, max: 1.0 },  // 17
        ParameterRange { name: "mean_heterotrophy".into(), min: 0.0, max: 1.0 },               // 18
        ParameterRange { name: "mean_mobility".into(), min: 0.0, max: 1.0 },                   // 19
        ParameterRange { name: "mean_kappa".into(), min: 0.0, max: 1.0 },                        // 20
        ParameterRange { name: "trait_covariance".into(), min: 0.1, max: 1.0 },                // 21
        ParameterRange { name: "initial_cluster_count".into(), min: 1.0, max: 5.0 },           // 22
        ParameterRange { name: "initial_energy_per_agent".into(), min: 1.0, max: 50.0 },       // 23
        ParameterRange { name: "base_nutrient_ratio".into(), min: 0.01, max: 0.5 },            // 24
        ParameterRange { name: "specification_nutrient_coefficient".into(), min: 0.01, max: 0.5 }, // 25
        ParameterRange { name: "mean_asexual_propensity".into(), min: 0.0, max: 1.0 },        // 26
        ParameterRange { name: "mean_dispersal".into(), min: 0.0, max: 2.0 },                 // 27
        ParameterRange { name: "maintenance_cost_exponent".into(), min: 1.5, max: 3.0 },     // 28
        ParameterRange { name: "growth_retention_multiplier".into(), min: 1.0, max: 5.0 },   // 29
        ParameterRange { name: "offspring_structure_fraction".into(), min: 0.05, max: 0.5 }, // 30
    ]
}

pub fn decode(values: &[f64], ranges: &[ParameterRange]) -> (WorldParameters, InitialDistribution) {
    let v = |i: usize| -> f64 {
        let r = &ranges[i];
        r.min + values[i] * (r.max - r.min)
    };

    let params = WorldParameters {
        solar_flux_magnitude: v(0) as f32,
        base_trophic_efficiency: v(1) as f32,
        trophic_distance_decay: v(2) as f32,
        reproduction_efficiency: v(3) as f32,
        base_metabolic_rate: v(4) as f32,
        movement_cost_coefficient: v(5) as f32,
        sensing_range_coefficient: v(6) as f32,
        reproduction_energy_threshold: v(7) as f32,
        reproduction_nutrient_threshold: 1.0,
        mutation_rate: v(8) as f32,
        mutation_magnitude: v(9) as f32,
        contact_range_coefficient: v(10) as f32,
        world_extent: v(11) as f32,
        initial_population_size: v(12).round() as u32,
        light_competition_radius: v(13) as f32,
        photo_maintenance_cost: v(14) as f32,
        heterotrophy_maintenance_cost: v(15) as f32,
        reproductive_compatibility_distance: v(16) as f32,
        initial_nutrient_pool: 0.0,
        growth_efficiency: 0.0,
        wear_rate: 0.0,
        wear_degradation_steepness: 0.0,
        somatic_maintenance_cost_coefficient: 0.0,
        use_wear_rate: 0.0,
        structure_maintenance_coefficient: 0.0,
        repair_decay: 0.0,
        base_nutrient_ratio: v(24) as f32,
        specification_nutrient_coefficient: v(25) as f32,
        mobility_maintenance_cost: 0.0,
        maintenance_cost_exponent: v(28) as f32,
        consumption_contact_half_saturation: 0.001,
        nutrient_grid_cell_size: 10.0,
        growth_retention_multiplier: v(29) as f32,
        offspring_structure_fraction: v(30) as f32,
        asexual_propensity_maintenance_cost: 0.01,
    };

    let dist = InitialDistribution {
        mean_traits: TraitVector {
            photosynthetic_absorption: v(17) as f32,
            heterotrophy: v(18) as f32,
            mobility: v(19) as f32,
            kappa: v(20) as f32,
            fecundity: 0.0,
            asexual_propensity: v(26) as f32,
            dispersal: v(27) as f32,
        },
        trait_covariance: v(21) as f32,
        initial_cluster_count: v(22).round() as u32,
        initial_energy_per_agent: v(23) as f32,
    };

    (params, dist)
}

pub fn run_search(config: &SearchConfig, base_seed: u64, rng: &mut impl Rng) -> SearchResult {
    let dims = config.ranges.len();

    let samples = lhs::sample(dims, config.lhs_samples, rng);

    let ensemble_config = EnsembleConfig {
        ensemble_size: config.ensemble_size,
        run_config: RunConfig {
            max_ticks: config.max_ticks,
            eval_config: EvalConfig::default(),
        },
    };

    let mut parameterisations = Vec::new();
    let mut fitnesses = Vec::new();

    for (i, sample) in samples.iter().enumerate() {
        let (wp, dist) = decode(sample, &config.ranges);
        let seed = base_seed.wrapping_add(i as u64 * 1000);
        let result = explorers_genesis::run_ensemble(&wp, &dist, &ensemble_config, seed);

        fitnesses.push(result.median_fitness as f64);
        parameterisations.push(to_evaluated(sample, &config.ranges, &result));
    }

    let evaluate_for_sobol = |unit: &[f64]| -> f64 {
        let (wp, dist) = decode(unit, &config.ranges);
        let result = explorers_genesis::run_ensemble(&wp, &dist, &ensemble_config, base_seed);
        result.median_fitness as f64
    };

    let indices = sobol::sobol_indices(evaluate_for_sobol, dims, config.lhs_samples, rng);
    let ranking = sobol::rank_by_total_effect(&indices);

    let sensitivity = SensitivityReport {
        rankings: ranking
            .rankings
            .iter()
            .map(|&(idx, _)| SensitivityEntry {
                name: config.ranges[idx].name.clone(),
                first_order: indices.first_order[idx],
                total_effect: indices.total_effect[idx],
            })
            .collect(),
    };

    let fixed: Vec<Option<f64>> = (0..dims)
        .map(|d| {
            if indices.total_effect[d] < config.sensitivity_threshold {
                Some(0.5)
            } else {
                None
            }
        })
        .collect();

    let bounds: Vec<(f64, f64)> = (0..dims).map(|_| (0.0, 1.0)).collect();

    let mut optimiser = BayesianOptimiser::new(bounds, fixed);
    let opt_result = optimiser.optimise(
        |x| {
            let (wp, dist) = decode(x, &config.ranges);
            let result = explorers_genesis::run_ensemble(&wp, &dist, &ensemble_config, base_seed);
            result.median_fitness as f64
        },
        config.bayesopt_iterations,
        rng,
    );

    let (opt_wp, opt_dist) = decode(&opt_result.best_x, &config.ranges);
    let opt_ensemble = explorers_genesis::run_ensemble(&opt_wp, &opt_dist, &ensemble_config, base_seed);
    let optimised = vec![to_evaluated(&opt_result.best_x, &config.ranges, &opt_ensemble)];

    parameterisations.sort_by(|a, b| b.median_fitness.partial_cmp(&a.median_fitness).unwrap());

    SearchResult {
        parameterisations,
        sensitivity,
        optimised,
    }
}

fn to_evaluated(
    sample: &[f64],
    ranges: &[ParameterRange],
    result: &EnsembleResult,
) -> EvaluatedParameterisation {
    EvaluatedParameterisation {
        parameters: sample.iter().enumerate().map(|(i, &v)| {
            let r = &ranges[i];
            r.min + v * (r.max - r.min)
        }).collect(),
        parameter_names: ranges.iter().map(|r| r.name.clone()).collect(),
        median_fitness: result.median_fitness,
        run_breakdowns: result
            .run_results
            .iter()
            .map(|r| RunBreakdown {
                fitness: r.fitness,
                failure: r.failure.as_ref().map(|f| format!("{:?}", f)),
                termination_tick: r.termination_tick,
                oscillation_strength: r.breakdown.oscillation_strength,
                clustering_strength: r.breakdown.clustering_strength,
                coexistence_duration: r.breakdown.coexistence_duration,
                turnover_score: r.breakdown.turnover_score,
                trophic_balance_score: r.breakdown.trophic_balance_score,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_maps_unit_interval_to_parameter_ranges() {
        let ranges = vec![
            ParameterRange { name: "a".into(), min: 10.0, max: 20.0 },
            ParameterRange { name: "b".into(), min: 0.0, max: 1.0 },
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
    fn end_to_end_small_search_completes() {
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let config = SearchConfig {
            lhs_samples: 3,
            ensemble_size: 1,
            max_ticks: 10,
            bayesopt_iterations: 2,
            sensitivity_threshold: 0.05,
            ..Default::default()
        };

        let result = run_search(&config, 42, &mut rng);

        assert_eq!(result.parameterisations.len(), 3);
        assert!(!result.sensitivity.rankings.is_empty());
        assert!(!result.optimised.is_empty());

        for p in &result.parameterisations {
            assert_eq!(p.parameter_names.len(), config.ranges.len());
            assert_eq!(p.parameters.len(), config.ranges.len());
        }
    }

    #[test]
    fn search_result_converts_to_valid_recipe() {
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let config = SearchConfig {
            lhs_samples: 3,
            ensemble_size: 1,
            max_ticks: 10,
            bayesopt_iterations: 2,
            sensitivity_threshold: 0.05,
            ..Default::default()
        };

        let result = run_search(&config, 42, &mut rng);
        let recipe = result.best_recipe(&config.ranges, config.max_ticks);

        let json = serde_json::to_string_pretty(&recipe).unwrap();
        let recovered: explorers_sim::WorldRecipe = serde_json::from_str(&json).unwrap();
        assert_eq!(recipe, recovered);
        assert!(recipe.parameters.solar_flux_magnitude > 0.0);
        assert!(recipe.parameters.initial_population_size > 0);
    }

    #[test]
    fn base_metabolic_rate_range_capped_at_half() {
        let ranges = default_ranges();
        let bmr = ranges.iter().find(|r| r.name == "base_metabolic_rate").unwrap();
        assert!((bmr.min - 0.01).abs() < 1e-10);
        assert!((bmr.max - 0.5).abs() < 1e-10);
    }

    #[test]
    fn trait_covariance_range_widened() {
        let ranges = default_ranges();
        let tc = ranges.iter().find(|r| r.name == "trait_covariance").unwrap();
        assert!((tc.min - 0.1).abs() < 1e-10);
        assert!((tc.max - 1.0).abs() < 1e-10);
    }

    #[test]
    fn stoichiometric_parameters_in_search_ranges() {
        let ranges = default_ranges();
        let bnr = ranges.iter().find(|r| r.name == "base_nutrient_ratio")
            .expect("base_nutrient_ratio should be in search ranges");
        assert!(bnr.min >= 0.0);
        assert!(bnr.max > bnr.min);

        let snc = ranges.iter().find(|r| r.name == "specification_nutrient_coefficient")
            .expect("specification_nutrient_coefficient should be in search ranges");
        assert!(snc.min >= 0.0);
        assert!(snc.max > snc.min);

        // decode at midpoint should produce non-zero values
        let unit = vec![0.5; ranges.len()];
        let (params, _) = decode(&unit, &ranges);
        assert!(params.base_nutrient_ratio > 0.0,
            "decoded base_nutrient_ratio should be positive");
        assert!(params.specification_nutrient_coefficient > 0.0,
            "decoded specification_nutrient_coefficient should be positive");
    }
}
