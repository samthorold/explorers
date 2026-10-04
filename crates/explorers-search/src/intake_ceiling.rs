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
//! The body-only window failed by outcome (#629), so the ceiling became
//! `C = m × (k_a + k_h × h_eff)` (#633). [`GateSample`] keeps, per relevant
//! agent-sample, everything that gate reads, so [`Region`] can evaluate both
//! outcomes (sated mixotrophs, producing consumers) over a `(k_a, k_h)` grid
//! after the run and [`region_report`] can print it (#634).
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
    /// Effective heterotrophy `h_eff` (wear included), which scales the
    /// ceiling's apparatus term (#634).
    pub h_eff: f32,
    /// Stoichiometric demand per unit structure, `ratio`.
    pub ratio: f32,
    /// The world's growth efficiency `η` (so `η · ratio` is
    /// `nutrient_per_energy`).
    pub growth_efficiency: f32,
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
                h_eff: eff_het(a),
                ratio: ratio(&a.traits),
                growth_efficiency: params.growth_efficiency,
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

    for (id, d) in realised_drains(agents, carcasses, params, events) {
        let r = readings.get_mut(&id).expect("every agent has a reading");
        r.drained_energy = d.energy;
        r.drained_nutrient = d.bound_nutrient;
    }
    readings
}

/// One consumer's realised drains in a tick, from its `Consumed` events.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Drained {
    /// Energy received: the structure drained times the transfer efficiency.
    pub energy: f32,
    /// The nutrient bound in what was drained, before the retention cap.
    pub bound_nutrient: f32,
    /// The nutrient retained: each bite's released nutrient capped at the
    /// consumer's stoichiometric demand on the energy it gained
    /// (`demand(traits, drain-time structure) × gained`), as the drain pass
    /// caps it (#637).
    pub retained_nutrient: f32,
}

/// Every consumer's [`Drained`] for a tick: `agents` is the drain-time
/// roster, `carcasses` the carcasses before the step, `events` the tick's
/// events. A carcass already spent of energy hands its nutrient to its
/// consumers in proportion to demand; that is split here by effective
/// heterotrophy, exact when the gate is off (expression 1).
pub fn realised_drains(
    agents: &[Agent],
    carcasses: &[Carcass],
    params: &WorldParameters,
    events: &[Event],
) -> HashMap<u64, Drained> {
    let mut out: HashMap<u64, Drained> = HashMap::new();
    for (e, b) in realised_bites(agents, carcasses, params, events) {
        let d = out.entry(e.source).or_default();
        d.energy += b.energy;
        d.bound_nutrient += b.bound_nutrient;
        d.retained_nutrient += b.retained_nutrient;
    }
    out
}

/// [`realised_drains`] bite by bite: each `Consumed` event whose consumer is
/// on the drain-time roster, with that one bite's [`Drained`].
pub fn realised_bites<'a>(
    agents: &[Agent],
    carcasses: &[Carcass],
    params: &WorldParameters,
    events: &'a [Event],
) -> Vec<(&'a Event, Drained)> {
    let k = params.wear_degradation_steepness;
    let eff_het = |a: &Agent| a.effective_trait_with_steepness(1, k);
    let ratio = |t: &TraitVector| explorers_sim::stoichiometric_demand(t, 1.0, params);
    let mut out: Vec<(&'a Event, Drained)> = Vec::new();
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
        let gained =
            drained * explorers_sim::trophic_transfer_efficiency(&c.traits, &target_traits, params);
        let need = explorers_sim::stoichiometric_demand(&c.traits, c.structure, params) * gained;
        out.push((
            e,
            Drained {
                energy: gained,
                bound_nutrient: nutrient,
                retained_nutrient: nutrient.min(need),
            },
        ));
    }
    out
}

