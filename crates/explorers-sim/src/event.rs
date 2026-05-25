#[derive(Clone, Debug, PartialEq)]
pub enum EventKind {
    Born,
    Died,
    Consumed,
    Decomposed,
    MateSelected,
    CarcassCreated,
    CarcassDepleted,
    MatingReadiness,
    Moved,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub tick: u64,
    pub seq: u64,
    pub kind: EventKind,
    pub source: u64,
    pub target: Option<u64>,
    pub energy_delta: f32,
    pub position: Option<(f32, f32)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ack {
    Ack,
    Nack,
}

#[derive(Clone, Debug)]
pub struct Response {
    pub ack: Ack,
    pub events: Vec<Event>,
}

pub struct EventLog {
    events: Vec<Event>,
}

impl EventLog {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn append(&mut self, event: Event) -> Result<(), &'static str> {
        if let Some(last) = self.events.last() {
            if event.seq <= last.seq {
                return Err("sequence number must be monotonically increasing");
            }
        }
        self.events.push(event);
        Ok(())
    }

    pub fn by_tick_range(&self, start: u64, end: u64) -> &[Event] {
        let lo = self.events.partition_point(|e| e.tick < start);
        let hi = self.events.partition_point(|e| e.tick < end);
        &self.events[lo..hi]
    }

    pub fn by_kind(&self, kind: &EventKind) -> Vec<&Event> {
        self.events.iter().filter(|e| &e.kind == kind).collect()
    }

    pub fn by_agent(&self, agent_id: u64) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| e.source == agent_id || e.target == Some(agent_id))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn since(&self, index: usize) -> &[Event] {
        if index >= self.events.len() {
            &[]
        } else {
            &self.events[index..]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    fn make_event(tick: u64, seq: u64, kind: EventKind) -> Event {
        Event {
            tick,
            seq,
            kind,
            source: 1,
            target: None,
            energy_delta: 10.0,
            position: None,
        }
    }

    #[test]
    fn event_fits_within_128_bytes() {
        assert!(
            mem::size_of::<Event>() <= 128,
            "Event is {} bytes, must be ≤128",
            mem::size_of::<Event>()
        );
    }

    #[test]
    fn response_fits_within_128_bytes() {
        assert!(
            mem::size_of::<Response>() <= 128,
            "Response is {} bytes, must be ≤128",
            mem::size_of::<Response>()
        );
    }

    #[test]
    fn nack_with_events_is_representable() {
        let response = Response {
            ack: Ack::Nack,
            events: vec![make_event(1, 0, EventKind::Consumed)],
        };
        assert_eq!(response.ack, Ack::Nack);
        assert_eq!(response.events.len(), 1);
    }

    #[test]
    fn append_rejects_non_monotonic_sequence() {
        let mut log = EventLog::new();
        log.append(make_event(1, 5, EventKind::Born)).unwrap();
        assert!(log.append(make_event(1, 5, EventKind::Died)).is_err());
        assert!(log.append(make_event(1, 3, EventKind::Died)).is_err());
        assert!(log.append(make_event(1, 6, EventKind::Died)).is_ok());
    }

    #[test]
    fn appended_event_is_returned_by_tick_range() {
        let mut log = EventLog::new();
        let e = make_event(5, 0, EventKind::Born);
        log.append(e.clone()).unwrap();
        let results = log.by_tick_range(5, 6);
        assert_eq!(results, &[e]);
    }

    #[test]
    fn tick_range_returns_only_matching_ticks_in_sequence_order() {
        let mut log = EventLog::new();
        log.append(make_event(1, 0, EventKind::Born)).unwrap();
        log.append(make_event(2, 1, EventKind::Died)).unwrap();
        log.append(make_event(2, 2, EventKind::Consumed)).unwrap();
        log.append(make_event(3, 3, EventKind::Born)).unwrap();
        log.append(make_event(5, 4, EventKind::Died)).unwrap();

        let results = log.by_tick_range(2, 4);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].tick, 2);
        assert_eq!(results[0].seq, 1);
        assert_eq!(results[1].tick, 2);
        assert_eq!(results[1].seq, 2);
        assert_eq!(results[2].tick, 3);
        assert_eq!(results[2].seq, 3);
    }

    #[test]
    fn tick_range_on_empty_log_returns_empty() {
        let log = EventLog::new();
        assert!(log.by_tick_range(0, 100).is_empty());
    }

    #[test]
    fn by_agent_matches_source_and_target() {
        let mut log = EventLog::new();
        log.append(Event {
            tick: 1, seq: 0, kind: EventKind::Consumed,
            source: 10, target: Some(20), energy_delta: 5.0, position: None,
        }).unwrap();
        log.append(Event {
            tick: 1, seq: 1, kind: EventKind::Born,
            source: 30, target: None, energy_delta: 8.0, position: None,
        }).unwrap();
        log.append(Event {
            tick: 2, seq: 2, kind: EventKind::Decomposed,
            source: 40, target: Some(10), energy_delta: 3.0, position: None,
        }).unwrap();

        let for_10: Vec<_> = log.by_agent(10);
        assert_eq!(for_10.len(), 2);
        assert_eq!(for_10[0].seq, 0); // source=10
        assert_eq!(for_10[1].seq, 2); // target=10

        let for_30: Vec<_> = log.by_agent(30);
        assert_eq!(for_30.len(), 1);

        assert!(log.by_agent(99).is_empty());
    }

    #[test]
    fn by_kind_returns_matching_events() {
        let mut log = EventLog::new();
        log.append(make_event(1, 0, EventKind::Born)).unwrap();
        log.append(make_event(1, 1, EventKind::Died)).unwrap();
        log.append(make_event(2, 2, EventKind::Born)).unwrap();
        log.append(make_event(3, 3, EventKind::Consumed)).unwrap();

        let born: Vec<_> = log.by_kind(&EventKind::Born);
        assert_eq!(born.len(), 2);
        assert_eq!(born[0].seq, 0);
        assert_eq!(born[1].seq, 2);

        assert!(log.by_kind(&EventKind::CarcassCreated).is_empty());
    }

    #[test]
    fn append_after_query_is_valid() {
        let mut log = EventLog::new();
        log.append(make_event(1, 0, EventKind::Born)).unwrap();

        let _ = log.by_tick_range(0, 10);
        let _ = log.by_agent(1);
        let _ = log.by_kind(&EventKind::Born);

        log.append(make_event(2, 1, EventKind::Died)).unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log.by_tick_range(2, 3).len(), 1);
    }
}
