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
use support::{WorldCase, world_case};

/// Relative tolerance for *summed* world totals under a permutation. The
/// execution model commits every RNG-derived quantity and every agent's
/// identity as exactly order-invariant, but explicitly allows the coordinated
/// non-RNG phases (light competition, nutrient uptake) to accumulate their
/// per-neighbour sums in slice order — so totals built from them may differ
/// by rounding (execution-model.md, "Re-seeding is a one-time event"). The
/// drain pass is id-ordered (#452), since its rounding feeds a discontinuity.
/// 1e-5 is ~100 ulps: ample for ≤ 20 ticks of a ≤ 40-agent world, and far
/// below any real order leak (see #451, which showed up as a position
/// difference of ~1e-2 after one tick).
const SUMMED_TOTAL_REL_TOLERANCE: f32 = 1e-5;

// ---------------------------------------------------------------------------
// Property 1: agent-order permutation
// ---------------------------------------------------------------------------

/// The part of an agent's state the execution model commits as *exactly*
/// order-invariant — identity, RNG-derived placement, traits, wear — as raw
/// bits, so equality is bit-identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct AgentExactBits {
    id: u64,
    position: (u32, u32),
    traits: [u32; 7],
    wear: [u32; 3],
}

impl AgentExactBits {
    fn of(a: &Agent) -> Self {
        AgentExactBits {
            id: a.id,
            position: (a.position.0.to_bits(), a.position.1.to_bits()),
            traits: std::array::from_fn(|d| a.traits.get(d).to_bits()),
            wear: std::array::from_fn(|f| a.wear[f].to_bits()),
        }
    }
}

/// The living population sorted by id, so two worlds can be compared as
/// multisets of agent state.
fn population_by_id(world: &World) -> Vec<&Agent> {
    let mut v: Vec<&Agent> = world.agents().iter().collect();
    v.sort_by_key(|a| a.id);
    v
}

