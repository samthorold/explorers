use std::collections::{HashMap, HashSet};

use crate::event::{EventKind, EventLog};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrophicRole {
    Producer,
    Consumer,
    Decomposer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum EdgeKind {
    /// Predation: energy drained from a living target (the green food web).
    Consumed,
    /// Decomposition: energy drained from a carcass (the brown/detrital food web).
    Decomposed,
}

/// Share of an agent's consumed energy that must flow through the detrital
/// (carcass) pathway for the readout to bucket it as a `Decomposer`. This is a
/// display constant for the debug instrument — the simulation itself has no
/// such threshold (continuum in the sim, buckets in the readout).
const DETRITAL_RELIANCE_THRESHOLD: f32 = 0.5;

pub struct TopologyProjection {
    cursor: usize,
    active_agents: HashSet<u64>,
    birth_tick: HashMap<u64, u64>,
    death_tick: HashMap<u64, u64>,
    edges: HashMap<(u64, u64, EdgeKind), f32>,
    lineage: HashMap<u64, (u64, u64)>,
    pending_parents: Option<(u64, u64)>,
}

impl TopologyProjection {
    pub fn new() -> Self {
        Self {
            cursor: 0,
            active_agents: HashSet::new(),
            birth_tick: HashMap::new(),
            death_tick: HashMap::new(),
            edges: HashMap::new(),
            lineage: HashMap::new(),
            pending_parents: None,
        }
    }

    pub fn update(&mut self, log: &EventLog) {
        for event in log.since(self.cursor) {
            match event.kind {
                EventKind::Reproduced => {
                    if let Some(target) = event.target {
                        // Reproduced with a target means parent pair info
                        self.pending_parents = Some((event.source, target));
                    } else {
                        // Reproduced without target means offspring born
                        self.active_agents.insert(event.source);
                        self.birth_tick.insert(event.source, event.tick);
                        if let Some(parents) = self.pending_parents.take() {
                            self.lineage.insert(event.source, parents);
                        }
                    }
                }
                EventKind::Died => {
                    self.active_agents.remove(&event.source);
                    self.death_tick.insert(event.source, event.tick);
                    let dead = event.source;
                    self.edges
                        .retain(|&(s, t, _), _| s != dead && t != dead);
                }
                EventKind::Consumed => {
                    if let Some(target) = event.target {
                        // Route by the raw interaction fact: carcass drains feed
                        // the brown (decomposition) web, living drains the green
                        // (predation) one.
                        let kind = if event.target_was_carcass {
                            EdgeKind::Decomposed
                        } else {
                            EdgeKind::Consumed
                        };
                        *self
                            .edges
                            .entry((event.source, target, kind))
                            .or_insert(0.0) += event.energy_delta;
                    }
                }
                _ => {}
            }
            self.cursor += 1;
        }
    }

    pub fn active_agents(&self) -> &HashSet<u64> {
        &self.active_agents
    }

    pub fn edge_weight(&self, source: u64, target: u64) -> f32 {
        // Total energy drained source -> target, across both predation and
        // decomposition (a given pair is only ever one or the other).
        self.edges
            .get(&(source, target, EdgeKind::Consumed))
            .copied()
            .unwrap_or(0.0)
            + self
                .edges
                .get(&(source, target, EdgeKind::Decomposed))
                .copied()
                .unwrap_or(0.0)
    }

    /// Classify each of the given agents into a trophic role, reading the green
    /// (predation) vs brown (decomposition) food web from accumulated consumption
    /// edges. The caller supplies the living population (the projection stores no
    /// trait state and seeded founders never emit a birth event), so roles are
    /// keyed to exactly those agents.
    ///
    /// **Producer** is a trait reading: an autotrophy-dominant agent is a producer
    /// regardless of any incidental consumption (no window artifact). Among
    /// heterotrophs, an agent is a **Decomposer** when detrital reliance reaches
    /// [`DETRITAL_RELIANCE_THRESHOLD`] of its consumed energy, else a **Consumer**;
    /// a non-eater defaults to Consumer.
    pub fn trophic_roles(&self, agents: &[crate::Agent]) -> HashMap<u64, TrophicRole> {
        let mut roles = HashMap::new();
        for a in agents {
            if a.traits.photosynthetic_absorption >= a.traits.heterotrophy {
                roles.insert(a.id, TrophicRole::Producer);
                continue;
            }

            let predation = self.outgoing_energy(a.id, EdgeKind::Consumed);
            let decomposition = self.outgoing_energy(a.id, EdgeKind::Decomposed);
            let consumed = predation + decomposition;

            let role = if consumed > 0.0
                && decomposition / consumed >= DETRITAL_RELIANCE_THRESHOLD
            {
                TrophicRole::Decomposer
            } else {
                TrophicRole::Consumer
            };
            roles.insert(a.id, role);
        }
        roles
    }

    /// Total energy an agent drained out via edges of the given kind.
    fn outgoing_energy(&self, agent: u64, kind: EdgeKind) -> f32 {
        self.edges
            .iter()
            .filter(|&(&(s, _, k), _)| s == agent && k == kind)
            .map(|(_, &w)| w)
            .sum()
    }

    pub fn active_agents_at(&self, tick: u64) -> HashSet<u64> {
        self.birth_tick
            .iter()
            .filter(|&(&agent, &born)| {
                born <= tick
                    && self
                        .death_tick
                        .get(&agent)
                        .map_or(true, |&died| died > tick)
            })
            .map(|(&agent, _)| agent)
            .collect()
    }

    pub fn lineage_parents(&self, agent_id: u64) -> Option<(u64, u64)> {
        self.lineage.get(&agent_id).copied()
    }

    pub fn lineage_clusters(&self) -> HashMap<u64, usize> {
        let mut parent: HashMap<u64, u64> = HashMap::new();
        let all_agents: HashSet<u64> = self
            .birth_tick
            .keys()
            .chain(self.death_tick.keys())
            .copied()
            .collect();
        for &a in &all_agents {
            parent.insert(a, a);
        }

        fn find(parent: &mut HashMap<u64, u64>, x: u64) -> u64 {
            let mut root = x;
            while parent[&root] != root {
                root = parent[&root];
            }
            let mut curr = x;
            while curr != root {
                let next = parent[&curr];
                parent.insert(curr, root);
                curr = next;
            }
            root
        }

        for (&child, &(p1, p2)) in &self.lineage {
            if parent.contains_key(&child) {
                if parent.contains_key(&p1) {
                    let rc = find(&mut parent, child);
                    let rp = find(&mut parent, p1);
                    if rc != rp {
                        parent.insert(rc, rp);
                    }
                }
                if parent.contains_key(&p2) {
                    let rc = find(&mut parent, child);
                    let rp = find(&mut parent, p2);
                    if rc != rp {
                        parent.insert(rc, rp);
                    }
                }
            }
        }

        let mut cluster_id_map: HashMap<u64, usize> = HashMap::new();
        let mut next_id = 0;
        let mut result = HashMap::new();
        for &a in &all_agents {
            let root = find(&mut parent, a);
            let id = *cluster_id_map.entry(root).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                id
            });
            result.insert(a, id);
        }
        result
    }

    pub fn energy_flow_between(
        &self,
        agents: &[crate::Agent],
        from: TrophicRole,
        to: TrophicRole,
    ) -> f32 {
        let roles = self.trophic_roles(agents);
        self.edges
            .iter()
            .filter(|&(&(s, t, _), _)| {
                roles.get(&s) == Some(&from) && roles.get(&t) == Some(&to)
            })
            .map(|(_, &w)| w)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;

    fn make_log(events: Vec<Event>) -> EventLog {
        let mut log = EventLog::new();
        for e in events {
            log.append(e).unwrap();
        }
        log
    }

    /// An agent carrying only the trophic traits `trophic_roles` reads: autotrophy
    /// (`photo`) and heterotrophy (`hetero`). Everything else is zeroed.
    fn agent(id: u64, photo: f32, hetero: f32) -> crate::Agent {
        crate::Agent {
            id,
            position: (0.0, 0.0),
            reserve: 1.0,
            structure: 1.0,
            nutrient: 0.0,
            traits: crate::TraitVector {
                photosynthetic_absorption: photo,
                heterotrophy: hetero,
                mobility: 0.0,
                kappa: 0.0,
                fecundity: 0.0,
                asexual_propensity: 0.0,
                dispersal: 0.0,
            },
            contact_time: 0,
            wear: [0.0; crate::FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 0.0,
        }
    }

    #[test]
    fn carcass_eating_heterotroph_reads_as_decomposer() {
        let log = make_log(vec![
            Event {
                tick: 1, seq: 0, kind: EventKind::Reproduced,
                source: 1, target: None,
                energy_delta: 10.0, position: None, target_was_carcass: false,
            },
            // Heterotroph 1 drains carcass 99 — the detrital (brown) pathway.
            Event {
                tick: 2, seq: 1, kind: EventKind::Consumed,
                source: 1, target: Some(99),
                energy_delta: 5.0, position: None, target_was_carcass: true,
            },
        ]);

        let mut proj = TopologyProjection::new();
        proj.update(&log);

        // Heterotrophy-dominant trait, all consumed energy detrital -> Decomposer.
        let roles = proj.trophic_roles(&[agent(1, 0.0, 1.0)]);
        assert_eq!(roles[&1], TrophicRole::Decomposer);
    }

    #[test]
    fn reproduced_event_without_target_adds_agent_to_active_set() {
        let log = make_log(vec![Event {
            tick: 1,
            seq: 0,
            kind: EventKind::Reproduced,
            source: 42,
            target: None,
            energy_delta: 10.0, position: None, target_was_carcass: false,
        }]);

        let mut proj = TopologyProjection::new();
        proj.update(&log);

        assert!(proj.active_agents().contains(&42));
    }

    #[test]
    fn consumed_event_creates_edge_with_energy_weight() {
        let log = make_log(vec![Event {
            tick: 1,
            seq: 0,
            kind: EventKind::Consumed,
            source: 10,
            target: Some(20),
            energy_delta: 5.0, position: None, target_was_carcass: false,
        }]);

        let mut proj = TopologyProjection::new();
        proj.update(&log);

        assert_eq!(proj.edge_weight(10, 20), 5.0);
        assert_eq!(proj.edge_weight(20, 10), 0.0);
    }

    #[test]
    fn repeated_consumption_reinforces_edge_weight() {
        let log = make_log(vec![
            Event {
                tick: 1,
                seq: 0,
                kind: EventKind::Consumed,
                source: 10,
                target: Some(20),
                energy_delta: 5.0, position: None, target_was_carcass: false,
            },
            Event {
                tick: 2,
                seq: 1,
                kind: EventKind::Consumed,
                source: 10,
                target: Some(20),
                energy_delta: 3.0, position: None, target_was_carcass: false,
            },
        ]);

        let mut proj = TopologyProjection::new();
        proj.update(&log);

        assert_eq!(proj.edge_weight(10, 20), 8.0);
    }

    #[test]
    fn died_event_removes_agent_edges_and_deactivates() {
        let log = make_log(vec![
            Event {
                tick: 1,
                seq: 0,
                kind: EventKind::Reproduced,
                source: 10,
                target: None,
                energy_delta: 10.0, position: None, target_was_carcass: false,
            },
            Event {
                tick: 1,
                seq: 1,
                kind: EventKind::Reproduced,
                source: 20,
                target: None,
                energy_delta: 10.0, position: None, target_was_carcass: false,
            },
            Event {
                tick: 2,
                seq: 2,
                kind: EventKind::Consumed,
                source: 10,
                target: Some(20),
                energy_delta: 5.0, position: None, target_was_carcass: false,
            },
            Event {
                tick: 3,
                seq: 3,
                kind: EventKind::Died,
                source: 20,
                target: None,
                energy_delta: 0.0, position: None, target_was_carcass: false,
            },
        ]);

        let mut proj = TopologyProjection::new();
        proj.update(&log);

        assert!(!proj.active_agents().contains(&20));
        assert_eq!(proj.edge_weight(10, 20), 0.0);
    }

    #[test]
    fn trophic_roles_classify_by_energy_flow() {
        let log = make_log(vec![
            Event {
                tick: 1, seq: 0, kind: EventKind::Reproduced,
                source: 1, target: None,
                energy_delta: 10.0, position: None, target_was_carcass: false,
            },
            Event {
                tick: 1, seq: 1, kind: EventKind::Reproduced,
                source: 2, target: None,
                energy_delta: 10.0, position: None, target_was_carcass: false,
            },
            Event {
                tick: 1, seq: 2, kind: EventKind::Reproduced,
                source: 3, target: None,
                energy_delta: 10.0, position: None, target_was_carcass: false,
            },
            // Agent 2 consumes agent 1 -> consumer
            Event {
                tick: 2, seq: 3, kind: EventKind::Consumed,
                source: 2, target: Some(1),
                energy_delta: 5.0, position: None, target_was_carcass: false,
            },
            // Agent 3 drains carcass 99 -> detrital pathway
            Event {
                tick: 2, seq: 4, kind: EventKind::Consumed,
                source: 3, target: Some(99),
                energy_delta: 3.0, position: None, target_was_carcass: true,
            },
        ]);

        let mut proj = TopologyProjection::new();
        proj.update(&log);

        // Agent 1 is autotrophy-dominant -> producer (a trait reading); agents 2
        // and 3 are heterotrophs split by what they ate.
        let agents = [agent(1, 1.0, 0.0), agent(2, 0.0, 1.0), agent(3, 0.0, 1.0)];
        let roles = proj.trophic_roles(&agents);
        assert_eq!(roles[&1], TrophicRole::Producer);
        assert_eq!(roles[&2], TrophicRole::Consumer);
        assert_eq!(roles[&3], TrophicRole::Decomposer);
    }

    #[test]
    fn autotrophy_dominant_agent_is_producer_even_when_it_eats() {
        // Producer is a trait reading, not a window artifact: an autotroph that
        // incidentally scavenges a carcass is still a producer.
        let log = make_log(vec![
            Event {
                tick: 1, seq: 0, kind: EventKind::Reproduced,
                source: 1, target: None,
                energy_delta: 10.0, position: None, target_was_carcass: false,
            },
            Event {
                tick: 2, seq: 1, kind: EventKind::Consumed,
                source: 1, target: Some(99),
                energy_delta: 5.0, position: None, target_was_carcass: true,
            },
        ]);

        let mut proj = TopologyProjection::new();
        proj.update(&log);

        let roles = proj.trophic_roles(&[agent(1, 1.0, 0.2)]);
        assert_eq!(roles[&1], TrophicRole::Producer);
    }

    #[test]
    fn heterotroph_below_detrital_threshold_is_consumer() {
        // Mostly-predation diet (1 of 4 units detrital = 25% < 50%) -> Consumer.
        let log = make_log(vec![
            Event {
                tick: 1, seq: 0, kind: EventKind::Reproduced,
                source: 1, target: None,
                energy_delta: 10.0, position: None, target_was_carcass: false,
            },
            Event {
                tick: 2, seq: 1, kind: EventKind::Consumed,
                source: 1, target: Some(2),
                energy_delta: 3.0, position: None, target_was_carcass: false,
            },
            Event {
                tick: 2, seq: 2, kind: EventKind::Consumed,
                source: 1, target: Some(99),
                energy_delta: 1.0, position: None, target_was_carcass: true,
            },
        ]);

        let mut proj = TopologyProjection::new();
        proj.update(&log);

        let roles = proj.trophic_roles(&[agent(1, 0.0, 1.0)]);
        assert_eq!(roles[&1], TrophicRole::Consumer);
    }

    #[test]
    fn heterotroph_at_detrital_threshold_is_decomposer() {
        // Exactly 50% detrital sits on the bucket boundary -> Decomposer (>=).
        let log = make_log(vec![
            Event {
                tick: 1, seq: 0, kind: EventKind::Reproduced,
                source: 1, target: None,
                energy_delta: 10.0, position: None, target_was_carcass: false,
            },
            Event {
                tick: 2, seq: 1, kind: EventKind::Consumed,
                source: 1, target: Some(2),
                energy_delta: 2.0, position: None, target_was_carcass: false,
            },
            Event {
                tick: 2, seq: 2, kind: EventKind::Consumed,
                source: 1, target: Some(99),
                energy_delta: 2.0, position: None, target_was_carcass: true,
            },
        ]);

        let mut proj = TopologyProjection::new();
        proj.update(&log);

        let roles = proj.trophic_roles(&[agent(1, 0.0, 1.0)]);
        assert_eq!(roles[&1], TrophicRole::Decomposer);
    }

    #[test]
    fn non_eating_heterotroph_defaults_to_consumer() {
        // A heterotroph that has eaten nothing has no detrital reliance to read,
        // so it defaults to Consumer rather than Decomposer.
        let log = make_log(vec![Event {
            tick: 1, seq: 0, kind: EventKind::Reproduced,
            source: 1, target: None,
            energy_delta: 10.0, position: None, target_was_carcass: false,
        }]);

        let mut proj = TopologyProjection::new();
        proj.update(&log);

        let roles = proj.trophic_roles(&[agent(1, 0.0, 1.0)]);
        assert_eq!(roles[&1], TrophicRole::Consumer);
    }

    #[test]
    fn lineage_tracks_parent_offspring_from_reproduced_events() {
        // Reproduced with target = parent pair, Reproduced without target = offspring
        let log = make_log(vec![
            Event { tick: 1, seq: 0, kind: EventKind::Reproduced, source: 10, target: Some(20), energy_delta: 0.0, position: None, target_was_carcass: false },
            Event { tick: 1, seq: 1, kind: EventKind::Reproduced, source: 30, target: None, energy_delta: 8.0, position: None, target_was_carcass: false },
        ]);

        let mut proj = TopologyProjection::new();
        proj.update(&log);

        assert_eq!(proj.lineage_parents(30), Some((10, 20)));
        assert_eq!(proj.lineage_parents(10), None);
    }

    #[test]
    fn active_agents_at_tick_reflects_births_and_deaths() {
        let log = make_log(vec![
            Event { tick: 1, seq: 0, kind: EventKind::Reproduced, source: 1, target: None, energy_delta: 10.0, position: None, target_was_carcass: false },
            Event { tick: 2, seq: 1, kind: EventKind::Reproduced, source: 2, target: None, energy_delta: 10.0, position: None, target_was_carcass: false },
            Event { tick: 3, seq: 2, kind: EventKind::Died, source: 1, target: None, energy_delta: 0.0, position: None, target_was_carcass: false },
            Event { tick: 4, seq: 3, kind: EventKind::Reproduced, source: 3, target: None, energy_delta: 10.0, position: None, target_was_carcass: false },
        ]);

        let mut proj = TopologyProjection::new();
        proj.update(&log);

        assert_eq!(proj.active_agents_at(0), HashSet::new());
        assert_eq!(proj.active_agents_at(1), HashSet::from([1]));
        assert_eq!(proj.active_agents_at(2), HashSet::from([1, 2]));
        assert_eq!(proj.active_agents_at(3), HashSet::from([2]));
        assert_eq!(proj.active_agents_at(4), HashSet::from([2, 3]));
    }

    #[test]
    fn energy_flow_between_trophic_groups() {
        let log = make_log(vec![
            Event { tick: 1, seq: 0, kind: EventKind::Reproduced, source: 1, target: None, energy_delta: 10.0, position: None, target_was_carcass: false },
            Event { tick: 1, seq: 1, kind: EventKind::Reproduced, source: 2, target: None, energy_delta: 10.0, position: None, target_was_carcass: false },
            Event { tick: 1, seq: 2, kind: EventKind::Reproduced, source: 3, target: None, energy_delta: 10.0, position: None, target_was_carcass: false },
            // Consumer 2 eats producer 1 twice
            Event { tick: 2, seq: 3, kind: EventKind::Consumed, source: 2, target: Some(1), energy_delta: 5.0, position: None, target_was_carcass: false },
            Event { tick: 3, seq: 4, kind: EventKind::Consumed, source: 2, target: Some(1), energy_delta: 3.0, position: None, target_was_carcass: false },
            // Consumer 3 also eats producer 1
            Event { tick: 3, seq: 5, kind: EventKind::Consumed, source: 3, target: Some(1), energy_delta: 2.0, position: None, target_was_carcass: false },
        ]);

        let mut proj = TopologyProjection::new();
        proj.update(&log);

        // 1 is autotrophy-dominant (producer); 2 and 3 are heterotroph predators.
        let agents = [agent(1, 1.0, 0.0), agent(2, 0.0, 1.0), agent(3, 0.0, 1.0)];
        assert_eq!(
            proj.energy_flow_between(&agents, TrophicRole::Consumer, TrophicRole::Producer),
            10.0
        );
        assert_eq!(
            proj.energy_flow_between(&agents, TrophicRole::Producer, TrophicRole::Consumer),
            0.0
        );
    }

    #[test]
    fn lineage_clusters_groups_descendants_with_ancestors() {
        let log = make_log(vec![
            // Two initial agents (roots) born independently
            Event { tick: 1, seq: 0, kind: EventKind::Reproduced, source: 1, target: None, energy_delta: 10.0, position: None, target_was_carcass: false },
            Event { tick: 1, seq: 1, kind: EventKind::Reproduced, source: 2, target: None, energy_delta: 10.0, position: None, target_was_carcass: false },
            // Agent 1 and 2 mate -> child 3
            Event { tick: 2, seq: 2, kind: EventKind::Reproduced, source: 1, target: Some(2), energy_delta: 0.0, position: None, target_was_carcass: false },
            Event { tick: 2, seq: 3, kind: EventKind::Reproduced, source: 3, target: None, energy_delta: 8.0, position: None, target_was_carcass: false },
            // Agent 4 born independently (separate lineage)
            Event { tick: 3, seq: 4, kind: EventKind::Reproduced, source: 4, target: None, energy_delta: 10.0, position: None, target_was_carcass: false },
        ]);

        let mut proj = TopologyProjection::new();
        proj.update(&log);

        let clusters = proj.lineage_clusters();
        // Agents 1, 2, 3 should be in the same cluster (connected via child 3)
        assert_eq!(clusters[&1], clusters[&2]);
        assert_eq!(clusters[&1], clusters[&3]);
        // Agent 4 should be in a different cluster
        assert_ne!(clusters[&1], clusters[&4]);
    }

    #[test]
    fn incremental_update_processes_only_new_events() {
        let mut log = EventLog::new();
        log.append(Event { tick: 1, seq: 0, kind: EventKind::Reproduced, source: 1, target: None, energy_delta: 10.0, position: None, target_was_carcass: false }).unwrap();

        let mut proj = TopologyProjection::new();
        proj.update(&log);
        assert_eq!(proj.active_agents().len(), 1);

        log.append(Event { tick: 2, seq: 1, kind: EventKind::Reproduced, source: 2, target: None, energy_delta: 10.0, position: None, target_was_carcass: false }).unwrap();
        proj.update(&log);
        assert_eq!(proj.active_agents().len(), 2);
        assert!(proj.active_agents().contains(&1));
        assert!(proj.active_agents().contains(&2));
    }
}
