//! The intake-ceiling window (issue #629).
//!
//! The intake gate (world rules, *Capability and expression are decoupled*)
//! reads each consumer once, at the start of drain resolution, against an
//! intake ceiling `k × metabolic_cost`. Before the stepper changes, this
//! module reads, on the ungated atlas, the two populations that bound `k`:
//! how much of a ceiling light already fills for light-fed mixotrophs (the
//! upper bound, `k ≤ 2 × p25`), and how much heterotrophs by diet take in
//! (the lower bound, `k ≥ p75`). [`IntakeReading`] is one agent's read,
//! [`intake_readings`] reads a tick from its [`PreStep`] and events, and
//! [`IntakeCensus`] pools the distributions and the [`Window`].
//!
//! Units: "ticks of maintenance" divides by the agent's drain-time metabolic
//! cost `m`. Nutrient is first converted to the energy it would match in
//! growth (`÷ η·ratio`), so both currencies share the yardstick.
//!
//! Observer-side only: the world is never touched.

use std::collections::HashMap;

use std::collections::HashSet;

use explorers_sim::event::{Event, EventKind};
use explorers_sim::spatial::SpatialGrid;
use explorers_sim::units::HETEROTROPHY_STRUCTURE_DRAIN_PER_TICK;
use explorers_sim::{Agent, Carcass, TraitVector, WorldParameters, phase};

use explorers_sim::topology::TrophicRole;

use crate::grazer_hunger::{PreStep, SurplusDistribution};
use crate::role_diet::DietGroup;

/// One agent's intake at the intake gate's read point: once per consumer at
/// the start of drain resolution, after photosynthesis and uptake, with
/// positions fixing what is in reach.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct IntakeReading {
    /// Per-tick metabolic cost `m` on the drain-time roster.
    pub maintenance: f32,
    /// Photosynthesis this tick.
    pub light: f32,
    /// Pool uptake this tick.
    pub uptake: f32,
    /// Drain income this tick, as energy received: the structure each
    /// `Consumed` event drained times the transfer efficiency.
    pub drained_energy: f32,
    /// The nutrient bound in that structure.
    pub drained_nutrient: f32,
    /// Drain potential `P_E`.
    pub potential_energy: f32,
    /// Drain potential `P_N`.
    pub potential_nutrient: f32,
    /// The nutrient growth binds per unit of energy, `η · ratio` (the
    /// consumer's stoichiometric demand per unit structure): nutrient ÷ this
    /// is the energy it would match. Zero when growth binds no nutrient.
    pub nutrient_per_energy: f32,
}

impl IntakeReading {
    /// Light in ticks of maintenance, `light / m`.
    pub fn light_ticks(&self) -> f32 {
        self.light / self.maintenance
    }

    /// Realised intake (light plus drain income) in ticks of maintenance.
    pub fn intake_ticks(&self) -> f32 {
        (self.light + self.drained_energy) / self.maintenance
    }

    /// Nutrient in ticks of maintenance through the energy it would match
    /// in growth, `n / (η·ratio) / m`; `None` without a nutrient side.
    fn nutrient_ticks(&self, n: f32) -> Option<f32> {
        (self.nutrient_per_energy > 0.0).then(|| n / self.nutrient_per_energy / self.maintenance)
    }

    /// Uptake in ticks of maintenance, nutrient-matched.
    pub fn uptake_ticks(&self) -> Option<f32> {
        self.nutrient_ticks(self.uptake)
    }

    /// Drain potential `P_N` in ticks of maintenance, nutrient-matched.
    pub fn potential_nutrient_ticks(&self) -> Option<f32> {
        self.nutrient_ticks(self.potential_nutrient)
    }

    /// Realised nutrient intake (uptake plus drained bound nutrient) in
    /// ticks of maintenance, nutrient-matched.
    pub fn intake_nutrient_ticks(&self) -> Option<f32> {
        self.nutrient_ticks(self.uptake + self.drained_nutrient)
    }

