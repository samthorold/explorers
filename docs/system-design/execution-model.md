# Execution Model

How the world rules are realised through time. The [world rules](world-rules.md) describe what can happen — stocks, flows, conservation laws, trade-offs. This document describes the machinery that makes things happen: how phases are ordered, how interactions are resolved, and how events are recorded.

The execution model is a monolithic tick loop. Agents are plain data structures — they have no behaviour methods. The tick loop reads agent state, computes physics, and mutates state in place. An immutable event log records what happened — it is an observation record, not the execution substrate.

## The simulation core is renderer-agnostic

The simulation is a self-contained core that advances world state and nothing else. It exposes a single stepping operation — conceptually `step(world)` — that applies one tick of physics to the state, and it holds no opinion about timing, rendering, input, or presentation. Everything that drives or observes the simulation is a *client* of this core: a renderer reads world state each frame and draws it; the genesis search runs the core headless, thousands of worlds in parallel, with no renderer at all. The core never reaches up to its clients.

This boundary is load-bearing, not incidental. World genesis — running long sequences of headless ticks to find parameterizations that produce the expected properties, discarding degenerate ones — is only tractable if the world can step without a render loop, a window, or an engine attached. Folding the simulation into whatever engine renders it would drag that engine into every genesis run and into every ecology test, coupling the physics to presentation concerns it must stay free of. Keeping the core a pure state-advancing library is what lets the same physics run under an interactive frontend and under a batch search unchanged. Which engine or framework renders the world is an implementation choice and may change; that the simulation core stays renderer-agnostic is a design constraint that does not.

## Core concepts

### Tick

The fundamental unit of time. Every process in the world — acquisition, metabolism, growth, interaction, wear, death, movement — happens once per tick in a fixed sequence. Time advances by one tick when the sequence completes. There is no sub-tick time; events within a tick are ordered by phase, not by timestamp subdivision.

### Agent

A plain data structure. Each agent holds:

- **reserve** — metabolic fuel (energy)
- **structure** — embodied biomass (energy)
- **nutrient** — incorporated nutrient
- **position** — location on the physical surface
- **traits** — heritable trait vector
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
    move (all agents)
    wear (all agents)
    check death thresholds (all agents)
    append events to log
    verify energy conservation
