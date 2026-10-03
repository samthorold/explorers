//! The fullness-gate region (issue #637).
//!
//! The intake gate reads each consumer's **fullness**, a moving average of
//! its intake per tick in each currency, against its intake ceiling
//! `C = k × maintenance` (world rules, *Capability and expression are
//! decoupled*; #636). Fullness depends only on the clearance time `τ`, never
//! on `k` or the exponent `n`, so on the ungated world each agent carries a
//! [`FullnessBank`]: one fullness per currency for each `τ` of a fixed grid.
//!
//! Observer-side only: the world is never touched.

use std::collections::HashMap;

use explorers_sim::event::{Event, EventKind};
use explorers_sim::{WorldParameters, phase};

use crate::grazer_hunger::{PreStep, SurplusDistribution};
use crate::intake_ceiling::{KEPT_BAR, SATED_BAR, log_grid, realised_drains};

/// One agent's intake in a tick, as the gate books it after drains, with
/// what its ceiling is read against.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TickIntake {
    /// Drain-time metabolic cost `m`.
    pub maintenance: f32,
    /// Photosynthesis this tick.
    pub light: f32,
    /// Pool uptake this tick.
    pub uptake: f32,
    /// Drain energy received this tick.
    pub drained_energy: f32,
    /// Drain nutrient retained this tick.
    pub retained_nutrient: f32,
    /// `η · ratio`: the nutrient ceiling is `C × η · ratio`. Zero when growth
    /// binds no nutrient, where the gate reads energy alone.
    pub nutrient_per_energy: f32,
}

impl TickIntake {
    /// Energy intake: light plus drain energy received.
    pub fn energy(&self) -> f32 {
        self.light + self.drained_energy
    }

    /// Nutrient intake: uptake plus drain nutrient retained.
    pub fn nutrient(&self) -> f32 {
        self.uptake + self.retained_nutrient
    }
}

/// Every drain-time agent's [`TickIntake`] for the tick that `pre`
/// captured, with `events` that tick's events (its `Consumed` events give
/// the drains).
pub fn tick_intakes(
    pre: &PreStep,
    params: &WorldParameters,
    events: &[Event],
) -> HashMap<u64, TickIntake> {
    let start = pre.drain_start(params);
    let drains = realised_drains(&start.agents, pre.carcasses(), params, events);
    start
        .agents
        .iter()
        .map(|a| {
            let d = drains.get(&a.id).copied().unwrap_or_default();
            let intake = TickIntake {
                maintenance: phase::metabolic_cost(a, params).max(0.0),
                light: start.light.get(&a.id).copied().unwrap_or(0.0),
                uptake: start.uptake.get(&a.id).copied().unwrap_or(0.0),
                drained_energy: d.energy,
                retained_nutrient: d.retained_nutrient,
                nutrient_per_energy: params.growth_efficiency.max(0.0)
                    * explorers_sim::stoichiometric_demand(&a.traits, 1.0, params),
            };
            (a.id, intake)
        })
        .collect()
}

/// The clearance times `τ` (ticks) the bank carries: [`TAU_POINTS`] values,
/// log-spaced over [1, 50].
pub const TAU_POINTS: usize = 12;

/// The `τ` grid, geometric from 1 to 50 inclusive.
pub fn tau_grid() -> [f32; TAU_POINTS] {
    let mut out = [0.0; TAU_POINTS];
    for (i, t) in out.iter_mut().enumerate() {
        *t = 50f32.powf(i as f32 / (TAU_POINTS - 1) as f32);
    }
    out[0] = 1.0;
    out[TAU_POINTS - 1] = 50.0;
    out
}

/// One agent's fullness in both currencies at every `τ` of [`tau_grid`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FullnessBank {
    pub energy: [f32; TAU_POINTS],
    pub nutrient: [f32; TAU_POINTS],
}

impl FullnessBank {
    /// A bank at a steady intake: every `τ` reads `energy` and `nutrient`.
    pub fn at(energy: f32, nutrient: f32) -> Self {
        Self {
            energy: [energy; TAU_POINTS],
            nutrient: [nutrient; TAU_POINTS],
        }
    }

    /// One tick's update, `F ← F + (intake − F) / τ`, at every `τ`.
    pub fn update(&mut self, energy: f32, nutrient: f32) {
        for (j, tau) in tau_grid().iter().enumerate() {
            self.energy[j] += (energy - self.energy[j]) / tau;
            self.nutrient[j] += (nutrient - self.nutrient[j]) / tau;
        }
    }
}

/// The fullness gate's expression for one currency,
/// `1 / (1 + (F / C)^n)`. A ceiling of zero leaves no room: any fullness
/// expresses nothing, and none expresses fully.
pub fn side_expression(fullness: f32, ceiling: f32, n: f32) -> f32 {
    if ceiling <= 0.0 {
        return if fullness > 0.0 { 0.0 } else { 1.0 };
    }
    1.0 / (1.0 + (fullness.max(0.0) / ceiling).powf(n))
}

/// A consumer's expression at one `(k, τ, n)` point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Expression {
    /// `E = max(E_E, E_N)`; `E_E` alone where growth binds no nutrient.
    pub co_limited: f32,
    /// `E_E`, the energy side alone.
    pub energy: f32,
}

