//! Property-based comparative statics over random parameterisations (#436,
//! Workstream C3): single-tick monotonicity properties the world rules commit
//! to, checked against the real phase functions across the search space,
//! reusing the C1 harness. Each property perturbs exactly one parameter (or
//! one stock) of a cloned world, runs one phase, and compares per-agent
//! quantities by id.
//!
//! A failing property is a design/implementation disagreement: it is filed as
//! a bug and kept (ignored with a reference), never weakened.

mod support;

use explorers_sim::spatial::SpatialGrid;
use explorers_sim::{Agent, Carcass, World, WorldParameters, phase};
use proptest::prelude::*;
use std::collections::HashMap;
use support::{WorldCase, world_case};

/// Multiplicative perturbation applied to the parameter under test. Bounded
/// away from 1 so the comparison is never a no-op, and modest so a case stays
/// inside the neighbourhood the search explores.
fn factor() -> impl Strategy<Value = f32> {
    1.05f32..=3.0
}

/// A world advanced `ticks − 1` steps, so the phase under test sees a
/// mid-trajectory population (wear, grown structure, drained targets) rather
/// than only tick-0 founders.
fn advanced_world(case: &WorldCase) -> World {
    let mut world = World::new(case.params.clone(), case.dist.clone(), case.seed);
    for _ in 0..case.ticks.saturating_sub(1) {
        world.step();
    }
    world
}

/// The spatial grid exactly as `World::step` builds it: keyed by slice index,
/// cell size from the light-competition radius.
fn index_grid(agents: &[Agent], params: &WorldParameters) -> SpatialGrid {
    let mut grid = SpatialGrid::new(
        params.world_extent,
        params.light_competition_radius.max(1.0),
    );
    for (i, a) in agents.iter().enumerate() {
        grid.insert(i as u64, a.position);
    }
    grid
}

/// `lower[id] ≤ upper[id]` for every agent id, with `what` naming the compared
/// quantity and `change` the perturbation, for the failure message.
fn assert_pointwise_le(
    lower: &HashMap<u64, f32>,
    upper: &HashMap<u64, f32>,
    what: &str,
    change: &str,
) -> Result<(), TestCaseError> {
    for (id, lo) in lower {
        let hi = upper[id];
        prop_assert!(
            *lo <= hi,
            "agent {id}: {what} is {lo} but {hi} after {change} — the world rule commits the opposite order"
        );
    }
    Ok(())
}

fn by_id<T>(agents: &[Agent], f: impl Fn(&Agent) -> T) -> HashMap<u64, T> {
    agents.iter().map(|a| (a.id, f(a))).collect()
}

/// Per-agent income from one `photosynthesise` pass under `params`.
fn photosynthetic_income(world: &World, params: &WorldParameters) -> HashMap<u64, f32> {
    let mut agents = world.agents().to_vec();
    let before = by_id(&agents, |a| a.reserve);
    let grid = index_grid(&agents, params);
    phase::photosynthesise(&mut agents, &grid, params);
    by_id(&agents, |a| a.reserve - before[&a.id])
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Input flows, flow 1 (Photosynthesis): producers absorb energy from the
    /// constant solar flux, attenuated only by light competition — a share of
    /// the flux, weighted by autotrophy and structure. The share is independent
    /// of the flux magnitude, so raising `solar_flux_magnitude` never lowers any
    /// producer's photosynthetic income that tick, all else equal.
    #[test]
    fn raising_solar_flux_never_lowers_photosynthetic_income(
        case in world_case(),
        factor in factor(),
    ) {
        let world = advanced_world(&case);
        let base = world.params().clone();
        let mut raised = base.clone();
        raised.solar_flux_magnitude *= factor;

        let low = photosynthetic_income(&world, &base);
        let high = photosynthetic_income(&world, &raised);
        assert_pointwise_le(
            &low,
            &high,
            "photosynthetic income",
            &format!("solar flux ×{factor}"),
        )?;
    }
}

