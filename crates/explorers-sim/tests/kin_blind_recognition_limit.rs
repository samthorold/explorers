//! The kin-blind limiting case of recognition (#604): at
//! `recognition_distance = 0` no living target resembles its consumer, so
//! worlds step exactly as they did before recognition. The fingerprints below
//! were taken from the pre-recognition stepper (main at c095f04, need-gated
//! consumption at the default satiation sensitivity) running the same
//! scenarios, seeds and horizons; a kin-blind world must reproduce them bit
//! for bit.

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

/// The scenarios, seeds and horizons, with the pre-recognition fingerprint of
/// each.
const PINNED: [(&str, u64, u64, u64); 5] = [
    ("example4.json", 7, 200, 0x6e9d_d215_9f72_ddb0),
    (
        "example9_detrital_pathway.json",
        11,
        200,
        0x2fde_47d6_42f8_6ae4,
    ),
    (
        "example10_predator_prey_hopf.json",
        3,
        200,
        0x2d39_3fb0_a74d_59a7,
    ),
    ("example13_closed_web.json", 5, 200, 0xaa58_bd9d_8728_d0f8),
    (
        "example11_branching_coexistence.json",
        2,
        200,
        0xc92d_6164_6204_61c6,
    ),
];

fn run(scenario: &str, seed: u64, ticks: u64, recognition_distance: Option<f32>) -> u64 {
    let mut recipe = load_recipe(scenario);
    if let Some(d) = recognition_distance {
        recipe.parameters.recognition_distance = d;
    }
    let mut world = World::from_recipe(&recipe, seed);
    for _ in 0..ticks {
        world.step();
    }
    fingerprint(&world)
}

#[test]
fn a_zero_recognition_distance_reproduces_the_kin_blind_stepper() {
    for (scenario, seed, ticks, pinned) in PINNED {
        let got = run(scenario, seed, ticks, Some(0.0));
        assert_eq!(
            got, pinned,
            "{scenario} seed {seed}: kin-blind {got:#018x} != pre-recognition {pinned:#018x}"
        );
    }
}

#[test]
fn the_default_recognition_distance_changes_these_worlds() {
    // The pin above has teeth only if these worlds graze resembling targets:
    // under the default recognition distance at least one of them must leave
    // the kin-blind trajectory.
    let moved = PINNED
        .iter()
        .filter(|&&(scenario, seed, ticks, pinned)| run(scenario, seed, ticks, None) != pinned)
        .count();
    assert!(
        moved > 0,
        "no pinned world is changed by default recognition"
    );
}
