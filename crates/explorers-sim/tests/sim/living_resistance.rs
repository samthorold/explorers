//! Living tissue resists drains (#813; world-rules.md, *Consumption is
//! incremental → Living tissue resists drains*).
//!
//! Every drain on a living target is multiplied by `(1 − ρ)`, `ρ` the world's
//! `living_resistance` in `[0, 1]`, after expression and recognition and
//! before the proportional split among co-feeders. Carcasses carry no `ρ`,
//! and `ρ = 0` is the world without resistance, a controlled variant. It is
//! physics in every world since #865, at the domain's `ρ = 1`
//! ([`DEFAULT_LIVING_RESISTANCE`]). The rung test of S25 (end-state.md,
//! part 1) is `s25_rung_…`.

use crate::support;

use explorers_sim::phase::{DrainResult, move_agents, resolve_drains};
use explorers_sim::spatial::{NutrientGrid, SpatialGrid};
use explorers_sim::{Agent, Carcass, DEFAULT_LIVING_RESISTANCE, TraitVector, WorldParameters};

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

/// The world's own parameters: the viable baseline, whose `ρ` is the
/// world's default.
fn world_params() -> WorldParameters {
    WorldParameters {
        world_extent: EXTENT,
        ..support::viable_baseline()
    }
}

/// The producer, dead: a carcass of its structure and traits at `SITE`.
fn dead_producer(structure: f32) -> Carcass {
    Carcass {
        id: TARGET,
        ..carcass(structure)
    }
}

/// Structure each consumer drained from the carcass `TARGET` this pass.
fn carcass_drains(result: &DrainResult) -> Vec<(u64, f32)> {
    let mut drains: Vec<(u64, f32)> = result
        .events
        .iter()
        .filter(|e| e.target_was_carcass && e.target == Some(TARGET))
        .map(|e| (e.source, e.energy_delta))
        .collect();
    drains.sort_by_key(|&(id, _)| id);
    drains
}

/// A foraging decomposer at `SITE`, apart from the drain scenes' consumers:
/// senses 5.0 (mobility 0.5 × `sensing_range_coefficient` 10) and steers
/// with heterotrophy 0.8, far in trait space from the producer.
fn forager() -> Agent {
    Agent {
        id: 0,
        traits: TraitVector {
            mobility: 0.5,
            ..traits(0.8)
        },
        ..consumer(0, 0.8)
    }
}

fn cue_params(base: WorldParameters) -> WorldParameters {
    WorldParameters {
        sensing_range_coefficient: 10.0,
        movement_cost_coefficient: 0.0,
        ..base
    }
}

/// The forager's position after one movement tick among `living` (forager
/// first) and `carcasses`.
fn step(
    mut living: Vec<Agent>,
    carcasses: &[Carcass],
    params: &WorldParameters,
    seed: u64,
) -> (u32, u32) {
    let mut grid = SpatialGrid::new(EXTENT, 10.0);
    for (i, a) in living.iter().enumerate() {
        grid.insert(i as u64, a.position);
    }
    move_agents(&mut living, carcasses, &grid, params, seed, 0);
    let p = living[0].position;
    (p.0.to_bits(), p.1.to_bits())
}

