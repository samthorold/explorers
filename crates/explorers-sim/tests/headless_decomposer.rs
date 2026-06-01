//! Behavioural verification for scenarios/example6_decomposer_viability.json
//! (issue #303): a self-thinning producer stand generates structure-rich
//! carcasses, and a seeded decomposer must (a) read as a `Decomposer`
//! behavioural role — it consumes carcasses, not living agents — and (b) sustain
//! itself on that carcass supply rather than starving out.
//!
//! Run with:
//!   cargo test -p explorers-sim --test headless_decomposer -- --nocapture

use explorers_sim::topology::{TopologyProjection, TrophicRole};
use explorers_sim::{World, WorldRecipe};

const SCENARIO: &str =
    "/Users/sam/Projects/explorers/scenarios/example6_decomposer_viability.json";

fn load() -> WorldRecipe {
    let contents = std::fs::read_to_string(SCENARIO).expect("read scenario");
    serde_json::from_str(&contents).expect("parse scenario")
}

/// Number of seeds the behavioural assertions sweep. example6's dynamics are
/// regime-sensitive (a productive stand can overshoot and collapse, or settle),
/// so the decomposer-role and detrital-pathway properties are asserted across a
/// spread of seeds rather than the single seed `eval_scenarios` happens to use —
/// a robust property, not a single-seed artifact.
const SEEDS: std::ops::RangeInclusive<u64> = 1..=8;

/// Run the scenario to completion under `seed`, returning the final world and the
/// topology projection accumulated over the whole run (so role classification
/// reads the full predation-vs-decomposition history).
fn run_seed(seed: u64) -> (World, TopologyProjection) {
    let recipe = load();
    let mut world = World::from_recipe(&recipe, seed);
    let mut topology = TopologyProjection::new();
    for _ in 0..recipe.max_ticks {
        world.step();
        topology.update(world.event_log());
        if world.agents().is_empty() {
            break;
        }
    }
    (world, topology)
}

/// Regression: a decomposer must drain a co-located carcass even after some
/// agents have died (which reindexes the living `agents` slice so that an agent's
/// index no longer equals its id). The drain phase queries the spatial grid,
/// which is keyed by slice index; conflating that index with the agent id makes
/// the carcass pass find zero consumers once any death has occurred — which is
/// exactly when carcasses exist. Carcasses then accumulate forever (the
/// suite-wide "carcasses accumulate unconsumed" symptom in #303/#293).
#[test]
fn decomposer_drains_carcass_after_a_death_reindexes_agents() {
    use explorers_sim::{Agent, Carcass, TraitVector};
    let recipe = load();
    let params = recipe.parameters.clone();
    let mut world = World::from_recipe(
        &WorldRecipe { parameters: params, agents: Some(vec![]), max_ticks: 50, initial_distribution: None },
        1,
    );
    // A doomed agent (id 0) with zero reserve dies on the first step, shifting
    // every later agent's slice index down by one (index != id thereafter).
    world.add_agent(Agent {
        id: 0, position: (80.0, 80.0), reserve: 0.0, structure: 0.0, peak_structure: 0.0, nutrient: 0.0,
        traits: TraitVector { photosynthetic_absorption: 0.0, heterotrophy: 0.0, mobility: 0.0, kappa: 0.5, fecundity: 0.0, asexual_propensity: 0.0, dispersal: 0.0 },
        contact_time: 0, wear: Default::default(), repro_reserve: 0.0, repro_nutrient: 0.0,
    });
    // The decomposer, co-located with a carcass it should drain.
    world.add_agent(Agent {
        id: 1, position: (10.0, 10.0), reserve: 100.0, structure: 5.0, peak_structure: 5.0, nutrient: 5.0,
        traits: TraitVector { photosynthetic_absorption: 0.4, heterotrophy: 1.1, mobility: 0.0, kappa: 0.5, fecundity: 0.0, asexual_propensity: 0.0, dispersal: 0.0 },
        contact_time: 0, wear: Default::default(), repro_reserve: 0.0, repro_nutrient: 0.0,
    });
    world.add_carcass(Carcass {
        id: 999, position: (10.0, 10.0), energy: 50.0, nutrient: 3.0,
        traits: TraitVector { photosynthetic_absorption: 0.45, heterotrophy: 0.0, mobility: 0.0, kappa: 0.5, fecundity: 0.3, asexual_propensity: 0.3, dispersal: 0.2 },
    });
    let before = world.carcasses().iter().find(|c| c.id == 999).unwrap().energy;
    for _ in 0..10 {
        world.step();
    }
    let after = world.carcasses().iter().find(|c| c.id == 999).map(|c| c.energy).unwrap_or(0.0);
    assert!(
        after < before,
        "decomposer must drain a co-located carcass even after a death reindexes agents ({before} -> {after})"
    );
}

