//! Hypothesis G, the decomposer guild, rung 0: budget arithmetic (#856;
//! `docs/system-design/end-state.md`, *The decomposer guild*, *Its ladder*).
//!
//! A lone decomposer founder's income at the domain's founding litter ratio,
//! against its upkeep. Refuted if income falls below upkeep: no reading on the
//! rungs above could then pass for the domain's reason. It is a necessary
//! condition, so it can refute the guild but never support it.
//!
//! Income and upkeep are the stepper's, not a hand-copied formula: the founder
//! is the committed mesocosm's ([`Mode1Spec::committed`]), its upkeep is what
//! the stepper charges it under the decomposer mesocosm's rules
//! ([`DecomposerMesocosm::apply`]), and its income is
//! what one tick of the stepper pays it from the litter in its reach. So the
//! test fails when the physics moves the budget.
//!
//! Upkeep is the metabolic cost the stepper charges plus movement at a full
//! stride, the design's founding upkeep (reference-modes.md, mode 1, *Founder
//! density*): the founder is not credited with standing still. The litter is
//! the ratio exactly, the last carcass holding the remainder, because the
//! budget sits close to break-even (about 169 in *Decomposer founders*, *The
//! stepper's budget agrees*) and a whole carcass is about 1.3 of ratio.

use explorers_reference_modes::mesocosm::{
    CARCASS_ENERGY_PER_CELL, CARCASS_NUTRIENT_PER_CELL, CARCASSES_PER_CELL, DecomposerMesocosm,
    MODE1_DECOMPOSER_CLUSTER, Mode1Spec,
};
use explorers_sim::event::EventKind;
use explorers_sim::{AgentSpec, CarcassSpec, World, WorldRecipe, trophic_transfer_efficiency};

/// The decomposer mesocosm's rules and the domain's founding litter ratio,
/// as the mesocosm crate declares them (#884): the rung reads at the ratio's
/// lower end, the leanest litter a founder meets in the domain.
const MESOCOSM: DecomposerMesocosm = DecomposerMesocosm::DECLARED;

const SEED: u64 = 1;

/// A world of the committed mesocosm's parameters holding one decomposer
/// founder, at the spec's heterotroph vertex and founder reserve, at the
/// origin, and `litter` carcasses of the mesocosm's producer litter, all
/// within its contact reach.
fn lone_founder(spec: &Mode1Spec, litter: Vec<CarcassSpec>) -> World {
    let mut params = spec.parameters.clone();
    params.initial_population_size = 1;
    let mut world = World::from_recipe(
        &WorldRecipe {
            parameters: params,
            initial_distribution: None,
            agents: Some(vec![AgentSpec {
                position: (0.0, 0.0),
                reserve: spec.founder_reserve,
                traits: spec.decomposer,
                nutrient: 0.0,
                founding_cluster: MODE1_DECOMPOSER_CLUSTER,
            }]),
            carcasses: Some(litter),
            max_ticks: spec.max_ticks,
        },
        SEED,
    );
    // As the mesocosm does: the litter's nutrient comes out of the pool.
    let drawn: Vec<_> = (world.carcasses().iter())
        .map(|c| (c.position, c.nutrient))
        .collect();
    for (position, nutrient) in drawn {
        world.nutrient_grid_mut().draw_nearest(position, nutrient);
    }
    world
}

/// The mesocosm's standing litter holding `structure` in all: carcasses of
/// the mesocosm's size, the last holding the remainder, laid on a disc of
/// radius 0.5 about the origin, inside a founder's contact-only reach.
fn litter_holding(spec: &Mode1Spec, structure: f32) -> Vec<CarcassSpec> {
    let size = CARCASS_ENERGY_PER_CELL / CARCASSES_PER_CELL as f32;
    let nutrient_per_energy = CARCASS_NUTRIENT_PER_CELL / CARCASS_ENERGY_PER_CELL;
    let whole = (structure / size).floor() as usize;
    let remainder = structure - whole as f32 * size;
    let sizes = std::iter::repeat_n(size, whole).chain((remainder > 0.0).then_some(remainder));
    let sizes: Vec<f32> = sizes.collect();
    let n = sizes.len() as f32;
    sizes
        .iter()
        .enumerate()
        .map(|(i, &energy)| {
            let (r, theta) = (0.5 * ((i as f32 + 0.5) / n).sqrt(), i as f32 * 2.399_963);
            CarcassSpec {
                position: (r * theta.cos(), r * theta.sin()),
                energy,
                traits: spec.producer,
                nutrient: energy * nutrient_per_energy,
            }
        })
        .collect()
}

#[test]
fn g_rung_0_lone_founder_income_covers_its_upkeep_at_the_founding_litter_ratio() {
    let spec = Mode1Spec::committed();
    let founder_structure = lone_founder(&spec, Vec::new()).agents()[0].structure;
    let litter = MESOCOSM.founding_litter_ratio * founder_structure;
    let mut world = lone_founder(&spec, litter_holding(&spec, litter));

    // The decomposer mesocosm's rules, on the world at founding.
    let upkeep = (MESOCOSM.apply(&mut world))
        .expect("the world has its founder")
        .upkeep
        .decomposer_founder
        .expect("the world has its founder")
        .upkeep;

    let founder = world.agents()[0].clone();
    assert_eq!(founder.structure, founder_structure);
    let reach = explorers_sim::phase::consumption_reach(
        founder.effective_trait_with_steepness(1, world.params().wear_degradation_steepness),
        founder.structure,
        world.params(),
    );
    assert!(
        world.carcasses().iter().all(|c| {
            explorers_sim::toroidal_distance(
                c.position,
                founder.position,
                world.params().world_extent,
            ) <= reach
        }),
        "every carcass is in the founder's reach"
    );
    let standing: f32 = world.carcasses().iter().map(|c| c.energy).sum();
    let carcasses = world.carcasses().to_vec();

    // One tick of the stepper: income is what it assimilates from the litter.
    world.step();
    let mut drained = 0.0_f32;
    let mut income = 0.0_f32;
    for e in world.event_log().by_agent(founder.id) {
        if e.kind == EventKind::Consumed && e.source == founder.id && e.target_was_carcass {
            let carcass = (carcasses.iter())
                .find(|c| Some(c.id) == e.target)
                .expect("a founding carcass");
            drained += e.energy_delta;
            income += e.energy_delta
                * trophic_transfer_efficiency(&founder.traits, &carcass.traits, world.params());
        }
    }

    eprintln!(
        "G rung 0: litter ratio {:.2} ({} carcasses, {standing:.3} structure on a founder of \
         {:.4}); drained {drained:.5}, income {income:.5}; upkeep {:.5} (metabolic {:.5} + \
         movement at full stride {:.5}); income / upkeep {:.4}",
        standing / founder.structure,
        carcasses.len(),
        founder.structure,
        upkeep.total,
        upkeep.metabolic,
        upkeep.movement,
        income / upkeep.total,
    );
    assert!((standing / founder.structure - MESOCOSM.founding_litter_ratio).abs() < 0.01);
    assert!(
        income >= upkeep.total,
        "G rung 0 refuted: a lone founder's income {income} is below its upkeep {}",
        upkeep.total
    );
}