    /// The intake gate's predicted expression at ceiling multiple `k`: rooms
    /// `max(0, k·m − light)` and `max(0, k·m·η·ratio − uptake)` against the
    /// drain potentials ([`predicted_expression`]).
    pub fn expression_at(&self, k: f32) -> f32 {
        let ceiling = k * self.maintenance;
        let energy = ((ceiling - self.light).max(0.0), self.potential_energy);
        let nutrient = (self.nutrient_per_energy > 0.0).then(|| {
            (
                (ceiling * self.nutrient_per_energy - self.uptake).max(0.0),
                self.potential_nutrient,
            )
        });
        predicted_expression(energy, nutrient)
    }
}

/// Every drain-time agent's [`IntakeReading`] for the tick that `pre`
/// captured, with `events` the tick's events as the stepper logged them
/// (its `Consumed` events give the realised drains).
///
/// **Drain potential** mirrors the drain pass's reach and per-target demand
/// (`phase::resolve_drains_with_expression`) with expression 1 and no
/// recognition restraint (capability, before the gate and before
/// recognition): every living agent with structure and every carcass holding
/// energy or nutrient within `consumption_reach(h_eff, structure)` of the
/// consumer, drained at `h_eff · u_H`, each target taken as a lone feeder
/// would take it (capped at what it holds), before co-feeders split it.
/// - `P_E`, as energy: the structure taken times the trophic transfer
///   efficiency, the energy the consumer would receive.
/// - `P_N`: the nutrient bound in the structure taken (`taken × ratio` of a
///   living target; a carcass's nutrient in proportion to the energy taken,
///   all of it when the bite exhausts the carcass), before the consumer's
///   retention cap.
///
/// Realised drains are measured the same way from the tick's `Consumed`
/// events. A carcass already spent of energy hands its nutrient to its
/// consumers in proportion to demand; that is split here by effective
/// heterotrophy, exact when the gate is off (expression 1).
pub fn intake_readings(
    pre: &PreStep,
    params: &WorldParameters,
    events: &[Event],
) -> HashMap<u64, IntakeReading> {
    let start = pre.drain_start(params);
    let agents = &start.agents;
    let carcasses = pre.carcasses();
    let k = params.wear_degradation_steepness;
    let extent = params.world_extent;
    let eff_het = |a: &Agent| a.effective_trait_with_steepness(1, k);
    let ratio = |t: &TraitVector| explorers_sim::stoichiometric_demand(t, 1.0, params);

    let mut readings: HashMap<u64, IntakeReading> = agents
        .iter()
        .map(|a| {
            let reading = IntakeReading {
                maintenance: phase::metabolic_cost(a, params).max(0.0),
                light: start.light.get(&a.id).copied().unwrap_or(0.0),
                uptake: start.uptake.get(&a.id).copied().unwrap_or(0.0),
                nutrient_per_energy: params.growth_efficiency.max(0.0) * ratio(&a.traits),
                ..Default::default()
            };
            (a.id, reading)
        })
        .collect();

    // Drain potential.
    let cell_size = params.light_competition_radius.max(1.0);
    let mut grid = SpatialGrid::new(extent, cell_size);
    for (i, a) in agents.iter().enumerate() {
        grid.insert(i as u64, a.position);
    }
    for c in agents {
        let h = eff_het(c);
        if h <= 0.0 {
            continue;
        }
        let reach = phase::consumption_reach(h, c.structure, params);
        let capability = h * HETEROTROPHY_STRUCTURE_DRAIN_PER_TICK;
        let (mut p_e, mut p_n) = (0.0_f32, 0.0_f32);
        let mut seen = HashSet::new();
        for j in grid.query_radius(c.position, reach) {
            if !seen.insert(j) {
                continue;
            }
            let t = &agents[j as usize];
            if t.id == c.id
                || t.structure <= 0.0
                || explorers_sim::toroidal_distance(c.position, t.position, extent) > reach
            {
                continue;
            }
            let taken = capability.min(t.structure);
            p_e += taken * explorers_sim::trophic_transfer_efficiency(&c.traits, &t.traits, params);
            p_n += taken * ratio(&t.traits);
        }
        for t in carcasses {
            if (t.energy <= 0.0 && t.nutrient <= 0.0)
                || explorers_sim::toroidal_distance(c.position, t.position, extent) > reach
            {
                continue;
            }
            let available = t.energy.max(0.0);
            let taken = capability.min(available);
            p_e += taken * explorers_sim::trophic_transfer_efficiency(&c.traits, &t.traits, params);
            p_n += if capability >= available {
                t.nutrient
            } else {
                t.nutrient * taken / available
            };
        }
        let r = readings.get_mut(&c.id).expect("every agent has a reading");
        r.potential_energy = p_e;
        r.potential_nutrient = p_n;
    }

    // Realised drains.
    let living: HashMap<u64, &Agent> = agents.iter().map(|a| (a.id, a)).collect();
    let dead: HashMap<u64, &Carcass> = carcasses.iter().map(|c| (c.id, c)).collect();
    let consumed = || {
        events
            .iter()
            .filter(|e| e.kind == EventKind::Consumed)
            .filter_map(|e| Some((e, living.get(&e.source)?, e.target?)))
    };
    // Effective heterotrophy feeding on each spent carcass, for its split.
    let mut spent_demand: HashMap<u64, f32> = HashMap::new();
    for (e, c, target) in consumed() {
        if e.target_was_carcass && dead.get(&target).is_some_and(|t| t.energy <= 0.0) {
            *spent_demand.entry(target).or_default() += eff_het(c);
        }
    }
    for (e, c, target) in consumed() {
        let drained = e.energy_delta;
        let (target_traits, nutrient) = if e.target_was_carcass {
            let Some(t) = dead.get(&target) else { continue };
            let nutrient = if t.energy > 0.0 {
                t.nutrient * drained / t.energy
            } else {
                let total = spent_demand.get(&target).copied().unwrap_or(0.0);
                if total > 0.0 {
                    t.nutrient * eff_het(c) / total
                } else {
                    0.0
                }
            };
            (t.traits, nutrient)
        } else {
            let Some(t) = living.get(&target) else {
                continue;
            };
            (t.traits, drained * ratio(&t.traits))
        };
        let r = readings.get_mut(&c.id).expect("every agent has a reading");
        r.drained_energy +=
            drained * explorers_sim::trophic_transfer_efficiency(&c.traits, &target_traits, params);
        r.drained_nutrient += nutrient;
    }
    readings
}