/// One agent-sample at the intake gate's read point, compact and exact
/// (#634): everything the gate reads, so its expression and the sample's
/// gated intake can be evaluated at any ceiling `C = m × (k_a + k_h × h_eff)`
/// after the run. Energies are absolute (per tick); [`GateSample::ceiling_ticks`]
/// and the evaluation convert to ticks of maintenance.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GateSample {
    /// Effective heterotrophy `h_eff`.
    #[serde(rename = "h")]
    pub h_eff: f32,
    /// Maintenance `m`, the drain-time metabolic cost.
    #[serde(rename = "m")]
    pub maintenance: f32,
    /// Photosynthesis this tick.
    pub light: f32,
    /// Pool uptake this tick.
    pub uptake: f32,
    /// Stoichiometric demand per unit structure.
    pub ratio: f32,
    /// Growth efficiency `η`.
    #[serde(rename = "eta")]
    pub growth_efficiency: f32,
    /// Drain potential `P_E`.
    #[serde(rename = "pe")]
    pub potential_energy: f32,
    /// Drain potential `P_N`.
    #[serde(rename = "pn")]
    pub potential_nutrient: f32,
    /// Realised energy intake, light plus drain income.
    pub intake: f32,
}

impl GateSample {
    pub fn of(r: &IntakeReading) -> Self {
        Self {
            h_eff: r.h_eff,
            maintenance: r.maintenance,
            light: r.light,
            uptake: r.uptake,
            ratio: r.ratio,
            growth_efficiency: r.growth_efficiency,
            potential_energy: r.potential_energy,
            potential_nutrient: r.potential_nutrient,
            intake: r.light + r.drained_energy,
        }
    }

    /// The ceiling in ticks of maintenance, `k_a + k_h × h_eff`.
    pub fn ceiling_ticks(&self, k_a: f32, k_h: f32) -> f32 {
        k_a + k_h * self.h_eff
    }

    /// The reading the gate sees, rebuilt from the sample.
    fn reading(&self) -> IntakeReading {
        IntakeReading {
            maintenance: self.maintenance,
            light: self.light,
            uptake: self.uptake,
            drained_energy: self.intake - self.light,
            drained_nutrient: 0.0,
            potential_energy: self.potential_energy,
            potential_nutrient: self.potential_nutrient,
            nutrient_per_energy: self.growth_efficiency.max(0.0) * self.ratio,
            h_eff: self.h_eff,
            ratio: self.ratio,
            growth_efficiency: self.growth_efficiency,
        }
    }

    /// The gate's predicted expression at `(k_a, k_h)`: #629's
    /// [`IntakeReading::expression_at`] at ceiling multiple
    /// `k_a + k_h × h_eff`.
    pub fn expression(&self, k_a: f32, k_h: f32) -> f32 {
        self.reading().expression_at(self.ceiling_ticks(k_a, k_h))
    }
}

impl GateSample {
    /// Realised intake in ticks of maintenance, `x = intake / m`.
    pub fn intake_ticks(&self) -> f32 {
        self.intake / self.maintenance
    }

    /// The energy room in ticks of maintenance at `(k_a, k_h)`:
    /// `max(0, k_a + k_h·h_eff − light / m)`.
    pub fn room_ticks(&self, k_a: f32, k_h: f32) -> f32 {
        (self.ceiling_ticks(k_a, k_h) - self.light / self.maintenance).max(0.0)
    }

    /// The static gated intake in ticks of maintenance, `x·room / (x + room)`:
    /// the disk equation ([`predicted_expression`]) with the realised intake
    /// `x` as the potential (energy side).
    pub fn gated_intake_ticks(&self, k_a: f32, k_h: f32) -> f32 {
        let x = self.intake_ticks();
        x * predicted_expression((self.room_ticks(k_a, k_h), x), None)
    }
}

/// Outcome 1, sated mixotrophs (#634): the share of `kills` (kin-kill
/// samples of light-fed mixotroph killers, one per pair) whose predicted
/// expression at `(k_a, k_h)` is below one half. `None` without samples.
pub fn sated_share(kills: &[GateSample], k_a: f32, k_h: f32) -> Option<f64> {
    (!kills.is_empty()).then(|| {
        let sated = kills
            .iter()
            .filter(|s| s.expression(k_a, k_h) < 0.5)
            .count();
        sated as f64 / kills.len() as f64
    })
}

