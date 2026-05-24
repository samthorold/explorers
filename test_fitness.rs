use explorers_genesis::{run_ensemble, EnsembleConfig, RunConfig, WorldParameters, InitialDistribution, EvalConfig};
use explorers_sim::TraitVector;

fn main() {
    // Use mid-range parameters
    let params = WorldParameters {
        solar_flux_magnitude: 10.5,
        consumption_efficiency: 0.5,
        decomposition_efficiency: 0.5,
        reproduction_efficiency: 0.5,
        base_metabolic_rate: 0.505,
        movement_cost_coefficient: 0.0505,
        sensing_cost_coefficient: 0.0505,
        reproduction_energy_threshold: 27.5,
        mutation_rate: 0.255,
        mutation_magnitude: 0.255,
        contact_radius: 2.75,
        world_extent: 60.0,
        initial_population_size: 27,
        light_competition_radius: 10.0,
        photo_maintenance_cost: 0.01,
        consumption_maintenance_cost: 0.01,
        scavenging_maintenance_cost: 0.01,
    };

    let dist = InitialDistribution {
        mean_traits: TraitVector {
            photosynthetic_absorption: 0.5,
            consumption_rate: 0.5,
            scavenging_rate: 0.5,
            mobility: 0.5,
            chemotaxis_sensitivity: 0.5,
            mate_selectivity: 0.5,
            sensing_range: 5.25,
            reproductive_investment: 0.5,
        },
        trait_covariance: 0.255,
        initial_cluster_count: 3,
        initial_energy_per_agent: 25.5,
    };

    let config = EnsembleConfig {
        ensemble_size: 1,
        run_config: RunConfig {
            max_ticks: 100,
            eval_config: EvalConfig::default(),
        },
    };

    let result = run_ensemble(&params, &dist, &config, 42);
    println!("Median fitness: {:.4}", result.median_fitness);
    
    for (i, run) in result.run_results.iter().enumerate() {
        println!("Run {}: fitness={:.4}, failure={:?}, termination_tick={}", 
                 i, run.fitness, run.failure, run.termination_tick);
        println!("  Breakdown: oscillation={:.4}, clustering={:.4}, coexistence={:.4}", 
                 run.breakdown.oscillation_strength, run.breakdown.clustering_strength, 
                 run.breakdown.coexistence_duration);
        println!("  Sanity check: {:?}", run.breakdown.sanity_check_failed);
    }
}
