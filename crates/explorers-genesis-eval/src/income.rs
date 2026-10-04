//! Trophic role read from recent realised income (#599).
//!
//! An agent's **trophic role** (CONTEXT.md) is a reading of what it lives on,
//! never of its trait vector — that is its **investment profile**, which only
//! the generalist-dominance gate reads. The [`IncomeLedger`] books each
//! agent's income by source off the rollout's observed events — light
//! (`Photosynthesized`), living targets and carcasses (`Consumed`, split by
//! `target_was_carcass`) — over a window that decays with half-life
//! [`INCOME_HALF_LIFE`], so a role follows a diet change. [`Income::role`]
//! reads it: a producer when light is at least [`PRODUCER_LIGHT_SHARE`] of
//! recent income, else a consumer or decomposer by the living / carcass split.
//! An agent with no income yet has no role (no default).
//!
//! Heterotrophic income is booked as energy *drained* (`Consumed`'s
//! `energy_delta`, before the trophic transfer efficiency), so the light share
//! is a lower bound on what the agent kept from light — the conservative
//! direction for calling an agent a producer.
//!
//! Observer-side only: the ledger lives with the evaluator, never in the
//! stepper, and is fed the log tail after each step like the topology
//! projection.

use std::collections::HashMap;

use explorers_sim::event::{EventKind, EventLog};
use explorers_sim::topology::{DETRITAL_RELIANCE_THRESHOLD, TrophicRole};

/// Light share of recent income at or above which an agent is a producer.
pub const PRODUCER_LIGHT_SHARE: f64 = 0.5;

/// An agent's energy income by source.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Income {
    /// Energy fixed from light (`Photosynthesized`).
    pub light: f64,
    /// Energy drained from living agents (`Consumed`, living target).
    pub living: f64,
    /// Energy drained from carcasses (`Consumed`, carcass target).
    pub carcass: f64,
}

impl Income {
    pub fn total(&self) -> f64 {
        self.light + self.living + self.carcass
    }

    /// Light's share of the income; `None` with no income.
    pub fn light_share(&self) -> Option<f64> {
        let total = self.total();
        (total > 0.0).then(|| self.light / total)
    }

    /// The trophic role this income reads as: `None` with no income. Light at
    /// or above [`PRODUCER_LIGHT_SHARE`] reads producer; otherwise the drained
    /// income splits decomposer / consumer at
    /// [`DETRITAL_RELIANCE_THRESHOLD`] of it coming from carcasses.
    pub fn role(&self) -> Option<TrophicRole> {
        let share = self.light_share()?;
        if share >= PRODUCER_LIGHT_SHARE {
            return Some(TrophicRole::Producer);
        }
        let drained = self.living + self.carcass;
        Some(
            if self.carcass / drained >= DETRITAL_RELIANCE_THRESHOLD as f64 {
                TrophicRole::Decomposer
            } else {
                TrophicRole::Consumer
            },
        )
    }
}

/// Half-life, in ticks, of the recent-income window: income booked `h` ticks
/// ago weighs half what income booked now does. A starting value, not a
/// settled one — long enough to average over the tick-to-tick flicker of
/// need-gated consumption, short enough that a diet change moves the role
/// within a few hundred ticks of a 2000-tick run.
pub const INCOME_HALF_LIFE: f64 = 50.0;

/// One agent's recent income, decayed to the tick of its last booking.
#[derive(Clone, Copy, Debug, Default)]
struct Entry {
    income: Income,
    as_of: u64,
}

impl Entry {
    /// Decay the booked income to `tick` (never backwards).
    fn advance(&mut self, tick: u64) {
        if tick > self.as_of {
            let factor = 0.5f64.powf((tick - self.as_of) as f64 / INCOME_HALF_LIFE);
            self.income.light *= factor;
            self.income.living *= factor;
            self.income.carcass *= factor;
            self.as_of = tick;
        }
    }
}

/// Per-agent income ledger, walked along an event log.
#[derive(Clone, Debug, Default)]
pub struct IncomeLedger {
    cursor: usize,
    income: HashMap<u64, Entry>,
}

