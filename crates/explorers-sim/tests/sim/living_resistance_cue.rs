//! A living target's movement cue carries living-tissue resistance (#816;
//! world-rules.md, *Foraging movement follows what a resource releases* and
//! *Consumption is incremental → Living tissue resists drains*).
//!
//! Movement reads what the drains read, so a living target's cue, its
//! structure times the expression recognition leaves toward it, is scaled by
//! the same `(1 − ρ)` as its drain. A carcass's cue, its remaining energy, is
//! not. Each test isolates the pull from the keyed random-walk term by
//! comparing two steps of the same forager (same id, run seed and tick, so the
//! same random term): the step a scene gives, against the step of a scene
//! whose pull is known.

use crate::support;

use explorers_sim::phase::move_agents;
use explorers_sim::spatial::SpatialGrid;
use explorers_sim::{Agent, Carcass, TraitVector, WorldParameters, toroidal_distance};

const EXTENT: f32 = 100.0;
const SITE: (f32, f32) = (50.0, 50.0);
const SEEDS: u64 = 20;

fn params(rho: f32) -> WorldParameters {
    WorldParameters {
        world_extent: EXTENT,
        sensing_range_coefficient: 10.0,
        movement_cost_coefficient: 0.0,
        living_resistance: rho,
        ..support::viable_baseline()
    }
}

fn zero_traits() -> TraitVector {
    TraitVector {
        photosynthetic_absorption: 0.0,
        heterotrophy: 0.0,
        mobility: 0.0,
        kappa: 0.5,
        fecundity: 0.0,
        asexual_propensity: 0.0,
        dispersal: 0.0,
    }
}

fn agent(id: u64, position: (f32, f32), structure: f32, traits: TraitVector) -> Agent {
    Agent {
        id,
        position,
        reserve: 100.0,
        structure,
        peak_structure: structure,
        nutrient: 5.0,
        traits,
        wear: Default::default(),
        repro_reserve: 0.0,
        repro_nutrient: 0.0,
        marker: Default::default(),
    }
}

/// A foraging decomposer at `SITE`: senses 5.0 (mobility 0.5 × coefficient
/// 10) and steers with heterotrophy 0.8.
fn forager() -> Agent {
    agent(
        0,
        SITE,
        5.0,
        TraitVector {
            heterotrophy: 0.8,
            mobility: 0.5,
            ..zero_traits()
        },
    )
}

/// A sessile living producer at `SITE + offset`, unrecognised by the forager
/// (far from it in trait space), so recognition spares none of it.
fn producer(id: u64, offset: (f32, f32), structure: f32) -> Agent {
    agent(
        id,
        (SITE.0 + offset.0, SITE.1 + offset.1),
        structure,
        TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        },
    )
}

fn carcass(id: u64, offset: (f32, f32), energy: f32) -> Carcass {
    Carcass {
        id,
        position: (SITE.0 + offset.0, SITE.1 + offset.1),
        energy,
        energy_at_death: energy,
        nutrient: 0.0,
        traits: zero_traits(),
    }
}

/// One movement tick of `living` (the forager first) among `carcasses`;
/// the forager's new position.
fn step(
    mut living: Vec<Agent>,
    carcasses: &[Carcass],
    params: &WorldParameters,
    run_seed: u64,
) -> (f32, f32) {
    let mut grid = SpatialGrid::new(EXTENT, 10.0);
    for (i, a) in living.iter().enumerate() {
        grid.insert(i as u64, a.position);
    }
    move_agents(&mut living, carcasses, &grid, params, run_seed, 0);
    living[0].position
}

fn bits(p: (f32, f32)) -> (u32, u32) {
    (p.0.to_bits(), p.1.to_bits())
}

#[test]
fn at_full_resistance_a_forager_steps_toward_the_carcass_as_if_the_living_target_were_absent() {
    // A large living producer on +x and a small carcass on −x, both in range.
    // At ρ = 1 the producer pulls nothing, so the step is the carcass-only
    // step, bit for bit; and that step is pulled toward the carcass.
    let params = params(1.0);
    let litter = [carcass(9, (-2.0, 0.0), 2.0)];
    let mut toward = 0;
    for seed in 0..SEEDS {
        let with_producer = step(
            vec![forager(), producer(1, (2.0, 0.0), 40.0)],
            &litter,
            &params,
            seed,
        );
        let carcass_only = step(vec![forager()], &litter, &params, seed);
        assert_eq!(bits(with_producer), bits(carcass_only), "seed {seed}");
        let alone = step(vec![forager()], &[], &params, seed);
        if toroidal_distance(carcass_only, litter[0].position, EXTENT)
            < toroidal_distance(alone, litter[0].position, EXTENT)
        {
            toward += 1;
        }
    }
    assert_eq!(
        toward, SEEDS,
        "the carcass pulled on {toward} of {SEEDS} seeds"
    );
}

#[test]
fn at_full_resistance_a_forager_sensing_only_living_targets_random_walks() {
    // Producers all around, unequal so they would pull: at ρ = 1 none does,
    // the pull is 0 and the step is the forager's step alone.
    let params = params(1.0);
    for seed in 0..SEEDS {
        let among_producers = step(
            vec![
                forager(),
                producer(1, (2.0, 0.0), 40.0),
                producer(2, (0.0, -3.0), 12.0),
                producer(3, (-1.0, 1.5), 3.0),
            ],
            &[],
            &params,
            seed,
        );
        let alone = step(vec![forager()], &[], &params, seed);
        assert_eq!(bits(among_producers), bits(alone), "seed {seed}");
    }
}

#[test]
fn at_partial_resistance_a_living_targets_cue_is_scaled_by_one_minus_rho() {
    // A living producer of structure S on +x balances a carcass of energy
    // S(1 − ρ) at the same distance on −x: the pull is 0, so the step is the
    // forager's step alone.
    for rho in [0.25_f32, 0.6, 0.9] {
        let params = params(rho);
        let structure = 8.0_f32;
        let litter = [carcass(9, (-2.0, 0.0), structure * (1.0 - rho))];
        for seed in 0..SEEDS {
            let balanced = step(
                vec![forager(), producer(1, (2.0, 0.0), structure)],
                &litter,
                &params,
                seed,
            );
            let alone = step(vec![forager()], &[], &params, seed);
            assert_eq!(bits(balanced), bits(alone), "rho {rho}, seed {seed}");
        }
    }
}

#[test]
fn a_carcass_cue_is_unchanged_by_resistance() {
    // The same litter, at ρ = 0, partial and 1: the forager steps the same,
    // bit for bit.
    let litter = [
        carcass(9, (-2.0, 0.5), 3.0),
        carcass(10, (1.0, 2.0), 7.5),
        carcass(11, (0.5, -1.5), 0.4),
    ];
    for seed in 0..SEEDS {
        let reference = step(vec![forager()], &litter, &params(0.0), seed);
        for rho in [0.4_f32, 1.0] {
            let resisted = step(vec![forager()], &litter, &params(rho), seed);
            assert_eq!(bits(resisted), bits(reference), "rho {rho}, seed {seed}");
        }
    }
}
