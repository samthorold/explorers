//! Regression for issue #384: a seeded consumer must not be rendered illiquid on
//! tick 1 by one-tick reserve liquidation.
//!
//! Flow 9 mobilises the above-buffer reserve excess and splits it by kappa into a
//! (mostly unspendable) reproductive earmark. Under the historical one-tick
//! liquidation (`reserve_mobilisation_rate = 1.0`), a freshly seeded consumer —
//! provisioned with a substantial reserve but no feeding income yet — has its
//! whole standing reserve dumped into the earmark on its first well-fed tick,
//! leaving it at the bare retention buffer with no cushion. A consumer so
//! stripped starves before it can reach prey and establish a feeding income.
//!
//! The reserve mobilisation rate (`f < 1`, DEB energy conductance) spreads the
//! mobilisation over many ticks, so the seed provisioning persists as a survival
//! buffer. This test asserts the cushion survives tick 1 and that the consumer
//! lives long enough to draw a feeding income — the assertion no test made before
//! the fix.
//!
//! Run with:
//!   cargo test -p explorers-sim --test seeded_consumer_liquidity -- --nocapture

use explorers_sim::{Agent, World, WorldParameters, WorldRecipe};

const SCENARIO: &str = "/Users/sam/Projects/explorers/scenarios/example4.json";

fn load() -> WorldRecipe {
    let contents = std::fs::read_to_string(SCENARIO).expect("read scenario");
    serde_json::from_str(&contents).expect("parse scenario")
}

fn is_consumer(a: &Agent) -> bool {
    a.traits.heterotrophy > a.traits.photosynthetic_absorption
}

/// The per-tick metabolic retention buffer the grow phase protects — reserve at or
/// below this is *not* mobilisable, so a consumer pinned here has no cushion.
fn retention_buffer(a: &Agent, p: &WorldParameters) -> f32 {
    let exp = p.maintenance_cost_exponent;
    let metabolic = p.base_metabolic_rate
        + a.traits.photosynthetic_absorption.powf(exp) * p.photo_maintenance_cost
        + a.traits.heterotrophy.powf(exp) * p.heterotrophy_maintenance_cost
        + a.traits.mobility.powf(exp) * p.mobility_maintenance_cost
        + a.traits.asexual_propensity.powf(exp) * p.asexual_propensity_maintenance_cost
        + a.structure * p.structure_maintenance_coefficient;
    metabolic * p.growth_retention_multiplier
}

/// example4 ships with `reserve_mobilisation_rate < 1.0`, so a seeded consumer
/// keeps a reserve cushion above its retention buffer past tick 1 instead of being
/// liquidated to the bare buffer — and it survives long enough to feed.
#[test]
fn seeded_consumer_keeps_a_reserve_cushion_past_tick_one() {
    let recipe = load();
    assert!(
        recipe.parameters.reserve_mobilisation_rate < 1.0,
        "example4 must set a reserve mobilisation rate below 1.0 so seeded \
         consumers are not liquidated on tick 1 (got {})",
        recipe.parameters.reserve_mobilisation_rate
    );

    let params = recipe.parameters.clone();
    let mut world = World::from_recipe(&recipe, 1);

    // Identify the seeded consumers and their starting reserve.
    let seed_consumers: Vec<(u64, f32)> = world
        .agents()
        .iter()
        .filter(|a| is_consumer(a))
        .map(|a| (a.id, a.reserve))
        .collect();
    assert!(
        !seed_consumers.is_empty(),
        "example4 must seed at least one consumer for this regression"
    );

    // One tick of the full loop runs the grow phase (flow 9 mobilisation).
    world.step();

    // After tick 1 every surviving seeded consumer must still hold a reserve
    // cushion meaningfully above its retention buffer. Under one-tick liquidation
    // (f = 1.0) the consumer would be pinned at the buffer with no cushion.
    for (id, seed_reserve) in &seed_consumers {
        let agent = world
            .agents()
            .iter()
            .find(|a| a.id == *id)
            .unwrap_or_else(|| panic!("seeded consumer {id} died on tick 1 — it was illiquid"));
        let buffer = retention_buffer(agent, &params);
        let cushion = agent.reserve - buffer;
        assert!(
            cushion > 0.1 * (seed_reserve - buffer),
            "seeded consumer {id} should keep a reserve cushion above its retention \
             buffer past tick 1 (reserve {:.3}, buffer {:.3}, cushion {:.3}); under \
             one-tick liquidation the cushion collapses to ~0",
            agent.reserve,
            buffer,
            cushion
        );
    }
}

/// The seeded consumers survive long enough to establish a feeding income — the
/// downstream consequence of keeping a cushion: they reach prey and draw real
/// predation energy rather than starving at the retention buffer first.
#[test]
fn seeded_consumers_survive_to_establish_a_feeding_income() {
    let recipe = load();
    let mut world = World::from_recipe(&recipe, 1);

    let consumer_ids: Vec<u64> = world
        .agents()
        .iter()
        .filter(|a| is_consumer(a))
        .map(|a| a.id)
        .collect();

    // Run a window long enough for a consumer to close on prey and draw a feeding
    // income, but short enough to stay a fast unit test. Under the historical
    // one-tick liquidation (f = 1.0) both seeded consumers are stripped to the
    // bare retention buffer on tick 1 and dead by tick 3 (verified by toggling the
    // rate) — so neither survives this window nor accumulates a feeding income.
    // With the bounded rate they keep a cushion, survive, and feed throughout.
    for _ in 0..10 {
        world.step();
    }

    let mut predation_income = 0.0f32;
    for ev in world.event_log().since(0) {
        if let explorers_sim::event::EventKind::Consumed = ev.kind {
            if consumer_ids.contains(&ev.source) && !ev.target_was_carcass {
                predation_income += ev.energy_delta;
            }
        }
    }

    let surviving = world
        .agents()
        .iter()
        .filter(|a| is_consumer(a) && consumer_ids.contains(&a.id))
        .count();

    assert!(
        surviving > 0,
        "at least one seeded consumer must survive the window to establish a \
         feeding income (all starved — the illiquidity the fix prevents)"
    );
    assert!(
        predation_income > 0.0,
        "seeded consumers must draw a real feeding income once they reach prey \
         (cumulative predation intake = {predation_income})"
    );
}
