//! Single-rollout benchmark for spike #354: scalar stepper vs. SoA-vectorised
//! elementwise phases.
//!
//! Run with optimisations (the only setting where autovectorisation is real):
//!   cargo run --release -p explorers-sim --bin bench_soa
//!
//! What it measures, per scenario, over a single rollout (one seed):
//!   - total wall-time of the full scalar `World::step()` loop (the latency
//!     floor #354 is attacking);
//!   - the summed wall-time of the three elementwise phases (`metabolise`,
//!     `grow`, `apply_wear`) run scalar vs. run over the SoA layout — measured
//!     on the *same* live population each tick, so the comparison is apples to
//!     apples and includes the SoA build + write-back cost;
//!   - the elementwise phases' share of a full tick.
//!
//! The SoA timing includes `AgentSoA::from_agents` and `to_agents` each tick,
//! i.e. the full standalone cost. In a real integrated stepper the world would
//! stay in SoA form across phases and pay the conversion at most once per tick,
//! so this is a conservative (pessimistic) read of the SoA win.

use std::time::Instant;

use explorers_sim::soa::AgentSoA;
use explorers_sim::{Agent, World, WorldParameters, WorldRecipe, phase, soa};

fn load_recipe(name: &str) -> WorldRecipe {
    let path = format!("{}/../../scenarios/{}", env!("CARGO_MANIFEST_DIR"), name);
    let contents = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_json::from_str(&contents).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

/// Synthetic per-agent usage map (the movement phase builds the real one; for
/// timing the wear phase any map of the right shape suffices).
fn usage_for(agents: &[Agent]) -> std::collections::HashMap<u64, [f32; 3]> {
    let mut m = std::collections::HashMap::with_capacity(agents.len());
    for a in agents {
        let f = ((a.id % 17) as f32) * 0.01;
        m.insert(a.id, [f, f * 0.3, f * 0.7]);
    }
    m
}

struct Row {
    scenario: String,
    ticks: u64,
    peak_pop: usize,
    final_pop: usize,
    total_scalar_s: f64,
    elementwise_scalar_s: f64,
    elementwise_soa_s: f64,
}

fn bench(scenario: &str, seed: u64) -> Row {
    let recipe = load_recipe(scenario);
    let params: WorldParameters = recipe.parameters.clone();
    let ticks = recipe.max_ticks;

    let mut world = World::from_recipe(&recipe, seed);
    let mut elementwise_scalar_s = 0.0_f64;
    let mut elementwise_soa_s = 0.0_f64;
    let mut peak_pop = 0usize;

    let t_total = Instant::now();
    for _ in 0..ticks {
        let agents: Vec<Agent> = world.agents().to_vec();
        peak_pop = peak_pop.max(agents.len());
        if !agents.is_empty() {
            let usage = usage_for(&agents);

            // Scalar elementwise: time the three phases on a fresh clone.
            let mut scalar = agents.clone();
            let t = Instant::now();
            let _ = phase::metabolise(&mut scalar, &params);
            let _ = phase::grow(&mut scalar, &params);
            let _ = phase::apply_wear(&mut scalar, &params, &usage);
            elementwise_scalar_s += t.elapsed().as_secs_f64();

            // SoA elementwise: include build + write-back (standalone cost).
            let t = Instant::now();
            let mut s = AgentSoA::from_agents(&agents);
            let _ = soa::metabolise_soa(&mut s, &params);
            let _ = soa::grow_soa(&mut s, &params);
            let _ = soa::apply_wear_soa(&mut s, &params, &usage);
            let _back = s.to_agents();
            elementwise_soa_s += t.elapsed().as_secs_f64();
        }
        world.step();
    }
    let total_scalar_s = t_total.elapsed().as_secs_f64();

    Row {
        scenario: scenario.to_string(),
        ticks,
        peak_pop,
        final_pop: world.agents().len(),
        total_scalar_s,
        elementwise_scalar_s,
        elementwise_soa_s,
    }
}

fn main() {
    println!("# Spike #354 — single-rollout benchmark (scalar vs SoA elementwise)\n");
    #[cfg(debug_assertions)]
    println!("WARNING: running in debug. Use `cargo run --release` for meaningful numbers.\n");

    let rows = [
        bench("example4.json", 1),
        bench("example9_detrital_pathway.json", 3),
    ];

    println!(
        "{:<32} {:>6} {:>9} {:>9} {:>12} {:>14} {:>12} {:>10} {:>9}",
        "scenario",
        "ticks",
        "peak_pop",
        "final_pop",
        "rollout(s)",
        "elem_scalar(s)",
        "elem_soa(s)",
        "elem%tick",
        "soa_x"
    );
    for r in &rows {
        let elem_share = 100.0 * r.elementwise_scalar_s / r.total_scalar_s;
        let speedup = if r.elementwise_soa_s > 0.0 {
            r.elementwise_scalar_s / r.elementwise_soa_s
        } else {
            f64::NAN
        };
        println!(
            "{:<32} {:>6} {:>9} {:>9} {:>12.3} {:>14.4} {:>12.4} {:>9.1}% {:>8.2}x",
            r.scenario,
            r.ticks,
            r.peak_pop,
            r.final_pop,
            r.total_scalar_s,
            r.elementwise_scalar_s,
            r.elementwise_soa_s,
            elem_share,
            speedup
        );
    }
    println!(
        "\nNote: elem_soa(s) includes AgentSoA build + write-back each tick (standalone cost)."
    );
}
