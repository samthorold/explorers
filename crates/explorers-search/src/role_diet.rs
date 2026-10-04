//! Trait-read role tag against realised diet (issue #596).
//!
//! Until #599 the evaluator's role tag read an agent as a heterotroph by
//! *trait* (heterotrophy > photosynthetic absorption) and split heterotrophs
//! into consumer and decomposer by realised diet (the projection's detrital
//! reliance). #591 found tagged "consumers" living almost entirely on light.
//! This module keeps that retired tag ([`trait_tag`]) as the thing measured,
//! books every agent's lifetime energy income by source — light
//! (`Photosynthesized`), living prey and carcasses (`Consumed`, split by
//! `target_was_carcass`) — and reads a **diet role** from it by the
//! evaluator's income rule ([`Income::role`]): a producer when light is at
//! least [`PRODUCER_LIGHT_SHARE`] of its income, else a consumer or
//! decomposer by the living/carcass split. The evaluator (#599) reads the same
//! rule over a decaying recent window; the census reads it over the lifetime.
//!
//! [`rollout`] mirrors the genesis rollout (`explorers_genesis::rollout`)
//! step for step: the same retention, the same `RolloutObservations`, the same
//! early stops and the same horizon verdict. On the evaluator's own
//! second-half sample ticks it takes the trait tag and the diet role of every
//! agent, so the guild rule (`guild::role_guilds_from_samples`) reads both on
//! identical samples. It also attributes every death:
//! age class, whether the agent was grazed alive in its death tick, and
//! whether a grazer was kin (parent, offspring or sibling, by `Born`).
//!
//! Heterotrophic income is booked as energy *drained* (`Consumed`'s
//! `energy_delta`, before the trophic transfer efficiency): the quantity the
//! tag's detrital-reliance split reads, and an upper bound on what the
//! agent kept. A light share read against it is therefore a lower bound — the
//! conservative direction for asking whether a tagged heterotroph lives on
//! light.
//!
//! Observer-side only: no trajectory changes.

use std::collections::HashMap;

use explorers_genesis_eval::guild::{RoleGuilds, RoleSnapshot, role_guilds_from_samples};
pub use explorers_genesis_eval::income::{Income, PRODUCER_LIGHT_SHARE};
use explorers_genesis_eval::{EvalConfig, FailureMode, RolloutObservations};
use explorers_sim::event::{Event, EventKind};
use explorers_sim::topology::{DETRITAL_RELIANCE_THRESHOLD, TopologyProjection, TrophicRole};
use explorers_sim::{InitialDistribution, TraitVector, World, WorldParameters};

use crate::fullness::{FullnessGrid, FullnessTracker, tick_intakes};
use crate::grazer_hunger::{
    GrazerHunger, KillerReading, PreStep, Surplus, SurplusDistribution, TraitDistances,
};
use crate::intake_ceiling::{GateSample, IntakeReading, intake_readings};
pub use crate::intake_ceiling::{GateSamples, IntakeCensus};

/// A death at or below this age (ticks from birth) is an infant death, as
/// #591's accountant counts it.
pub const INFANT_AGE: u64 = 50;

/// Deciles of the light share.
pub const LIGHT_SHARE_BINS: usize = 10;

/// The role an agent's lifetime income reads as, by the evaluator's income
/// rule ([`Income::role`]): `None` with no income yet.
pub fn diet_role(income: &Income) -> Option<TrophicRole> {
    income.role()
}

/// The retired trait-read role tag (pre-#599), kept as the thing this census
/// measures: a producer when autotrophy is at least heterotrophy, else a
/// decomposer when the projection's detrital reliance reaches
/// [`DETRITAL_RELIANCE_THRESHOLD`], else a consumer (a non-eater included).
pub fn trait_tag(id: u64, traits: &TraitVector, topology: &TopologyProjection) -> TrophicRole {
    if traits.photosynthetic_absorption >= traits.heterotrophy {
        return TrophicRole::Producer;
    }
    match topology.detrital_reliance(id) {
        Some(r) if r >= DETRITAL_RELIANCE_THRESHOLD => TrophicRole::Decomposer,
        _ => TrophicRole::Consumer,
    }
}

/// Index of a role in the census tables: producer, consumer, decomposer.
fn role_index(role: TrophicRole) -> usize {
    match role {
        TrophicRole::Producer => 0,
        TrophicRole::Consumer => 1,
        TrophicRole::Decomposer => 2,
    }
}

/// Deaths in one bucket.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DeathCounts {
    pub deaths: u64,
    /// At or below [`INFANT_AGE`] ticks old.
    pub infant: u64,
    /// Drained alive (a `Consumed` event on it as a living target) in the
    /// tick it died.
    pub grazed: u64,
    pub infant_grazed: u64,
    /// Grazed in its death tick by at least one parent, offspring or sibling.
    pub grazed_by_kin: u64,
    pub infant_grazed_by_kin: u64,
}

impl DeathCounts {
    fn add(&mut self, infant: bool, grazed: bool, by_kin: bool) {
        self.deaths += 1;
        self.infant += u64::from(infant);
        self.grazed += u64::from(grazed);
        self.infant_grazed += u64::from(infant && grazed);
        self.grazed_by_kin += u64::from(grazed && by_kin);
        self.infant_grazed_by_kin += u64::from(infant && grazed && by_kin);
    }

    pub fn merge(&mut self, other: &DeathCounts) {
        self.deaths += other.deaths;
        self.infant += other.infant;
        self.grazed += other.grazed;
        self.infant_grazed += other.infant_grazed;
        self.grazed_by_kin += other.grazed_by_kin;
        self.infant_grazed_by_kin += other.infant_grazed_by_kin;
    }
}

/// Deaths by what the agent was: a producer by trait, and a heterotroph by
/// trait split by its diet at death — lived on light (a producer by diet),
/// lived on drained energy (a heterotroph by diet), or died before booking any
/// income.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DeathTable {
    pub trait_producer: DeathCounts,
    pub trait_heterotroph_light_fed: DeathCounts,
    pub trait_heterotroph_diet_fed: DeathCounts,
    pub trait_heterotroph_no_income: DeathCounts,
}

impl DeathTable {
    pub fn merge(&mut self, other: &DeathTable) {
        self.trait_producer.merge(&other.trait_producer);
        self.trait_heterotroph_light_fed
            .merge(&other.trait_heterotroph_light_fed);
        self.trait_heterotroph_diet_fed
            .merge(&other.trait_heterotroph_diet_fed);
        self.trait_heterotroph_no_income
            .merge(&other.trait_heterotroph_no_income);
    }
}

/// The deaths table's groups (and the reproduction table's): a producer by
/// trait, or a heterotroph by trait split by its diet — lived on light, lived
/// on drained energy, or no income yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DietGroup {
    TraitProducer,
    TraitHeterotrophLightFed,
    TraitHeterotrophDietFed,
    TraitHeterotrophNoIncome,
}

/// The group of an agent with these traits and this lifetime income.
pub fn diet_group(traits: &TraitVector, income: &Income) -> DietGroup {
    if traits.photosynthetic_absorption >= traits.heterotrophy {
        return DietGroup::TraitProducer;
    }
    match diet_role(income) {
        Some(TrophicRole::Producer) => DietGroup::TraitHeterotrophLightFed,
        Some(_) => DietGroup::TraitHeterotrophDietFed,
        None => DietGroup::TraitHeterotrophNoIncome,
    }
}

