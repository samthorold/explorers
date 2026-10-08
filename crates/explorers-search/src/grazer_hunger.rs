//! A grazer's hunger at the moment it grazes (issue #606).
//!
//! Need-gated consumption (#600, #603) scaled a consumer's drain by its
//! expression `1 / (1 + c·s)`, where `s` is its satiation: the lesser of its
//! reserve and the energy its free nutrient can match, in ticks of its own
//! maintenance. Recognition (#604) withholds `r·w(d)` of capability from a
//! living target that resembles the consumer. So whether a kill was
//! *hunger-driven* was a question about the grazer's satiation in the drain
//! pass, and #605 found neither instrument could read it. #684 removed the
//! gate (expression is ungated, `r = 1`); the satiation and surplus readings
//! stay, as readings of the grazer's state, with no gate replayed on them.
//!
//! The drain pass reads each consumer's state after the tick's first four
//! phases (photosynthesise, absorb nutrients, metabolise, grow). [`PreStep`]
//! captures the roster and nutrient grid before a step and replays those
//! phases on a copy with the stepper's own public phase functions, so the
//! state it returns is the state the drain pass read, exactly (pinned in the
//! tests against the stepper's own drains). [`Satiation`] reads both
//! currencies from it, and [`GrazerHunger`] tallies (grazer, victim) pairs
//! by kinship, trait distance and satiation.
//!
//! #621 redefined satiation as **surplus** above the grow phase's retention
//! buffer, read after metabolism and before growth, and #623 made the stepper
//! read it until #684. [`PreStep::metabolised_agents`] replays the first
//! three phases and [`Surplus`] reads it there. #622 measured its
//! distribution ([`SurplusDistribution`]) to set the then default
//! sensitivity. [`Satiation`] is the previous, reserve-based reading
//! (#600–#619), which the [`GrazerHunger`] tallies still band by.
//!
//! #629 reads intake at the same point: [`PreStep::drain_start`] keeps what
//! the replayed photosynthesis and uptake credited each agent, for
//! `crate::intake_ceiling`.
//!
//! Observer-side only: the world is never touched.

use explorers_sim::event::EventKind;
use explorers_sim::phase;
use explorers_sim::spatial::{NutrientGrid, SpatialGrid};
use explorers_sim::{Agent, Carcass, TraitVector, World, WorldParameters};
use std::collections::HashMap;

/// The roster and nutrient grid before a step: enough to replay the tick's
/// phases up to the drain pass.
#[derive(Clone)]
pub struct PreStep {
    agents: Vec<Agent>,
    nutrient_grid: NutrientGrid,
    /// The carcasses as the step found them, before leaching.
    start_carcasses: Vec<Carcass>,
    /// The carcasses the drain pass reads: `start_carcasses` leached.
    carcasses: Vec<Carcass>,
    /// The world's most recent move distances, by id: what this tick's
    /// hyphal uptake reads as substrate contact (#727).
    last_move_distance: HashMap<u64, f32>,
}

/// The state at the start of drain resolution (#629): the drain-time roster
/// ([`PreStep::drain_time_agents`]) with each agent's photosynthesis and
/// nutrient uptake this tick, by id (absent = none). Before the drain pass
/// carcasses only leach (#698), so [`PreStep::carcasses`] is theirs.
#[derive(Clone, Debug)]
pub struct DrainStart {
    pub agents: Vec<Agent>,
    pub light: HashMap<u64, f32>,
    pub uptake: HashMap<u64, f32>,
    /// The hyphal part of each agent's `uptake` (#727): `uptake × c·H_eff /
    /// (A_eff + c·H_eff)`, its hyphal demand's share of its own demand (the
    /// size term is common to both). Empty with `hyphal_uptake` off.
    pub hyphal: HashMap<u64, f32>,
    /// Each agent's nutrient deficit as the stepper reads it for retention
    /// (`phase::retention_deficit`, after metabolise and before grow), by id.
    pub deficit: HashMap<u64, f32>,
}

impl PreStep {
    /// Capture `world` before it steps.
    pub fn capture(world: &World) -> Self {
        let start_carcasses = world.carcasses().to_vec();
        let mut carcasses = start_carcasses.clone();
        let mut scratch = world.nutrient_grid().clone();
        phase::leach_carcasses(&mut carcasses, &mut scratch, world.params());
        PreStep {
            agents: world.agents().to_vec(),
            nutrient_grid: world.nutrient_grid().clone(),
            start_carcasses,
            carcasses,
            last_move_distance: world.last_move_distance().clone(),
        }
    }

    /// Uptake replayed as the stepper absorbs: substrate contact off the
    /// world's most recent move distances.
    fn absorb(
        &self,
        agents: &mut [Agent],
        grid: &mut NutrientGrid,
        params: &WorldParameters,
    ) -> Vec<explorers_sim::event::Event> {
        phase::absorb_nutrients_after_moves(agents, grid, params, &self.last_move_distance)
    }

    /// The hyphal part of each agent's replayed `uptake` (empty with the
    /// switch off), read on the agents as uptake found them.
    fn hyphal_parts(
        &self,
        agents: &[Agent],
        uptake: &HashMap<u64, f32>,
        params: &WorldParameters,
    ) -> HashMap<u64, f32> {
        if !params.hyphal_uptake {
            return HashMap::new();
        }
        let k = params.wear_degradation_steepness;
        agents
            .iter()
            .filter_map(|a| {
                let u = *uptake.get(&a.id)?;
                let moved = self.last_move_distance.get(&a.id).copied().unwrap_or(0.0);
                let h = phase::substrate_contact(moved, params)
                    * a.effective_trait_with_steepness(1, k);
                let rate = a.effective_trait_with_steepness(0, k) + h;
                (rate > 0.0).then(|| (a.id, u * h / rate))
            })
            .collect()
    }

    /// The nutrient grid as uptake reads it: the pre-step grid with this
    /// tick's leaching (#698) added, as the stepper leaches after
    /// photosynthesis and before uptake.
    fn leached_grid(&self, params: &WorldParameters) -> NutrientGrid {
        let mut grid = self.nutrient_grid.clone();
        let mut carcasses = self.start_carcasses.clone();
        phase::leach_carcasses(&mut carcasses, &mut grid, params);
        grid
    }

