//! Growth dilutes wear (#840; world-rules.md *Somatic wear*, *Growth dilutes
//! wear*).
//!
//! New structure is built unworn, so under `growth_dilutes_wear` a tick of
//! growth that takes structure from `S` to `S + ΔS` scales the wear on every
//! functional trait by `S / (S + ΔS)`. Loss of structure leaves wear
//! unchanged, and the rule applies to every agent, producers included. With
//! the switch off nothing of this runs.

use crate::support;

use explorers_sim::{Agent, InitialDistribution, TraitVector, World, WorldParameters, phase};

fn params(growth_dilutes_wear: bool) -> WorldParameters {
    WorldParameters {
        growth_dilutes_wear,
        // Repair off, so a tick of growth is the only thing moving wear.
        repair_rate: 0.0,
        ..support::viable_baseline()
    }
}

fn producer() -> TraitVector {
    TraitVector {
        photosynthetic_absorption: 1.0,
        heterotrophy: 0.0,
        mobility: 0.0,
        kappa: 0.8,
        fecundity: 1.0,
        asexual_propensity: 0.0,
        dispersal: 0.0,
    }
}

fn decomposer() -> TraitVector {
    TraitVector {
        photosynthetic_absorption: 0.0,
        heterotrophy: 1.0,
        ..producer()
    }
}

/// A well-fed worn agent of structure `structure`, with nutrient to spare so
/// its growth is energy-limited.
fn worn(traits: TraitVector, structure: f32) -> Agent {
    let mut a = Agent::new(1, (0.0, 0.0), 100.0, structure, 1000.0, traits);
    a.wear = [0.3, 0.2, 0.1];
    a
}

#[track_caller]
fn assert_close(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() <= 1e-6 * expected.abs().max(1.0),
        "{what}: {actual} != {expected}"
    );
}

#[test]
fn a_tick_of_growth_scales_each_traits_wear_by_s_over_s_plus_delta_s() {
    for (kind, traits) in [("producer", producer()), ("decomposer", decomposer())] {
        let mut agents = vec![worn(traits, 10.0)];
        let before = agents[0].clone();
        phase::grow(&mut agents, &params(true));
        let after = &agents[0];
        assert!(after.structure > before.structure, "{kind} grew");
        let factor = before.structure / after.structure;
        for ft in 0..3 {
            assert_close(after.wear[ft], before.wear[ft] * factor, kind);
        }
        assert!(after.wear[0] < before.wear[0], "{kind}'s wear diluted");
    }
}

#[test]
fn with_the_switch_off_growth_leaves_wear_alone() {
    let mut agents = vec![worn(producer(), 10.0)];
    let before = agents[0].clone();
    phase::grow(&mut agents, &params(false));
    assert!(agents[0].structure > before.structure, "it grew");
    assert_eq!(agents[0].wear, before.wear);
}

#[test]
fn a_body_with_no_structure_keeps_its_wear_when_it_grows() {
    // S = 0: nothing worn to spread the new structure over.
    let mut agents = vec![worn(decomposer(), 0.0)];
    let before = agents[0].clone();
    phase::grow(&mut agents, &params(true));
    assert!(agents[0].structure > 0.0, "it grew");
    assert_eq!(agents[0].wear, before.wear);
}

#[test]
fn a_fall_in_structure_leaves_wear_unchanged() {
    // A starving body burns structure for its upkeep (the starvation
    // cascade), then has no surplus to grow with: its structure falls and its
    // wear stays where it was.
    let p = WorldParameters {
        starvation_shrinks: true,
        ..params(true)
    };
    for traits in [producer(), decomposer()] {
        let mut a = worn(traits, 10.0);
        a.reserve = 0.0;
        let mut agents = vec![a];
        let before = agents[0].clone();
        phase::metabolise(&mut agents, &p);
        phase::grow(&mut agents, &p);
        assert!(agents[0].structure < before.structure, "it shrank");
        assert_eq!(agents[0].wear, before.wear);
    }
}

/// Step one agent through repair, growth and wear accumulation for `ticks`
/// ticks, refilling its reserve each tick in proportion to its structure, so a
/// body free to grow does so at a steady relative rate; returns
/// the wear on its first functional trait after each tick, with the
/// structure it had before and after that tick's growth.
fn wear_path(nutrient: f32, ticks: usize) -> Vec<(f32, f32, f32, f32)> {
    let p = WorldParameters {
        repair_rate: 0.1,
        wear_rate: 0.01,
        structure_maintenance_coefficient: 0.0,
        ..params(true)
    };
    let mut a = worn(producer(), 10.0);
    a.nutrient = nutrient;
    let mut agents = vec![a];
    let usage = std::collections::HashMap::new();
    let mut path = Vec::new();
    for _ in 0..ticks {
        agents[0].reserve = 1.0 + 0.1 * agents[0].structure;
        let (w, s) = (agents[0].wear[0], agents[0].structure);
        phase::grow(&mut agents, &p);
        let s_after = agents[0].structure;
        phase::apply_wear(&mut agents, &p, &usage);
        path.push((w, s, s_after, agents[0].wear[0]));
    }
    path
}