impl FullnessBank {
    /// The gate's expression from this bank at `τ` index `j`, ceiling
    /// multiple `k` and exponent `n`, with `intake` giving the agent's
    /// maintenance and `η · ratio`: ceilings `C = k·m` and `C·η·ratio`.
    pub fn expression(&self, j: usize, k: f32, n: f32, intake: &TickIntake) -> Expression {
        let ceiling = k * intake.maintenance;
        let energy = side_expression(self.energy[j], ceiling, n);
        let co_limited = if intake.nutrient_per_energy > 0.0 {
            let nutrient =
                side_expression(self.nutrient[j], ceiling * intake.nutrient_per_energy, n);
            energy.max(nutrient)
        } else {
            energy
        };
        Expression { co_limited, energy }
    }
}

/// The ceiling multiples `k` of the grid: [0.25, 20] at ≥ 8 points per
/// decade (17 values), as #634's `k_a`.
pub fn k_grid() -> Vec<f32> {
    log_grid(0.25, 20.0, 8.0)
}

/// The exponents the region is read at; the design fixes 4.
pub const EXPONENTS: [f32; 3] = [2.0, 4.0, 8.0];

/// Agents' lifetime mean intake per tick in ticks of maintenance,
/// `mean(intake_E / m)`, one value per agent (#637's separation check).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AgentIntake {
    pub agents: u64,
    /// Sum of the agents' means.
    pub sum: f64,
    pub distribution: SurplusDistribution,
}

impl AgentIntake {
    fn record(&mut self, mean: f64) {
        self.agents += 1;
        self.sum += mean;
        self.distribution.record(mean as f32);
    }

    /// The mean over agents of their lifetime mean.
    pub fn mean(&self) -> Option<f64> {
        (self.agents > 0).then(|| self.sum / self.agents as f64)
    }

    pub fn merge(&mut self, other: &AgentIntake) {
        self.agents += other.agents;
        self.sum += other.sum;
        self.distribution.merge(&other.distribution);
    }
}

/// The fullness gate's outcomes over a fixed `(n, k, τ)` grid, accumulated
/// online over one seed (#637). Pooling across seeds is a sum
/// ([`FullnessGrid::merge`]).
///
/// - **Sated mixotrophs:** at each kin kill by a light-fed mixotroph (one per
///   pair, #634's classification), the kill is counted, and per `(k, τ)`
///   whether the killer's expression from the fullness carried into the tick
///   is below one half (`sated`), and the same on the energy side alone
///   (`sated_energy`). `E < 1/2` exactly when the fullness is past the
///   ceiling on both sides, whatever `n`, so these carry no `n` axis.
/// - **Producing consumers:** on each tick of each heterotroph by diet in
///   the second-half window, `max(0, x − 1)` is added to `produced` and, per
///   `(n, k, τ)`, `max(0, x_g − 1)` to `produced_gated`, with
///   `x = (light + drain energy) / m` and `x_g` the same with the drain
///   energy scaled by the cell's `E`.
/// - **Separation:** each population's agents' lifetime mean intake / m.
///
/// Layouts: `sated[i·T + j]` is `(k[i], τ[j])`;
/// `produced_gated[(e·K + i)·T + j]` is `(exponents[e], k[i], τ[j])`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FullnessGrid {
    pub k: Vec<f32>,
    pub tau: Vec<f32>,
    pub exponents: Vec<f32>,
    pub kills: u64,
    pub sated: Vec<u64>,
    pub sated_energy: Vec<u64>,
    pub consumer_ticks: u64,
    pub produced: f64,
    pub produced_gated: Vec<f64>,
    /// Light-fed mixotroph kin killers (agents with at least one such kill).
    pub killers: AgentIntake,
    /// Heterotrophs by diet (agents with at least one such tick in the
    /// window).
    pub consumers: AgentIntake,
}

impl FullnessGrid {
    /// An empty grid on [`k_grid`], [`tau_grid`] and [`EXPONENTS`].
    pub fn new() -> Self {
        let (k, tau) = (k_grid(), tau_grid().to_vec());
        let cells = k.len() * tau.len();
        Self {
            sated: vec![0; cells],
            sated_energy: vec![0; cells],
            produced_gated: vec![0.0; EXPONENTS.len() * cells],
            k,
            tau,
            exponents: EXPONENTS.to_vec(),
            ..Default::default()
        }
    }

    /// The flat index of `(k[i], τ[j])`.
    pub fn cell(&self, i: usize, j: usize) -> usize {
        i * self.tau.len() + j
    }

    /// The flat index of `(exponents[e], k[i], τ[j])`.
    pub fn gated_cell(&self, e: usize, i: usize, j: usize) -> usize {
        (e * self.k.len() + i) * self.tau.len() + j
    }

    fn record_kill(&mut self, bank: &FullnessBank, intake: &TickIntake) {
        self.kills += 1;
        for i in 0..self.k.len() {
            for j in 0..self.tau.len() {
                let e = bank.expression(j, self.k[i], 4.0, intake);
                let c = self.cell(i, j);
                self.sated[c] += u64::from(e.co_limited < 0.5);
                self.sated_energy[c] += u64::from(e.energy < 0.5);
            }
        }
    }

