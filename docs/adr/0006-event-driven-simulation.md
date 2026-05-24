# Event-driven simulation with log, projections, and ephemeral broadcasts

The current simulation uses a rigid six-phase step function where all agents pass through every phase every tick (sense → light → move → consume → decompose → reproduce/die). Agents sense each other directly by inspecting positions and traits. This works but limits us in two ways: we cannot reconstruct the causal history of the ecology without replaying from seed, and agents have no memory of prior activity — only instantaneous awareness.

## Considered options

- **Event log with spatial field projections and ephemeral broadcasts (chosen).** Replace the phase-structured step with a discrete event simulation (DES). All activity produces events. A durable event log stores topology changes (births, deaths, trophic transfers, mate selections) as the source of truth. Agents perceive the world through projections — spatial read models derived from the log that decay over time. The DES broadcasts events to agents within sensing range; agents react to broadcasts and consult projections to make decisions. Interactions resolve within a tick; field projections update between ticks.
- **Keep phase structure, add an event log as an observer.** Bolt logging onto the existing architecture. Low disruption but agents still sense via direct inspection, so no stigmergy and the log is just a diagnostic tool rather than a coordination substrate. Genesis evaluation gains queryable structure but agent behaviour doesn't improve.
- **Full continuous-time DES (no ticks).** Remove discrete time entirely; events carry timestamps and the simulation advances event-by-event. More expressive but much harder to reason about determinism, and rendering synchronisation becomes complex. Overkill for the current agent complexity.

## The architecture

**Event log** — source of truth, immutable, append-only. Stores only topology changes: events that alter the agent graph or energy flow network. Not a complete state record — full energy reconstruction requires replaying from seed. Queryable by genesis evaluation for food web structure, lineage graphs, and population dynamics.

**Projections** — read models derived from the event log. Multiple projections can exist for different consumers (agent perception, genesis evaluation, replay tooling). Spatial projections decay over time and are what agents read for navigation and context. A projection only presents valid options — agent decisions are valid by construction.

**Ephemeral broadcasts** — the DES pushes events to agents within sensing range. These are stimuli that agents react to. Includes intermediary signals (e.g., mating availability) that don't persist in the log. Only the resolved outcome (e.g., MateSelected) enters the log.

**Agent decisions** — one action per category per tick (not multiple moves, but can move and photosynthesise and signal for mating). Agents respond to broadcasts and consult projections. Decisions produce events; the DES resolves consequences.

**Consequence cascading** — when an interaction resolves (consumption drains energy), the DES propagates state changes (drain → death → carcass creation) until the world is quiescent before the next agent's decision is processed. Decisions are made by agents; consequences are resolved by the DES.

## Laws of the world

- All perception is local — sensing range is the sole broadcast boundary.
- Agents are processed in ID order within a tick (deterministic for a given seed; randomised ordering is a future option).
- Event processing within a tick is FIFO, with consequence events inserted at higher priority than pending agent decisions.
- Interactions resolve within the tick (agents see depleted targets); field projections update between ticks (navigation uses last tick's state).
- Log the non-deterministic (agent-to-agent interactions), recompute the deterministic (photosynthesis, metabolism, movement cost — derivable from traits, position, and world parameters).
- Gestation is a searchable delay: longer gestation costs parents more energy but produces higher-energy offspring.

## Consequences

- The six-phase `World::step()` is replaced by a priority event queue with two levels: consequence resolution (high) and agent decisions (normal).
- Direct agent-to-agent sensing (iterating over nearby agents and reading their traits) is removed. Agents perceive only through projections and broadcasts.
- The event log gives genesis evaluation direct access to food web structure and lineage graphs without re-running simulations. Aggregate metrics (oscillation, clustering) can be computed from log queries rather than tick-by-tick observation.
- Stigmergy emerges: agents respond to traces of past activity rather than live state. This enables richer ecological dynamics (trail following, predator avoidance from death traces, attraction to productive zones).
- The simulation remains deterministic for a given seed. Reproducibility is preserved.
- Performance in genesis: the event log is sparse (topology changes only). The field projections are O(1) reads per agent. Broadcast filtering is spatial (sensing range), keeping the DES cost at O(events × local_agents) per tick.
- The causality model (ordering, priority, cascade rules) is fixed law. Timing parameters (decay rate, gestation duration) are searchable by genesis.