/// Reproduction in one diet group (#624).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GroupReproduction {
    /// Births with a parent in this group at the birth, counted once per
    /// parent (a two-parent birth books to each parent's group).
    pub births: u64,
    /// Second-half agent-samples whose earmark fill was read.
    pub samples: u64,
    /// Summed earmark fill over those samples: the energy the grow phase
    /// moved into the reproductive earmark in the step that led to the
    /// sample (the accounting's earmark fill).
    pub earmark_fill: f64,
}

impl GroupReproduction {
    pub fn record_fill(&mut self, fill: f32) {
        self.samples += 1;
        self.earmark_fill += fill as f64;
    }

    /// Mean earmark fill per agent-sample (energy per tick).
    pub fn mean_earmark_fill(&self) -> Option<f64> {
        (self.samples > 0).then(|| self.earmark_fill / self.samples as f64)
    }

    pub fn merge(&mut self, other: &GroupReproduction) {
        self.births += other.births;
        self.samples += other.samples;
        self.earmark_fill += other.earmark_fill;
    }
}

/// Births and earmark fill by [`DietGroup`], the deaths table's rows (#624).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReproductionTable {
    pub trait_producer: GroupReproduction,
    pub trait_heterotroph_light_fed: GroupReproduction,
    pub trait_heterotroph_diet_fed: GroupReproduction,
    pub trait_heterotroph_no_income: GroupReproduction,
}

impl ReproductionTable {
    pub fn group_mut(&mut self, group: DietGroup) -> &mut GroupReproduction {
        match group {
            DietGroup::TraitProducer => &mut self.trait_producer,
            DietGroup::TraitHeterotrophLightFed => &mut self.trait_heterotroph_light_fed,
            DietGroup::TraitHeterotrophDietFed => &mut self.trait_heterotroph_diet_fed,
            DietGroup::TraitHeterotrophNoIncome => &mut self.trait_heterotroph_no_income,
        }
    }

    pub fn merge(&mut self, other: &ReproductionTable) {
        self.trait_producer.merge(&other.trait_producer);
        self.trait_heterotroph_light_fed
            .merge(&other.trait_heterotroph_light_fed);
        self.trait_heterotroph_diet_fed
            .merge(&other.trait_heterotroph_diet_fed);
        self.trait_heterotroph_no_income
            .merge(&other.trait_heterotroph_no_income);
    }
}

/// The per-agent ledger: income by source, descent, age, and the living
/// grazers of the current tick. Fed one tick's events at a time.
#[derive(Clone, Debug, Default)]
pub struct DietLedger {
    income: HashMap<u64, Income>,
    traits: HashMap<u64, TraitVector>,
    born_tick: HashMap<u64, u64>,
    parents: HashMap<u64, [Option<u64>; 2]>,
    children: HashMap<u64, Vec<u64>>,
    deaths: DeathTable,
    kills: GrazerHunger,
    births: TraitDistances,
    reproduction: ReproductionTable,
    /// The intake-ceiling multiple the killing grazers' predicted intake-gate
    /// expression is read at (#629); `None` skips the read.
    intake_ceiling_k: Option<f32>,
    /// Whether to keep each kin-killing pair's killer intake reading for
    /// [`DietLedger::take_kin_kill_readings`] (#634).
    keep_kin_kills: bool,
    /// Kin-killing pairs' killers since the last take: (killer, its
    /// heterotrophy trait, its intake reading at the kill).
    kin_kills: Vec<(u64, f32, IntakeReading)>,
}

impl DietLedger {
    /// Keep every kin-killing pair's killer intake reading at the start of
    /// drain resolution, one per pair, until taken (#634).
    pub fn with_kin_kill_readings(mut self) -> Self {
        self.keep_kin_kills = true;
        self
    }

    /// The kin-killing pairs' killer readings kept since the last take:
    /// (killer id, heterotrophy trait, intake reading).
    pub fn take_kin_kill_readings(&mut self) -> Vec<(u64, f32, IntakeReading)> {
        std::mem::take(&mut self.kin_kills)
    }

    /// Read each killing grazer's predicted intake-gate expression at
    /// ceiling multiple `k` (#629).
    pub fn with_intake_ceiling_k(mut self, k: Option<f32>) -> Self {
        self.intake_ceiling_k = k;
        self
    }

    /// A ledger over a world's founders (born at tick 0, no parents).
    pub fn new(world: &World) -> Self {
        let mut ledger = Self::default();
        ledger.admit(world, 0);
        ledger
    }

    /// Record the traits of every agent not yet known, born at `tick`.
    fn admit(&mut self, world: &World, tick: u64) {
        for a in world.agents() {
            self.traits.entry(a.id).or_insert(a.traits);
            self.born_tick.entry(a.id).or_insert(tick);
        }
    }

    /// Book one stepped tick: `events` are the log's events of that tick and
    /// `world` is the state after it. Births are recorded before deaths are
    /// attributed, so a grazer born this tick still counts as kin. With the
    /// tick's [`PreStep`], every (grazer, victim) pair of a grazed death is
    /// tallied by the grazer's drain-time satiation ([`DietLedger::kills`]).
    pub fn ingest(&mut self, world: &World, events: &[Event], pre: Option<&PreStep>) {
        let mut grazers: HashMap<u64, Vec<u64>> = HashMap::new();
        let mut died: Vec<(u64, u64)> = Vec::new();
        for e in events {
            match e.kind {
                EventKind::Photosynthesized => {
                    self.income.entry(e.source).or_default().light += e.energy_delta as f64;
                }
                EventKind::Consumed => {
                    let income = self.income.entry(e.source).or_default();
                    if e.target_was_carcass {
                        income.carcass += e.energy_delta as f64;
                    } else {
                        income.living += e.energy_delta as f64;
                        if let Some(victim) = e.target {
                            grazers.entry(victim).or_default().push(e.source);
                        }
                    }
                }
                EventKind::Born => {
                    let parents = [e.target, e.second_parent];
                    for p in parents.into_iter().flatten() {
                        self.children.entry(p).or_default().push(e.source);
                    }
                    self.parents.insert(e.source, parents);
                    self.born_tick.insert(e.source, e.tick);
                }
                EventKind::Died => died.push((e.source, e.tick)),
                _ => {}
            }
        }
        self.record_births(world, events);
        type Readings = (HashMap<u64, KillerReading>, HashMap<u64, IntakeReading>);
        let mut readings: Option<Readings> = None;
        let (ceiling_k, keep) = (self.intake_ceiling_k, self.keep_kin_kills);
        // Every kill is read before any death is attributed: a grazer that
        // died this tick too is still on the ledger.
        for &(id, _) in &died {
            let by = grazers.get(&id).map(Vec::as_slice).unwrap_or(&[]);
            if let (Some(pre), false) = (pre, by.is_empty()) {
                let (reading, intake) = readings.get_or_insert_with(|| {
                    let params = world.params();
                    let mut readings = pre.killer_readings(params);
                    let intake = if ceiling_k.is_some() || keep {
                        intake_readings(pre, params, events)
                    } else {
                        HashMap::new()
                    };
                    if let Some(k) = ceiling_k {
                        for (id, i) in &intake {
                            if let Some(r) = readings.get_mut(id) {
                                r.intake_expression = Some(i.expression_at(k));
                            }
                        }
                    }
                    (readings, intake)
                });
                self.record_kills(id, by, reading, intake);
            }
        }
        for (id, tick) in died {
            let by = grazers.get(&id).map(Vec::as_slice).unwrap_or(&[]);
            self.attribute_death(id, tick, by);
        }
        self.admit(world, world.tick());
    }