/// Outcome 2, producing consumers (#634), energy side: pooled production
/// kept, `Σ max(0, g − 1) / Σ max(0, x − 1)` over heterotroph-by-diet
/// samples, with `x` the realised intake and `g` the static gated intake
/// ([`GateSample::gated_intake_ticks`]), in ticks of maintenance. `None`
/// when no sample produces ungated.
pub fn production_kept(diet: &[GateSample], k_a: f32, k_h: f32) -> Option<f64> {
    let produced = |x: f32| f64::from((x - 1.0).max(0.0));
    let ungated: f64 = diet.iter().map(|s| produced(s.intake_ticks())).sum();
    let gated: f64 = diet
        .iter()
        .map(|s| produced(s.gated_intake_ticks(k_a, k_h)))
        .sum();
    (ungated > 0.0).then(|| gated / ungated)
}

/// The bar on sated mixotrophs: at least this share of the light-fed
/// mixotroph killers' kin-killing pairs below half expression.
pub const SATED_BAR: f64 = 0.60;
/// The bar on producing consumers: heterotrophs by diet keep at least this
/// share of their ungated production.
pub const KEPT_BAR: f64 = 0.75;

/// The per-sample records the `(k_a, k_h)` region is evaluated from (#634),
/// both read on the ungated world at the intake gate's read point:
///
/// - **`mixotroph_kin_kills`**: one sample per kin-killing (killer, victim)
///   pair whose killer is a light-fed mixotroph (income-role producer with
///   heterotrophy above zero, as #629's population), the killer read on the
///   tick of the kill;
/// - **`diet_fed`**: one per second-half agent-sample of a heterotroph by
///   diet, as #629's intake population.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GateSamples {
    #[serde(default)]
    pub mixotroph_kin_kills: Vec<GateSample>,
    #[serde(default)]
    pub diet_fed: Vec<GateSample>,
}

impl GateSamples {
    pub fn merge(&mut self, other: &GateSamples) {
        self.mixotroph_kin_kills
            .extend_from_slice(&other.mixotroph_kin_kills);
        self.diet_fed.extend_from_slice(&other.diet_fed);
    }
}

/// One `(k_a, k_h)` point of the region: both outcomes there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RegionCell {
    pub k_a: f32,
    pub k_h: f32,
    /// [`sated_share`].
    pub sated: Option<f64>,
    /// [`production_kept`].
    pub kept: Option<f64>,
}

impl RegionCell {
    /// The margin on both outcomes, `min(sated − 0.60, kept − 0.75)`:
    /// non-negative exactly where the point is feasible. `None` where an
    /// outcome has no samples.
    pub fn margin(&self) -> Option<f64> {
        Some((self.sated? - SATED_BAR).min(self.kept? - KEPT_BAR))
    }

    /// Whether both bars are met.
    pub fn feasible(&self) -> bool {
        self.margin().is_some_and(|m| m >= 0.0)
    }
}

/// Both outcomes over a `(k_a, k_h)` grid, rows `k_a`, columns `k_h`.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    pub k_a: Vec<f32>,
    pub k_h: Vec<f32>,
    /// Row-major: `cells[i * k_h.len() + j]` is `(k_a[i], k_h[j])`.
    pub cells: Vec<RegionCell>,
    /// Samples behind each outcome.
    pub n_kills: usize,
    pub n_diet: usize,
}

impl Region {
    pub fn evaluate(samples: &GateSamples, k_a: &[f32], k_h: &[f32]) -> Self {
        let n_kills = samples.mixotroph_kin_kills.len();
        let n_diet = samples.diet_fed.len();
        let cells = k_a
            .iter()
            .flat_map(|&a| {
                k_h.iter().map(move |&h| RegionCell {
                    k_a: a,
                    k_h: h,
                    sated: sated_share(&samples.mixotroph_kin_kills, a, h),
                    kept: production_kept(&samples.diet_fed, a, h),
                })
            })
            .collect();
        Self {
            k_a: k_a.to_vec(),
            k_h: k_h.to_vec(),
            cells,
            n_kills,
            n_diet,
        }
    }

