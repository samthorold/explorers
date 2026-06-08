pub use explorers_genesis_eval::{EvalConfig, FailureMode, FitnessBreakdown};
pub use explorers_sim::{InitialDistribution, WorldParameters};
use rayon::prelude::*;

pub struct RunConfig {
    pub max_ticks: u64,
    pub eval_config: EvalConfig,
}

pub struct RunResult {
    pub fitness: f32,
    pub failure: Option<FailureMode>,
    pub termination_tick: u64,
    pub breakdown: FitnessBreakdown,
}

pub struct EnsembleConfig {
    pub ensemble_size: u32,
    pub run_config: RunConfig,
}

pub struct EnsembleResult {
    pub median_fitness: f32,
    pub run_results: Vec<RunResult>,
}

pub fn run_single(
    params: &WorldParameters,
    distribution: &InitialDistribution,
    run_config: &RunConfig,
    seed: u64,
) -> RunResult {
    let mut world = explorers_sim::World::new(params.clone(), distribution.clone(), seed);

    // Per-tick series the rollout observes for the descriptors that need a
    // temporal trace: free energy (issue #302), carcass fraction (#342) and
    // producer share (#392) sampled every tick, plus a coarse-interval trait-
    // vector snapshot for coexistence (#394). The world stays history-free — the
    // series live here, bundled as `RolloutObservations` for the evaluator. The
    // snapshots are pure observation; genesis does NOT cluster, the evaluator runs
    // DBSCAN on each.
    let cap = run_config.max_ticks as usize;
    let mut observations = explorers_genesis_eval::RolloutObservations {
        free_energy: Vec::with_capacity(cap),
        carcass_fraction: Vec::with_capacity(cap),
        producer_share: Vec::with_capacity(cap),
        cluster_snapshots: Vec::new(),
    };
    let interval = run_config.eval_config.coexistence_sample_interval.max(1);
    for _ in 0..run_config.max_ticks {
        world.step();
        observations.free_energy.push(world.free_energy());
        observations
            .carcass_fraction
            .push(world.carcass_locked_nutrient_fraction());
        observations
            .producer_share
            .push(world.producer_energy_share());
        if world.tick() % interval as u64 == 0 {
            observations.cluster_snapshots.push((
                world.tick(),
                world.agents().iter().map(|a| a.traits).collect(),
            ));
        }
        if world.agents().is_empty() {
            break;
        }
        if world.agents().len() > run_config.eval_config.max_population {
            break;
        }
    }

    let breakdown = explorers_genesis_eval::evaluate_from_log(
        &world,
        &observations,
        &run_config.eval_config,
        run_config.max_ticks,
    );
    let termination_tick = world.tick();
    RunResult {
        fitness: breakdown.fitness,
        failure: breakdown.failure.clone(),
        termination_tick,
        breakdown,
    }
}

pub fn run_ensemble(
    params: &WorldParameters,
    distribution: &InitialDistribution,
    config: &EnsembleConfig,
    base_seed: u64,
) -> EnsembleResult {
    // The seed loop is embarrassingly parallel: each seed builds its own
    // `World::new(…, seed)` with an independent RNG stream, so rollouts never
    // interact (issue #350). `into_par_iter().collect()` over the contiguous
    // seed range is order-stable — results land in seed order, bit-identical to
    // the sequential map — because rayon's IndexedParallelIterator preserves
    // index order on collect (no racy push).
    let run_results: Vec<RunResult> = (0..config.ensemble_size)
        .into_par_iter()
        .map(|i| {
            let seed = base_seed.wrapping_add(i as u64);
            run_single(params, distribution, &config.run_config, seed)
        })
        .collect();

    let median_fitness = median(&run_results.iter().map(|r| r.fitness).collect::<Vec<_>>());

    EnsembleResult {
        median_fitness,
        run_results,
    }
}

