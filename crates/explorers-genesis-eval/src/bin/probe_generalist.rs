//! Generalist-dominance diagnostic for issue #325 — measures whether broad
//! generalists are confined or dominate in a scenario, over the standard 8-seed
//! ensemble (mirrors `eval_scenarios`' seed set: `base_seed=1 .. 8`).
//!
//! The `observed.json` evidence carries the evaluator's `generalist_dominance`
//! gate, but that gate is dbscan- and 20-agent-gated and energy-weights whole
//! clusters; it does not report the survivor *trait breadth* the issue asks for.
//! This binary defines the breadth/dominance measure directly on survivor traits
//! (no dbscan, no population floor): a *trophic generalist* invests in both
//! autotrophy and heterotrophy (photo > 0.25 AND het > 0.25); a *broad
//! generalist* additionally invests in mobility (the rooted-producer + roving-
//! hunter the design rules out by incompatibility). It reports the energy-
//! weighted generalist share and the mean structural fragility (trait-vector
//! entropy = "breadth") of survivors — the read behind the #325 verdict in
//! `scenarios/verdicts.md`. Run:
//!   cargo run -p explorers-genesis-eval --bin probe_generalist -- \
//!     scenarios/example12_generalist_dominance.json

use explorers_sim::{World, WorldRecipe, structural_fragility};

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: probe_generalist FILE");
    let recipe: WorldRecipe =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

    println!(
        "{:>4} {:>5} | {:>6} {:>6} {:>6} {:>6} | {:>8} {:>8} | {:>6}",
        "seed", "pop", "prod", "cons", "broad", "compat", "gen_E%", "broadE%", "fragil"
    );
    let mut gen_shares = Vec::new();
    let mut broad_shares = Vec::new();
    for i in 0..8u64 {
        let seed = 1u64.wrapping_add(i);
        let mut world = World::from_recipe(&recipe, seed);
        for _ in 0..recipe.max_ticks {
            world.step();
            if world.agents().is_empty() {
                break;
            }
        }
        let agents = world.agents();
        let total_e: f32 = agents.iter().map(|a| a.energy()).sum();
        let (mut prod, mut cons, mut broad, mut compat) = (0, 0, 0, 0);
        let (mut gen_e, mut broad_e) = (0.0f32, 0.0f32);
        let mut frag_sum = 0.0f32;
        for a in agents {
            let t = a.traits;
            let auto = t.photosynthetic_absorption > 0.25;
            let het = t.heterotrophy > 0.25;
            let mob = t.mobility > 0.25;
            let trophic_gen = auto && het;
            let broad_gen = trophic_gen && mob;
            if trophic_gen {
                gen_e += a.energy();
            }
            if broad_gen {
                broad_e += a.energy();
            }
            // archetype tally by dominant trait pattern (descriptive only)
            match (auto, het, mob) {
                (true, false, _) => prod += 1,
                (false, true, _) => cons += 1,
                (true, true, true) => broad += 1,
                (true, true, false) => compat += 1,
                _ => {}
            }
            frag_sum += structural_fragility(&t);
        }
        let n = agents.len().max(1) as f32;
        let gshare = if total_e > 0.0 { gen_e / total_e } else { 0.0 };
        let bshare = if total_e > 0.0 {
            broad_e / total_e
        } else {
            0.0
        };
        gen_shares.push(gshare);
        broad_shares.push(bshare);
        println!(
            "{:>4} {:>5} | {:>6} {:>6} {:>6} {:>6} | {:>7.1}% {:>7.1}% | {:>6.3}",
            seed,
            agents.len(),
            prod,
            cons,
            broad,
            compat,
            gshare * 100.0,
            bshare * 100.0,
            frag_sum / n
        );
    }
    let med = |v: &mut Vec<f32>| {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v[v.len() / 2]
    };
    println!(
        "median trophic-generalist energy share: {:.1}%",
        med(&mut gen_shares) * 100.0
    );
    println!(
        "median broad-generalist energy share:   {:.1}%",
        med(&mut broad_shares) * 100.0
    );
}
