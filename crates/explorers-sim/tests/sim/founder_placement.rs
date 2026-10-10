//! Founder placement (#601): founders are seeded as spatially aggregated
//! patches, one per founding cluster, with the **founder aggregation** scalar
//! of the `InitialDistribution` setting how tightly — from tight, separate
//! patches (towards 1) to today's well-mixed scatter over the whole torus (0).
//! See `docs/system-design/expected-properties.md`, "Founder placement".

use crate::support;

use explorers_sim::{
    Agent, DEFAULT_FOUNDER_AGGREGATION, InitialDistribution, TraitVector, World, WorldParameters,
    WorldRecipe, spatial::NutrientGrid, toroidal_displacement, toroidal_distance,
};

/// A recipe written before founder aggregation existed omits the field; it
/// reads back at the aggregated design default.
#[test]
fn a_recipe_omitting_founder_aggregation_reads_back_at_the_aggregated_default() {
    let json = r#"{
        "parameters": PARAMS,
        "initial_distribution": {
            "mean_traits": {
                "photosynthetic_absorption": 0.5, "heterotrophy": 0.0, "mobility": 0.1,
                "kappa": 0.5, "fecundity": 0.35, "asexual_propensity": 0.5, "dispersal": 0.5
            },
            "trait_covariance": 0.1,
            "initial_cluster_count": 2,
            "initial_energy_per_agent": 20.0
        },
        "max_ticks": 10
    }"#
    .replace(
        "PARAMS",
        &serde_json::to_string(&support::viable_baseline()).unwrap(),
    );
    let recipe: WorldRecipe = serde_json::from_str(&json).unwrap();
    let dist: InitialDistribution = recipe.initial_distribution.unwrap();
    assert_eq!(dist.founder_aggregation, DEFAULT_FOUNDER_AGGREGATION);
    assert!(
        DEFAULT_FOUNDER_AGGREGATION > 0.0,
        "the design default is aggregated, not well-mixed"
    );
}

fn founding_world(aggregation: f32, clusters: u32, seed: u64) -> World {
    founding_world_of(40, aggregation, clusters, seed)
}

fn founding_world_of(population: u32, aggregation: f32, clusters: u32, seed: u64) -> World {
    let params = WorldParameters {
        world_extent: 100.0,
        initial_population_size: population,
        ..support::viable_baseline()
    };
    let dist = InitialDistribution {
        mean_traits: TraitVector {
            photosynthetic_absorption: 0.5,
            heterotrophy: 0.3,
            mobility: 0.2,
            kappa: 0.5,
            fecundity: 0.35,
            asexual_propensity: 0.5,
            dispersal: 0.5,
        },
        trait_covariance: 0.1,
        initial_cluster_count: clusters,
        initial_energy_per_agent: 20.0,
        founder_aggregation: aggregation,
    };
    World::new(params, dist, seed)
}

/// Founders are assigned to clusters round-robin by id (`World::new`'s seeding
/// rule), so a founder's cluster is its id modulo the cluster count.
fn cluster_of(agent: &Agent, clusters: u32) -> u64 {
    agent.id % clusters as u64
}

/// At tight aggregation every cluster founds its own patch: any two founders of
/// different clusters sit farther apart than any two founders of the same one.
#[test]
fn at_tight_aggregation_founders_of_different_clusters_sit_in_separate_patches() {
    for clusters in 2..=5 {
        for seed in 0..8 {
            let world = founding_world(0.9, clusters, seed);
            let extent = world.params().world_extent;
            let agents = world.agents();
            let mut widest_within = 0.0_f32;
            let mut nearest_between = f32::INFINITY;
            for (i, a) in agents.iter().enumerate() {
                for b in &agents[i + 1..] {
                    let d = toroidal_distance(a.position, b.position, extent);
                    if cluster_of(a, clusters) == cluster_of(b, clusters) {
                        widest_within = widest_within.max(d);
                    } else {
                        nearest_between = nearest_between.min(d);
                    }
                }
            }
            assert!(
                nearest_between > widest_within,
                "clusters={clusters} seed={seed}: nearest founders of different clusters \
                 ({nearest_between}) must sit farther apart than the widest patch ({widest_within})"
            );
        }
    }
}

