//! Hypothesis Sp (`docs/system-design/end-state.md`, part 2, under P1): where
//! nutrient is not scarce, specialists, not mixotrophs, hold the world's
//! living energy. Its setup is `scenarios/example12_generalist_dominance.json`
//! and its reading rule is declared in the end state; this file implements
//! the rule.

use explorers_genesis_eval::{EvalConfig, generalist_energy_share};
use explorers_sim::{TraitVector, World, WorldRecipe};

/// One seed's read at the end of its run.
#[derive(Clone, Copy, Debug, PartialEq)]
enum SeedRead {
    /// Living energy remains, and mixotrophs hold at most the dominance
    /// fraction of it.
    Holds,
    /// Mixotrophs hold more than the dominance fraction: generalist dominance.
    Fails,
    /// No living energy: the seed says nothing about who holds it.
    Void,
}

/// The panel's reading under the rule.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Reading {
    Supported,
    Refuted,
    Undecided,
}

/// Supported when the read holds on more than half the panel's seeds, refuted
/// when it fails on more than half, undecided otherwise. A void seed counts
/// toward neither.
fn read_panel(seeds: &[SeedRead]) -> Reading {
    let count = |read: SeedRead| seeds.iter().filter(|&&s| s == read).count();
    if 2 * count(SeedRead::Holds) > seeds.len() {
        Reading::Supported
    } else if 2 * count(SeedRead::Fails) > seeds.len() {
        Reading::Refuted
    } else {
        Reading::Undecided
    }
}

/// The end-of-run roster's read: mixotrophs by investment are the agents whose
/// own trophic coordinates both exceed the evaluator's generalist threshold,
/// and they fail the seed when they hold more than its dominance fraction of
/// living energy (the generalist-dominance failure mode, read per agent).
fn read_seed(traits: &[TraitVector], energies: &[f32], config: &EvalConfig) -> SeedRead {
    if energies.iter().sum::<f32>() <= 0.0 {
        return SeedRead::Void;
    }
    let share = generalist_energy_share(traits, energies, config.generalist_threshold);
    if share > config.generalist_dominance_fraction {
        SeedRead::Fails
    } else {
        SeedRead::Holds
    }
}

const SETUP: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../scenarios/example12_generalist_dominance.json"
);

/// The panel the rule declares: seeds 1 to 8.
const PANEL: std::ops::RangeInclusive<u64> = 1..=8;

fn load_setup() -> WorldRecipe {
    let contents = std::fs::read_to_string(SETUP).expect("read the Sp setup");
    serde_json::from_str(&contents).expect("parse the Sp setup")
}

/// Run one seed of the setup to its `max_ticks`, stopping early only on
/// extinction, and read the end-of-run roster.
fn run_seed(recipe: &WorldRecipe, seed: u64, config: &EvalConfig) -> SeedRead {
    let mut world = World::from_recipe(recipe, seed);
    for _ in 0..recipe.max_ticks {
        world.step();
        if world.agents().is_empty() {
            break;
        }
    }
    let traits: Vec<TraitVector> = world.agents().iter().map(|a| a.traits).collect();
    let energies: Vec<f32> = world.agents().iter().map(|a| a.energy()).collect();
    read_seed(&traits, &energies, config)
}

/// The reading itself: the setup over the declared panel, read by the rule.
/// Ignored because it is a long run (eight seeds of 2000 ticks); run it by
/// hand and record the result in the register:
///   cargo test --release -p explorers-genesis-eval --test specialists_hold_living_energy -- --ignored --nocapture
#[test]
#[ignore]
fn reading_sp_on_its_setup() {
    let recipe = load_setup();
    let config = EvalConfig::default();
    let seeds: Vec<SeedRead> = PANEL
        .map(|seed| {
            let read = run_seed(&recipe, seed, &config);
            println!("seed {seed}: {read:?}");
            read
        })
        .collect();
    let reading = read_panel(&seeds);
    println!("Sp: {reading:?}");
    assert_ne!(reading, Reading::Refuted, "Sp is refuted on its setup");
}

/// The setup holds the case the hypothesis reads: specialists and
/// mixotrophs by investment, both seeded, so neither side wins by absence.
#[test]
fn the_setup_seeds_specialists_and_mixotrophs() {
    let recipe = load_setup();
    let config = EvalConfig::default();
    let roster = recipe.agents.expect("the setup seeds a fixed roster");
    let traits: Vec<TraitVector> = roster.iter().map(|a| a.traits).collect();
    let ones = vec![1.0; traits.len()];
    let share = generalist_energy_share(&traits, &ones, config.generalist_threshold);
    assert!(
        share > 0.0 && share < 1.0,
        "by head count, mixotrophs are {share} of the seeded roster"
    );
}

fn traits(autotrophy: f32, heterotrophy: f32) -> TraitVector {
    TraitVector {
        photosynthetic_absorption: autotrophy,
        heterotrophy,
        mobility: 0.0,
        kappa: 0.5,
        fecundity: 0.2,
        asexual_propensity: 0.2,
        dispersal: 0.2,
    }
}

#[test]
fn a_seed_whose_specialists_hold_the_energy_holds() {
    let config = EvalConfig::default();
    let roster = [traits(0.5, 0.0), traits(0.0, 0.5), traits(0.5, 0.5)];
    assert_eq!(
        read_seed(&roster, &[40.0, 30.0, 30.0], &config),
        SeedRead::Holds
    );
}

#[test]
fn a_seed_whose_mixotrophs_hold_most_of_the_energy_fails() {
    let config = EvalConfig::default();
    let roster = [traits(0.5, 0.0), traits(0.5, 0.5)];
    assert_eq!(read_seed(&roster, &[40.0, 60.0], &config), SeedRead::Fails);
}

#[test]
fn a_seed_with_no_living_energy_is_void() {
    let config = EvalConfig::default();
    assert_eq!(read_seed(&[], &[], &config), SeedRead::Void);
}

#[test]
fn a_panel_holding_on_most_seeds_is_supported() {
    use SeedRead::*;
    let seeds = [Holds, Holds, Holds, Holds, Holds, Fails, Fails, Void];
    assert_eq!(read_panel(&seeds), Reading::Supported);
}

#[test]
fn a_panel_failing_on_most_seeds_is_refuted() {
    use SeedRead::*;
    let seeds = [Fails, Fails, Fails, Fails, Fails, Holds, Holds, Holds];
    assert_eq!(read_panel(&seeds), Reading::Refuted);
}

#[test]
fn a_panel_with_no_majority_either_way_is_undecided() {
    use SeedRead::*;
    let seeds = [Holds, Holds, Holds, Holds, Fails, Fails, Fails, Void];
    assert_eq!(read_panel(&seeds), Reading::Undecided);
}
