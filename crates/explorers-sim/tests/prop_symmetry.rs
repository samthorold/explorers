//! Property-based symmetry checks over random parameterisations (#435,
//! Workstream C2): the symmetries the execution model commits to — agent-order
//! permutation, toroidal translation, extensive scaling — checked as invariants
//! of the stepper across the search space, reusing the C1 harness.

mod support;

use explorers_sim::spatial::SpatialGrid;
use explorers_sim::{
    Agent, AgentSpec, World, WorldRecipe, phase, toroidal_distance, wrap_position,
};
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

/// The C1 domain with the dispersal trait bounded away from zero so offspring
/// never land exactly on a parent: founders draw `max(0, mean + N(0, cov))`
/// with `mean ≥ 1.5, cov ≤ 0.25` (a 6σ event to reach zero), and mutation
/// adds `N(0, magnitude)` with `magnitude ≤ 0.2` (5σ to reach zero from 1.0).
/// Narrower than the search ranges on those three dimensions only.
fn world_case_bounded_dispersal() -> impl Strategy<Value = WorldCase> {
    (
        world_case_integer_exponent(),
        1.5f32..=2.0,
        0.1f32..=0.25,
        0.01f32..=0.2,
    )
        .prop_map(|(mut c, dispersal, cov, magnitude)| {
            c.dist.mean_traits.dispersal = dispersal;
            c.dist.trait_covariance = cov;
            c.params.mutation_magnitude = magnitude;
            c
        })
}

