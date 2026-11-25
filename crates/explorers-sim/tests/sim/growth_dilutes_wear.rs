//! Growth dilutes wear (#840, in every world since #858; world-rules.md
//! *Somatic wear*, *Growth dilutes wear*; end state S20).
//!
//! New structure is built unworn, so a tick of growth that takes structure
//! from `S` to `S + ΔS` scales the wear on every functional trait by
//! `S / (S + ΔS)`. Loss of structure leaves wear unchanged, and the rule
//! applies to every agent, producers included. It has no switch.

use crate::support;

use explorers_sim::{Agent, TraitVector, WorldParameters, phase};

fn params() -> WorldParameters {
    WorldParameters {
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

/// S20's rung test (end-state.md, part 1, *Growth dilutes wear*): in the
/// world's own physics, with no switch to set, a growing agent's wear falls
/// by the share of its body that is new tissue, `ΔS / (S + ΔS)`, and a body
/// of steady size gets none. Three agents in one growth phase: a producer and
/// a decomposer that grow, and a decomposer held at steady size by its
/// nutrient (growth is co-limited, so with no free nutrient it builds
/// nothing). Repair is off so growth is the only thing moving wear; repair's
/// own rule is S5's.
#[test]
fn s20_rung_a_growing_agents_wear_falls_by_the_share_of_new_tissue_and_a_steady_body_gets_none() {
    let p = params();
    let mut steady = worn(decomposer(), 10.0);
    steady.nutrient = 0.0;
    let mut agents = vec![worn(producer(), 10.0), worn(decomposer(), 4.0), steady];
    let before = agents.clone();
    phase::grow(&mut agents, &p);

    for (b, a) in before.iter().zip(&agents).take(2) {
        assert!(a.structure > b.structure, "a growing agent grew");
        let new_share = (a.structure - b.structure) / a.structure;
        for ft in 0..3 {
            assert_close(
                b.wear[ft] - a.wear[ft],
                b.wear[ft] * new_share,
                "wear falls by the share of new tissue",
            );
        }
    }
    assert_eq!(agents[2].structure, before[2].structure, "steady size");
    assert_eq!(agents[2].wear, before[2].wear, "a steady body gets none");
}

#[test]
fn a_body_with_no_structure_keeps_its_wear_when_it_grows() {
    // S = 0: nothing worn to spread the new structure over.
    let mut agents = vec![worn(decomposer(), 0.0)];
    let before = agents[0].clone();
    phase::grow(&mut agents, &params());
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
        ..params()
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
    // No base rate: since #862 it is `b · S`, so a retention buffer that grew
    // with the body would eat the refill and stall the steady relative growth.
    let p = WorldParameters {
        repair_rate: 0.1,
        wear_rate: 0.01,
        base_metabolic_rate: 0.0,
        structure_maintenance_coefficient: 0.0,
        ..params()
    };
    let mut a = worn(producer(), 10.0);
    a.nutrient = nutrient;
    let mut agents = vec![a];
    let usage = rustc_hash::FxHashMap::default();
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
