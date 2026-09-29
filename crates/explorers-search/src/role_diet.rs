//! Trait-read role tag against realised diet (issue #596).
//!
//! The evaluator's role tag reads an agent as a heterotroph by *trait*
//! (heterotrophy > photosynthetic absorption) and splits heterotrophs into
//! consumer and decomposer by realised diet (`TopologyProjection::
//! trophic_roles_of`). #591 found tagged "consumers" living almost entirely on
//! light. This module books every agent's lifetime energy income by source —
//! light (`Photosynthesized`), living prey and carcasses (`Consumed`, split by
//! `target_was_carcass`) — and reads a **diet role** from it: a producer by
//! diet when light is at least [`PRODUCER_LIGHT_SHARE`] of its income, else a
//! consumer or decomposer by the same living/carcass split the tag uses.
//!
//! [`rollout`] mirrors the genesis rollout (`explorers_genesis::rollout`)
//! step for step: the same retention, the same `RolloutObservations`, the same
//! early stops and the same horizon verdict. On the evaluator's own
//! second-half role snapshots it also takes the diet role of every agent, so
//! the guild rule (`guild::role_guilds_from_samples`) reads the trait tag and
//! the diet role on identical samples. It also attributes every death:
//! age class, whether the agent was grazed alive in its death tick, and
//! whether a grazer was kin (parent, offspring or sibling, by `Born`).
//!
//! Heterotrophic income is booked as energy *drained* (`Consumed`'s
//! `energy_delta`, before the trophic transfer efficiency): the quantity the
//! tag's detrital-reliance split already reads, and an upper bound on what the
//! agent kept. A light share read against it is therefore a lower bound — the
//! conservative direction for asking whether a tagged heterotroph lives on
//! light.
//!
//! Observer-side only: no trajectory changes.

use std::collections::HashMap;

use explorers_genesis_eval::guild::{RoleGuilds, RoleSnapshot, role_guilds_from_samples};
use explorers_genesis_eval::{EvalConfig, FailureMode, RolloutObservations};
use explorers_sim::event::{Event, EventKind};
use explorers_sim::topology::{DETRITAL_RELIANCE_THRESHOLD, TrophicRole};
use explorers_sim::{InitialDistribution, TraitVector, World, WorldParameters};

/// Light share of lifetime income at or above which an agent is a producer by
/// diet. A starting value: the census reports the whole light-share
/// distribution so a reading does not hinge on it.
pub const PRODUCER_LIGHT_SHARE: f64 = 0.5;

/// A death at or below this age (ticks from birth) is an infant death, as
/// #591's accountant counts it.
pub const INFANT_AGE: u64 = 50;

/// Deciles of the light share.
pub const LIGHT_SHARE_BINS: usize = 10;

/// An agent's lifetime energy income by source.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Income {
    pub light: f64,
    /// Energy drained from living agents.
    pub living: f64,
    /// Energy drained from carcasses.
    pub carcass: f64,
}

impl Income {
    pub fn total(&self) -> f64 {
        self.light + self.living + self.carcass
    }

    /// Light's share of the income; `None` for an agent with no income yet.
    pub fn light_share(&self) -> Option<f64> {
        let total = self.total();
        (total > 0.0).then(|| self.light / total)
    }
}