    /// The agents before the step.
    pub fn agents(&self) -> &[Agent] {
        &self.agents
    }

    /// The carcasses the drain pass reads: those before the step, less what
    /// each leached this tick (none at `leaching_rate` 0).
    pub fn carcasses(&self) -> &[Carcass] {
        &self.carcasses
    }

    /// The tick's first four phases replayed as [`PreStep::drain_time_agents`]
    /// does, keeping what photosynthesis and uptake credited each agent (their
    /// events' `energy_delta`).
    pub fn drain_start(&self, params: &WorldParameters) -> DrainStart {
        let mut agents = self.agents.clone();
        let mut nutrient_grid = self.leached_grid(params);
        let cell_size = params.light_competition_radius.max(1.0);
        let mut grid = SpatialGrid::new(params.world_extent, cell_size);
        for (i, a) in agents.iter().enumerate() {
            grid.insert(i as u64, a.position);
        }
        let by_source = |events: Vec<explorers_sim::event::Event>, kind: EventKind| {
            let mut out: HashMap<u64, f32> = HashMap::new();
            for e in events.into_iter().filter(|e| e.kind == kind) {
                *out.entry(e.source).or_default() += e.energy_delta;
            }
            out
        };
        let light = by_source(
            phase::photosynthesise(&mut agents, &grid, params),
            EventKind::Photosynthesized,
        );
        let absorbing = agents.clone();
        let uptake = by_source(
            self.absorb(&mut agents, &mut nutrient_grid, params),
            EventKind::NutrientAbsorbed,
        );
        let hyphal = self.hyphal_parts(&absorbing, &uptake, params);
        phase::metabolise(&mut agents, params);
        let deficit = agents
            .iter()
            .map(|a| (a.id, phase::retention_deficit(a, params)))
            .collect();
        phase::grow(&mut agents, params);
        DrainStart {
            agents,
            light,
            uptake,
            hyphal,
            deficit,
        }
    }

    /// The roster as the grow phase reads it: the tick's first three phases
    /// (photosynthesise, absorb nutrients, metabolise) replayed, in the
    /// stepper's order, on a copy of the pre-step state. Where [`Surplus`] is
    /// read (#622).
    pub fn metabolised_agents(&self, params: &WorldParameters) -> Vec<Agent> {
        let mut agents = self.agents.clone();
        let mut nutrient_grid = self.leached_grid(params);
        let cell_size = params.light_competition_radius.max(1.0);
        let mut grid = SpatialGrid::new(params.world_extent, cell_size);
        for (i, a) in agents.iter().enumerate() {
            grid.insert(i as u64, a.position);
        }
        phase::photosynthesise(&mut agents, &grid, params);
        self.absorb(&mut agents, &mut nutrient_grid, params);
        phase::metabolise(&mut agents, params);
        agents
    }

    /// The roster as the drain pass read it: [`PreStep::metabolised_agents`]
    /// grown — the tick's first four phases.
    pub fn drain_time_agents(&self, params: &WorldParameters) -> Vec<Agent> {
        self.drain_start(params).agents
    }

    /// The metabolised roster ([`PreStep::metabolised_agents`]), each agent
    /// with the energy the grow phase moves into its reproductive earmark
    /// this tick: `repro_reserve` after growth less before, the accounting's
    /// earmark fill.
    pub fn earmark_fills(&self, params: &WorldParameters) -> Vec<(Agent, f32)> {
        let metabolised = self.metabolised_agents(params);
        let mut grown = metabolised.clone();
        phase::grow(&mut grown, params);
        metabolised
            .into_iter()
            .zip(&grown)
            .map(|(m, g)| {
                let fill = g.repro_reserve - m.repro_reserve;
                (m, fill)
            })
            .collect()
    }

    /// Every agent's [`KillerReading`] for the tick: drain-time
    /// [`Satiation`] off the grown roster, [`Surplus`] off the metabolised
    /// roster (the pre-growth read).
    pub fn killer_readings(&self, params: &WorldParameters) -> HashMap<u64, KillerReading> {
        let metabolised = self.metabolised_agents(params);
        let mut grown = metabolised.clone();
        phase::grow(&mut grown, params);
        metabolised
            .iter()
            .zip(&grown)
            .map(|(m, g)| {
                let reading = KillerReading::of(Satiation::of(g, params), Surplus::of(m, params));
                (m.id, reading)
            })
            .collect()
    }
}

/// What a killing grazer is read at (#606, #624): its drain-time
/// [`Satiation`] (the pre-#623 reading) and its pre-growth [`Surplus`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KillerReading {
    pub satiation: Satiation,
    pub surplus: Surplus,
    /// The intake gate's predicted expression at a given ceiling multiple
    /// `k` (#629; `IntakeReading::expression_at`), when one was given.
    pub intake_expression: Option<f32>,
}

impl KillerReading {
    /// A reading of these two states, with no intake-gate expression.
    pub fn of(satiation: Satiation, surplus: Surplus) -> Self {
        KillerReading {
            satiation,
            surplus,
            intake_expression: None,
        }
    }
}

/// An agent's maintenance need: its per-tick metabolic cost, floored at zero.
fn maintenance_need(agent: &Agent, params: &WorldParameters) -> f32 {
    explorers_sim::phase::metabolic_cost(agent, params).max(0.0)
}

/// The reserve energy an agent's free nutrient can match when built into
/// structure, `N / (growth_efficiency × demand ratio)`: infinite when growth
/// binds no nutrient.
fn nutrient_matched_energy(agent: &Agent, params: &WorldParameters) -> f32 {
    let binding =
        params.growth_efficiency * explorers_sim::stoichiometric_demand(&agent.traits, 1.0, params);
    if binding > 0.0 {
        agent.nutrient.max(0.0) / binding
    } else {
        f32::INFINITY
    }
}

/// An agent's **surplus satiation** (world rules, *Capability and expression
/// are decoupled*; settled in #621): the reserve above the grow phase's
/// retention buffer, co-limited by the energy its free nutrient can match, in
/// ticks of its own maintenance. Read after metabolism and before growth
/// ([`PreStep::metabolised_agents`]): the surplus the tick's grow phase is
/// about to mobilise. Only the energy side is shifted by the buffer, so
/// `energy` is negative below it.
///
/// The stepper read it for the surplus gate from #623 until #684 removed
/// the gate; it stays a reading of the agent's state, split by side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surplus {
    /// `(reserve − buffer) / m`, with `buffer = growth_retention_multiplier × m`.
    pub energy: f32,
    /// `N / (growth_efficiency × ratio) / m`.
    pub nutrient: f32,
}