    pub fn feasible_count(&self) -> usize {
        self.cells.iter().filter(|c| c.feasible()).count()
    }

    /// The feasible cell of largest margin (the first in grid order on a
    /// tie); `None` when the region is empty.
    pub fn default_point(&self) -> Option<RegionCell> {
        let mut best: Option<RegionCell> = None;
        for c in self.cells.iter().filter(|c| c.feasible()) {
            if best.is_none_or(|b| c.margin() > b.margin()) {
                best = Some(*c);
            }
        }
        best
    }
}

/// A grid value, at most three decimals, trailing zeros dropped.
fn grid_label(k: f32) -> String {
    let s = format!("{k:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn percent(v: Option<f64>, decimals: usize) -> String {
    v.map_or("–".to_string(), |v| format!("{:.*}", decimals, 100.0 * v))
}

/// The region as the census summary prints it (#634): a map of both
/// outcomes over the grid (cells `sated/kept` in %, `*` where feasible),
/// the feasible-cell count, the default with both outcomes and its margin
/// or "region EMPTY", and the `k_h = 0` column for comparison with #629.
pub fn region_report(r: &Region) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "Cells: sated / kept, in %. Sated: share of kin-killing pairs whose killer is a light-fed mixotroph with predicted E < 0.5 (bar ≥ {:.0} %, n = {} kin-kill pairs). Kept: heterotrophs by diet's production kept, energy side, static: Σ max(0, x·room/(x+room) − 1) / Σ max(0, x − 1), x = intake/m, room = max(0, k_a + k_h·h_eff − light/m) (bar ≥ {:.0} %, n = {} samples). * = feasible. Margin = min(sated − {SATED_BAR:.2}, kept − {KEPT_BAR:.2}).\n",
        100.0 * SATED_BAR,
        r.n_kills,
        100.0 * KEPT_BAR,
        r.n_diet
    );
    let heads: Vec<String> = r.k_h.iter().map(|&k| grid_label(k)).collect();
    let _ = writeln!(out, "| k_a \\ k_h | {} |", heads.join(" | "));
    let _ = writeln!(out, "|---|{}", "---:|".repeat(r.k_h.len()));
    for (i, &a) in r.k_a.iter().enumerate() {
        let row: Vec<String> = r.cells[i * r.k_h.len()..(i + 1) * r.k_h.len()]
            .iter()
            .map(|c| {
                format!(
                    "{}/{}{}",
                    percent(c.sated, 0),
                    percent(c.kept, 0),
                    if c.feasible() { "*" } else { "" }
                )
            })
            .collect();
        let _ = writeln!(out, "| {} | {} |", grid_label(a), row.join(" | "));
    }
    let _ = writeln!(
        out,
        "\nfeasible cells: {} of {}",
        r.feasible_count(),
        r.cells.len()
    );
    match r.default_point() {
        Some(d) => {
            let _ = writeln!(
                out,
                "default (largest margin): k_a = {}, k_h = {}: sated {} %, kept {} %, margin {:.3}",
                grid_label(d.k_a),
                grid_label(d.k_h),
                percent(d.sated, 1),
                percent(d.kept, 1),
                d.margin().unwrap_or(f64::NAN)
            );
        }
        None => {
            let best = r
                .cells
                .iter()
                .filter(|c| c.margin().is_some())
                .max_by(|a, b| a.margin().partial_cmp(&b.margin()).unwrap());
            let _ = write!(
                out,
                "region EMPTY: no (k_a, k_h) on the grid meets both bars"
            );
            if let Some(b) = best {
                let _ = write!(
                    out,
                    " (closest: k_a = {}, k_h = {}: sated {} %, kept {} %, margin {:.3})",
                    grid_label(b.k_a),
                    grid_label(b.k_h),
                    percent(b.sated, 1),
                    percent(b.kept, 1),
                    b.margin().unwrap_or(f64::NAN)
                );
            }
            let _ = writeln!(out);
        }
    }
    if let Some(j) = r.k_h.iter().position(|&k| k == 0.0) {
        let _ = writeln!(
            out,
            "\nk_h = 0 (the body-only ceiling of #629, k = k_a):\n\n| k_a | sated | kept |\n|---|---:|---:|"
        );
        for (i, &a) in r.k_a.iter().enumerate() {
            let c = r.cells[i * r.k_h.len() + j];
            let _ = writeln!(
                out,
                "| {} | {} % | {} % |",
                grid_label(a),
                percent(c.sated, 1),
                percent(c.kept, 1)
            );
        }
    }
    out
}