/// The intake gate's expression (world rules, *Capability and expression are
/// decoupled*):
/// `E = max(room_E / (room_E + P_E), room_N / (room_N + P_N))`.
///
/// Each side is `(room, potential)`. `nutrient` is `None` where growth binds
/// no nutrient (`growth_efficiency = 0`): the gate reads energy alone. A side
/// with no room expresses nothing; one with room and nothing in reach
/// (`P = 0`) expresses fully.
pub fn predicted_expression(energy: (f32, f32), nutrient: Option<(f32, f32)>) -> f32 {
    let side = |(room, potential): (f32, f32)| {
        if room <= 0.0 {
            0.0
        } else {
            room / (room + potential.max(0.0))
        }
    };
    nutrient.map_or(side(energy), |n| side(energy).max(side(n)))
}

/// The intake-ceiling window's two populations over agent-samples (#629),
/// each read in both currencies in ticks of maintenance:
///
/// - **light-fed mixotrophs**: income-role producers (light at least half of
///   recent income, #599) with heterotrophy above zero; their light, and
///   their uptake nutrient-matched;
/// - **heterotrophs by diet**: the census's trait heterotrophs living on
///   drained energy ([`DietGroup::TraitHeterotrophDietFed`]); their realised
///   intake (light plus drain income), the same nutrient-matched (uptake plus
///   drained bound nutrient), and their drain potentials.
///
/// The nutrient columns are absent (not recorded) where growth binds no
/// nutrient.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IntakeCensus {
    pub mixotroph_light: SurplusDistribution,
    pub mixotroph_uptake: SurplusDistribution,
    pub heterotroph_intake: SurplusDistribution,
    pub heterotroph_intake_nutrient: SurplusDistribution,
    pub heterotroph_potential: SurplusDistribution,
    pub heterotroph_potential_nutrient: SurplusDistribution,
}