impl Surplus {
    pub fn of(agent: &Agent, params: &WorldParameters) -> Self {
        let need = maintenance_need(agent, params);
        let buffer = params.growth_retention_multiplier * need;
        let ticks = |e: f32| if need > 0.0 { e / need } else { f32::INFINITY };
        Surplus {
            energy: ticks(agent.reserve - buffer),
            nutrient: ticks(nutrient_matched_energy(agent, params)),
        }
    }

    /// `s = max(0, min(energy, nutrient))`: the surplus the withdrawn need
    /// gate read.
    pub fn ticks(&self) -> f32 {
        self.energy.min(self.nutrient).max(0.0)
    }
}
/// Lower edge of the [`SurplusDistribution`]'s log bins, in ticks of
/// maintenance; positive surplus below it is binned together.
pub const SURPLUS_LOW: f64 = 1e-3;
/// Log bins per decade.
pub const SURPLUS_BINS_PER_DECADE: usize = 20;
/// Decades the log bins span from [`SURPLUS_LOW`] (to 1e5 ticks); surplus at
/// or above the top edge (an infinite one included) is binned together.
pub const SURPLUS_DECADES: usize = 8;
const SURPLUS_LOG_BINS: usize = SURPLUS_BINS_PER_DECADE * SURPLUS_DECADES;

/// A distribution of [`Surplus::ticks`] over agent-samples: agents at
/// exactly zero (at or below the buffer, or without free nutrient) counted
/// apart, the rest in fine log bins, so percentiles read within a bin's
/// width (~12 %). `bins` is `[below SURPLUS_LOW, log bins…, at or above the
/// top]`, empty until something positive is recorded.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SurplusDistribution {
    pub zero: u64,
    pub bins: Vec<u64>,
    /// Samples recorded with [`SurplusDistribution::record_surplus`] whose
    /// nutrient side was the lesser.
    #[serde(default)]
    pub nutrient_limited: u64,
    /// Of the zeros, those held at zero by nutrient (no free nutrient) rather
    /// than by the buffer.
    #[serde(default)]
    pub zero_nutrient_limited: u64,
}

impl SurplusDistribution {
    /// Record a surplus read, noting whether nutrient bound it.
    pub fn record_surplus(&mut self, surplus: &Surplus) {
        let s = surplus.ticks();
        let nutrient = surplus.nutrient < surplus.energy;
        self.nutrient_limited += u64::from(nutrient);
        self.zero_nutrient_limited += u64::from(nutrient && s <= 0.0);
        self.record(s);
    }

    pub fn record(&mut self, s: f32) {
        if s <= 0.0 || s.is_nan() {
            self.zero += 1;
            return;
        }
        if self.bins.is_empty() {
            self.bins = vec![0; SURPLUS_LOG_BINS + 2];
        }
        let s = s as f64;
        let i = if s < SURPLUS_LOW {
            0
        } else {
            let k = ((s / SURPLUS_LOW).log10() * SURPLUS_BINS_PER_DECADE as f64).floor();
            (k as usize).min(SURPLUS_LOG_BINS) + 1
        };
        self.bins[i] += 1;
    }

    pub fn count(&self) -> u64 {
        self.zero + self.bins.iter().sum::<u64>()
    }

    /// Share of samples at zero surplus; `None` when empty.
    pub fn zero_share(&self) -> Option<f64> {
        let n = self.count();
        (n > 0).then(|| self.zero as f64 / n as f64)
    }

    /// The `q` quantile (`0 ≤ q ≤ 1`), interpolated geometrically within its
    /// log bin; `None` when empty. The bottom bin interpolates linearly from
    /// zero, and the top bin reads as its lower edge.
    pub fn percentile(&self, q: f64) -> Option<f64> {
        let n = self.count();
        if n == 0 {
            return None;
        }
        let mut rank = q.clamp(0.0, 1.0) * n as f64;
        if rank < self.zero as f64 {
            return Some(0.0);
        }
        rank -= self.zero as f64;
        let step = 10f64.powf(1.0 / SURPLUS_BINS_PER_DECADE as f64);
        let last = self.bins.len().saturating_sub(1);
        for (i, &c) in self.bins.iter().enumerate() {
            if c == 0 {
                continue;
            }
            if rank < c as f64 || i == last {
                let frac = (rank / c as f64).min(1.0);
                return Some(match i {
                    0 => SURPLUS_LOW * frac,
                    i if i == last => SURPLUS_LOW * step.powi(SURPLUS_LOG_BINS as i32),
                    i => SURPLUS_LOW * step.powf((i - 1) as f64 + frac),
                });
            }
            rank -= c as f64;
        }
        // Only reachable at q = 1 with an empty top bin: the last occupied
        // bin's upper edge.
        let top = self.bins.iter().rposition(|&c| c > 0)?;
        Some(SURPLUS_LOW * step.powi(top as i32))
    }

    /// The `q` quantile of the positive samples alone (the zeros left out);
    /// `None` when none is positive. The default-sensitivity criterion reads
    /// this: `c = 1 / s₂₅` over light-fed mixotrophs with surplus (#622).
    pub fn positive_percentile(&self, q: f64) -> Option<f64> {
        if self.bins.iter().all(|&c| c == 0) {
            return None;
        }
        SurplusDistribution {
            zero: 0,
            bins: self.bins.clone(),
            ..Default::default()
        }
        .percentile(q)
    }

    pub fn merge(&mut self, other: &SurplusDistribution) {
        self.zero += other.zero;
        self.nutrient_limited += other.nutrient_limited;
        self.zero_nutrient_limited += other.zero_nutrient_limited;
        if other.bins.is_empty() {
            return;
        }
        if self.bins.is_empty() {
            self.bins = vec![0; other.bins.len()];
        }
        for (a, b) in self.bins.iter_mut().zip(&other.bins) {
            *a += b;
        }
    }
}

