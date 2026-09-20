//! Heterotroph guild read (#490): a heterotroph *population* — sustained size
//! and recruitment — as distinct from a heterotroph *individual* carrying a
//! role tag. Reported only (genesis-search.md, the authority boundary): never
//! a behaviour axis, never a fitness term, never read by the robustness floor.

use std::collections::{HashMap, HashSet};

use explorers_sim::TraitVector;
use explorers_sim::event::{Event, EventKind, EventLog};
use explorers_sim::topology::{TopologyProjection, TrophicRole};

/// The living roster at one world tick, as `(id, traits)` pairs.
pub type RosterSnapshot = (u64, Vec<(u64, TraitVector)>);

/// Minimum role count a guild must hold on every sampled tick of the window.
/// A starting value (#443 §4.7), not a settled threshold — the atlas
/// regeneration that reports it is what tests it.
pub const GUILD_MIN_SIZE: usize = 5;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HeterotrophGuilds {
    pub consumer: bool,
    pub decomposer: bool,
}

/// The guild read for every trophic role. The evaluator reports only the two
/// heterotroph guilds ([`heterotroph_guilds`]); the producer read exists for
/// instruments that ask "is there a *population* of this role" of all three
/// roles by one rule (the invasion-growth presence gate, #493).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct RoleGuilds {
    pub producer: bool,
    pub consumer: bool,
    pub decomposer: bool,
}

impl RoleGuilds {
    pub fn get(self, role: TrophicRole) -> bool {
        match role {
            TrophicRole::Producer => self.producer,
            TrophicRole::Consumer => self.consumer,
            TrophicRole::Decomposer => self.decomposer,
        }
    }
}

/// Read the two heterotroph guilds off a finished run: [`role_guilds`]
/// restricted to consumer and decomposer.
pub fn heterotroph_guilds(
    log: &EventLog,
    roster_snapshots: &[RosterSnapshot],
    max_ticks: u64,
) -> HeterotrophGuilds {
    let g = role_guilds(log, roster_snapshots, max_ticks);
    HeterotrophGuilds {
        consumer: g.consumer,
        decomposer: g.decomposer,
    }
}

/// The living roster at one world tick, each agent read into its trophic
/// role by the projection as of that tick (`TopologyProjection::update_before`
/// then `trophic_roles_of`).
pub type RoleSnapshot = (u64, HashMap<u64, TrophicRole>);

/// The descent facts of one `Born` event — what the guild's recruitment clause
/// reads. Taken off the log at observation time so the rollout need not keep
/// the event itself (#502).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Birth {
    /// The tick the birth happened in (the event's stamp).
    pub tick: u64,
    /// The parent — for a sexual birth the seed parent.
    pub parent: Option<u64>,
    /// The mate of a sexual birth; `None` for an asexual one.
    pub second_parent: Option<u64>,
}

impl Birth {
    /// The descent facts of a `Born` event; `None` for any other kind.
    pub fn of(event: &Event) -> Option<Self> {
        (event.kind == EventKind::Born).then_some(Self {
            tick: event.tick,
            parent: event.target,
            second_parent: event.second_parent,
        })
    }
}

/// Read every role's guild off a finished run whose log still holds its
/// `Consumed` / `Reproduced` / `Died` / `Born` history: the projection is
/// walked to each second-half roster snapshot to classify it, and the rule is
/// [`role_guilds_from_samples`]. A rollout that drops history once read
/// classifies each sample as it is taken instead
/// (`RolloutObservations::observe`) and calls the rule directly.
pub fn role_guilds(
    log: &EventLog,
    roster_snapshots: &[RosterSnapshot],
    max_ticks: u64,
) -> RoleGuilds {
    let window_start = window_start(max_ticks);
    let mut topo = TopologyProjection::new();
    let role_snapshots: Vec<RoleSnapshot> = roster_snapshots
        .iter()
        .filter(|(t, _)| *t >= window_start)
        .map(|(tick, roster)| {
            topo.update_before(log, *tick);
            (
                *tick,
                topo.trophic_roles_of(roster.iter().map(|(id, tr)| (*id, tr))),
            )
        })
        .collect();
    let births: Vec<Birth> = log
        .by_kind(&EventKind::Born)
        .into_iter()
        .filter_map(Birth::of)
        .collect();
    role_guilds_from_samples(&role_snapshots, &births, max_ticks)
}

