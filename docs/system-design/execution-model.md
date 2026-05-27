# Execution Model

How the world rules are realised through time. The [world rules](world-rules.md) describe what can happen — stocks, flows, conservation laws, trade-offs. This document describes the machinery that makes things happen: how phases are ordered, how interactions are resolved, and how events are recorded.

The execution model is a monolithic tick loop. State lives in agent data structures and is mutated in place. An immutable event log records what happened — it is an observation record, not the execution substrate.

## Core concepts

### Tick

The fundamental unit of time. Every process in the world — acquisition, metabolism, growth, interaction, wear, death — happens once per tick in a fixed sequence. Time advances by one tick when the sequence completes. There is no sub-tick time; events within a tick are ordered by phase, not by timestamp subdivision.

### Agent state

Each agent's state — reserve, structure, nutrient, position, traits, wear — lives in a mutable data structure. State is the source of truth. When a phase function runs, it reads current state, computes results, and mutates state in place. Changes made by one phase are immediately visible to all subsequent phases in the same tick.

### Event

An immutable record of something that happened. "Agent 7 consumed 3.2 energy from agent 12 at tick 40" — a fact, not an instruction. Events are produced as output by each phase function and appended to the log. They do not flow through a queue. They do not trigger agent responses. They do not chain causally through re-entrant processing.

### Event log

The immutable, append-only record of every event that has occurred. The log serves three purposes: debugging (trace back through history), presentation (drive visual and audio effects), and replay (deterministic reconstruction from log). The complete world history can be reconstructed by replaying the log from the beginning.

If log and state diverge, the log has a bug — state is truth.

## The tick loop

A single loop with explicit phases, run once per tick:

```
for each tick:
    photosynthesise (all agents)
    absorb nutrients (all agents)
    metabolise (all agents)
    grow (all agents)
    collect intents
    resolve consumption → mark deaths
    resolve reproduction (excluding dead agents)
    resolve redistribution (excluding dead agents)
    wear (all agents)
    check death thresholds (all agents)
    append events to log
```

### Properties

**Ordering is visible in one place.** The phase sequence is the source of truth for what happens before what. No priority values, no implicit contracts about queue behaviour.

**Deaths are immediately reflected.** An agent killed by consumption is excluded from reproduction resolution in the same tick. No stale-read problem.

**Each phase is a function.** Inputs in, results out. Results applied to state and appended to the log. The function boundaries are the natural unit of testing.

**Autonomous phases are vectorisable.** Phases that operate independently per agent (photosynthesis, metabolism, growth, wear) are candidates for batch or matrix computation. No queue serialisation prevents this.

## Autonomous phases

The first four phases and the last two change only one agent's state each. They run across all agents, independently — no coordination required. Each agent evaluates its own traits, local conditions, and current state, producing a result that is applied to its own state.

Autonomous phases, in the order they run:

1. **Photosynthesise** — agents absorb energy from local solar flux into reserve.
2. **Absorb nutrients** — agents extract nutrient from the local available pool.
3. **Metabolise** — agents pay energy costs (base + trait maintenance + somatic maintenance) from reserve to heat.
4. **Grow** — agents convert reserve surplus to structure (lossy).
5. **Wear** — agents' functional traits degrade from use and baseline accumulation. (Runs after coordinated phases.)
6. **Check death thresholds** — agents whose reserve or structure has crossed a death threshold die, producing carcasses. (Runs after wear.)

The ordering follows energy flow direction: acquire before spend, spend before convert. Wear and death checks run after coordinated phases so that the full tick's activity — including consumption, reproduction, and redistribution — is accounted for before evaluating degradation and survival.

## Coordinated phases

Between grow and wear, three phases resolve multi-agent interactions. Because these interactions change multiple agents' states and can conflict (multiple consumers targeting the same agent, multiple agents competing to reproduce), a single resolution function — the interaction coordinator — arbitrates the outcome.

### Interaction coordinator

The coordinator is an arbiter that applies world physics. Agents declare what they want to do. The coordinator determines what actually happens given the rules of the world.

Agents own the decision of *which* actions to declare. The coordinator owns *what results* from those declarations — magnitudes, efficiency losses, conflict resolution, conservation enforcement.

The coordinator is stateless between ticks. Within a single tick's resolution it maintains working state (intermediate results between passes), but this is discarded once resolution completes. It is a pure function: `(intents, projections) → events`.

The coordinator is called at a known point in the phase sequence — after autonomous acquisition and growth, before wear and death checks. It is not a distinguished agent; it is a function.

### Intents

An intent is a thin declaration emitted by an agent. It names what the agent wants to do, not how much or how well. The coordinator computes magnitudes from world rules and projections.

Intent shape varies by verb:

- **Consume**: `{verb: consume, source, target, position, traits}`
- **Decompose**: `{verb: decompose, source, target, position, traits}`
- **Reproduce**: `{verb: reproduce, source, position, traits}` (undirected — no target)
- **Redistribute**: `{verb: redistribute, source, target, position}` (deferred)

Agents self-filter before emitting: an agent only declares intents it believes it can afford based on its current state.

An agent may emit multiple intents per tick. There is no exclusivity rule — the energy budget is the only constraint on how many actions fire.

