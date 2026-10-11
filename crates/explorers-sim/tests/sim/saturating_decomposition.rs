//! Saturating carcass decomposition (#803; world-rules.md, *Consumption is
//! incremental → Carcass decomposition saturates in its decomposers*).
//!
//! Per carcass, per tick, the structure drained is
//! `D = k_d · S · Σc / (K + Σc)`, each drainer taking `c / Σc` of it, `c` its
//! drain before the cap (`eff_heterotrophy × u_H`). A drained carcass below
//! `CARCASS_REMNANT_FLOOR_FRACTION` of its structure at death leaves the
//! world. Living targets are untouched, and `k_d = 0` is today's additive
//! drain.

use crate::support;

use explorers_sim::phase::{DrainResult, resolve_drains};
use explorers_sim::spatial::{NutrientGrid, SpatialGrid};
use explorers_sim::units::HETEROTROPHY_STRUCTURE_DRAIN_PER_TICK;
use explorers_sim::{
    Agent, CARCASS_REMNANT_FLOOR_FRACTION, Carcass, TraitVector, World, WorldParameters,
    WorldRecipe,
};

const EXTENT: f32 = 100.0;
const SITE: (f32, f32) = (50.0, 50.0);

fn params(k_d: f32, half_saturation: f32) -> WorldParameters {
    WorldParameters {
        world_extent: EXTENT,
        carcass_decay_constant: k_d,
        carcass_half_saturation: half_saturation,
        ..support::viable_baseline()
    }
}

fn traits(heterotrophy: f32) -> TraitVector {
    TraitVector {
        photosynthetic_absorption: 0.0,
        heterotrophy,
        mobility: 0.0,
        kappa: 0.5,
        fecundity: 0.0,
        asexual_propensity: 0.0,
        dispersal: 0.0,
    }
}

fn drainer(id: u64, heterotrophy: f32) -> Agent {
    Agent {
        id,
        position: SITE,
        reserve: 100.0,
        structure: 5.0,
        peak_structure: 5.0,
        nutrient: 5.0,
        traits: traits(heterotrophy),
        wear: Default::default(),
        repro_reserve: 0.0,
        repro_nutrient: 0.0,
    }
}

fn carcass(structure: f32, nutrient: f32) -> Carcass {
    Carcass {
        id: 999,
        position: SITE,
        energy: structure,
        energy_at_death: structure,
        nutrient,
        traits: TraitVector {
            photosynthetic_absorption: 0.5,
            ..traits(0.0)
        },
    }
}

/// One drain pass over `agents` and `carcasses`, all at `SITE`.
fn drain(
    agents: &mut [Agent],
    carcasses: &mut [Carcass],
    params: &WorldParameters,
    nutrient_grid: &mut NutrientGrid,
) -> DrainResult {
    let mut grid = SpatialGrid::new(EXTENT, 10.0);
    for (i, a) in agents.iter().enumerate() {
        grid.insert(i as u64, a.position);
    }
    resolve_drains(agents, carcasses, &grid, params, nutrient_grid)
}

/// Structure each agent drained from carcass 999 this pass, by agent id.
fn carcass_drains(result: &DrainResult) -> Vec<(u64, f32)> {
    let mut drains: Vec<(u64, f32)> = result
        .events
        .iter()
        .filter(|e| e.target_was_carcass && e.target == Some(999))
        .map(|e| (e.source, e.energy_delta))
        .collect();
    drains.sort_by_key(|&(id, _)| id);
    drains
}

/// Total structure `n` unit-heterotrophy drainers take from one fresh carcass.
fn yield_with(n: u64, params: &WorldParameters, structure: f32) -> f32 {
    let mut agents: Vec<Agent> = (0..n).map(|i| drainer(i, 1.0)).collect();
    let mut carcasses = vec![carcass(structure, 0.0)];
    let mut nutrient_grid = NutrientGrid::new(EXTENT, 10.0, 0.0);
    let result = drain(&mut agents, &mut carcasses, params, &mut nutrient_grid);
    carcass_drains(&result).iter().map(|&(_, d)| d).sum()
}