/// An agent's satiation in both currencies, in ticks of its own maintenance
/// (its per-tick metabolic cost): `energy` is its reserve, `nutrient` the
/// energy its free nutrient can match when built into structure
/// (`N / (growth_efficiency × demand ratio)`), infinite when growth binds no
/// nutrient. The need gate read the lesser ([`Satiation::ticks`]) on the
/// whole reserve, after growth, until #623 moved it to [`Surplus`]; kept for
/// the #606 grazer-hunger tallies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Satiation {
    pub energy: f32,
    pub nutrient: f32,
}

impl Satiation {
    pub fn of(agent: &Agent, params: &WorldParameters) -> Self {
        let need = maintenance_need(agent, params);
        let ticks = |e: f32| if need > 0.0 { e / need } else { f32::INFINITY };
        Satiation {
            energy: ticks(agent.reserve.max(0.0)),
            nutrient: ticks(nutrient_matched_energy(agent, params)),
        }
    }

    /// The co-limited satiation the pre-#623 need gate read: the scarcer currency.
    pub fn ticks(&self) -> f32 {
        self.energy.min(self.nutrient)
    }

    /// Whether nutrient, not energy, is the scarcer currency.
    pub fn nutrient_limited(&self) -> bool {
        self.nutrient < self.energy
    }
}
/// Upper edges of the trait-distance bands (on `TraitVector::distance`, the
/// recognition metric): `[0, 0.1)`, `[0.1, 0.25)`, `[0.25, 0.5)`,
/// `[0.5, 1)`, `≥ 1`. At the default recognition distance 0.5 the first three
/// are inside it, where resemblance `w` is above 0.92, 0.56 and 0.
pub const DISTANCE_EDGES: [f32; 4] = [0.1, 0.25, 0.5, 1.0];
pub const DISTANCE_BANDS: usize = DISTANCE_EDGES.len() + 1;
/// The bands inside the default recognition distance.
pub const RECOGNITION_BANDS: usize = 3;

/// Upper edges of the satiation bands, in ticks of maintenance:
/// `[0, 1)`, `[1, 5)`, `[5, 10)`, `[10, 20)`, `[20, 50)`, `≥ 50`. At the
/// pre-#623 default `satiation_sensitivity = 0.1`, 10 ticks was half
/// expression, from which recognition spares an identical target entirely.
pub const SATIATION_EDGES: [f32; 5] = [1.0, 5.0, 10.0, 20.0, 50.0];
pub const SATIATION_BANDS: usize = SATIATION_EDGES.len() + 1;
/// The bands under half expression (at the pre-#623 default): hungry.
pub const HUNGRY_BANDS: usize = 3;

/// Upper edges of the expression bands (#624, #629), on a gate expression
/// `E ∈ [0, 1]`: `[0, 0.1)`, `[0.1, 0.5)`, `[0.5, 0.9)`, `≥ 0.9`. The intake
/// gate's predicted expression is banded on them.
pub const EXPRESSION_EDGES: [f32; 3] = [0.1, 0.5, 0.9];
pub const EXPRESSION_BANDS: usize = EXPRESSION_EDGES.len() + 1;
/// The first band at or above half expression (`E ≥ 0.5`).
pub const HALF_EXPRESSION_BAND: usize = 2;

fn band(value: f32, edges: &[f32]) -> usize {
    edges.iter().take_while(|&&e| value >= e).count()
}

/// The [`EXPRESSION_EDGES`] band an expression falls in.
pub fn expression_band(e: f32) -> usize {
    band(e, &EXPRESSION_EDGES)
}

/// Band index of a trait distance.
pub fn distance_band(d: f32) -> usize {
    band(d, &DISTANCE_EDGES)
}

/// (grazer, victim) pairs tallied by kinship, the pair's trait distance and
/// the grazer's drain-time satiation. Index `[kin]` is `0` for non-kin and
/// `1` for kin.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GrazerHunger {
    /// `[kin][distance band][satiation band]`.
    pub pairs: [[[u64; SATIATION_BANDS]; DISTANCE_BANDS]; 2],
    /// Of those, pairs whose grazer's scarcer currency was nutrient:
    /// `[kin][satiation band]`.
    pub nutrient_limited: [[u64; SATIATION_BANDS]; 2],
    /// `[kin]`: pairs whose grazer held no pre-growth surplus (`s = 0`, full
    /// expression).
    #[serde(default)]
    pub surplus_zero: [u64; 2],
    /// `[kin][expression band]`: pairs by the grazer's predicted intake-gate
    /// expression at the census's `--intake-ceiling-k` (#629). Empty without
    /// one, and on rows written before #629.
    #[serde(default)]
    pub intake_expression: [[u64; EXPRESSION_BANDS]; 2],
}

impl GrazerHunger {
    pub fn record(&mut self, kin: bool, distance: f32, reading: KillerReading) {
        let k = usize::from(kin);
        let satiation = reading.satiation;
        let s = band(satiation.ticks(), &SATIATION_EDGES);
        self.pairs[k][distance_band(distance)][s] += 1;
        if satiation.nutrient_limited() {
            self.nutrient_limited[k][s] += 1;
        }
        self.surplus_zero[k] += u64::from(reading.surplus.ticks() <= 0.0);
        if let Some(e) = reading.intake_expression {
            self.intake_expression[k][band(e, &EXPRESSION_EDGES)] += 1;
        }
    }

    pub fn merge(&mut self, other: &GrazerHunger) {
        for k in 0..2 {
            for d in 0..DISTANCE_BANDS {
                for s in 0..SATIATION_BANDS {
                    self.pairs[k][d][s] += other.pairs[k][d][s];
                }
            }
            for s in 0..SATIATION_BANDS {
                self.nutrient_limited[k][s] += other.nutrient_limited[k][s];
            }
            for e in 0..EXPRESSION_BANDS {
                self.intake_expression[k][e] += other.intake_expression[k][e];
            }
            self.surplus_zero[k] += other.surplus_zero[k];
        }
    }

    /// Pairs of this kinship.
    pub fn total(&self, kin: bool) -> u64 {
        self.pairs[usize::from(kin)].iter().flatten().sum()
    }

    /// Pairs of this kinship whose grazer was hungry (under half expression).
    pub fn hungry(&self, kin: bool) -> u64 {
        self.pairs[usize::from(kin)]
            .iter()
            .map(|row| row[..HUNGRY_BANDS].iter().sum::<u64>())
            .sum()
    }

