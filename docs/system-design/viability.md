# Viability

The *a priori* lens on the world rules. [World rules](world-rules.md) say what the mechanisms are; [expected properties](expected-properties.md) say what should emerge if the mechanisms are calibrated well. This document sits between them and asks: **given a parameterization, can it produce the expected properties at all — before we ever run the simulation?**

Formally, the [execution model](execution-model.md) defines a discrete dynamical system: the tick loop is a map `state_{t+1} = T(state_t)` whose phase order is fixed. Viability analysis is the study of that map's fixed points — **where in world-parameter space does `T` admit a non-trivial, persistent living state, and where does every trajectory collapse into one of the degenerate configurations?** This is not implementation; it is the mathematics of the map the execution model specifies, and it is the most detailed element of system design that remains implementation-agnostic.

## What this document is, and is not

This document **commits nothing**. It introduces no new mechanism, no new functional form, no new parameter. Every result here is a *consequence* of what the world rules and execution model already commit. Its sole output is the **functional form of the parameter regions that cannot work** — the regions where no amount of search will find life, because the physics forbids it.

This has a direct payoff for [world genesis](world-rules.md). The genesis search is expensive (ensembles of replicate runs); viability analysis is nearly free (arithmetic). Running the gates first **shrinks the search box** by excluding provably-dead parameterizations before any ensemble is spent. The search never goes away — that is its purpose — but it should not waste runs on a region viability can rule out in closed form.

When a region's deadness *cannot* be decided here, that is itself a finding: the system design is under-committed at that point. The fix is to commit the missing detail in system design (world rules / execution model) — never to invent it here.

## How much can be decided a priori

The sharpness of a gate is bounded by how much of the map `T` is pinned. Three tiers:

- **Fully pinned** — the phase order; the kappa flow-allocation rule; the linear nutrient demand `demand = base_nutrient_ratio + spec_coeff · (autotrophy + heterotrophy + mobility)`; the route-agnostic nutrient split; both conservation laws; proportional competition; the reproduction gate. Here the math is exact, symbolic in a few constants.
- **Form pinned, coefficients searched** — convex/superlinear trait maintenance; movement cost linear in distance; the monotone structural death threshold. Here the math is parametric: real inequalities and comparative statics.
- **Open** — the photosynthesis income magnitude; the wear accumulation/repair law. Here only structural or bounding statements are possible. (The efficiency-vs-distance form is *parameterised in the scenario files* but not yet owned by system design — see Findings.)

A gate is cheap and decidable today exactly when it rests on the first two tiers.

## Organising principle: the failure modes as gates

The degenerate configurations in [expected properties](expected-properties.md) are the natural index. Each is a way the map `T` fails to hold a living fixed point, and they split by how amenable they are to a priori analysis:

- **Existence failures — cheaply provable dead-zones.** These ask "does a living fixed point exist at all?" — exactly what static analysis answers. *Extinction* and *energy death* are gated below; *generalist dominance* is currently an open finding.
- **Dynamics failures — out of cheap a priori reach (for now).** *Monoculture* vs. coexistence, *frozen dynamics* vs. oscillation, and *population explosion* are not about whether a fixed point exists but about its *stability and multiplicity* — bifurcation and stability territory. They remain the domain of the search until the affirmative theory is built out.

> **Direction of travel.** Today this document is a set of *negative gates* — proofs that a parameterization does **not** fail in a given way. In future these can be reoriented toward the *affirmative* — constructive conditions under which a property **will** appear. The gates are the negative-space sketch of that eventual theory.

## Gate — extinction (flux floor)

*Form-free.* Producer income is bounded by the solar flux in its light-competition neighbourhood; an isolated producer receives at most the full flux magnitude `F`. Metabolism charges at least the base rate `B` every tick (flow 8). So the per-tick net of any producer is at most `F − B`.

> **If `F ≤ B`, no producer is ever net-positive — no energy enters the living system — extinction, for every other parameter and every functional form.**

This is the weakest necessary condition for life — clearing it is far from sufficient — but failing it is a guaranteed kill, decidable from two numbers.

## Gate — energy death (nutrient floor)

