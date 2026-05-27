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
    Consumed,
}

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
                        *self
                            .edges
                            .entry((event.source, target, EdgeKind::Consumed))
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
        self.edges
            .get(&(source, target, EdgeKind::Consumed))
            .copied()
            .unwrap_or(0.0)
    }

    pub fn trophic_roles(&self) -> HashMap<u64, TrophicRole> {
        let mut roles = HashMap::new();
        for &agent in &self.active_agents {
            let consumed_energy: f32 = self
                .edges
                .iter()
                .filter(|&(&(s, _, k), _)| s == agent && k == EdgeKind::Consumed)
                .map(|(_, &w)| w)
                .sum();

            let role = if consumed_energy > 0.0 {
                TrophicRole::Consumer
            } else {
                TrophicRole::Producer
            };
            roles.insert(agent, role);
        }
        roles
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

    pub fn energy_flow_between(&self, from: TrophicRole, to: TrophicRole) -> f32 {
        let roles = self.trophic_roles();
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

    #[test]
    fn reproduced_event_without_target_adds_agent_to_active_set() {
        let log = make_log(vec![Event {
            tick: 1,
            seq: 0,
            kind: EventKind::Reproduced,
            source: 42,
            target: None,
            energy_delta: 10.0, position: None,
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
            energy_delta: 5.0, position: None,
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
                energy_delta: 5.0, position: None,
            },
            Event {
                tick: 2,
                seq: 1,
                kind: EventKind::Consumed,
                source: 10,
                target: Some(20),
                energy_delta: 3.0, position: None,
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
                energy_delta: 10.0, position: None,
            },
            Event {
                tick: 1,
                seq: 1,
                kind: EventKind::Reproduced,
                source: 20,
                target: None,
                energy_delta: 10.0, position: None,
            },
            Event {
                tick: 2,
                seq: 2,
                kind: EventKind::Consumed,
                source: 10,
                target: Some(20),
                energy_delta: 5.0, position: None,
            },
            Event {
                tick: 3,
                seq: 3,
                kind: EventKind::Died,
                source: 20,
                target: None,
                energy_delta: 0.0, position: None,
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
                energy_delta: 10.0, position: None,
            },
            Event {
                tick: 1, seq: 1, kind: EventKind::Reproduced,
                source: 2, target: None,
                energy_delta: 10.0, position: None,
            },
            Event {
                tick: 1, seq: 2, kind: EventKind::Reproduced,
                source: 3, target: None,
                energy_delta: 10.0, position: None,
            },
            // Agent 2 consumes agent 1 -> consumer
            Event {
                tick: 2, seq: 3, kind: EventKind::Consumed,
                source: 2, target: Some(1),
                energy_delta: 5.0, position: None,
            },
            // Agent 3 consumes carcass 99 (decomposition is now Consumed)
            Event {
                tick: 2, seq: 4, kind: EventKind::Consumed,
                source: 3, target: Some(99),
                energy_delta: 3.0, position: None,
            },
            // Agent 1 has no outgoing edges -> producer
        ]);

        let mut proj = TopologyProjection::new();
        proj.update(&log);

        let roles = proj.trophic_roles();
        assert_eq!(roles[&1], TrophicRole::Producer);
        assert_eq!(roles[&2], TrophicRole::Consumer);
        assert_eq!(roles[&3], TrophicRole::Consumer);
    }

    #[test]
    fn lineage_tracks_parent_offspring_from_reproduced_events() {
        // Reproduced with target = parent pair, Reproduced without target = offspring
        let log = make_log(vec![
            Event { tick: 1, seq: 0, kind: EventKind::Reproduced, source: 10, target: Some(20), energy_delta: 0.0, position: None },
            Event { tick: 1, seq: 1, kind: EventKind::Reproduced, source: 30, target: None, energy_delta: 8.0, position: None },
        ]);

        let mut proj = TopologyProjection::new();
        proj.update(&log);

        assert_eq!(proj.lineage_parents(30), Some((10, 20)));
        assert_eq!(proj.lineage_parents(10), None);
    }

    #[test]
    fn active_agents_at_tick_reflects_births_and_deaths() {
        let log = make_log(vec![
            Event { tick: 1, seq: 0, kind: EventKind::Reproduced, source: 1, target: None, energy_delta: 10.0, position: None },
            Event { tick: 2, seq: 1, kind: EventKind::Reproduced, source: 2, target: None, energy_delta: 10.0, position: None },
            Event { tick: 3, seq: 2, kind: EventKind::Died, source: 1, target: None, energy_delta: 0.0, position: None },
            Event { tick: 4, seq: 3, kind: EventKind::Reproduced, source: 3, target: None, energy_delta: 10.0, position: None },
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
            Event { tick: 1, seq: 0, kind: EventKind::Reproduced, source: 1, target: None, energy_delta: 10.0, position: None },
            Event { tick: 1, seq: 1, kind: EventKind::Reproduced, source: 2, target: None, energy_delta: 10.0, position: None },
            Event { tick: 1, seq: 2, kind: EventKind::Reproduced, source: 3, target: None, energy_delta: 10.0, position: None },
            // Consumer 2 eats producer 1 twice
            Event { tick: 2, seq: 3, kind: EventKind::Consumed, source: 2, target: Some(1), energy_delta: 5.0, position: None },
            Event { tick: 3, seq: 4, kind: EventKind::Consumed, source: 2, target: Some(1), energy_delta: 3.0, position: None },
            // Consumer 3 also eats producer 1
            Event { tick: 3, seq: 5, kind: EventKind::Consumed, source: 3, target: Some(1), energy_delta: 2.0, position: None },
        ]);

        let mut proj = TopologyProjection::new();
        proj.update(&log);

        assert_eq!(
            proj.energy_flow_between(TrophicRole::Consumer, TrophicRole::Producer),
            10.0
        );
        assert_eq!(
            proj.energy_flow_between(TrophicRole::Producer, TrophicRole::Consumer),
            0.0
        );
    }

    #[test]
    fn lineage_clusters_groups_descendants_with_ancestors() {
        let log = make_log(vec![
            // Two initial agents (roots) born independently
            Event { tick: 1, seq: 0, kind: EventKind::Reproduced, source: 1, target: None, energy_delta: 10.0, position: None },
            Event { tick: 1, seq: 1, kind: EventKind::Reproduced, source: 2, target: None, energy_delta: 10.0, position: None },
            // Agent 1 and 2 mate -> child 3
            Event { tick: 2, seq: 2, kind: EventKind::Reproduced, source: 1, target: Some(2), energy_delta: 0.0, position: None },
            Event { tick: 2, seq: 3, kind: EventKind::Reproduced, source: 3, target: None, energy_delta: 8.0, position: None },
            // Agent 4 born independently (separate lineage)
            Event { tick: 3, seq: 4, kind: EventKind::Reproduced, source: 4, target: None, energy_delta: 10.0, position: None },
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
        log.append(Event { tick: 1, seq: 0, kind: EventKind::Reproduced, source: 1, target: None, energy_delta: 10.0, position: None }).unwrap();

        let mut proj = TopologyProjection::new();
        proj.update(&log);
        assert_eq!(proj.active_agents().len(), 1);

        log.append(Event { tick: 2, seq: 1, kind: EventKind::Reproduced, source: 2, target: None, energy_delta: 10.0, position: None }).unwrap();
        proj.update(&log);
        assert_eq!(proj.active_agents().len(), 2);
        assert!(proj.active_agents().contains(&1));
        assert!(proj.active_agents().contains(&2));
    }
}