    fn record_consumer_tick(&mut self, bank: &FullnessBank, intake: &TickIntake) {
        let m = intake.maintenance;
        if m <= 0.0 {
            return;
        }
        self.consumer_ticks += 1;
        let produced = |x: f32| f64::from((x - 1.0).max(0.0));
        self.produced += produced(intake.energy() / m);
        for e in 0..self.exponents.len() {
            for i in 0..self.k.len() {
                for j in 0..self.tau.len() {
                    let ex = bank.expression(j, self.k[i], self.exponents[e], intake);
                    let x_g = (intake.light + ex.co_limited * intake.drained_energy) / m;
                    let c = self.gated_cell(e, i, j);
                    self.produced_gated[c] += produced(x_g);
                }
            }
        }
    }

    /// Pool another seed's grid: every counter is summed. An empty grid
    /// (an old row's) adds nothing.
    pub fn merge(&mut self, other: &FullnessGrid) {
        if other.k.is_empty() {
            return;
        }
        if self.k.is_empty() {
            *self = other.clone();
            return;
        }
        assert!(
            self.k == other.k && self.tau == other.tau && self.exponents == other.exponents,
            "pooled fullness grids must share their axes"
        );
        self.kills += other.kills;
        self.consumer_ticks += other.consumer_ticks;
        self.produced += other.produced;
        for (a, b) in self.sated.iter_mut().zip(&other.sated) {
            *a += b;
        }
        for (a, b) in self.sated_energy.iter_mut().zip(&other.sated_energy) {
            *a += b;
        }
        for (a, b) in self.produced_gated.iter_mut().zip(&other.produced_gated) {
            *a += b;
        }
        self.killers.merge(&other.killers);
        self.consumers.merge(&other.consumers);
    }
}

/// One agent as the tracker follows it.
#[derive(Clone, Debug)]
struct Track {
    bank: FullnessBank,
    /// Σ intake_E / m over its ticks, and their count.
    intake_ticks: f64,
    ticks: u64,
    killer: bool,
    consumer: bool,
}

impl Track {
    fn new(bank: FullnessBank) -> Self {
        Self {
            bank,
            intake_ticks: 0.0,
            ticks: 0,
            killer: false,
            consumer: false,
        }
    }
}

/// The fullness readout, online over one ungated run (#637): every living
/// agent's [`FullnessBank`], updated each tick from that tick's intake after
/// drains exactly as the gate would, and the [`FullnessGrid`] accumulated
/// from it. A founder's bank starts at its first tick's light and uptake; a
/// newborn's is a copy of its parent's.
#[derive(Clone, Debug)]
pub struct FullnessTracker {
    agents: HashMap<u64, Track>,
    grid: FullnessGrid,
}

impl Default for FullnessTracker {
    fn default() -> Self {
        Self {
            agents: HashMap::new(),
            grid: FullnessGrid::new(),
        }
    }
}

impl FullnessTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// An agent's bank as carried into the next tick.
    pub fn bank(&self, id: u64) -> Option<&FullnessBank> {
        self.agents.get(&id).map(|t| &t.bank)
    }

    /// Book one tick: `intakes` from [`tick_intakes`], `events` the tick's
    /// events (its `Born` and `Died` events give births and deaths),
    /// `kin_kills` the killer of each kin-killing pair whose killer is a
    /// light-fed mixotroph, `consumers` the heterotrophs by diet sampled this
    /// tick. Kills and consumer ticks are read from the fullness carried into
    /// the tick; the banks are then updated from the tick's intake.
    pub fn observe(
        &mut self,
        intakes: &HashMap<u64, TickIntake>,
        events: &[Event],
        kin_kills: &[u64],
        consumers: &[u64],
    ) {
        for (id, i) in intakes {
            self.agents
                .entry(*id)
                .or_insert_with(|| Track::new(FullnessBank::at(i.light, i.uptake)));
        }
        for g in kin_kills {
            if let (Some(t), Some(i)) = (self.agents.get_mut(g), intakes.get(g)) {
                t.killer = true;
                self.grid.record_kill(&t.bank, i);
            }
        }
        for c in consumers {
            if let (Some(t), Some(i)) = (self.agents.get_mut(c), intakes.get(c)) {
                t.consumer = true;
                self.grid.record_consumer_tick(&t.bank, i);
            }
        }
        for (id, i) in intakes {
            if let Some(t) = self.agents.get_mut(id) {
                t.bank.update(i.energy(), i.nutrient());
                if i.maintenance > 0.0 {
                    t.intake_ticks += f64::from(i.energy() / i.maintenance);
                    t.ticks += 1;
                }
            }
        }
        for e in events {
            if e.kind == EventKind::Born
                && let Some(parent) = e.target.and_then(|p| self.agents.get(&p))
            {
                let track = Track::new(parent.bank);
                self.agents.insert(e.source, track);
            }
        }
        for e in events.iter().filter(|e| e.kind == EventKind::Died) {
            if let Some(t) = self.agents.remove(&e.source) {
                self.retire(&t);
            }
        }
    }

    fn retire(&mut self, t: &Track) {
        if t.ticks == 0 {
            return;
        }
        let mean = t.intake_ticks / t.ticks as f64;
        if t.killer {
            self.grid.killers.record(mean);
        }
        if t.consumer {
            self.grid.consumers.record(mean);
        }
    }

    /// The run's grid, with every agent still alive retired at its
    /// lifetime so far.
    pub fn finish(mut self) -> FullnessGrid {
        let mut ids: Vec<u64> = self.agents.keys().copied().collect();
        ids.sort_unstable();
        for id in ids {
            let t = self.agents.remove(&id).expect("listed");
            self.retire(&t);
        }
        self.grid
    }
}

