# Execution Model

How the world rules are realised through time. The [world rules](world-rules.md) describe what can happen — stocks, flows, conservation laws, trade-offs. This document describes the machinery that makes things happen: how phases are ordered, how interactions are resolved, and how events are recorded.

The execution model is a monolithic tick loop. Agents are plain data structures — they have no behaviour methods. The tick loop reads agent state, computes physics, and mutates state in place. An immutable event log records what happened — it is an observation record, not the execution substrate.

## Core concepts

### Tick

The fundamental unit of time. Every process in the world — acquisition, metabolism, growth, interaction, wear, death, movement — happens once per tick in a fixed sequence. Time advances by one tick when the sequence completes. There is no sub-tick time; events within a tick are ordered by phase, not by timestamp subdivision.

### Agent

A plain data structure. Each agent holds:

- **reserve** — metabolic fuel (energy)
- **structure** — embodied biomass (energy)
- **nutrient** — incorporated nutrient
- **position** — location on the physical surface
- **traits** — heritable trait vector (L1-normalised budget)
- **contact time** — consecutive ticks at current position
- **wear** — per-functional-trait somatic degradation

These fields are defined by the [world rules](world-rules.md). Agents are automata — their traits determine their strategy. The tick loop derives what each agent does from its trait vector, spatial context, and current state. Agents do not emit intents, make decisions, or respond to events. The trait vector *is* the strategy; evolution selects strategies.

### State

Agent state is the source of truth. When a phase function runs, it reads current state, computes results, and mutates state in place. Changes made by one phase are immediately visible to all subsequent phases in the same tick.

### Event

An immutable record of something that happened. "Agent 7 consumed 3.2 energy from agent 12 at tick 40" — a fact, not an instruction. Events are produced as output by each phase and appended to the log. They do not flow through a queue. They do not trigger agent responses. They do not chain causally through re-entrant processing. The tick loop never reads the event log.

### Event log

The immutable, append-only record of every event that has occurred. The log serves three purposes: debugging (trace back through history), presentation (drive visual and audio effects), and replay (deterministic reconstruction from log).

If log and state diverge, the log has a bug — state is truth.

The event log may be consumed by post-hoc analysis tools (trophic network reconstruction, lineage tracking) outside the tick loop. These tools are consumers of the log, not participants in the execution model.

## The tick loop

A single loop with explicit phases, run once per tick:

```
for each tick:
    build spatial grid
    photosynthesise (all agents)
    absorb nutrients (all agents)
    metabolise (all agents)
    grow (all agents)
    resolve drains (coordinated — living and carcass targets)
    mark deaths
    resolve reproduction (coordinated — excluding dead agents)
    wear (all agents)
    check death thresholds (all agents)
    move (all agents)
    append events to log
    verify energy conservation
```

### Properties

**Ordering is visible in one place.** The phase sequence is the source of truth for what happens before what. No priority values, no implicit contracts about queue behaviour.

**Acquire before spend, spend before convert.** The first two phases acquire energy and nutrient. Metabolism spends. Growth converts surplus. This follows the energy flow direction established in the world rules.

**Positions are stable within a tick.** The spatial grid is built once at the start of the tick. All phases — autonomous and coordinated — resolve at the positions agents have held since last tick's movement. Movement is the final phase, repositioning agents for the next tick.

**Deaths are immediately reflected.** An agent killed by consumption is excluded from reproduction resolution in the same tick. No stale-read problem.

**Each phase is a function.** State in, deltas out. Deltas applied to state and recorded for the event log. The function boundaries are the natural unit of testing.

**Autonomous phases are vectorisable.** Phases that operate independently per agent (photosynthesis, metabolism, growth, wear, movement) are candidates for batch or matrix computation.

## Autonomous phases

Autonomous phases change only one agent's state each. They run across all agents, independently — no coordination required. The tick loop derives each agent's behaviour from its trait vector, spatial context, and current state.

Autonomous phases, in the order they run:

1. **Photosynthesise** — agents with photosynthetic absorption absorb energy from local solar flux into reserve. Light is shared proportionally among co-located producers (spatial grid query). Photosynthesis is unconditional — it is not exclusive with consumption or any other activity. The trait budget makes dual investment expensive, but the physics do not forbid it.
2. **Absorb nutrients** — agents with nutrient absorption extract nutrient from the local available pool. Uptake depends on the agent's trait investment and contact time at current position.
3. **Metabolise** — agents pay fixed energy costs from reserve to heat: base rate, trait maintenance, somatic maintenance, structure maintenance. These costs are independent of activity — they are the price of existing with a given trait vector.
4. **Grow** — agents convert reserve surplus to structure (lossy). Growth is automatic — a consequence of being well-fed, not a decision.
5. **Wear** — agents' functional traits degrade from baseline accumulation and use-dependent wear. Somatic repair reduces wear proportional to the agent's somatic maintenance trait. (Runs after coordinated phases so the full tick's activity is accounted for.)
6. **Check death thresholds** — agents whose reserve has reached zero (starvation) or whose structure has dropped below their complexity-dependent death threshold die, producing carcasses. (Runs after wear.)
7. **Move** — agents reposition on the physical surface. Movement direction is derived from the agent's traits and current spatial context (nearby agents and carcasses, sensed via the spatial grid). Movement distance is scaled by effective mobility. Movement costs energy from reserve. Moving resets contact time. (Runs last — movement is an investment in next tick's positioning.)

The ordering of phases 1–4 follows energy flow direction: acquire before spend, spend before convert. Wear and death checks run after coordinated phases so that the full tick's activity is accounted for before evaluating degradation and survival. Movement runs last so that all phases in the current tick resolve at stable positions, and the energy spent on movement is bounded by what remains after all other costs.