/// S25 (end-state.md, part 1): living tissue resists every drain, and death
/// removes the resistance. At the world's own `ρ`, on a few agents:
/// - each consumer's drain on a living producer is `(1 − ρ)` of what it
///   drains with no resistance, alone and in a split among co-feeders;
/// - the producer's movement cue is scaled by the same `(1 − ρ)`: a forager
///   among it and a carcass steps as it would toward the carcass and a
///   producer of `(1 − ρ)` its structure in the world without resistance;
/// - the same producer, dead, is drained, and pulls, exactly as with no
///   resistance.
#[test]
fn s25_rung_a_living_target_resists_its_drains_and_its_cue_at_the_worlds_resistance_and_its_carcass_does_not()
 {
    let world = world_params();
    let rho = world.living_resistance;
    assert_eq!(rho, DEFAULT_LIVING_RESISTANCE, "the world's own ρ");
    assert_eq!(rho, 1.0, "the domain closes living tissue (#865)");
    let open = WorldParameters {
        living_resistance: 0.0,
        ..world
    };

    // Drains on a living producer, ample and scarce.
    for structure in [500.0, 1.0] {
        let run = |p: &WorldParameters| {
            let mut agents = vec![consumer(1, 1.2), consumer(2, 0.7), producer(structure)];
            let before = agents[2].structure;
            let drains = living_drains(&drain(&mut agents, &mut [], p));
            (drains, before - agents[2].structure)
        };
        let (open_drains, _) = run(&open);
        assert_eq!(open_drains.len(), 2, "S = {structure}: both drain, open");
        let (resisted, lost) = run(&world);
        let open_total: f32 = open_drains.iter().map(|&(_, d)| d).sum();
        for &(id, d) in &open_drains {
            let got = resisted
                .iter()
                .find(|&&(r, _)| r == id)
                .map_or(0.0, |&(_, r)| r);
            let bound = (1.0 - rho) * d;
            assert!(
                got <= bound + 1e-6 * open_total,
                "S = {structure}, consumer {id}: drained {got}, above (1 − ρ) × {d}"
            );
            if structure > open_total {
                // Ample: the split does not bind, so each takes its whole
                // resisted demand.
                assert!(
                    (got - bound).abs() <= 1e-6 * open_total,
                    "S = {structure}, consumer {id}: drained {got}, expected {bound}"
                );
            }
        }
        assert!(
            lost <= (1.0 - rho) * open_total + 1e-6 * open_total,
            "S = {structure}: the producer lost {lost}"
        );
    }

    // Death removes the resistance: the dead producer is drained bit for bit
    // as with no resistance.
    let carcass_pass = |p: &WorldParameters| {
        let mut agents = vec![consumer(1, 1.2), consumer(2, 0.7)];
        let mut carcasses = vec![dead_producer(40.0)];
        let drains = carcass_drains(&drain(&mut agents, &mut carcasses, p));
        let stocks = [carcasses[0].energy, carcasses[0].nutrient];
        (
            drains
                .iter()
                .map(|&(id, d)| (id, d.to_bits()))
                .collect::<Vec<_>>(),
            stocks.map(f32::to_bits),
        )
    };
    let open_carcass = carcass_pass(&open);
    assert_eq!(open_carcass.0.len(), 2, "both drain the carcass");
    assert_eq!(carcass_pass(&world), open_carcass, "a carcass carries no ρ");

    // The cue. A producer of structure S on +x, a carcass on −x.
    let world_cue = cue_params(world);
    let open_cue = cue_params(open);
    let structure = 8.0_f32;
    let mut living = producer(structure);
    living.position = (SITE.0 + 2.0, SITE.1);
    let mut litter = dead_producer(3.0);
    litter.id = 99;
    litter.position = (SITE.0 - 2.0, SITE.1);
    let mut scaled = living.clone();
    scaled.structure *= 1.0 - rho;
    scaled.peak_structure = scaled.structure;
    for seed in 0..20 {
        let resisted = step(
            vec![forager(), living.clone()],
            std::slice::from_ref(&litter),
            &world_cue,
            seed,
        );
        let reference_living = if scaled.structure > 0.0 {
            vec![forager(), scaled.clone()]
        } else {
            vec![forager()]
        };
        let reference = step(
            reference_living,
            std::slice::from_ref(&litter),
            &open_cue,
            seed,
        );
        assert_eq!(resisted, reference, "seed {seed}: the living cue is scaled");
        // The carcass's cue is not: alone with the forager, it steps the
        // same at the world's ρ and with none.
        assert_eq!(
            step(
                vec![forager()],
                std::slice::from_ref(&litter),
                &world_cue,
                seed
            ),
            step(
                vec![forager()],
                std::slice::from_ref(&litter),
                &open_cue,
                seed
            ),
            "seed {seed}: the carcass cue carries no ρ"
        );
        assert_ne!(
            step(
                vec![forager()],
                std::slice::from_ref(&litter),
                &world_cue,
                seed
            ),
            step(vec![forager()], &[], &world_cue, seed),
            "seed {seed}: the carcass pulls"
        );
    }
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