Whether intents require an explicit data structure or are implicit in the trait vector plus spatial position depends on whether agents have genuine choices (choosing *not* to consume when they could). This is an open design question.

### Projections

The coordinator receives read-only projections as input alongside the intents. These provide the world state needed to compute physics:

- **Spatial** — positions of all agents, indexed for proximity queries. Used to validate that directed intents (consume, decompose) are within range, and to enforce proximity constraints on mating.
- **Stock** — current reserve, structure, and nutrient of all agents. Used to cap proportional splits and to validate outgoing budgets in pass 2.
- **Liveness** — which agents are alive, which are carcasses. Used to validate verb/target compatibility (consume targets living agents; decompose targets carcasses).
- **Contact time** — consecutive ticks each agent has spent at current position. Used in physics calculations (nutrient uptake saturation, consumption effectiveness).
- **Traits** — trait vectors of all agents. Used to compute trophic efficiency, defensive modifiers, mate pairing, and all trait-dependent physics.

Because the coordinator runs after all autonomous phases have completed and mutated state, these projections reflect the current tick's autonomous results. There is no stale-read problem — an agent killed by reserve depletion during metabolism is already dead when the coordinator reads liveness.

### Resolution

Resolution happens in two passes within a single tick.

#### Pass 1 — Incoming

Resolve all intents that drain a target:

1. For each target, gather all consume/decompose intents against it.
2. Compute each intent's effective magnitude using world physics (consumer traits, target traits, contact time, trophic efficiency, stoichiometric mismatch).
3. If total demand exceeds target's available stock, apply **proportional split** — each consumer receives a share proportional to their demand relative to total demand.
4. Record effective drains against each target in working state.
5. Apply drains to target state. Targets whose structure drops below their complexity-dependent death threshold are marked dead.

#### Pass 2 — Outgoing

Resolve all intents where the source invests its own resources:

1. Compute each agent's remaining budget (current stock minus drains received in pass 1).
2. For reproduction: collect all reproduce intents, filter to agents still alive and above reproduction threshold post-drain, pair candidates by closest trait-space distance within spatial range, compute investment costs, cap at remaining budget.
3. For redistribution: compute transfer amounts, cap at remaining budget.
4. Apply results to agent state.

#### Conflict resolution

**Inter-agent conflicts** (multiple agents targeting the same target): proportional split. Each consumer's effective drain is scaled by `(their_demand / total_demand) * available_supply`. When total demand does not exceed supply, everyone gets exactly what the physics computed.

**Mate pairing** (multiple agents available for reproduction): the coordinator pairs by closest trait-space distance within spatial proximity. Traits are carried on the intent. An agent that receives multiple potential pairings is matched to the closest in trait space.

**No intra-agent exclusivity**: agents may consume, reproduce, and redistribute in the same tick. The budget is the constraint, not a priority rule.

#### Conservation

The two-pass structure enforces conservation:

- Pass 1 ensures no target is drained beyond its available stock (proportional split).
- Pass 2 ensures no agent spends beyond its post-drain budget (cap at remaining).
- Overdraw is not permitted. The system never creates more currency than exists in stocks.

### Coordinated event output

The coordinator produces events — immutable facts about what happened:

- **Consumed(consumer, target, energy_amount, nutrient_amount, t)**
- **Decomposed(consumer, carcass, energy_amount, nutrient_amount, t)**
- **Reproduced(parent_a, parent_b, offspring_traits, position, t)**
- **Redistributed(sender, receiver, energy_amount, nutrient_amount, t)** (deferred)

These events are appended to the log alongside the autonomous phase events. They serve the same purposes — debugging, presentation, replay — and are not fed back into the loop.

### Negotiation window

Resolution covers exactly one tick. There is no multi-tick negotiation, no persistent locks on targets, no ongoing interactions tracked by the coordinator.

"Sustained contact" (e.g., consumption over many ticks) emerges from an agent repeatedly declaring the same intent tick after tick. The coordinator resolves each tick independently.

## Event vocabulary

The domain vocabulary serves as the observation language. Events are produced by each phase function and appended to the log.

### Autonomous events

- **Photosynthesized** — agent absorbed energy from local solar flux
- **NutrientAbsorbed** — agent absorbed nutrient from local available pool
- **Metabolized** — agent paid energy costs from reserve
- **Grew** — agent converted reserve surplus to structure
- **Wore** — agent's functional traits degraded
- **Died** — agent crossed a death threshold

### Coordinated events

- **Consumed** — consumer drained target's structure
- **Decomposed** — consumer drained carcass structure
- **Reproduced** — parents produced offspring
- **Redistributed** — resource transfer through network

## What events are not

Events are not the execution mechanism. They do not flow through a queue. They do not trigger agent responses. They do not chain causally through re-entrant processing. They are records of what happened, produced as output by each phase.

## What this document does not define

- **Functional forms.** How much energy a given trait investment produces, what the metabolic base rate is, what the growth conversion efficiency is — these are calibration, not execution model.
- **Agent decision logic.** How an agent decides which intents to emit is agent behaviour, not execution model.
- **Specific physics formulas.** Trophic efficiency curves, contact time saturation functions, defensive trait modifiers — these are world rules, not coordinator architecture.
