# Execution Model

How the world rules are realised through time. The [world rules](world-rules.md) describe what can happen — stocks, flows, conservation laws, trade-offs. This document describes the machinery that makes things happen: how events are produced, ordered, distributed, and recorded.

The execution model is a discrete event simulation (DES) with event sourcing. Events are immutable facts. State is derived from the event log.

## Core concepts

### Event

An immutable record of something that happened. "Agent 7 consumed 3.2 energy from agent 12 at t=4.71" — a fact, not an instruction. Once appended to the event log, an event is never modified or deleted.

Events are the atoms of causality. Every change in the world — birth, death, movement, consumption, energy transfer — is an event. If it isn't in the log, it didn't happen.

### Event log

The immutable, append-only record of every event that has occurred. The event log is the single source of truth for the simulation. The complete world history can be reconstructed by replaying the log from the beginning.

The log is ordered: every event has a timestamp and a position in the log. Events are appended in the order they are processed.

### Event queue

The transient machinery that determines what happens next. The queue holds events waiting to be processed, ordered by timestamp (earliest first). Events at the same timestamp are ordered by priority — some events must resolve before others within the same moment (e.g., death before interaction).

The queue is not the source of truth. It is working state — the frontier of unprocessed events. Once an event is popped from the queue and processed, it moves to the log.

### Clock agent

A distinguished agent that emits events at regular intervals. The clock is the sole source of time advance — time moves forward only when the clock emits. All time-driven processes (metabolism, wear, growth) happen in response to clock events, not through a separate execution path.

The clock is not special machinery. It is an agent that follows the same rules as every other agent: it receives events, it emits events. It just happens to emit on a schedule rather than in response to stimuli.

### Agent state

Each agent maintains private state — its own projection of the event log, optimised for fast response. Agent state is a performance cache, not a source of truth. It is never shared between agents. Any agent's state could be reconstructed from scratch by replaying the event log.

When the DES broadcasts an event to an agent, the agent may update its private state and respond with zero or more new events. The state update and the response are the agent's only interface with the world.

### Projection

A derived data structure computed from the event log for a specific purpose. Agent state is one kind of projection. Genesis evaluation, replay tooling, and spatial navigation each use their own projections over the same log. Multiple projections can coexist, each folding the log differently.

A projection is not the truth — the log is. A projection is a lens.

## The DES loop

The simulation advances through a single, uniform loop:

1. **Pop** the next event from the queue (earliest timestamp, highest priority).
2. **Append** the event to the log.
3. **Broadcast** the event to all relevant agents.
4. Each agent may update its private state and **respond** with zero or more new events.
5. **Enqueue** all response events.
6. Repeat from 1.

When the queue is empty, the simulation has settled. The clock agent's next scheduled event will eventually arrive to advance time.

Every event flows through the same loop. There are no special paths for time-driven vs. interaction-driven processes. A clock event and a consumption event are processed identically — pop, append, broadcast, collect responses, enqueue.

### Ordering guarantees

Events are processed strictly one at a time. When an event is broadcast, each receiving agent sees the world state as of that event — including all prior events and their consequences. There is no batching, no simultaneous resolution.

Within the same timestamp, priority determines order. This is how causal consequences resolve within a single moment: a death event at t=5.0 with high priority is processed before a consumption event at t=5.0 with normal priority, preventing interactions with dead agents.

### Causal chains

An event may trigger responses that trigger further responses. A clock event at t=5.0 may cause agent 7 to emit a movement event, which causes agent 7 to enter consumption range of agent 12, which causes a consumption event, which causes agent 12's structure to drop below threshold, which causes a death event. All of these carry timestamp t=5.0 and resolve through the queue's priority ordering before time advances to the next clock event.

A causal chain settles when no agent produces further responses. The queue drains of events at the current timestamp, and the next clock event advances time.

## What this document does not define

- **Which event types exist.** The vocabulary of events (birth, death, consumption, metabolism, etc.) is defined by the world rules. The execution model processes them uniformly.
- **What "relevant" means.** How the DES determines which agents receive a broadcast — spatial filtering, subscription, network topology — is a design question to be resolved separately.
- **Agent decision logic.** How an agent decides what events to emit in response to a broadcast is agent behaviour, not execution model.
- **Specific priority levels.** Which events are higher priority than others is a calibration concern. The execution model provides the ordering mechanism.