/// One `(k, τ)` point at one exponent: both outcomes there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FullnessCell {
    pub k: f32,
    pub tau: f32,
    /// Share of the kills sated (`E < 0.5`); `None` without kills.
    pub sated: Option<f64>,
    /// The same on the energy side alone (`E_E < 0.5`).
    pub sated_energy: Option<f64>,
    /// Production kept, `Σ max(0, x_g − 1) / Σ max(0, x − 1)`; `None` when
    /// nothing is produced ungated.
    pub kept: Option<f64>,
}

impl FullnessCell {
    /// `min(sated − 0.60, kept − 0.75)`: non-negative exactly where feasible.
    pub fn margin(&self) -> Option<f64> {
        Some((self.sated? - SATED_BAR).min(self.kept? - KEPT_BAR))
    }

    pub fn feasible(&self) -> bool {
        self.margin().is_some_and(|m| m >= 0.0)
    }
}

/// Both outcomes over the `(k, τ)` grid at one exponent, rows `k`, columns
/// `τ` (`cells[i·T + j]`).
#[derive(Clone, Debug, PartialEq)]
pub struct FullnessRegion {
    pub exponent: f32,
    pub k: Vec<f32>,
    pub tau: Vec<f32>,
    pub cells: Vec<FullnessCell>,
}

impl FullnessRegion {
    /// The region at `exponents[e]` from pooled counters.
    pub fn evaluate(g: &FullnessGrid, e: usize) -> Self {
        let share = |n: u64| (g.kills > 0).then(|| n as f64 / g.kills as f64);
        let mut cells = Vec::with_capacity(g.k.len() * g.tau.len());
        for (i, &k) in g.k.iter().enumerate() {
            for (j, &tau) in g.tau.iter().enumerate() {
                let c = g.cell(i, j);
                cells.push(FullnessCell {
                    k,
                    tau,
                    sated: share(g.sated[c]),
                    sated_energy: share(g.sated_energy[c]),
                    kept: (g.produced > 0.0)
                        .then(|| g.produced_gated[g.gated_cell(e, i, j)] / g.produced),
                });
            }
        }
        Self {
            exponent: g.exponents[e],
            k: g.k.clone(),
            tau: g.tau.clone(),
            cells,
        }
    }

    pub fn feasible_count(&self) -> usize {
        self.cells.iter().filter(|c| c.feasible()).count()
    }

    /// The feasible cell of largest margin (the first in grid order on a
    /// tie); `None` when the region is empty.
    pub fn default_point(&self) -> Option<FullnessCell> {
        let mut best: Option<FullnessCell> = None;
        for c in self.cells.iter().filter(|c| c.feasible()) {
            if best.is_none_or(|b| c.margin() > b.margin()) {
                best = Some(*c);
            }
        }
        best
    }

    /// The cell of largest margin, feasible or not.
    pub fn closest(&self) -> Option<FullnessCell> {
        let mut best: Option<FullnessCell> = None;
        for c in self.cells.iter().filter(|c| c.margin().is_some()) {
            if best.is_none_or(|b| c.margin() > b.margin()) {
                best = Some(*c);
            }
        }
        best
    }
}

fn label(v: f32) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn percent(v: Option<f64>, decimals: usize) -> String {
    v.map_or("–".to_string(), |v| format!("{:.*}", decimals, 100.0 * v))
}

fn describe(c: &FullnessCell) -> String {
    format!(
        "k = {}, τ = {}: sated {} % (energy side alone {} %), kept {} %, margin {:.3}",
        label(c.k),
        label(c.tau),
        percent(c.sated, 1),
        percent(c.sated_energy, 1),
        percent(c.kept, 1),
        c.margin().unwrap_or(f64::NAN)
    )
}