fn median(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use explorers_sim::TraitVector;

    fn test_params() -> WorldParameters {
        WorldParameters {
            solar_flux_magnitude: 5.0,
            base_trophic_efficiency: 0.5,
            trophic_distance_decay: 0.0,
            reproduction_efficiency: 0.5,
            base_metabolic_rate: 0.1,
            movement_cost_coefficient: 0.01,
            sensing_range_coefficient: 10.0,
            reproduction_energy_threshold: 20.0,
            reproduction_nutrient_threshold: 1.0,
            mutation_rate: 0.1,
            mutation_magnitude: 0.1,
            contact_range_coefficient: 2.0,
            world_extent: 50.0,
            initial_population_size: 10,
            light_competition_radius: 1000.0,
            photo_maintenance_cost: 0.0,
            heterotrophy_maintenance_cost: 0.0,
            initial_nutrient_pool: 0.0,
            growth_efficiency: 0.0,
            wear_rate: 0.0,
            wear_degradation_steepness: 0.0,
            somatic_maintenance_cost_coefficient: 0.0,
            use_wear_rate: 0.0,
            structure_maintenance_coefficient: 0.0,
            repair_decay: 0.0,
            base_nutrient_ratio: 0.1,
            specification_nutrient_coefficient: 0.2,
            reproductive_compatibility_distance: 2.0,
            mobility_maintenance_cost: 0.0,
            maintenance_cost_exponent: 1.0,
            nutrient_grid_cell_size: 10.0,
            growth_retention_multiplier: 2.0,
            reserve_mobilisation_rate: 1.0,
            offspring_structure_fraction: 0.2,
            asexual_propensity_maintenance_cost: 0.0,
            dispersal_propagule_cost_coefficient: 0.0,
            dispersal_propagule_cost_exponent: 2.0,
            dispersal_reach_coefficient: 0.0,
            body_reach_coefficient: 0.0,
            network_connection_cap: 0,
            network_creation_cost: 0.0,
            network_maintenance_cost: 0.0,
            network_redistribution_rate: 0.0,
            network_transfer_efficiency: 0.0,
        }
    }

    fn test_distribution() -> InitialDistribution {
        InitialDistribution {
            mean_traits: TraitVector {
                photosynthetic_absorption: 0.8,
                heterotrophy: 0.1,
                mobility: 0.3,
                kappa: 0.5,
                fecundity: 0.0,
                asexual_propensity: 0.0,
                dispersal: 0.0,
            },
            trait_covariance: 0.1,
            initial_cluster_count: 1,
            initial_energy_per_agent: 10.0,
        }
    }

    #[test]
    fn median_of_odd_count() {
        assert_eq!(median(&[3.0, 1.0, 2.0]), 2.0);
    }

    #[test]
    fn median_of_even_count() {
        assert_eq!(median(&[4.0, 1.0, 3.0, 2.0]), 2.5);
    }

    #[test]
    fn median_of_single_value() {
        assert_eq!(median(&[7.0]), 7.0);
    }

    #[test]
    fn different_seeds_produce_different_runs() {
        let params = WorldParameters {
            initial_population_size: 30,
            contact_range_coefficient: 10.0,
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            world_extent: 20.0,
            solar_flux_magnitude: 10.0,
            growth_efficiency: 0.5,
            // Seed agents with all-reserve, no structure, so the small
            // populations explored here don't collapse identically against
            // the structural death threshold.
            offspring_structure_fraction: 0.0,
            ..test_params()
        };
        let dist = InitialDistribution {
            trait_covariance: 0.5,
            initial_energy_per_agent: 100.0,
            ..test_distribution()
        };
        // A live post-grace window (the default 0.2 grace) so the seed-driven
        // differences in the post-grace descriptors actually surface — under a
        // full-run grace (1.0) the oscillation and coexistence guards zero those
        // descriptors for a world that survives exactly to max_ticks, masking the
        // trajectory difference this test asserts.
        let config = RunConfig {
            max_ticks: 200,
            eval_config: EvalConfig::default(),
        };
        let result_a = run_single(&params, &dist, &config, 1);
        let result_b = run_single(&params, &dist, &config, 12345);
        let a = &result_a.breakdown;
        let b = &result_b.breakdown;
        assert!(
            result_a.termination_tick != result_b.termination_tick
                || result_a.fitness != result_b.fitness
                || a.oscillation_strength != b.oscillation_strength
                || a.clustering_strength != b.clustering_strength
                || a.coexistence_duration != b.coexistence_duration
                || a.turnover_score != b.turnover_score
                || a.trophic_balance_score != b.trophic_balance_score,
            "different seeds should produce different trajectories \
             (a: tick={} fit={}, b: tick={} fit={})",
            result_a.termination_tick,
            result_a.fitness,
            result_b.termination_tick,
            result_b.fitness,
        );
    }

    #[test]
    fn ensemble_reproducible_with_same_base_seed() {
        let params = test_params();
        let distribution = test_distribution();
        let config = EnsembleConfig {
            ensemble_size: 3,
            run_config: RunConfig {
                max_ticks: 30,
                eval_config: EvalConfig::default(),
            },
        };

        let result1 = run_ensemble(&params, &distribution, &config, 99);
        let result2 = run_ensemble(&params, &distribution, &config, 99);

        assert_eq!(result1.median_fitness, result2.median_fitness);
        assert_eq!(result1.run_results.len(), result2.run_results.len());
        for (r1, r2) in result1.run_results.iter().zip(result2.run_results.iter()) {
            assert_eq!(r1.fitness, r2.fitness);
            assert_eq!(r1.termination_tick, r2.termination_tick);
        }
    }

    #[test]
    fn parallel_ensemble_matches_sequential_reference() {
        // Central correctness property of issue #350: parallelising the outer
        // (seed) loop with rayon must be bit-identical to a sequential run.
        // Seeds are independent, so an order-stable indexed collect keeps every
        // per-seed result in seed order and unchanged.
        let params = WorldParameters {
            initial_population_size: 20,
            contact_range_coefficient: 8.0,
            reproduction_energy_threshold: 15.0,
            world_extent: 30.0,
            solar_flux_magnitude: 8.0,
            growth_efficiency: 0.5,
            offspring_structure_fraction: 0.0,
            ..test_params()
        };
        let distribution = InitialDistribution {
            trait_covariance: 0.5,
            initial_energy_per_agent: 60.0,
            ..test_distribution()
        };
        let config = EnsembleConfig {
            ensemble_size: 8,
            run_config: RunConfig {
                max_ticks: 120,
                eval_config: EvalConfig::default(),
            },
        };
        let base_seed: u64 = 7;

        // Sequential reference, computed independently of run_ensemble.
        let sequential: Vec<RunResult> = (0..config.ensemble_size)
            .map(|i| {
                let seed = base_seed.wrapping_add(i as u64);
                run_single(&params, &distribution, &config.run_config, seed)
            })
            .collect();
        let sequential_median = median(&sequential.iter().map(|r| r.fitness).collect::<Vec<_>>());

        // The (now parallel) run_ensemble.
        let parallel = run_ensemble(&params, &distribution, &config, base_seed);

        assert_eq!(parallel.median_fitness, sequential_median);
        assert_eq!(parallel.run_results.len(), sequential.len());
        for (p, s) in parallel.run_results.iter().zip(sequential.iter()) {
            assert_eq!(p.fitness, s.fitness);
            assert_eq!(p.termination_tick, s.termination_tick);
            assert_eq!(p.failure, s.failure);
        }
    }

    #[test]
    fn ensemble_all_degenerate_returns_zero_fitness() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 100.0,
            ..test_params()
        };
        let distribution = InitialDistribution {
            initial_energy_per_agent: 1.0,
            ..test_distribution()
        };
        let config = EnsembleConfig {
            ensemble_size: 5,
            run_config: RunConfig {
                max_ticks: 1000,
                eval_config: EvalConfig::default(),
            },
        };

        let result = run_ensemble(&params, &distribution, &config, 42);

        assert_eq!(result.median_fitness, 0.0);
        assert_eq!(result.run_results.len(), 5);
        for run in &result.run_results {
            assert_eq!(run.fitness, 0.0);
            assert!(run.failure.is_some());
        }
    }

    #[test]
    fn same_seed_produces_identical_results() {
        let params = test_params();
        let distribution = test_distribution();
        let config = RunConfig {
            max_ticks: 50,
            eval_config: EvalConfig::default(),
        };

        let result1 = run_single(&params, &distribution, &config, 123);
        let result2 = run_single(&params, &distribution, &config, 123);

        assert_eq!(result1.fitness, result2.fitness);
        assert_eq!(result1.termination_tick, result2.termination_tick);
        assert_eq!(result1.failure, result2.failure);
    }

    #[test]
    fn single_run_terminates_early_on_extinction() {
        let params = WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 100.0,
            ..test_params()
        };
        let distribution = InitialDistribution {
            initial_energy_per_agent: 1.0,
            ..test_distribution()
        };
        let config = RunConfig {
            max_ticks: 1000,
            eval_config: EvalConfig::default(),
        };

        let result = run_single(&params, &distribution, &config, 42);

        assert!(result.termination_tick < 1000);
        assert_eq!(result.failure, Some(FailureMode::Extinction));
        assert_eq!(result.fitness, 0.0);
    }

    #[test]
    fn single_run_completes_at_max_ticks_when_no_failure() {
        let params = WorldParameters {
            reproduction_energy_threshold: 500.0, // prevent reproduction-related death
            contact_range_coefficient: 5.0,
            solar_flux_magnitude: 10.0,
            base_metabolic_rate: 0.01,
            growth_efficiency: 0.5,
            ..test_params()
        };
        let distribution = InitialDistribution {
            initial_energy_per_agent: 50.0,
            trait_covariance: 0.5,
            ..test_distribution()
        };
        let config = RunConfig {
            max_ticks: 20,
            eval_config: EvalConfig {
                grace_period_fraction: 1.0,
                ..EvalConfig::default()
            },
        };

        let result = run_single(&params, &distribution, &config, 42);

        assert_eq!(result.termination_tick, 20);
        assert!(result.failure.is_none());
    }
}