```

### Properties

**Ordering is visible in one place.** The phase sequence is the source of truth for what happens before what. No priority values, no implicit contracts about queue behaviour.

**Acquire before spend, spend before convert.** The first two phases acquire energy and nutrient. Metabolism spends. Growth converts surplus. This follows the energy flow direction established in the world rules.

**Positions are stable within a tick.** The spatial grid is built once at the start of the tick. All position-dependent phases — autonomous and coordinated — resolve at the positions agents have held since last tick's movement. Movement is the final position-determining phase, repositioning agents for the next tick. The two phases that follow it, wear and the death check, read only per-agent state and never the grid, so position stability holds for everything that depends on it.

**Deaths are immediately reflected.** An agent killed by consumption is excluded from reproduction resolution in the same tick. No stale-read problem.

**Each phase is a function.** State in, deltas out. Deltas applied to state and recorded for the event log. The function boundaries are the natural unit of testing.

**Autonomous phases are vectorisable.** Phases that operate independently per agent (photosynthesis, metabolism, growth, wear, movement) are candidates for batch or matrix computation.

## Autonomous phases

Autonomous phases change only one agent's state each. They run across all agents, independently — no coordination required. The tick loop derives each agent's behaviour from its trait vector, spatial context, and current state.

Autonomous phases, in the order they run:

1. **Photosynthesise** — agents with photosynthetic absorption absorb energy from local solar flux into reserve. Light is shared proportionally among co-located producers (spatial grid query). Photosynthesis is unconditional — it is not exclusive with consumption or any other activity. The trait budget makes dual investment expensive, but the physics do not forbid it.
2. **Absorb nutrients** — agents with autotrophy investment extract nutrient from the local available pool. Uptake depends on the agent's autotrophy trait — the same infrastructure that captures light also extracts nutrients.
3. **Metabolise** — agents pay fixed energy costs from reserve to heat: base rate, per-trait maintenance (superlinear in each specification trait), structure maintenance. These costs are independent of activity — they are the price of existing with a given trait vector. Asexual propensity also carries a small superlinear maintenance cost here — the only reproduction trait that does, so that unused asexual machinery is under directional selection toward zero rather than drifting (see [trait space](trait-space.md), *Asexual propensity*). Somatic maintenance is *not* charged here — it is funded from the kappa-allocated soma share in `grow` (phase 4), where it competes directly with growth and reproduction.
4. **Grow** — agents split reserve surplus by kappa (DEB-style flow allocation). The kappa share funds soma — somatic repair first, then growth (reserve → structure, lossy). The (1 − kappa) share is committed to the agent's reproductive allocation, an earmarked sub-account of reserve that accumulates across ticks until a reproductive event draws it down. The allocation happens to the *flow* of surplus, not to the *stock* of reserve at spend time — once earmarked, the reproductive share is not available to fund subsequent metabolism. Growth itself is automatic — a consequence of being well-fed, not a decision.
5. **Move** — agents reposition on the physical surface. Movement direction is derived from the agent's traits and current spatial context (nearby agents and carcasses, sensed via the spatial grid). Movement distance is scaled by effective mobility. Movement costs energy from reserve. (Runs after all position-dependent phases, so they resolve at stable positions; runs *before* wear so that this tick's movement is charged as mobility use-wear within the same tick.)
6. **Wear** — agents' functional traits degrade from baseline accumulation and use-dependent wear, including the distance moved in this tick's `move` phase. Somatic repair — funded earlier in the tick from the kappa-allocated soma share in `grow` (phase 4) — reduces accumulated wear as a whole-organism investment, not a per-trait selective repair. There is no separate "somatic maintenance trait"; the soma-vs-reproduction allocation is governed by the heritable kappa trait. (Runs after move so the full tick's activity — movement included — is accounted for. Wear reads only per-agent state, never the spatial grid, so running it after move does not disturb position stability.)
7. **Check death thresholds** — agents whose reserve has reached zero (starvation) or whose structure has dropped below their complexity-dependent death threshold die, producing carcasses. (Runs last, after wear, so that a tick's own movement-wear can carry an agent over its death threshold within the same tick.)

The ordering of phases 1–4 follows energy flow direction: acquire before spend, spend before convert. Movement runs after all position-dependent phases so they resolve at stable positions, and so the energy spent on movement is bounded by what remains after all other costs. Wear and the death check run after movement so that the full tick's activity — including the distance just moved — is accounted for before evaluating degradation and survival. Because wear and the death check are autonomous (they read per-agent state, not the grid), placing them after move costs nothing in position stability and removes the need to carry movement distance across ticks.

### Movement as the final repositioning phase

Movement at the end of the tick creates a unified pattern for all strategies:

- **Sessile agents** harvest where they are. They do not move (or barely move), saving energy. Their autotrophy investment captures light and extracts nutrients at their fixed position.
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
2. Compute each consumer's effective demand using world physics (consumer traits, target traits, trophic efficiency, stoichiometric mismatch).
3. If total demand exceeds target's available stock, apply **proportional split** — each consumer receives a share proportional to their demand relative to total demand. When total demand does not exceed supply, everyone gets exactly what the physics computed.
4. Apply drains to target state.
5. Targets whose structure drops below their complexity-dependent death threshold are marked dead. Carcasses created by deaths in this pass are not available for decomposition until the next tick — there is no re-entrant processing within a tick.

#### Pass 2 — Investments

Resolve all interactions where the source invests its own resources:

1. Compute each agent's remaining budget (current stock minus drains received in pass 1).
2. For reproduction: filter to agents still alive and above reproduction threshold post-drain, pair candidates by closest trait-space distance within spatial range, compute investment costs, cap at the parent's accumulated reproductive allocation (the earmarked sub-account of reserve filled by kappa in the grow phase — see world rules flow 9). Reproduction does not draw from unallocated reserve.
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

## Stochasticity and determinism

The simulation is stochastic in exactly three phases — movement (random-walk jitter) and the two reproduction paths (asexual: propensity roll, fecundity, mutation, dispersal; sexual: fecundity, seed-parent choice, crossover, mutation, dispersal) — yet its load-bearing invariant is *the same run seed reproduces the same trajectory, exactly*. Everything above the simulation (the genesis search, deterministic replay, the differential tests) relies on bit-identical replay. How randomness is sourced is therefore part of the execution model, not an implementation detail to be left implicit.

### Keyed-stateless per-agent-per-phase derivation

Randomness is **keyed-stateless**. At each stochastic site the simulation derives a key from the run seed, the acting agent's *stable identity*, the current tick, and a *phase tag*, and seeds a fresh local generator from that key for that one unit of work. No RNG state is stored on the agent or threaded through the tick. Each agent's draws come off its own local stream in deterministic code order; draws belonging to different agents never share a stream.

The key is a pure function of `(run seed, identity, tick, phase tag)`. The single entropy source for a run is still its seed — the per-site key derivation expands that one seed into independent streams, it does not introduce new entropy. The hash that folds the key fields and the generator that consumes the key are implementation choices the code owns and freezes; what this document fixes is the *shape* of the key and the architecture around it.

**Why key on per-agent identity rather than draw off one shared stream.** The natural implementation — one generator threaded through the tick, every phase drawing off it in consumption order — makes each agent's outcome depend on *which agents drew before it, and in what order*. Determinism then silently rests on every stochastic phase iterating agents in a stable, world-state-derived order forever. That is a fragile, invisible contract: any phase that iterates agents in an order derived from a hash set, a spatial-grid bucket, or an unstable sort breaks replay without any local code looking wrong. Keying each draw on stable identity dissolves the contract — iteration order stops being load-bearing because the stream an agent draws from is determined by *who the agent is*, not *when it is reached*. (This is the general form of a concrete replay bug whose narrow symptom was once patched with a sorted-iteration fix; per-agent keying removes the whole class.)

**Why a phase tag.** The phase tag makes the three stochastic phases draw from disjoint streams even for the same agent and tick. This is a firewall: adding, removing, or reordering a draw inside one phase cannot perturb another phase's stream. Movement and the two reproduction paths evolve independently.

**Single-agent versus ordered-pair keying.** Movement jitter and asexual reproduction are single-agent events: they key on the one acting agent's identity. Sexual reproduction is a *pair* event, and its key is the **symmetric ordered pair** of the two parents' identities — the pair is sorted into a canonical (low, high) order before keying, so the stream is independent of which parent the resolution loop happens to visit as "first". A direct consequence, and an intended one: the seed-parent choice (which parent's position and dispersal kernel place the brood) becomes a pure function of the parent pair and the tick. Every per-pair decision that reads from this stream — fecundity, the seed coin, crossover, mutation, dispersal — likewise binds its branches to the parents' stable identities rather than to their slice positions, so a permuted agent ordering cannot flip which parent contributes what.

### Order-independent newborn identity

Keyed draws are necessary but not sufficient for order-independent reproduction, because newborns also need *identities*, and identity assignment is itself an ordering hazard. If newborns took their id from a counter advanced in iteration order, a reordered reproduction loop would mint the same offspring with different ids — and since the next tick keys every newborn's stochastic outcome on its id, divergent ids diverge the entire downstream trajectory.

Newborn ids are therefore assigned in a **canonical order**, not in birth order. All offspring produced in a tick — asexual and sexual together — are sorted by a world-state-derived key (the producing parent's identity, or the producing pair's ordered identities, then the offspring's slot within that brood), and only then handed sequential ids from the population's id counter. Ids stay compact, monotonic, and unique, and the same brood receives the same ids no matter what order the parents were resolved in.

### What this buys, and what stays out of scope

Together these make every RNG-derived quantity — every jitter, every offspring's traits and placement, every agent's identity — a pure function of world state, invariant under any permutation of agent iteration order. A trajectory-level guard exercises exactly this: it permutes agent order within the stochastic phases across a full run and asserts the demographics and identities are unchanged.

Re-seeding is a one-time event. Because outcomes are keyed differently than a shared-stream design would key them, adopting keyed-stateless RNG shifts every birth count and population number once — expected, and not a regression. What must *not* shift is classification: each scenario keeps its qualitative verdict across the seed ensemble. Two boundaries are deliberately preserved: the non-stochastic phases (acquisition, metabolism, growth, drains, wear, the death check) remain RNG-free and must never begin consuming randomness; and floating-point summation in the coordinated non-RNG phases is associative-order-sensitive at the level of a unit in the last place, so summed-energy state can differ by rounding under a reordering — that is ordinary floating-point behaviour in phases this model leaves untouched, not a determinism leak, and it is distinct from the RNG-derived quantities, which are exactly invariant.

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
- **Specific physics formulas.** Trophic efficiency curves, defensive trait modifiers — these are world rules, not tick loop architecture.