fn capability(agent: &Agent, params: &WorldParameters) -> f32 {
    agent.effective_trait_with_steepness(1, params.wear_degradation_steepness)
        * HETEROTROPHY_STRUCTURE_DRAIN_PER_TICK
}

#[test]
fn a_carcass_yield_rises_with_each_drainer_and_saturates_toward_k_d_s() {
    let (k_d, half, s) = (0.5, 1.0, 100.0);
    let params = params(k_d, half);
    let c = capability(&drainer(0, 1.0), &params);
    let mut previous = 0.0;
    for n in 1..=8u64 {
        let total = yield_with(n, &params, s);
        let sum_c = n as f32 * c;
        let expected = k_d * s * sum_c / (half + sum_c);
        assert!(
            (total - expected).abs() <= 1e-4 * expected,
            "{n} drainers: yield {total}, expected k_d·S·Σc/(K+Σc) = {expected}"
        );
        assert!(
            total > previous,
            "{n} drainers yield {total}, no more than {} drainers' {previous}",
            n - 1
        );
        assert!(total < k_d * s, "the yield never reaches k_d·S: {total}");
        previous = total;
    }
    let crowded = yield_with(200, &params, s);
    assert!(
        (k_d * s - crowded) / (k_d * s) < 0.01,
        "a crowded carcass yields close to k_d·S = {}, got {crowded}",
        k_d * s
    );
}

#[test]
fn each_drainer_takes_its_capability_share_of_the_yield() {
    let (k_d, half, s) = (0.4, 2.0, 60.0);
    let params = params(k_d, half);
    let mut agents = vec![drainer(3, 0.5), drainer(1, 1.0), drainer(2, 1.5)];
    let caps: Vec<(u64, f32)> = agents
        .iter()
        .map(|a| (a.id, capability(a, &params)))
        .collect();
    let sum_c: f32 = caps.iter().map(|&(_, c)| c).sum();
    let total = k_d * s * sum_c / (half + sum_c);
    let mut carcasses = vec![carcass(s, 0.0)];
    let mut nutrient_grid = NutrientGrid::new(EXTENT, 10.0, 0.0);
    let result = drain(&mut agents, &mut carcasses, &params, &mut nutrient_grid);
    let drains = carcass_drains(&result);
    assert_eq!(drains.len(), 3);
    for (id, d) in drains {
        let c = caps.iter().find(|&&(i, _)| i == id).unwrap().1;
        let share = total * c / sum_c;
        assert!(
            (d - share).abs() <= 1e-5 * total,
            "drainer {id} took {d}, expected D·c/Σc = {share}"
        );
    }
    assert!((carcasses[0].energy - (s - total)).abs() <= 1e-4);
}

#[test]
fn a_lone_carcass_drained_steadily_decays_geometrically() {
    let (k_d, half, s) = (0.3, 1.0, 50.0);
    let params = params(k_d, half);
    let mut agents = vec![drainer(0, 1.0)];
    let c = capability(&agents[0], &params);
    let ratio = 1.0 - k_d * c / (half + c);
    let mut carcasses = vec![carcass(s, 4.0)];
    let mut nutrient_grid = NutrientGrid::new(EXTENT, 10.0, 0.0);
    for tick in 0..12 {
        let before = carcasses[0].energy;
        drain(&mut agents, &mut carcasses, &params, &mut nutrient_grid);
        let after = carcasses[0].energy;
        assert!(
            (after / before - ratio).abs() < 1e-5,
            "tick {tick}: S fell by {}, expected the constant ratio {ratio}",
            after / before
        );
    }
    let expected = s * ratio.powi(12);
    assert!((carcasses[0].energy - expected).abs() <= 1e-4 * s);
}