impl IncomeLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Absorb the log tail since the last call.
    pub fn update(&mut self, log: &EventLog) {
        self.update_before(log, u64::MAX);
        self.cursor = log.len();
    }

    /// Absorb the log tail up to (excluding) events stamped `tick`, leaving
    /// the cursor there so a later call continues from the next event — the
    /// ledger as of a roster read at `World::tick() == tick`, for an observer
    /// walking a finished log sample by sample (as
    /// `TopologyProjection::update_before`).
    pub fn update_before(&mut self, log: &EventLog, tick: u64) {
        for event in log.since(self.cursor) {
            if event.tick >= tick {
                break;
            }
            match event.kind {
                EventKind::Photosynthesized => {
                    self.book(event.source, event.tick).light += event.energy_delta as f64;
                }
                EventKind::Consumed => {
                    let income = self.book(event.source, event.tick);
                    if event.target_was_carcass {
                        income.carcass += event.energy_delta as f64;
                    } else {
                        income.living += event.energy_delta as f64;
                    }
                }
                EventKind::Died => {
                    self.income.remove(&event.source);
                }
                _ => {}
            }
            self.cursor += 1;
        }
    }

    /// The agent's income, decayed to `tick`, ready to book at it.
    fn book(&mut self, id: u64, tick: u64) -> &mut Income {
        let entry = self.income.entry(id).or_insert(Entry {
            income: Income::default(),
            as_of: tick,
        });
        entry.advance(tick);
        &mut entry.income
    }

    /// How many agents the ledger holds income for: every agent that has
    /// booked income and not died.
    pub fn tracked(&self) -> usize {
        self.income.len()
    }

    /// The trophic roles of the given agents, keyed by id. An agent with no
    /// income yet has no role and is left out.
    pub fn roles_of(&self, ids: impl IntoIterator<Item = u64>) -> HashMap<u64, TrophicRole> {
        ids.into_iter()
            .filter_map(|id| self.role(id).map(|role| (id, role)))
            .collect()
    }

    /// The agent's recent income, as of its last booking; zero with none.
    /// Only its shares are meaningful across agents (each is decayed to its
    /// own last booking).
    pub fn income(&self, id: u64) -> Income {
        self.income.get(&id).map(|e| e.income).unwrap_or_default()
    }

    /// The agent's trophic role; `None` with no income yet.
    pub fn role(&self, id: u64) -> Option<TrophicRole> {
        self.income.get(&id).and_then(|e| e.income.role())
    }

    /// The agent's recent income decayed to `tick` (never backwards), so
    /// incomes read at one tick sum across agents.
    pub fn income_at(&self, id: u64, tick: u64) -> Income {
        self.income
            .get(&id)
            .map(|e| {
                let mut e = *e;
                e.advance(tick);
                e.income
            })
            .unwrap_or_default()
    }
}

/// A share per heterotroph **trophic role**. The producer share is the rest.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RoleShares {
    pub consumer: f32,
    pub decomposer: f32,
}

/// The heterotroph shares of a roster (expected-properties.md, *Trophic
/// structure*): of living energy and of recent income, each split by trophic
/// role. A reported per-seed observable under the heterotroph guilds'
/// authority boundary — never a behaviour axis, never a fitness term (#602).
/// Only agents with a role count, on either side of a share: one with no
/// income yet is on neither.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HeterotrophShares {
    pub energy: RoleShares,
    pub income: RoleShares,
}

