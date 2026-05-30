//! Headless run of scenarios/example4_consumer_tuning.json to diagnose why no
//! reproduction is observed. Run with:
//!   cargo test -p explorers-sim --test headless_example4 -- --nocapture

use explorers_sim::{Agent, World, WorldRecipe};

const SCENARIO: &str = "/Users/sam/Projects/explorers/scenarios/example4_consumer_tuning.json";

fn is_producer(a: &Agent) -> bool {
    a.traits.photosynthetic_absorption >= a.traits.heterotrophy
}

fn load() -> WorldRecipe {
    let contents = std::fs::read_to_string(SCENARIO).expect("read scenario");
    serde_json::from_str(&contents).expect("parse scenario")
}

#[test]
fn diagnose_example4() {
    let recipe = load();
    let e_thresh = recipe.parameters.reproduction_energy_threshold;
    let n_thresh = recipe.parameters.reproduction_nutrient_threshold;
    println!(
        "thresholds: repro_energy >= {e_thresh}, repro_nutrient >= {n_thresh}; max_ticks {}",
        recipe.max_ticks
    );

    let seed = 1u64;
    let mut world = World::from_recipe(&recipe, seed);
    let ticks = recipe.max_ticks;

    let mut cum_births = 0usize;
    let mut cum_deaths = 0usize;
    // Peak gate values ever seen, per role, to see how close each role gets.
    let mut peak_prod_e = 0f32;
    let mut peak_prod_n = 0f32;
    let mut peak_cons_e = 0f32;
    let mut peak_cons_n = 0f32;
    // How many ticks at least one agent was reproduction-eligible (both gates) but
    // population did not grow that tick (suggests a mate-finding block, not a gate block).
    let mut eligible_but_no_birth_ticks = 0usize;

    println!(
        "{:>5} {:>5} {:>5} {:>6} {:>7} {:>7} | {:>9} {:>9} | {:>9} {:>9} | {:>5} {:>5}",
        "tick", "prod", "cons", "carc", "births", "deaths",
        "prodRepE", "prodRepN", "consRepE", "consRepN", "elig", "eligC"
    );

    for t in 0..ticks {
        world.step();
        let b = world.last_tick_births();
        let d = world.last_tick_deaths();
        cum_births += b;
        cum_deaths += d;

        let agents = world.agents();
        let mut n_prod = 0;
        let mut n_cons = 0;
        let mut max_prod_e = 0f32;
        let mut max_prod_n = 0f32;
        let mut max_cons_e = 0f32;
        let mut max_cons_n = 0f32;
        let mut eligible = 0;
        let mut eligible_cons = 0;
        for a in agents {
            let elig = a.repro_reserve >= e_thresh && a.repro_nutrient >= n_thresh;
            if elig {
                eligible += 1;
            }
            if is_producer(a) {
                n_prod += 1;
                max_prod_e = max_prod_e.max(a.repro_reserve);
                max_prod_n = max_prod_n.max(a.repro_nutrient);
            } else {
                n_cons += 1;
                max_cons_e = max_cons_e.max(a.repro_reserve);
                max_cons_n = max_cons_n.max(a.repro_nutrient);
                if elig {
                    eligible_cons += 1;
                }
            }
        }
        peak_prod_e = peak_prod_e.max(max_prod_e);
        peak_prod_n = peak_prod_n.max(max_prod_n);
        peak_cons_e = peak_cons_e.max(max_cons_e);
        peak_cons_n = peak_cons_n.max(max_cons_n);
        if eligible > 0 && b == 0 {
            eligible_but_no_birth_ticks += 1;
        }

        if t < 10 || t % 50 == 49 || (b > 0) {
            println!(
                "{:>5} {:>5} {:>5} {:>6} {:>7} {:>7} | {:>9.2} {:>9.2} | {:>9.2} {:>9.2} | {:>5} {:>5}",
                t + 1, n_prod, n_cons, world.carcasses().len(), b, d,
                max_prod_e, max_prod_n, max_cons_e, max_cons_n, eligible, eligible_cons
            );
        }
    }

    let agents = world.agents();
    let final_prod = agents.iter().filter(|a| is_producer(a)).count();
    let final_cons = agents.len() - final_prod;
    println!("\n=== SUMMARY (seed {seed}) ===");
    println!("final population: {} ({final_prod} producers, {final_cons} consumers)", agents.len());
    println!("cumulative births: {cum_births}, cumulative deaths: {cum_deaths}");
    println!("peak producer gates: repro_reserve {peak_prod_e:.2}/{e_thresh}, repro_nutrient {peak_prod_n:.2}/{n_thresh}");
    println!("peak consumer gates: repro_reserve {peak_cons_e:.2}/{e_thresh}, repro_nutrient {peak_cons_n:.2}/{n_thresh}");
    println!("ticks with an eligible agent but zero births: {eligible_but_no_birth_ticks}");
}

#[test]
fn births_across_seeds() {
    let recipe = load();
    println!("births over {} ticks, by seed:", recipe.max_ticks);
    for seed in 0..8u64 {
        let mut world = World::from_recipe(&recipe, seed);
        let mut births = 0usize;
        let mut deaths = 0usize;
        for _ in 0..recipe.max_ticks {
            world.step();
            births += world.last_tick_births();
            deaths += world.last_tick_deaths();
        }
        let agents = world.agents();
        let prod = agents.iter().filter(|a| is_producer(a)).count();
        let cons = agents.len() - prod;
        println!(
            "seed {seed}: births {births:>4}, deaths {deaths:>4}, final pop {:>3} ({prod} prod / {cons} cons)",
            agents.len()
        );
    }
}
