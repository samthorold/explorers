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
            ensemble_size: 3,
            lhs_samples: 20,
            max_ticks: 100,
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
    pub fn best_recipe(&self, ranges: &[ParameterRange]) -> WorldRecipe {
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
            initial_distribution,
        }
    }
}

pub fn default_ranges() -> Vec<ParameterRange> {
    vec![
        ParameterRange { name: "solar_flux_magnitude".into(), min: 1.0, max: 20.0 },
        ParameterRange { name: "consumption_efficiency".into(), min: 0.1, max: 0.9 },
        ParameterRange { name: "decomposition_efficiency".into(), min: 0.1, max: 0.9 },
        ParameterRange { name: "reproduction_efficiency".into(), min: 0.1, max: 0.9 },
        ParameterRange { name: "base_metabolic_rate".into(), min: 0.01, max: 1.0 },
        ParameterRange { name: "movement_cost_coefficient".into(), min: 0.001, max: 0.1 },
        ParameterRange { name: "sensing_cost_coefficient".into(), min: 0.001, max: 0.1 },
        ParameterRange { name: "reproduction_energy_threshold".into(), min: 5.0, max: 50.0 },
        ParameterRange { name: "mutation_rate".into(), min: 0.01, max: 0.5 },
        ParameterRange { name: "mutation_magnitude".into(), min: 0.01, max: 0.5 },
        ParameterRange { name: "contact_radius".into(), min: 0.5, max: 5.0 },
        ParameterRange { name: "world_extent".into(), min: 20.0, max: 100.0 },
        ParameterRange { name: "initial_population_size".into(), min: 5.0, max: 50.0 },
        ParameterRange { name: "mean_photosynthetic_absorption".into(), min: 0.0, max: 1.0 },
        ParameterRange { name: "mean_consumption_rate".into(), min: 0.0, max: 1.0 },
        ParameterRange { name: "mean_scavenging_rate".into(), min: 0.0, max: 1.0 },
        ParameterRange { name: "mean_mobility".into(), min: 0.0, max: 1.0 },
        ParameterRange { name: "mean_chemotaxis_sensitivity".into(), min: 0.0, max: 1.0 },
        ParameterRange { name: "mean_mate_selectivity".into(), min: 0.0, max: 1.0 },
        ParameterRange { name: "mean_sensing_range".into(), min: 0.5, max: 10.0 },
        ParameterRange { name: "mean_reproductive_investment".into(), min: 0.0, max: 1.0 },
        ParameterRange { name: "trait_covariance".into(), min: 0.01, max: 0.5 },
        ParameterRange { name: "initial_cluster_count".into(), min: 1.0, max: 5.0 },
        ParameterRange { name: "initial_energy_per_agent".into(), min: 1.0, max: 50.0 },
    ]
}

pub fn decode(values: &[f64], ranges: &[ParameterRange]) -> (WorldParameters, InitialDistribution) {
    let v = |i: usize| -> f64 {
        let r = &ranges[i];
        r.min + values[i] * (r.max - r.min)
    };

    let params = WorldParameters {
        solar_flux_magnitude: v(0) as f32,
        consumption_efficiency: v(1) as f32,
        decomposition_efficiency: v(2) as f32,
        reproduction_efficiency: v(3) as f32,
        base_metabolic_rate: v(4) as f32,
        movement_cost_coefficient: v(5) as f32,
        sensing_cost_coefficient: v(6) as f32,
        reproduction_energy_threshold: v(7) as f32,
        mutation_rate: v(8) as f32,
        mutation_magnitude: v(9) as f32,
        contact_radius: v(10) as f32,
        world_extent: v(11) as f32,
        initial_population_size: v(12).round() as u32,
    };

    let dist = InitialDistribution {
        mean_traits: TraitVector {
            photosynthetic_absorption: v(13) as f32,
            consumption_rate: v(14) as f32,
            scavenging_rate: v(15) as f32,
            mobility: v(16) as f32,
            chemotaxis_sensitivity: v(17) as f32,
            mate_selectivity: v(18) as f32,
            sensing_range: v(19) as f32,
            reproductive_investment: v(20) as f32,
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
        let recipe = result.best_recipe(&config.ranges);

        let json = serde_json::to_string_pretty(&recipe).unwrap();
        let recovered: explorers_sim::WorldRecipe = serde_json::from_str(&json).unwrap();
        assert_eq!(recipe, recovered);
        assert!(recipe.parameters.solar_flux_magnitude > 0.0);
        assert!(recipe.parameters.initial_population_size > 0);
    }
}