/// First tick of the guild window: the second half of the run.
fn window_start(max_ticks: u64) -> u64 {
    max_ticks / 2 + 1
}

/// The guild rule over classified samples. Membership is the `trophic_roles`
/// read (heterotroph by trait, consumer/decomposer by realised diet) on each
/// role snapshot whose tick falls in the second half of the run
/// (`tick > max_ticks / 2`). A guild holds when the role count reaches
/// [`GUILD_MIN_SIZE`] on every such sample **and** at least one birth in the
/// window names a member of the role (either parent) — which rules out a
/// long-lived sterile founder cohort sitting at exactly the floor. "Member"
/// is the union of the role over the window's samples, so a parent that bred
/// between two samples still counts as long as it was read in the role at one.
pub fn role_guilds_from_samples(
    role_snapshots: &[RoleSnapshot],
    births: &[Birth],
    max_ticks: u64,
) -> RoleGuilds {
    const ROLES: [TrophicRole; 3] = [
        TrophicRole::Producer,
        TrophicRole::Consumer,
        TrophicRole::Decomposer,
    ];
    let window_start = window_start(max_ticks);
    let mut sustained: HashMap<TrophicRole, bool> = ROLES.iter().map(|r| (*r, true)).collect();
    let mut members: HashMap<TrophicRole, HashSet<u64>> = HashMap::new();
    let mut sampled = false;
    for (_, roles) in role_snapshots.iter().filter(|(t, _)| *t >= window_start) {
        sampled = true;
        for (role, ok) in sustained.iter_mut() {
            let count = roles.values().filter(|r| *r == role).count();
            *ok &= count >= GUILD_MIN_SIZE;
        }
        for (id, role) in roles {
            members.entry(*role).or_default().insert(*id);
        }
    }
    // Recruitment: a birth is stamped with the tick it happened in, and the
    // world's counter advances after the step, so a birth stamped `t` is in the
    // window when the tick after it (`t + 1`) is.
    let recruited = |role: TrophicRole| {
        let Some(members) = members.get(&role) else {
            return false;
        };
        births.iter().any(|b| {
            b.tick + 1 >= window_start
                && [b.parent, b.second_parent]
                    .into_iter()
                    .flatten()
                    .any(|parent| members.contains(&parent))
        })
    };
    let guild = |role: TrophicRole| sampled && sustained[&role] && recruited(role);
    RoleGuilds {
        producer: guild(TrophicRole::Producer),
        consumer: guild(TrophicRole::Consumer),
        decomposer: guild(TrophicRole::Decomposer),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use explorers_sim::TraitVector;
    use explorers_sim::event::{Event, EventKind, EventLog};

    fn heterotroph(kappa: f32) -> TraitVector {
        TraitVector {
            photosynthetic_absorption: 0.2,
            heterotrophy: 1.0,
            mobility: 0.0,
            kappa,
            fecundity: 0.0,
            asexual_propensity: 0.0,
            dispersal: 0.0,
        }
    }

    fn producer() -> TraitVector {
        TraitVector {
            photosynthetic_absorption: 1.0,
            heterotrophy: 0.1,
            mobility: 0.0,
            kappa: 0.5,
            fecundity: 0.0,
            asexual_propensity: 0.0,
            dispersal: 0.0,
        }
    }

    fn event(tick: u64, seq: u64, kind: EventKind, source: u64, target: Option<u64>) -> Event {
        Event {
            tick,
            seq,
            kind,
            source,
            target,
            energy_delta: 1.0,
            position: None,
            target_was_carcass: false,
            second_parent: None,
        }
    }

    /// A log where each of `decomposers` drains a carcass on tick 0 (so realised
    /// diet reads them as decomposers from the first sample on).
    fn log_with_carcass_drains(decomposers: &[u64]) -> EventLog {
        let mut log = EventLog::new();
        for (i, &id) in decomposers.iter().enumerate() {
            let mut e = event(0, i as u64, EventKind::Consumed, id, Some(1_000 + id));
            e.target_was_carcass = true;
            log.append(e).unwrap();
        }
        log
    }

    const MAX_TICKS: u64 = 100;
    const INTERVAL: u64 = 10;

    /// Roster snapshots on the evaluator's cadence, every sample carrying the
    /// same roster.
    fn constant_roster(roster: &[(u64, TraitVector)]) -> Vec<RosterSnapshot> {
        (1..=MAX_TICKS / INTERVAL)
            .map(|k| (k * INTERVAL, roster.to_vec()))
            .collect()
    }

    #[test]
    fn one_long_lived_decomposer_is_not_a_guild() {
        let log = log_with_carcass_drains(&[7]);
        let roster: Vec<(u64, TraitVector)> = (1..=20)
            .map(|id| (id, producer()))
            .chain([(7, heterotroph(0.5))])
            .collect();
        let guilds = heterotroph_guilds(&log, &constant_roster(&roster), MAX_TICKS);
        assert!(!guilds.decomposer);
        assert!(!guilds.consumer);
    }

    #[test]
    fn five_sterile_decomposers_present_throughout_are_not_a_guild() {
        let ids: Vec<u64> = (101..=105).collect();
        let log = log_with_carcass_drains(&ids);
        let roster: Vec<(u64, TraitVector)> = (1..=20)
            .map(|id| (id, producer()))
            .chain(ids.iter().map(|&id| (id, heterotroph(1.0))))
            .collect();
        let guilds = heterotroph_guilds(&log, &constant_roster(&roster), MAX_TICKS);
        assert!(
            !guilds.decomposer,
            "size alone is not a guild: no recruitment"
        );
    }

    #[test]
    fn five_decomposers_on_every_sample_with_a_birth_in_the_window_are_a_guild() {
        let ids: Vec<u64> = (101..=106).collect();
        let mut log = log_with_carcass_drains(&ids);
        // A member breeds in the second half (stamped tick 70 → world tick 71).
        let mut born = event(70, 100, EventKind::Born, 107, Some(103));
        born.second_parent = Some(104);
        log.append(born).unwrap();
        let roster: Vec<(u64, TraitVector)> = (1..=20)
            .map(|id| (id, producer()))
            .chain(ids.iter().map(|&id| (id, heterotroph(0.4))))
            .collect();
        let guilds = heterotroph_guilds(&log, &constant_roster(&roster), MAX_TICKS);
        assert!(guilds.decomposer);
        assert!(!guilds.consumer);
    }

    #[test]
    fn a_birth_before_the_window_does_not_recruit() {
        let ids: Vec<u64> = (101..=106).collect();
        let mut log = log_with_carcass_drains(&ids);
        // Stamped tick 49 → world tick 50 = first half of a 100-tick run.
        log.append(event(49, 100, EventKind::Born, 107, Some(103)))
            .unwrap();
        let roster: Vec<(u64, TraitVector)> = (1..=20)
            .map(|id| (id, producer()))
            .chain(ids.iter().map(|&id| (id, heterotroph(0.4))))
            .collect();
        let guilds = heterotroph_guilds(&log, &constant_roster(&roster), MAX_TICKS);
        assert!(!guilds.decomposer);
    }

    #[test]
    fn a_guild_that_dies_out_in_the_second_half_is_not_a_guild() {
        let ids: Vec<u64> = (101..=106).collect();
        let mut log = log_with_carcass_drains(&ids);
        log.append(event(60, 100, EventKind::Born, 107, Some(103)))
            .unwrap();
        for (i, &id) in ids.iter().enumerate() {
            log.append(event(80, 200 + i as u64, EventKind::Died, id, None))
                .unwrap();
        }
        let full: Vec<(u64, TraitVector)> = (1..=20)
            .map(|id| (id, producer()))
            .chain(ids.iter().map(|&id| (id, heterotroph(0.4))))
            .collect();
        let producers_only: Vec<(u64, TraitVector)> = (1..=20).map(|id| (id, producer())).collect();
        let snapshots: Vec<RosterSnapshot> = (1..=MAX_TICKS / INTERVAL)
            .map(|k| {
                let tick = k * INTERVAL;
                let roster = if tick > 80 { &producers_only } else { &full };
                (tick, roster.clone())
            })
            .collect();
        let guilds = heterotroph_guilds(&log, &snapshots, MAX_TICKS);
        assert!(!guilds.decomposer);
    }

    /// The consumer twin: heterotrophs whose realised diet is living prey.
    fn log_with_living_drains(consumers: &[u64]) -> EventLog {
        let mut log = EventLog::new();
        for (i, &id) in consumers.iter().enumerate() {
            log.append(event(0, i as u64, EventKind::Consumed, id, Some(1)))
                .unwrap();
        }
        log
    }

    #[test]
    fn consumer_guild_reads_by_the_same_rule() {
        let ids: Vec<u64> = (101..=105).collect();
        let sterile = log_with_living_drains(&ids);
        let roster: Vec<(u64, TraitVector)> = (1..=20)
            .map(|id| (id, producer()))
            .chain(ids.iter().map(|&id| (id, heterotroph(0.4))))
            .collect();
        let no_births = heterotroph_guilds(&sterile, &constant_roster(&roster), MAX_TICKS);
        assert!(!no_births.consumer);
        assert!(!no_births.decomposer);

        let mut recruiting = log_with_living_drains(&ids);
        recruiting
            .append(event(90, 100, EventKind::Born, 106, Some(101)))
            .unwrap();
        let guilds = heterotroph_guilds(&recruiting, &constant_roster(&roster), MAX_TICKS);
        assert!(guilds.consumer);
        assert!(!guilds.decomposer);

        let four = &roster[..24];
        let too_small = heterotroph_guilds(&recruiting, &constant_roster(four), MAX_TICKS);
        assert!(
            !too_small.consumer,
            "four consumers are below GUILD_MIN_SIZE"
        );
    }

    #[test]
    fn producer_guild_reads_by_the_same_rule_through_role_guilds() {
        // Five producers on every sample; a birth in the window names one.
        let roster: Vec<(u64, TraitVector)> = (1..=5).map(|id| (id, producer())).collect();
        let mut recruiting = EventLog::new();
        recruiting
            .append(event(60, 0, EventKind::Born, 99, Some(1)))
            .unwrap();
        let guilds = role_guilds(&recruiting, &constant_roster(&roster), MAX_TICKS);
        assert!(guilds.producer);
        assert!(!guilds.consumer);
        assert!(!guilds.decomposer);
        assert_eq!(guilds.get(TrophicRole::Producer), true);
        // The heterotroph view of the same log ignores the producers.
        let h = heterotroph_guilds(&recruiting, &constant_roster(&roster), MAX_TICKS);
        assert_eq!(h, HeterotrophGuilds::default());
        // Sterile producers are not a guild.
        let sterile = EventLog::new();
        assert!(!role_guilds(&sterile, &constant_roster(&roster), MAX_TICKS).producer);
    }

    #[test]
    fn no_samples_in_the_window_reads_no_guild() {
        let ids: Vec<u64> = (101..=106).collect();
        let log = log_with_carcass_drains(&ids);
        let roster: Vec<(u64, TraitVector)> =
            ids.iter().map(|&id| (id, heterotroph(0.4))).collect();
        let first_half_only = vec![(10, roster.clone()), (20, roster)];
        let guilds = heterotroph_guilds(&log, &first_half_only, MAX_TICKS);
        assert_eq!(guilds, HeterotrophGuilds::default());
    }
}