#[test]
fn a_body_that_keeps_growing_settles_at_lower_wear_than_one_held_at_fixed_size() {
    // Held at fixed size by its nutrient (growth is co-limited, so with no
    // free nutrient it builds nothing), the same body repairs the same and
    // ages as before.
    let growing = wear_path(1.0e9, 300);
    let fixed = wear_path(0.0, 300);
    let (rho_kappa, a) = (0.1 * producer().kappa, 0.01);
    for (t, (g, f)) in growing.iter().zip(&fixed).enumerate() {
        let (w, s, s_after, w_after) = *g;
        assert!(s_after > s, "tick {t}: it keeps growing");
        assert_eq!(f.1, f.2, "tick {t}: the fixed body does not grow");
        // Per tick: repair, then dilution by S / (S + ΔS), then accumulation.
        // To first order that is Δw = a − ρκw − (ΔS/S)w.
        assert_close(w_after, w * (1.0 - rho_kappa) * s / s_after + a, "growing");
        assert_close(f.3, f.0 * (1.0 - rho_kappa) + a, "fixed");
        assert!(w_after < f.3, "tick {t}: {w_after} !< {}", f.3);
    }
    // The fixed body settles at its repair equilibrium; the growing one well
    // below it.
    let w_fixed = fixed.last().unwrap().3;
    let w_star = a / rho_kappa;
    assert!(
        (w_fixed - w_star).abs() < 0.02 * w_star,
        "{w_fixed} vs {w_star}"
    );
    assert!(growing.last().unwrap().3 < 0.9 * w_fixed);
}

fn world(growth_dilutes_wear: bool, wear_rate: f32) -> World {
    let p = WorldParameters {
        growth_dilutes_wear,
        wear_rate,
        initial_population_size: 30,
        world_extent: 40.0,
        ..support::viable_baseline()
    };
    let dist = InitialDistribution {
        mean_traits: TraitVector {
            mobility: 0.3,
            ..producer()
        },
        trait_covariance: 0.1,
        initial_cluster_count: 2,
        initial_energy_per_agent: 50.0,
        founder_aggregation: explorers_sim::DEFAULT_FOUNDER_AGGREGATION,
    };
    World::new(p, dist, 11)
}

/// Every agent's state, bit for bit, sorted by id.
fn state(world: &World) -> Vec<(u64, [u32; 2], [u32; 6], [u32; 3])> {
    let mut v: Vec<_> = (world.agents().iter())
        .map(|a| {
            (
                a.id,
                [a.position.0.to_bits(), a.position.1.to_bits()],
                [
                    a.reserve,
                    a.structure,
                    a.peak_structure,
                    a.nutrient,
                    a.repro_reserve,
                    a.repro_nutrient,
                ]
                .map(f32::to_bits),
                a.wear.map(f32::to_bits),
            )
        })
        .collect();
    v.sort_by_key(|x| x.0);
    v
}

#[test]
fn with_no_wear_to_dilute_the_switch_changes_nothing() {
    // The switch draws no randomness and moves nothing but wear: on a world
    // with no wear, on and off are the same world state for state.
    let (mut on, mut off) = (world(true, 0.0), world(false, 0.0));
    let mut grew = false;
    for _ in 0..80 {
        let before: f32 = on.agents().iter().map(|a| a.structure).sum();
        on.step();
        off.step();
        grew |= on.agents().iter().map(|a| a.structure).sum::<f32>() > before;
        assert_eq!(state(&on), state(&off), "tick {}", on.tick());
    }
    assert!(grew, "the run has growth");
}

#[test]
fn in_a_world_with_wear_dilution_lowers_the_wear_of_growing_agents() {
    let (mut on, mut off) = (world(true, 0.01), world(false, 0.01));
    for _ in 0..30 {
        on.step();
        off.step();
    }
    let mean_wear = |w: &World| {
        let n = w.agents().len().max(1) as f32;
        w.agents().iter().map(|a| a.wear[0]).sum::<f32>() / n
    };
    assert!(!on.agents().is_empty() && !off.agents().is_empty());
    assert!(
        mean_wear(&on) < mean_wear(&off),
        "{} !< {}",
        mean_wear(&on),
        mean_wear(&off)
    );
}