/// Per-agent reserve after one `metabolise` pass under `params`.
fn reserve_after_metabolise(world: &World, params: &WorldParameters) -> HashMap<u64, f32> {
    let mut agents = world.agents().to_vec();
    phase::metabolise(&mut agents, params);
    by_id(&agents, |a| a.reserve)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Dissipation, flow 8 (Metabolism): every living agent pays a base rate —
    /// "the minimum cost of being alive, independent of traits or activity" —
    /// from reserve to heat. The base rate is additive to the trait and
    /// structure terms, so raising `base_metabolic_rate` never raises any
    /// agent's reserve after the metabolise phase, all else equal.
    #[test]
    fn raising_base_metabolic_rate_never_raises_reserve(
        case in world_case(),
        factor in factor(),
    ) {
        let world = advanced_world(&case);
        let base = world.params().clone();
        let mut raised = base.clone();
        raised.base_metabolic_rate *= factor;

        let low = reserve_after_metabolise(&world, &base);
        let high = reserve_after_metabolise(&world, &raised);
        assert_pointwise_le(
            &high,
            &low,
            "reserve after metabolise",
            &format!("base metabolic rate ×{factor}"),
        )?;
    }
}

/// Per-consumer reserve gain from one `resolve_drains` pass under `params`
/// (living-target and carcass drains together), for every agent alive at the
/// start of the pass. Consumers the pass kills are recorded from the phase's
/// mutated slice too, since the death threshold check leaves the slice intact.
fn drain_energy_gain(world: &World, params: &WorldParameters) -> HashMap<u64, f32> {
    let mut agents = world.agents().to_vec();
    // `Carcass` is not `Clone`; copy its public fields.
    let mut carcasses: Vec<Carcass> = world
        .carcasses()
        .iter()
        .map(|c| Carcass {
            id: c.id,
            position: c.position,
            energy: c.energy,
            nutrient: c.nutrient,
            traits: c.traits,
        })
        .collect();
    let mut nutrient_grid = world.nutrient_grid().clone();
    let before = by_id(&agents, |a| a.reserve);
    let grid = index_grid(&agents, params);
    phase::resolve_drains(
        &mut agents,
        &mut carcasses,
        &grid,
        params,
        &mut nutrient_grid,
    );
    by_id(&agents, |a| a.reserve - before[&a.id])
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Dissipation, flow 7 (Trophic transfer loss): a consumer retains
    /// `base_trophic_efficiency · exp(−trophic_distance_decay · d)` of the
    /// structure it drains, `d ≥ 0` the trait-space distance, so the decay
    /// rate only ever lowers efficiency. The drain itself (flow 3) is the
    /// consumer's effective heterotrophy, independent of the decay, so raising
    /// `trophic_distance_decay` never raises any consumer's energy gain from
    /// consumption that tick, all else equal.
    #[test]
    fn raising_trophic_distance_decay_never_raises_consumption_gain(
        case in world_case(),
        factor in factor(),
    ) {
        let world = advanced_world(&case);
        let base = world.params().clone();
        let mut raised = base.clone();
        raised.trophic_distance_decay *= factor;

        let low = drain_energy_gain(&world, &base);
        let high = drain_energy_gain(&world, &raised);
        assert_pointwise_le(
            &high,
            &low,
            "consumption energy gain",
            &format!("trophic distance decay ×{factor}"),
        )?;
    }
}

/// Per-agent maintenance charge (reserve paid to heat) from one `metabolise`
/// pass under `params`.
fn maintenance_charge(world: &World, params: &WorldParameters) -> HashMap<u64, f32> {
    let mut agents = world.agents().to_vec();
    let before = by_id(&agents, |a| a.reserve);
    phase::metabolise(&mut agents, params);
    by_id(&agents, |a| before[&a.id] - a.reserve)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Cost structure, trade-off 1 (Acquire vs. maintain) / flow 8: each
    /// capability costs energy to maintain whether or not it is in use, as
    /// `coefficient · trait^exponent`, and the per-trait terms are independent
    /// and additive. Raising `photo_maintenance_cost` therefore never lowers
    /// any agent's maintenance charge that tick, all else equal. Runs over the
    /// full exponent range: founders are floored into the non-negative trait
    /// domain (#444), so no per-trait term can be negative.
    #[test]
    fn raising_photo_maintenance_cost_never_lowers_maintenance_charge(
        case in world_case(),
        factor in factor(),
    ) {
        let world = advanced_world(&case);
        let base = world.params().clone();
        let mut raised = base.clone();
        raised.photo_maintenance_cost *= factor;

        let low = maintenance_charge(&world, &base);
        let high = maintenance_charge(&world, &raised);
        assert_pointwise_le(
            &low,
            &high,
            "maintenance charge",
            &format!("photo maintenance cost ×{factor}"),
        )?;
    }
}

