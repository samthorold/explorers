//! The base rate is proportional to structure (#807, in every world since
//! #862; world-rules.md flow 8, *The base rate scales with structure*, *`β` is
//! 1*, *How large `b` is*; end state S22).
//!
//! An agent's base cost is `b · S`, `S` its structure, at the mass-specific
//! rate `b = base_metabolic_rate / S_p + structure_maintenance_coefficient`,
//! `S_p` the world's `base_rate_reference_structure`: the structure of the
//! median established producer, which pays exactly what it paid under the
//! per-agent constant. `b` absorbs the linear structure term, which is not
//! charged on top. Trait maintenance and the cross-trait cost are unchanged.
//! It has no switch.

use crate::support;

use explorers_sim::phase::{metabolic_cost, metabolise};
use explorers_sim::{Agent, DEFAULT_BASE_RATE_REFERENCE_STRUCTURE, TraitVector, WorldParameters};

fn traits() -> TraitVector {
    TraitVector {
        photosynthetic_absorption: 0.4,
        heterotrophy: 0.3,
        mobility: 0.2,
        kappa: 0.5,
        fecundity: 0.0,
        asexual_propensity: 0.15,
        dispersal: 0.0,
    }
}

fn decomposer() -> TraitVector {
    TraitVector {
        photosynthetic_absorption: 0.0,
        heterotrophy: 1.0,
        mobility: 0.0,
        ..traits()
    }
}

fn agent(structure: f32, traits: TraitVector) -> Agent {
    Agent {
        id: 1,
        position: (50.0, 50.0),
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

fn params() -> WorldParameters {
    WorldParameters {
        base_metabolic_rate: 0.3,
        structure_maintenance_coefficient: 0.01,
        photo_maintenance_cost: 0.02,
        heterotrophy_maintenance_cost: 0.03,
        mobility_maintenance_cost: 0.04,
        asexual_propensity_maintenance_cost: 0.05,
        cross_trait_cost: 0.06,
        ..support::viable_baseline()
    }
}

/// The trait maintenance and cross-trait cost alone: `metabolic_cost` with
/// the base rate and the structure coefficient at 0, so `b` is 0.
fn trait_terms(a: &Agent, p: &WorldParameters) -> f32 {
    let bare = WorldParameters {
        base_metabolic_rate: 0.0,
        structure_maintenance_coefficient: 0.0,
        ..p.clone()
    };
    metabolic_cost(a, &bare)
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() <= 1e-5 * a.abs().max(b.abs()).max(1.0)
}

/// S22's rung (end-state.md, part 1): on a few agents, a producer and a
/// decomposer at sizes from a founder's to well past the established
/// producer's, what `metabolise` charges is `b · S` plus trait maintenance and
/// the cross-trait cost, at the declared `b`, to float rounding: linear in
/// structure, through 0, with no per-agent constant and no structure term
/// charged on top. An agent of the declared `S_p` pays the per-agent rule's
/// `base_metabolic_rate + structure_maintenance_coefficient · S_p`, so the
/// median established producer's cost is conserved. No switch is set: the
/// world's own parameters, at their defaults, declare the rule.
#[test]
fn s22_rung_upkeep_is_linear_in_structure_at_the_declared_mass_specific_rate_and_the_established_producers_cost_is_conserved()
 {
    let p = params();
    let s_p = p.base_rate_reference_structure;
    assert_eq!(s_p, DEFAULT_BASE_RATE_REFERENCE_STRUCTURE);
    let b = p.mass_specific_base_rate();
    assert!(close(
        b,
        p.base_metabolic_rate / s_p + p.structure_maintenance_coefficient
    ));

    for traits in [traits(), decomposer()] {
        for s in [0.5_f32, 2.4588, s_p, 21.6, 100.0] {
            let a = agent(s, traits);
            let mut agents = vec![a.clone()];
            let (_, dissipated) = metabolise(&mut agents, &p);
            let base = dissipated - trait_terms(&a, &p);
            assert!(close(base, b * s), "S {s}: base cost {base}, b·S {}", b * s);
            assert!(close(agents[0].reserve, 100.0 - dissipated));
        }
        // Linear: the cost of the base doubles with the body.
        let base_at = |s: f32| {
            let a = agent(s, traits);
            metabolic_cost(&a, &p) - trait_terms(&a, &p)
        };
        assert!(close(base_at(20.0), 2.0 * base_at(10.0)));
        assert!(close(base_at(0.0), 0.0));

        // Conserved at S_p: the per-agent rule's cost there.
        let per_agent = p.base_metabolic_rate + p.structure_maintenance_coefficient * s_p;
        assert!(
            close(base_at(s_p), per_agent),
            "at S_p: {} against the per-agent {per_agent}",
            base_at(s_p)
        );
        // Smaller bodies pay less than under the per-agent rule, larger more.
        assert!(base_at(2.4588) < p.base_metabolic_rate + 0.01 * 2.4588);
        assert!(base_at(100.0) > p.base_metabolic_rate + 0.01 * 100.0);
    }
}

/// The structure term is folded into `b`, not charged on top of it: the cost
/// is `b · S` plus the trait terms whatever `base_metabolic_rate`,
/// `structure_maintenance_coefficient` and `S_p` are.
#[test]
fn structure_term_is_not_double_charged() {
    let s = 30.0;
    for (base, coefficient, s_p) in [(0.3, 0.01, 6.937), (0.0, 0.0, 6.937), (5.0, 2.0, 50.0)] {
        let p = WorldParameters {
            base_metabolic_rate: base,
            structure_maintenance_coefficient: coefficient,
            base_rate_reference_structure: s_p,
            ..params()
        };
        let a = agent(s, traits());
        let cost = metabolic_cost(&a, &p);
        let expected = (base / s_p + coefficient) * s + trait_terms(&a, &p);
        assert!(
            close(cost, expected),
            "base {base}, coefficient {coefficient}, S_p {s_p}: cost {cost}, expected {expected}"
        );
    }
}
