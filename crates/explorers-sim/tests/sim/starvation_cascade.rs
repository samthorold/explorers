//! The starvation cascade (#821; world-rules.md flow 8, *When reserve cannot
//! pay: the starvation cascade*).
//!
//! Upkeep is paid from unallocated reserve. With `starvation_draws_earmark`
//! on, a shortfall is drawn next from the energy earmark (`repro_reserve`),
//! the reproductive-nutrient earmark untouched. With `starvation_shrinks` on,
//! what is left is paid by burning structure at its full embodied energy,
//! the burned structure's bound nutrient returning to the free store and the
//! peak structure unchanged; the agent dies when its structure falls below
//! `structural_fragility × peak_structure`. With both off it dies when its
//! reserve runs out.
//!
//! Run with:
//!   cargo test -p explorers-sim --test sim starvation_cascade::

use crate::support;

use explorers_sim::phase::{metabolic_cost, resolve_drains};
use explorers_sim::spatial::{NutrientGrid, SpatialGrid};
use explorers_sim::{
    Agent, TraitVector, World, WorldParameters, WorldRecipe, death_threshold, stoichiometric_demand,
};

/// A sessile heterotroph with nothing to eat.
fn traits() -> TraitVector {
    TraitVector {
        photosynthetic_absorption: 0.0,
        heterotrophy: 0.5,
        mobility: 0.0,
        kappa: 0.5,
        fecundity: 0.0,
        asexual_propensity: 0.0,
        dispersal: 0.0,
    }
}

/// A still world with no nutrient pool, no mutation and no wear, so a lone
/// agent's stores change only by its upkeep.
fn params(draws_earmark: bool, shrinks: bool) -> WorldParameters {
    WorldParameters {
        starvation_draws_earmark: draws_earmark,
        starvation_shrinks: shrinks,
        initial_nutrient_pool: 0.0,
        mutation_rate: 0.0,
        mutation_magnitude: 0.0,
        ..support::viable_baseline()
    }
}

/// A starving agent: `reserve` short of its upkeep, holding `earmark` energy
/// and 2.0 of earmarked nutrient, and `structure` at its peak.
fn starving(reserve: f32, earmark: f32, structure: f32) -> Agent {
    Agent {
        id: 0,
        position: (50.0, 50.0),
        reserve,
        structure,
        peak_structure: structure,
        nutrient: 1.0,
        traits: traits(),
        wear: Default::default(),
        repro_reserve: earmark,
        repro_nutrient: 2.0,
    }
}

fn world_with(params: WorldParameters, agent: &Agent) -> World {
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
    world.add_agent(agent.clone());
    world
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() <= 1e-5 * a.abs().max(b.abs()).max(1.0)
}

/// With the earmark switch on, the shortfall after unallocated reserve is
/// drawn from the energy earmark: the agent lives, its reserve at zero, its
/// earmark down by the shortfall, its nutrient earmark and structure as they
/// were.
#[test]
fn the_earmark_pays_the_shortfall_and_the_nutrient_earmark_is_untouched() {
    let p = params(true, false);
    let agent = starving(0.1, 10.0, 5.0);
    let cost = metabolic_cost(&agent, &p);
    assert!(cost > 0.1, "the agent must be short: cost {cost}");
    let mut world = world_with(p, &agent);
    world.step();

    let after = world.agents().first().expect("the earmark keeps it alive");
    assert_eq!(after.reserve, 0.0);
    assert!(
        close(after.repro_reserve, 10.0 - (cost - 0.1)),
        "earmark {}, expected {}",
        after.repro_reserve,
        10.0 - (cost - 0.1)
    );
    assert_eq!(after.repro_nutrient, 2.0);
    assert_eq!(after.structure, 5.0);
}

/// With shrinking on, the shortfall is paid by burning structure at its full
/// embodied energy: structure falls by exactly the shortfall, the nutrient
/// bound in it returns to the free store, and the peak does not fall.
#[test]
fn shrinking_burns_structure_at_full_energy_and_frees_its_nutrient() {
    let p = params(false, true);
    let agent = starving(0.1, 10.0, 5.0);
    let cost = metabolic_cost(&agent, &p);
    let burned = cost - 0.1;
    assert!(
        5.0 - burned > death_threshold(&agent.traits, 5.0),
        "one tick's burn must leave it above its threshold"
    );
    let mut world = world_with(p.clone(), &agent);
    world.step();

    let after = world.agents().first().expect("shrinking keeps it alive");
    assert_eq!(after.reserve, 0.0);
    assert!(
        close(after.structure, 5.0 - burned),
        "structure {}, expected {}",
        after.structure,
        5.0 - burned
    );
    assert_eq!(after.peak_structure, 5.0);
    let freed = stoichiometric_demand(&agent.traits, burned, &p);
    assert!(freed > 0.0);
    assert!(
        close(after.nutrient, 1.0 + freed),
        "free nutrient {}, expected {}",
        after.nutrient,
        1.0 + freed
    );
    assert_eq!(after.repro_reserve, 10.0, "the earmark switch is off");
    assert_eq!(after.repro_nutrient, 2.0);
}