/// An intake-ceiling window `lower ≤ k ≤ upper`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
    /// `2 × p25` of the light-fed mixotrophs' supply.
    pub upper: f64,
    /// `p75` of the heterotrophs' intake.
    pub lower: f64,
}

impl Window {
    /// Whether some `k` serves both populations.
    pub fn is_open(&self) -> bool {
        self.lower <= self.upper
    }

    /// The proposed default `k`: the window's geometric midpoint; `None`
    /// when it is empty.
    pub fn default_k(&self) -> Option<f64> {
        self.is_open().then(|| (self.lower * self.upper).sqrt())
    }
}

impl IntakeCensus {
    pub fn record(
        &mut self,
        role: Option<TrophicRole>,
        heterotrophy: f32,
        group: DietGroup,
        reading: &IntakeReading,
    ) {
        if role == Some(TrophicRole::Producer) && heterotrophy > 0.0 {
            self.mixotroph_light.record(reading.light_ticks());
            if let Some(u) = reading.uptake_ticks() {
                self.mixotroph_uptake.record(u);
            }
        }
        if group == DietGroup::TraitHeterotrophDietFed {
            self.heterotroph_intake.record(reading.intake_ticks());
            self.heterotroph_potential
                .record(reading.potential_energy / reading.maintenance);
            if let Some(n) = reading.intake_nutrient_ticks() {
                self.heterotroph_intake_nutrient.record(n);
            }
            if let Some(p) = reading.potential_nutrient_ticks() {
                self.heterotroph_potential_nutrient.record(p);
            }
        }
    }

    /// The energy-side window: light / maintenance over light-fed
    /// mixotrophs against intake / maintenance over heterotrophs by diet.
    pub fn energy_window(&self) -> Option<Window> {
        window(&self.mixotroph_light, &self.heterotroph_intake)
    }

    /// The same on the nutrient side.
    pub fn nutrient_window(&self) -> Option<Window> {
        window(&self.mixotroph_uptake, &self.heterotroph_intake_nutrient)
    }

    pub fn merge(&mut self, other: &IntakeCensus) {
        self.mixotroph_light.merge(&other.mixotroph_light);
        self.mixotroph_uptake.merge(&other.mixotroph_uptake);
        self.heterotroph_intake.merge(&other.heterotroph_intake);
        self.heterotroph_intake_nutrient
            .merge(&other.heterotroph_intake_nutrient);
        self.heterotroph_potential
            .merge(&other.heterotroph_potential);
        self.heterotroph_potential_nutrient
            .merge(&other.heterotroph_potential_nutrient);
    }
}

/// A window as the census summary reads it: both bounds and the default
/// `k`, or "window EMPTY" first when no `k` serves both populations.
/// `supply` and `intake` name the two readings (e.g. "light", "intake").
pub fn window_line(w: Option<Window>, supply: &str, intake: &str) -> String {
    let Some(w) = w else {
        return "– (no samples)".to_string();
    };
    let bounds = format!(
        "upper ≤ {:.3} (2 × p25 {supply}/maintenance, light-fed mixotrophs), lower ≥ {:.3} (p75 {intake}/maintenance, heterotrophs by diet)",
        w.upper, w.lower
    );
    match w.default_k() {
        Some(k) => format!("{bounds}, default k = {k:.3} (geometric midpoint)"),
        None => format!("window EMPTY: {bounds}"),
    }
}