/// The well-mixed end is today's uniform scatter over the whole torus: one
/// placement shared by every cluster count (clusters do not own patches), and
/// founders spread across every region of the world.
#[test]
fn at_the_well_mixed_end_placement_is_the_uniform_scatter() {
    let seed = 7;
    let reference: Vec<(f32, f32)> = founding_world(0.0, 1, seed)
        .agents()
        .iter()
        .map(|a| a.position)
        .collect();
    for clusters in 2..=5 {
        let placed: Vec<(f32, f32)> = founding_world(0.0, clusters, seed)
            .agents()
            .iter()
            .map(|a| a.position)
            .collect();
        assert_eq!(
            placed, reference,
            "well-mixed placement must not depend on the cluster count ({clusters})"
        );
    }
    // Uniform over the torus: every quadrant of the world holds founders from
    // every cluster, which no tight patch can do (a large founding keeps an empty
    // quadrant from being a chance draw).
    let world = founding_world_of(400, 0.0, 2, seed);
    for cluster in 0..2 {
        let mut quadrants = [0usize; 4];
        for a in world
            .agents()
            .iter()
            .filter(|a| cluster_of(a, 2) == cluster)
        {
            let q = (a.position.0 >= 0.0) as usize + 2 * (a.position.1 >= 0.0) as usize;
            quadrants[q] += 1;
        }
        assert!(
            quadrants.iter().all(|&n| n > 0),
            "cluster {cluster} must scatter over the whole torus: {quadrants:?}"
        );
    }
}

/// Aggregation sets how tightly: each cluster's founders fit within a patch of
/// side `(1 − aggregation) × extent`, on both axes.
#[test]
fn each_cluster_founds_within_a_patch_that_shrinks_with_aggregation() {
    for aggregation in [0.3, 0.5, 0.8, 0.95] {
        for seed in 0..4 {
            let clusters = 3;
            let world = founding_world(aggregation, clusters, seed);
            let extent = world.params().world_extent;
            let side = (1.0 - aggregation) * extent;
            let agents = world.agents();
            for a in agents {
                for b in agents
                    .iter()
                    .filter(|b| cluster_of(b, clusters) == cluster_of(a, clusters))
                {
                    let (dx, dy) = toroidal_displacement(a.position, b.position, extent);
                    assert!(
                        dx.abs() <= side + 1e-3 && dy.abs() <= side + 1e-3,
                        "aggregation={aggregation} seed={seed}: founders {} and {} are \
                         ({dx}, {dy}) apart, outside a patch of side {side}",
                        a.id,
                        b.id
                    );
                }
            }
        }
    }
}

/// Placement is a pure function of the world seed: the same seed founds the
/// same patches, and a different seed founds different ones.
#[test]
fn founding_patches_are_deterministic_per_seed() {
    let positions = |seed| -> Vec<(f32, f32)> {
        founding_world(DEFAULT_FOUNDER_AGGREGATION, 3, seed)
            .agents()
            .iter()
            .map(|a| a.position)
            .collect()
    };
    assert_eq!(positions(11), positions(11), "same seed, same founding");
    assert_ne!(positions(11), positions(12), "another seed, other patches");
}

/// Seeded structure still binds nutrient drawn from the pool at each founder's
/// own location, so the tight patches deplete their cells, the rest of the
/// world keeps its uniform share, and total nutrient at creation is the pool.
#[test]
fn aggregated_founders_bind_nutrient_from_the_pool_where_they_stand() {
    let mut world = founding_world(0.9, 3, 5);
    let params = world.params().clone();
    let mut expected = NutrientGrid::new(
        params.world_extent,
        params.nutrient_grid_cell_size,
        params.initial_nutrient_pool,
    );
    for a in world.agents() {
        *expected.at_position(a.position) -= a.bound_nutrient(&params);
    }
    let cols = (params.world_extent / params.nutrient_grid_cell_size).ceil() as usize;
    for cell in 0..cols * cols {
        let got = *world.nutrient_grid_mut().cell_mut(cell);
        let want = *expected.cell_mut(cell);
        assert!(
            (got - want).abs() <= 1e-3,
            "cell {cell}: pool {got}, expected {want} after founders bind in place"
        );
    }
    let living: f32 = world
        .agents()
        .iter()
        .map(|a| a.nutrient_total(&params))
        .sum();
    let total = world.nutrient_pool() + living;
    assert!(
        (total - params.initial_nutrient_pool).abs() <= params.initial_nutrient_pool * 1e-5,
        "nutrient at creation {total} must equal the initial pool {}",
        params.initial_nutrient_pool
    );
}
