//! The detrital pathway, on the example9 scenario.
//!
//! Everything here runs in the fast lane:
//!
//!   1. A correctness regression for the #303 drain-phase index/id bug: a
//!      decomposer must drain a co-located carcass even after a death has
//!      reindexed the living `agents` slice (so an agent's index no longer equals
//!      its id). This test builds its world programmatically; it borrows only
//!      example9's fully-specified `WorldParameters` block.
//!   2. Determinism guards: example9 run twice, or with its agents' storage
//!      order shuffled every tick, steps identically.
//!   3. The wiring of the path from producer to carcass to decomposer, exact on
//!      example9: a sessile decomposer seeded on a carcass deposit, with the
//!      producer ring outside its feeding reach. It checks the code path closes
//!      end to end, not that detritivory emerges (the decomposer's diet is
//!      forced by the geometry).
//!
//! Run with:
//!   cargo test -p explorers-sim --test sim headless_decomposer:: -- --nocapture

use explorers_sim::{World, WorldRecipe};

const PATHWAY_SCENARIO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../scenarios/example9_detrital_pathway.json"
);

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
        wear: Default::default(),
        repro_reserve: 0.0,
        repro_nutrient: 0.0,
        marker: Default::default(),
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
        wear: Default::default(),
        repro_reserve: 0.0,
        repro_nutrient: 0.0,
        marker: Default::default(),
    });
    world.add_carcass(Carcass {
        id: 999,
        position: (10.0, 10.0),
        energy: 50.0,
        energy_at_death: 50.0,
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
    /// Births per tick, population per tick, and each final agent's id and position bits.
    type RunTrace = (Vec<u64>, Vec<usize>, Vec<(u64, (u32, u32))>);

    fn run(seed: u64, shuffle: bool) -> RunTrace {
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

/// The wiring of the path from producer to carcass to decomposer, exact on
/// example9 (seed 1, 800 ticks; since #863 the starvation cascade carries a
/// starving producer on its earmark and structure, and the ring's first death
/// comes at tick 660, where it came within 200 before). The scenario holds the producer ring far
/// outside the decomposer's fixed feeding reach (`body_reach_coefficient = 0`),
/// so the geometry fixes what each leg of the path must do:
///
///   * producer to carcass: every agent that dies leaves a carcass under its
///     own id, and the ring's deaths are among them;
///   * carcass to decomposer: the seeded deposit is drained, and only by the
///     decomposer's lineage;
///   * the two lineages never drain each other: no `Consumed` event joins the
///     decomposer's lineage to the producers' in either direction.
///
/// Each is an equality, refuted by any violation, so it runs in the fast lane.
#[test]
fn example9_wires_producer_to_carcass_to_decomposer_exactly() {
    use explorers_sim::event::EventKind;
    use std::collections::BTreeSet;

    let recipe = load_pathway();
    let mut world = World::from_recipe(&recipe, 1);
    let deposit: BTreeSet<u64> = world.carcasses().iter().map(|c| c.id).collect();
    assert_eq!(deposit.len(), 8, "example9 seeds an eight-carcass deposit");
    let mut decomposers: BTreeSet<u64> = world
        .agents()
        .iter()
        .filter(|a| a.traits.heterotrophy > 0.0)
        .map(|a| a.id)
        .collect();
    assert_eq!(decomposers.len(), 1, "example9 seeds one decomposer");
    let mut producers: BTreeSet<u64> = world
        .agents()
        .iter()
        .filter(|a| a.traits.heterotrophy == 0.0)
        .map(|a| a.id)
        .collect();

    let mut dead: Vec<u64> = Vec::new();
    let mut deposit_drains_by_decomposers = 0usize;
    let mut deposit_drains_by_others = 0usize;
    let mut cross_lineage_drains = 0usize;
    let mut cursor = 0usize;
    for _ in 0..800 {
        world.step();
        for ev in world.event_log().since(cursor) {
            match ev.kind {
                EventKind::Born => {
                    let parent = ev.target.expect("a birth names its parent");
                    if decomposers.contains(&parent) {
                        decomposers.insert(ev.source);
                    } else if producers.contains(&parent) {
                        producers.insert(ev.source);
                    }
                }
                EventKind::Died => dead.push(ev.source),
                EventKind::Consumed => {
                    let target = ev.target.expect("a drain names its target");
                    if ev.target_was_carcass && deposit.contains(&target) {
                        if decomposers.contains(&ev.source) {
                            deposit_drains_by_decomposers += 1;
                        } else {
                            deposit_drains_by_others += 1;
                        }
                    }
                    if !ev.target_was_carcass {
                        let crosses = (decomposers.contains(&ev.source)
                            && producers.contains(&target))
                            || (producers.contains(&ev.source) && decomposers.contains(&target));
                        if crosses {
                            cross_lineage_drains += 1;
                        }
                    }
                }
                _ => {}
            }
        }
        cursor = world.event_log().len();
    }

    let carcasses: BTreeSet<u64> = world.carcasses().iter().map(|c| c.id).collect();
    let dead_producers = dead.iter().filter(|id| producers.contains(id)).count();
    assert!(
        dead_producers > 0,
        "the ring must lose producers inside the horizon"
    );
    for id in &dead {
        assert!(
            carcasses.contains(id),
            "agent {id} died but left no carcass under its id"
        );
    }
    assert!(
        deposit_drains_by_decomposers > 0,
        "the decomposer must drain the deposit"
    );
    assert_eq!(
        deposit_drains_by_others, 0,
        "only the decomposer's lineage reaches the deposit"
    );
    assert_eq!(
        cross_lineage_drains, 0,
        "the decomposer's lineage and the producer ring never drain each other"
    );
}

/// Horizon of the example9 determinism guards above: long enough that the
/// ring's sexual mate pairing and the deposit's drains run for hundreds of
/// ticks, short of the scenario's 2000-tick `max_ticks`.
const PATHWAY_TEST_MAX_TICKS: u64 = 500;
