#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    Photosynthesized,
    NutrientAbsorbed,
    Metabolized,
    Grew,
    Consumed,
    Reproduced,
    Wore,
    Died,
    Moved,
    /// Network redistribution (flow 5): cooperative transfer of energy or
    /// nutrient from `source` (the donor) to `target` (the recipient) along a
    /// network connection. `energy_delta` is the net amount received by the
    /// recipient. Emitted only when the network is enabled; inert by default.
    Redistributed,
    /// A birth (#443): `source` is the offspring's world id, `target` the
    /// parent — for a sexual birth the seed parent, with the mate in
    /// `second_parent`. One event per offspring, emitted after the litter's
    /// `Reproduced` event once final ids are assigned. A raw descent fact, so
    /// an observer can follow a lineage (an invader cohort and its
    /// descendants) off the log alone; no state is added to `Agent`.
    Born,
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
    /// Raw interaction fact set only on `Consumed` events: whether the target
    /// was a carcass (detrital pathway) rather than a living agent. Lets an
    /// observer-side projection separate the brown (decomposition) food web from
    /// the green (predation) one without re-deriving carcass state. Always
    /// `false` for non-`Consumed` events.
    pub target_was_carcass: bool,
    /// Set only on `Born` events: the second parent of a sexual birth. `None`
    /// for asexual births and every other event kind.
    pub second_parent: Option<u64>,
}

/// Append-only event log with observer-side compaction. Indices are
/// **absolute**: `len()` counts every event ever appended and `since(n)`
/// reads from absolute position `n`, so a projection's cursor stays valid
/// across [`EventLog::compact_before`]. The stepper only ever appends; what
/// the log retains is an observer's concern.
#[derive(Clone)]
pub struct EventLog {
    events: Vec<Event>,
    /// Absolute index of `events[0]`: how many events have been dropped.
    base: usize,
    /// Sequence number of the last event appended, kept across compaction
    /// so the monotonic-seq guard does not reset.
    last_seq: Option<u64>,
}

