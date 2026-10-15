//! Living tissue resists drains (#813; world-rules.md, *Consumption is
//! incremental → Living tissue resists drains*).
//!
//! Every drain on a living target is multiplied by `(1 − ρ)`, `ρ` the world's
//! `living_resistance` in `[0, 1]`, after expression and recognition and
//! before the proportional split among co-feeders. Carcasses carry no `ρ`,
//! and `ρ = 0` is the world without resistance.

use crate::support;

use explorers_sim::phase::{DrainResult, resolve_drains};
use explorers_sim::spatial::{NutrientGrid, SpatialGrid};
use explorers_sim::{Agent, Carcass, TraitVector, WorldParameters};

const EXTENT: f32 = 100.0;
const SITE: (f32, f32) = (50.0, 50.0);
const TARGET: u64 = 7;

fn params(rho: f32) -> WorldParameters {
    WorldParameters {
        world_extent: EXTENT,
        living_resistance: rho,
        ..support::viable_baseline()
    }
}

fn traits(heterotrophy: f32) -> TraitVector {
    TraitVector {
        photosynthetic_absorption: 0.0,
        heterotrophy,
        mobility: 0.0,
        kappa: 0.5,
        fecundity: 0.0,
        asexual_propensity: 0.0,
        dispersal: 0.0,
    }
}

fn consumer(id: u64, heterotrophy: f32) -> Agent {
    Agent {
        id,
        position: SITE,
        reserve: 100.0,
        structure: 5.0,
        peak_structure: 5.0,
        nutrient: 5.0,
        traits: traits(heterotrophy),
        wear: Default::default(),
        repro_reserve: 0.0,
        repro_nutrient: 0.0,
        marker: Default::default(),
    }
}

/// A living producer at `SITE` holding `structure`.
fn producer(structure: f32) -> Agent {
    Agent {
        structure,
        peak_structure: structure,
        traits: TraitVector {
            photosynthetic_absorption: 1.0,
            ..traits(0.0)
        },
        ..consumer(TARGET, 0.0)
    }
}

/// One drain pass over `agents` and `carcasses`, all at `SITE`.
fn drain(agents: &mut [Agent], carcasses: &mut [Carcass], params: &WorldParameters) -> DrainResult {
    let mut grid = SpatialGrid::new(EXTENT, 10.0);
    for (i, a) in agents.iter().enumerate() {
        grid.insert(i as u64, a.position);
    }
    let mut nutrient_grid = NutrientGrid::new(EXTENT, 10.0, 0.0);
    resolve_drains(agents, carcasses, &grid, params, &mut nutrient_grid)
}

/// Structure each consumer drained from the living target this pass, by id.
fn living_drains(result: &DrainResult) -> Vec<(u64, f32)> {
    let mut drains: Vec<(u64, f32)> = result
        .events
        .iter()
        .filter(|e| !e.target_was_carcass && e.target == Some(TARGET))
        .map(|e| (e.source, e.energy_delta))
        .collect();
    drains.sort_by_key(|&(id, _)| id);
    drains
}

/// What one consumer of heterotrophy 1.2 drains from an ample living
/// producer at `rho`.
fn lone_drain(rho: f32) -> f32 {
    let mut agents = vec![consumer(1, 1.2), producer(50.0)];
    let result = drain(&mut agents, &mut [], &params(rho));
    living_drains(&result).iter().map(|&(_, d)| d).sum()
}

#[test]
fn a_drain_on_a_living_target_scales_by_one_minus_rho() {
    let open = lone_drain(0.0);
    assert!(open > 0.0, "without resistance the consumer drains");
    for rho in [0.25_f32, 0.5, 0.9] {
        let resisted = lone_drain(rho);
        let expected = (1.0 - rho) * open;
        assert!(
            (resisted - expected).abs() <= 1e-6 * open,
            "ρ = {rho}: drained {resisted}, expected (1 − ρ) × {open} = {expected}"
        );
    }
}

#[test]
fn at_rho_one_a_living_target_is_not_drained() {
    let mut agents = vec![consumer(1, 1.2), consumer(2, 0.8), producer(3.0)];
    let before = agents[2].structure;
    let result = drain(&mut agents, &mut [], &params(1.0));
    assert!(
        living_drains(&result).is_empty(),
        "no consumer drains a living target at ρ = 1"
    );
    assert_eq!(agents[2].structure, before);
    assert!(!result.dead_agents.contains(&TARGET));
}

