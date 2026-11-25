//! Size-scaled uptake through the stepper (#644, #866; world-rules.md flow 2,
//! *Size-scaled uptake*).
//!
//! Size-scaled uptake is physics in every world since #866, with no switch:
//! pool-uptake demand is `rate × u_A × S / s_ref`, linear in structure `S`,
//! `s_ref` the world's uptake reference structure. The phase unit tests pin
//! the demand rule; this pins S28's rung through the stepper.
//!
//! Run with:
//!   cargo test -p explorers-sim --test sim size_scaled_uptake::

use crate::support;

use explorers_sim::event::EventKind;
use explorers_sim::units::AUTOTROPHY_NUTRIENT_UPTAKE_PER_TICK;
use explorers_sim::{
    Agent, DEFAULT_BASE_RATE_REFERENCE_STRUCTURE, DEFAULT_UPTAKE_REFERENCE_STRUCTURE, TraitVector,
    World, WorldParameters, WorldRecipe,
};

fn traits(autotrophy: f32, heterotrophy: f32) -> TraitVector {
    TraitVector {
        photosynthetic_absorption: autotrophy,
        heterotrophy,
        mobility: 0.0,
        kappa: 0.5,
        fecundity: 0.0,
        asexual_propensity: 0.0,
        dispersal: 0.0,
    }
}

/// The world's default physics with no mutation and no wear, so each agent's
/// effective traits are its raw traits.
fn params() -> WorldParameters {
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

/// S28 (end-state.md part 1), exact, on a few agents:
/// - the world's uptake reference structure is the median established
///   producer's, the base rate's `S_p`;
/// - a producer on an ample cell takes up `effective autotrophy × u_A × S /
///   s_ref` a tick: its uptake doubles when its body doubles, and at `s_ref`
///   it is the size-blind `effective autotrophy × u_A`;
/// - co-located agents, roots and hyphae alike, share a depleted cell in
///   proportion to their size-scaled demand, and leave it at zero, never
///   below.
#[test]
fn s28_rung_a_producers_uptake_grows_in_proportion_to_its_structure_and_a_depleted_cell_is_shared_by_size_scaled_demand()
 {
    let params = params();
    let s_ref = params.uptake_reference_structure;
    assert_eq!(s_ref, DEFAULT_UPTAKE_REFERENCE_STRUCTURE);
    assert_eq!(
        DEFAULT_UPTAKE_REFERENCE_STRUCTURE, DEFAULT_BASE_RATE_REFERENCE_STRUCTURE,
        "uptake is anchored at the base rate's S_p"
    );

    // Producers of one autotrophy, each alone in its own cell of an ample pool.
    let u_a = AUTOTROPHY_NUTRIENT_UPTAKE_PER_TICK;
    let sizes = [0.5, 1.0, 2.0, 4.0];
    let corners = [(-35.0, -35.0), (35.0, -35.0), (-35.0, 35.0), (35.0, 35.0)];
    let producers: Vec<Agent> = sizes
        .iter()
        .zip(corners)
        .map(|(k, at)| Agent::new(0, at, 100.0, k * s_ref, 5.0, traits(0.5, 0.0)))
        .collect();
    let mut world = world_with(params.clone(), &producers);
    let ids: Vec<u64> = world.agents().iter().map(|a| a.id).collect();
    let first = world.tick();
    world.step();

    let got: Vec<f32> = ids
        .iter()
        .map(|&id| absorbed_on(&world, first, id))
        .collect();
    for (k, took) in sizes.iter().zip(&got) {
        let want = 0.5 * u_a * k;
        assert!(
            (took - want).abs() <= want * 1e-5,
            "a producer of {k} s_ref took up {took}, want {want}"
        );
    }
    assert!(
        (got[1] - 0.5 * u_a).abs() <= 1e-6,
        "at s_ref a producer takes up the size-blind 0.5 u_A, took {}",
        got[1]
    );
    for pair in got.windows(2) {
        assert!(
            (pair[1] / pair[0] - 2.0).abs() < 1e-5,
            "uptake doubles when the body doubles: {} then {}",
            pair[0],
            pair[1]
        );
    }

    // One depleted cell of 0.09: a producer of autotrophy 0.6 at s_ref, one of
    // 0.3 at 2 s_ref and a still heterotroph of 0.3 at s_ref demand 0.6, 0.6
    // and 0.3 u_A, so they take 0.036, 0.036 and 0.018.
    let depleted = WorldParameters {
        initial_nutrient_pool: 0.09 * 100.0,
        ..params
    };
    let cell_pool = 0.09_f32;
    let agents = [
        Agent::new(0, (-25.0, -25.0), 100.0, s_ref, 5.0, traits(0.6, 0.0)),
        Agent::new(0, (-24.0, -24.0), 100.0, 2.0 * s_ref, 5.0, traits(0.3, 0.0)),
        Agent::new(0, (-23.0, -23.0), 100.0, s_ref, 5.0, traits(0.0, 0.3)),
    ];
    let mut world = world_with(depleted, &agents);
    let grid = world.nutrient_grid();
    let cell = grid.cell_index_for((-25.0, -25.0));
    assert_eq!(cell, grid.cell_index_for((-24.0, -24.0)));
    assert_eq!(cell, grid.cell_index_for((-23.0, -23.0)));
    assert!((grid.cells()[cell] - cell_pool).abs() < 1e-6);
    let ids: Vec<u64> = world.agents().iter().map(|a| a.id).collect();
    let first = world.tick();
    world.step();
    let shares: Vec<f32> = ids
        .iter()
        .map(|&id| absorbed_on(&world, first, id))
        .collect();
    for (share, demand) in shares.iter().zip([0.6, 0.6, 0.3]) {
        let want = cell_pool * demand / 1.5;
        assert!(
            (share - want).abs() < 1e-6,
            "a demand of {demand} took {share} of {cell_pool}, want {want}"
        );
    }
    let taken: f32 = shares.iter().sum();
    assert!(
        taken <= cell_pool,
        "the cell gives no more than it holds: {taken} from {cell_pool}"
    );
}