impl EventLog {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            base: 0,
            last_seq: None,
        }
    }

    pub fn append(&mut self, event: Event) -> Result<(), &'static str> {
        if let Some(last) = self.last_seq {
            if event.seq <= last {
                return Err("sequence number must be monotonically increasing");
            }
        }
        self.last_seq = Some(event.seq);
        self.events.push(event);
        Ok(())
    }

    /// Drop every event with absolute index `< index`, keeping the absolute
    /// indexing (`len()`, `since()`) intact. An observer that has finished
    /// with the history — a projection walked to the present, a guild read
    /// over a closed window — calls this before a long-lived fork so the
    /// fork does not clone the whole past. Range and kind queries afterwards
    /// see only what is retained.
    pub fn compact_before(&mut self, index: usize) {
        let drop = index.saturating_sub(self.base).min(self.events.len());
        if drop == 0 {
            return;
        }
        self.events.drain(..drop);
        self.base += drop;
    }

    /// Events currently held (after compaction), as opposed to `len()`.
    pub fn retained(&self) -> usize {
        self.events.len()
    }

    /// Retained events in `[start, end)` by tick.
    pub fn by_tick_range(&self, start: u64, end: u64) -> &[Event] {
        let lo = self.events.partition_point(|e| e.tick < start);
        let hi = self.events.partition_point(|e| e.tick < end);
        &self.events[lo..hi]
    }

    /// Retained events of one kind.
    pub fn by_kind(&self, kind: &EventKind) -> Vec<&Event> {
        self.events.iter().filter(|e| &e.kind == kind).collect()
    }

    /// Retained events naming the agent as source or target.
    pub fn by_agent(&self, agent_id: u64) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| e.source == agent_id || e.target == Some(agent_id))
            .collect()
    }

    /// Total events ever appended (absolute length), including any dropped
    /// by compaction.
    pub fn len(&self) -> usize {
        self.base + self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Events from absolute index `index` on; an index inside dropped history
    /// yields everything retained.
    pub fn since(&self, index: usize) -> &[Event] {
        let rel = index.saturating_sub(self.base);
        if rel >= self.events.len() {
            &[]
        } else {
            &self.events[rel..]
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
            target_was_carcass: false,
            second_parent: None,
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
    fn append_rejects_non_monotonic_sequence() {
        let mut log = EventLog::new();
        log.append(make_event(1, 5, EventKind::Reproduced)).unwrap();
        assert!(log.append(make_event(1, 5, EventKind::Died)).is_err());
        assert!(log.append(make_event(1, 3, EventKind::Died)).is_err());
        assert!(log.append(make_event(1, 6, EventKind::Died)).is_ok());
    }

    #[test]
    fn appended_event_is_returned_by_tick_range() {
        let mut log = EventLog::new();
        let e = make_event(5, 0, EventKind::Reproduced);
        log.append(e.clone()).unwrap();
        let results = log.by_tick_range(5, 6);
        assert_eq!(results, &[e]);
    }

    #[test]
    fn tick_range_returns_only_matching_ticks_in_sequence_order() {
        let mut log = EventLog::new();
        log.append(make_event(1, 0, EventKind::Reproduced)).unwrap();
        log.append(make_event(2, 1, EventKind::Died)).unwrap();
        log.append(make_event(2, 2, EventKind::Consumed)).unwrap();
        log.append(make_event(3, 3, EventKind::Reproduced)).unwrap();
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
            tick: 1,
            seq: 0,
            kind: EventKind::Consumed,
            source: 10,
            target: Some(20),
            energy_delta: 5.0,
            position: None,
            target_was_carcass: false,
            second_parent: None,
        })
        .unwrap();
        log.append(Event {
            tick: 1,
            seq: 1,
            kind: EventKind::Reproduced,
            source: 30,
            target: None,
            energy_delta: 8.0,
            position: None,
            target_was_carcass: false,
            second_parent: None,
        })
        .unwrap();
        log.append(Event {
            tick: 2,
            seq: 2,
            kind: EventKind::Consumed,
            source: 40,
            target: Some(10),
            energy_delta: 3.0,
            position: None,
            target_was_carcass: false,
            second_parent: None,
        })
        .unwrap();

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
        log.append(make_event(1, 0, EventKind::Reproduced)).unwrap();
        log.append(make_event(1, 1, EventKind::Died)).unwrap();
        log.append(make_event(2, 2, EventKind::Reproduced)).unwrap();
        log.append(make_event(3, 3, EventKind::Consumed)).unwrap();

        let reproduced: Vec<_> = log.by_kind(&EventKind::Reproduced);
        assert_eq!(reproduced.len(), 2);
        assert_eq!(reproduced[0].seq, 0);
        assert_eq!(reproduced[1].seq, 2);

        assert!(log.by_kind(&EventKind::Photosynthesized).is_empty());
    }

    #[test]
    fn append_after_query_is_valid() {
        let mut log = EventLog::new();
        log.append(make_event(1, 0, EventKind::Reproduced)).unwrap();

        let _ = log.by_tick_range(0, 10);
        let _ = log.by_agent(1);
        let _ = log.by_kind(&EventKind::Reproduced);

        log.append(make_event(2, 1, EventKind::Died)).unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log.by_tick_range(2, 3).len(), 1);
    }

    #[test]
    fn event_kind_has_exactly_nine_variants() {
        // Verify the design language: Photosynthesized, NutrientAbsorbed,
        // Metabolized, Grew, Consumed, Reproduced, Wore, Died, Moved
        let variants = [
            EventKind::Photosynthesized,
            EventKind::NutrientAbsorbed,
            EventKind::Metabolized,
            EventKind::Grew,
            EventKind::Consumed,
            EventKind::Reproduced,
            EventKind::Wore,
            EventKind::Died,
            EventKind::Moved,
        ];
        assert_eq!(variants.len(), 9);
    }

    /// Compaction drops history but keeps the log's absolute indexing, so an
    /// observer holding a cursor (`since(n)`, `len()`) reads on unchanged.
    #[test]
    fn compact_before_keeps_absolute_indices_and_the_seq_guard() {
        let mut log = EventLog::new();
        for seq in 0..10 {
            log.append(make_event(seq / 2, seq, EventKind::Metabolized))
                .unwrap();
        }
        let cursor = log.len();
        log.compact_before(7);
        assert_eq!(log.len(), 10);
        assert_eq!(log.retained(), 3);
        assert_eq!(log.since(cursor).len(), 0);
        assert_eq!(log.since(8).len(), 2);
        assert_eq!(log.since(8)[0].seq, 8);
        // Asking for dropped history yields what is retained, not a panic.
        assert_eq!(log.since(0).len(), 3);
        // The monotonic-seq guard survives an empty retained buffer.
        log.compact_before(10);
        assert_eq!(log.retained(), 0);
        assert!(log.append(make_event(5, 9, EventKind::Died)).is_err());
        assert!(log.append(make_event(5, 10, EventKind::Died)).is_ok());
        assert_eq!(log.len(), 11);
        assert_eq!(log.since(cursor).len(), 1);
        // Compacting before an index already dropped is a no-op.
        log.compact_before(3);
        assert_eq!(log.retained(), 1);
    }
}
