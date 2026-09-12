//! Property-based conservation checks over random parameterisations (#431,
//! Workstream C1). The ledgers are already asserted on curated scenarios; here
//! they are checked as invariants of the physics across the search space.

mod support;

use explorers_sim::{InitialDistribution, TraitVector, World, WorldParameters};
use proptest::prelude::*;
use support::{WorldCase, world_case};

/// Relative f32 tolerance for the energy identity: f32 carries ~7 significant
/// digits and a ≤ 40-agent, ≤ 20-tick world performs a few thousand flows, so
/// rounding stays near 1e-5 of the budget; 1e-4 (the per-tick
/// `EnergyLedger::assert_balanced` bound) leaves an order of magnitude of
/// headroom without hiding the leaks this suite has already caught (#445).
const ENERGY_REL_TOLERANCE: f32 = 1e-4;

/// Relative tolerance for nutrient closure, in units of `N_total`. The baseline
/// pool (50000 over 9 cells) puts each cell near 5.5e3, whose f32 ulp is 4.9e-4;
/// every sub-ulp uptake from a cell rounds by up to half an ulp, and a 20-tick
/// world can boom to ~1e4 agent-ticks, bounding the accumulated rounding at
/// ~2.5. 1e-4 × N_total = 5 gives 2× headroom over that bound; a real leak
/// scales with throughput and blows through it within a few ticks.
const NUTRIENT_REL_TOLERANCE: f32 = 1e-4;

fn run(case: &WorldCase) -> World {
    let mut world = World::new(case.params.clone(), case.dist.clone(), case.seed);
    for _ in 0..case.ticks {
        world.step();
    }
    world
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Energy ledger identity over the full search domain: after `k` steps,
    /// `endowment + solar input == dissipated + retained (agents + carcasses)`.
    #[test]
    fn energy_ledger_identity_holds(case in world_case()) {
        check_energy_ledger_identity(&case)?;
    }

    /// Nutrient closure: after `k` steps,
    /// `available + living (free + earmark + structure-bound) + carcass-locked == N_total`.
    #[test]
    fn nutrient_closure_holds(case in world_case()) {
        check_nutrient_closure(&case)?;
    }

    /// Non-negativity, strict: after every step, no living agent holds a
    /// negative reserve, structure, free nutrient or reproductive-nutrient
    /// earmark, and the available pool is never negative. Ignored until #446
    /// (nutrient-limited growth leaves free nutrient at -1 ulp) is fixed;
    /// `stores_never_go_negative_beyond_rounding` runs meanwhile.
    #[test]
    #[ignore = "see #446"]
    fn stores_never_go_negative(case in world_case()) {
        check_stores_non_negative(&case, 0.0)?;
    }

    /// Non-negativity beyond rounding: as above, but free nutrient may carry the
    /// #446 residue — one ulp of the nutrient bound into structure, which is
    /// the magnitude of the `(n / ratio) * ratio` round trip that produces it.
    /// Reserve, structure, the earmark and the pool stay strictly non-negative.
    #[test]
    fn stores_never_go_negative_beyond_rounding(case in world_case()) {
        check_stores_non_negative(&case, f32::EPSILON)?;
    }
}

fn check_energy_ledger_identity(case: &WorldCase) -> Result<(), TestCaseError> {
    let endowment = case.dist.initial_energy_per_agent * case.params.initial_population_size as f32;
    let world = run(case);

    let input = endowment + world.total_solar_input();
    let retained_agents: f32 = world.agents().iter().map(|a| a.energy()).sum();
    let retained_carcasses: f32 = world.carcasses().iter().map(|c| c.energy).sum();
    let output = world.dissipated_energy() + retained_agents + retained_carcasses;

    let tolerance = input.abs().max(1.0) * ENERGY_REL_TOLERANCE;
    prop_assert!(
        (input - output).abs() <= tolerance,
        "energy ledger identity violated after {} ticks: input={input} \
         (endowment={endowment}, solar={}), output={output} (dissipated={}, \
         agents={retained_agents}, carcasses={retained_carcasses}), tolerance={tolerance}",
        case.ticks,
        world.total_solar_input(),
        world.dissipated_energy(),
    );
    Ok(())
}

fn check_nutrient_closure(case: &WorldCase) -> Result<(), TestCaseError> {
    let n_total = case.params.initial_nutrient_pool;
    let world = run(case);

    let available = world.nutrient_pool();
    let living: f32 = world
        .agents()
        .iter()
        .map(|a| a.nutrient_total(world.params()))
        .sum();
    let carcass_locked: f32 = world.carcasses().iter().map(|c| c.nutrient).sum();
    let total = available + living + carcass_locked;

    let tolerance = n_total.abs().max(1.0) * NUTRIENT_REL_TOLERANCE;
    prop_assert!(
        (total - n_total).abs() <= tolerance,
        "nutrient closure violated after {} ticks: N_total={n_total}, total={total} \
         (available={available}, living={living}, carcass_locked={carcass_locked}), \
         tolerance={tolerance}",
        case.ticks,
    );
    Ok(())
}

