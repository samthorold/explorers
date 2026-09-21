pub use explorers_genesis_eval::{EvalConfig, FailureMode, FitnessBreakdown};
pub use explorers_sim::{InitialDistribution, WorldParameters};
use rayon::prelude::*;

pub struct RunConfig {
    pub max_ticks: u64,
    pub eval_config: EvalConfig,
    /// Fraction of rollouts an incremental dead-pool gate stops (energy
    /// death, nutrient lockup) that are carried to the horizon anyway and
    /// re-verdicted on the full series, in `[0, 1]`. The gates as defined are
    /// not proven irreversible — a world flagged at tick 600 might recover by
    /// `T` — so the carry is the falsification interlock the prefilter
    /// cross-check already is at tick 0, moved along the trajectory
    /// (genesis-search.md). A carried run's own verdict is still the gate's;
    /// the horizon read is *recorded* on [`RunResult::early_stop`] for the
    /// search to surface as a disagreement, never swallowed and never a
    /// cell. The draw is seeded from the rollout seed
    /// ([`crosscheck_selected`]) so it is reproducible. 0 disables the carry.
    pub early_stop_crosscheck_fraction: f32,
}

pub struct RunResult {
    pub fitness: f32,
    pub failure: Option<FailureMode>,
    pub termination_tick: u64,
    pub breakdown: FitnessBreakdown,
    /// Set when an incremental dead-pool gate stopped the rollout before the
    /// horizon (extinction and explosion stop it too, but are terminal by
    /// definition and are not recorded here). Carries the horizon verdict
    /// when the rollout was drawn into the carry-to-`T` cross-check.
    pub early_stop: Option<EarlyStop>,
}

/// A rollout stopped where a dead-pool gate fired (#506).
#[derive(Clone, Debug, PartialEq)]
pub struct EarlyStop {
    /// The gate that fired: `EnergyDeath` or `NutrientLockup`.
    pub failure: FailureMode,
    /// The tick it fired on — the run's termination tick and the frontier
    /// entry's `ticks_survived`.
    pub tick: u64,
    /// The full-series verdict, present only when the rollout was drawn into
    /// the cross-check and carried to the horizon.
    pub horizon: Option<HorizonVerdict>,
}

/// What a carried rollout read at the horizon.
#[derive(Clone, Debug, PartialEq)]
pub struct HorizonVerdict {
    pub failure: Option<FailureMode>,
    pub fitness: f32,
    /// Where the carried run actually ended: the horizon, or earlier if it
    /// went extinct or exploded on the way.
    pub termination_tick: u64,
}

impl EarlyStop {
    /// The cross-check disagreement, if any: the gate said dead but the
    /// carried run is *alive* at the horizon (no failure on the full series).
    /// A carried run that dies of something else by `T` still agrees with
    /// the gate's claim that the world was dead.
    pub fn disagreement(&self) -> Option<&HorizonVerdict> {
        self.horizon.as_ref().filter(|h| h.failure.is_none())
    }
}

