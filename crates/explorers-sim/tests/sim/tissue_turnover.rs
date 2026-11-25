//! Tissue turnover (#859; world-rules.md *Somatic wear*, *Tissue turnover*;
//! end state S21).
//!
//! Each tick every agent sheds a fraction `τ` of its structure. Of the shed
//! slice `τS`, with the nutrient it binds, the recycled fraction `r` returns
//! to the agent (energy to its reserve, nutrient to its free store) and the
//! rest leaves as necromass at its cell: dead structure carrying its trait
//! vector, which drains as a carcass does. It is physics in every world, with
//! no switch.

use crate::support;

use explorers_sim::event::EventKind;
use explorers_sim::{
    AgentSpec, DEFAULT_TISSUE_TURNOVER_RATE, TISSUE_LIFETIME_LITTER_HALF_LIVES,
    TISSUE_RECYCLED_FRACTION, TraitVector, World, WorldParameters, WorldRecipe,
    stoichiometric_demand, tissue_turnover_rate,
};

fn decomposer() -> TraitVector {
    TraitVector {
        photosynthetic_absorption: 0.0,
        heterotrophy: 1.0,
        mobility: 0.0,
        kappa: 0.8,
        fecundity: 1.0,
        asexual_propensity: 0.0,
        dispersal: 0.0,
    }
}

fn producer() -> TraitVector {
    TraitVector {
        photosynthetic_absorption: 1.0,
        heterotrophy: 0.0,
        ..decomposer()
    }
}

/// A world holding one agent of `traits` at the origin, founded with most of
/// its reserve in structure, in the world's own physics (no turnover setting
/// is touched).
fn lone(params: WorldParameters, traits: TraitVector) -> World {
    World::from_recipe(
        &WorldRecipe {
            parameters: WorldParameters {
                initial_population_size: 1,
                offspring_structure_fraction: 0.9,
                ..params
            },
            initial_distribution: None,
            agents: Some(vec![AgentSpec {
                position: (0.0, 0.0),
                reserve: 40.0,
                traits,
                nutrient: 5.0,
                founding_cluster: 0,
            }]),
            carcasses: None,
            max_ticks: 100,
        },
        1,
    )
}

/// A body whose income has fallen: a producer in the dark, on ground with no
/// free nutrient, and a retention buffer so wide that what reserve it has
/// never mobilises into growth. Its only flows are its upkeep and its
/// turnover. (A decomposer will not do: its own litter is food in its reach.)
fn starving() -> World {
    let mut world = lone(
        WorldParameters {
            solar_flux_magnitude: 0.0,
            growth_retention_multiplier: 100.0,
            ..support::viable_baseline()
        },
        producer(),
    );
    let cells = world.nutrient_grid().cells().len();
    for i in 0..cells {
        *world.nutrient_grid_mut().cell_mut(i) = 0.0;
    }
    world
}

#[track_caller]
fn assert_close(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() <= 1e-5 * expected.abs().max(1.0),
        "{what}: {actual} != {expected}"
    );
}

/// S21's rung test (end-state.md, part 1, *Tissue turnover*): in the world's
/// own physics, with no switch to set, a body whose income has fallen sheds
/// `τS` a tick and does not regrow it, so it shrinks geometrically at `τ`. The
/// shed tissue becomes litter: one necromass carcass at its cell, carrying its
/// trait vector and holding `(1 − r) τS` with the nutrient that binds, while
/// `r τS` of energy feeds its reserve and `r τS × ratio` of nutrient its free
/// store. Its peak structure falls with it and its wear does not move.
#[test]
fn s21_rung_a_body_whose_income_falls_sheds_at_the_declared_rate_into_litter_and_feeds_itself() {
    let mut world = starving();
    let tau = world.params().tissue_turnover_rate;
    assert_eq!(tau, DEFAULT_TISSUE_TURNOVER_RATE, "the world's own rate");
    assert!(tau > 0.0, "turnover acts in every world");
    let r = TISSUE_RECYCLED_FRACTION;
    let ratio = stoichiometric_demand(&producer(), 1.0, world.params());

    let mut litter = 0.0_f32;
    // Eight ticks: its reserve pays about nine of upkeep.
    for tick in 0..8 {
        let before = world.agents()[0].clone();
        let seq = world.event_log().len();
        world.step();
        let after = world.agents()[0].clone();
        let shed = tau * before.structure;

        assert_close(after.structure, before.structure - shed, "shrinks at τ");
        assert_close(
            after.peak_structure,
            before.peak_structure * (1.0 - tau),
            "peak",
        );
        assert_eq!(after.wear, before.wear, "tick {tick}: shedding leaves wear");

        let events = world.event_log().since(seq);
        let upkeep: f32 = (events.iter())
            .filter(|e| e.kind == EventKind::Metabolized && e.source == before.id)
            .map(|e| e.energy_delta)
            .sum();
        assert_close(
            after.reserve,
            before.reserve - upkeep + r * shed,
            "feeds itself",
        );
        assert_close(
            after.nutrient,
            before.nutrient + r * shed * ratio,
            "recycled nutrient",
        );

        litter += (1.0 - r) * shed;
        assert_eq!(world.carcasses().len(), 1, "one pile of litter at its cell");
        let c = &world.carcasses()[0];
        assert_close(c.energy, litter, "shed tissue becomes litter");
        assert_close(
            c.nutrient,
            litter * ratio,
            "its nutrient is bound, as a carcass's",
        );
        assert_eq!(c.traits, producer(), "it carries the shedder's traits");
        assert_eq!(c.position, before.position);
    }
}

