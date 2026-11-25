//! Hyphal uptake through the stepper (#727, #860; world-rules.md flow 2,
//! *Hyphal uptake* and *Substrate contact is one predicate*).
//!
//! Hyphal uptake is physics in every world since #860, with no switch. The
//! phase unit tests pin the demand rule; these pin what only the stepper can
//! show: S27's rung (a still heterotroph takes up pool nutrient in proportion
//! to its effective heterotrophy, a roving one next to none, and roots and
//! hyphae share a depleted cell in proportion to demand), that uptake reads
//! the distance the move phase actually moved, and that nutrient stays
//! conserved.
//!
//! Run with:
//!   cargo test -p explorers-sim --test sim hyphal_uptake::

use crate::support;

use explorers_sim::event::EventKind;
use explorers_sim::units::{AUTOTROPHY_NUTRIENT_UPTAKE_PER_TICK, MOBILITY_DISTANCE_PER_TICK};
use explorers_sim::{Agent, TraitVector, World, WorldParameters, WorldRecipe};

fn heterotroph(heterotrophy: f32, mobility: f32) -> TraitVector {
    TraitVector {
        photosynthetic_absorption: 0.0,
        heterotrophy,
        mobility,
        kappa: 0.5,
        fecundity: 0.0,
        asexual_propensity: 0.0,
        dispersal: 0.0,
    }
}

fn producer(autotrophy: f32) -> TraitVector {
    TraitVector {
        photosynthetic_absorption: autotrophy,
        heterotrophy: 0.0,
        ..heterotroph(0.0, 0.0)
    }
}

/// A still world of lone heterotrophs at the world's default physics: no
/// mutation, no wear, so each agent's effective traits are its raw traits and
/// a mobile agent's stride is `mobility × u_M` every tick.
fn lone_heterotroph_params() -> WorldParameters {
    WorldParameters {
        mutation_rate: 0.0,
        mutation_magnitude: 0.0,
        ..support::viable_baseline()
    }
}

fn world_with(params: WorldParameters, agents: &[Agent]) -> World {
    let mut world = World::from_recipe(
        &WorldRecipe {
            parameters: params,
            agents: Some(vec![]),
            carcasses: None,
            max_ticks: 10,
            initial_distribution: None,
        },
        7,
    );
    for a in agents {
        world.add_agent(a.clone());
    }
    world
}

/// Pool uptake per agent id on `tick`, from the event log.
fn absorbed_on(world: &World, tick: u64, id: u64) -> f32 {
    world
        .event_log()
        .since(0)
        .iter()
        .filter(|e| e.kind == EventKind::NutrientAbsorbed && e.tick == tick && e.source == id)
        .map(|e| e.energy_delta)
        .sum()
}

/// S27 (end-state.md part 1), exact, on a few agents:
/// - a still heterotroph on an ample cell takes up `effective heterotrophy ×
///   u_A` a tick, so two of different heterotrophy take up in its ratio;
/// - a roving heterotroph at a full stride (`u_M = 10 d_c`) takes up `e^−10`
///   of what a still one of the same heterotrophy does, once it has moved:
///   next to none;
/// - a producer's roots and a still heterotroph's hyphae on one depleted cell
///   share it in proportion to demand, and leave it at zero, never below.
#[test]
fn s27_rung_a_still_heterotroph_takes_up_pool_nutrient_by_heterotrophy_a_roving_one_next_to_none_and_roots_and_hyphae_share_by_demand()
 {
    // Still and roving, on an ample pool, each in its own cell.
    let params = lone_heterotroph_params();
    assert_eq!(
        MOBILITY_DISTANCE_PER_TICK,
        10.0 * params.contact_distance,
        "a full stride is ten contact distances"
    );
    let low = Agent::new(0, (-35.0, -35.0), 100.0, 3.0, 5.0, heterotroph(0.25, 0.0));
    let high = Agent::new(0, (35.0, -35.0), 100.0, 3.0, 5.0, heterotroph(0.5, 0.0));
    let rover = Agent::new(0, (35.0, 35.0), 100.0, 3.0, 5.0, heterotroph(0.5, 1.0));
    let mut world = world_with(params, &[low, high, rover]);
    let ids: Vec<u64> = world.agents().iter().map(|a| a.id).collect();
    let first = world.tick();
    world.step();
    world.step();

    let u_a = AUTOTROPHY_NUTRIENT_UPTAKE_PER_TICK;
    let low_got = absorbed_on(&world, first, ids[0]);
    let high_got = absorbed_on(&world, first, ids[1]);
    assert!(
        (low_got - 0.25 * u_a).abs() < 1e-6,
        "still heterotroph of 0.25 took up {low_got}, want {}",
        0.25 * u_a
    );
    assert!(
        (high_got - 0.5 * u_a).abs() < 1e-6,
        "still heterotroph of 0.5 took up {high_got}, want {}",
        0.5 * u_a
    );
    let roved = absorbed_on(&world, first + 1, ids[2]);
    let still = absorbed_on(&world, first + 1, ids[1]);
    let want = (-10.0_f32).exp() * still;
    assert!(
        (roved - want).abs() <= want * 1e-3,
        "rover took up {roved} after a full stride, want e^-10 × {still} = {want}"
    );
    assert!(
        roved < 1e-4 * still,
        "a roving heterotroph takes up next to none"
    );

    // Roots and hyphae on one depleted cell: 0.09 a cell against a demand of
    // 0.6 + 0.3.
    let depleted = WorldParameters {
        initial_nutrient_pool: 0.09 * 100.0,
        ..lone_heterotroph_params()
    };
    let cell_pool = 0.09_f32;
    let roots = Agent::new(0, (-25.0, -25.0), 100.0, 3.0, 5.0, producer(0.6));
    let hyphae = Agent::new(0, (-24.0, -24.0), 100.0, 3.0, 5.0, heterotroph(0.3, 0.0));
    let mut world = world_with(depleted, &[roots, hyphae]);
    let cell = world.nutrient_grid().cell_index_for((-25.0, -25.0));
    assert_eq!(cell, world.nutrient_grid().cell_index_for((-24.0, -24.0)));
    assert!((world.nutrient_grid().cells()[cell] - cell_pool).abs() < 1e-6);
    let ids: Vec<u64> = world.agents().iter().map(|a| a.id).collect();
    let first = world.tick();
    world.step();
    let root_share = absorbed_on(&world, first, ids[0]);
    let hyphal_share = absorbed_on(&world, first, ids[1]);
    assert!(
        (root_share - cell_pool * 0.6 / 0.9).abs() < 1e-6,
        "roots took {root_share} of {cell_pool}, want two thirds"
    );
    assert!(
        (hyphal_share - cell_pool * 0.3 / 0.9).abs() < 1e-6,
        "hyphae took {hyphal_share} of {cell_pool}, want a third"
    );
    assert!(
        root_share + hyphal_share <= cell_pool,
        "the cell gives no more than it holds: {root_share} + {hyphal_share} from {cell_pool}"
    );
}

