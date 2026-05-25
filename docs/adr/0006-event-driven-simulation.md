# Event-driven simulation with log, projections, and ephemeral broadcasts

The current simulation uses a rigid six-phase step function where all agents pass through every phase every tick (sense → light → move → consume → decompose → reproduce/die). Agents sense each other directly by inspecting positions and traits. This works but limits us in two ways: we cannot reconstruct the causal history of the ecology without replaying from seed, and agents have no memory of prior activity — only instantaneous awareness.

## Considered options

- **Event log with spatial field projections and ephemeral broadcasts (chosen).** Replace the phase-structured step with a discrete event simulation (DES). All activity produces events. A durable event log stores topology changes (births, deaths, trophic transfers, mate selections) as the source of truth. Agents perceive the world through projections — spatial read models derived from the log that decay over time. The DES broadcasts events to agents within sensing range; agents react to broadcasts and consult projections to make decisions. Interactions resolve within a tick; field projections update between ticks.
- **Keep phase structure, add an event log as an observer.** Bolt logging onto the existing architecture. Low disruption but agents still sense via direct inspection, so no stigmergy and the log is just a diagnostic tool rather than a coordination substrate. Genesis evaluation gains queryable structure but agent behaviour doesn't improve.
- **Full continuous-time DES (no ticks).** Remove discrete time entirely; events carry timestamps and the simulation advances event-by-event. More expressive but much harder to reason about determinism, and rendering synchronisation becomes complex. Overkill for the current agent complexity.

## The architecture

**Event log** — source of truth, immutable, append-only. Stores only topology changes: events that alter the agent graph or energy flow network. Not a complete state record — full energy reconstruction requires replaying from seed. Queryable by genesis evaluation for food web structure, lineage graphs, and population dynamics.

**Projections** — left folds over the event log that produce derived data for specific consumers. Multiple projections can exist (agent perception, genesis evaluation, replay tooling). Spatial projections decay over time and provide navigational context. The DES computes projections and hands the data to agents alongside broadcasts — agents do not query projections themselves. A projection only presents valid options — agent decisions are valid by construction.

**Broadcasts** — the DES notification mechanism. When an event occurs, the DES delivers it to agents within sensing range. Broadcast is infrastructure — the dispatch protocol, not an ecological concept. Two filtering layers: spatial (sensing range) and subscription (agents may NACK an event type to stop receiving it). NACKs are per-agent and do not inherit to offspring. Includes intermediary signals (e.g., mating availability) that don't persist in the log. Only the resolved outcome (e.g., MateSelected) enters the log.

**Agent responses** — an agent receives a single triggering event (the broadcast) plus relevant projection data. It returns a `Response` struct with two fields: an `Ack` (ACK or NACK) and a `Vec<Event>`. ACK means the agent received the event; the DES queues any returned events. NACK means unsubscribe from this event type; the DES ignores any events in the response and stops delivering that type to this agent. NACKs are per-agent and do not inherit to offspring. The DES queues returned events without validation — correctness is a testing concern. Agents pattern-match on broadcast events and act only on relevant types. Multiple broadcasts may reach the same agent within a tick; each is delivered and responded to individually.

**Consequence cascading** — when an interaction resolves (consumption drains energy), the DES propagates state changes (drain → death → carcass creation) until the world is quiescent before the next agent's decision is processed. Decisions are made by agents; consequences are resolved by the DES.

## Laws of the world

- All perception is local — sensing range is the sole spatial filtering boundary for broadcasts.
- Agents are processed in ID order within a tick (deterministic for a given seed; randomised ordering is a future option).
- Event processing within a tick is FIFO, with consequence events inserted at higher priority than pending agent decisions.
- Interactions resolve within the tick (agents see depleted targets); field projections update between ticks (navigation uses last tick's state).
- Log the non-deterministic (agent-to-agent interactions), recompute the deterministic (photosynthesis, metabolism, movement cost — derivable from traits, position, and world parameters).
- Gestation is a searchable delay: longer gestation costs parents more energy but produces higher-energy offspring.

## Consequences

- The six-phase `World::step()` is replaced by a priority event queue with two levels: consequence resolution (high) and agent decisions (normal).
- Direct agent-to-agent sensing (iterating over nearby agents and reading their traits) is removed. Agents perceive only through broadcast events and projection data, both delivered by the DES.
- The event log gives genesis evaluation direct access to food web structure and lineage graphs without re-running simulations. Aggregate metrics (oscillation, clustering) can be computed from log queries rather than tick-by-tick observation.
- Stigmergy emerges: agents respond to traces of past activity rather than live state. This enables richer ecological dynamics (trail following, predator avoidance from death traces, attraction to productive zones).
- The simulation remains deterministic for a given seed. Reproducibility is preserved.
- Performance in genesis: the event log is sparse (topology changes only). The field projections are O(1) reads per agent. Broadcast filtering is spatial (sensing range), keeping the DES cost at O(events × local_agents) per tick.
- The causality model (ordering, priority, cascade rules) is fixed law. Timing parameters (decay rate, gestation duration) are searchable by genesis.