/// The fullness-gate region as the census summary prints it (#637): per
/// exponent, the verdict line first (the largest-margin default, or "region
/// EMPTY" with the closest cell), then the map (cells `sated/kept` in %,
/// `*` where feasible); then the energy-alone sated map (`n`-free), and
/// each population's per-agent lifetime mean intake / m.
pub fn fullness_report(g: &FullnessGrid) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "Fullness F ← F + (intake − F)/τ per currency, ceiling C = k·m (nutrient C·η·ratio), E = max(E_E, E_N), E_x = 1/(1 + (F_x/C_x)^n), read from the fullness carried into the tick, on the ungated world. Sated: share of kin-killing pairs whose killer is a light-fed mixotroph with E < 0.5 (bar ≥ {:.0} %, n = {} kills). Kept: heterotrophs by diet, every second-half tick, Σ max(0, x_g − 1) / Σ max(0, x − 1), x = (light + drain energy)/m, x_g with the drain scaled by E (bar ≥ {:.0} %, {} consumer ticks). Static: a gated consumer's later recouping is not seen. Margin = min(sated − {SATED_BAR:.2}, kept − {KEPT_BAR:.2}).",
        100.0 * SATED_BAR,
        g.kills,
        100.0 * KEPT_BAR,
        g.consumer_ticks
    );
    let heads: Vec<String> = g.tau.iter().map(|&t| label(t)).collect();
    for e in 0..g.exponents.len() {
        let r = FullnessRegion::evaluate(g, e);
        let n = label(r.exponent);
        let _ = writeln!(out, "\n### n = {n}\n");
        match r.default_point() {
            Some(d) => {
                let _ = writeln!(
                    out,
                    "feasible cells: {} of {}; default (largest margin) {}",
                    r.feasible_count(),
                    r.cells.len(),
                    describe(&d)
                );
            }
            None => {
                let _ = write!(
                    out,
                    "region EMPTY at n = {n}: no (k, τ) on the grid meets both bars"
                );
                if let Some(c) = r.closest() {
                    let _ = write!(out, " (closest: {})", describe(&c));
                }
                let _ = writeln!(out);
            }
        }
        let _ = writeln!(out, "\n| k \\ τ | {} |", heads.join(" | "));
        let _ = writeln!(out, "|---|{}", "---:|".repeat(g.tau.len()));
        for (i, &k) in r.k.iter().enumerate() {
            let row: Vec<String> = r.cells[i * g.tau.len()..(i + 1) * g.tau.len()]
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
            let _ = writeln!(out, "| {} | {} |", label(k), row.join(" | "));
        }
    }
    if !g.exponents.is_empty() {
        let r = FullnessRegion::evaluate(g, 0);
        let _ = writeln!(
            out,
            "\nSated on the energy side alone (E_E < 0.5), % of kills; n-free:\n\n| k \\ τ | {} |\n|---|{}",
            heads.join(" | "),
            "---:|".repeat(g.tau.len())
        );
        for (i, &k) in r.k.iter().enumerate() {
            let row: Vec<String> = r.cells[i * g.tau.len()..(i + 1) * g.tau.len()]
                .iter()
                .map(|c| percent(c.sated_energy, 0))
                .collect();
            let _ = writeln!(out, "| {} | {} |", label(k), row.join(" | "));
        }
    }
    let _ = writeln!(
        out,
        "\nPer-agent lifetime mean intake / m (energy: light + drain energy received):\n\n| population | agents | p10 | p25 | median | p75 | p90 | mean |\n|---|---:|---:|---:|---:|---:|---:|---:|"
    );
    let fmt = |v: Option<f64>| v.map_or("–".to_string(), |x| format!("{x:.3}"));
    for (name, a) in [
        ("light-fed mixotroph kin killers", &g.killers),
        ("heterotrophs by diet", &g.consumers),
    ] {
        let d = &a.distribution;
        let _ = writeln!(
            out,
            "| {name} | {} | {} | {} | {} | {} | {} | {} |",
            a.agents,
            fmt(d.percentile(0.1)),
            fmt(d.percentile(0.25)),
            fmt(d.percentile(0.5)),
            fmt(d.percentile(0.75)),
            fmt(d.percentile(0.9)),
            fmt(a.mean())
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_source::{ConfigSource, resolve_config, sampled_units};
    use crate::search::default_ranges;
    use explorers_sim::{AgentSpec, TraitVector, World, WorldRecipe};

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

    /// `sample:31`'s physics, ungated, recognition off.
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

    fn world(params: &WorldParameters, agents: Vec<AgentSpec>) -> World {
        World::from_recipe(
            &WorldRecipe {
                parameters: params.clone(),
                initial_distribution: None,
                agents: Some(agents),
                carcasses: Some(vec![]),
                max_ticks: 10,
            },
            7,
        )
    }

    /// Step `world` once: the tick's intakes and events.
    fn step(world: &mut World) -> (HashMap<u64, TickIntake>, Vec<Event>) {
        let params = world.params().clone();
        let pre = PreStep::capture(world);
        let cursor = world.event_log().len();
        world.step();
        let events = world.event_log().since(cursor).to_vec();
        (tick_intakes(&pre, &params, &events), events)
    }

    fn income(events: &[Event], id: u64, kind: EventKind) -> f32 {
        events
            .iter()
            .filter(|e| e.kind == kind && e.source == id)
            .map(|e| e.energy_delta)
            .sum()
    }

    /// A founder starts at its first tick's light and uptake; under steady
    /// light its fullness converges to the light rate at every `τ`.
    #[test]
    fn a_founder_starts_at_its_light_and_steady_light_converges_to_the_light_rate() {
        let params = params();
        let lone = TraitVector {
            fecundity: 0.0,
            ..traits(0.8, 0.0)
        };
        let mut w = world(&params, vec![spec((10.0, 10.0), 400.0, lone)]);
        let mut tracker = FullnessTracker::new();
        let (intakes, events) = step(&mut w);
        tracker.observe(&intakes, &events, &[], &[]);
        let first = intakes[&0];
        assert!(first.light > 0.0);
        assert_eq!(
            *tracker.bank(0).unwrap(),
            FullnessBank::at(first.light, first.uptake)
        );
        let mut last = first;
        for _ in 0..300 {
            let (intakes, events) = step(&mut w);
            tracker.observe(&intakes, &events, &[], &[]);
            last = intakes[&0];
        }
        let bank = tracker.bank(0).unwrap();
        assert_eq!(bank.energy[0], last.light, "τ = 1 is this tick's light");
        for (j, f) in bank.energy.iter().enumerate() {
            assert!(
                close(*f, last.light, 0.02),
                "τ index {j}: {f} vs {}",
                last.light
            );
        }
    }

    fn event(kind: EventKind, source: u64, target: Option<u64>) -> Event {
        Event {
            tick: 0,
            seq: 0,
            kind,
            source,
            target,
            energy_delta: 0.0,
            position: None,
            target_was_carcass: false,
            second_parent: None,
        }
    }

    fn fed(light: f32, drained: f32) -> TickIntake {
        TickIntake {
            maintenance: 1.0,
            light,
            uptake: 0.1,
            drained_energy: drained,
            retained_nutrient: 0.0,
            nutrient_per_energy: 0.5,
        }
    }

    /// Each side is `1 / (1 + (F / C)^n)` against its own ceiling, `k·m`
    /// on energy and `k·m·η·ratio` on nutrient, and the consumer expresses
    /// the emptier: at half its ceiling it keeps `1/(1 + 2^-4) ≈ 94 %` at
    /// `n = 4`, at twice it `≈ 6 %`.
    #[test]
    fn expression_follows_the_emptier_side_against_its_own_ceiling() {
        let intake = TickIntake {
            maintenance: 2.0,
            nutrient_per_energy: 0.5,
            ..Default::default()
        };
        // k = 1.5: C_E = 3, C_N = 1.5.
        let bank = FullnessBank::at(6.0, 0.75);
        let e = bank.expression(0, 1.5, 4.0, &intake);
        assert!(close(e.energy, 1.0 / 17.0, 1e-6), "{e:?}");
        assert!(close(e.co_limited, 16.0 / 17.0, 1e-6), "{e:?}");
        let e = bank.expression(0, 1.5, 2.0, &intake);
        assert!(close(e.energy, 0.2, 1e-6) && close(e.co_limited, 0.8, 1e-6));
        assert_eq!(side_expression(1.0, 0.0, 4.0), 0.0);
        assert_eq!(side_expression(0.0, 0.0, 4.0), 1.0);
    }

    /// With `growth_efficiency = 0` there is no nutrient ceiling: the gate
    /// reads energy alone, however empty the nutrient side.
    #[test]
    fn without_growth_efficiency_the_gate_reads_energy_alone() {
        let mut params = params();
        params.growth_efficiency = 0.0;
        let mut w = world(
            &params,
            vec![
                spec((10.0, 10.0), 50.0, traits(0.5, 0.5)),
                spec((10.01, 10.0), 400.0, traits(1.0, 0.0)),
            ],
        );
        let (intakes, _) = step(&mut w);
        let i = intakes[&0];
        assert_eq!(i.nutrient_per_energy, 0.0);
        let bank = FullnessBank::at(4.0 * i.maintenance, 0.0);
        let e = bank.expression(5, 2.0, 4.0, &i);
        assert!(close(e.co_limited, 1.0 / 17.0, 1e-6), "{e:?}");
        assert_eq!(e.co_limited, e.energy);
    }

    /// Below half expression means past the ceiling on both sides, whatever
    /// the exponent: `1/(1 + r^n) < 1/2` exactly when `r > 1`. So the sated
    /// share does not depend on `n`; only production kept does.
    #[test]
    fn being_below_half_expression_does_not_depend_on_the_exponent() {
        let intake = TickIntake {
            maintenance: 1.0,
            nutrient_per_energy: 0.3,
            ..Default::default()
        };
        for (fe, fn_) in [
            (2.0, 0.9),
            (0.5, 0.9),
            (2.0, 0.2),
            (0.99, 0.29),
            (1.01, 0.31),
        ] {
            let bank = FullnessBank::at(fe, fn_);
            let sated: Vec<bool> = [2.0, 4.0, 8.0]
                .iter()
                .map(|&n| bank.expression(0, 1.0, n, &intake).co_limited < 0.5)
                .collect();
            assert!(
                sated.iter().all(|&s| s == sated[0]),
                "{fe} {fn_}: {sated:?}"
            );
        }
    }

    /// A newborn starts with its parent's bank as the parent stands after
    /// the birth tick's update; a dead agent's bank is dropped.
    #[test]
    fn a_newborn_starts_with_its_parent_s_bank() {
        let mut tracker = FullnessTracker::new();
        tracker.observe(&HashMap::from([(1, fed(2.0, 0.0))]), &[], &[], &[]);
        tracker.observe(&HashMap::from([(1, fed(0.0, 9.0))]), &[], &[], &[]);
        tracker.observe(
            &HashMap::from([(1, fed(1.0, 0.0))]),
            &[event(EventKind::Born, 2, Some(1))],
            &[],
            &[],
        );
        let parent = *tracker.bank(1).unwrap();
        assert_ne!(
            parent,
            FullnessBank::at(1.0, 0.1),
            "the feast is still clearing"
        );
        assert_eq!(tracker.bank(2), Some(&parent));
        tracker.observe(
            &HashMap::from([(1, fed(1.0, 0.0)), (2, fed(0.0, 0.0))]),
            &[event(EventKind::Died, 1, None)],
            &[],
            &[],
        );
        assert_eq!(tracker.bank(1), None);
        assert!(tracker.bank(2).unwrap().energy[3] < parent.energy[3]);
    }

    /// A kin kill is read from the killer's carried-in fullness: a founder
    /// killing on its first tick reads its light 3 m and uptake 1 against
    /// `C_E = k·m` and `C_N = 0.5·k·m`, so it is sated (both sides past the
    /// ceiling) for `k < 2` and sated on energy for `k < 3`, at every `τ`.
    /// A consumer tick adds `max(0, x − 1)` ungated and, per cell,
    /// `max(0, x_g − 1)` with its drain scaled by `E`.
    #[test]
    fn the_grid_counts_hand_computed_kills_and_consumer_ticks() {
        let mut t = FullnessTracker::new();
        let killer = TickIntake {
            maintenance: 1.0,
            light: 3.0,
            uptake: 1.0,
            nutrient_per_energy: 0.5,
            ..Default::default()
        };
        let consumer = |light: f32, drained: f32| TickIntake {
            maintenance: 2.0,
            light,
            drained_energy: drained,
            ..Default::default()
        };
        t.observe(
            &HashMap::from([(1, killer), (2, consumer(0.0, 8.0))]),
            &[],
            &[1, 1],
            &[],
        );
        // Agent 2 now carries F_E = 8/τ (its founder start was 0 light).
        t.observe(
            &HashMap::from([(1, killer), (2, consumer(1.0, 4.0))]),
            &[],
            &[],
            &[2],
        );
        let g = t.finish();
        let (ks, taus) = (k_grid(), tau_grid());
        assert_eq!(g.kills, 2);
        for (i, &k) in ks.iter().enumerate() {
            for j in 0..TAU_POINTS {
                let c = g.cell(i, j);
                assert_eq!(g.sated[c], if k < 2.0 { 2 } else { 0 }, "k {k}");
                assert_eq!(g.sated_energy[c], if k < 3.0 { 2 } else { 0 }, "k {k}");
            }
        }
        assert_eq!(g.consumer_ticks, 1);
        // x = (1 + 4) / 2 = 2.5.
        assert!((g.produced - 1.5).abs() < 1e-9);
        for (e, &n) in EXPONENTS.iter().enumerate() {
            for (i, &k) in ks.iter().enumerate() {
                for (j, &tau) in taus.iter().enumerate() {
                    let expr = 1.0 / (1.0 + ((8.0 / tau) / (2.0 * k)).powf(n));
                    let want = ((1.0 + expr * 4.0) / 2.0 - 1.0).max(0.0);
                    let got = g.produced_gated[g.gated_cell(e, i, j)];
                    assert!(
                        (got - want as f64).abs() < 1e-5,
                        "n {n} k {k} τ {tau}: {got} vs {want}"
                    );
                }
            }
        }
        // τ = 1, k = 20, n = 8: F/C = 0.2, so the feast passes nearly whole.
        let top = g.produced_gated[g.gated_cell(2, ks.len() - 1, 0)];
        assert!((top - 1.5).abs() < 1e-3, "{top}");
        // Separation: agent 1 killed, mean intake 3 m; agent 2 consumed,
        // mean (4 + 2.5) / 2 = 3.25 m.
        assert_eq!((g.killers.agents, g.consumers.agents), (1, 1));
        assert!((g.killers.mean().unwrap() - 3.0).abs() < 1e-9);
        assert!((g.consumers.mean().unwrap() - 3.25).abs() < 1e-9);
    }

    /// Pooling is a sum, cell by cell; an old row's empty grid adds nothing.
    #[test]
    fn pooling_grids_is_a_sum() {
        let mut t = FullnessTracker::new();
        let i = TickIntake {
            maintenance: 1.0,
            light: 0.5,
            drained_energy: 3.0,
            ..Default::default()
        };
        t.observe(&HashMap::from([(1, i)]), &[], &[1], &[1]);
        t.observe(&HashMap::from([(1, i)]), &[], &[], &[1]);
        let one = t.finish();
        let mut pooled = FullnessGrid::default();
        pooled.merge(&one);
        pooled.merge(&FullnessGrid::default());
        pooled.merge(&one);
        assert_eq!(pooled.kills, 2 * one.kills);
        assert_eq!(pooled.consumer_ticks, 4);
        assert_eq!(pooled.produced, 2.0 * one.produced);
        for (p, o) in pooled.sated_energy.iter().zip(&one.sated_energy) {
            assert_eq!(*p, 2 * o);
        }
        for (p, o) in pooled.produced_gated.iter().zip(&one.produced_gated) {
            assert_eq!(*p, 2.0 * o);
        }
        assert_eq!(pooled.consumers.agents, 2);
        assert_eq!(pooled.killers.distribution.count(), 2);
    }

    /// A hand-filled grid on two `k` and two `τ`: sated falls with `k`,
    /// kept rises with it. The evaluation reads both shares per cell, finds
    /// the feasible ones, picks the largest margin, and says EMPTY with the
    /// closest cell when none is feasible.
    #[test]
    fn the_evaluation_picks_the_largest_margin_default_or_reports_empty() {
        let mut g = FullnessGrid {
            k: vec![1.0, 4.0],
            tau: vec![2.0, 8.0],
            exponents: vec![2.0, 4.0],
            kills: 100,
            // (k, τ) = (1, 2), (1, 8), (4, 2), (4, 8)
            sated: vec![90, 95, 62, 70],
            sated_energy: vec![99, 99, 80, 85],
            consumer_ticks: 10,
            produced: 10.0,
            // n = 2, then n = 4
            produced_gated: vec![1.0, 0.5, 7.0, 7.6, 2.0, 1.0, 7.5, 9.0],
            ..Default::default()
        };
        let r = FullnessRegion::evaluate(&g, 1);
        assert_eq!(r.exponent, 4.0);
        let c = r.cells[3];
        assert_eq!((c.k, c.tau), (4.0, 8.0));
        assert_eq!(
            (c.sated, c.sated_energy, c.kept),
            (Some(0.70), Some(0.85), Some(0.9))
        );
        assert_eq!(r.feasible_count(), 2, "(4, 2) at 62/75 and (4, 8) at 70/90");
        let d = r.default_point().unwrap();
        assert_eq!((d.k, d.tau), (4.0, 8.0));
        assert!((d.margin().unwrap() - 0.10).abs() < 1e-9);
        let text = fullness_report(&g);
        assert!(text.contains("### n = 4"), "{text}");
        assert!(text.contains("default (largest margin) k = 4, τ = 8: sated 70.0 % (energy side alone 85.0 %), kept 90.0 %, margin 0.100"), "{text}");
        assert!(text.contains("| 4 | 62/75* | 70/90* |"), "{text}");
        // n = 2: (4, 8) keeps 76 %, (4, 2) 70 %: one feasible cell.
        assert_eq!(FullnessRegion::evaluate(&g, 0).feasible_count(), 1);

        g.produced_gated = vec![0.0; 8];
        let r = FullnessRegion::evaluate(&g, 1);
        assert_eq!(r.default_point(), None);
        let text = fullness_report(&g);
        assert!(text.contains("region EMPTY at n = 4: no (k, τ) on the grid meets both bars (closest: k = 1, τ = 2"), "{text}");
        assert!(text.contains("light-fed mixotroph kin killers"), "{text}");
    }

    /// A consumer beside a large prey takes in the energy it received and,
    /// on the nutrient side, the bound nutrient released capped at its
    /// stoichiometric demand on that energy (the drain pass's retention);
    /// its light and uptake count too.
    #[test]
    fn a_consumer_books_drain_energy_received_and_nutrient_retained() {
        let params = params();
        let mut w = world(
            &params,
            vec![
                spec((10.0, 10.0), 50.0, traits(0.3, 1.0)),
                spec((10.01, 10.0), 400.0, traits(1.0, 0.0)),
            ],
        );
        let drain_time = PreStep::capture(&w).drain_start(&params).agents;
        let (intakes, events) = step(&mut w);
        let drained = income(&events, 0, EventKind::Consumed);
        assert!(drained > 0.0, "the consumer fed");
        let (c, t) = (traits(0.3, 1.0), traits(1.0, 0.0));
        let gained = drained * explorers_sim::trophic_transfer_efficiency(&c, &t, &params);
        let released = drained * explorers_sim::stoichiometric_demand(&t, 1.0, &params);
        let need =
            explorers_sim::stoichiometric_demand(&c, drain_time[0].structure, &params) * gained;
        let i = intakes[&0];
        assert!(close(i.drained_energy, gained, 1e-6), "{i:?}");
        assert!(
            close(i.retained_nutrient, released.min(need), 1e-6),
            "{i:?}"
        );
        assert_eq!(i.light, income(&events, 0, EventKind::Photosynthesized));
        assert!(close(i.energy(), i.light + gained, 1e-6));
        assert!(close(i.nutrient(), i.uptake + released.min(need), 1e-6));
        let m = phase::metabolic_cost(&drain_time[0], &params);
        assert!(close(i.maintenance, m, 1e-6));
        assert!(i.nutrient_per_energy > 0.0);
    }

    fn close(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol * b.abs().max(1.0)
    }

    /// From empty, a steady intake fills every `τ` toward the rate: after
    /// `t` ticks the shortfall is `(1 − 1/τ)^t` of it.
    #[test]
    fn a_steady_intake_converges_to_its_rate() {
        let mut bank = FullnessBank::at(0.0, 0.0);
        for _ in 0..400 {
            bank.update(3.0, 0.5);
        }
        for j in 0..TAU_POINTS {
            assert!(close(bank.energy[j], 3.0, 1e-3), "{bank:?}");
            assert!(close(bank.nutrient[j], 0.5, 1e-3), "{bank:?}");
        }
    }

    /// A single feast on an empty bank spikes it by `intake / τ`, then each
    /// following empty tick clears a `1/τ` share: after `t` ticks it reads
    /// `(intake / τ)(1 − 1/τ)^t`, an exponential decay with time constant
    /// about `τ`. At `τ = 1` fullness is this tick's intake alone.
    #[test]
    fn a_single_feast_spikes_then_decays_with_time_constant_tau() {
        let taus = tau_grid();
        assert_eq!((taus[0], taus[TAU_POINTS - 1]), (1.0, 50.0));
        let mut bank = FullnessBank::at(0.0, 0.0);
        bank.update(10.0, 2.0);
        for (j, tau) in taus.iter().enumerate() {
            assert!(close(bank.energy[j], 10.0 / tau, 1e-6));
            assert!(close(bank.nutrient[j], 2.0 / tau, 1e-6));
        }
        for _ in 0..7 {
            bank.update(0.0, 0.0);
        }
        for (j, tau) in taus.iter().enumerate() {
            let want = 10.0 / tau * (1.0 - 1.0 / tau).powi(7);
            assert!(close(bank.energy[j], want, 1e-5), "τ {tau}");
        }
        assert_eq!(bank.energy[0], 0.0, "τ = 1 holds this tick only");
    }
}