/// The authoritative decomposer signal: a surviving agent reads as a
/// `Decomposer` — `trophic_roles` buckets a heterotroph as a Decomposer when its
/// *own* lifetime diet is majority detrital (carcass-sourced). Asserted across
/// `SEEDS` so it reflects a robust behavioural property rather than one seed's
/// luck. (This — not a system-wide consumption share — is the domain definition
/// of "reads as a Decomposer".)
#[test]
fn decomposer_reads_as_decomposer_role() {
    for seed in SEEDS {
        let (world, topology) = run_seed(seed);
        let roles = topology.trophic_roles(world.agents());
        let decomposers = roles
            .values()
            .filter(|&&r| r == TrophicRole::Decomposer)
            .count();
        assert!(
            decomposers >= 1,
            "seed {seed}: expected a surviving agent to read as Decomposer; roles = {:?}",
            roles
        );
    }
}

/// The seeded decomposer must not starve out: a heterotrophy-dominant agent
/// persists to the end of the run on the carcass supply, on every seed.
#[test]
fn decomposer_sustains_itself_to_end_of_run() {
    for seed in SEEDS {
        let (world, _topology) = run_seed(seed);
        let surviving_heterotrophs = world
            .agents()
            .iter()
            .filter(|a| a.traits.heterotrophy > a.traits.photosynthetic_absorption)
            .count();
        assert!(
            surviving_heterotrophs >= 1,
            "seed {seed}: decomposer lineage must survive to the end of the run, but no \
             heterotroph-dominant agent remains (final population {})",
            world.agents().len()
        );
    }
}

/// Cumulative predation vs decomposition energy across the run, summed from the
/// `Consumed` event stream (the same green/brown split `trophic_roles` reads).
fn predation_vs_decomposition(seed: u64) -> (f32, f32) {
    let recipe = load();
    let mut world = World::from_recipe(&recipe, seed);
    let mut predation = 0.0f32;
    let mut decomposition = 0.0f32;
    let mut cursor = 0usize;
    for _ in 0..recipe.max_ticks {
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
        if world.agents().is_empty() {
            break;
        }
    }
    (predation, decomposition)
}

/// Regression for #303 ("carcasses accumulate unconsumed"): the detrital pathway
/// must carry real, *substantial* energy on every run — carcasses are actively
/// drained, not merely accumulating. Whether detrital is the MAJORITY of the
/// decomposer's intake is the authoritative role check
/// (`decomposer_reads_as_decomposer_role`), which reads the surviving
/// decomposer's own diet; this test instead guards that the brown pathway is a
/// major, live energy route system-wide. A flat-majority assertion on
/// *system-wide* consumption was dropped: it conflated the decomposer's own diet
/// with the transient predation of short-lived lineage members, and was an
/// artifact of the pre-#310 bug (doomed newborns were pure-decomposition
/// carcasses).
#[test]
fn carcasses_are_consumed_through_the_detrital_pathway() {
    for seed in SEEDS {
        let (predation, decomposition) = predation_vs_decomposition(seed);
        assert!(
            decomposition > 0.0,
            "seed {seed}: the detrital pathway must carry real energy \
             (decomposition = {decomposition})"
        );
        let detrital_share = decomposition / (predation + decomposition);
        assert!(
            detrital_share > 0.3,
            "seed {seed}: detrital energy must be a substantial share of system consumption \
             (predation {predation:.1}, decomposition {decomposition:.1}, \
             detrital share {detrital_share:.3})"
        );
    }
}
