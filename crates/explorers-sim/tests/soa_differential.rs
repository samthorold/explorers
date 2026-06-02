//! Differential bit-identity tests for the SoA vectorised elementwise phases
//! (spike #354). For each elementwise phase we run a populated world's agent
//! set through both the scalar AoS phase and the SoA phase and assert that
//! *every* agent field is bit-identical (compared by `f32::to_bits`), and that
//! the returned events and dissipated totals match exactly.
//!
//! This is the green bar for the determinism guard: if any field differs, the
//! SoA phase has changed evaluation order or arithmetic and is NOT a
//! transparent optimisation.

use explorers_sim::soa::AgentSoA;
use explorers_sim::{Agent, World, WorldParameters, WorldRecipe, phase, soa};

/// Load a scenario recipe by filename under scenarios/.
fn load_recipe(name: &str) -> WorldRecipe {
    let path = format!("{}/../../scenarios/{}", env!("CARGO_MANIFEST_DIR"), name);
    let contents = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_json::from_str(&contents).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

/// Produce a realistic, populated agent set by running a scenario `warmup`
/// ticks and snapshotting the live agents. A warmed world exercises wear,
/// nonzero reproductive earmarks, varied structure — the states the phases act
/// on — rather than the uniform tick-0 seeding.
fn warmed_agents(recipe: &WorldRecipe, seed: u64, warmup: u64) -> (Vec<Agent>, WorldParameters) {
    let mut world = World::from_recipe(recipe, seed);
    for _ in 0..warmup {
        world.step();
    }
    let agents: Vec<Agent> = world.agents().to_vec();
    (agents, recipe.parameters.clone())
}

/// Assert two agents are bit-identical across every field.
#[track_caller]
fn assert_agent_bit_identical(a: &Agent, b: &Agent, ctx: &str) {
    assert_eq!(a.id, b.id, "{ctx}: id");
    assert_eq!(a.contact_time, b.contact_time, "{ctx}: contact_time");
    bit_eq(a.position.0, b.position.0, ctx, "position.0");
    bit_eq(a.position.1, b.position.1, ctx, "position.1");
    bit_eq(a.reserve, b.reserve, ctx, "reserve");
    bit_eq(a.structure, b.structure, ctx, "structure");
    bit_eq(a.peak_structure, b.peak_structure, ctx, "peak_structure");
    bit_eq(a.nutrient, b.nutrient, ctx, "nutrient");
    bit_eq(a.repro_reserve, b.repro_reserve, ctx, "repro_reserve");
    bit_eq(a.repro_nutrient, b.repro_nutrient, ctx, "repro_nutrient");
    for d in 0..7 {
        bit_eq(a.traits.get(d), b.traits.get(d), ctx, "trait");
    }
    for ft in 0..3 {
        bit_eq(a.wear[ft], b.wear[ft], ctx, "wear");
    }
}

#[track_caller]
fn bit_eq(a: f32, b: f32, ctx: &str, field: &str) {
    assert_eq!(
        a.to_bits(),
        b.to_bits(),
        "{ctx}: field {field} differs: scalar={a} ({:#010x}) soa={b} ({:#010x})",
        a.to_bits(),
        b.to_bits()
    );
}

/// Run a phase both ways and assert bit-identity of agents + events + dissipated.
fn assert_metabolise_identical(agents: &[Agent], params: &WorldParameters, ctx: &str) {
    let mut scalar = agents.to_vec();
    let (scalar_events, scalar_diss) = phase::metabolise(&mut scalar, params);

    let mut s = AgentSoA::from_agents(agents);
    let (soa_events, soa_diss) = soa::metabolise_soa(&mut s, params);
    let after = s.to_agents();

    bit_eq(scalar_diss, soa_diss, ctx, "dissipated");
    assert_eq!(scalar_events.len(), soa_events.len(), "{ctx}: event count");
    for (se, oe) in scalar_events.iter().zip(soa_events.iter()) {
        assert_eq!(se.kind, oe.kind, "{ctx}: event kind");
        assert_eq!(se.source, oe.source, "{ctx}: event source");
        bit_eq(se.energy_delta, oe.energy_delta, ctx, "event energy_delta");
    }
    assert_eq!(scalar.len(), after.len(), "{ctx}: agent count");
    for (sa, oa) in scalar.iter().zip(after.iter()) {
        assert_agent_bit_identical(sa, oa, ctx);
    }
}

fn assert_grow_identical(agents: &[Agent], params: &WorldParameters, ctx: &str) {
    let mut scalar = agents.to_vec();
    let (scalar_events, scalar_diss) = phase::grow(&mut scalar, params);

    let mut s = AgentSoA::from_agents(agents);
    let (soa_events, soa_diss) = soa::grow_soa(&mut s, params);
    let after = s.to_agents();

    bit_eq(scalar_diss, soa_diss, ctx, "dissipated");
    assert_eq!(scalar_events.len(), soa_events.len(), "{ctx}: event count");
    for (se, oe) in scalar_events.iter().zip(soa_events.iter()) {
        assert_eq!(se.kind, oe.kind, "{ctx}: event kind");
        assert_eq!(se.source, oe.source, "{ctx}: event source");
        bit_eq(se.energy_delta, oe.energy_delta, ctx, "event energy_delta");
    }
    assert_eq!(scalar.len(), after.len(), "{ctx}: agent count");
    for (sa, oa) in scalar.iter().zip(after.iter()) {
        assert_agent_bit_identical(sa, oa, ctx);
    }
}

fn assert_wear_identical(
    agents: &[Agent],
    params: &WorldParameters,
    usage: &std::collections::HashMap<u64, [f32; 3]>,
    ctx: &str,
) {
    let mut scalar = agents.to_vec();
    let scalar_events = phase::apply_wear(&mut scalar, params, usage);

    let mut s = AgentSoA::from_agents(agents);
    let soa_events = soa::apply_wear_soa(&mut s, params, usage);
    let after = s.to_agents();

    assert_eq!(scalar_events.len(), soa_events.len(), "{ctx}: event count");
    for (se, oe) in scalar_events.iter().zip(soa_events.iter()) {
        assert_eq!(se.kind, oe.kind, "{ctx}: event kind");
        assert_eq!(se.source, oe.source, "{ctx}: event source");
        bit_eq(se.energy_delta, oe.energy_delta, ctx, "event energy_delta");
    }
    assert_eq!(scalar.len(), after.len(), "{ctx}: agent count");
    for (sa, oa) in scalar.iter().zip(after.iter()) {
        assert_agent_bit_identical(sa, oa, ctx);
    }
}

#[test]
fn soa_store_round_trips_bit_identically() {
    let recipe = load_recipe("example4.json");
    let (agents, _params) = warmed_agents(&recipe, 1, 50);
    assert!(!agents.is_empty(), "warmed world should have agents");

    let s = AgentSoA::from_agents(&agents);
    assert_eq!(s.len(), agents.len());
    let back = s.to_agents();
    for (orig, rt) in agents.iter().zip(back.iter()) {
        assert_agent_bit_identical(orig, rt, "round-trip");
    }
}

#[test]
fn metabolise_soa_is_bit_identical_example4() {
    let recipe = load_recipe("example4.json");
    let (agents, params) = warmed_agents(&recipe, 1, 50);
    assert!(!agents.is_empty());
    assert_metabolise_identical(&agents, &params, "example4 metabolise");
}

#[test]
fn grow_soa_is_bit_identical_example4() {
    let recipe = load_recipe("example4.json");
    let (agents, params) = warmed_agents(&recipe, 1, 50);
    assert!(!agents.is_empty());
    assert_grow_identical(&agents, &params, "example4 grow");
}

#[test]
fn apply_wear_soa_is_bit_identical_example4() {
    let recipe = load_recipe("example4.json");
    let (agents, params) = warmed_agents(&recipe, 1, 50);
    assert!(!agents.is_empty());
    // Build a usage map covering most agents with varied per-trait usage, plus
    // leave some agents absent (default-zero usage) to exercise both branches.
    let mut usage = std::collections::HashMap::new();
    for (k, a) in agents.iter().enumerate() {
        if k % 5 == 0 {
            continue; // some agents have no usage entry
        }
        let f = (k as f32) * 0.013;
        usage.insert(a.id, [f, f * 0.5, f * 0.25]);
    }
    assert_wear_identical(&agents, &params, &usage, "example4 wear");
}

/// Stronger integration check: step a real world for many ticks, and at every
/// tick snapshot the live population and run the three elementwise phases both
/// ways, asserting bit-identity of every agent field. This exercises the
/// evolving population (varied wear, structure, earmarks; births and deaths
/// reshaping the store) across the full trajectory, not just one warmed state.
fn multitick_differential(scenario: &str, seed: u64, ticks: u64) {
    let recipe = load_recipe(scenario);
    let params = recipe.parameters.clone();
    let mut world = World::from_recipe(&recipe, seed);
    let mut max_pop = 0usize;
    for t in 0..ticks {
        let agents: Vec<Agent> = world.agents().to_vec();
        max_pop = max_pop.max(agents.len());
        if !agents.is_empty() {
            let ctx = format!("{scenario} seed {seed} tick {t}");
            assert_metabolise_identical(&agents, &params, &ctx);
            assert_grow_identical(&agents, &params, &ctx);
            // Deterministic synthetic usage map derived from agent id.
            let mut usage = std::collections::HashMap::new();
            for a in &agents {
                let f = ((a.id % 17) as f32) * 0.01;
                usage.insert(a.id, [f, f * 0.3, f * 0.7]);
            }
            assert_wear_identical(&agents, &params, &usage, &ctx);
        }
        world.step();
    }
    assert!(max_pop > 0, "{scenario}: world never had agents");
}

#[test]
fn elementwise_phases_bit_identical_over_example4_trajectory() {
    multitick_differential("example4.json", 1, 200);
}

#[test]
fn elementwise_phases_bit_identical_over_example9_trajectory() {
    multitick_differential("example9_detrital_pathway.json", 3, 200);
}
