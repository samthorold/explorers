//! Property-based symmetry checks over random parameterisations (#435,
//! Workstream C2): the symmetries the execution model commits to — agent-order
//! permutation, toroidal translation, extensive scaling — checked as invariants
//! of the stepper across the search space, reusing the C1 harness.

mod support;

use explorers_sim::{Agent, World};
use proptest::prelude::*;
use support::{WorldCase, world_case_integer_exponent};

/// Relative tolerance for *summed* world totals under a permutation. The
/// execution model commits every RNG-derived quantity and every agent's
/// identity as exactly order-invariant, but explicitly allows the coordinated
/// non-RNG phases (light competition, drain proportional split, nutrient
/// uptake) to accumulate their per-neighbour sums in slice order — so totals
/// built from them may differ by rounding (execution-model.md, "Re-seeding is
/// a one-time event"). 1e-5 is ~100 ulps: ample for ≤ 20 ticks of a ≤ 40-agent
/// world, and far below any real order leak (see #451, which showed up as a
/// position difference of ~1e-2 after one tick).
const SUMMED_TOTAL_REL_TOLERANCE: f32 = 1e-5;

/// One agent's state split by what the execution model commits under a
/// permutation: `exact` (identity, RNG-derived placement, traits, wear) must be
/// bit-identical; `summed` (the energy and nutrient stores accumulated in the
/// coordinated non-RNG phases) may differ by rounding.
#[derive(Debug, Clone, PartialEq)]
struct AgentSnapshot {
    exact: AgentExactBits,
    summed: [f32; 6],
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct AgentExactBits {
    id: u64,
    position: (u32, u32),
    traits: [u32; 7],
    wear: [u32; 3],
}

const SUMMED_FIELDS: [&str; 6] = [
    "reserve",
    "structure",
    "peak_structure",
    "nutrient",
    "repro_reserve",
    "repro_nutrient",
];

impl AgentSnapshot {
    fn of(a: &Agent) -> Self {
        AgentSnapshot {
            exact: AgentExactBits {
                id: a.id,
                position: (a.position.0.to_bits(), a.position.1.to_bits()),
                traits: std::array::from_fn(|d| a.traits.get(d).to_bits()),
                wear: std::array::from_fn(|f| a.wear[f].to_bits()),
            },
            summed: [
                a.reserve,
                a.structure,
                a.peak_structure,
                a.nutrient,
                a.repro_reserve,
                a.repro_nutrient,
            ],
        }
    }
}

/// The living population's state as a multiset (sorted by id).
fn population_multiset(world: &World) -> Vec<AgentSnapshot> {
    let mut v: Vec<AgentSnapshot> = world.agents().iter().map(AgentSnapshot::of).collect();
    v.sort_by(|a, b| a.exact.cmp(&b.exact));
    v
}

/// Run a case, optionally permuting agent order before every step.
fn run(case: &WorldCase, permute: bool) -> World {
    let mut world = World::new(case.params.clone(), case.dist.clone(), case.seed);
    for t in 0..case.ticks {
        if permute {
            // A different rotation each tick so the permutation is not a fixed
            // offset the keying could accidentally be invariant to.
            world.permute_agent_order_for_test((t as usize).wrapping_mul(7) + 1);
        }
        world.step();
    }
    world
}

fn assert_rel_close(name: &str, a: f32, b: f32, rel: f32) -> Result<(), TestCaseError> {
    let tolerance = a.abs().max(b.abs()).max(1.0) * rel;
    prop_assert!(
        (a - b).abs() <= tolerance,
        "{name} differs beyond rounding: {a} vs {b} (tolerance {tolerance})"
    );
    Ok(())
}

/// The #451 / #452 workaround domains. `sensing_range_coefficient = 0` makes
/// the chemotaxis term of `move_agents` inert (#451), so movement is the keyed
/// random walk alone. `contact_range_coefficient = 0` (with the baseline's
/// `body_reach_coefficient = 0`) gives every consumer zero feeding reach, so
/// `resolve_drains` fires only for *exactly* co-located pairs (#452). Founders
/// are placed at random f32 positions and never coincide; the one way a pair
/// can coincide is a zero-dispersal offspring landing on its parent, which the
/// two domains below rule out differently. Delete once both bugs are fixed.
fn without_chemotaxis_or_contact(mut case: WorldCase) -> WorldCase {
    case.params.sensing_range_coefficient = 0.0;
    case.params.contact_range_coefficient = 0.0;
    case
}

/// Reproduction disabled (unreachable energy threshold), so no offspring exist
/// to coincide with a parent. Covers acquisition, metabolism, growth, the
/// random walk, wear and death over the full trait-covariance range.
fn world_case_without_reproduction() -> impl Strategy<Value = WorldCase> {
    world_case_integer_exponent().prop_map(|mut c| {
        c.params.reproduction_energy_threshold = f32::INFINITY;
        without_chemotaxis_or_contact(c)
    })
}

/// Reproduction enabled, with the dispersal trait bounded away from zero so
/// offspring never land exactly on a parent: founders draw
/// `max(0, mean + N(0, cov))` with `mean ≥ 1.5, cov ≤ 0.25` (a 6σ event to
/// reach zero), and mutation adds `N(0, magnitude)` with `magnitude ≤ 0.2`
/// (5σ to reach zero from 1.0). Narrower than the search ranges on those three
/// dimensions only; every other dimension is the C1 domain.
fn world_case_with_reproduction() -> impl Strategy<Value = WorldCase> {
    (world_case_integer_exponent(), 1.5f32..=2.0, 0.1f32..=0.25, 0.01f32..=0.2).prop_map(
        |(mut c, dispersal, cov, magnitude)| {
            c.dist.mean_traits.dispersal = dispersal;
            c.dist.trait_covariance = cov;
            c.params.mutation_magnitude = magnitude;
            without_chemotaxis_or_contact(c)
        },
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Agent-order permutation over the full (finite, #444-scoped) domain:
    /// stepping a world with its agent slice permuted before every tick yields
    /// the same multiset of agent states, bit for bit, and the same ledger
    /// totals to rounding. Ignored until two sequential-update leaks are fixed:
    /// #451 (`move_agents` chemotaxis reads neighbours' already-moved positions)
    /// and #452 (`resolve_drains` stoichiometric need reads already-drained
    /// structure, so the retained/excreted nutrient split is order-dependent).
    /// `..._without_chemotaxis` covers everything else meanwhile.
    #[test]
    #[ignore = "see #451, #452"]
    fn trajectory_is_invariant_under_agent_order_permutation(
        case in world_case_integer_exponent()
    ) {
        check_order_permutation_invariance(&case)?;
    }

    /// As above on the domain where the two leaking phases are inert
    /// (chemotaxis off for #451, consumption unreachable for #452) and
    /// reproduction is off. Every remaining phase is per-agent or keyed on
    /// stable identity, so identity, position, traits and wear are
    /// bit-identical and every summed store and total is equal to rounding.
    #[test]
    fn trajectory_is_invariant_under_agent_order_permutation_without_reproduction(
        case in world_case_without_reproduction()
    ) {
        check_order_permutation_invariance(&case)?;
    }

    /// As above with reproduction on — the phase most sensitive to order
    /// (pair keying, canonical newborn ids, brood placement) — on the domain
    /// where offspring cannot coincide with a parent. Non-vacuous: ~30% of
    /// generated cases produce births within their tick budget.
    #[test]
    fn trajectory_is_invariant_under_agent_order_permutation_with_reproduction(
        case in world_case_with_reproduction()
    ) {
        check_order_permutation_invariance(&case)?;
    }
}

fn check_order_permutation_invariance(case: &WorldCase) -> Result<(), TestCaseError> {
    let baseline = run(case, false);
    let permuted = run(case, true);

    let base_pop = population_multiset(&baseline);
    let perm_pop = population_multiset(&permuted);
    prop_assert_eq!(
        base_pop.iter().map(|a| a.exact.clone()).collect::<Vec<_>>(),
        perm_pop.iter().map(|a| a.exact.clone()).collect::<Vec<_>>(),
        "identity / position / trait / wear multiset differs under order permutation after {} ticks",
        case.ticks
    );
    for (b, p) in base_pop.iter().zip(&perm_pop) {
        for (f, name) in SUMMED_FIELDS.iter().enumerate() {
            assert_rel_close(
                &format!("agent {} {name}", b.exact.id),
                b.summed[f],
                p.summed[f],
                SUMMED_TOTAL_REL_TOLERANCE,
            )?;
        }
    }
    prop_assert_eq!(
        baseline.carcasses().len(),
        permuted.carcasses().len(),
        "carcass count differs under order permutation"
    );
    let rel = SUMMED_TOTAL_REL_TOLERANCE;
    assert_rel_close(
        "dissipated_energy",
        baseline.dissipated_energy(),
        permuted.dissipated_energy(),
        rel,
    )?;
    assert_rel_close(
        "total_solar_input",
        baseline.total_solar_input(),
        permuted.total_solar_input(),
        rel,
    )?;
    assert_rel_close(
        "nutrient_pool",
        baseline.nutrient_pool(),
        permuted.nutrient_pool(),
        rel,
    )?;
    Ok(())
}