/// `free_nutrient_residue_ulps` scales the allowed negative free-nutrient
/// residue: `residue = ulps × max(1, bound_nutrient)`. Zero is strict.
fn check_stores_non_negative(
    case: &WorldCase,
    free_nutrient_residue_ulps: f32,
) -> Result<(), TestCaseError> {
    let mut world = World::new(case.params.clone(), case.dist.clone(), case.seed);
    for tick in 0..case.ticks {
        world.step();
        for a in world.agents() {
            let residue = free_nutrient_residue_ulps * a.bound_nutrient(world.params()).max(1.0);
            prop_assert!(
                a.reserve >= 0.0
                    && a.structure >= 0.0
                    && a.nutrient >= -residue
                    && a.repro_nutrient >= 0.0,
                "agent {} has a negative store after tick {tick}: reserve={}, \
                 structure={}, nutrient={}, repro_nutrient={}",
                a.id,
                a.reserve,
                a.structure,
                a.nutrient,
                a.repro_nutrient,
            );
        }
        prop_assert!(
            world.nutrient_pool() >= 0.0,
            "available pool is negative after tick {tick}: {}",
            world.nutrient_pool(),
        );
    }
    Ok(())
}

/// The proptest-shrunk minimal case from #444: every searched parameter at its
/// range minimum (notably `maintenance_cost_exponent = 1.5`), founder means at
/// zero with `trait_covariance = 0.1`, so `World::new` used to sample negative
/// specification traits whose `powf(1.5)` maintenance term is NaN.
fn issue_444_minimal_case() -> WorldCase {
    let params = WorldParameters {
        solar_flux_magnitude: 1.0,
        base_trophic_efficiency: 0.1,
        trophic_distance_decay: 0.1,
        reproduction_efficiency: 0.1,
        base_metabolic_rate: 0.01,
        movement_cost_coefficient: 0.001,
        sensing_range_coefficient: 1.0,
        reproduction_energy_threshold: 5.0,
        mutation_rate: 0.01,
        mutation_magnitude: 0.01,
        contact_range_coefficient: 0.5,
        world_extent: 20.0,
        initial_population_size: 5,
        light_competition_radius: 1.0,
        photo_maintenance_cost: 0.001,
        heterotrophy_maintenance_cost: 0.001,
        reproductive_compatibility_distance: 0.5,
        base_nutrient_ratio: 0.01,
        specification_nutrient_coefficient: 0.01,
        maintenance_cost_exponent: 1.5,
        growth_retention_multiplier: 1.0,
        offspring_structure_fraction: 0.05,
        reserve_mobilisation_rate: 0.05,
        ..support::viable_baseline()
    };
    let dist = InitialDistribution {
        mean_traits: TraitVector {
            photosynthetic_absorption: 0.0,
            heterotrophy: 0.0,
            mobility: 0.0,
            kappa: 0.0,
            fecundity: 0.35,
            asexual_propensity: 0.0,
            dispersal: 0.0,
        },
        trait_covariance: 0.1,
        initial_cluster_count: 1,
        initial_energy_per_agent: 1.0,
    };
    WorldCase {
        params,
        dist,
        seed: 0,
        ticks: 1,
    }
}

/// Founders live in the same non-negative trait domain as offspring (#444):
/// `World::new` floors every sampled dimension at zero, as mutation does.
#[test]
fn founders_never_carry_a_negative_trait() {
    let case = issue_444_minimal_case();
    for seed in 0..64u64 {
        let world = World::new(case.params.clone(), case.dist.clone(), seed);
        for agent in world.agents() {
            for dim in 0..TraitVector::NUM_DIMS {
                let v = agent.traits.get(dim);
                assert!(
                    v >= 0.0,
                    "seed {seed}: founder {} has negative trait dim {dim} = {v}",
                    agent.id
                );
            }
        }
    }
}

/// The #444 minimal case stays finite and balanced after one tick: no NaN in
/// dissipation or any reserve, and the energy identity holds.
#[test]
fn issue_444_minimal_case_is_finite_after_one_tick() {
    let case = issue_444_minimal_case();
    let world = run(&case);
    assert!(
        world.dissipated_energy().is_finite(),
        "dissipated_energy = {}",
        world.dissipated_energy()
    );
    for agent in world.agents() {
        assert!(agent.reserve.is_finite(), "agent {} reserve NaN", agent.id);
    }
    check_energy_ledger_identity(&case).unwrap();
}