/// Reproduction enabled on the bounded-dispersal domain, so a zero-reach
/// consumer never coincides with a target.
fn world_case_with_reproduction() -> impl Strategy<Value = WorldCase> {
    world_case_bounded_dispersal().prop_map(without_chemotaxis_or_contact)
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

// ---------------------------------------------------------------------------
// Property 2: toroidal translation
// ---------------------------------------------------------------------------

/// Absolute tolerance for un-translated positions, in world units. With
/// chemotaxis off (see `translation_case_from`) a position is only ever
/// `wrap(pos + keyed_jitter)`, so the two worlds' coordinates differ by the
/// rounding of one addition per tick: ≤ 1 ulp at magnitude ≤ 15 (~1e-6),
/// accumulating linearly to ≤ 2e-5 over 20 ticks. 1e-4 is 5× headroom.
const TRANSLATION_POS_TOLERANCE: f32 = 1e-4;
/// Relative tolerance for stores and totals under a translation. Every
/// distance-derived flow inherits the ~1e-6 position rounding; 1e-4 (the C1
/// ledger bound) leaves two orders of magnitude of headroom.
const TRANSLATION_REL_TOLERANCE: f32 = 1e-4;

/// A translation case: a C1 case whose extent is an exact multiple of the
/// nutrient cell size, plus a whole-cell translation vector (never zero).
#[derive(Debug, Clone)]
struct TranslationCase {
    case: WorldCase,
    shift: (f32, f32),
}

/// Extent is snapped to `cells × cell_size` so the nutrient grid tiles the
/// torus exactly: with a partial last cell, translating by whole cells would
/// move agents between cells of different area, which is not a symmetry.
/// The spatial grid (light competition, contact, sensing) is distance-filtered
/// and so already translation-symmetric regardless of alignment.
///
/// Chemotaxis is switched off (`sensing_range_coefficient = 0`) in every
/// translation domain, permanently and by the nature of the check rather than
/// as a bug workaround: it weights neighbours by `1/dist` and normalises the
/// summed direction, so the 1-ulp position rounding a translation introduces
/// is amplified by `1/|dir|` whenever the attraction and jitter terms nearly
/// cancel — a heavy-tailed, unbounded amplification (measured gaps of 1e-4
/// within 5 ticks and 2e-2 within 18 across a few thousand random cases).
/// That is sensitive dependence, not an asymmetry, but it means the
/// trajectory has no f32-tolerance-stable form with chemotaxis on. Its
/// geometry is covered separately by `chemotaxis_neighbour_counts_are_
/// invariant_under_toroidal_translation`, on an integer statistic.
fn translation_case_from(
    cases: impl Strategy<Value = WorldCase>,
) -> impl Strategy<Value = TranslationCase> {
    (cases, 2u32..=3, 0u32..=2, 0u32..=2)
        .prop_filter("translation must be non-zero", |(_, _, i, j)| {
            *i != 0 || *j != 0
        })
        .prop_map(|(mut case, cells, i, j)| {
            let cell = case.params.nutrient_grid_cell_size;
            case.params.world_extent = cells as f32 * cell;
            case.params.sensing_range_coefficient = 0.0;
            TranslationCase {
                case,
                shift: (i as f32 * cell, j as f32 * cell),
            }
        })
}

/// Full C1 domain (less chemotaxis, see above).
fn translation_case() -> impl Strategy<Value = TranslationCase> {
    translation_case_from(world_case_integer_exponent())
}

/// The #453 workaround domain: consumption unreachable, so no carcass is ever
/// drained and the zero-energy nutrient discontinuity cannot fire; dispersal
/// bounded away from zero so a zero-reach consumer never coincides with a
/// target either.
fn translation_case_without_consumption() -> impl Strategy<Value = TranslationCase> {
    translation_case_from(world_case_bounded_dispersal().prop_map(|mut c| {
        c.params.contact_range_coefficient = 0.0;
        c
    }))
}

/// Sample the founders `World::new` would place for this case, as a spec'd
/// roster, so the same population can be re-founded at translated positions.
fn founder_roster(case: &WorldCase) -> Vec<AgentSpec> {
    World::new(case.params.clone(), case.dist.clone(), case.seed)
        .agents()
        .iter()
        .map(|a| AgentSpec {
            position: a.position,
            reserve: case.dist.initial_energy_per_agent,
            traits: a.traits,
            nutrient: 0.0,
        })
        .collect()
}

fn run_roster(case: &WorldCase, roster: Vec<AgentSpec>) -> World {
    let recipe = WorldRecipe {
        parameters: case.params.clone(),
        initial_distribution: None,
        agents: Some(roster),
        carcasses: None,
        max_ticks: case.ticks as u64,
    };
    let mut world = World::from_recipe(&recipe, case.seed);
    for _ in 0..case.ticks {
        world.step();
    }
    world
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Toroidal translation: founding the same roster shifted by a constant
    /// whole-cell vector yields the same trajectory shifted — identity, traits
    /// and demographics exact, positions shifted to f32 tolerance, stores and
    /// totals equal to rounding. Ignored until #453 (carcass nutrient release
    /// is discontinuous at zero energy, so a 1-ulp difference in a drained
    /// carcass's residue decides whether all or none of its nutrient
    /// recycles) is fixed; `..._without_consumption` runs meanwhile.
    #[test]
    #[ignore = "see #453"]
    fn trajectory_is_covariant_under_toroidal_translation(tc in translation_case()) {
        check_translation_covariance(&tc)?;
    }

    /// As above with consumption unreachable (#453 workaround). Covers light
    /// competition, nutrient uptake by cell, the random walk, mate search by
    /// spatial reach, and brood placement.
    #[test]
    fn trajectory_is_covariant_under_toroidal_translation_without_consumption(
        tc in translation_case_without_consumption()
    ) {
        check_translation_covariance(&tc)?;
    }

    /// Chemotaxis geometry: the number of neighbours each founder senses is an
    /// integer function of relative toroidal geometry, so it is exactly
    /// invariant under translation. This is the covariant statistic that
    /// survives the `1/|dir|` amplification that keeps chemotaxis out of the
    /// trajectory check (see `translation_case_from`). Run as a single
    /// movement phase over the founding roster, with sensing on and drawn
    /// from the full search range.
    #[test]
    fn chemotaxis_neighbour_counts_are_invariant_under_toroidal_translation(
        tc in translation_case_from(world_case_integer_exponent()),
        sensing in 1.0f32..=30.0,
    ) {
        let mut case = tc.case.clone();
        case.params.sensing_range_coefficient = sensing;
        let extent = case.params.world_extent;
        let (sx, sy) = tc.shift;
        let base = World::new(case.params.clone(), case.dist.clone(), case.seed);
        let mut agents = base.agents().to_vec();
        let mut shifted = agents.clone();
        for a in &mut shifted {
            a.position = wrap_position((a.position.0 + sx, a.position.1 + sy), extent);
        }
        let counts = sensed_neighbour_counts(&mut agents, &case.params, case.seed);
        let shifted_counts = sensed_neighbour_counts(&mut shifted, &case.params, case.seed);
        prop_assert_eq!(
            counts,
            shifted_counts,
            "sensed neighbour counts differ under translation by {:?}",
            tc.shift
        );
    }
}

/// One movement phase over `agents` exactly as `World::step` runs it (grid
/// keyed by slice index, cell size from the light-competition radius),
/// returning each agent's sensed neighbour count.
fn sensed_neighbour_counts(
    agents: &mut [Agent],
    params: &explorers_sim::WorldParameters,
    seed: u64,
) -> Vec<u32> {
    let mut grid = SpatialGrid::new(
        params.world_extent,
        params.light_competition_radius.max(1.0),
    );
    for (i, a) in agents.iter().enumerate() {
        grid.insert(i as u64, a.position);
    }
    phase::move_agents(agents, &[], &grid, params, seed, 0)
        .sensing_throughput
        .iter()
        .map(|c| *c as u32)
        .collect()
}

fn check_translation_covariance(tc: &TranslationCase) -> Result<(), TestCaseError> {
    let case = &tc.case;
    let extent = case.params.world_extent;
    let (sx, sy) = tc.shift;
    let roster = founder_roster(case);
    let shifted_roster: Vec<AgentSpec> = roster
        .iter()
        .map(|spec| AgentSpec {
            position: wrap_position((spec.position.0 + sx, spec.position.1 + sy), extent),
            ..spec.clone()
        })
        .collect();

    let base = run_roster(case, roster);
    let shifted = run_roster(case, shifted_roster);

    let mut base_pop: Vec<&Agent> = base.agents().iter().collect();
    let mut shifted_pop: Vec<&Agent> = shifted.agents().iter().collect();
    base_pop.sort_by_key(|a| a.id);
    shifted_pop.sort_by_key(|a| a.id);
    prop_assert_eq!(
        base_pop.iter().map(|a| a.id).collect::<Vec<_>>(),
        shifted_pop.iter().map(|a| a.id).collect::<Vec<_>>(),
        "population identity differs under translation after {} ticks",
        case.ticks
    );
    prop_assert_eq!(base.carcasses().len(), shifted.carcasses().len());

    let rel = TRANSLATION_REL_TOLERANCE;
    for (b, s) in base_pop.iter().zip(&shifted_pop) {
        let ctx = format!("agent {}", b.id);
        // Traits: crossover and mutation are keyed on identity, so exact.
        for d in 0..7 {
            prop_assert_eq!(
                b.traits.get(d).to_bits(),
                s.traits.get(d).to_bits(),
                "{} trait {} differs under translation",
                &ctx,
                d
            );
        }
        let expected = wrap_position((b.position.0 + sx, b.position.1 + sy), extent);
        let gap = toroidal_distance(expected, s.position, extent);
        prop_assert!(
            gap <= TRANSLATION_POS_TOLERANCE,
            "{ctx} position {:?} is not the translate of {:?} (expected {:?}, gap {gap})",
            s.position,
            b.position,
            expected
        );
        let stores = [
            ("reserve", b.reserve, s.reserve),
            ("structure", b.structure, s.structure),
            ("peak_structure", b.peak_structure, s.peak_structure),
            ("nutrient", b.nutrient, s.nutrient),
            ("repro_reserve", b.repro_reserve, s.repro_reserve),
            ("repro_nutrient", b.repro_nutrient, s.repro_nutrient),
            ("wear[0]", b.wear[0], s.wear[0]),
            ("wear[1]", b.wear[1], s.wear[1]),
            ("wear[2]", b.wear[2], s.wear[2]),
        ];
        for (name, x, y) in stores {
            assert_rel_close(&format!("{ctx} {name}"), x, y, rel)?;
        }
    }
    assert_rel_close(
        "dissipated_energy",
        base.dissipated_energy(),
        shifted.dissipated_energy(),
        rel,
    )?;
    assert_rel_close(
        "total_solar_input",
        base.total_solar_input(),
        shifted.total_solar_input(),
        rel,
    )?;
    assert_rel_close(
        "nutrient_pool",
        base.nutrient_pool(),
        shifted.nutrient_pool(),
        rel,
    )?;
    Ok(())
}