    /// Whether `a` and `b` are parent and offspring or siblings (share a
    /// parent).
    pub fn is_kin(&self, a: u64, b: u64) -> bool {
        let parents_of = |x: u64| self.parents.get(&x).copied().unwrap_or([None, None]);
        let (pa, pb) = (parents_of(a), parents_of(b));
        pa.contains(&Some(b))
            || pb.contains(&Some(a))
            || pa
                .iter()
                .flatten()
                .any(|p| pb.iter().flatten().any(|q| p == q))
    }

    /// Each birth's trait distance to each of its parents, and the birth
    /// booked to each parent's diet group.
    fn record_births(&mut self, world: &World, events: &[Event]) {
        let born: Vec<&Event> = events
            .iter()
            .filter(|e| e.kind == EventKind::Born)
            .collect();
        if born.is_empty() {
            return;
        }
        for e in &born {
            for p in [e.target, e.second_parent].into_iter().flatten() {
                if let Some(traits) = self.traits.get(&p) {
                    let group = diet_group(traits, &self.income(p));
                    self.reproduction.group_mut(group).births += 1;
                }
            }
        }
        let roster: HashMap<u64, TraitVector> =
            world.agents().iter().map(|a| (a.id, a.traits)).collect();
        for e in born {
            let Some(child) = roster.get(&e.source) else {
                continue;
            };
            for p in [e.target, e.second_parent].into_iter().flatten() {
                if let Some(parent) = self.traits.get(&p) {
                    self.births.record(parent, child);
                }
            }
        }
    }

    /// Tally each grazer of a grazed death by kinship, trait distance to the
    /// victim, drain-time satiation and pre-growth surplus expression.
    fn record_kills(
        &mut self,
        victim: u64,
        grazers: &[u64],
        reading: &HashMap<u64, KillerReading>,
        intake: &HashMap<u64, IntakeReading>,
    ) {
        let Some(traits) = self.traits.get(&victim).copied() else {
            return;
        };
        let mut seen: Vec<u64> = Vec::with_capacity(grazers.len());
        for &g in grazers {
            if seen.contains(&g) {
                continue;
            }
            seen.push(g);
            let (Some(r), Some(gt)) = (reading.get(&g), self.traits.get(&g)) else {
                continue;
            };
            let kin = self.is_kin(victim, g);
            self.kills.record(kin, traits.distance(gt), *r);
            if kin
                && self.keep_kin_kills
                && let Some(i) = intake.get(&g)
            {
                self.kin_kills.push((g, gt.heterotrophy, *i));
            }
        }
    }

    fn attribute_death(&mut self, id: u64, tick: u64, grazers: &[u64]) {
        let Some(traits) = self.traits.remove(&id) else {
            return;
        };
        let age = tick.saturating_sub(self.born_tick.get(&id).copied().unwrap_or(0));
        let grazed = !grazers.is_empty();
        let by_kin = grazers.iter().any(|&g| self.is_kin(id, g));
        let income = self.income.remove(&id).unwrap_or_default();
        let bucket = match diet_group(&traits, &income) {
            DietGroup::TraitProducer => &mut self.deaths.trait_producer,
            DietGroup::TraitHeterotrophLightFed => &mut self.deaths.trait_heterotroph_light_fed,
            DietGroup::TraitHeterotrophDietFed => &mut self.deaths.trait_heterotroph_diet_fed,
            DietGroup::TraitHeterotrophNoIncome => &mut self.deaths.trait_heterotroph_no_income,
        };
        bucket.add(age <= INFANT_AGE, grazed, by_kin);
    }

    pub fn income(&self, id: u64) -> Income {
        self.income.get(&id).copied().unwrap_or_default()
    }

    pub fn deaths(&self) -> &DeathTable {
        &self.deaths
    }

    /// (grazer, victim) pairs of grazed deaths by the grazer's drain-time
    /// satiation (booked only when [`DietLedger::ingest`] had the tick's
    /// [`PreStep`]).
    pub fn kills(&self) -> &GrazerHunger {
        &self.kills
    }

    /// Parent–offspring trait distances.
    pub fn births(&self) -> &TraitDistances {
        &self.births
    }

    /// Births by the parent's diet group at the birth (earmark fill is
    /// sampled by the rollout, not the ledger).
    pub fn reproduction(&self) -> &ReproductionTable {
        &self.reproduction
    }
}

/// Trait tag × diet role over agent-samples: rows producer / consumer /
/// decomposer by the tag, columns the same by diet plus "no income".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Confusion(pub [[u64; 4]; 3]);

impl Confusion {
    fn add(&mut self, tag: TrophicRole, diet: Option<TrophicRole>) {
        self.0[role_index(tag)][diet.map_or(3, role_index)] += 1;
    }

    pub fn merge(&mut self, other: &Confusion) {
        for (row, other) in self.0.iter_mut().zip(other.0) {
            for (c, o) in row.iter_mut().zip(other) {
                *c += o;
            }
        }
    }
}

/// Surplus satiation ([`Surplus`], #622) over agent-samples, by recent-income
/// role (#599): `roles` is producer, consumer, decomposer, no role (no income
/// yet). The **light-fed mixotrophs** — income producers carrying
/// heterotrophy above zero — are tallied again apart: the population whose
/// 25th percentile sets the default `satiation_sensitivity` (world rules,
/// *Capability and expression are decoupled*), read over those with
/// positive surplus ([`SurplusByRole::proposed_sensitivity`]).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SurplusByRole {
    pub roles: [SurplusDistribution; 4],
    pub light_fed_mixotrophs: SurplusDistribution,
    /// The light-fed mixotrophs' energy side alone, `max(0, reserve −
    /// buffer) / m`: what the read would be without nutrient co-limitation.
    #[serde(default)]
    pub light_fed_mixotrophs_energy: SurplusDistribution,
}

impl SurplusByRole {
    pub fn record(&mut self, role: Option<TrophicRole>, heterotrophy: f32, surplus: &Surplus) {
        self.roles[role.map_or(3, role_index)].record_surplus(surplus);
        if role == Some(TrophicRole::Producer) && heterotrophy > 0.0 {
            self.light_fed_mixotrophs.record_surplus(surplus);
            self.light_fed_mixotrophs_energy
                .record(surplus.energy.max(0.0));
        }
    }

    /// The default `satiation_sensitivity` this census proposes: `1 / s₂₅`
    /// over the light-fed mixotrophs with positive surplus. Those at zero are
    /// short of free nutrient (or at their buffer), and the design leaves
    /// them at full capability at any `c`, so they do not set it (#622).
    pub fn proposed_sensitivity(&self) -> Option<f64> {
        let s25 = self.light_fed_mixotrophs.positive_percentile(0.25)?;
        (s25 > 0.0).then(|| 1.0 / s25)
    }

    pub fn merge(&mut self, other: &SurplusByRole) {
        for (a, b) in self.roles.iter_mut().zip(&other.roles) {
            a.merge(b);
        }
        self.light_fed_mixotrophs.merge(&other.light_fed_mixotrophs);
        self.light_fed_mixotrophs_energy
            .merge(&other.light_fed_mixotrophs_energy);
    }
}