/// The role an agent's realised diet reads as: `None` with no income yet.
/// Light at or above [`PRODUCER_LIGHT_SHARE`] reads producer; otherwise the
/// heterotrophic income splits consumer / decomposer at the tag's own
/// detrital-reliance threshold.
pub fn diet_role(income: &Income) -> Option<TrophicRole> {
    let share = income.light_share()?;
    if share >= PRODUCER_LIGHT_SHARE {
        return Some(TrophicRole::Producer);
    }
    let drained = income.living + income.carcass;
    Some(
        if income.carcass / drained >= DETRITAL_RELIANCE_THRESHOLD as f64 {
            TrophicRole::Decomposer
        } else {
            TrophicRole::Consumer
        },
    )
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
    /// The evaluator's guild read (the trait tag).
    pub tag_guilds: [bool; 3],
    /// The same rule on the same samples with every agent read by its diet
    /// role (an agent with no income yet keeps its tag).
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
    /// tag (the evaluator's read) and by diet role (an agent with no income
    /// keeps its tag). `None` for an empty roster.
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
        let sampled_before = observations.role_snapshots.len();
        observations.observe(&world, interval);
        if observations.role_snapshots.len() > sampled_before {
            let (tick, tags) = observations.role_snapshots.last().expect("just pushed");
            if *tick >= window_start {
                let mut diet = HashMap::with_capacity(tags.len());
                for (&id, &tag) in tags {
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
                    diet.insert(id, role.unwrap_or(tag));
                }
                diet_snapshots.push((*tick, diet));
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
            let tag = role_guilds_from_samples(
                &observations.role_snapshots,
                &observations.born,
                max_ticks,
            );
            let diet = role_guilds_from_samples(&diet_snapshots, &observations.born, max_ticks);
            debug_assert_eq!(
                (tag.consumer, tag.decomposer),
                (breakdown.has_consumer_guild, breakdown.has_decomposer_guild),
                "the tag read is the evaluator's"
            );
            let (tag, diet) = (guild_flags(tag), guild_flags(diet));
            (breakdown.failure, world.tick(), tag, diet)
        }
    };
    let (trophic_balance_tag, trophic_balance_diet) = if world.agents().is_empty() {
        (None, None)
    } else {
        let agents = world.agents();
        let tags = observations
            .topology()
            .trophic_roles_of(agents.iter().map(|a| (a.id, &a.traits)));
        let energies: Vec<f32> = agents.iter().map(|a| a.energy()).collect();
        let tag_roles: Vec<TrophicRole> = agents.iter().map(|a| tags[&a.id]).collect();
        let diet_roles: Vec<TrophicRole> = agents
            .iter()
            .map(|a| diet_role(&ledger.income(a.id)).unwrap_or(tags[&a.id]))
            .collect();
        (
            Some(explorers_genesis_eval::trophic_balance_score(
                &tag_roles, &energies,
            )),
            Some(explorers_genesis_eval::trophic_balance_score(
                &diet_roles,
                &energies,
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

    #[test]
    fn diet_role_reads_light_share_then_the_living_carcass_split() {
        let role = |light, living, carcass| {
            diet_role(&Income {
                light,
                living,
                carcass,
            })
        };
        assert_eq!(role(0.0, 0.0, 0.0), None);
        assert_eq!(role(9.99, 0.01, 0.0), Some(TrophicRole::Producer));
        assert_eq!(
            role(1.0, 1.0, 0.0),
            Some(TrophicRole::Producer),
            "at the share"
        );
        assert_eq!(role(0.9, 1.0, 0.1), Some(TrophicRole::Consumer));
        assert_eq!(role(0.0, 0.2, 0.8), Some(TrophicRole::Decomposer));
        assert_eq!(
            Income {
                light: 3.0,
                living: 1.0,
                carcass: 0.0
            }
            .light_share(),
            Some(0.75)
        );
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
    /// the same verdict, termination tick and heterotroph guild read as
    /// `explorers_genesis::rollout`, and it is deterministic.
    #[test]
    fn rollout_reproduces_the_genesis_verdict_and_guilds() {
        let (params, dist) = sample_31();
        let eval = EvalConfig::default();
        let horizon = 200;
        let mut balance_checked = 0;
        for seed in [1000, 1001] {
            let ours = rollout(&params, &dist, seed, horizon, &eval);
            let run_config = explorers_genesis::RunConfig {
                max_ticks: horizon,
                eval_config: eval.clone(),
                early_stop_crosscheck_fraction: 0.0,
            };
            let theirs = explorers_genesis::rollout(&params, &dist, &run_config, seed).result;
            assert_eq!(
                ours.failure,
                theirs
                    .failure
                    .as_ref()
                    .map(|f| failure_label(f).to_string())
            );
            assert_eq!(ours.termination_tick, theirs.termination_tick);
            assert_eq!(ours.tag_guilds[1], theirs.breakdown.has_consumer_guild);
            assert_eq!(ours.tag_guilds[2], theirs.breakdown.has_decomposer_guild);
            if theirs.failure.is_none() {
                assert_eq!(
                    ours.trophic_balance_tag,
                    Some(theirs.breakdown.trophic_balance_score),
                    "the tag balance is the evaluator's term"
                );
                balance_checked += 1;
            }
            assert_eq!(ours, rollout(&params, &dist, seed, horizon, &eval));
            // Every second-half sample is in the confusion table.
            let samples: u64 = ours.confusion.0.iter().flatten().sum();
            assert!(samples > 0, "{ours:?}");
        }
        assert!(balance_checked > 0, "a live seed pins the balance");
    }
}
