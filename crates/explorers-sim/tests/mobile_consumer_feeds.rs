//! Behavioural verification that a *mobile* consumer can feed (issue #380).
//!
//! This is the gap #379 identified: under the old contact-duration feeding ramp
//! (`demand = eff * ct/(ct+K)`), any consumer with `mobility > 0` reset its own
//! contact clock to zero every tick it moved — so `ct` never accumulated, demand
//! was always ~zero, and every mobile heterotroph starved. No unit test caught
//! this because the unit tests set the contact-duration field by hand and never
//! ran a moving consumer through the stepper.
//!
//! Consumption is now a binary-reach drain: while a target is within feeding
//! reach, the consumer drains it at its effective heterotrophy each tick, with no
//! warm-up. A mobile consumer co-located with abundant prey must therefore draw
//! real energy from it and survive past tick 2.
//!
//! Run with:
//!   cargo test -p explorers-sim --test mobile_consumer_feeds -- --nocapture

use explorers_sim::{Agent, TraitVector, World, WorldParameters, WorldRecipe};

fn feeding_params() -> WorldParameters {
    let contents = std::fs::read_to_string(
        "/Users/sam/Projects/explorers/scenarios/example10_predator_prey_hopf.json",
    )
    .expect("read scenario");
    let recipe: WorldRecipe = serde_json::from_str(&contents).expect("parse scenario");
    recipe.parameters
}

fn trait_with(photosynthetic_absorption: f32, heterotrophy: f32, mobility: f32) -> TraitVector {
    TraitVector {
        photosynthetic_absorption,
        heterotrophy,
        mobility,
        kappa: 0.5,
        fecundity: 0.0,
        asexual_propensity: 0.0,
        dispersal: 0.0,
    }
}

/// A *mobile* consumer (`mobility > 0`) co-located with prey must drain energy and
/// survive past tick 2 — the assertion no unit test made. We seed one mobile
/// heterotroph on top of a fat sessile producer (a large prey body, so the
/// per-tick drain is sub-lethal) and run a short headless loop, asserting the
/// consumer draws real predation energy from the event log and is still alive at
/// the end.
#[test]
fn mobile_consumer_co_located_with_prey_feeds_and_survives_past_tick_two() {
    let params = feeding_params();
    let mut world = World::from_recipe(
        &WorldRecipe {
            parameters: params,
            agents: Some(vec![]),
            carcasses: None,
            max_ticks: 20,
            initial_distribution: None,
        },
        1,
    );

    // Mobile consumer: real mobility, heterotrophy-dominant, modest reserve.
    // (World::add_agent assigns ids, so capture the id it was given.)
    let consumer = Agent {
        reserve: 5.0,
        structure: 3.0,
        peak_structure: 3.0,
        nutrient: 5.0,
        ..Agent::new(0, (50.0, 50.0), 5.0, 3.0, 5.0, trait_with(0.0, 1.0, 0.6))
    };
    world.add_agent(consumer);
    let consumer_id = world.agents().last().unwrap().id;

    // A fat sessile producer co-located with the consumer: a large prey body, so a
    // per-tick heterotrophy drain stays below the structural death threshold
    // (grazing, not predation) and supplies the consumer for the whole run.
    let prey = Agent {
        reserve: 100.0,
        structure: 200.0,
        peak_structure: 200.0,
        nutrient: 50.0,
        ..Agent::new(
            0,
            (50.0, 50.0),
            100.0,
            200.0,
            50.0,
            trait_with(1.0, 0.0, 0.0),
        )
    };
    world.add_agent(prey);
    let prey_id = world.agents().last().unwrap().id;

    for _ in 0..10 {
        world.step();
    }

    // Tally the consumer's cumulative predation intake from the event log: the
    // direct evidence it is *feeding* (not merely alive on its seed reserve). A
    // mobile consumer reset its contact clock every tick under the bug, so this
    // sum was exactly zero.
    let mut predation_income = 0.0f32;
    for ev in world.event_log().since(0) {
        if let explorers_sim::event::EventKind::Consumed = ev.kind {
            if ev.source == consumer_id && !ev.target_was_carcass {
                predation_income += ev.energy_delta;
            }
        }
    }

    assert!(
        world.agents().iter().any(|a| a.id == consumer_id),
        "a mobile consumer co-located with abundant prey must survive past tick 2 \
         (it starved under the contact-duration ramp bug, #379)"
    );
    assert!(
        predation_income > 0.0,
        "the mobile consumer must draw real energy from the prey while in reach \
         (cumulative predation intake = {predation_income})"
    );
    assert!(
        world.agents().iter().any(|a| a.id == prey_id),
        "the prey body is large enough that the per-tick drain is sub-lethal — it \
         should survive (grazing, not predation)"
    );
}
