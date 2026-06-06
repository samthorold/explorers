//! Behavioural verification for the detrital pathway.
//!
//! Two things are guarded here:
//!
//!   1. A fast correctness regression for the #303 drain-phase index/id bug — a
//!      decomposer must drain a co-located carcass even after a death has
//!      reindexed the living `agents` slice (so an agent's index no longer equals
//!      its id). This test builds its world programmatically; it borrows only a
//!      fully-specified `WorldParameters` block from a scenario file.
//!   2. The example9 *pathway wiring* sweeps (issue #311): a sessile decomposer
//!      seeded on a standing carcass deposit with no living agent inside its
//!      consumption reach drives `detrital_share > 0.5` **by construction** (the
//!      decomposer physically cannot reach a living agent). These verify the
//!      producer->carcass->decomposer code path closes end to end — they are a
//!      regression on the *wiring*, not evidence that detritivory *emerges*
//!      (emergence evidence now comes from the genesis search; see issue #328 and
//!      the deferred genesis-emergence-regression follow-up).
//!
//! Note: `scenarios/example6_decomposer_viability.json` was retired in #328 (its
//! decomposer never established a lineage and its producers mass-died in a single
//! tick, so it demonstrated neither viability nor a sustained carcass supply).
//! The drain regression below now sources its parameters from example9, which
//! uses the same fully-specified parameter template.
//!
//! Run with:
//!   cargo test -p explorers-sim --test headless_decomposer -- --nocapture
//!
//! ## The `slow_` marker convention
//!
//! The multi-seed behavioural sweeps in this file each run the scenario to
//! completion across a spread of seeds, so they dominate the suite's wall-clock
//! cost. They are first-class tests — they stay in the default run (NOT
//! `#[ignore]`d) — but they are tagged with a `slow_` name prefix so the slow
//! sweeps form a selectable category:
//!   * `cargo test` / `cargo test --workspace` — runs everything, sweeps included.
//!   * `cargo test slow_` — run ONLY the slow sweeps.
//!   * `cargo test -- --skip slow_` — the tight inner loop: everything EXCEPT the
//!     sweeps (sub-second; leaves only the fast correctness regression).
//! Any future slow sweep should adopt the `slow_` prefix to join the category for
//! free. The cheap correctness regressions stay unprefixed.

use explorers_sim::topology::{TopologyProjection, TrophicRole};
use explorers_sim::{World, WorldRecipe};

const PATHWAY_SCENARIO: &str =
    "/Users/sam/Projects/explorers/scenarios/example9_detrital_pathway.json";

fn load_pathway() -> WorldRecipe {
    let contents = std::fs::read_to_string(PATHWAY_SCENARIO).expect("read pathway scenario");
    serde_json::from_str(&contents).expect("parse pathway scenario")
}

/// Regression: a decomposer must drain a co-located carcass even after some
/// agents have died (which reindexes the living `agents` slice so that an agent's
/// index no longer equals its id). The drain phase queries the spatial grid,
/// which is keyed by slice index; conflating that index with the agent id makes
/// the carcass pass find zero consumers once any death has occurred — which is
/// exactly when carcasses exist. Carcasses then accumulate forever (the
/// suite-wide "carcasses accumulate unconsumed" symptom in #303/#293).
///
/// The world is built programmatically; only the fully-specified
/// `WorldParameters` block is borrowed from a scenario file (example9, post-#328).
#[test]
fn decomposer_drains_carcass_after_a_death_reindexes_agents() {
    use explorers_sim::{Agent, Carcass, TraitVector};
    let recipe = load_pathway();
    let params = recipe.parameters.clone();
    let mut world = World::from_recipe(
        &WorldRecipe {
            parameters: params,
            agents: Some(vec![]),
            carcasses: None,
            max_ticks: 50,
            initial_distribution: None,
        },
        1,
    );
    // A doomed agent (id 0) with zero reserve dies on the first step, shifting
    // every later agent's slice index down by one (index != id thereafter).
    world.add_agent(Agent {
        id: 0,
        position: (80.0, 80.0),
        reserve: 0.0,
        structure: 0.0,
        peak_structure: 0.0,
        nutrient: 0.0,
        traits: TraitVector {
            photosynthetic_absorption: 0.0,
            heterotrophy: 0.0,
            mobility: 0.0,
            kappa: 0.5,
            fecundity: 0.0,
            asexual_propensity: 0.0,
            dispersal: 0.0,
        },
        contact_time: 0,
        wear: Default::default(),
        repro_reserve: 0.0,
        repro_nutrient: 0.0,
    });
    // The decomposer, co-located with a carcass it should drain.
    world.add_agent(Agent {
        id: 1,
        position: (10.0, 10.0),
        reserve: 100.0,
        structure: 5.0,
        peak_structure: 5.0,
        nutrient: 5.0,
        traits: TraitVector {
            photosynthetic_absorption: 0.4,
            heterotrophy: 1.1,
            mobility: 0.0,
            kappa: 0.5,
            fecundity: 0.0,
            asexual_propensity: 0.0,
            dispersal: 0.0,
        },
        contact_time: 0,
        wear: Default::default(),
        repro_reserve: 0.0,
        repro_nutrient: 0.0,
    });
    world.add_carcass(Carcass {
        id: 999,
        position: (10.0, 10.0),
        energy: 50.0,
        nutrient: 3.0,
        traits: TraitVector {
            photosynthetic_absorption: 0.45,
            heterotrophy: 0.0,
            mobility: 0.0,
            kappa: 0.5,
            fecundity: 0.3,
            asexual_propensity: 0.3,
            dispersal: 0.2,
        },
    });
    let before = world
        .carcasses()
        .iter()
        .find(|c| c.id == 999)
        .unwrap()
        .energy;
    for _ in 0..10 {
        world.step();
    }
    let after = world
        .carcasses()
        .iter()
        .find(|c| c.id == 999)
        .map(|c| c.energy)
        .unwrap_or(0.0);
    assert!(
        after < before,
        "decomposer must drain a co-located carcass even after a death reindexes agents ({before} -> {after})"
    );
}