### Movement as the final phase

Movement at the end of the tick creates a unified pattern for all strategies:

- **Sessile agents** harvest where they have been holding position. Contact time accumulates across ticks, increasing nutrient uptake effectiveness. They do not move (or barely move), preserving contact time and saving energy.
- **Mobile agents** harvest at their current position (consumption resolves at held positions), then reposition for the next tick. The hunt spans ticks: reposition, then harvest. Movement cost is bounded by remaining reserve after all other income and costs.

Both strategies follow the same pattern: position yourself, then harvest. Sessile agents position once and harvest indefinitely. Mobile agents reposition every tick and harvest the following tick.

## Coordinated phases

Between grow and wear, two phases resolve multi-agent interactions. Because these interactions change multiple agents' states and can conflict (multiple consumers targeting the same agent, multiple agents competing to reproduce), resolution uses a two-pass structure that enforces conservation and resolves conflicts simultaneously.

The tick loop derives which agents participate from their trait vectors and spatial context — agents with nonzero effective consumption or scavenging traits within range of a valid target are consumers; agents above the reproduction threshold with compatible mates in range are candidates for reproduction. There is no intent collection step — the physics determines participation directly from state.

### Resolution

Resolution happens in two passes within a single tick.

#### Pass 1 — Drains

Resolve all interactions that drain a target. Living agents and carcasses are resolved in a single pass — the proportional-split algorithm is the same regardless of target type.

1. For each target (living or carcass), gather all agents with consumption or scavenging traits within contact range.
2. Compute each consumer's effective demand using world physics (consumer traits, target traits, contact time, trophic efficiency, stoichiometric mismatch).
3. If total demand exceeds target's available stock, apply **proportional split** — each consumer receives a share proportional to their demand relative to total demand. When total demand does not exceed supply, everyone gets exactly what the physics computed.
4. Apply drains to target state.
5. Targets whose structure drops below their complexity-dependent death threshold are marked dead. Carcasses created by deaths in this pass are not available for decomposition until the next tick — there is no re-entrant processing within a tick.

#### Pass 2 — Investments

Resolve all interactions where the source invests its own resources:

1. Compute each agent's remaining budget (current stock minus drains received in pass 1).
2. For reproduction: filter to agents still alive and above reproduction threshold post-drain, pair candidates by closest trait-space distance within spatial range, compute investment costs, cap at remaining budget.
3. For redistribution: compute transfer amounts, cap at remaining budget. (Deferred.)
4. Apply results to agent state.

#### Conflict resolution

**Inter-agent conflicts** (multiple agents targeting the same target): proportional split. Each consumer's effective drain is scaled by `(their_demand / total_demand) * available_supply`.

**Mate pairing** (multiple agents available for reproduction): paired by closest trait-space distance within spatial proximity. An agent that receives multiple potential pairings is matched to the closest in trait space.

**No intra-agent exclusivity**: agents may consume, decompose, reproduce, and redistribute in the same tick. An agent with both consumption and scavenging traits can drain a living agent and a carcass in the same tick if both are in range. The budget is the constraint, not a priority rule.

#### Conservation

The two-pass structure enforces conservation:

- Pass 1 ensures no target is drained beyond its available stock (proportional split).
- Pass 2 ensures no agent spends beyond its post-drain budget (cap at remaining).
- Overdraw is not permitted. The system never creates more currency than exists in stocks.

### Negotiation window

Resolution covers exactly one tick. There is no multi-tick negotiation, no persistent locks on targets, no ongoing interactions.

"Sustained contact" (e.g., consumption over many ticks) emerges from an agent remaining in range of the same target tick after tick. Each tick is resolved independently.

## Energy conservation verification

Each phase function computes and returns energy deltas. The tick loop applies deltas to agent state and records them separately for conservation verification. Phase functions do not interact with the verification system — they compute physics and return results.

At the tick boundary, the energy ledger verifies:

- Solar input is the sole energy source. No energy flows into the solar tap.
- Dissipation is the sole energy sink. No energy flows out of dissipation.
- No agent or carcass spends more energy than it received.
- Total input (solar + pre-tick endowments) equals total dissipation plus energy retained by agents and carcasses.

Conservation verification is an orthogonal concern — a wrapper around the tick loop, not a participant in it. It can be disabled in release builds without affecting the simulation.

## Event vocabulary

The domain vocabulary serves as the observation language. Events are produced by each phase and appended to the log at the end of the tick.

### Autonomous events

- **Photosynthesized** — agent absorbed energy from local solar flux
- **NutrientAbsorbed** — agent absorbed nutrient from local available pool
- **Metabolized** — agent paid energy costs from reserve
- **Grew** — agent converted reserve surplus to structure
- **Wore** — agent's functional traits degraded
- **Died** — agent crossed a death threshold
- **Moved** — agent repositioned on the physical surface

### Coordinated events

- **Consumed** — consumer drained target's structure (living or carcass)
- **Reproduced** — parents produced offspring
- **Redistributed** — resource transfer through network (deferred)

## What events are not

Events are not the execution mechanism. They do not flow through a queue. They do not trigger agent responses. They do not chain causally through re-entrant processing. They are records of what happened, produced as output by each phase. The tick loop never reads the event log.

## What this document does not define

- **Functional forms.** How much energy a given trait investment produces, what the metabolic base rate is, what the growth conversion efficiency is — these are calibration, not execution model.
- **Movement direction computation.** How agents derive movement direction from spatial context (chemotaxis, random walk components) is a world-rules detail, not execution model.
- **Specific physics formulas.** Trophic efficiency curves, contact time saturation functions, defensive trait modifiers — these are world rules, not tick loop architecture.