fn carcass(structure: f32) -> Carcass {
    Carcass {
        id: 999,
        position: SITE,
        energy: structure,
        energy_at_death: structure,
        nutrient: 2.0,
        traits: TraitVector {
            photosynthetic_absorption: 1.0,
            ..traits(0.0)
        },
    }
}

/// Bit patterns of the carcass's stocks and of each consumer's carcass drain
/// in one pass at `rho`. The two consumers are also each other's living
/// targets, which `ρ` does resist, so only the carcass side is compared.
fn carcass_pass(rho: f32, k_d: f32, structure: f32) -> Vec<u32> {
    let mut p = params(rho);
    p.carcass_decay_constant = k_d;
    let mut agents = vec![consumer(1, 1.2), consumer(2, 0.7)];
    let mut carcasses = vec![carcass(structure)];
    let result = drain(&mut agents, &mut carcasses, &p);
    let mut drains: Vec<(u64, f32)> = result
        .events
        .iter()
        .filter(|e| e.target_was_carcass && e.target == Some(999))
        .map(|e| (e.source, e.energy_delta))
        .collect();
    drains.sort_by_key(|&(id, _)| id);
    [carcasses[0].energy, carcasses[0].nutrient]
        .into_iter()
        .chain(drains.into_iter().map(|(_, d)| d))
        .map(f32::to_bits)
        .collect()
}

#[test]
fn a_carcass_drain_is_unchanged_by_rho_on_the_additive_path() {
    // Ample (each takes its demand) and scarce (the split at the cap).
    for structure in [40.0, 1.0] {
        let open = carcass_pass(0.0, 0.0, structure);
        assert_eq!(open.len(), 4, "the consumers drain the carcass");
        assert_eq!(open, carcass_pass(1.0, 0.0, structure), "S = {structure}");
        assert_eq!(open, carcass_pass(0.6, 0.0, structure), "S = {structure}");
    }
}

#[test]
fn a_carcass_drain_is_unchanged_by_rho_on_the_saturating_path() {
    let open = carcass_pass(0.0, 0.4, 40.0);
    assert_eq!(open.len(), 4, "the consumers drain the carcass");
    assert_eq!(open, carcass_pass(1.0, 0.4, 40.0));
    assert_eq!(open, carcass_pass(0.6, 0.4, 40.0));
}

#[test]
fn the_split_among_co_feeders_on_a_living_target_uses_the_resisted_demands() {
    // Each consumer's open demand on the producer, read from an ample one.
    let rho = 0.5;
    let open: Vec<(u64, f32)> = {
        let mut agents = vec![consumer(1, 1.2), consumer(2, 0.7), producer(500.0)];
        living_drains(&drain(&mut agents, &mut [], &params(0.0)))
    };
    assert_eq!(open.len(), 2);
    let open_total: f32 = open.iter().map(|&(_, d)| d).sum();
    // A producer holding more than the resisted total but less than the open
    // one: open, the split binds and empties it; resisted, it does not bind.
    let available = 0.75 * open_total;
    let run = |rho: f32| {
        let mut agents = vec![consumer(1, 1.2), consumer(2, 0.7), producer(available)];
        let drains = living_drains(&drain(&mut agents, &mut [], &params(rho)));
        (drains, agents[2].structure)
    };

    let (split, left) = run(0.0);
    for (&(id, d), &(_, demand)) in split.iter().zip(&open) {
        let share = demand / open_total * available;
        assert!(
            (d - share).abs() <= 1e-5,
            "{id}: open split {d}, expected {share}"
        );
    }
    assert!(
        left.abs() <= 1e-5,
        "the open split empties the producer: {left}"
    );

    let (resisted, left) = run(rho);
    for (&(id, d), &(_, demand)) in resisted.iter().zip(&open) {
        let expected = (1.0 - rho) * demand;
        assert!(
            (d - expected).abs() <= 1e-5,
            "{id}: resisted drain {d}, expected its whole resisted demand {expected}"
        );
    }
    let expected_left = available - (1.0 - rho) * open_total;
    assert!(
        (left - expected_left).abs() <= 1e-5,
        "the resisted drains leave {expected_left}, left {left}"
    );
}