/// One seed's census.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SeedDiet {
    pub seed: u64,
    /// The evaluator's failure label, `None` for a live run.
    pub failure: Option<String>,
    pub termination_tick: u64,
    /// The guild rule read with the retired trait tag ([`trait_tag`]; the
    /// evaluator's read before #599).
    pub tag_guilds: [bool; 3],
    /// The same rule on the same samples with every agent read by its
    /// lifetime diet role (an agent with no income yet has no role and is
    /// left out, as in the evaluator).
    pub diet_guilds: [bool; 3],
    /// Trait tag × diet role over the second-half samples.
    pub confusion: Confusion,
    /// Light share of lifetime income, in deciles, over the second-half
    /// samples of agents the tag reads as heterotrophs (with income).
    pub heterotroph_light_share: [u64; LIGHT_SHARE_BINS],
    /// Every death of the run, attributed.
    pub deaths: DeathTable,
    /// The terminal roster's producer share of living energy — the
    /// evaluator's trophic-balance term until #602 retired it — with agents bucketed by the trait
    /// tag (the evaluator's read before #599) and by lifetime diet role (an
    /// agent with no income is left out). `None` for an empty roster.
    #[serde(default)]
    pub trophic_balance_tag: Option<f32>,
    #[serde(default)]
    pub trophic_balance_diet: Option<f32>,
    /// Every grazed death's (grazer, victim) pairs, by kinship, trait
    /// distance and the grazer's satiation in the drain pass (#606).
    #[serde(default)]
    pub kills: GrazerHunger,
    /// Every birth's trait distance to each parent (#606).
    #[serde(default)]
    pub births: TraitDistances,
    /// Every second-half sampled agent's surplus satiation, read after
    /// metabolism and before growth in the step that led to the sample, by
    /// the evaluator's recent-income role at the sample (#622). Agents born
    /// in that step have no pre-growth state and are left out.
    #[serde(default)]
    pub surplus: SurplusByRole,
    /// Births by the parent's diet group at the birth (every birth of the
    /// run), and earmark fill by diet group over the second-half
    /// agent-samples the surplus read takes (#624).
    #[serde(default)]
    pub reproduction: ReproductionTable,
    /// The intake-ceiling window's populations (#629): every second-half
    /// sampled agent's intake at the start of drain resolution in the step
    /// that led to the sample, light-fed mixotrophs (by recent-income role,
    /// as the surplus read) and heterotrophs by diet (as the reproduction
    /// table's group). Agents born in that step are left out.
    #[serde(default)]
    pub intake: IntakeCensus,
    /// Per-sample gate records for the `(k_a, k_h)` region (#634): every
    /// kin-killing pair whose killer is a light-fed mixotroph (income-role
    /// producer, as of the start of the kill's tick, with heterotrophy above
    /// zero), the killer read at the start of drain resolution, over the
    /// whole run as the kill bands; and every heterotroph-by-diet sample of
    /// [`SeedDiet::intake`].
    #[serde(default)]
    pub gates: GateSamples,
    /// The fullness-gate region's counters (#637), when the readout ran:
    /// the same kin kills as [`SeedDiet::gates`], read against every agent's
    /// online fullness bank, and heterotrophs by diet on every second-half
    /// tick. Absent otherwise, and on older rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fullness: Option<FullnessGrid>,
}

fn guild_flags(g: RoleGuilds) -> [bool; 3] {
    [g.producer, g.consumer, g.decomposer]
}

/// A failure mode as the atlas's cliff label.
pub fn failure_label(f: &FailureMode) -> &'static str {
    match f {
        FailureMode::Extinction => "extinction",
        FailureMode::PopulationExplosion => "population_explosion",
        FailureMode::EnergyDeath => "energy_death",
        FailureMode::NutrientLockup => "nutrient_lockup",
        FailureMode::BloomStop => "bloom_stop",
        FailureMode::Monoculture => "monoculture",
        FailureMode::GeneralistDominance => "generalist_dominance",
    }
}

/// The census's verdict on a finished rollout: `stopped` is the early stop
/// that ended it, if any. An extinction or explosion stop is left to the
/// evaluator, which reads those itself; any other early stop is the verdict
/// (there is no settled window to read); a run that reached the horizon is
/// evaluated from its log.
pub fn census_failure(
    world: &World,
    observations: &RolloutObservations,
    eval_config: &EvalConfig,
    max_ticks: u64,
    stopped: Option<FailureMode>,
) -> Option<FailureMode> {
    match stopped {
        Some(FailureMode::Extinction) | Some(FailureMode::PopulationExplosion) | None => {
            explorers_genesis_eval::evaluate_from_log(world, observations, eval_config, max_ticks)
                .failure
        }
        Some(failure) => Some(failure),
    }
}

/// Seeds pooled by the census's verdict (#645): count per failure label,
/// with live runs under "persisted".
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Outcomes(std::collections::BTreeMap<String, u64>);

const PERSISTED: &str = "persisted";

impl Outcomes {
    pub fn record(&mut self, failure: Option<&FailureMode>) {
        let label = failure.map_or(PERSISTED, failure_label);
        *self.0.entry(label.to_string()).or_default() += 1;
    }

    pub fn merge(&mut self, o: &Outcomes) {
        for (k, n) in &o.0 {
            *self.0.entry(k.clone()).or_default() += n;
        }
    }

    pub fn seeds(&self) -> u64 {
        self.0.values().sum()
    }

    pub fn persisted(&self) -> u64 {
        self.0.get(PERSISTED).copied().unwrap_or(0)
    }

    /// `persisted n, <label> n, …` (persisted first, then by label); `–`
    /// when empty.
    pub fn describe(&self) -> String {
        let parts: Vec<String> = (self.persisted() > 0)
            .then(|| format!("{PERSISTED} {}", self.persisted()))
            .into_iter()
            .chain(
                self.0
                    .iter()
                    .filter(|(k, _)| k.as_str() != PERSISTED)
                    .map(|(k, n)| format!("{k} {n}")),
            )
            .collect();
        if parts.is_empty() {
            "–".into()
        } else {
            parts.join(", ")
        }
    }
}

/// One seed rolled out as the genesis rollout does it (unbudgeted, no
/// cross-check carry), with the diet census taken alongside.
pub fn rollout(
    params: &WorldParameters,
    dist: &InitialDistribution,
    seed: u64,
    max_ticks: u64,
    eval_config: &EvalConfig,
) -> SeedDiet {
    rollout_with(params, dist, seed, max_ticks, eval_config, None)
}

/// [`rollout`], with the killing grazers' predicted intake-gate expression
/// read at ceiling multiple `intake_ceiling_k` when given (#629).
pub fn rollout_with(
    params: &WorldParameters,
    dist: &InitialDistribution,
    seed: u64,
    max_ticks: u64,
    eval_config: &EvalConfig,
    intake_ceiling_k: Option<f32>,
) -> SeedDiet {
    rollout_with_fullness(
        params,
        dist,
        seed,
        max_ticks,
        eval_config,
        intake_ceiling_k,
        false,
    )
}