*Fully pinned.* Nutrient is conserved. A living agent holds free nutrient, a reproductive earmark, and nutrient bound in structure (`structure · demand`). For a lineage to persist, at least one agent must be simultaneously **embodied** (`structure > 0`, required even to photosynthesise — light share is weighted by structure, flow 1) and **reproduction-ready** (earmark `≥ N_repro_threshold`, the reproduction nutrient gate). Both draw on the conserved total. Hence a necessary condition:

```
N_total  ≥  structure_min · (base_nutrient_ratio + spec_coeff · σ_min)   +   N_repro_threshold
              └──── nutrient bound in one minimal viable body ────┘            └─ earmark to clear the gate ─┘
```

where `σ_min` is the specification sum of a minimal viable agent.

> **If `N_total` is below this, no agent can be both embodied and able to reproduce — no reproduction, no persistence — regardless of solar flux. Energy abundance cannot rescue a nutrient-starved world.**

The gate is deliberately conservative: it ignores nutrient locked in carcasses in transit (death flux ÷ decomposition rate) and the available pool that ongoing uptake needs, both of which only *raise* the real floor. So failing the closed form is sufficient for deadness; clearing it is necessary, not sufficient. This is the analytic statement of the [energy death](expected-properties.md#energy-death) failure mode and the [no-passive-decay](world-rules.md#no-passive-decay) consequence.

## Open finding — generalist dominance

[Expected properties](expected-properties.md) treats generalist dominance as prevented by superlinear *per-trait* maintenance ("a mathematical property of convex maintenance costs"). On analysis this does not hold as stated: with *independent additive* convex costs, a generalist exploiting two niches at `(t, t)` pays `2·c·t^p` and earns roughly `2·R(t)` — exactly twice a specialist's *net*, not less. Convex per-trait cost alone is neutral-to-favourable for breadth.

What actually penalises breadth must be one (or more) of: a **cross/interaction term** in maintenance (the world rules already hint at one — "steepen the autotrophy–mobility interaction cost"), the **structural-fragility** penalty (cost-structure trade-off #9), the **sessile-vs-mobile physical incompatibility** (trade-off #2), or **per-niche income saturation**. Until the anti-generalist mechanism is stated precisely in system design, this failure mode has no clean static gate. (Compounding the uncertainty: the scenario files expose only *linear* maintenance coefficients, with no exponent — a possible docs-vs-implementation divergence.)

This is recorded here as the first product of the lens: an a priori check that caught a place where the stated mechanism may not produce the expected property. Resolving it is a system-design task, not a viability one.

## Place in the validation triad

Viability is one of three complementary lenses on the quality of the system design (see [expected properties](expected-properties.md#how-we-test-for-these-properties)):

1. **A priori viability** (this document) — nearly free; rules regions out in closed form.
2. **Diagnostic examples** (`scenarios/`) — a hand-built initial condition probing a specific failure mode, run headless to observe whether it occurs or the system recovers.
3. **Genesis search** — the broad, expensive empirical sweep of parameter space.

The three are bound together by the **example file**, the only shared concrete object: the *same* initial condition can be predicted by viability, observed by a simulation run, and located relative to the search. That makes the example the unit of cross-validation — a viability gate is a *falsifiable prediction*, and the cheapest place to test it is a single example run. Agreement across the lenses raises confidence; disagreement is diagnostic:

- viability says dead, the run shows life → the gate is wrong or too conservative, or models a mechanism the design lacks;
- viability says nothing, the run fails → the gate set is incomplete, or the failure is a dynamics failure beyond cheap reach.

For this loop to work, an example must carry not just its initial condition but the **failure mode it probes, viability's prediction, and the observed outcome**. Today examples carry only the initial condition; closing that gap is tracked as follow-up work.

## Findings and follow-ups

- **Promote committed forms into system design.** The efficiency-vs-distance form is already parameterised in the scenario files (`base_trophic_efficiency`, `trophic_distance_decay`) but is not owned by system design. Promoting it would convert the trophic-pyramid-height gate from conditional to decidable.
- **State the anti-generalist mechanism precisely** (see the open finding above).
- **Unify the canonical parameter/form model** across genesis, scenarios, and viability so the three lenses describe the same physics.