/// A heterotroph striding `3 d_c` per tick gets `exp(−3)` of what a sessile
/// one of the same heterotrophy gets, once it has moved. Movement runs after
/// uptake within a tick, so uptake reads the distance moved in the most
/// recent move phase (the previous tick's): on the first tick neither agent
/// has moved yet and both absorb in full.
#[test]
fn a_heterotroph_striding_three_contact_distances_absorbs_e_to_the_minus_three_of_a_still_one() {
    let params = lone_heterotroph_params();
    let stride = 3.0 * params.contact_distance;
    let still = Agent::new(0, (-25.0, -25.0), 100.0, 3.0, 5.0, heterotroph(0.5, 0.0));
    let mover = Agent::new(0, (25.0, 25.0), 100.0, 3.0, 5.0, heterotroph(0.5, stride));
    let mut world = world_with(params, &[still, mover]);
    let still_id = world.agents()[0].id;
    let mover_id = world.agents()[1].id;
    let first = world.tick();

    world.step();
    world.step();

    let full = absorbed_on(&world, first, still_id);
    assert!(full > 0.0, "a still heterotroph absorbs from the pool");
    assert_eq!(absorbed_on(&world, first, mover_id), full);
    assert_eq!(absorbed_on(&world, first + 1, still_id), full);
    let moved = absorbed_on(&world, first + 1, mover_id);
    let want = (-3.0_f32).exp() * full;
    assert!(
        (moved - want).abs() < 1e-6,
        "mover absorbed {moved} after a 3 d_c stride, want {want}"
    );
}

/// A recipe written before hyphal uptake existed loads with the contact
/// distance at its default `0.1 u_M`.
#[test]
fn a_recipe_without_the_contact_distance_loads_at_its_default() {
    let contents = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../scenarios/example4.json"
    ))
    .expect("read scenario");
    assert!(!contents.contains("contact_distance"));
    let recipe: WorldRecipe = serde_json::from_str(&contents).expect("parse scenario");
    assert_eq!(
        recipe.parameters.contact_distance,
        0.1 * explorers_sim::units::MOBILITY_DISTANCE_PER_TICK
    );
}

/// A world of heterotrophs, mixotrophs and producers keeps its nutrient
/// closed over a multi-tick run with its heterotrophs drawing on the pool
/// (the stepper's debug ledger also asserts balance every tick).
#[test]
fn nutrient_is_conserved_over_a_run_with_hyphal_uptake() {
    let contents = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../scenarios/example12_generalist_dominance.json"
    ))
    .expect("read scenario");
    let recipe: WorldRecipe = serde_json::from_str(&contents).expect("parse scenario");
    let mut world = World::from_recipe(&recipe, 3);
    let total = |w: &World| -> f32 {
        w.nutrient_pool()
            + w.agents()
                .iter()
                .map(|a| a.nutrient_total(w.params()))
                .sum::<f32>()
            + w.carcasses().iter().map(|c| c.nutrient).sum::<f32>()
    };
    let start = total(&world);
    for _ in 0..200 {
        world.step();
    }
    let end = total(&world);
    assert!(
        (end - start).abs() <= start.abs().max(1.0) * 1e-4,
        "nutrient {start} at start, {end} after 200 ticks"
    );
}
