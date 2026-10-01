//! Founders bind their seed structure's nutrient from the available pool at
//! world creation, and no cell of free nutrient is ever driven negative doing
//! so (#612). See `docs/system-design/world-rules.md`, "Founders bind nutrient
//! at world creation".

mod support;

use explorers_sim::{InitialDistribution, TraitVector, World, WorldParameters, WorldRecipe};

fn load_recipe(name: &str) -> WorldRecipe {
    let path = format!("{}/../../scenarios/{}", env!("CARGO_MANIFEST_DIR"), name);
    let contents = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_json::from_str(&contents).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

fn founding_world(pool: f32, population: u32, aggregation: f32, clusters: u32, seed: u64) -> World {
    let params = WorldParameters {
        world_extent: 100.0,
        initial_population_size: population,
        initial_nutrient_pool: pool,
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

fn lowest_cell(world: &World) -> f32 {
    world
        .nutrient_grid()
        .cells()
        .iter()
        .copied()
        .fold(f32::INFINITY, f32::min)
}

/// Total nutrient at creation: the free pool plus every founder's nutrient.
fn total_nutrient(world: &World) -> f32 {
    let living: f32 = world
        .agents()
        .iter()
        .map(|a| a.nutrient_total(world.params()))
        .sum();
    world.nutrient_pool() + living
}

/// Would binding every founder's nutrient in its own cell, as the binding did
/// before #612, have driven a cell negative?
fn in_place_binding_goes_negative(world: &World) -> bool {
    let params = world.params();
    let mut grid = explorers_sim::spatial::NutrientGrid::new(
        params.world_extent,
        params.nutrient_grid_cell_size,
        params.initial_nutrient_pool,
    );
    for a in world.agents() {
        *grid.at_position(a.position) -= a.bound_nutrient(params);
    }
    grid.cells().iter().any(|&c| c < 0.0)
}

/// A tight patch of many founders on a small pool: their bound nutrient
/// exceeds what the few cells under the patch hold, though the pool as a whole
/// covers it. No cell may go negative, and nutrient at creation is the pool.
#[test]
fn a_tight_patch_on_a_small_pool_drives_no_cell_negative() {
    let world = founding_world(100.0, 40, 0.95, 1, 3);
    assert!(
        in_place_binding_goes_negative(&world),
        "the reproduction needs a patch its own cells cannot cover"
    );
    let low = lowest_cell(&world);
    assert!(low >= 0.0, "a cell holds negative free nutrient: {low}");
    let total = total_nutrient(&world);
    assert!(
        (total - 100.0).abs() <= 1e-3,
        "nutrient at creation {total} must equal the pool 100"
    );
}

/// FNV-1a over every founder's state and every nutrient cell at creation,
/// bit-exact.
fn creation_fingerprint(world: &World) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |x: u64| {
        for b in x.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    eat(world.agents().len() as u64);
    for a in world.agents() {
        eat(a.id);
        for f in [
            a.position.0,
            a.position.1,
            a.reserve,
            a.structure,
            a.nutrient,
            a.repro_reserve,
            a.repro_nutrient,
        ] {
            eat(f.to_bits() as u64);
        }
    }
    eat(world.dissipated_energy().to_bits() as u64);
    for &c in world.nutrient_grid().cells() {
        eat(c.to_bits() as u64);
    }
    h
}

/// The worlds the regression pin covers: every committed scenario, and
/// generated foundings across the aggregation range on the baseline pool.
fn pinned_worlds() -> Vec<(String, World)> {
    let mut worlds = Vec::new();
    let mut names: Vec<String> =
        std::fs::read_dir(format!("{}/../../scenarios", env!("CARGO_MANIFEST_DIR")))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.starts_with("example") && n.ends_with(".json"))
            .collect();
    names.sort();
    for name in names {
        let world = World::from_recipe(&load_recipe(&name), 7);
        worlds.push((name, world));
    }
    for aggregation in [0.0, 0.8, 0.95] {
        for clusters in [1, 3] {
            let world = founding_world(50000.0, 40, aggregation, clusters, 11);
            worlds.push((format!("a={aggregation} k={clusters}"), world));
        }
    }
    worlds
}

/// Creation fingerprints of [`pinned_worlds`], taken from the in-place binding
/// before #612 (main at c6fe53a). None of these worlds drove a cell negative,
/// so the floor must leave every one of them bit-identical.
const PINNED: [u64; 18] = [
    0x5bf80459e62185c7, // example1.json
    0xe5346176bd0f8c22, // example10_predator_prey_hopf.json
    0x01d5704faf8eb1ab, // example11_branching_coexistence.json
    0xd0c4e55ba33bed1f, // example12_generalist_dominance.json
    0xab24701ec724817d, // example13_closed_web.json
    0x1653a01de64c3472, // example2.json
    0xfa22bef0890d172d, // example3.json
    0xe7319027725e1962, // example4.json
    0xe668c96ece83d23c, // example5.json
    0x65bef6b1ad321cae, // example7.json
    0x8af86598faf12048, // example8.json
    0x41a124a5cef704e3, // example9_detrital_pathway.json
    0x9587ded5b3dc151a, // a=0 k=1
    0xe1d3e3254a2ee77e, // a=0 k=3
    0xa047f97bfbf11d65, // a=0.8 k=1
    0x3bfe0d356bc8b083, // a=0.8 k=3
    0x8443f1ac3d00ca43, // a=0.95 k=1
    0xdaa8572696b5b7d4, // a=0.95 k=3
];

/// Prints [`PINNED`] in source form. Ignored: run by hand only to re-pin.
#[test]
#[ignore]
fn print_pinned() {
    for (name, world) in pinned_worlds() {
        println!("{:#018x}, // {name}", creation_fingerprint(&world));
    }
}

/// Where every founder's own cell covers its draw, binding is unchanged: worlds
/// that never went negative are bit-identical to before the floor.
#[test]
fn worlds_whose_cells_cover_their_founders_bind_exactly_as_before() {
    let worlds = pinned_worlds();
    assert_eq!(worlds.len(), PINNED.len());
    for ((name, world), pinned) in worlds.iter().zip(PINNED) {
        assert!(
            !in_place_binding_goes_negative(world),
            "{name}: a pinned world must be one whose cells covered its founders"
        );
        let got = creation_fingerprint(world);
        assert_eq!(got, pinned, "{name}: {got:#018x} != pinned {pinned:#018x}");
    }
}

/// A draw its own cell cannot cover takes the shortfall from the nearest cells
/// first: the eight cells around it (wrapping over the torus edge) give before
/// any farther cell is touched, and nothing goes negative or is lost.
#[test]
fn a_shortfall_is_drawn_from_the_nearest_cells_outward() {
    use explorers_sim::spatial::NutrientGrid;
    // 5 × 5 cells of 10 nutrient each; the corner cell (0, 0) sits at
    // (-20, -20) and its ring-1 neighbours wrap to columns and rows 4.
    let mut grid = NutrientGrid::new(50.0, 10.0, 250.0);
    let corner = (-20.0, -20.0);
    let undrawn = grid.draw_nearest(corner, 50.0);
    assert_eq!(undrawn, 0.0);
    let cells = grid.cells();
    let ring = |idx: usize| {
        let (col, row) = ((idx % 5) as i32, (idx / 5) as i32);
        let d = |a: i32| a.min(5 - a);
        d(col).max(d(row))
    };
    assert_eq!(cells[0], 0.0, "the founder's own cell gives first");
    let ring1: f32 = (0..25).filter(|&i| ring(i) == 1).map(|i| cells[i]).sum();
    assert_eq!(ring1, 80.0 - 40.0, "the ring around it gives the other 40");
    for i in (0..25).filter(|&i| ring(i) == 2) {
        assert_eq!(cells[i], 10.0, "cell {i}, two rings out, is untouched");
    }
    assert!(cells.iter().all(|&c| c >= 0.0));
    assert_eq!(grid.total(), 200.0);
}

/// A pool too small for every founder's seed body: founding is co-limited by
/// nutrient as growth is. Founders build only the structure the whole pool can
/// bind, the energy that structure would have taken stays in their reserve,
/// and the pool is bound out exactly — no cell negative, no nutrient conjured,
/// no energy lost.
#[test]
fn a_pool_too_small_for_the_founders_bodies_co_limits_their_structure() {
    let pool = 5.0;
    let roomy = founding_world(50000.0, 40, 0.95, 1, 3);
    let bound_wanted: f32 = roomy
        .agents()
        .iter()
        .map(|a| a.bound_nutrient(roomy.params()))
        .sum();
    assert!(
        bound_wanted > pool,
        "the pool must be short: {bound_wanted}"
    );

    let world = founding_world(pool, 40, 0.95, 1, 3);
    let low = lowest_cell(&world);
    assert!(low >= 0.0, "a cell holds negative free nutrient: {low}");
    let total = total_nutrient(&world);
    assert!(
        (total - pool).abs() <= 1e-4,
        "nutrient at creation {total} must equal the pool {pool}"
    );
    assert!(
        world.nutrient_pool() <= 1e-4,
        "the founders bind out the whole pool, leaving {}",
        world.nutrient_pool()
    );
    for (short, full) in world.agents().iter().zip(roomy.agents()) {
        assert!(
            short.structure > 0.0 && short.structure < full.structure,
            "founder {} must be embodied, on a smaller body: {} vs {}",
            short.id,
            short.structure,
            full.structure
        );
        assert_eq!(short.peak_structure, short.structure);
        assert!(
            short.reserve > full.reserve,
            "unbuilt energy stays in reserve"
        );
    }
    let endowment = 20.0 * 40.0;
    let held: f32 = world.agents().iter().map(|a| a.energy()).sum();
    let accounted = held + world.dissipated_energy();
    assert!(
        (accounted - endowment).abs() <= endowment * 1e-5,
        "founding energy {accounted} must equal the endowment {endowment}"
    );
}

/// A recipe's hand-placed founders are co-limited the same way: on a pool too
/// small for their seed bodies they build what the pool can bind, and the
/// pool's nutrient all ends up bound, none negative and none conjured.
#[test]
fn a_recipe_roster_on_a_pool_too_small_is_co_limited_too() {
    let recipe = load_recipe("example4.json");
    let roomy = World::from_recipe(&recipe, 7);
    let bound_wanted: f32 = roomy
        .agents()
        .iter()
        .map(|a| a.bound_nutrient(roomy.params()))
        .sum();
    assert!(bound_wanted > 0.0);
    let pool = bound_wanted / 4.0;
    let mut short = recipe.clone();
    short.parameters.initial_nutrient_pool = pool;
    let world = World::from_recipe(&short, 7);
    let low = lowest_cell(&world);
    assert!(low >= 0.0, "a cell holds negative free nutrient: {low}");
    let bound: f32 = world
        .agents()
        .iter()
        .map(|a| a.bound_nutrient(world.params()))
        .sum();
    assert!(
        (bound + world.nutrient_pool() - pool).abs() <= pool * 1e-5,
        "bound {bound} + free {} must equal the pool {pool}",
        world.nutrient_pool()
    );
    let energy = |w: &World| -> f32 {
        w.agents().iter().map(|a| a.energy()).sum::<f32>() + w.dissipated_energy()
    };
    assert!(
        (energy(&world) - energy(&roomy)).abs() <= energy(&roomy) * 1e-5,
        "co-limiting moves energy from structure to reserve, losing none"
    );
}