/// [`rollout_with`], with the fullness readout (#637) when `fullness`:
/// every agent's fullness bank kept online from each tick's intake
/// ([`crate::fullness`]), observer-only.
pub fn rollout_with_fullness(
    params: &WorldParameters,
    dist: &InitialDistribution,
    seed: u64,
    max_ticks: u64,
    eval_config: &EvalConfig,
    intake_ceiling_k: Option<f32>,
    fullness: bool,
) -> SeedDiet {
    let mut fullness = fullness.then(FullnessTracker::new);
    let mut world = World::new(params.clone(), dist.clone(), seed);
    let mut kinds = explorers_genesis_eval::EVALUATOR_EVENT_KINDS.to_vec();
    for k in [
        EventKind::Photosynthesized,
        EventKind::Born,
        EventKind::Died,
    ] {
        if !kinds.contains(&k) {
            kinds.push(k);
        }
    }
    world.retain_event_kinds(&kinds);
    let mut observations = RolloutObservations::with_capacity(max_ticks as usize);
    let interval = eval_config.coexistence_sample_interval;
    let sustainable_stock = explorers_genesis_eval::sustainable_stock(params);
    let window_start = max_ticks / 2 + 1;

    let mut ledger = DietLedger::new(&world)
        .with_intake_ceiling_k(intake_ceiling_k)
        .with_kin_kill_readings();
    let mut topology = TopologyProjection::new();
    let mut tag_snapshots: Vec<RoleSnapshot> = Vec::new();
    let mut cursor = 0;
    let mut diet_snapshots: Vec<RoleSnapshot> = Vec::new();
    let mut confusion = Confusion::default();
    let mut light_share = [0u64; LIGHT_SHARE_BINS];
    let mut surplus = SurplusByRole::default();
    let mut fills = ReproductionTable::default();
    let mut intake = IntakeCensus::default();
    let mut gates = GateSamples::default();
    let mut stopped: Option<FailureMode> = None;
    for _ in 0..max_ticks {
        let pre = PreStep::capture(&world);
        world.step();
        let tail: Vec<Event> = world.event_log().since(cursor).to_vec();
        cursor = world.event_log().len();
        ledger.ingest(&world, &tail, Some(&pre));
        // The killer's role as of the start of the kill's tick: the income
        // ledger has not yet walked this tick.
        let mut kin_killers = Vec::new();
        for (g, heterotrophy, r) in ledger.take_kin_kill_readings() {
            if heterotrophy > 0.0 && observations.income().role(g) == Some(TrophicRole::Producer) {
                gates.mixotroph_kin_kills.push(GateSample::of(&r));
                kin_killers.push(g);
            }
        }
        if let Some(tracker) = fullness.as_mut() {
            let intakes = tick_intakes(&pre, params, &tail);
            let consumers: Vec<u64> = if world.tick() >= window_start {
                world
                    .agents()
                    .iter()
                    .filter(|a| {
                        intakes.contains_key(&a.id)
                            && diet_group(&a.traits, &ledger.income(a.id))
                                == DietGroup::TraitHeterotrophDietFed
                    })
                    .map(|a| a.id)
                    .collect()
            } else {
                Vec::new()
            };
            tracker.observe(&intakes, &tail, &kin_killers, &consumers);
        }
        topology.update(world.event_log());
        let sampled_before = observations.role_snapshots.len();
        observations.observe(&world, interval);
        if observations.role_snapshots.len() > sampled_before {
            let tick = world.tick();
            if tick >= window_start {
                let tags: HashMap<u64, TrophicRole> = world
                    .agents()
                    .iter()
                    .map(|a| (a.id, trait_tag(a.id, &a.traits, &topology)))
                    .collect();
                let mut diet = HashMap::with_capacity(tags.len());
                for (&id, &tag) in &tags {
                    let income = ledger.income(id);
                    let role = diet_role(&income);
                    confusion.add(tag, role);
                    if tag != TrophicRole::Producer
                        && let Some(share) = income.light_share()
                    {
                        let bin =
                            ((share * LIGHT_SHARE_BINS as f64) as usize).min(LIGHT_SHARE_BINS - 1);
                        light_share[bin] += 1;
                    }
                    if let Some(role) = role {
                        diet.insert(id, role);
                    }
                }
                diet_snapshots.push((tick, diet));
                if let Some((_, roles)) = observations.role_snapshots.last() {
                    for (a, fill) in pre.earmark_fills(params) {
                        if tags.contains_key(&a.id) {
                            let s = Surplus::of(&a, params);
                            surplus.record(roles.get(&a.id).copied(), a.traits.heterotrophy, &s);
                            let group = diet_group(&a.traits, &ledger.income(a.id));
                            fills.group_mut(group).record_fill(fill);
                        }
                    }
                    let traits: HashMap<u64, TraitVector> =
                        world.agents().iter().map(|a| (a.id, a.traits)).collect();
                    for (id, reading) in intake_readings(&pre, params, &tail) {
                        if !tags.contains_key(&id) {
                            continue;
                        }
                        let Some(t) = traits.get(&id) else { continue };
                        let group = diet_group(t, &ledger.income(id));
                        intake.record(roles.get(&id).copied(), t.heterotrophy, group, &reading);
                        if group == DietGroup::TraitHeterotrophDietFed {
                            gates.diet_fed.push(GateSample::of(&reading));
                        }
                    }
                }
                tag_snapshots.push((tick, tags));
            }
        }
        world.compact_event_log_before(cursor.min(observations.consumed_events()));
        cursor = world.event_log().len();
        match explorers_genesis_eval::early_stop(
            world.agents().len(),
            &observations,
            eval_config,
            sustainable_stock,
        ) {
            None => {}
            Some(FailureMode::Extinction) | Some(FailureMode::PopulationExplosion) => break,
            Some(failure) => {
                stopped = Some(failure);
                break;
            }
        }
    }

    let (failure, termination_tick, tag_guilds, diet_guilds) = match stopped {
        // An early stop has no settled window to read: no guild (#527).
        Some(failure) => (Some(failure), world.tick(), [false; 3], [false; 3]),
        None => {
            let failure = census_failure(&world, &observations, eval_config, max_ticks, None);
            let tag = role_guilds_from_samples(&tag_snapshots, &observations.born, max_ticks);
            let diet = role_guilds_from_samples(&diet_snapshots, &observations.born, max_ticks);
            let (tag, diet) = (guild_flags(tag), guild_flags(diet));
            (failure, world.tick(), tag, diet)
        }
    };
    let (trophic_balance_tag, trophic_balance_diet) = if world.agents().is_empty() {
        (None, None)
    } else {
        let agents = world.agents();
        let energies: Vec<f32> = agents.iter().map(|a| a.energy()).collect();
        let tag_roles: Vec<TrophicRole> = agents
            .iter()
            .map(|a| trait_tag(a.id, &a.traits, &topology))
            .collect();
        let (diet_roles, diet_energies): (Vec<TrophicRole>, Vec<f32>) = agents
            .iter()
            .filter_map(|a| diet_role(&ledger.income(a.id)).map(|r| (r, a.energy())))
            .unzip();
        (
            Some(producer_energy_share(&tag_roles, &energies)),
            Some(producer_energy_share(&diet_roles, &diet_energies)),
        )
    };
    SeedDiet {
        seed,
        failure: failure.as_ref().map(|f| failure_label(f).to_string()),
        termination_tick,
        tag_guilds,
        diet_guilds,
        confusion,
        heterotroph_light_share: light_share,
        deaths: *ledger.deaths(),
        trophic_balance_tag,
        trophic_balance_diet,
        kills: *ledger.kills(),
        births: *ledger.births(),
        surplus,
        reproduction: {
            let mut r = *ledger.reproduction();
            r.merge(&fills);
            r
        },
        intake,
        gates,
        fullness: fullness.map(FullnessTracker::finish),
    }
}