/// Determinism guard (issue #343): the deterministic stepper's load-bearing
/// invariant is *same seed -> same run*. example9 is the only scenario combining
/// a dense, high-birth sexual producer ring with an active decomposition pathway,
/// so it is the only one that exercised the sexual mate-pairing phase's
/// nondeterministic iteration order. Running the same seed twice in the same
/// process must reproduce identical demographics down to the count; if it does
/// not, an iteration-order-dependent RNG draw has leaked into the stepper.
#[test]
fn example9_is_bit_deterministic_across_repeated_runs() {
    fn run_demographics(seed: u64) -> (u64, usize) {
        let recipe = load_pathway();
        let mut world = World::from_recipe(&recipe, seed);
        let mut total_births: u64 = 0;
        let ticks = recipe.max_ticks.min(PATHWAY_TEST_MAX_TICKS);
        for _ in 0..ticks {
            world.step();
            total_births += world.last_tick_births() as u64;
            if world.agents().is_empty() {
                break;
            }
        }
        (total_births, world.agents().len())
    }

    let seed = 1;
    let first = run_demographics(seed);
    let second = run_demographics(seed);
    assert_eq!(
        first, second,
        "example9 must be deterministic per seed: (total_births, final_population) \
         diverged between two identical runs of seed {seed} — first {first:?}, second {second:?}"
    );
}

