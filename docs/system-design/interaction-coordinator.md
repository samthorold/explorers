# Interaction Coordinator

How coordinated events are resolved. The [execution model](execution-model.md) defines the DES loop and event categories. This document defines what the interaction coordinator knows, receives, and does.

## Role

The coordinator is an arbiter that applies world physics. Agents declare what they want to do. The coordinator determines what actually happens given the rules of the world.

Agents own the decision of *which* actions to declare. The coordinator owns *what results* from those declarations — magnitudes, efficiency losses, conflict resolution, conservation enforcement.

## Statefulness

Stateless between ticks. The coordinator carries no memory from one tick to the next. Within a single tick's resolution it maintains working state (intermediate results between passes), but this is discarded once resolution completes.

The coordinator is a pure function: `(intents, projections) → events`.

## Intents

An intent is a thin declaration emitted by an agent. It names what the agent wants to do, not how much or how well. The coordinator computes magnitudes from world rules and projections.

Intent shape varies by verb:

- **Consume**: `{verb: consume, source, target, position, traits}`
- **Decompose**: `{verb: decompose, source, target, position, traits}`
- **Reproduce**: `{verb: reproduce, source, position, traits}` (undirected — no target)
- **Redistribute**: `{verb: redistribute, source, target, position}` (deferred)

Agents self-filter before emitting: an agent only declares intents it believes it can afford based on its current state. The coordinator does not reject intents for budget reasons during pass 1.

An agent may emit multiple intents per tick. There is no exclusivity rule — the energy budget is the only constraint on how many actions fire.

## Projections

The coordinator receives read-only projections as input alongside the intents. These provide the world state needed to compute physics:

- **Spatial** — positions of all agents, indexed for proximity queries. Used to validate that directed intents (consume, decompose) are within range, and to enforce proximity constraints on mating.
- **Stock** — current reserve, structure, and nutrient of all agents. Used to cap proportional splits and to validate outgoing budgets in pass 2.
- **Liveness** — which agents are alive, which are carcasses. Used to validate verb/target compatibility (consume targets living agents; decompose targets carcasses).
- **Contact time** — consecutive ticks each agent has spent at current position. Used in physics calculations (nutrient uptake saturation, consumption effectiveness).
- **Traits** — trait vectors of all agents. Used to compute trophic efficiency, defensive modifiers, mate pairing, and all trait-dependent physics.

## Resolution

Resolution happens in two passes within a single tick.

### Pass 1 — Incoming

Resolve all intents that drain a target:

1. For each target, gather all consume/decompose intents against it.
2. Compute each intent's effective magnitude using world physics (consumer traits, target traits, contact time, trophic efficiency, stoichiometric mismatch).
3. If total demand exceeds target's available stock, apply **proportional split** — each consumer receives a share proportional to their demand relative to total demand.
4. Record effective drains against each target in working state.

### Pass 2 — Outgoing

Resolve all intents where the source invests its own resources:

1. Compute each agent's remaining budget (stock from projection minus drains received in pass 1).
2. For reproduction: collect all reproduce intents, filter to agents still above reproduction threshold post-drain, pair candidates by closest trait-space distance within spatial range, compute investment costs, cap at remaining budget.
3. For redistribution: compute transfer amounts, cap at remaining budget.

### Conflict resolution

**Inter-agent conflicts** (multiple agents targeting the same target): proportional split. Each consumer's effective drain is scaled by `(their_demand / total_demand) * available_supply`. When total demand does not exceed supply, everyone gets exactly what the physics computed.

**Mate pairing** (multiple agents available for reproduction): the coordinator pairs by closest trait-space distance within spatial proximity. Traits are carried on the intent. An agent that receives multiple potential pairings is matched to the closest in trait space.

**No intra-agent exclusivity**: agents may consume, reproduce, and redistribute in the same tick. The budget is the constraint, not a priority rule.

### Conservation

The two-pass structure enforces conservation:

- Pass 1 ensures no target is drained beyond its available stock (proportional split).
- Pass 2 ensures no agent spends beyond its post-drain budget (cap at remaining).
- Overdraw is not permitted. The system never creates more currency than exists in stocks.

## Output

The coordinator emits events — immutable facts about what happened:

- **Consumed(consumer, target, energy_amount, nutrient_amount, t)**
- **Decomposed(consumer, carcass, energy_amount, nutrient_amount, t)**
- **Reproduced(parent_a, parent_b, offspring_traits, position, t)**
- **Redistributed(sender, receiver, energy_amount, nutrient_amount, t)** (deferred)

Events are broadcast. Distribution strategy (all agents vs. participants only vs. proximity-based) is an implementation decision.

## Negotiation window

Resolution covers exactly one tick. There is no multi-tick negotiation, no persistent locks on targets, no ongoing interactions tracked by the coordinator.

"Sustained contact" (e.g., consumption over many ticks) emerges from an agent repeatedly declaring the same intent tick after tick. The coordinator resolves each tick independently.

## What this document does not define

- **Agent decision logic.** How an agent decides which intents to emit is agent behaviour.
- **Specific physics formulas.** Trophic efficiency curves, contact time saturation functions, defensive trait modifiers — these are world rules, not coordinator architecture.
- **Broadcast filtering.** Who receives the coordinator's output events is an implementation decision.
- **Priority ordering relative to autonomous events.** The execution model defines that autonomous events settle before coordinated events within a timestamp.