/// The order is reserve, then the earmark, then structure: with both on and
/// an earmark smaller than the shortfall, the reserve and the earmark are
/// emptied and structure pays only the rest.
#[test]
fn reserve_then_the_earmark_then_structure() {
    let p = params(true, true);
    let earmark = 0.05;
    let agent = starving(0.1, earmark, 5.0);
    let cost = metabolic_cost(&agent, &p);
    assert!(
        cost > 0.1 + earmark,
        "the earmark must fall short: cost {cost}"
    );
    let mut world = world_with(p, &agent);
    world.step();

    let after = world.agents().first().expect("structure keeps it alive");
    assert_eq!(after.reserve, 0.0);
    assert_eq!(after.repro_reserve, 0.0);
    let burned = cost - 0.1 - earmark;
    assert!(
        close(after.structure, 5.0 - burned),
        "structure {}, expected {}",
        after.structure,
        5.0 - burned
    );
}

/// A shrinking agent dies when its structure falls below
/// `structural_fragility × peak_structure`, the structural death rule.
#[test]
fn a_shrinking_agent_dies_when_structure_crosses_the_fragility_threshold() {
    let p = params(false, true);
    let agent = starving(0.0, 0.0, 5.0);
    let threshold = death_threshold(&agent.traits, 5.0);
    let mut world = world_with(p, &agent);
    let mut last_structure = 5.0;
    for _ in 0..10_000 {
        world.step();
        match world.agents().first() {
            Some(a) => {
                assert!(a.structure >= threshold, "alive below its threshold");
                assert_eq!(a.peak_structure, 5.0);
                last_structure = a.structure;
            }
            None => break,
        }
    }
    assert!(world.agents().is_empty(), "it never died");
    let carcass = &world.carcasses()[0];
    assert!(
        carcass.energy < threshold,
        "it died at structure {}, threshold {threshold}",
        carcass.energy
    );
    assert!(last_structure >= threshold);
}

/// The earmark switch alone: the agent lives on its earmark, then dies of
/// reserve depletion once the earmark is spent, its structure never burned.
#[test]
fn the_earmark_alone_dies_of_reserve_depletion_once_the_earmark_is_spent() {
    let p = params(true, false);
    let agent = starving(0.0, 1.0, 5.0);
    let cost = metabolic_cost(&agent, &p);
    let mut world = world_with(p, &agent);
    let mut ticks_alive = 0;
    while world.agents().first().is_some() && ticks_alive < 10_000 {
        world.step();
        if let Some(a) = world.agents().first() {
            assert_eq!(a.structure, 5.0, "structure is never burned");
            ticks_alive += 1;
        }
    }
    assert!(world.agents().is_empty(), "it never died");
    assert_eq!(ticks_alive, (1.0 / cost).floor() as usize);
    assert_eq!(
        world.carcasses()[0].energy,
        5.0,
        "died whole, of starvation"
    );
}

/// Shrinking alone goes straight from reserve to structure: the earmark is
/// never drawn.
#[test]
fn shrinking_alone_goes_straight_from_reserve_to_structure() {
    let p = params(false, true);
    let agent = starving(0.0, 10.0, 5.0);
    let mut world = world_with(p, &agent);
    world.step();
    let after = world.agents().first().expect("shrinking keeps it alive");
    assert_eq!(after.repro_reserve, 10.0);
    assert!(after.structure < 5.0);
}

/// With both switches off the agent dies when its reserve runs out, its
/// earmark and structure untouched, as before the cascade.
#[test]
fn with_both_off_the_agent_dies_when_reserve_runs_out() {
    let p = params(false, false);
    let agent = starving(0.1, 10.0, 5.0);
    let mut world = world_with(p, &agent);
    world.step();
    assert!(world.agents().is_empty());
    assert_eq!(world.carcasses()[0].energy, 5.0);
}

/// Under the cascade a reserve at zero is not death in the drain pass either:
/// a living target that holds no reserve, drained well above its structural
/// threshold, lives with either switch on and dies with both off.
#[test]
fn a_drained_target_at_zero_reserve_dies_only_with_the_cascade_off() {
    let dies_when_drained = |p: WorldParameters| {
        let consumer = Agent {
            id: 1,
            reserve: 100.0,
            traits: TraitVector {
                heterotrophy: 1.2,
                ..traits()
            },
            ..starving(100.0, 0.0, 5.0)
        };
        let target = Agent {
            id: 2,
            traits: TraitVector {
                photosynthetic_absorption: 1.0,
                heterotrophy: 0.0,
                ..traits()
            },
            ..starving(0.0, 10.0, 50.0)
        };
        let mut agents = vec![consumer, target];
        let mut grid = SpatialGrid::new(100.0, 10.0);
        for (i, a) in agents.iter().enumerate() {
            grid.insert(i as u64, a.position);
        }
        let mut nutrient_grid = NutrientGrid::new(100.0, 10.0, 0.0);
        let result = resolve_drains(&mut agents, &mut [], &grid, &p, &mut nutrient_grid);
        assert!(agents[1].structure < 50.0, "the target is drained");
        assert!(agents[1].structure > death_threshold(&agents[1].traits, 50.0));
        result.dead_agents.contains(&2)
    };
    assert!(dies_when_drained(params(false, false)));
    assert!(!dies_when_drained(params(true, false)));
    assert!(!dies_when_drained(params(false, true)));
    assert!(!dies_when_drained(params(true, true)));
}
