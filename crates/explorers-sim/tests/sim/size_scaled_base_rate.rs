//! The size-scaled base metabolic rate (#807; world-rules.md flow 8, *The
//! base rate scales with structure*, *`β` is 1*, *How large `b` is*).
//!
//! With `mass_specific_base_rate` (`b`) positive, an agent's base cost is
//! `b · S`, in place of both the per-agent `base_metabolic_rate` and the
//! linear `structure_maintenance_coefficient · S` term, which `b` absorbs.
//! Trait maintenance and the cross-trait cost are unchanged. At `b = 0` the
//! per-agent rule holds.

use crate::support;

use explorers_sim::phase::{metabolic_cost, metabolise};
use explorers_sim::{Agent, TraitVector, WorldParameters};

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

fn params(b: f32) -> WorldParameters {
    WorldParameters {
        base_metabolic_rate: 0.3,
        structure_maintenance_coefficient: 0.01,
        photo_maintenance_cost: 0.02,
        heterotrophy_maintenance_cost: 0.03,
        mobility_maintenance_cost: 0.04,
        asexual_propensity_maintenance_cost: 0.05,
        cross_trait_cost: 0.06,
        mass_specific_base_rate: b,
        ..support::viable_baseline()
    }
}

/// The trait maintenance and cross-trait cost alone: `metabolic_cost` with no
/// base rate and no structure term, under the per-agent rule.
fn trait_terms(a: &Agent, p: &WorldParameters) -> f32 {
    let bare = WorldParameters {
        base_metabolic_rate: 0.0,
        structure_maintenance_coefficient: 0.0,
        mass_specific_base_rate: 0.0,
        ..p.clone()
    };
    metabolic_cost(a, &bare)
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() <= 1e-5 * a.abs().max(b.abs()).max(1.0)
}

/// With every trait at 0, the whole cost is the base, `b · S`, at any size:
/// neither `base_metabolic_rate` nor the structure term is charged.
#[test]
fn base_cost_is_b_times_structure() {
    let bare = TraitVector {
        photosynthetic_absorption: 0.0,
        heterotrophy: 0.0,
        mobility: 0.0,
        asexual_propensity: 0.0,
        ..traits()
    };
    let b = 0.025;
    let p = params(b);
    for s in [0.5_f32, 2.459, 21.6, 100.0] {
        let cost = metabolic_cost(&agent(s, bare), &p);
        assert!(close(cost, b * s), "S {s}: cost {cost}, b·S {}", b * s);
    }
}

/// The structure term is folded into `b`, not charged on top of it: with
/// traits carried, the cost is `b · S` plus trait maintenance and the
/// cross-trait cost, whatever `structure_maintenance_coefficient` and
/// `base_metabolic_rate` are.
#[test]
fn structure_term_is_not_double_charged() {
    let b = 0.025;
    let s = 30.0;
    for (base, coefficient) in [(0.3, 0.01), (0.0, 0.0), (5.0, 2.0)] {
        let p = WorldParameters {
            base_metabolic_rate: base,
            structure_maintenance_coefficient: coefficient,
            ..params(b)
        };
        let a = agent(s, traits());
        let cost = metabolic_cost(&a, &p);
        let expected = b * s + trait_terms(&a, &p);
        assert!(
            close(cost, expected),
            "base {base}, coefficient {coefficient}: cost {cost}, expected {expected}"
        );
    }
}

/// *How large `b` is*: at `b = base_metabolic_rate / S_p +
/// structure_maintenance_coefficient`, an agent of structure `S_p` pays the
/// same cost as under the per-agent rule; smaller agents pay less and larger
/// ones more.
#[test]
fn an_agent_of_structure_s_p_pays_what_it_paid_under_the_per_agent_rule() {
    let off = params(0.0);
    let s_p = 21.6;
    let b = off.base_metabolic_rate / s_p + off.structure_maintenance_coefficient;
    let on = params(b);
    let at = |s: f32, p: &WorldParameters| metabolic_cost(&agent(s, traits()), p);
    assert!(
        close(at(s_p, &on), at(s_p, &off)),
        "at S_p: on {}, off {}",
        at(s_p, &on),
        at(s_p, &off)
    );
    assert!(at(2.459, &on) < at(2.459, &off));
    assert!(at(100.0, &on) > at(100.0, &off));
}

/// `metabolise` charges the size-scaled cost (capped at the reserve as
/// ever), and at `b = 0` it charges the per-agent rule exactly.
#[test]
fn metabolise_charges_the_size_scaled_cost() {
    let a = agent(10.0, traits());
    let b = 0.025;
    let on = params(b);
    let mut agents = vec![a.clone()];
    let (_, dissipated) = metabolise(&mut agents, &on);
    assert_eq!(dissipated.to_bits(), metabolic_cost(&a, &on).to_bits());
    assert!(close(dissipated, b * 10.0 + trait_terms(&a, &on)));
    assert!(close(agents[0].reserve, 100.0 - dissipated));

    let off = params(0.0);
    let (_, dissipated) = metabolise(&mut vec![a.clone()], &off);
    let per_agent = off.base_metabolic_rate
        + trait_terms(&a, &off)
        + 10.0 * off.structure_maintenance_coefficient;
    assert!(close(dissipated, per_agent), "{dissipated} vs {per_agent}");
}