    /// Pairs of this kinship in the first `bands` distance bands.
    pub fn within(&self, kin: bool, bands: usize) -> u64 {
        self.pairs[usize::from(kin)][..bands].iter().flatten().sum()
    }

    /// Pairs of this kinship by satiation band, summed over distance.
    pub fn by_satiation(&self, kin: bool) -> [u64; SATIATION_BANDS] {
        let mut out = [0; SATIATION_BANDS];
        for row in &self.pairs[usize::from(kin)] {
            for (o, v) in out.iter_mut().zip(row) {
                *o += v;
            }
        }
        out
    }

    /// Pairs of this kinship by distance band, summed over satiation.
    pub fn by_distance(&self, kin: bool) -> [u64; DISTANCE_BANDS] {
        let mut out = [0; DISTANCE_BANDS];
        for (o, row) in out.iter_mut().zip(&self.pairs[usize::from(kin)]) {
            *o = row.iter().sum();
        }
        out
    }
}

/// Trait distances tallied in the [`DISTANCE_EDGES`] bands, with their mean:
/// the parent–offspring distance a census reads off `Born` events.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TraitDistances {
    pub bands: [u64; DISTANCE_BANDS],
    pub sum: f64,
    pub sum_sq: f64,
}

impl TraitDistances {
    pub fn record(&mut self, a: &TraitVector, b: &TraitVector) {
        let d = a.distance(b);
        self.bands[distance_band(d)] += 1;
        self.sum += d as f64;
        self.sum_sq += (d as f64) * (d as f64);
    }

    pub fn count(&self) -> u64 {
        self.bands.iter().sum()
    }

    pub fn mean(&self) -> Option<f64> {
        let n = self.count();
        (n > 0).then(|| self.sum / n as f64)
    }

    /// Root mean square distance: `σ·√(7·p)` for an asexual birth at
    /// mutation magnitude `σ` and rate `p`, away from the trait floors.
    pub fn rms(&self) -> Option<f64> {
        let n = self.count();
        (n > 0).then(|| (self.sum_sq / n as f64).sqrt())
    }

