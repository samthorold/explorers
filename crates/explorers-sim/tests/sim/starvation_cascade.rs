//! The starvation cascade (#821, #863; world-rules.md flow 8, *When reserve
//! cannot pay: the starvation cascade*): hypothesis S23 of the end state.
//!
//! Upkeep is paid from unallocated reserve. What reserve cannot pay is drawn
//! next from the energy earmark (`repro_reserve`), the reproductive-nutrient
//! earmark untouched, and what is left is paid by burning structure at its
//! full embodied energy, the burned structure's bound nutrient returning to
//! the free store and the peak structure unchanged. The agent dies when its
//! structure falls below `structural_fragility × peak_structure`. The cascade
//! is physics in every world since #863, with no switch.
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
/// agent's stores change only by its upkeep. Tissue turnover is held at 0, a
/// controlled variant: it would recycle shed tissue into the stores the
/// cascade draws on, and leave the agent its own litter to eat, and this
/// suite reads the cascade alone (S23); turnover's own rule is S21's.
fn params() -> WorldParameters {
    WorldParameters {
        tissue_turnover_rate: 0.0,
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
        marker: Default::default(),
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

/// S23's rung: a starving agent pays the upkeep its reserve cannot from its
/// energy earmark first, its structure untouched while the earmark lasts and
/// its nutrient earmark never drawn; then from structure, burned at full
/// energy with its bound nutrient freed and the peak held; and it dies when
/// its structure falls below its fragility threshold, never living below it.
/// Every tick is checked exactly against the agent's upkeep at the start of
/// that tick.
#[test]
fn s23_rung_a_starving_agent_pays_upkeep_from_its_earmark_then_from_structure_down_to_its_fragility_threshold()
 {
    let p = params();
    let agent = starving(0.1, 1.0, 5.0);
    let threshold = death_threshold(&agent.traits, 5.0);
    assert!(
        metabolic_cost(&agent, &p) > 0.1,
        "the agent must be short of its upkeep"
    );
    let mut world = world_with(p.clone(), &agent);
    let (mut ticks_on_earmark, mut ticks_on_structure) = (0, 0);
    let mut died = false;
    for _ in 0..10_000 {
        let before = world.agents()[0].clone();
        let upkeep = metabolic_cost(&before, &p);
        let shortfall = upkeep - before.reserve;
        assert!(shortfall > 0.0, "the agent has no income");
        let from_earmark = shortfall.min(before.repro_reserve);
        let burned = shortfall - from_earmark;
        world.step();

        let Some(after) = world.agents().first() else {
            // Dead: only once the earmark is spent and structure has fallen
            // below the fragility threshold.
            assert_eq!(from_earmark, before.repro_reserve, "died holding earmark");
            assert!(
                before.structure - burned < threshold,
                "died at structure {} above its threshold {threshold}",
                before.structure - burned
            );
            let carcass = &world.carcasses()[0];
            assert!(close(carcass.energy, before.structure - burned));
            died = true;
            break;
        };
        assert_eq!(after.reserve, 0.0);
        assert!(
            close(after.repro_reserve, before.repro_reserve - from_earmark),
            "earmark {}, expected {}",
            after.repro_reserve,
            before.repro_reserve - from_earmark
        );
        assert!(after.repro_reserve >= 0.0, "the earmark overdrawn");
        assert_eq!(after.repro_nutrient, 2.0, "the nutrient earmark is drawn");
        assert!(
            close(after.structure, before.structure - burned),
            "structure {}, expected {}",
            after.structure,
            before.structure - burned
        );
        let freed = stoichiometric_demand(&agent.traits, burned, &p);
        assert!(
            close(after.nutrient, before.nutrient + freed),
            "free nutrient {}, expected {}",
            after.nutrient,
            before.nutrient + freed
        );
        assert_eq!(after.peak_structure, 5.0, "the peak fell");
        assert!(after.structure >= threshold, "alive below its threshold");
        if from_earmark > 0.0 {
            ticks_on_earmark += 1;
        }
        if burned > 0.0 {
            assert!(
                after.repro_reserve <= 0.0,
                "structure burned before the earmark"
            );
            ticks_on_structure += 1;
        }
    }
    assert!(died, "it never died");
    assert!(
        ticks_on_earmark >= 2,
        "lived {ticks_on_earmark} ticks on its earmark"
    );
    assert!(
        ticks_on_structure >= 2,
        "lived {ticks_on_structure} ticks on structure"
    );
}

/// The cascade runs in every world, with nothing to set: a starving agent in
/// the baseline world, tissue turnover at its default rate, lives on its
/// earmark when its reserve is short rather than dying with it.
#[test]
fn the_cascade_runs_with_turnover_at_its_default_rate() {
    let p = WorldParameters {
        initial_nutrient_pool: 0.0,
        ..support::viable_baseline()
    };
    let agent = starving(0.0, 10.0, 5.0);
    let mut world = world_with(p, &agent);
    world.step();
    let after = world.agents().first().expect("the earmark keeps it alive");
    assert!(after.repro_reserve < 10.0, "the earmark paid");
}

/// A reserve at zero is not death in the drain pass either: a living target
/// that holds no reserve, drained well above its structural threshold, lives.
#[test]
fn a_drained_target_at_zero_reserve_lives_above_its_threshold() {
    let p = params();
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
    assert!(!result.dead_agents.contains(&2));
}