/// A geometric grid from `lo` to `hi` inclusive with at least `per_decade`
/// points per decade.
pub fn log_grid(lo: f32, hi: f32, per_decade: f32) -> Vec<f32> {
    let decades = (hi / lo).log10();
    let steps = (decades * per_decade).ceil().max(1.0) as usize;
    (0..=steps)
        .map(|i| lo * (hi / lo).powf(i as f32 / steps as f32))
        .collect()
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

    /// The gate sample (#634) keeps everything the intake gate reads at any
    /// `(k_a, k_h)`: a lone producer has no effective heterotrophy, nothing
    /// in reach, and its intake is its light; its ceiling is `k_a` ticks of
    /// maintenance whatever `k_h`.
    #[test]
    fn a_light_only_producer_samples_its_light_and_a_ceiling_of_k_a() {
        let params = params();
        let (readings, events, world) = one_tick(
            &params,
            vec![spec((10.0, 10.0), 50.0, traits(0.8, 0.0))],
            vec![],
        );
        let s = GateSample::of(&readings[&0]);
        let light = income(&events, 0, EventKind::Photosynthesized);
        assert!(light > 0.0);
        let a = &world.agents()[0];
        assert_eq!(s.h_eff, 0.0);
        assert_eq!((s.light, s.intake), (light, light));
        assert!(close(s.maintenance, phase::metabolic_cost(a, &params)));
        assert_eq!((s.potential_energy, s.potential_nutrient), (0.0, 0.0));
        assert_eq!(s.growth_efficiency, params.growth_efficiency);
        let ratio = explorers_sim::stoichiometric_demand(&a.traits, 1.0, &params);
        assert!(close(s.ratio, ratio), "{s:?}");
        assert_eq!(s.uptake, readings[&0].uptake);
        let k = 0.5 * s.light / s.maintenance;
        for k_h in [0.0, 3.0, 20.0] {
            assert_eq!(s.ceiling_ticks(k, k_h), k);
            assert_eq!(
                s.expression(k, k_h),
                readings[&0].expression_at(k),
                "the same gate"
            );
        }
    }

    /// A mixotroph with a little heterotrophy: its sample carries its
    /// effective heterotrophy, so its ceiling sits just above `k_a`, and its
    /// expression at `(k_a, k_h)` is the disk equation on that ceiling in
    /// both currencies.
    #[test]
    fn a_mixotroph_with_small_heterotrophy_has_a_ceiling_just_above_k_a() {
        let params = params();
        let mixo = traits(0.8, 0.05);
        let (readings, events, world) = one_tick(
            &params,
            vec![
                spec((10.0, 10.0), 50.0, mixo),
                spec((10.01, 10.0), 400.0, traits(1.0, 0.0)),
            ],
            vec![],
        );
        let s = GateSample::of(&readings[&0]);
        let a = &world.agents()[0];
        let h = a.effective_trait_with_steepness(1, params.wear_degradation_steepness);
        assert!(h > 0.0 && h < 0.1 && close(s.h_eff, h), "{s:?} vs {h}");
        let light = income(&events, 0, EventKind::Photosynthesized);
        assert!(
            s.potential_energy > 0.0 && s.intake > light && light > 0.0,
            "{s:?}"
        );
        let (k_a, k_h) = (1.0, 10.0);
        let c = k_a + k_h * s.h_eff;
        assert!(close(s.ceiling_ticks(k_a, k_h), c) && c > k_a && c < k_a + 1.0);
        let m = s.maintenance;
        let room_e = (c * m - s.light).max(0.0);
        let side = |room: f32, p: f32| if room > 0.0 { room / (room + p) } else { 0.0 };
        let room_n = (c * m * s.growth_efficiency * s.ratio - s.uptake).max(0.0);
        let want = side(room_e, s.potential_energy).max(side(room_n, s.potential_nutrient));
        assert!(close(s.expression(k_a, k_h), want), "{s:?}");
    }

    /// A pure consumer: no light, its intake is its drain income, and its
    /// ceiling is `k_a + k_h` ticks at full heterotrophy.
    #[test]
    fn a_pure_consumer_samples_its_drain_income_as_intake() {
        let params = params();
        let (readings, events, _) = one_tick(
            &params,
            vec![
                spec((10.0, 10.0), 50.0, traits(0.0, 1.0)),
                spec((10.01, 10.0), 400.0, traits(1.0, 0.0)),
            ],
            vec![],
        );
        let r = readings[&0];
        let s = GateSample::of(&r);
        assert!(income(&events, 0, EventKind::Consumed) > 0.0);
        assert_eq!(s.light, 0.0);
        assert!(s.intake > 0.0 && close(s.intake, r.drained_energy), "{s:?}");
        assert!(
            close(s.h_eff, 1.0),
            "a fresh founder carries no wear: {s:?}"
        );
        assert!(close(s.ceiling_ticks(2.0, 5.0), 7.0));
        let room = 7.0 * s.maintenance;
        let e_side = room / (room + s.potential_energy);
        assert!(s.expression(2.0, 5.0) >= e_side - 1e-6);
    }

    /// A well-lit mixotroph on a starved pool: where light fills the energy
    /// ceiling, the sample's nutrient side (`η · ratio`, uptake, `P_N`)
    /// keeps it feeding, exactly as the #629 reading does.
    #[test]
    fn a_nutrient_limited_sample_is_kept_hungry_by_its_nutrient_room() {
        let mut params = params();
        params.initial_nutrient_pool = 1e-3;
        let (readings, _, _) = one_tick(
            &params,
            vec![
                spec((10.0, 10.0), 50.0, traits(0.8, 0.3)),
                spec((10.01, 10.0), 400.0, traits(1.0, 0.0)),
            ],
            vec![],
        );
        let r = readings[&0];
        let s = GateSample::of(&r);
        let k_a = 0.5 * s.light / s.maintenance - 0.3 * s.h_eff;
        let (k_h, c) = (0.3, 0.5 * s.light / s.maintenance);
        assert!(k_a > 0.0 && close(s.ceiling_ticks(k_a, k_h), c), "{s:?}");
        let room_n = c * s.maintenance * s.growth_efficiency * s.ratio - s.uptake;
        assert!(room_n > 0.0 && s.potential_nutrient > 0.0, "{s:?}");
        let e = s.expression(k_a, k_h);
        assert!(close(e, room_n / (room_n + s.potential_nutrient)), "{e}");
        assert!(close(e, r.expression_at(c)), "{e}");
    }

    /// Where growth binds no nutrient the sample has no nutrient side: the
    /// gate reads energy alone.
    #[test]
    fn without_growth_efficiency_a_sample_reads_energy_alone() {
        let (readings, _, _) = one_tick_then(
            &params(),
            vec![
                spec((10.0, 10.0), 50.0, traits(0.5, 0.5)),
                spec((10.01, 10.0), 400.0, traits(1.0, 0.0)),
            ],
            vec![],
            |p| p.growth_efficiency = 0.0,
        );
        let s = GateSample::of(&readings[&0]);
        assert_eq!(s.growth_efficiency, 0.0);
        let c = 0.5 * s.light / s.maintenance;
        let k_a = c - 0.2 * s.h_eff;
        assert_eq!(s.expression(k_a, 0.2), 0.0, "light fills it: {s:?}");
        let (k_a, k_h) = (2.0 * s.light / s.maintenance, 1.0);
        let room = s.ceiling_ticks(k_a, k_h) * s.maintenance - s.light;
        let want = room / (room + s.potential_energy);
        assert!(close(s.expression(k_a, k_h), want));
    }

    /// A synthetic sample on the energy side alone (`η = 0`).
    fn gate(m: f32, h: f32, light: f32, p_e: f32, intake: f32) -> GateSample {
        GateSample {
            h_eff: h,
            maintenance: m,
            light,
            potential_energy: p_e,
            intake,
            ..Default::default()
        }
    }

    fn near(a: Option<f64>, b: f64) -> bool {
        a.is_some_and(|a| (a - b).abs() < 1e-5)
    }

    /// Both outcomes against hand-computed values. Sated mixotrophs: the
    /// share of kin-kill samples predicted below half expression. Producing
    /// consumers: pooled gated production `max(0, x·room/(x+room) − 1)`
    /// over ungated `max(0, x − 1)`, with `x = intake / m` and the sample's
    /// own `room = max(0, k_a + k_h·h_eff − light / m)`.
    #[test]
    fn both_outcomes_match_hand_computed_values() {
        let kills = [
            // c = 2: room 0, E = 0. c = 3: room 1, E = 1 / 1.5.
            gate(1.0, 0.1, 2.0, 0.5, 2.5),
            // c = 2: room 1, E = 0.5, not below half.
            gate(1.0, 0.0, 1.0, 1.0, 1.0),
        ];
        assert!(near(sated_share(&kills, 2.0, 0.0), 0.5));
        assert!(near(sated_share(&kills, 2.0, 10.0), 0.0));
        assert_eq!(sated_share(&[], 2.0, 0.0), None);
        let diet = [
            // x = 4, ungated production 3.
            gate(1.0, 1.0, 0.0, 4.0, 4.0),
            // x = 1: no production either way.
            gate(2.0, 1.0, 0.0, 2.0, 2.0),
            // x = 3 with light 0.5: room shrinks by the light.
            gate(1.0, 0.5, 0.5, 2.5, 3.0),
        ];
        // (2, 0): rooms 2, 2, 1.5 → gated 4/3, 2/3, 4.5/4.5 = 1.
        let ungated = 3.0 + 2.0;
        let gated = 4.0 * 2.0 / 6.0 - 1.0;
        assert!(near(production_kept(&diet, 2.0, 0.0), gated / ungated));
        // (2, 10): rooms 12, 12, 6.5 → gated 3, 12/13, 19.5/9.5.
        let gated = 2.0 + (19.5 / 9.5 - 1.0);
        assert!(near(production_kept(&diet, 2.0, 10.0), gated / ungated));
        assert_eq!(
            production_kept(&diet[1..2], 2.0, 0.0),
            None,
            "nothing to keep"
        );
    }

    /// The two populations of the region tests: one killer with a little
    /// heterotrophy that light nearly fills, one consumer eating 4 ticks.
    fn region_samples() -> GateSamples {
        GateSamples {
            mixotroph_kin_kills: vec![gate(1.0, 0.01, 2.0, 0.5, 2.5)],
            diet_fed: vec![gate(1.0, 1.0, 0.0, 4.0, 4.0)],
        }
    }

    /// Feasible cells meet both bars (sated ≥ 60 %, production kept
    /// ≥ 75 %); the default is the feasible cell of largest margin
    /// `min(sated − 0.60, kept − 0.75)`. Hand-computed: the killer is sated
    /// wherever `c = k_a + 0.01·k_h < 3`; the consumer keeps
    /// `(4·c/(4 + c) − 1) / 3`, 0.787 at c = 21 and 0.795 at c = 22.
    #[test]
    fn the_region_is_the_feasible_cells_and_the_default_its_largest_margin() {
        let r = Region::evaluate(&region_samples(), &[1.0, 2.0], &[0.0, 10.0, 20.0]);
        let cell = |k_a: f32, k_h: f32| {
            *r.cells
                .iter()
                .find(|c| c.k_a == k_a && c.k_h == k_h)
                .unwrap()
        };
        assert!(near(cell(2.0, 10.0).sated, 1.0));
        assert!(near(cell(2.0, 10.0).kept, 2.0 / 3.0));
        assert!(!cell(2.0, 10.0).feasible());
        assert!(near(cell(1.0, 20.0).kept, (84.0 / 25.0 - 1.0) / 3.0));
        assert!(cell(1.0, 20.0).feasible());
        assert_eq!(r.feasible_count(), 2);
        let d = r.default_point().unwrap();
        assert_eq!((d.k_a, d.k_h), (2.0, 20.0));
        let margin = (88.0 / 26.0 - 1.0) / 3.0 - 0.75;
        assert!(near(d.margin(), margin), "{d:?}");

        let empty = Region::evaluate(&region_samples(), &[1.0, 2.0], &[0.0, 10.0]);
        assert_eq!(empty.feasible_count(), 0);
        assert_eq!(empty.default_point(), None);
    }

    /// The log grid spans its ends at no fewer points per decade than asked.
    #[test]
    fn the_log_grid_spans_its_ends_geometrically() {
        let g = log_grid(0.25, 20.0, 8.0);
        assert_eq!(g.len(), 17);
        assert!(
            (g[0] - 0.25).abs() < 1e-6 && (g[16] - 20.0).abs() < 1e-4,
            "{g:?}"
        );
        let step = g[1] / g[0];
        assert!(g.windows(2).all(|w| (w[1] / w[0] - step).abs() < 1e-4));
        assert!(step <= 10f32.powf(1.0 / 8.0));
    }

    /// At `k_h = 0` the ceiling is #629's body-only `k × m`: the predicted
    /// expression is #629's for the same inputs, and a lightless consumer's
    /// gated intake is #629's static `x·k / (x + k)`.
    #[test]
    fn at_k_h_zero_the_gate_reduces_to_the_body_only_ceiling() {
        for (light, drained, uptake) in [(1.0, 0.0, 0.2), (3.0, 2.0, 0.0), (0.5, 6.0, 1.5)] {
            let mut r = reading(light, drained, uptake);
            r.potential_energy = 1.5;
            r.potential_nutrient = 0.4;
            r.h_eff = 0.7;
            r.ratio = 0.25;
            r.growth_efficiency = 2.0;
            let s = GateSample::of(&r);
            for k in [0.25, 1.0, 1.704, 5.0, 20.0] {
                assert_eq!(s.expression(k, 0.0), r.expression_at(k), "{r:?} at {k}");
            }
        }
        let s = gate(2.0, 0.9, 0.0, 6.0, 6.0);
        for k in [1.7_f32, 5.0, 10.0] {
            let x = 3.0;
            assert!(close(s.gated_intake_ticks(k, 0.0), x * k / (x + k)));
        }
    }

    /// The report maps both outcomes over the grid, marks feasible cells,
    /// names the default with both outcomes and its margin, or says plainly
    /// that the region is empty; it repeats the `k_h = 0` column for #629.
    #[test]
    fn the_region_report_names_the_default_or_says_empty() {
        let r = Region::evaluate(&region_samples(), &[1.0, 2.0], &[0.0, 10.0, 20.0]);
        let text = region_report(&r);
        assert!(text.contains("| k_a \\ k_h | 0 | 10 | 20 |"), "{text}");
        assert!(text.contains("| 2 | 100/11 | 100/67 | 100/79* |"), "{text}");
        assert!(text.contains("feasible cells: 2 of 6"), "{text}");
        assert!(
            text.contains(
                "default (largest margin): k_a = 2, k_h = 20: sated 100.0 %, kept 79.5 %, margin 0.045"
            ),
            "{text}"
        );
        assert!(text.contains("n = 1 kin-kill pairs") && text.contains("n = 1 samples"));
        assert!(
            text.contains("| 1 | 100.0 % | 0.0 % |"),
            "k_h = 0 column: {text}"
        );
        assert!(!text.contains("region EMPTY"));

        let empty = Region::evaluate(&region_samples(), &[1.0, 2.0], &[0.0, 10.0]);
        let text = region_report(&empty);
        assert!(text.contains("region EMPTY"), "{text}");
        assert!(!text.contains("default (largest margin)"), "{text}");
    }
}