#[test]
fn a_drain_on_a_living_target_is_unchanged_by_the_parameters() {
    let run = |params: &WorldParameters| {
        let mut target = drainer(7, 0.0);
        target.traits.photosynthetic_absorption = 1.0;
        target.structure = 3.0;
        let mut agents = vec![drainer(1, 1.2), drainer(2, 0.7), target];
        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut nutrient_grid = NutrientGrid::new(EXTENT, 10.0, 0.0);
        let result = drain(&mut agents, &mut carcasses, params, &mut nutrient_grid);
        let state: Vec<(u64, u32, u32, u32)> = agents
            .iter()
            .map(|a| {
                (
                    a.id,
                    a.reserve.to_bits(),
                    a.structure.to_bits(),
                    a.nutrient.to_bits(),
                )
            })
            .collect();
        (state, result.dissipated.to_bits(), result.events.len())
    };
    let off = run(&params(0.0, 1.0));
    let on = run(&params(0.6, 0.5));
    assert!(!off.0.is_empty());
    assert_eq!(
        off, on,
        "saturation must not touch drains on living targets"
    );
}

/// A world holding one sessile decomposer on one carcass and nothing else.
/// The decomposer neither grows, reproduces nor ages, so it outlives the
/// carcass and drains it every tick.
fn floor_world(params: WorldParameters) -> World {
    let mut world = World::from_recipe(
        &WorldRecipe {
            parameters: WorldParameters {
                reserve_mobilisation_rate: 0.0,
                reproduction_energy_threshold: f32::INFINITY,
                senescence_hazard: 0.0,
                ..params
            },
            agents: Some(vec![]),
            carcasses: None,
            max_ticks: 50,
            initial_distribution: None,
        },
        1,
    );
    world.add_agent(drainer(1, 1.0));
    world.add_carcass(carcass(40.0, 3.0));
    world
}

fn world_energy(world: &World) -> f32 {
    world.agents().iter().map(|a| a.energy()).sum::<f32>()
        + world.carcasses().iter().map(|c| c.energy).sum::<f32>()
}

fn world_nutrient(world: &World) -> f32 {
    world.nutrient_pool()
        + world
            .agents()
            .iter()
            .map(|a| a.nutrient_total(world.params()))
            .sum::<f32>()
        + world.carcasses().iter().map(|c| c.nutrient).sum::<f32>()
}

#[test]
fn a_carcass_leaves_the_world_at_the_floor_and_the_ledgers_balance() {
    // k_d · Σc / (K + Σc) ≈ 0.89 a tick: the carcass crosses 1e-6 of its
    // structure at death within ten ticks.
    let mut world = floor_world(params(0.9, 0.01));
    let energy_start = world_energy(&world);
    let nutrient_start = world_nutrient(&world);
    let mut removed_at = None;
    for tick in 0..20 {
        let before = world
            .carcasses()
            .iter()
            .find(|c| c.id == 999)
            .map(|c| c.energy);
        world.step();
        world.energy_ledger().assert_balanced();
        world.nutrient_ledger().assert_balanced();
        let after = world.carcasses().iter().find(|c| c.id == 999);
        match (before, after) {
            (Some(_), Some(c)) => assert!(
                c.energy >= CARCASS_REMNANT_FLOOR_FRACTION * c.energy_at_death,
                "tick {tick}: a carcass below the floor stands: {}",
                c.energy
            ),
            (Some(_), None) => {
                removed_at = Some(tick);
                break;
            }
            _ => unreachable!("carcass 999 vanished before the tick"),
        }
    }
    assert!(removed_at.is_some(), "the carcass never reached the floor");
    let energy_end = world_energy(&world) + world.dissipated_energy();
    let input = energy_start + world.total_solar_input();
    assert!(
        (energy_end - input).abs() <= 1e-4 * input,
        "energy: in {input}, out {energy_end}"
    );
    let nutrient_end = world_nutrient(&world);
    assert!(
        (nutrient_end - nutrient_start).abs() <= 1e-4 * nutrient_start.max(1.0),
        "nutrient: {nutrient_start} -> {nutrient_end}"
    );
}

#[test]
fn with_k_d_zero_a_spent_carcass_stands_as_today() {
    // Off: the additive drain empties the carcass and it stays in the
    // carcass list, empty, exactly as before #803.
    let mut world = floor_world(params(0.0, 0.01));
    for _ in 0..60 {
        world.step();
    }
    assert_eq!(world.agents().len(), 1, "the decomposer outlives the run");
    let c = world
        .carcasses()
        .iter()
        .find(|c| c.id == 999)
        .expect("the off path never removes a carcass");
    assert!(c.energy <= 0.0);
}
