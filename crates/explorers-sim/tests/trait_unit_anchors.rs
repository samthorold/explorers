//! Unit anchors: trait-to-flow conversions (#459) and use-wear couplings (#460).
//!
//! Five committed forms read a dimensionless trait *as* a dimensional flow
//! (see `docs/research/440-dimensionless-groups.md`, smell S1). The design's
//! AFK posture is to name those anchors as explicit constants — each `1.0` in
//! today's units — without changing any trajectory. This suite is the
//! byte-identity guard: a golden hash of fixed-seed runs pinned *before* the
//! constants were introduced, which must survive their introduction unchanged.
//! The same guard covers the three use-wear anchors (smell S2), pinned on a
//! configuration where `use_wear_rate > 0` and all three couplings are live.

use explorers_sim::{World, WorldRecipe};
use std::hash::{DefaultHasher, Hash, Hasher};

fn load_recipe(name: &str) -> WorldRecipe {
    let path = format!("{}/../../scenarios/{}", env!("CARGO_MANIFEST_DIR"), name);
    let contents = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_json::from_str(&contents).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

/// Bit-exact digest of a run: every agent and carcass field (via `to_bits`),
/// plus the nutrient grid, folded through a `DefaultHasher` after each tick so
/// the whole trajectory — not just the end state — is pinned.
fn trajectory_hash(name: &str, seed: u64, ticks: u64) -> u64 {
    recipe_trajectory_hash(name, &load_recipe(name), seed, ticks)
}

fn recipe_trajectory_hash(name: &str, recipe: &WorldRecipe, seed: u64, ticks: u64) -> u64 {
    let mut world = World::from_recipe(recipe, seed);
    let mut h = DefaultHasher::new();
    for _ in 0..ticks {
        world.step();
        world.tick().hash(&mut h);
        for a in world.agents() {
            a.id.hash(&mut h);
            a.position.0.to_bits().hash(&mut h);
            a.position.1.to_bits().hash(&mut h);
            a.reserve.to_bits().hash(&mut h);
            a.structure.to_bits().hash(&mut h);
            a.nutrient.to_bits().hash(&mut h);
            a.repro_reserve.to_bits().hash(&mut h);
            a.repro_nutrient.to_bits().hash(&mut h);
            for d in 0..7 {
                a.traits.get(d).to_bits().hash(&mut h);
            }
            for w in a.wear.iter() {
                w.to_bits().hash(&mut h);
            }
        }
        for c in world.carcasses() {
            c.id.hash(&mut h);
            c.energy.to_bits().hash(&mut h);
            c.nutrient.to_bits().hash(&mut h);
        }
        world.nutrient_grid().total().to_bits().hash(&mut h);
    }
    assert!(
        !world.agents().is_empty(),
        "{name}: population died before tick {ticks}; the golden run no longer exercises the anchors"
    );
    h.finish()
}

/// Golden trajectory digests, pinned on `main` at 5a7bede (before the unit
/// anchors were named). Three scenarios between them exercise every anchor:
/// example4 (mobility → distance, autotrophy → uptake, heterotrophy → drain,
/// dispersal → σ), example8 (wear_rate > 0, so kappa → repair is live),
/// example10 (predator–prey, sexual and asexual placement kernels).
const GOLDEN: [(&str, u64, u64, u64); 3] = [
    ("example4.json", 7, 300, 0xe96aa0b808bdf6fb),
    ("example8.json", 11, 300, 0x366aea7e88b3c291),
    (
        "example10_predator_prey_hopf.json",
        3,
        300,
        0xbb8c3ea933cf05fb,
    ),
];

#[test]
fn naming_the_unit_anchors_leaves_every_trajectory_byte_identical() {
    for (name, seed, ticks, expected) in GOLDEN {
        let got = trajectory_hash(name, seed, ticks);
        assert_eq!(
            got, expected,
            "{name} seed={seed} ticks={ticks}: trajectory digest {got:#018x} != golden {expected:#018x}"
        );
    }
}

/// Use-dependent wear anchors (#460). `use_wear_rate` multiplies three
/// dimensionally distinct usages — energy captured, energy drained, distance
/// moved — with one coefficient. Naming the three implied unit constants must
/// not change the `use_wear_rate > 0` branch bit-for-bit, so pin it on a
/// config where all three couplings are live at once: example10's sessile
/// producers capture energy while its lightly mobile consumers both drain
/// structure and move. Digest pinned on `main` at 5a7bede, before the anchors
/// were named.
const USE_WEAR_GOLDEN: (&str, f32, u64, u64, u64) = (
    "example10_predator_prey_hopf.json",
    0.02,
    3,
    300,
    0x2ddcc795057d930b,
);

#[test]
fn naming_the_use_wear_anchors_leaves_the_use_wear_branch_byte_identical() {
    let (name, use_wear_rate, seed, ticks, expected) = USE_WEAR_GOLDEN;
    let mut recipe = load_recipe(name);
    recipe.parameters.use_wear_rate = use_wear_rate;
    let got = recipe_trajectory_hash(name, &recipe, seed, ticks);
    assert_eq!(
        got, expected,
        "{name} use_wear_rate={use_wear_rate} seed={seed} ticks={ticks}: trajectory digest {got:#018x} != golden {expected:#018x}"
    );
}

#[test]
fn every_trait_to_flow_anchor_is_one_in_todays_units() {
    use explorers_sim::units::*;
    // The AFK posture for #459: name the anchors, do not change them. Each is
    // exactly 1.0 — the value the stepper has always used implicitly.
    assert_eq!(
        MOBILITY_DISTANCE_PER_TICK, 1.0,
        "u_M: length per tick per trait unit"
    );
    assert_eq!(DISPERSAL_KERNEL_SIGMA, 1.0, "u_D: length per trait unit");
    assert_eq!(
        AUTOTROPHY_NUTRIENT_UPTAKE_PER_TICK, 1.0,
        "u_A: nutrient per tick per trait unit"
    );
    assert_eq!(
        HETEROTROPHY_STRUCTURE_DRAIN_PER_TICK, 1.0,
        "u_H: energy per tick per trait unit"
    );
    assert_eq!(
        KAPPA_REPAIR_PER_TICK, 1.0,
        "u_R: energy per tick per unit kappa"
    );
}

#[test]
fn every_use_wear_anchor_is_one_in_todays_units() {
    use explorers_sim::units::*;
    // The AFK posture for #460: `use_wear_rate` stays one coefficient; the
    // three unit constants it silently carries are named, not parameterised.
    assert_eq!(
        USE_WEAR_PER_ENERGY_CAPTURED, 1.0,
        "u_WA: wear (energy) per energy captured — dimensionless"
    );
    assert_eq!(
        USE_WEAR_PER_ENERGY_DRAINED, 1.0,
        "u_WH: wear (energy) per energy drained — dimensionless"
    );
    assert_eq!(
        USE_WEAR_PER_DISTANCE_MOVED, 1.0,
        "u_WM: wear (energy) per unit distance moved"
    );
}
