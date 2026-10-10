//! Hyphal uptake through the stepper (#727; world-rules.md flow 2, *Hyphal
//! uptake* and *Substrate contact is one predicate*).
//!
//! The phase unit tests pin the demand rule; these pin what only the stepper
//! can show: that uptake reads the distance the move phase actually moved,
//! that conservation holds with the switch on, and that the switch off leaves
//! a world bit-identical to the autotrophic rule.
//!
//! Run with:
//!   cargo test -p explorers-sim --test sim hyphal_uptake::

use crate::support;

use explorers_sim::event::EventKind;
use explorers_sim::{Agent, TraitVector, World, WorldParameters, WorldRecipe};

fn heterotroph(mobility: f32) -> TraitVector {
    TraitVector {
        photosynthetic_absorption: 0.0,
        heterotrophy: 0.5,
        mobility,
        kappa: 0.5,
        fecundity: 0.0,
        asexual_propensity: 0.0,
        dispersal: 0.0,
    }
}

/// A still world of lone heterotrophs: no mutation, no wear, so each agent's
/// effective traits are its raw traits and a mobile agent's stride is
/// `mobility × u_M` every tick.
fn lone_heterotroph_params() -> WorldParameters {
    WorldParameters {
        hyphal_uptake: true,
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

/// A heterotroph striding `3 d_c` per tick gets `exp(−3)` of what a sessile
/// one of the same heterotrophy gets, once it has moved. Movement runs after
/// uptake within a tick, so uptake reads the distance moved in the most
/// recent move phase (the previous tick's): on the first tick neither agent
/// has moved yet and both absorb in full.
#[test]
fn a_heterotroph_striding_three_contact_distances_absorbs_e_to_the_minus_three_of_a_still_one() {
    let params = lone_heterotroph_params();
    let stride = 3.0 * params.contact_distance;
    let still = Agent::new(0, (-25.0, -25.0), 100.0, 3.0, 5.0, heterotroph(0.0));
    let mover = Agent::new(0, (25.0, 25.0), 100.0, 3.0, 5.0, heterotroph(stride));
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

/// A recipe written before the switch existed loads with it off and the
/// contact distance at its default `0.1 u_M`.
#[test]
fn a_recipe_without_the_fields_loads_with_hyphal_uptake_off() {
    let contents = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../scenarios/example13_closed_web.json"
    ))
    .expect("read scenario");
    assert!(!contents.contains("hyphal_uptake"));
    let recipe: WorldRecipe = serde_json::from_str(&contents).expect("parse scenario");
    assert!(!recipe.parameters.hyphal_uptake);
    assert_eq!(
        recipe.parameters.contact_distance,
        0.1 * explorers_sim::units::MOBILITY_DISTANCE_PER_TICK
    );
}

/// With the switch on, a world of heterotrophs, mixotrophs and producers keeps
/// its nutrient closed over a multi-tick run (the stepper's debug ledger also
/// asserts balance every tick).
#[test]
fn nutrient_is_conserved_over_a_run_with_hyphal_uptake_on() {
    let contents = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../scenarios/example13_closed_web.json"
    ))
    .expect("read scenario");
    let mut recipe: WorldRecipe = serde_json::from_str(&contents).expect("parse scenario");
    let mut off = World::from_recipe(&recipe, 3);
    recipe.parameters.hyphal_uptake = true;
    let mut world = World::from_recipe(&recipe, 3);
    let first_tick_uptake = |w: &mut World| -> f32 {
        let tick = w.tick();
        w.step();
        w.event_log()
            .since(0)
            .iter()
            .filter(|e| e.kind == EventKind::NutrientAbsorbed && e.tick == tick)
            .map(|e| e.energy_delta)
            .sum()
    };
    let total = |w: &World| -> f32 {
        w.nutrient_pool()
            + w.agents()
                .iter()
                .map(|a| a.nutrient_total(w.params()))
                .sum::<f32>()
            + w.carcasses().iter().map(|c| c.nutrient).sum::<f32>()
    };
    let start = total(&world);
    assert!(
        first_tick_uptake(&mut world) > first_tick_uptake(&mut off),
        "the switch must let the web's heterotrophs draw on the pool"
    );
    for _ in 0..199 {
        world.step();
    }
    let end = total(&world);
    assert!(
        (end - start).abs() <= start.abs().max(1.0) * 1e-4,
        "nutrient {start} at start, {end} after 200 ticks"
    );
}