/// Per-agent nutrient uptake from one `absorb_nutrients` pass, with the pool
/// at the cell of the agent at `focus` (a fraction of the slice) first set to
/// `pool`. Uptake is the growth of the free store plus the reproductive
/// earmark, the two accounts the kappa split credits.
fn nutrient_uptake(world: &World, focus: f32, pool: f32) -> HashMap<u64, f32> {
    let mut agents = world.agents().to_vec();
    let mut nutrient_grid = world.nutrient_grid().clone();
    let idx = ((agents.len() - 1) as f32 * focus).round() as usize;
    let cell = nutrient_grid.cell_index_for(agents[idx].position);
    *nutrient_grid.cell_mut(cell) = pool;
    let before = by_id(&agents, |a| a.nutrient + a.repro_nutrient);
    phase::absorb_nutrients(&mut agents, &mut nutrient_grid, world.params());
    by_id(&agents, |a| a.nutrient + a.repro_nutrient - before[&a.id])
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Input flows, flow 2 (Nutrient uptake): agents extract nutrient from the
    /// available pool at their location, and co-located agents share that pool
    /// proportionally by effective uptake rate when demand exceeds supply. A
    /// smaller pool can only bind harder, so reducing the available pool at a
    /// location never raises any agent's uptake that tick, all else equal.
    /// The pool is set to a level spanning scarcity to abundance (per-agent
    /// demand is at most the autotrophy trait, ~1) so the proportional-split
    /// branch is exercised, not only the demand-met branch.
    #[test]
    fn reducing_available_pool_never_raises_uptake(
        case in world_case(),
        focus in 0.0f32..=1.0,
        pool in 0.0f32..=4.0,
        reduction in 0.0f32..=0.95,
    ) {
        let world = advanced_world(&case);
        prop_assume!(!world.agents().is_empty());

        let high = nutrient_uptake(&world, focus, pool);
        let low = nutrient_uptake(&world, focus, pool * reduction);
        assert_pointwise_le(
            &low,
            &high,
            "nutrient uptake",
            &format!("pool {pool} → {}", pool * reduction),
        )?;
    }
}

/// Per-agent structure built by one `grow` pass under `params`.
fn structure_growth(world: &World, params: &WorldParameters) -> HashMap<u64, f32> {
    let mut agents = world.agents().to_vec();
    let before = by_id(&agents, |a| a.structure);
    phase::grow(&mut agents, params);
    by_id(&agents, |a| a.structure - before[&a.id])
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Dissipation, flow 9 (Growth): only reserve above the retention buffer
    /// `metabolic_cost × growth_retention_multiplier` is mobilisable, and the
    /// kappa share of the mobilised flow funds repair then growth. A larger
    /// buffer mobilises less, so raising `growth_retention_multiplier` never
    /// raises any agent's structure growth that tick, all else equal. Runs over
    /// the full exponent range: founders are floored into the non-negative
    /// trait domain (#444), so the metabolic cost is never negative.
    #[test]
    fn raising_retention_multiplier_never_raises_structure_growth(
        case in world_case(),
        factor in factor(),
    ) {
        let world = advanced_world(&case);
        let base = world.params().clone();
        let mut raised = base.clone();
        raised.growth_retention_multiplier *= factor;

        let low = structure_growth(&world, &base);
        let high = structure_growth(&world, &raised);
        assert_pointwise_le(
            &high,
            &low,
            "structure growth",
            &format!("retention multiplier ×{factor}"),
        )?;
    }
}