/// Trajectory-level shuffle guard (issue #376). This is the test the
/// keyed-stateless RNG refactor exists to enable: deliberately permute the
/// in-memory iteration order of the agents slice *between every step* and assert
/// the full trajectory is bit-identical to an unpermuted run of the same seed.
///
/// Under the old single-shared-stream design this would diverge — every agent's
/// stochastic outcome depended on which agents drew before it, so reordering the
/// slice reordered the draws. With per-agent-per-phase keyed RNG (movement and
/// reproduction key on stable agent id / the symmetric ordered pair) and
/// canonical newborn-id assignment, iteration order is no longer load-bearing:
/// the demographics, and indeed every agent's id, position and reserve, must
/// match exactly. example9 is used because it is the only scenario that drives a
/// dense, high-birth sexual producer ring (the phase most sensitive to order).
#[test]
fn example9_demographics_are_invariant_under_agent_order_shuffle() {
    /// A trajectory fingerprint capturing exactly what keyed-stateless RNG plus
    /// canonical newborn-id assignment make order-independent:
    ///   * `births_per_tick` and `pop_per_tick` — the demographics (#376's gated
    ///     property: deliberately permuting iteration order must not change who
    ///     is born or how many live).
    ///   * the final population as sorted `(id, position)` — agent *identity* and
    ///     its RNG-derived placement (movement jitter + dispersal kernel).
    ///
    /// Summed-energy fields (reserve, structure) are deliberately *excluded*:
    /// they are accumulated in the coordinated NON-RNG phases (light-competition
    /// and drain proportional split), whose per-neighbour summation order follows
    /// the spatial grid's bucket order and so shifts by ~1 ULP of float rounding
    /// under a slice permutation. That is ordinary floating-point
    /// non-associativity in phases this refactor does not touch (and must not —
    /// they stay RNG-free, guarded by the SoA differential tests), not an
    /// RNG-order leak. The property under test is that *identity, demographics,
    /// and every RNG-derived quantity* are order-invariant — which they are.
    fn run(seed: u64, shuffle: bool) -> (Vec<u64>, Vec<usize>, Vec<(u64, (u32, u32))>) {
        let recipe = load_pathway();
        let mut world = World::from_recipe(&recipe, seed);
        let mut births_per_tick = Vec::new();
        let mut pop_per_tick = Vec::new();
        let ticks = recipe.max_ticks.min(PATHWAY_TEST_MAX_TICKS);
        for t in 0..ticks {
            if shuffle {
                // A different rotation each tick so the permutation is not a
                // fixed offset the keying could accidentally be invariant to.
                world.permute_agent_order_for_test((t as usize).wrapping_mul(7) + 1);
            }
            world.step();
            births_per_tick.push(world.last_tick_births() as u64);
            pop_per_tick.push(world.agents().len());
            if world.agents().is_empty() {
                break;
            }
        }
        let mut snapshot: Vec<(u64, (u32, u32))> = world
            .agents()
            .iter()
            .map(|a| (a.id, (a.position.0.to_bits(), a.position.1.to_bits())))
            .collect();
        snapshot.sort_by_key(|e| e.0);
        (births_per_tick, pop_per_tick, snapshot)
    }

    let seed = 1;
    let baseline = run(seed, false);
    let shuffled = run(seed, true);
    assert_eq!(
        baseline.0, shuffled.0,
        "per-tick birth counts must be invariant under agent-order shuffle (seed {seed})"
    );
    assert_eq!(
        baseline.1, shuffled.1,
        "per-tick population must be invariant under agent-order shuffle (seed {seed})"
    );
    assert_eq!(
        baseline.2, shuffled.2,
        "final population identity + RNG-derived position must be bit-identical \
         under agent-order shuffle (seed {seed}) — iteration order leaked into determinism"
    );
}

// ----------------------------------------------------------------------------
// example9_detrital_pathway.json (issue #311): majority-detrital BY CONSTRUCTION.
//
// A sessile decomposer is seeded on a standing carcass deposit with no living
// agent inside its consumption reach (body_reach_coefficient = 0 keeps reach
// fixed and structure-independent, so the geometry is exact). Its diet is
// detrital from tick 0 deterministically. These assertions verify the property
// holds across the full seed sweep — a regression on the *wiring* of the
// producer->carcass->decomposer pathway, robust by geometry, not evidence that
// detritivory emerges (that now comes from the genesis search; see #328).
// ----------------------------------------------------------------------------

/// Seed sweep for the by-construction detrital-pathway assertions. The
/// majority-detrital property holds *by geometry* (the decomposer physically
/// cannot reach a living agent), so it is seed-independent — a sweep over a
/// dozen seeds is ample to demonstrate it is not a single-seed artifact, and
/// keeps each full-run test (90+ agents over 2000 ticks) under the suite's
/// time budget. (Verified to also hold over 1..=20 during development.)
const PATHWAY_SEEDS: std::ops::RangeInclusive<u64> = 1..=12;

/// Everything the three pathway property tests assert on, distilled from a single
/// run of the scenario at a given seed. The example9 run is the suite's dominant
/// cost (~90 agents over 2000 ticks); the three tests previously each re-ran the
/// full 12-seed sweep (36 identical runs). Distilling into seed-keyed scalars and
/// memoising (`pathway_seed_result`) collapses that to one run per seed.
#[derive(Clone, Copy)]
struct PathwaySeedResult {
    /// Cumulative predation energy (living prey) across the run.
    predation: f32,
    /// Cumulative decomposition energy (carcass-sourced) across the run.
    decomposition: f32,
    /// Agents that read as a `Decomposer` behavioural role at end of run.
    decomposers: usize,
    /// Surviving heterotrophy-dominant agents at end of run.
    surviving_heterotrophs: usize,
    /// Final living population.
    final_population: usize,
}

/// Test-path run horizon for the example9 sweep. The scenario file declares
/// `max_ticks = 2000` (its budget for `eval_scenarios`/genesis), but all three
/// pathway properties stabilise by ~tick 2 and hold flat to tick 2000 on every
/// seed: detrital_share sits at 0.89–0.99 (never near the 0.5 threshold) and the
/// decomposer + a heterotroph persist throughout. 500 ticks is a generous sustain
/// horizon — the decomposer feeds for 500 ticks on the supply, well past
/// stabilisation — while cutting the suite's dominant cost ~4x. (Verified the
/// properties are unchanged at 2000 ticks during this work; see issue #320.)
const PATHWAY_TEST_MAX_TICKS: u64 = 500;