/// Producer share of living energy, each agent bucketed by `roles` (parallel
/// to `energies`); 0 with no energy. The evaluator's trophic-balance term
/// until #602 retired it from fitness — kept here so the #596 census still
/// reads the tag-vs-diet comparison it was built for.
fn producer_energy_share(roles: &[TrophicRole], energies: &[f32]) -> f32 {
    let total: f32 = energies.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    let producer: f32 = roles
        .iter()
        .zip(energies)
        .filter(|(role, _)| **role == TrophicRole::Producer)
        .map(|(_, &e)| e)
        .sum();
    producer / total
}

/// Every agent id the ledger still tracks as alive (test support).
#[cfg(test)]
fn tracked(ledger: &DietLedger) -> std::collections::HashSet<u64> {
    ledger.traits.keys().copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_source::{ConfigSource, resolve_config, sampled_units};
    use crate::grazer_hunger::EXPRESSION_BANDS;
    use crate::search::default_ranges;

    fn event(tick: u64, kind: EventKind, source: u64, target: Option<u64>, e: f32) -> Event {
        Event {
            tick,
            seq: 0,
            kind,
            source,
            target,
            energy_delta: e,
            position: None,
            target_was_carcass: false,
            second_parent: None,
        }
    }

    fn sample_31() -> (WorldParameters, InitialDistribution) {
        resolve_config(
            ConfigSource::SAMPLE,
            31,
            &Default::default(),
            &sampled_units(default_ranges().len()),
        )
    }

    /// The ledger books light and drained energy per agent, records descent,
    /// and attributes a death: infant, grazed alive in its death tick, and by
    /// kin when a grazer is its parent, offspring or sibling.
    #[test]
    fn ledger_books_income_descent_and_attributes_a_kin_grazed_infant_death() {
        let (params, dist) = sample_31();
        let world = World::new(params, dist, 1000);
        let mut ledger = DietLedger::new(&world);
        let ids: Vec<u64> = world.agents().iter().map(|a| a.id).collect();
        let (parent, stranger) = (ids[0], ids[2]);
        let child = 10_000;
        let mut born = event(5, EventKind::Born, child, Some(parent), 0.0);
        born.second_parent = None;
        ledger.ingest(&world, &[born], None);
        ledger.traits.insert(child, world.agents()[0].traits);
        let mut carcass = event(6, EventKind::Consumed, parent, Some(99), 0.5);
        carcass.target_was_carcass = true;
        ledger.ingest(
            &world,
            &[
                event(6, EventKind::Photosynthesized, parent, None, 2.0),
                carcass,
                event(6, EventKind::Consumed, parent, Some(child), 0.25),
                event(6, EventKind::Consumed, stranger, Some(child), 0.25),
                event(6, EventKind::Died, child, None, 0.0),
            ],
            None,
        );
        assert_eq!(
            ledger.income(parent),
            Income {
                light: 2.0,
                living: 0.25,
                carcass: 0.5
            }
        );
        assert!(ledger.is_kin(child, parent) && ledger.is_kin(parent, child));
        assert!(!ledger.is_kin(child, stranger));
        let d = ledger.deaths();
        let all = [
            d.trait_producer,
            d.trait_heterotroph_light_fed,
            d.trait_heterotroph_diet_fed,
            d.trait_heterotroph_no_income,
        ];
        let total: u64 = all.iter().map(|c| c.deaths).sum();
        assert_eq!(total, 1);
        let c = all.iter().find(|c| c.deaths == 1).unwrap();
        assert_eq!(
            (c.infant, c.grazed, c.grazed_by_kin, c.infant_grazed_by_kin),
            (1, 1, 1, 1)
        );
        assert!(!tracked(&ledger).contains(&child), "the dead are dropped");
        // Siblings share a parent.
        ledger.ingest(
            &world,
            &[
                event(7, EventKind::Born, 10_001, Some(parent), 0.0),
                event(7, EventKind::Born, 10_002, Some(parent), 0.0),
            ],
            None,
        );
        assert!(ledger.is_kin(10_001, 10_002));
    }

    /// Births are booked to each parent's diet group at the birth — the
    /// deaths table's classification (trait producer; trait heterotroph fed
    /// on light, on drained energy, or with no income yet) — once per parent.
    #[test]
    fn ledger_books_births_by_the_parent_s_diet_group() {
        let (params, dist) = sample_31();
        let world = World::new(params, dist, 1000);
        let mut ledger = DietLedger::new(&world);
        let ids: Vec<u64> = world.agents().iter().map(|a| a.id).collect();
        let base = world.agents()[0].traits;
        let producer = TraitVector {
            photosynthetic_absorption: 0.9,
            heterotrophy: 0.1,
            ..base
        };
        let heterotroph = TraitVector {
            photosynthetic_absorption: 0.1,
            heterotrophy: 0.9,
            ..base
        };
        let (p, fed, lit, unfed) = (ids[0], ids[1], ids[2], ids[3]);
        ledger.traits.insert(p, producer);
        for h in [fed, lit, unfed] {
            ledger.traits.insert(h, heterotroph);
        }
        ledger.ingest(
            &world,
            &[
                event(5, EventKind::Consumed, fed, Some(ids[4]), 1.0),
                event(5, EventKind::Photosynthesized, lit, None, 1.0),
            ],
            None,
        );
        let mut sexual = event(6, EventKind::Born, 20_003, Some(fed), 0.0);
        sexual.second_parent = Some(unfed);
        ledger.ingest(
            &world,
            &[
                event(6, EventKind::Born, 20_000, Some(p), 0.0),
                event(6, EventKind::Born, 20_001, Some(fed), 0.0),
                event(6, EventKind::Born, 20_002, Some(lit), 0.0),
                sexual,
            ],
            None,
        );
        let r = ledger.reproduction();
        assert_eq!(
            [
                r.trait_producer.births,
                r.trait_heterotroph_light_fed.births,
                r.trait_heterotroph_diet_fed.births,
                r.trait_heterotroph_no_income.births,
            ],
            [1, 1, 2, 1]
        );
    }

    /// Earmark fill by diet group is a mean over agent-samples; merging adds
    /// births, samples and fill; rows written before #624 read back empty.
    #[test]
    fn reproduction_by_diet_means_earmark_fill_over_samples() {
        let mut r = ReproductionTable::default();
        assert_eq!(r.trait_heterotroph_diet_fed.mean_earmark_fill(), None);
        r.trait_heterotroph_diet_fed.record_fill(0.5);
        r.trait_heterotroph_diet_fed.record_fill(0.0);
        r.trait_producer.record_fill(2.0);
        assert_eq!(r.trait_heterotroph_diet_fed.mean_earmark_fill(), Some(0.25));
        let mut sum = r;
        sum.merge(&r);
        assert_eq!(sum.trait_heterotroph_diet_fed.samples, 4);
        assert_eq!(sum.trait_producer.mean_earmark_fill(), Some(2.0));
    }

    /// On a real seed the rollout books every birth to a diet group (one per
    /// parent, so at least the parent–offspring distances recorded), samples
    /// earmark fill on the same agents as the surplus read, and rows written
    /// before #624 read back with an empty table.
    #[test]
    fn rollout_records_reproduction_by_diet_group() {
        let (params, dist) = sample_31();
        let eval = EvalConfig::default();
        let ours = rollout(&params, &dist, 1000, 300, &eval);
        let r = ours.reproduction;
        let groups = [
            r.trait_producer,
            r.trait_heterotroph_light_fed,
            r.trait_heterotroph_diet_fed,
            r.trait_heterotroph_no_income,
        ];
        let births: u64 = groups.iter().map(|g| g.births).sum();
        assert!(births >= ours.births.count() && births > 0, "{r:?}");
        let samples: u64 = groups.iter().map(|g| g.samples).sum();
        let surplus: u64 = ours.surplus.roles.iter().map(|d| d.count()).sum();
        assert_eq!(samples, surplus);
        assert!(groups.iter().any(|g| g.earmark_fill > 0.0), "{r:?}");
        let mut json: serde_json::Value = serde_json::to_value(&ours).unwrap();
        json.as_object_mut().unwrap().remove("reproduction");
        let old: SeedDiet = serde_json::from_value(json).unwrap();
        assert_eq!(old.reproduction, ReproductionTable::default());
    }

    /// The census's rollout is the genesis rollout: on real seeds it reaches
    /// the same verdict and termination tick as `explorers_genesis::rollout`,
    /// its diet read agrees with the evaluator's income role on both
    /// heterotroph guilds (#599), and it is deterministic.
    #[test]
    fn rollout_reproduces_the_genesis_verdict_and_its_diet_read_agrees_with_the_evaluator() {
        let (params, dist) = sample_31();
        let eval = EvalConfig::default();
        let horizon = 200;
        let mut live = 0;
        let mut producer_guilds = 0;
        for seed in [1000, 1001] {
            let ours = rollout(&params, &dist, seed, horizon, &eval);
            let run_config = explorers_genesis::RunConfig {
                max_ticks: horizon,
                eval_config: eval.clone(),
                early_stop_crosscheck_fraction: 0.0,
            };
            let genesis = explorers_genesis::rollout(&params, &dist, &run_config, seed);
            let theirs = genesis.result;
            assert_eq!(
                ours.failure,
                theirs
                    .failure
                    .as_ref()
                    .map(|f| failure_label(f).to_string())
            );
            assert_eq!(ours.termination_tick, theirs.termination_tick);
            assert_eq!(ours.diet_guilds[1], theirs.breakdown.has_consumer_guild);
            assert_eq!(ours.diet_guilds[2], theirs.breakdown.has_decomposer_guild);
            if theirs.failure.is_none() {
                // All three roles, the producer guild included, on the
                // evaluator's own role snapshots.
                let evaluator = role_guilds_from_samples(
                    &genesis.observations.role_snapshots,
                    &genesis.observations.born,
                    horizon,
                );
                assert_eq!(ours.diet_guilds, guild_flags(evaluator));
                producer_guilds += usize::from(evaluator.producer);
                live += 1;
            }
            assert_eq!(ours, rollout(&params, &dist, seed, horizon, &eval));
            // Every second-half sample is in the confusion table.
            let samples: u64 = ours.confusion.0.iter().flatten().sum();
            assert!(samples > 0, "{ours:?}");
        }
        assert!(live > 0, "a live seed pins the guild read");
        assert!(
            producer_guilds > 0,
            "a guild that holds is pinned, not only absences"
        );
    }

    /// The census reads every second-half sampled agent's surplus satiation
    /// before growth (#622), by its recent-income role (#599), with the
    /// light-fed mixotrophs — income producers with heterotrophy above zero —
    /// apart; old rows without it still read back.
    #[test]
    fn rollout_records_surplus_by_income_role() {
        let (params, dist) = sample_31();
        let eval = EvalConfig::default();
        let ours = rollout(&params, &dist, 1000, 300, &eval);
        let sp = &ours.surplus;
        let by_role: u64 = sp.roles.iter().map(|d| d.count()).sum();
        let samples: u64 = ours.confusion.0.iter().flatten().sum();
        assert!(by_role > 0 && by_role <= samples, "{by_role} of {samples}");
        let producers = sp.roles[0].count();
        let mixotrophs = sp.light_fed_mixotrophs.count();
        assert!(
            mixotrophs > 0 && mixotrophs <= producers,
            "{mixotrophs} of {producers}"
        );
        assert!(sp.light_fed_mixotrophs.percentile(0.5).is_some());
        // The energy side alone, for the same agents: never below the
        // co-limited read.
        let energy = &sp.light_fed_mixotrophs_energy;
        assert_eq!(energy.count(), mixotrophs);
        assert!(energy.zero <= sp.light_fed_mixotrophs.zero);
        assert!(energy.percentile(0.5) >= sp.light_fed_mixotrophs.percentile(0.5));

        let mut json: serde_json::Value = serde_json::to_value(&ours).unwrap();
        json.as_object_mut().unwrap().remove("surplus");
        let old: SeedDiet = serde_json::from_value(json).unwrap();
        assert_eq!(old.surplus, SurplusByRole::default());
        assert_eq!(
            SeedDiet {
                surplus: ours.surplus.clone(),
                ..old
            },
            ours
        );
    }

    /// The census reads each second-half sampled agent's intake at the
    /// drain pass's start (#629): light-fed mixotrophs are the surplus read's
    /// (the same agent-samples), heterotrophs by diet the reproduction
    /// table's; old rows without it still read back.
    #[test]
    fn rollout_records_intake_by_population() {
        let (params, dist) = sample_31();
        let eval = EvalConfig::default();
        let ours = rollout(&params, &dist, 1000, 300, &eval);
        let ic = &ours.intake;
        assert!(ic.mixotroph_light.count() > 0);
        assert_eq!(
            ic.mixotroph_light.count(),
            ours.surplus.light_fed_mixotrophs.count()
        );
        assert_eq!(ic.mixotroph_uptake.count(), ic.mixotroph_light.count());
        let diet_fed = ours.reproduction.trait_heterotroph_diet_fed.samples;
        assert_eq!(ic.heterotroph_intake.count(), diet_fed);
        assert_eq!(ic.heterotroph_potential.count(), diet_fed);

        let mut json: serde_json::Value = serde_json::to_value(&ours).unwrap();
        json.as_object_mut().unwrap().remove("intake");
        let old: SeedDiet = serde_json::from_value(json).unwrap();
        assert_eq!(old.intake, IntakeCensus::default());
        assert_eq!(
            SeedDiet {
                intake: ours.intake.clone(),
                ..old
            },
            ours
        );
    }

    /// The census keeps a gate sample (#634) per heterotroph-by-diet
    /// agent-sample (the #629 intake population, one for one) and per
    /// kin-killing pair whose killer is a light-fed mixotroph: a subset of
    /// the kin pairs, read exactly as the #629 killer bands read them. Old
    /// rows without them still read back.
    #[test]
    fn rollout_keeps_gate_samples_for_both_populations() {
        let (params, dist) = sample_31();
        let eval = EvalConfig::default();
        let k = 1.7;
        let ours = rollout_with(&params, &dist, 1000, 300, &eval, Some(k));
        let g = &ours.gates;
        assert_eq!(
            g.diet_fed.len() as u64,
            ours.intake.heterotroph_intake.count()
        );
        let kills = &g.mixotroph_kin_kills;
        assert!(!kills.is_empty(), "sample:31's mixotrophs kill kin");
        assert!(kills.len() as u64 <= ours.kills.total(true));
        assert!(kills.iter().all(|s| s.maintenance > 0.0));
        let mut bands = [0u64; EXPRESSION_BANDS];
        for s in kills {
            bands[crate::grazer_hunger::expression_band(s.expression(k, 0.0))] += 1;
        }
        for (b, n) in bands.iter().enumerate() {
            assert!(*n <= ours.kills.intake_expression[1][b], "{bands:?}");
        }

        let mut json: serde_json::Value = serde_json::to_value(&ours).unwrap();
        json.as_object_mut().unwrap().remove("gates");
        let old: SeedDiet = serde_json::from_value(json).unwrap();
        assert_eq!(old.gates, GateSamples::default());
        assert_eq!(
            SeedDiet {
                gates: ours.gates.clone(),
                ..old
            },
            ours
        );
    }

    /// The fullness readout (#637) is observer-only: turning it on leaves
    /// every other field of the rollout identical. It counts exactly the
    /// kin kills the gate samples keep (the same #634 classification), reads
    /// heterotrophs by diet on every window tick (at least the sampled
    /// ones), and old rows without it read back.
    #[test]
    fn the_fullness_readout_leaves_the_rollout_identical() {
        let (params, dist) = sample_31();
        let eval = EvalConfig::default();
        let off = rollout_with(&params, &dist, 1000, 300, &eval, None);
        let on = rollout_with_fullness(&params, &dist, 1000, 300, &eval, None, true);
        assert_eq!(off.fullness, None);
        let f = on.fullness.clone().expect("the readout ran");
        assert_eq!(
            SeedDiet {
                fullness: None,
                ..on.clone()
            },
            off
        );
        assert_eq!(f.kills, on.gates.mixotroph_kin_kills.len() as u64);
        assert!(f.kills > 0);
        assert!(f.consumer_ticks >= on.gates.diet_fed.len() as u64);
        assert!(f.killers.agents > 0 && f.killers.agents <= f.kills);

        let json = serde_json::to_string(&off).unwrap();
        assert!(
            !json.contains("fullness"),
            "rows without it write as before"
        );
        assert_eq!(serde_json::from_str::<SeedDiet>(&json).unwrap(), off);
        let back: SeedDiet = serde_json::from_str(&serde_json::to_string(&on).unwrap()).unwrap();
        assert_eq!(back.fullness.map(|g| g.kills), Some(f.kills));
    }

    /// With an intake-ceiling `k` (#629), every killing pair is also banded
    /// by the grazer's predicted intake-gate expression at that `k`; without
    /// one, nothing is banded and the rollout is otherwise identical.
    #[test]
    fn rollout_bands_killing_grazers_by_predicted_intake_expression() {
        let (params, dist) = sample_31();
        let eval = EvalConfig::default();
        let plain = rollout(&params, &dist, 1000, 300, &eval);
        assert_eq!(plain.kills.intake_expression, [[0; EXPRESSION_BANDS]; 2]);
        let at_k = rollout_with(&params, &dist, 1000, 300, &eval, Some(4.0));
        let k = at_k.kills;
        for kin in [false, true] {
            let banded: u64 = k.intake_expression[usize::from(kin)].iter().sum();
            assert_eq!(banded, k.total(kin), "{k:?}");
        }
        assert!(k.total(true) + k.total(false) > 0);
        let tight = rollout_with(&params, &dist, 1000, 300, &eval, Some(1e-3)).kills;
        assert!(
            tight.intake_expression[0][0] + tight.intake_expression[1][0]
                >= k.intake_expression[0][0] + k.intake_expression[1][0],
            "a tighter ceiling gates no less"
        );
        assert_eq!(
            SeedDiet {
                kills: GrazerHunger {
                    intake_expression: Default::default(),
                    ..k
                },
                ..at_k
            },
            plain
        );
    }

    /// The proposed default sensitivity is `1 / s₂₅` over the light-fed
    /// mixotrophs with positive surplus; zero-surplus mixotrophs and other
    /// roles do not move it.
    #[test]
    fn proposed_sensitivity_is_the_inverse_positive_mixotroph_quartile() {
        let surplus = |t: f32| Surplus {
            energy: t,
            nutrient: t,
        };
        let mut sp = SurplusByRole::default();
        assert_eq!(sp.proposed_sensitivity(), None);
        for i in 1..=400 {
            sp.record(Some(TrophicRole::Producer), 0.1, &surplus(i as f32 * 0.1));
        }
        let c = sp.proposed_sensitivity().unwrap();
        for _ in 0..1000 {
            sp.record(Some(TrophicRole::Producer), 0.1, &surplus(0.0));
            sp.record(Some(TrophicRole::Consumer), 0.9, &surplus(100.0));
            sp.record(Some(TrophicRole::Producer), 0.0, &surplus(100.0));
        }
        assert_eq!(sp.proposed_sensitivity(), Some(c));
        assert!((c * 10.0 - 1.0).abs() < 0.15, "{c}");
    }

    /// The census reads each killing graze's grazer at drain time (#606):
    /// every grazed death contributes at least one (grazer, victim) pair and
    /// every kin-grazed death a kin pair; births are read against their
    /// parents' traits; and the read is deterministic and leaves the rollout
    /// untouched.
    #[test]
    fn rollout_tallies_killing_grazers_hunger_and_birth_distances() {
        let (params, dist) = sample_31();
        let eval = EvalConfig::default();
        let ours = rollout(&params, &dist, 1000, 300, &eval);
        let d = ours.deaths;
        let all = [
            d.trait_producer,
            d.trait_heterotroph_light_fed,
            d.trait_heterotroph_diet_fed,
            d.trait_heterotroph_no_income,
        ];
        let grazed: u64 = all.iter().map(|c| c.grazed).sum();
        let by_kin: u64 = all.iter().map(|c| c.grazed_by_kin).sum();
        let k = ours.kills;
        assert!(grazed > 0 && by_kin > 0, "{d:?}");
        assert!(k.total(true) + k.total(false) >= grazed, "{k:?}");
        assert!(k.total(true) >= by_kin, "{k:?}");
        let births = ours.births;
        assert!(births.count() > 0);
        assert!(births.mean().unwrap() < 0.5, "{births:?}");
        assert_eq!(ours, rollout(&params, &dist, 1000, 300, &eval));
    }

    /// #645's persistence readout pools the census's per-seed verdicts:
    /// seeds per failure label, "persisted" for a live run, in a stable
    /// order (persisted first, then the labels alphabetically).
    #[test]
    fn outcomes_pool_seed_verdicts_by_failure_label() {
        let mut a = Outcomes::default();
        a.record(None);
        a.record(Some(&FailureMode::Monoculture));
        a.record(None);
        let mut b = Outcomes::default();
        b.record(Some(&FailureMode::BloomStop));
        b.record(Some(&FailureMode::Monoculture));
        a.merge(&b);
        assert_eq!(a.seeds(), 5);
        assert_eq!(a.persisted(), 2);
        assert_eq!(a.describe(), "persisted 2, bloom_stop 1, monoculture 2");
        assert_eq!(Outcomes::default().describe(), "–");
        let mut dead = Outcomes::default();
        dead.record(Some(&FailureMode::Extinction));
        assert_eq!(dead.persisted(), 0);
        assert_eq!(dead.describe(), "extinction 1");
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(serde_json::from_str::<Outcomes>(&json).unwrap(), a);
    }
}
