//! Property checks for the energy-bound derivation in
//! `docs/research/433-energy-bound.md` (#433). Each property is one lemma of
//! that note, checked against the real stepper over the search domain so the
//! proof stays falsifiable rather than a paper exercise.

mod support;

use explorers_sim::World;
use proptest::prelude::*;
use support::{WorldCase, world_case_integer_exponent};

/// Relative slack for f32 summation of a few hundred per-tick flows.
const REL_TOLERANCE: f32 = 1e-4;

/// Lemma 1 of the note: with light-competition radius `r > 0` on a torus of
/// extent `L`, a square cell of side `< r/√2` has every pair of its points within
/// `r`, so `m = ⌊√2·L/r⌋ + 1` cells per axis tile the torus into `m²` cells whose
/// producers each share at most one flux `F`. Total per-tick solar income is
/// therefore at most `F · min(number of producers, m²)`.
fn solar_income_cap(case: &WorldCase, producers_at_tick_start: usize) -> f32 {
    let p = &case.params;
    let m = (std::f32::consts::SQRT_2 * p.world_extent / p.light_competition_radius).floor() + 1.0;
    let cells = m * m;
    p.solar_flux_magnitude * (producers_at_tick_start as f32).min(cells)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Per-tick total solar input never exceeds the config-only cap of Lemma 1.
    #[test]
    fn per_tick_solar_input_never_exceeds_cell_cap(case in world_case_integer_exponent()) {
        let mut world = World::new(case.params.clone(), case.dist.clone(), case.seed);
        for tick in 0..case.ticks {
            let producers = world
                .agents()
                .iter()
                .filter(|a| a.traits.photosynthetic_absorption > 0.0 && a.structure > 0.0)
                .count();
            let cap = solar_income_cap(&case, producers);
            let before = world.total_solar_input();
            world.step();
            let income = world.total_solar_input() - before;
            prop_assert!(
                income <= cap * (1.0 + REL_TOLERANCE),
                "tick {tick}: solar income {income} exceeds cap {cap} (producers={producers})"
            );
        }
    }
}

/// Total system energy: living (reserve + reproductive allocation + structure)
/// plus carcass-held energy.
fn total_energy(world: &World) -> f32 {
    let living: f32 = world
        .agents()
        .iter()
        .map(|a| a.reserve + a.repro_reserve + a.structure)
        .sum();
    let carcass: f32 = world.carcasses().iter().map(|c| c.energy).sum();
    living + carcass
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Lemma 2 of the note: every agent alive at the start of a tick that is
    /// still alive at its end paid its full metabolic cost (at least the base
    /// rate `B`) to heat, and no other flow creates energy, so
    /// `E_tot(t+1) ≤ E_tot(t) + P(t) − B · N_s(t)` with `N_s` the tick's
    /// survivors. Deaths only move energy to carcasses or heat, so this is
    /// independent of the #445 starvation-overdraft accounting bug.
    ///
    /// The maintenance exponent is pinned even (2) because `World::new` can seed
    /// negative founder traits (#444), and a negative trait under an odd
    /// exponent makes its maintenance term *negative* — metabolism then pays
    /// the agent, a second energy tap outside the committed rules (recorded in
    /// the note as obstruction O3). The lemma is stated for the committed
    /// non-negative trait domain, which an even exponent restores.
    #[test]
    fn total_energy_drops_by_base_rate_per_survivor(case in world_case_integer_exponent()) {
        let mut params = case.params.clone();
        params.maintenance_cost_exponent = 2.0;
        let mut world = World::new(params, case.dist.clone(), case.seed);
        let b = case.params.base_metabolic_rate;
        for tick in 0..case.ticks {
            let before_ids: std::collections::HashSet<u64> =
                world.agents().iter().map(|a| a.id).collect();
            let e_before = total_energy(&world);
            let solar_before = world.total_solar_input();
            world.step();
            let income = world.total_solar_input() - solar_before;
            let survivors = world
                .agents()
                .iter()
                .filter(|a| before_ids.contains(&a.id))
                .count();
            let e_after = total_energy(&world);
            let bound = e_before + income - b * survivors as f32;
            let slack = (e_before + income).abs().max(1.0) * REL_TOLERANCE;
            prop_assert!(
                e_after <= bound + slack,
                "tick {tick}: E_tot {e_after} exceeds {bound} \
                 (E_before={e_before}, solar={income}, survivors={survivors})"
            );
        }
    }
}

/// Obstruction O1 of the note, exhibited: a lone producer with `kappa < 1`,
/// zero asexual propensity and no mate routes the (1 − kappa) share of every
/// tick's surplus into a reproductive allocation that nothing ever draws down,
/// taxes, or caps. Its living energy grows by a fixed positive amount every
/// tick, so no `E_max` in the committed parameters can bound `E_living(t)`.
#[test]
fn lone_producer_hoards_reproductive_allocation_without_bound() {
    use explorers_sim::{AgentSpec, TraitVector, WorldRecipe};
    let recipe = WorldRecipe {
        parameters: support::viable_baseline(),
        initial_distribution: None,
        agents: Some(vec![AgentSpec {
            position: (0.0, 0.0),
            reserve: 10.0,
            traits: TraitVector {
                photosynthetic_absorption: 1.0,
                heterotrophy: 0.0,
                mobility: 0.0,
                kappa: 0.5,
                fecundity: 1.0,
                asexual_propensity: 0.0,
                dispersal: 0.0,
            },
            nutrient: 1.0,
        }]),
        carcasses: None,
        max_ticks: 0,
    };
    let mut world = World::from_recipe(&recipe, 0);
    let living = |w: &World| -> f32 {
        w.agents()
            .iter()
            .map(|a| a.reserve + a.repro_reserve + a.structure)
            .sum()
    };
    // Past the transient the per-tick gain settles toward a positive limit
    // (structure, the only taxed stock, saturates; the allocation does not).
    for _ in 0..50 {
        world.step();
    }
    assert_eq!(world.agents().len(), 1, "the lone producer must persist");
    let e_50 = living(&world);
    let allocation_50 = world.agents()[0].repro_reserve;
    for _ in 0..50 {
        world.step();
    }
    let e_100 = living(&world);
    let allocation_100 = world.agents()[0].repro_reserve;
    for _ in 0..100 {
        world.step();
    }
    let e_200 = living(&world);
    let allocation_200 = world.agents()[0].repro_reserve;

    let gain_a = e_100 - e_50;
    let gain_b = (e_200 - e_100) / 2.0;
    assert!(
        gain_a > 0.0,
        "living energy must keep rising (gain {gain_a})"
    );
    assert!(
        gain_b > 0.8 * gain_a,
        "gain per 50 ticks must not collapse: {gain_a} then {gain_b}"
    );
    assert!(
        allocation_200 > allocation_100 && allocation_100 > allocation_50,
        "the growth is the untaxed reproductive allocation: {allocation_50} < {allocation_100} < {allocation_200}"
    );
    assert!(
        allocation_200 > recipe.parameters.reproduction_energy_threshold,
        "the allocation is past the reproduction threshold ({allocation_200}) yet no event fires"
    );
}