/// Run the pathway scenario once at `seed` and distil every property the tests
/// need. The same single step-loop accumulates the topology projection (for the
/// role classification) and the predation-vs-decomposition tally (the green/brown
/// split), so no property requires a second run of the same seed.
fn compute_pathway_seed(seed: u64) -> PathwaySeedResult {
    let recipe = load_pathway();
    let mut world = World::from_recipe(&recipe, seed);
    let mut topology = TopologyProjection::new();
    let mut predation = 0.0f32;
    let mut decomposition = 0.0f32;
    let mut cursor = 0usize;
    let ticks = recipe.max_ticks.min(PATHWAY_TEST_MAX_TICKS);
    for _ in 0..ticks {
        world.step();
        for ev in world.event_log().since(cursor) {
            if let explorers_sim::event::EventKind::Consumed = ev.kind {
                if ev.target_was_carcass {
                    decomposition += ev.energy_delta;
                } else {
                    predation += ev.energy_delta;
                }
            }
        }
        cursor = world.event_log().len();
        topology.update(world.event_log());
        if world.agents().is_empty() {
            break;
        }
    }
    let roles = topology.trophic_roles(world.agents());
    let decomposers = roles
        .values()
        .filter(|&&r| r == TrophicRole::Decomposer)
        .count();
    let surviving_heterotrophs = world
        .agents()
        .iter()
        .filter(|a| a.traits.heterotrophy > a.traits.photosynthetic_absorption)
        .count();
    PathwaySeedResult {
        predation,
        decomposition,
        decomposers,
        surviving_heterotrophs,
        final_population: world.agents().len(),
    }
}

/// Memoised accessor: the expensive example9 run for `seed` happens exactly once
/// per test-binary invocation, shared across all three pathway property tests
/// (which run in parallel by default). Keyed by seed so the full 12-seed sweep is
/// preserved — only the redundant re-runs are removed.
fn pathway_seed_result(seed: u64) -> PathwaySeedResult {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<u64, PathwaySeedResult>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(cached) = cache.lock().unwrap().get(&seed) {
        return *cached;
    }
    // Compute outside the lock so concurrent tests fan out across seeds rather
    // than serialising on the cache mutex.
    let result = compute_pathway_seed(seed);
    cache.lock().unwrap().insert(seed, result);
    result
}

/// The headline property: detrital intake is the MAJORITY of system consumption
/// on EVERY seed — `detrital_share > 0.5` holds by construction (the decomposer
/// physically cannot reach a living agent), not by parameter luck. Should
/// comfortably clear 0.5; if it is borderline, the construction has failed.
#[test]
fn slow_pathway_is_majority_detrital_on_every_seed() {
    for seed in PATHWAY_SEEDS {
        let PathwaySeedResult {
            predation,
            decomposition,
            ..
        } = pathway_seed_result(seed);
        assert!(
            decomposition > 0.0,
            "seed {seed}: the detrital pathway must carry real energy \
             (decomposition = {decomposition})"
        );
        let detrital_share = decomposition / (predation + decomposition);
        assert!(
            detrital_share > 0.5,
            "seed {seed}: detrital intake must be the MAJORITY of system consumption \
             by construction (predation {predation:.1}, decomposition {decomposition:.1}, \
             detrital share {detrital_share:.3})"
        );
    }
}

/// The surviving decomposer reads as a `Decomposer` behavioural role on every
/// seed — its own lifetime diet is majority detrital.
#[test]
fn slow_pathway_decomposer_reads_as_decomposer_role() {
    for seed in PATHWAY_SEEDS {
        let result = pathway_seed_result(seed);
        assert!(
            result.decomposers >= 1,
            "seed {seed}: expected a surviving agent to read as Decomposer; \
             decomposers = {}",
            result.decomposers
        );
    }
}

/// The decomposer sustains to the end of the run on the seeded deposit + carcass
/// supply, on every seed — it does not starve out.
#[test]
fn slow_pathway_decomposer_sustains_itself_to_end_of_run() {
    for seed in PATHWAY_SEEDS {
        let result = pathway_seed_result(seed);
        assert!(
            result.surviving_heterotrophs >= 1,
            "seed {seed}: decomposer lineage must survive to the end of the run, but no \
             heterotroph-dominant agent remains (final population {})",
            result.final_population
        );
    }
}