/// Whether the rollout on `seed` is drawn into the carry-to-horizon
/// cross-check at `fraction`: a deterministic coin on the seed alone
/// (a SplitMix64 mix of it against the unit interval), so the draw is
/// reproducible across runs and independent of evaluation order.
pub fn crosscheck_selected(seed: u64, fraction: f32) -> bool {
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction <= 0.0 {
        return false;
    }
    if fraction >= 1.0 {
        return true;
    }
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    let unit = (z >> 40) as f64 / (1u64 << 24) as f64;
    unit < fraction as f64
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
    // The log keeps only what the evaluator reads (#502): the retention list
    // and the audit of the reads behind it live with the evaluator
    // (`EVALUATOR_EVENT_KINDS`). Observer-side — no trajectory changes.
    world.retain_event_kinds(explorers_genesis_eval::EVALUATOR_EVENT_KINDS);

    // Per-tick series the rollout observes for the descriptors that need a
    // temporal trace: free energy (issue #302), carcass fraction (#342) and
    // producer share (#392) sampled every tick, plus a coarse-interval trait-
    // vector snapshot for coexistence (#394) and a role-classified roster
    // snapshot for the heterotroph guild read (#490), and the turnover counts
    // and descent facts read off the log tail. The world stays history-free —
    // the series live here, bundled as `RolloutObservations` for the
    // evaluator, and the log is dropped as soon as `observe` has read it, so
    // a settled-community horizon (`T = 2000`) fits in memory on dense
    // configs. The snapshots are pure observation; genesis does NOT cluster,
    // the evaluator runs DBSCAN on each.
    let mut observations =
        explorers_genesis_eval::RolloutObservations::with_capacity(run_config.max_ticks as usize);
    let interval = run_config.eval_config.coexistence_sample_interval;
    let eval_config = &run_config.eval_config;
    // The rollout stops where it dies (genesis-search.md, *The frontier costs
    // a bloom, the atlas costs the horizon*): the evaluator's incremental
    // terminal check reads extinction and explosion every tick and the two
    // dead-pool gates on the series-so-far at the window cadence. A stop is
    // a dead-frontier entry, never a cell — the gated breakdown is built
    // here, nothing is scored.
    //
    // A sampled fraction of dead-pool stops is carried to the horizon anyway
    // (the cross-check): the gate's verdict is kept as the run's, the
    // full-series verdict is recorded beside it.
    let carry = crosscheck_selected(seed, run_config.early_stop_crosscheck_fraction);
    // The energy-death reference is a property of the config (#508),
    // computed once per rollout.
    let sustainable_stock = explorers_genesis_eval::sustainable_stock(params);
    let mut stopped: Option<EarlyStop> = None;
    for _ in 0..run_config.max_ticks {
        world.step();
        observations.observe(&world, interval);
        world.compact_event_log_before(observations.consumed_events());
        match explorers_genesis_eval::early_stop(
            world.agents().len(),
            &observations,
            eval_config,
            sustainable_stock,
        ) {
            None => {}
            Some(FailureMode::Extinction) | Some(FailureMode::PopulationExplosion) => break,
            Some(failure) => {
                if stopped.is_none() {
                    stopped = Some(EarlyStop {
                        failure,
                        tick: world.tick(),
                        horizon: None,
                    });
                    if !carry {
                        break;
                    }
                }
            }
        }
    }

    let horizon_breakdown = || {
        explorers_genesis_eval::evaluate_from_log(
            &world,
            &observations,
            eval_config,
            run_config.max_ticks,
        )
    };
    let (breakdown, termination_tick, early_stop) = match stopped {
        Some(mut stop) => {
            if carry {
                let full = horizon_breakdown();
                stop.horizon = Some(HorizonVerdict {
                    failure: full.failure,
                    fitness: full.fitness,
                    termination_tick: world.tick(),
                });
            }
            let breakdown = FitnessBreakdown::gated(stop.failure.clone(), stop.tick);
            (breakdown, stop.tick, Some(stop))
        }
        None => (horizon_breakdown(), world.tick(), None),
    };
    RunResult {
        fitness: breakdown.fitness,
        failure: breakdown.failure.clone(),
        termination_tick,
        breakdown,
        early_stop,
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
        // A short grace (the 40 ticks this test was written under) and a
        // horizon whose settled window `(T/2, T]` still holds this small
        // world's seed-dependent dynamics: under a grace at or beyond max_ticks
        // the oscillation and coexistence guards zero those descriptors, these
        // worlds trip a gate under the default grace, and by tick 100 they have
        // frozen, so a longer horizon reads identical (all-zero) windows.
        let config = RunConfig {
            max_ticks: 120,
            eval_config: EvalConfig {
                grace_ticks: 40,
                ..EvalConfig::default()
            },
            early_stop_crosscheck_fraction: 0.0,
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
             (a: tick={} fit={} {:?}, b: tick={} fit={} {:?})",
            result_a.termination_tick,
            result_a.fitness,
            a.failure,
            result_b.termination_tick,
            result_b.fitness,
            b.failure,
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
                early_stop_crosscheck_fraction: 0.0,
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
                early_stop_crosscheck_fraction: 0.0,
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
                early_stop_crosscheck_fraction: 0.0,
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
            early_stop_crosscheck_fraction: 0.0,
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
            early_stop_crosscheck_fraction: 0.0,
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
                grace_ticks: u64::MAX,
                ..EvalConfig::default()
            },
            early_stop_crosscheck_fraction: 0.0,
        };

        let result = run_single(&params, &distribution, &config, 42);

        assert_eq!(result.termination_tick, 20);
        assert!(result.failure.is_none());
    }

    /// A world whose living stock is small against what its resource base
    /// could sustain: a light radius of 1 on a 100-wide torus gives a
    /// sustainable stock of `F·m² ≈ 20 000 E`, while its starved founders —
    /// base metabolism 0.9 against a flux of 1, provisioned at 5 E — hold
    /// under a thousand for the whole run (15 bodies at tick 600 on seed
    /// 3). Reads `EnergyDeath` from the first post-grace window on under a
    /// zero grace, and still at a 600-tick horizon.
    fn energy_dying_world() -> (WorldParameters, InitialDistribution) {
        (
            WorldParameters {
                solar_flux_magnitude: 1.0,
                base_metabolic_rate: 0.9,
                initial_population_size: 30,
                contact_range_coefficient: 10.0,
                world_extent: 100.0,
                light_competition_radius: 1.0,
                growth_efficiency: 0.5,
                ..test_params()
            },
            InitialDistribution {
                initial_energy_per_agent: 5.0,
                trait_covariance: 0.5,
                ..test_distribution()
            },
        )
    }

    #[test]
    fn single_run_stops_where_the_energy_death_gate_fires() {
        // The frontier costs a bloom, the atlas costs the horizon
        // (genesis-search.md): a rollout the dead-pool gate catches is
        // tallied and stopped where it dies, carrying the mode and the
        // termination tick as an extinction does — never scored.
        let (params, dist) = energy_dying_world();
        let config = RunConfig {
            max_ticks: 600,
            eval_config: EvalConfig {
                grace_ticks: 0,
                ..EvalConfig::default()
            },
            early_stop_crosscheck_fraction: 0.0,
        };
        let result = run_single(&params, &dist, &config, 3);
        assert_eq!(result.failure, Some(FailureMode::EnergyDeath));
        assert!(
            result.termination_tick < 600,
            "stopped where it died, not at the horizon (tick {})",
            result.termination_tick
        );
        assert_eq!(result.breakdown.ticks_survived, result.termination_tick);
        assert_eq!(result.fitness, 0.0);
    }

    #[test]
    fn crosscheck_carries_a_stopped_rollout_to_the_horizon_without_changing_its_verdict() {
        // The gates are not proven irreversible, so a configurable fraction of
        // early-stopped rollouts are carried to T anyway and re-verdicted on
        // the full series. The carry changes only what is *recorded* — the
        // run's own verdict stays the gate's, so the atlas is the same map
        // whether or not a rollout was drawn for the check.
        let (params, dist) = energy_dying_world();
        let config = |fraction: f32| RunConfig {
            max_ticks: 600,
            eval_config: EvalConfig {
                grace_ticks: 0,
                ..EvalConfig::default()
            },
            early_stop_crosscheck_fraction: fraction,
        };
        let stopped = run_single(&params, &dist, &config(0.0), 3);
        let carried = run_single(&params, &dist, &config(1.0), 3);

        let stop = stopped.early_stop.as_ref().expect("the gate stopped it");
        assert_eq!(stop.failure, FailureMode::EnergyDeath);
        assert_eq!(stop.tick, stopped.termination_tick);
        assert!(stop.horizon.is_none(), "not drawn: never carried");

        let carry = carried.early_stop.as_ref().expect("the gate still fired");
        assert_eq!(
            (carry.failure.clone(), carry.tick),
            (stop.failure.clone(), stop.tick)
        );
        let horizon = carry.horizon.as_ref().expect("drawn: carried to T");
        assert_eq!(horizon.termination_tick, 600);
        // This world stays dead: the horizon read agrees with the gate.
        assert_eq!(horizon.failure, Some(FailureMode::EnergyDeath));
        assert!(carry.disagreement().is_none());

        // The recorded verdict is the gate's either way.
        assert_eq!(carried.failure, stopped.failure);
        assert_eq!(carried.termination_tick, stopped.termination_tick);
        assert_eq!(carried.fitness, 0.0);
    }

    #[test]
    fn crosscheck_selection_is_a_deterministic_draw_on_the_rollout_seed() {
        // Seeded from the rollout seed, not the search rng: the same seed is
        // drawn the same way on every run, so search output stays
        // reproducible under rayon, and a fraction in (0, 1) draws some seeds
        // and not others.
        let drawn: Vec<bool> = (0..200u64)
            .map(|seed| crosscheck_selected(seed, 0.3))
            .collect();
        assert_eq!(
            drawn,
            (0..200u64)
                .map(|seed| crosscheck_selected(seed, 0.3))
                .collect::<Vec<_>>()
        );
        let n = drawn.iter().filter(|&&d| d).count();
        assert!((30..=90).contains(&n), "≈30 % of 200 seeds drawn, got {n}");
        assert!((0..200u64).all(|seed| !crosscheck_selected(seed, 0.0)));
        assert!((0..200u64).all(|seed| crosscheck_selected(seed, 1.0)));
    }
}