    pub fn merge(&mut self, other: &TraitDistances) {
        for (a, b) in self.bands.iter_mut().zip(other.bands) {
            *a += b;
        }
        self.sum += other.sum;
        self.sum_sq += other.sum_sq;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_source::{ConfigSource, resolve_config, sampled_units};

    use explorers_sim::event::EventKind;

    fn sample_31_world(seed: u64) -> World {
        let (params, dist) = resolve_config(
            ConfigSource::SAMPLE,
            31,
            &Default::default(),
            &sampled_units(),
        );
        World::new(params, dist, seed)
    }

    /// Replaying the pre-step state recovers what the stepper drained with:
    /// every living drain a lone consumer took from a target that could cover
    /// it equals `h_eff · u_H · max(0, 1 − r·w)` (expression ungated, #684),
    /// with the target's structure off the drain-time roster, bit for bit.
    #[test]
    fn replayed_state_reproduces_the_stepper_s_living_drains() {
        let mut world = sample_31_world(1000);
        world.retain_event_kinds(&[EventKind::Consumed]);
        let params = world.params().clone();
        let k = params.wear_degradation_steepness;
        let mut checked = 0;
        for _ in 0..120 {
            let pre = PreStep::capture(&world);
            let cursor = world.event_log().len();
            world.step();
            let events: Vec<_> = world
                .event_log()
                .since(cursor)
                .iter()
                .filter(|e| e.kind == EventKind::Consumed && !e.target_was_carcass)
                .cloned()
                .collect();
            if events.is_empty() {
                continue;
            }
            let drain_time = pre.drain_time_agents(&params);
            let by_id: HashMap<u64, &Agent> = drain_time.iter().map(|a| (a.id, a)).collect();
            let mut consumers_of: HashMap<u64, usize> = HashMap::new();
            for e in &events {
                *consumers_of.entry(e.target.unwrap()).or_default() += 1;
            }
            for e in &events {
                let target = e.target.unwrap();
                if consumers_of[&target] != 1 {
                    continue;
                }
                let (c, t) = (by_id[&e.source], by_id[&target]);
                let expression = (1.0
                    - phase::RECOGNITION_RESTRAINT
                        * phase::resemblance(&c.traits, &t.traits, &params))
                .max(0.0);
                let demand = c.effective_trait_with_steepness(1, k)
                    * explorers_sim::units::HETEROTROPHY_STRUCTURE_DRAIN_PER_TICK
                    * expression;
                if demand > t.structure {
                    continue;
                }
                assert_eq!(e.energy_delta.to_bits(), demand.to_bits(), "{e:?}");
                checked += 1;
            }
            world.compact_event_log_before(world.event_log().len());
        }
        assert!(checked > 10, "only {checked} drains checked");
    }

    /// With carcass leaching on (#698, #700), the replay leaches as the
    /// stepper does before uptake: each agent's replayed uptake is the
    /// stepper's `NutrientAbsorbed`, bit for bit, and the drain-time
    /// carcasses are the pre-step ones less what each leached.
    #[test]
    fn replay_leaches_carcasses_before_uptake_as_the_stepper_does() {
        let mut world = sample_31_world(1000);
        world.params_mut().leaching_rate = 0.2;
        world.retain_event_kinds(&[EventKind::NutrientAbsorbed, EventKind::Leached]);
        let params = world.params().clone();
        let (mut leached, mut absorbed) = (0, 0);
        for _ in 0..150 {
            let pre = PreStep::capture(&world);
            let raw: HashMap<u64, f32> = world
                .carcasses()
                .iter()
                .map(|c| (c.id, c.nutrient))
                .collect();
            let cursor = world.event_log().len();
            world.step();
            let events = world.event_log().since(cursor).to_vec();
            let start = pre.drain_start(&params);
            let drain_time: HashMap<u64, f32> =
                pre.carcasses().iter().map(|c| (c.id, c.nutrient)).collect();
            for e in &events {
                match e.kind {
                    EventKind::NutrientAbsorbed => {
                        assert_eq!(
                            start.uptake.get(&e.source).map(|u| u.to_bits()),
                            Some(e.energy_delta.to_bits()),
                            "{e:?}"
                        );
                        absorbed += 1;
                    }
                    EventKind::Leached => {
                        assert_eq!(
                            drain_time[&e.source].to_bits(),
                            (raw[&e.source] - e.nutrient_delta).to_bits(),
                            "{e:?}"
                        );
                        leached += 1;
                    }
                    _ => {}
                }
            }
            world.compact_event_log_before(world.event_log().len());
        }
        assert!(leached > 10, "only {leached} leaching events");
        assert!(absorbed > 10, "only {absorbed} uptakes");
    }

    /// With hyphal uptake on (#727), the replay reads substrate contact off
    /// the world's most recent move distances as the stepper does: each
    /// agent's replayed uptake is the stepper's `NutrientAbsorbed`, bit for
    /// bit, and its hyphal part is `uptake × c·H_eff / (A_eff + c·H_eff)`.
    #[test]
    fn replay_reads_substrate_contact_as_the_stepper_does_with_hyphal_uptake_on() {
        let mut world = sample_31_world(1000);
        world.params_mut().hyphal_uptake = true;
        world.retain_event_kinds(&[EventKind::NutrientAbsorbed]);
        let params = world.params().clone();
        let k = params.wear_degradation_steepness;
        let (mut absorbed, mut hyphal, mut partial) = (0, 0.0_f64, 0);
        for _ in 0..150 {
            let pre = PreStep::capture(&world);
            let moved = world.last_move_distance().clone();
            let cursor = world.event_log().len();
            world.step();
            let events = world.event_log().since(cursor).to_vec();
            let start = pre.drain_start(&params);
            for e in &events {
                assert_eq!(
                    start.uptake.get(&e.source).map(|u| u.to_bits()),
                    Some(e.energy_delta.to_bits()),
                    "{e:?}"
                );
                absorbed += 1;
                let a = pre.agents().iter().find(|a| a.id == e.source).unwrap();
                let c = phase::substrate_contact(moved.get(&a.id).copied().unwrap_or(0.0), &params);
                let (aa, h) = (
                    a.effective_trait_with_steepness(0, k),
                    c * a.effective_trait_with_steepness(1, k),
                );
                let want = e.energy_delta * h / (aa + h);
                let got = start.hyphal.get(&e.source).copied().unwrap_or(0.0);
                assert!(
                    (got - want).abs() <= 1e-6 * e.energy_delta.max(1.0),
                    "{e:?}"
                );
                hyphal += f64::from(got);
                partial += usize::from(c < 1.0 && h > 0.0);
            }
            world.compact_event_log_before(world.event_log().len());
        }
        assert!(absorbed > 10, "only {absorbed} uptakes");
        assert!(hyphal > 0.0, "no hyphal uptake on sample:31");
        assert!(partial > 0, "no moving heterotroph absorbed");
    }

    /// With hyphal uptake off, nothing is booked as hyphal.
    #[test]
    fn nothing_is_hyphal_with_the_switch_off() {
        let mut world = sample_31_world(1000);
        let params = world.params().clone();
        assert!(!params.hyphal_uptake);
        for _ in 0..60 {
            let start = PreStep::capture(&world).drain_start(&params);
            assert!(start.hyphal.values().all(|&h| h == 0.0));
            world.step();
        }
    }

    /// Satiation reads both currencies in ticks of maintenance, the reading
    /// the need gate took before #623: `min(s_E, s_N)` on the whole reserve
    /// at drain time.
    #[test]
    fn satiation_is_the_reserve_based_co_limited_reading() {
        let mut world = sample_31_world(1001);
        for _ in 0..60 {
            world.step();
        }
        let params = world.params().clone();
        let drain_time = PreStep::capture(&world).drain_time_agents(&params);
        let (mut energy_limited, mut nutrient_limited) = (0, 0);
        for a in &drain_time {
            let s = Satiation::of(a, &params);
            let m = phase::metabolic_cost(a, &params);
            assert!((s.energy - a.reserve.max(0.0) / m).abs() <= 1e-5 * s.energy.max(1.0));
            assert_eq!(s.ticks(), s.energy.min(s.nutrient));
            if s.nutrient_limited() {
                nutrient_limited += 1;
            } else {
                energy_limited += 1;
            }
        }
        assert!(energy_limited + nutrient_limited > 10);
    }

    fn lone_agent(reserve: f32, nutrient: f32) -> (Agent, WorldParameters) {
        let params = sample_31_world(1000).params().clone();
        let traits = TraitVector {
            photosynthetic_absorption: 0.8,
            heterotrophy: 0.2,
            mobility: 0.0,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 0.5,
            dispersal: 0.3,
        };
        (
            Agent::new(0, (0.0, 0.0), reserve, 1.0, nutrient, traits),
            params,
        )
    }

    /// Surplus satiation (#621, #622) counts reserve above the grow phase's
    /// retention buffer, in ticks of maintenance.
    #[test]
    fn surplus_counts_reserve_above_the_retention_buffer() {
        let (probe, params) = lone_agent(0.0, 1e6);
        let m = phase::metabolic_cost(&probe, &params);
        let buffer = params.growth_retention_multiplier * m;
        let (above, _) = lone_agent(buffer + 3.0 * m, 1e6);
        let s = Surplus::of(&above, &params).ticks();
        assert!((s - 3.0).abs() < 1e-4, "{s}");
    }

    /// An agent at or below its buffer has no surplus, however much reserve
    /// it holds: `s` is floored at zero, and the energy side reads negative.
    #[test]
    fn surplus_is_zero_at_and_below_the_buffer() {
        let (probe, params) = lone_agent(0.0, 1e6);
        let m = phase::metabolic_cost(&probe, &params);
        let buffer = params.growth_retention_multiplier * m;
        assert!(buffer > m, "a buffer of several ticks: {buffer} vs {m}");
        let (at, _) = lone_agent(buffer, 1e6);
        assert!(Surplus::of(&at, &params).ticks().abs() < 1e-5);
        let (below, _) = lone_agent(buffer - 0.5 * m, 1e6);
        let s = Surplus::of(&below, &params);
        assert_eq!(s.ticks(), 0.0);
        assert!((s.energy + 0.5).abs() < 1e-4, "{s:?}");
        let (empty, _) = lone_agent(0.0, 1e6);
        assert_eq!(Surplus::of(&empty, &params).ticks(), 0.0);
    }

    /// Surplus energy counts only as far as free nutrient could match it in
    /// growth, and the nutrient side is not shifted by the buffer: an agent
    /// far above its buffer, holding nutrient to match 2 ticks of maintenance,
    /// reads 2 ticks (not 2 minus the buffer's depth).
    #[test]
    fn surplus_is_co_limited_by_unbuffered_free_nutrient() {
        let (probe, params) = lone_agent(0.0, 0.0);
        let m = phase::metabolic_cost(&probe, &params);
        let buffer = params.growth_retention_multiplier * m;
        let binding = params.growth_efficiency
            * explorers_sim::stoichiometric_demand(&probe.traits, 1.0, &params);
        assert!(binding > 0.0 && params.growth_retention_multiplier > 2.0);
        let (limited, _) = lone_agent(buffer + 10.0 * m, 2.0 * m * binding);
        let s = Surplus::of(&limited, &params);
        assert!((s.nutrient - 2.0).abs() < 1e-4, "{s:?}");
        assert!((s.ticks() - 2.0).abs() < 1e-4, "{s:?}");
        let (starved, _) = lone_agent(buffer + 10.0 * m, 0.0);
        assert_eq!(Surplus::of(&starved, &params).ticks(), 0.0);
    }

    /// The surplus read sits between metabolism and growth: growing the
    /// metabolised roster is the drain-time roster, bit for bit, and the
    /// surplus read off it is the stepper's buffer arithmetic.
    #[test]
    fn metabolised_roster_is_the_drain_time_roster_before_growth() {
        let mut world = sample_31_world(1002);
        for _ in 0..60 {
            world.step();
        }
        let params = world.params().clone();
        let pre = PreStep::capture(&world);
        let mut grown = pre.metabolised_agents(&params);
        let metabolised = grown.clone();
        phase::grow(&mut grown, &params);
        let drain_time = pre.drain_time_agents(&params);
        assert_eq!(grown.len(), drain_time.len());
        let mut positive = 0;
        for (a, b) in grown.iter().zip(&drain_time) {
            assert_eq!(a.reserve.to_bits(), b.reserve.to_bits());
            assert_eq!(a.structure.to_bits(), b.structure.to_bits());
            assert_eq!(a.nutrient.to_bits(), b.nutrient.to_bits());
        }
        for a in &metabolised {
            let m = phase::metabolic_cost(a, &params);
            let s = Surplus::of(a, &params);
            let want = (a.reserve - params.growth_retention_multiplier * m) / m;
            assert!(
                (s.energy - want).abs() <= 1e-4 * want.abs().max(1.0),
                "{s:?}"
            );
            positive += usize::from(s.ticks() > 0.0);
        }
        assert!(positive > 0, "some agent holds a surplus before growth");
    }

    /// The surplus distribution counts agents at zero apart and reads
    /// percentiles off fine log bins, within a bin's width; merging adds.
    #[test]
    fn surplus_distribution_reads_the_zero_share_and_percentiles() {
        let mut d = SurplusDistribution::default();
        assert_eq!(d.percentile(0.5), None);
        for _ in 0..100 {
            d.record(0.0);
        }
        for i in 1..=300 {
            d.record(i as f32 * 0.1);
        }
        assert_eq!(d.count(), 400);
        assert_eq!(d.zero, 100);
        assert!((d.zero_share().unwrap() - 0.25).abs() < 1e-12);
        assert_eq!(d.percentile(0.2), Some(0.0));
        // The 50th percentile is the 100th positive value (10.0), the 75th the
        // 200th (20.0); bins are 20 per decade, so within ~12 %.
        let close = |got: f64, want: f64| (got / want - 1.0).abs() < 0.13;
        assert!(close(d.percentile(0.5).unwrap(), 10.0), "{d:?}");
        assert!(close(d.percentile(0.75).unwrap(), 20.0));
        d.record(f32::INFINITY);
        d.record(1e-6);
        assert_eq!(d.count(), 402);
        let mut sum = d.clone();
        sum.merge(&d);
        assert_eq!(sum.count(), 804);
        assert_eq!(sum.zero, 200);
        assert!(close(
            sum.percentile(0.5).unwrap(),
            d.percentile(0.5).unwrap()
        ));
    }

    /// The positive-surplus percentile ignores the zeros: the criterion for
    /// the default sensitivity (#622) reads only agents that can be sated.
    #[test]
    fn positive_percentile_reads_only_samples_above_zero() {
        let mut d = SurplusDistribution::default();
        assert_eq!(d.positive_percentile(0.25), None);
        for _ in 0..1000 {
            d.record(0.0);
        }
        assert_eq!(d.positive_percentile(0.25), None);
        for i in 1..=400 {
            d.record(i as f32 * 0.1);
        }
        assert_eq!(d.percentile(0.25), Some(0.0));
        // The 100th of 400 positive values is 10.0.
        let got = d.positive_percentile(0.25).unwrap();
        assert!((got / 10.0 - 1.0).abs() < 0.13, "{got}");
    }

    /// Recording a [`Surplus`] also counts which side bound it: samples
    /// whose nutrient side is the lesser, and of the zeros, those held there
    /// by nutrient rather than by the buffer.
    #[test]
    fn surplus_distribution_splits_nutrient_limited_samples() {
        let mut d = SurplusDistribution::default();
        d.record_surplus(&Surplus {
            energy: 3.0,
            nutrient: 1.0,
        });
        d.record_surplus(&Surplus {
            energy: 3.0,
            nutrient: 0.0,
        });
        d.record_surplus(&Surplus {
            energy: -1.0,
            nutrient: 5.0,
        });
        d.record_surplus(&Surplus {
            energy: 2.0,
            nutrient: 5.0,
        });
        assert_eq!((d.count(), d.zero), (4, 2));
        assert_eq!((d.nutrient_limited, d.zero_nutrient_limited), (2, 1));
        let mut sum = d.clone();
        sum.merge(&d);
        assert_eq!((sum.nutrient_limited, sum.zero_nutrient_limited), (4, 2));
    }

    fn sated(ticks: f32, nutrient_limited: bool) -> Satiation {
        if nutrient_limited {
            Satiation {
                energy: ticks + 1.0,
                nutrient: ticks,
            }
        } else {
            Satiation {
                energy: ticks,
                nutrient: ticks + 1.0,
            }
        }
    }

    /// A reading with only drain-time satiation set (zero surplus).
    fn unread(satiation: Satiation) -> KillerReading {
        KillerReading::of(
            satiation,
            Surplus {
                energy: 0.0,
                nutrient: 0.0,
            },
        )
    }

    /// A pair lands in its kinship, trait-distance and satiation bands; the
    /// hungry share is the pairs whose grazer held under half-expression
    /// satiation, and merging adds tallies.
    #[test]
    fn grazer_hunger_bands_pairs_by_kinship_distance_and_satiation() {
        let mut h = GrazerHunger::default();
        h.record(true, 0.05, unread(sated(0.5, true)));
        h.record(true, 0.3, unread(sated(12.0, false)));
        h.record(false, 1.4, unread(sated(60.0, false)));
        assert_eq!(h.pairs[1][0][0], 1);
        assert_eq!(h.pairs[1][2][3], 1);
        assert_eq!(h.pairs[0][4][5], 1);
        assert_eq!(h.nutrient_limited[1][0], 1);
        assert_eq!(h.total(true), 2);
        assert_eq!(h.hungry(true), 1);
        assert_eq!(h.within(true, RECOGNITION_BANDS), 2);
        let mut sum = h;
        sum.merge(&h);
        assert_eq!(sum.total(true), 4);
    }

    /// A grazer whose pre-growth surplus is `s` ticks (energy-limited).
    fn with_surplus(s: f32) -> KillerReading {
        KillerReading::of(
            sated(30.0, false),
            Surplus {
                energy: s,
                nutrient: s + 1.0,
            },
        )
    }

    /// Each kill pair whose grazer held no pre-growth surplus is counted
    /// apart (#624); a negative energy side is zero surplus.
    #[test]
    fn grazer_hunger_counts_pairs_at_zero_surplus() {
        let mut h = GrazerHunger::default();
        for s in [0.0, 0.002, 1.0, -2.0] {
            h.record(true, 0.05, with_surplus(s));
        }
        h.record(false, 0.05, with_surplus(1.0));
        assert_eq!(h.surplus_zero, [0, 2]);
        let mut sum = h;
        sum.merge(&h);
        assert_eq!(sum.surplus_zero, [0, 4]);
    }

    /// Rows written before #624 carry no surplus or intake bands and read
    /// back empty; rows written before #684 carry the withdrawn gate's
    /// expression bands, which are ignored.
    #[test]
    fn grazer_hunger_reads_back_rows_from_before_and_after_the_gate() {
        let mut old = serde_json::to_value(GrazerHunger::default()).unwrap();
        let o = old.as_object_mut().unwrap();
        o.remove("surplus_zero");
        o.remove("intake_expression");
        o.insert(
            "expression".into(),
            serde_json::json!([[1, 2, 3, 4], [5, 6, 7, 8]]),
        );
        let back: GrazerHunger = serde_json::from_value(old).unwrap();
        assert_eq!(back, GrazerHunger::default());
    }

    /// The killer reading takes drain-time satiation off the grown roster
    /// and surplus off the metabolised roster, before growth spends it.
    #[test]
    fn killer_readings_read_surplus_on_the_pre_growth_roster() {
        let mut world = sample_31_world(1003);
        for _ in 0..60 {
            world.step();
        }
        let params = world.params().clone();
        let pre = PreStep::capture(&world);
        let readings = pre.killer_readings(&params);
        let metabolised = pre.metabolised_agents(&params);
        let drain_time = pre.drain_time_agents(&params);
        assert_eq!(readings.len(), metabolised.len());
        let mut grown_apart = 0;
        for (m, d) in metabolised.iter().zip(&drain_time) {
            let r = readings[&m.id];
            let want = Surplus::of(m, &params);
            assert_eq!(r.surplus, want);
            assert_eq!(r.satiation, Satiation::of(d, &params));
            grown_apart += usize::from(Surplus::of(d, &params) != want);
        }
        assert!(grown_apart > 0, "growth moves some agent's surplus");
    }

    /// The grow phase's earmark fill (the accounting's definition, #591):
    /// `(1 − κ)` of the mobilised reserve above the retention buffer, read
    /// off the metabolised roster — what moves into `repro_reserve`.
    #[test]
    fn earmark_fills_are_the_grow_phase_s_repro_share_of_mobilised_surplus() {
        let mut world = sample_31_world(1004);
        for _ in 0..60 {
            world.step();
        }
        let params = world.params().clone();
        let fills = PreStep::capture(&world).earmark_fills(&params);
        assert_eq!(fills.len(), world.agents().len());
        let mut positive = 0;
        for (a, fill) in &fills {
            let m = phase::metabolic_cost(a, &params);
            let excess = (a.reserve - params.growth_retention_multiplier * m).max(0.0);
            let want =
                excess * params.reserve_mobilisation_rate * (1.0 - a.traits.kappa.clamp(0.0, 1.0));
            assert!(
                (fill - want).abs() <= 1e-4 * want.max(1.0),
                "{fill} vs {want}"
            );
            positive += usize::from(*fill > 0.0);
        }
        assert!(positive > 0, "some agent fills its earmark");
    }

    #[test]
    fn trait_distances_band_and_average_pair_distances() {
        let a = TraitVector {
            photosynthetic_absorption: 1.0,
            heterotrophy: 0.0,
            mobility: 0.0,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 0.5,
            dispersal: 0.3,
        };
        let mut t = TraitDistances::default();
        t.record(
            &a,
            &TraitVector {
                heterotrophy: 0.05,
                ..a
            },
        );
        t.record(
            &a,
            &TraitVector {
                heterotrophy: 0.6,
                ..a
            },
        );
        assert_eq!(t.bands, [1, 0, 0, 1, 0]);
        assert!((t.mean().unwrap() - 0.325).abs() < 1e-6);
        assert!((t.rms().unwrap() - ((0.0025 + 0.36) / 2.0f64).sqrt()).abs() < 1e-6);
        assert_eq!(TraitDistances::default().mean(), None);
    }
}