impl HeterotrophShares {
    /// Read the shares of `roster` (each agent's id and living energy) off
    /// `ledger`, incomes decayed to `tick`. `None` when no agent has a role or
    /// the agents with one hold no energy.
    pub fn read(
        ledger: &IncomeLedger,
        roster: impl IntoIterator<Item = (u64, f32)>,
        tick: u64,
    ) -> Option<Self> {
        let mut energy = [0.0f64; 3];
        let mut income = [0.0f64; 3];
        let slot = |role| match role {
            TrophicRole::Producer => 0,
            TrophicRole::Consumer => 1,
            TrophicRole::Decomposer => 2,
        };
        for (id, e) in roster {
            let recent = ledger.income_at(id, tick);
            if let Some(role) = recent.role() {
                energy[slot(role)] += e as f64;
                income[slot(role)] += recent.total();
            }
        }
        let split = |v: [f64; 3]| {
            let total: f64 = v.iter().sum();
            RoleShares {
                consumer: (v[1] / total) as f32,
                decomposer: (v[2] / total) as f32,
            }
        };
        (energy.iter().sum::<f64>() > 0.0).then(|| HeterotrophShares {
            energy: split(energy),
            income: split(income),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use explorers_sim::event::{Event, EventKind, EventLog};
    use explorers_sim::topology::TrophicRole;

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
            nutrient_delta: 0.0,
        }
    }

    fn log(events: Vec<Event>) -> EventLog {
        let mut log = EventLog::new();
        for (seq, mut e) in events.into_iter().enumerate() {
            e.seq = seq as u64;
            log.append(e).unwrap();
        }
        log
    }

    #[test]
    fn an_agent_living_on_light_reads_as_a_producer_whatever_it_drains() {
        let log = log(vec![
            event(1, EventKind::Photosynthesized, 7, None, 2.0),
            event(1, EventKind::Consumed, 7, Some(3), 1.0),
        ]);
        let mut ledger = IncomeLedger::new();
        ledger.update(&log);
        assert_eq!(ledger.role(7), Some(TrophicRole::Producer));
    }

    #[test]
    fn an_agent_living_on_carcasses_reads_as_a_decomposer_and_a_newborn_has_no_role() {
        let mut carcass = event(1, EventKind::Consumed, 8, Some(99), 1.0);
        carcass.target_was_carcass = true;
        let log = log(vec![
            event(1, EventKind::Photosynthesized, 8, None, 0.2),
            carcass,
            event(1, EventKind::Consumed, 8, Some(3), 0.3),
            event(1, EventKind::Born, 9, Some(8), 0.0),
        ]);
        let mut ledger = IncomeLedger::new();
        ledger.update(&log);
        assert_eq!(ledger.role(8), Some(TrophicRole::Decomposer));
        assert_eq!(ledger.role(9), None, "no income yet, no role");
        assert_eq!(
            ledger.roles_of([8, 9]),
            HashMap::from([(8, TrophicRole::Decomposer)]),
            "an agent with no role is left out"
        );
    }

    #[test]
    fn role_follows_a_diet_change_within_the_window() {
        // Two half-lives on light, then one on living prey: lifetime light
        // still outweighs the new diet two to one, but the recent window has
        // moved on.
        let h = INCOME_HALF_LIFE as u64;
        let mut events: Vec<Event> = (1..=2 * h)
            .map(|t| event(t, EventKind::Photosynthesized, 5, None, 1.0))
            .collect();
        events.extend((2 * h + 1..=3 * h).map(|t| event(t, EventKind::Consumed, 5, Some(3), 1.0)));
        let mut ledger = IncomeLedger::new();
        ledger.update(&log(events[..2 * h as usize].to_vec()));
        assert_eq!(ledger.role(5), Some(TrophicRole::Producer));
        let mut ledger = IncomeLedger::new();
        ledger.update(&log(events));
        assert_eq!(ledger.role(5), Some(TrophicRole::Consumer));
    }

    #[test]
    fn the_dead_are_forgotten() {
        let log = log(vec![
            event(1, EventKind::Photosynthesized, 4, None, 1.0),
            event(2, EventKind::Died, 4, None, 0.0),
        ]);
        let mut ledger = IncomeLedger::new();
        ledger.update(&log);
        assert_eq!(ledger.role(4), None);
        assert_eq!(ledger.tracked(), 0);
    }

    #[test]
    fn income_role_reads_light_share_then_the_living_carcass_split() {
        let role = |light, living, carcass| {
            Income {
                light,
                living,
                carcass,
            }
            .role()
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
}