/// The window from a supply distribution (upper bound `2 × p25`) and an
/// intake distribution (lower bound `p75`); `None` when either is empty.
fn window(supply: &SurplusDistribution, intake: &SurplusDistribution) -> Option<Window> {
    Some(Window {
        upper: 2.0 * supply.percentile(0.25)?,
        lower: intake.percentile(0.75)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_source::{ConfigSource, resolve_config, sampled_units};
    use crate::search::default_ranges;
    use explorers_sim::{AgentSpec, CarcassSpec, World, WorldRecipe};

    fn traits(photo: f32, het: f32) -> TraitVector {
        TraitVector {
            photosynthetic_absorption: photo,
            heterotrophy: het,
            mobility: 0.0,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 0.5,
            dispersal: 0.3,
        }
    }

    /// `sample:31`'s physics with the gate off (`c = 0`, ungated drains).
    fn params() -> WorldParameters {
        let (mut params, _) = resolve_config(
            ConfigSource::SAMPLE,
            31,
            &Default::default(),
            &sampled_units(default_ranges().len()),
        );
        params.satiation_sensitivity = 0.0;
        params.recognition_distance = 0.0;
        params
    }

    fn spec(position: (f32, f32), reserve: f32, t: TraitVector) -> AgentSpec {
        AgentSpec {
            position,
            reserve,
            traits: t,
            nutrient: 1.0,
        }
    }

    /// A hand-built world stepped once: its readings and the tick's events.
    fn one_tick(
        params: &WorldParameters,
        agents: Vec<AgentSpec>,
        carcasses: Vec<CarcassSpec>,
    ) -> (HashMap<u64, IntakeReading>, Vec<Event>, World) {
        one_tick_then(params, agents, carcasses, |_| {})
    }

    /// [`one_tick`], with `tweak` applied to the world's parameters after
    /// its founders are provisioned.
    fn one_tick_then(
        params: &WorldParameters,
        agents: Vec<AgentSpec>,
        carcasses: Vec<CarcassSpec>,
        tweak: impl Fn(&mut WorldParameters),
    ) -> (HashMap<u64, IntakeReading>, Vec<Event>, World) {
        let recipe = WorldRecipe {
            parameters: params.clone(),
            initial_distribution: None,
            agents: Some(agents),
            carcasses: Some(carcasses),
            max_ticks: 10,
        };
        let mut world = World::from_recipe(&recipe, 7);
        tweak(world.params_mut());
        let params = world.params().clone();
        let pre = PreStep::capture(&world);
        let cursor = world.event_log().len();
        world.step();
        let events = world.event_log().since(cursor).to_vec();
        (intake_readings(&pre, &params, &events), events, world)
    }

    fn income(events: &[Event], id: u64, kind: EventKind) -> f32 {
        events
            .iter()
            .filter(|e| e.kind == kind && e.source == id)
            .map(|e| e.energy_delta)
            .sum()
    }

    /// A lone producer: its reading is this tick's photosynthesis in ticks
    /// of its drain-time maintenance, with nothing drained and nothing in
    /// reach.
    #[test]
    fn a_light_only_agent_reads_its_photosynthesis_in_ticks_of_maintenance() {
        let params = params();
        let (readings, events, world) = one_tick(
            &params,
            vec![spec((10.0, 10.0), 50.0, traits(0.8, 0.0))],
            vec![],
        );
        let r = readings[&0];
        let light = income(&events, 0, EventKind::Photosynthesized);
        assert!(light > 0.0);
        assert_eq!(r.light, light);
        let m = phase::metabolic_cost(&world.agents()[0], &params);
        assert!(close(r.maintenance, m), "{} vs {m}", r.maintenance);
        assert!(close(r.light_ticks(), light / m));
    }

    /// A pure consumer beside one large prey: what it drained this tick is
    /// the structure taken times the transfer efficiency, and, a lone feeder
    /// on a target that covers its capability, its potential is exactly what
    /// it took, in both currencies.
    #[test]
    fn a_drain_only_agent_reads_its_drain_income_and_matching_potential() {
        let params = params();
        let (readings, events, world) = one_tick(
            &params,
            vec![
                spec((10.0, 10.0), 50.0, traits(0.0, 1.0)),
                spec((10.01, 10.0), 400.0, traits(1.0, 0.0)),
            ],
            vec![],
        );
        let drained = income(&events, 0, EventKind::Consumed);
        assert!(drained > 0.0, "the consumer fed");
        let (c, t) = (traits(0.0, 1.0), traits(1.0, 0.0));
        let gained = drained * explorers_sim::trophic_transfer_efficiency(&c, &t, &params);
        let bound = drained * explorers_sim::stoichiometric_demand(&t, 1.0, &params);
        let r = readings[&0];
        assert_eq!(r.light, 0.0);
        assert!(close(r.drained_energy, gained), "{r:?} vs {gained}");
        assert!(close(r.drained_nutrient, bound), "{r:?} vs {bound}");
        assert!(close(r.potential_energy, gained), "{r:?}");
        assert!(close(r.potential_nutrient, bound), "{r:?}");
        let m = phase::metabolic_cost(&world.agents()[0], &params);
        assert!(close(r.intake_ticks(), gained / m));
        assert_eq!(
            readings[&1].potential_energy, 0.0,
            "a producer has no reach"
        );
    }

    /// A mixotroph beside a prey and a small carcass: its intake is its
    /// light plus both drains as energy received, and its potential counts
    /// the carcass it would exhaust whole (all of the carcass's nutrient).
    #[test]
    fn a_light_and_drain_agent_counts_every_route() {
        let params = params();
        let mixo = traits(0.5, 0.5);
        let prey = traits(1.0, 0.0);
        let dead = traits(0.2, 0.9);
        let (readings, events, world) = one_tick(
            &params,
            vec![
                spec((10.0, 10.0), 50.0, mixo),
                spec((10.01, 10.0), 400.0, prey),
            ],
            vec![CarcassSpec {
                position: (10.0, 10.01),
                energy: 1e-3,
                traits: dead,
                nutrient: 0.4,
            }],
        );
        let tte = |t: &TraitVector| explorers_sim::trophic_transfer_efficiency(&mixo, t, &params);
        let (mut gained, mut from_carcass) = (0.0, 0.0);
        for e in events
            .iter()
            .filter(|e| e.kind == EventKind::Consumed && e.source == 0)
        {
            if e.target_was_carcass {
                from_carcass += e.energy_delta;
                gained += e.energy_delta * tte(&dead);
            } else {
                gained += e.energy_delta * tte(&prey);
            }
        }
        assert!(close(from_carcass, 1e-3), "the carcass was exhausted");
        let light = income(&events, 0, EventKind::Photosynthesized);
        assert!(light > 0.0 && gained > 0.0);
        let r = readings[&0];
        let m = phase::metabolic_cost(&world.agents()[0], &params);
        assert!(close(r.intake_ticks(), (light + gained) / m), "{r:?}");
        assert!(close(r.potential_energy, gained), "{r:?} vs {gained}");
        let living_bound = (r.drained_nutrient - 0.4).max(0.0);
        assert!(living_bound > 0.0, "{r:?}");
        assert!(close(r.potential_nutrient, r.drained_nutrient), "{r:?}");
    }

    /// A well-lit mixotroph on a starved pool beside a prey: uptake and
    /// drained nutrient read in ticks of maintenance through the nutrient
    /// that would match the energy in growth, `N / (η·ratio) / m`. At a `k`
    /// light already fills, the energy room is full, but the nutrient room
    /// is not, so the hungrier nutrient side keeps it feeding.
    #[test]
    fn a_nutrient_limited_agent_is_kept_hungry_by_its_nutrient_room() {
        let mut params = params();
        params.initial_nutrient_pool = 1e-3;
        let mixo = traits(0.8, 0.3);
        let (readings, _, _) = one_tick(
            &params,
            vec![
                spec((10.0, 10.0), 50.0, mixo),
                spec((10.01, 10.0), 400.0, traits(1.0, 0.0)),
            ],
            vec![],
        );
        let r = readings[&0];
        let m = r.maintenance;
        let binding =
            params.growth_efficiency * explorers_sim::stoichiometric_demand(&mixo, 1.0, &params);
        assert!(binding > 0.0 && r.potential_nutrient > 0.0, "{r:?}");
        let uptake = r.uptake_ticks().unwrap();
        assert!(close(uptake, r.uptake / binding / m), "{r:?}");
        let intake = r.intake_nutrient_ticks().unwrap();
        assert!(
            close(intake, (r.uptake + r.drained_nutrient) / binding / m),
            "{r:?}"
        );
        let k = 0.5 * r.light_ticks();
        assert!(
            k > uptake,
            "light fills the ceiling, uptake does not: {r:?}"
        );
        let room_n = k * m * binding - r.uptake;
        let want = room_n / (room_n + r.potential_nutrient);
        let e = r.expression_at(k);
        assert!(e > 0.0 && close(e, want), "{e} vs {want}");
    }

    /// Where growth binds no nutrient there is no nutrient side to read:
    /// the gate reads energy alone.
    #[test]
    fn without_growth_efficiency_the_nutrient_side_is_absent() {
        let (readings, _, _) = one_tick_then(
            &params(),
            vec![
                spec((10.0, 10.0), 50.0, traits(0.5, 0.5)),
                spec((10.01, 10.0), 400.0, traits(1.0, 0.0)),
            ],
            vec![],
            |p| p.growth_efficiency = 0.0,
        );
        let r = readings[&0];
        assert_eq!(r.uptake_ticks(), None);
        assert_eq!(r.intake_nutrient_ticks(), None);
        assert!(r.potential_energy > 0.0 && r.light > 0.0, "{r:?}");
        let k = 2.0 * r.light_ticks();
        let room = k * r.maintenance - r.light;
        let want = room / (room + r.potential_energy);
        assert!(close(r.expression_at(k), want));
        assert_eq!(
            r.expression_at(0.5 * r.light_ticks()),
            0.0,
            "light fills it"
        );
    }

    /// On a running `sample:31` world with the gate and recognition off,
    /// what each agent drained never exceeds its potential (co-feeders only
    /// split it), and most consumers take exactly their potential: the
    /// reach and per-target demand mirror the drain pass.
    #[test]
    fn realised_drains_are_bounded_by_and_mostly_equal_the_potential() {
        let params = params();
        let (_, dist) = resolve_config(
            ConfigSource::SAMPLE,
            31,
            &Default::default(),
            &sampled_units(default_ranges().len()),
        );
        let mut world = World::new(params.clone(), dist, 1000);
        world.retain_event_kinds(&[EventKind::Consumed]);
        let (mut fed, mut equal) = (0, 0);
        for _ in 0..80 {
            let pre = PreStep::capture(&world);
            let cursor = world.event_log().len();
            world.step();
            let events = world.event_log().since(cursor).to_vec();
            for r in intake_readings(&pre, &params, &events).values() {
                let tol = |p: f32| p * 1e-5 + 1e-7;
                assert!(
                    r.drained_energy <= r.potential_energy + tol(r.potential_energy),
                    "{r:?}"
                );
                assert!(
                    r.drained_nutrient <= r.potential_nutrient + tol(r.potential_nutrient),
                    "{r:?}"
                );
                if r.drained_energy > 0.0 {
                    fed += 1;
                    equal += usize::from(close(r.drained_energy, r.potential_energy));
                }
            }
            world.compact_event_log_before(world.event_log().len());
        }
        assert!(fed > 20 && equal * 2 > fed, "{equal} of {fed} at potential");
    }

    fn reading(light: f32, drained: f32, uptake: f32) -> IntakeReading {
        IntakeReading {
            maintenance: 2.0,
            light,
            uptake,
            drained_energy: drained,
            drained_nutrient: 0.25 * drained,
            nutrient_per_energy: 0.5,
            ..Default::default()
        }
    }

    /// Light-fed mixotrophs set the upper bound (`2 × p25` of their light
    /// in ticks of maintenance), heterotrophs by diet the lower (`p75` of
    /// their intake); the default is the geometric midpoint. Other agents
    /// count in neither population.
    #[test]
    fn the_window_reads_both_populations() {
        let mut c = IntakeCensus::default();
        let producer = Some(TrophicRole::Producer);
        for light in [8.0, 10.0, 12.0, 14.0] {
            // light 4, 5, 6, 7 ticks; uptake 2 / 0.5 / 2 = 2 ticks
            c.record(
                producer,
                0.2,
                DietGroup::TraitProducer,
                &reading(light, 0.0, 2.0),
            );
        }
        // Not a mixotroph: no heterotrophy.
        c.record(
            producer,
            0.0,
            DietGroup::TraitProducer,
            &reading(0.2, 0.0, 0.0),
        );
        let consumer = Some(TrophicRole::Consumer);
        for drained in [1.0, 2.0, 3.0, 4.0] {
            // intake 1, 1.5, 2, 2.5 ticks; nutrient 0.25, 0.5, 0.75, 1
            c.record(
                consumer,
                0.9,
                DietGroup::TraitHeterotrophDietFed,
                &reading(1.0, drained, 0.0),
            );
        }
        // A light-fed trait heterotroph is not a heterotroph by diet.
        c.record(
            consumer,
            0.9,
            DietGroup::TraitHeterotrophLightFed,
            &reading(0.0, 400.0, 0.0),
        );
        assert_eq!(c.mixotroph_light.count(), 4);
        assert_eq!(c.heterotroph_intake.count(), 4);
        let w = c.energy_window().unwrap();
        let rel = |a: f64, b: f64| (a - b).abs() <= 0.13 * b;
        assert!(rel(w.upper, 2.0 * 5.0) && rel(w.lower, 2.5), "{w:?}");
        assert!(w.is_open());
        let k = w.default_k().unwrap();
        assert!((k - (w.upper * w.lower).sqrt()).abs() < 1e-9);
        let n = c.nutrient_window().unwrap();
        assert!(rel(n.upper, 4.0) && rel(n.lower, 1.0), "{n:?}");

        let mut merged = IntakeCensus::default();
        merged.merge(&c);
        merged.merge(&c);
        assert_eq!(merged.heterotroph_intake.count(), 8);
        assert_eq!(merged.energy_window().map(|w| w.is_open()), Some(true));
    }

    /// The summary line names both bounds and the default, or says plainly
    /// that the window is empty.
    #[test]
    fn the_window_line_reports_bounds_default_or_empty() {
        let open = window_line(
            Some(Window {
                upper: 8.0,
                lower: 2.0,
            }),
            "light",
            "intake",
        );
        assert_eq!(
            open,
            "upper ≤ 8.000 (2 × p25 light/maintenance, light-fed mixotrophs), lower ≥ 2.000 (p75 intake/maintenance, heterotrophs by diet), default k = 4.000 (geometric midpoint)"
        );
        let empty = window_line(
            Some(Window {
                upper: 2.0,
                lower: 3.0,
            }),
            "light",
            "intake",
        );
        assert!(empty.starts_with("window EMPTY: upper ≤ 2.000"), "{empty}");
        assert!(empty.contains("lower ≥ 3.000"), "{empty}");
        assert_eq!(window_line(None, "light", "intake"), "– (no samples)");
    }

    /// No `k` serves both when consumers take in more than twice the
    /// mixotrophs' light: the window is empty and proposes no default.
    #[test]
    fn an_empty_window_proposes_no_default() {
        let w = Window {
            upper: 2.0,
            lower: 3.0,
        };
        assert!(!w.is_open());
        assert_eq!(w.default_k(), None);
        assert_eq!(IntakeCensus::default().energy_window(), None);
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-6 * b.abs().max(1.0)
    }

    /// Holling's disk equation on the energy side: `E = room / (room + P)`,
    /// so the expressed intake `E·P` saturates at the room.
    #[test]
    fn expression_is_the_disk_equation_on_the_energy_side() {
        let e = predicted_expression((2.0, 6.0), None);
        assert!(close(e, 0.25), "{e}");
        assert!(close(e * 6.0, 2.0 * 6.0 / 8.0));
    }

    /// A full room expresses nothing however much is in reach; nothing in
    /// reach leaves any room fully expressed.
    #[test]
    fn a_full_room_expresses_nothing_and_an_empty_reach_expresses_fully() {
        assert_eq!(predicted_expression((0.0, 5.0), Some((0.0, 1.0))), 0.0);
        assert_eq!(predicted_expression((0.0, 0.0), None), 0.0);
        assert_eq!(predicted_expression((3.0, 0.0), Some((0.5, 0.0))), 1.0);
        assert_eq!(predicted_expression((0.0, 4.0), Some((0.5, 0.0))), 1.0);
    }

    /// Co-limited: hunger follows the currency with more room, so a
    /// consumer whose energy room is full still drains for nutrient.
    #[test]
    fn the_hungrier_nutrient_side_sets_expression() {
        let e = predicted_expression((0.0, 4.0), Some((1.0, 3.0)));
        assert!(close(e, 0.25), "{e}");
        let e = predicted_expression((1.0, 1.0), Some((3.0, 1.0)));
        assert!(close(e, 0.75), "{e}");
    }
}
