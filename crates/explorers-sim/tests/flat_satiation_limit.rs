//! The flat limiting case of need-gated consumption (#600): at
//! `satiation_sensitivity = 0` every consumer expresses its full heterotrophic
//! capability, so worlds step exactly as they did before need-gating. The
//! fingerprints below were taken from the pre-need-gating stepper (main at
//! f6a1c1f) running the same scenarios, seeds and horizons; a flat gate must
//! reproduce them bit for bit.

use explorers_sim::{World, WorldRecipe};

fn load_recipe(name: &str) -> WorldRecipe {
    let path = format!("{}/../../scenarios/{}", env!("CARGO_MANIFEST_DIR"), name);
    let contents = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_json::from_str(&contents).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

/// FNV-1a over every living agent's and carcass's state, bit-exact.
fn fingerprint(world: &World) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |x: u64| {
        for b in x.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    eat(world.agents().len() as u64);
    for a in world.agents() {
        eat(a.id);
        for f in [
            a.position.0,
            a.position.1,
            a.reserve,
            a.structure,
            a.nutrient,
            a.repro_reserve,
            a.repro_nutrient,
        ] {
            eat(f.to_bits() as u64);
        }
    }
    eat(world.carcasses().len() as u64);
    for c in world.carcasses() {
        eat(c.id);
        for f in [c.position.0, c.position.1, c.energy, c.nutrient] {
            eat(f.to_bits() as u64);
        }
    }
    h
}

/// The scenarios, seeds and horizons, with the pre-need-gating fingerprint of
/// each.
const PINNED: [(&str, u64, u64, u64); 3] = [
    ("example4.json", 7, 200, 0x5b79_dab7_1a9d_0b82),
    (
        "example9_detrital_pathway.json",
        11,
        200,
        0x691b_a488_6ebf_7c69,
    ),
    (
        "example10_predator_prey_hopf.json",
        3,
        200,
        0x8eae_0217_0bec_ce73,
    ),
];

fn run(scenario: &str, seed: u64, ticks: u64, satiation_sensitivity: Option<f32>) -> u64 {
    let mut recipe = load_recipe(scenario);
    if let Some(c) = satiation_sensitivity {
        recipe.parameters.satiation_sensitivity = c;
    }
    let mut world = World::from_recipe(&recipe, seed);
    for _ in 0..ticks {
        world.step();
    }
    fingerprint(&world)
}

#[test]
fn a_flat_satiation_response_reproduces_the_ungated_stepper() {
    for (scenario, seed, ticks, pinned) in PINNED {
        let got = run(scenario, seed, ticks, Some(0.0));
        assert_eq!(
            got, pinned,
            "{scenario} seed {seed}: flat gate {got:#018x} != pre-need-gating {pinned:#018x}"
        );
    }
}

#[test]
fn the_default_satiation_response_gates_these_worlds() {
    // The pin above has teeth only if these worlds feed: under the default
    // (gated) response at least one of them must leave the ungated trajectory.
    let moved = PINNED
        .iter()
        .filter(|&&(scenario, seed, ticks, pinned)| run(scenario, seed, ticks, None) != pinned)
        .count();
    assert!(moved > 0, "no pinned world is changed by the default gate");
}