/// The six energy and nutrient stores accumulated in the coordinated non-RNG
/// phases, which may legitimately differ by rounding between two runs.
fn stores(a: &Agent) -> [(&'static str, f32); 6] {
    [
        ("reserve", a.reserve),
        ("structure", a.structure),
        ("peak_structure", a.peak_structure),
        ("nutrient", a.nutrient),
        ("repro_reserve", a.repro_reserve),
        ("repro_nutrient", a.repro_nutrient),
    ]
}

fn assert_stores_close(a: &Agent, b: &Agent, rel: f32) -> Result<(), TestCaseError> {
    for ((name, x), (_, y)) in stores(a).into_iter().zip(stores(b)) {
        assert_rel_close(&format!("agent {} {name}", a.id), x, y, rel)?;
    }
    Ok(())
}

/// The world-level totals: carcass count exact, summed energy and nutrient
/// totals to `rel`.
fn assert_world_totals_close(a: &World, b: &World, rel: f32) -> Result<(), TestCaseError> {
    prop_assert_eq!(
        a.carcasses().len(),
        b.carcasses().len(),
        "carcass count differs"
    );
    assert_rel_close(
        "dissipated_energy",
        a.dissipated_energy(),
        b.dissipated_energy(),
        rel,
    )?;
    assert_rel_close(
        "total_solar_input",
        a.total_solar_input(),
        b.total_solar_input(),
        rel,
    )?;
    assert_rel_close("nutrient_pool", a.nutrient_pool(), b.nutrient_pool(), rel)?;
    Ok(())
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

/// Reproduction disabled (unreachable energy threshold). Covers acquisition,
/// consumption, metabolism, growth, the random walk with chemotaxis, wear and
/// death over the full trait-covariance range, with the phase most sensitive
/// to order (reproduction) held out so a failure localises elsewhere.
fn world_case_without_reproduction() -> impl Strategy<Value = WorldCase> {
    world_case().prop_map(|mut c| {
        c.params.reproduction_energy_threshold = f32::INFINITY;
        c
    })
}

/// The C1 domain with the dispersal trait bounded away from zero so offspring
/// never land exactly on a parent: founders draw `max(0, mean + N(0, cov))`
/// with `mean ≥ 1.5, cov ≤ 0.25` (a 6σ event to reach zero), and mutation
/// adds `N(0, magnitude)` with `magnitude ≤ 0.2` (5σ to reach zero from 1.0).
/// Narrower than the search ranges on those three dimensions only. Used by
/// the translation property (property 2) below.
fn world_case_bounded_dispersal() -> impl Strategy<Value = WorldCase> {
    (world_case(), 1.5f32..=2.0, 0.1f32..=0.25, 0.01f32..=0.2).prop_map(
        |(mut c, dispersal, cov, magnitude)| {
            c.dist.mean_traits.dispersal = dispersal;
            c.dist.trait_covariance = cov;
            c.params.mutation_magnitude = magnitude;
            c
        },
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Agent-order permutation over the full search domain:
    /// stepping a world with its agent slice permuted before every tick yields
    /// the same multiset of agent states, bit for bit, and the same ledger
    /// totals to rounding. Every phase — chemotaxis (#451), the drain
    /// proportional split and its stoichiometric nutrient split (#452), and
    /// reproduction (pair keying, canonical newborn ids, brood placement) —
    /// is per-agent, keyed on stable identity, or evaluated against tick-start
    /// state.
    #[test]
    fn trajectory_is_invariant_under_agent_order_permutation(
        case in world_case()
    ) {
        check_order_permutation_invariance(&case)?;
    }

    /// As above with reproduction off, so a failure localises to the
    /// non-reproductive phases (consumption included).
    #[test]
    fn trajectory_is_invariant_under_agent_order_permutation_without_reproduction(
        case in world_case_without_reproduction()
    ) {
        check_order_permutation_invariance(&case)?;
    }
}

fn check_order_permutation_invariance(case: &WorldCase) -> Result<(), TestCaseError> {
    let baseline = run(case, false);
    let permuted = run(case, true);

    let base_pop = population_by_id(&baseline);
    let perm_pop = population_by_id(&permuted);
    prop_assert_eq!(
        base_pop
            .iter()
            .map(|a| AgentExactBits::of(a))
            .collect::<Vec<_>>(),
        perm_pop
            .iter()
            .map(|a| AgentExactBits::of(a))
            .collect::<Vec<_>>(),
        "identity / position / trait / wear multiset differs under order permutation after {} ticks",
        case.ticks
    );
    for (b, p) in base_pop.iter().zip(&perm_pop) {
        assert_stores_close(b, p, SUMMED_TOTAL_REL_TOLERANCE)?;
    }
    assert_world_totals_close(&baseline, &permuted, SUMMED_TOTAL_REL_TOLERANCE)
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
    translation_case_from(world_case())
}

/// Dispersal bounded away from zero, so a zero-reach consumer never
/// coincides with a target.
fn translation_case_bounded_dispersal() -> impl Strategy<Value = TranslationCase> {
    translation_case_from(world_case_bounded_dispersal())
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

/// Run a roster for the case's ticks, rejecting the case if the trajectory
/// ever brings a consumer's feeding reach within `TRANSLATION_POS_TOLERANCE`
/// of a target (see `contact_on_reach_boundary`).
fn run_roster(case: &WorldCase, roster: Vec<AgentSpec>) -> Result<World, TestCaseError> {
    let recipe = WorldRecipe {
        parameters: case.params.clone(),
        initial_distribution: None,
        agents: Some(roster),
        carcasses: None,
        max_ticks: case.ticks as u64,
    };
    let mut world = World::from_recipe(&recipe, case.seed);
    for _ in 0..case.ticks {
        if let Some((consumer, target)) = contact_on_reach_boundary(&world) {
            return Err(TestCaseError::reject(format!(
                "consumer {consumer} sits on its feeding-reach boundary to target {target} \
                 at tick {}",
                world.tick()
            )));
        }
        world.step();
    }
    Ok(world)
}

/// Feeding reach is a step function of position (binary-reach drain, #380):
/// a target is drained if it is within reach and untouched if it is an ulp
/// beyond. A translation perturbs every distance by the position rounding,
/// so a pair that happens to sit within that rounding of the boundary is
/// drained in one world and not the other — a measure-zero event (≈ 1e-4 of
/// cases at the C1 domain) that is sensitive dependence, not an asymmetry,
/// and that no f32 tolerance can absorb. Such a trajectory is outside the
/// property's domain and the case is rejected. Scanned between ticks: the
/// drain phase runs before movement, on the previous tick's positions and
/// wear, and `body_reach_coefficient = 0` in this domain, so this is exactly
/// the reach the next drain will apply. Returns the first (consumer, target)
/// pair on the boundary.
fn contact_on_reach_boundary(world: &World) -> Option<(u64, u64)> {
    let params = world.params();
    let extent = params.world_extent;
    let k = params.wear_degradation_steepness;
    for consumer in world.agents() {
        let eff_heterotrophy = consumer.effective_trait_with_steepness(1, k);
        if eff_heterotrophy <= 0.0 {
            continue;
        }
        let reach = phase::consumption_reach(eff_heterotrophy, consumer.structure, params);
        let on_boundary = |target_pos: (f32, f32)| {
            (toroidal_distance(consumer.position, target_pos, extent) - reach).abs()
                <= TRANSLATION_POS_TOLERANCE
        };
        if let Some(target) = world
            .agents()
            .iter()
            .filter(|t| t.id != consumer.id)
            .find(|t| on_boundary(t.position))
        {
            return Some((consumer.id, target.id));
        }
        if let Some(carcass) = world.carcasses().iter().find(|c| on_boundary(c.position)) {
            return Some((consumer.id, carcass.id));
        }
    }
    None
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Toroidal translation: founding the same roster shifted by a constant
    /// whole-cell vector yields the same trajectory shifted — identity, traits
    /// and demographics exact, positions shifted to f32 tolerance, stores and
    /// totals equal to rounding.
    #[test]
    fn trajectory_is_covariant_under_toroidal_translation(tc in translation_case()) {
        check_translation_covariance(&tc)?;
    }

    /// As above with dispersal bounded away from zero, so a zero-reach
    /// consumer never coincides with a target. Covers light competition,
    /// nutrient uptake by cell, the random walk, mate search by spatial reach,
    /// and brood placement.
    #[test]
    fn trajectory_is_covariant_under_toroidal_translation_with_bounded_dispersal(
        tc in translation_case_bounded_dispersal()
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
        tc in translation_case_from(world_case()),
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

    let base = run_roster(case, roster)?;
    let shifted = run_roster(case, shifted_roster)?;

    let base_pop = population_by_id(&base);
    let shifted_pop = population_by_id(&shifted);
    prop_assert_eq!(
        base_pop.iter().map(|a| a.id).collect::<Vec<_>>(),
        shifted_pop.iter().map(|a| a.id).collect::<Vec<_>>(),
        "population identity differs under translation after {} ticks",
        case.ticks
    );

    let rel = TRANSLATION_REL_TOLERANCE;
    for (b, s) in base_pop.iter().zip(&shifted_pop) {
        // Traits: crossover and mutation are keyed on identity, so exact.
        for d in 0..7 {
            prop_assert_eq!(
                b.traits.get(d).to_bits(),
                s.traits.get(d).to_bits(),
                "agent {} trait {} differs under translation",
                b.id,
                d
            );
        }
        let expected = wrap_position((b.position.0 + sx, b.position.1 + sy), extent);
        let gap = toroidal_distance(expected, s.position, extent);
        prop_assert!(
            gap <= TRANSLATION_POS_TOLERANCE,
            "agent {} position {:?} is not the translate of {:?} (expected {:?}, gap {gap})",
            b.id,
            s.position,
            b.position,
            expected
        );
        assert_stores_close(b, s, rel)?;
        // Wear accumulates from usage (energy captured, distance moved), which
        // inherits the position rounding — so to rounding, not exact.
        for f in 0..3 {
            assert_rel_close(
                &format!("agent {} wear[{f}]", b.id),
                b.wear[f],
                s.wear[f],
                rel,
            )?;
        }
    }
    assert_world_totals_close(&base, &shifted, rel)
}

// ---------------------------------------------------------------------------
// Property 3: extensive scaling
// ---------------------------------------------------------------------------
//
// Doubling the linear extent (4× area) together with the founding population
// and the nutrient pool must leave per-area living energy and per-area
// available nutrient unchanged in expectation. The scale factor is 2 in
// extent rather than √2 so the nutrient grid (fixed 10-unit cells) tiles both
// worlds exactly and per-cell density is unchanged.
//
// This is a statistical property, asserted on ensemble means over
// `SCALING_SEEDS` seeds per size with a bound justified as follows.
//
// Domain. The property holds in the thermodynamic limit `r ≪ L` only: an
// interaction disc that wraps onto itself on the torus changes the physics
// with size. Diagnostics at L = 20 showed |z| up to 37 once the
// light-competition radius exceeded L/2, and z ≈ 3–4 systematic residue for
// radii in (L/4, L/2) where second-order (neighbour-of-neighbour, scale 2r)
// structure still wraps. The base world is therefore L = 40 with every
// interaction radius capped at L/4 = 10: light competition ≤ 10, sensing
// coefficient ≤ 5 (radius ≲ 7.5 at mobility ≤ 1.5), contact ≤ 5 × 1.5,
// and `dispersal_reach_coefficient = 1` so mating reach ≲ 10. Everything else
// is the C1 domain.
//
// Bound. `|Δmean| ≤ 8·SE_diff + 5%·max(|mean|)`. The z-term absorbs seed
// noise (16 seeds per size; |t| > 8 at ~30 df is a 1e-8 event). The relative
// floor absorbs two things the z-term cannot: statistics that are nearly
// constant across seeds (per-area available nutrient moves by ~1e-4 of
// itself in 20 ticks, so SE ≈ 0 and any rounding gives a large z), and
// genuine O(1/N) finite-size effects — founders are dealt to trait clusters
// round-robin, so a population of 16 in 5 clusters is 25/19/19/19/19% but 64
// is 20/20/20/20/20%, and that composition shift is systematic, not seed
// noise. Measured null: over 1000 random cases the worst `|Δ| / bound` was
// 0.46. Non-extensive physics (solar input, nutrient endowment or any flow
// scaling with something other than area) shows up as an O(1) relative
// difference with a small SE — a ratio in the tens (leaving the pool
// unscaled gives |Δ| = 15 × bound) — so the bound loses no power against
// the bugs it is for.
//
// Cost: 64 cases × 2 sizes × 16 seeds × ≤ 20 ticks ≈ 3 s.

/// Seeds per size in the scaling ensemble.
const SCALING_SEEDS: u64 = 16;
/// Base linear extent; the scaled world is `2 ×` this.
const SCALING_BASE_EXTENT: f32 = 40.0;
/// Every interaction radius is capped at this (= L/4), see the module note.
const SCALING_MAX_RADIUS: f32 = SCALING_BASE_EXTENT / 4.0;
/// z-score term of the bound.
const SCALING_Z: f32 = 8.0;
/// Relative-floor term of the bound.
const SCALING_REL_FLOOR: f32 = 0.05;

/// Per-area statistics of one world after `ticks` steps.
struct PerArea {
    living_energy: f32,
    available_nutrient: f32,
}

fn per_area_after(case: &WorldCase, seed: u64) -> PerArea {
    let mut world = World::new(case.params.clone(), case.dist.clone(), seed);
    for _ in 0..case.ticks {
        world.step();
    }
    let area = case.params.world_extent * case.params.world_extent;
    PerArea {
        living_energy: world.free_energy() / area,
        available_nutrient: world.nutrient_pool() / area,
    }
}

/// The same parameterisation scaled by `k` in linear extent: area, founding
/// population and nutrient pool all scale by `k²`, so every intensive
/// (per-area) quantity is unchanged in expectation.
fn scaled_by(case: &WorldCase, k: u32) -> WorldCase {
    let mut scaled = case.clone();
    scaled.params.world_extent *= k as f32;
    scaled.params.initial_population_size *= k * k;
    scaled.params.initial_nutrient_pool *= (k * k) as f32;
    scaled
}

fn mean_and_se(xs: &[f32]) -> (f32, f32) {
    let n = xs.len() as f32;
    let mean = xs.iter().sum::<f32>() / n;
    let var = xs.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / (n - 1.0);
    (mean, (var / n).sqrt())
}

/// The scaling domain: the C1 domain at `SCALING_BASE_EXTENT` with every
/// interaction radius capped at `SCALING_MAX_RADIUS` (see the module note).
fn scaling_case() -> impl Strategy<Value = WorldCase> {
    world_case().prop_map(|mut c| {
        c.params.world_extent = SCALING_BASE_EXTENT;
        c.params.light_competition_radius =
            c.params.light_competition_radius.min(SCALING_MAX_RADIUS);
        c.params.sensing_range_coefficient = c
            .params
            .sensing_range_coefficient
            .min(SCALING_MAX_RADIUS / 2.0);
        c.params.dispersal_reach_coefficient = 1.0;
        c
    })
}

fn assert_ensemble_means_agree(
    name: &str,
    small: &[f32],
    big: &[f32],
) -> Result<(), TestCaseError> {
    let (mean_small, se_small) = mean_and_se(small);
    let (mean_big, se_big) = mean_and_se(big);
    let se_diff = (se_small * se_small + se_big * se_big).sqrt();
    let scale = mean_small.abs().max(mean_big.abs());
    let bound = SCALING_Z * se_diff + SCALING_REL_FLOOR * scale;
    let delta = (mean_small - mean_big).abs();
    prop_assert!(
        delta <= bound,
        "per-area {name} is not extensive: {mean_small} (SE {se_small}) at L vs \
         {mean_big} (SE {se_big}) at 2L; |Δ| = {delta} > bound {bound} \
         ({SCALING_Z}·SE_diff + {SCALING_REL_FLOOR}·scale)"
    );
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Extensive scaling: doubling the extent with 4× population and 4×
    /// nutrient pool leaves per-area living energy and per-area available
    /// nutrient unchanged, up to the finite-size bound above.
    #[test]
    fn per_area_statistics_are_invariant_under_extensive_scaling(case in scaling_case()) {
        let big = scaled_by(&case, 2);
        let mut energy = (Vec::new(), Vec::new());
        let mut nutrient = (Vec::new(), Vec::new());
        for s in 0..SCALING_SEEDS {
            let seed = case.seed.wrapping_add(s);
            let small = per_area_after(&case, seed);
            let large = per_area_after(&big, seed);
            energy.0.push(small.living_energy);
            energy.1.push(large.living_energy);
            nutrient.0.push(small.available_nutrient);
            nutrient.1.push(large.available_nutrient);
        }
        assert_ensemble_means_agree("living energy", &energy.0, &energy.1)?;
        assert_ensemble_means_agree("available nutrient", &nutrient.0, &nutrient.1)?;
    }
}