/// A body of steady size renews its tissue at the declared rate: a producer
/// in full light regrows what it sheds, and every tick its litter grows by
/// `(1 − r) τ` of the structure it shed from.
#[test]
fn s21_rung_a_growing_body_sheds_tau_of_its_structure_every_tick() {
    let mut world = lone(support::viable_baseline(), producer());
    let tau = world.params().tissue_turnover_rate;
    let r = TISSUE_RECYCLED_FRACTION;
    let mut grew = false;
    for _ in 0..30 {
        let seq = world.event_log().len();
        world.step();
        let events = world.event_log().since(seq);
        grew |= events.iter().any(|e| e.kind == EventKind::Grew);
        let shed: Vec<_> = events
            .iter()
            .filter(|e| e.kind == EventKind::Shed)
            .collect();
        assert_eq!(shed.len(), 1, "one shedding a tick");
        // Shedding is the tick's last change to structure.
        let s = world.agents()[0].structure / (1.0 - tau);
        assert_close(
            shed[0].energy_delta,
            (1.0 - r) * tau * s,
            "litter is (1 − r) τS",
        );
    }
    assert!(grew, "the body regrows");
}

/// Shed tissue is food: a decomposer beside a growing producer drains the
/// producer's necromass, and every tick's energy ledger balances, the litter
/// entering as a flow from its shedder.
#[test]
fn a_decomposer_drains_a_producers_necromass_and_the_ledger_balances() {
    let p = support::viable_baseline();
    let mut world = World::from_recipe(
        &WorldRecipe {
            parameters: WorldParameters {
                initial_population_size: 2,
                offspring_structure_fraction: 0.9,
                ..p
            },
            initial_distribution: None,
            agents: Some(vec![
                AgentSpec {
                    position: (0.0, 0.0),
                    reserve: 40.0,
                    traits: producer(),
                    nutrient: 5.0,
                    founding_cluster: 0,
                },
                AgentSpec {
                    position: (1.0, 0.0),
                    reserve: 40.0,
                    traits: decomposer(),
                    nutrient: 5.0,
                    founding_cluster: 1,
                },
            ]),
            carcasses: None,
            max_ticks: 100,
        },
        3,
    );
    let mut ate_litter = false;
    for _ in 0..10 {
        let seq = world.event_log().len();
        world.step();
        world.energy_ledger().assert_balanced();
        let piles: Vec<u64> = (world.event_log().since(seq).iter())
            .filter(|e| e.kind == EventKind::Shed && e.source == 0)
            .filter_map(|e| e.target)
            .collect();
        ate_litter |= world.event_log().since(seq).iter().any(|e| {
            e.kind == EventKind::Consumed
                && e.source == 1
                && e.target_was_carcass
                && e.target
                    .is_some_and(|t| t >= explorers_sim::phase::NECROMASS_ID_BASE)
        });
        assert_eq!(piles.len(), 1);
    }
    assert!(ate_litter, "the decomposer fed on shed tissue");
}

/// The rate is the domain's: tissue lifetime `1/τ` is `H` litter half-lives,
/// `τ = 1 / (H × L × T_p)`, inside the band 0.25–1 the domain gives
/// (`docs/ecology/fungi.md`, *Turnover*), stated against the world's time
/// anchor, litter half-life 1 producer lifespan of 229 ticks.
#[test]
fn the_turnover_rate_is_a_tissue_lifetime_inside_the_domains_band() {
    assert!((0.25..=1.0).contains(&TISSUE_LIFETIME_LITTER_HALF_LIVES));
    assert_eq!(
        DEFAULT_TISSUE_TURNOVER_RATE,
        tissue_turnover_rate(TISSUE_LIFETIME_LITTER_HALF_LIVES, 1.0, 229.0)
    );
    let loaded: WorldParameters = serde_json::from_value({
        let mut v = serde_json::to_value(support::viable_baseline()).unwrap();
        v.as_object_mut().unwrap().remove("tissue_turnover_rate");
        v
    })
    .unwrap();
    assert_eq!(loaded.tissue_turnover_rate, DEFAULT_TISSUE_TURNOVER_RATE);
}
