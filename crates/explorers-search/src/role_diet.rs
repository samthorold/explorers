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
}

impl DietLedger {
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
    /// attributed, so a grazer born this tick still counts as kin.
    pub fn ingest(&mut self, world: &World, events: &[Event]) {
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

    fn attribute_death(&mut self, id: u64, tick: u64, grazers: &[u64]) {
        let Some(traits) = self.traits.remove(&id) else {
            return;
        };
        let age = tick.saturating_sub(self.born_tick.get(&id).copied().unwrap_or(0));
        let grazed = !grazers.is_empty();
        let by_kin = grazers.iter().any(|&g| self.is_kin(id, g));
        let income = self.income.remove(&id).unwrap_or_default();
        let bucket = if traits.photosynthetic_absorption >= traits.heterotrophy {
            &mut self.deaths.trait_producer
        } else {
            match diet_role(&income) {
                Some(TrophicRole::Producer) => &mut self.deaths.trait_heterotroph_light_fed,
                Some(_) => &mut self.deaths.trait_heterotroph_diet_fed,
                None => &mut self.deaths.trait_heterotroph_no_income,
            }
        };
        bucket.add(age <= INFANT_AGE, grazed, by_kin);
    }

    pub fn income(&self, id: u64) -> Income {
        self.income.get(&id).copied().unwrap_or_default()
    }

    pub fn deaths(&self) -> &DeathTable {
        &self.deaths
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
    /// evaluator's trophic-balance term — with agents bucketed by the trait
    /// tag (the evaluator's read before #599) and by lifetime diet role (an
    /// agent with no income is left out). `None` for an empty roster.
    #[serde(default)]
    pub trophic_balance_tag: Option<f32>,
    #[serde(default)]
    pub trophic_balance_diet: Option<f32>,
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

/// One seed rolled out as the genesis rollout does it (unbudgeted, no
/// cross-check carry), with the diet census taken alongside.
pub fn rollout(
    params: &WorldParameters,
    dist: &InitialDistribution,
    seed: u64,
    max_ticks: u64,
    eval_config: &EvalConfig,
) -> SeedDiet {
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

    let mut ledger = DietLedger::new(&world);
    let mut topology = TopologyProjection::new();
    let mut tag_snapshots: Vec<RoleSnapshot> = Vec::new();
    let mut cursor = 0;
    let mut diet_snapshots: Vec<RoleSnapshot> = Vec::new();
    let mut confusion = Confusion::default();
    let mut light_share = [0u64; LIGHT_SHARE_BINS];
    let mut stopped: Option<FailureMode> = None;
    for _ in 0..max_ticks {
        world.step();
        let tail: Vec<Event> = world.event_log().since(cursor).to_vec();
        cursor = world.event_log().len();
        ledger.ingest(&world, &tail);
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
            let breakdown = explorers_genesis_eval::evaluate_from_log(
                &world,
                &observations,
                eval_config,
                max_ticks,
            );
            let tag = role_guilds_from_samples(&tag_snapshots, &observations.born, max_ticks);
            let diet = role_guilds_from_samples(&diet_snapshots, &observations.born, max_ticks);
            let (tag, diet) = (guild_flags(tag), guild_flags(diet));
            (breakdown.failure, world.tick(), tag, diet)
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
            Some(explorers_genesis_eval::trophic_balance_score(
                &tag_roles, &energies,
            )),
            Some(explorers_genesis_eval::trophic_balance_score(
                &diet_roles,
                &diet_energies,
            )),
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
    }
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
        ledger.ingest(&world, &[born]);
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
        );
        assert!(ledger.is_kin(10_001, 10_002));
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
}
