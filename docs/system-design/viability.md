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
- **Form pinned, coefficients searched** — convex/superlinear trait maintenance; movement cost linear in distance; the monotone structural death threshold; the exponential efficiency-vs-distance trophic-transfer form (`base_trophic_efficiency · exp(−trophic_distance_decay · d)`, [world rules](world-rules.md), flow 7). Here the math is parametric: real inequalities and comparative statics.
- **Open** — the photosynthesis income magnitude; the wear accumulation/repair law. Here only structural or bounding statements are possible.

A gate is cheap and decidable today exactly when it rests on the first two tiers.

## Organising principle: the failure modes as gates

The degenerate configurations in [expected properties](expected-properties.md) are the natural index. Each is a way the map `T` fails to hold a living fixed point, and they split by how amenable they are to a priori analysis:

- **Existence failures — cheaply provable dead-zones.** These ask "does a living fixed point exist at all?" — exactly what static analysis answers. *Extinction* and *energy death* are gated below.
- **Dynamics failures — out of cheap a priori reach (for now).** *Monoculture* vs. coexistence, *frozen dynamics* vs. oscillation, *population explosion*, *generalist dominance*, and *nutrient lockup* are not about whether a fixed point exists but about its *stability and multiplicity* — bifurcation and stability territory. (Nutrient lockup shares the nutrient-floor lineage of the energy-death gate but is realized dynamically, through decomposer reach and turnover rather than a static `N_total` inequality — see that gate below.) (Generalist dominance was once filed as a candidate existence gate; the analysis below shows its prevention rests on structural fragility and functional incompatibility — a mortality-and-morphology force, not a static income/cost inequality — so it belongs here.) They remain the domain of the search until the affirmative theory is built out.

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

The carcass-locked nutrient this static gate deliberately sets aside is exactly what the [nutrient lockup](expected-properties.md#nutrient-lockup) failure mode tracks dynamically: a world can clear `N_total` here yet still strand its nutrient in an unconsumed dead pool when decomposers are absent, out of reach, or non-viable, dragging the *available* pool below the floor over time. So nutrient lockup is a **dynamics** failure — turnover- and reach-dependent — not decidable from `N_total` alone, and it sits with the dynamics modes rather than this existence gate.

## Resolved finding — generalist dominance has no static gate (by design)

[Expected properties](expected-properties.md) once treated generalist dominance as prevented by superlinear *per-trait* maintenance ("a mathematical property of convex maintenance costs"). On analysis this does not hold: with *independent additive* convex costs, a generalist exploiting two niches at `(t, t)` pays `2·c·t^p` and earns roughly `2·R(t)` — exactly twice a specialist's *net* — and against a specialist at `2t` it even pays *less* (`2·c·t^p < 2^p·c·t^p` for `p > 1`). Per-trait convexity penalises concentration, not breadth; its real role is to make each trait self-limiting. (The implementation does apply this convex per-trait cost — `maintenance_cost_exponent`, default 2 — so the *form* is real and committed; it was the *attribution*, not the cost, that was wrong.)

The system design has since been corrected ([world rules](world-rules.md), cost structure; [expected properties](expected-properties.md), generalist dominance; [trait space](trait-space.md), "No L1 budget constraint"). The anti-generalist force is the conjunction of **structural fragility** (cost-structure trade-off #9 — a broad, high-entropy body crosses its peak-relative death threshold after a smaller structural loss, raising both predation and starvation mortality) and **functional incompatibility** (trade-off #2 — the substrate-anchored stillness autotrophy and nutrient uptake require is broken by mobility, so the broadest generalist cannot physically express its traits). Both are committed mechanisms; neither is a static income/cost inequality. Fragility acts through mortality and incompatibility through morphology — so generalist-dominance prevention is a **dynamics** property (stability of the differentiated state), not an existence gate decidable in closed form. By design there is no clean static gate; this failure mode sits with monoculture and frozen dynamics, beyond cheap a priori reach for now.

A static gate *would* become available if the design committed a **cross-trait interaction maintenance term** (a cost in the product of co-invested traits): a generalist would then be dominated whenever its interaction overhead plus its fragility-driven mortality exceeded its multi-niche income gain — a closed-form inequality. The design holds this term in reserve rather than committing it (it adds a knob and is unnecessary if fragility and incompatibility suffice), to be promoted only if genesis search finds generalists dominating where they should be confined — the same contingency the world rules already name for autotrophy–mobility. Until then the prevention is real but dynamic, and the matching gate stays out of reach.

This is recorded as the first product of the lens: an a priori check that caught a place where the stated mechanism did not produce the expected property, and drove a system-design correction rather than a viability one.

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

- **Promote committed forms into system design — done for trophic transfer.** The efficiency-vs-distance form (`base_trophic_efficiency · exp(−trophic_distance_decay · d)`) is now owned by [world rules](world-rules.md) (flow 7), not just parameterised in the scenario files. With the form pinned, the trophic-pyramid-height question moves from conditional to parametric: pyramid height is bounded by how many transfers can each retain enough of the level below to clear metabolism, and the per-transfer ceiling is now a closed form in `base_trophic_efficiency` and `trophic_distance_decay` rather than a free shape.
- **Anti-generalist mechanism — stated, gate deferred by design.** Prevention now rests on structural fragility (#9) and functional incompatibility (#2); a closed-form generalist-dominance gate becomes possible only if a cross-trait interaction maintenance term is later committed (see the resolved finding above). Tracked as a contingency, not current work.
- **Unify the canonical parameter/form model** across genesis, scenarios, and viability so the three lenses describe the same physics.
