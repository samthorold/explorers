# Genesis Search

The empirical lens on the world rules — the broad, expensive sweep of world-parameter space that
[viability](viability.md) calls *lens 3* of the validation triad. [Viability](viability.md) decides in
closed form which parameterizations *cannot* work; this is where the parameterizations that *might* work
are actually found, by running them.

This document fixes the **shape of what genesis search produces**, and why it has that shape. It does
not prescribe an optimiser — the search method is implementation and is free to change. What a
replacement implementation must preserve is the contract stated here.

## Genesis illuminates; it does not optimise

The genesis search returns an **[atlas](../../CONTEXT.md)** — a map from world-parameter space onto the
failure-mode coordinates — **not a single tuned [recipe](../../CONTEXT.md)**. A playable recipe is still
drawn *from* the atlas, but it is a projection of the map, never the map's purpose.

This is the load-bearing choice, and it follows from three properties of the objective, not from a
preference for one optimiser over another:

- **The space is high-dimensional.** The search varies ~30 world parameters and initial-distribution
  fields at once. A scalar surrogate optimiser (Gaussian-process Bayesian optimisation) is reliable only
  in low dimensions and degrades badly here — so badly that any such method needs a manual prefilter to
  fix the dimensions that "don't matter" before it can run at all. That prefilter is a global, one-shot
  guess that discards any dimension whose importance is *conditional* on another.
- **The objective is gated.** Most of parameter space is a flat zero-fitness desert: the
  [degenerate configurations](../../CONTEXT.md) collapse fitness to zero around a thin viable manifold.
  An optimiser climbing a scalar gets **no gradient across the desert** — which direction leads to life
  is invisible from fitness alone. This is the textbook deceptive-landscape failure.
- **What the designer wants is plural.** The goal is not *one* best world but a map of *which regions
  avoid which failure modes* — so the designer can see the structure of viability, and so a downstream
  surrogate can be trained on the whole manifold, not just its peak. A single maximised number cannot
  answer a plural question.

A **quality-diversity** search dissolves all three at once. It selects on *novelty in the failure-mode
coordinates*, not on fitness rank — so a config that scores zero but lands in an unexplored region is
*kept and bred from*, giving the search stepping-stones across the desert an optimiser cannot find. Its
native output *is* the atlas. And a covariance-adapting emitter learns the relevant subspace as it
moves, retiring the manual dimension-fixing prefilter the surrogate optimiser needed. The specific
emitter and archive are implementation; the illumination contract is the design.

## The three behaviour axes are the failure-mode coordinates

The atlas bins each surviving world on three **[behaviour axes](../../CONTEXT.md)**, each in `[0, 1]`.
They are not chosen by eye — each is the coordinate a [dynamics failure mode](viability.md) lives on, so
the atlas's cells correspond to distinct ecological regimes:

| Axis | What it measures | The mode it indexes |
|---|---|---|
| **oscillation strength** | endogenous population rhythm between trophic levels | **frozen dynamics** (low) ↔ healthy oscillation (mid) |
| **clustering strength** | multimodality of the trait-distance distribution (the dip statistic) | **monoculture** (low) ↔ **coexistence** (high) |
| **carcass-locked fraction** | the dead pool's share of conserved nutrient (the trailing-window mean the [lockup gate](viability.md) reads) | healthy throughput (low) ↔ **nutrient lockup** (high) |

All three are observables the **evaluator already computes** while scoring a run — they are read off the
evaluator's output, never re-derived by the search. The third axis is deliberately the carcass-locked
fraction and **not** trophic balance: trophic balance is decomposer-blind (it reads the
producer-vs-consumer energy share and cannot see the dead-vs-living distinction), so it would put a
healthy world and an about-to-lock-up world in the *same* cell. The carcass-locked fraction is the cheap
observable of the [flux-balance order parameter `C*`](viability.md) the lockup cliff actually lives on.

## The dead frontier is the atlas's most valuable layer

A world that hits a terminal gate has no meaningful behaviour coordinate (a world extinct by tick three
has no oscillation), so it gets **no cell**. Instead it is tallied to the **[dead frontier](../../CONTEXT.md)**,
keyed by which failure mode it hit. The frontier is *where parameter space stops being survivable, and
which cliff it hits when it does* — the structure the designer most wants to see, and the negative-space
sketch the [viability](viability.md) gates aim to predict in closed form.

The frontier holds **two kinds of entry**, and the distinction is load-bearing:

- **A priori deaths** — configs the [viability](viability.md) prefilter gates *before any rollout*. The
  two committed closed-form gates (extinction's flux floor `F ≤ B`; energy death's nutrient floor on
  `N_total`) run first and route provably-dead configs straight to the frontier, spending no ensemble on
  them. This is the lens-1 → lens-3 interlock viability already names as its payoff: it shrinks the
  search box so budget concentrates on the survivable region.
- **Observed deaths** — configs a rollout actually carried into a cliff.

Because the prefilter and the rollout both verdict the *same* config, they **must agree**: a config
viability calls extinct must, when run, be extinct. The search keeps a sampling cross-check that
prefiltered-dead configs would also die if simulated — so every genesis sweep is a continuous
falsification test of the two gates, exactly the cross-validation the [validation triad](viability.md)
turns on. Disagreement is diagnostic, not a nuisance: it localises a mis-drawn gate.

The **nutrient-lockup cliff cannot be prefiltered** — viability shows it has no cheap a priori gate (it
is turnover- and decomposer-mass-dependent, emergent per seed). So the lockup layer of the frontier is
populated only by *running* worlds that survive yet strand their nutrient. Reaching that thin
high-carcass region is the emitter's job (directed exploration along the carcass axis), not the
prefilter's. The atlas maps the lockup cliff by running, because the physics forbids gating it.

## Authority boundary: the decomposer guild is reported, never optimised

A **decomposer** is a behavioural role read from an agent's trait vector and diet, confirmed across seed
ensembles but **sporadic per seed** — a persistent guild forms in only a fraction of surviving runs
([expected properties](expected-properties.md); CONTEXT.md, *Decomposition*). The atlas therefore records
it as a **per-cell distribution** — the fraction of a cell's seed ensemble that sprouts a persistent
guild, with the sample count — and **never** as a behaviour axis or a fitness term.

This is the same existence-vs-distributional boundary [viability](viability.md) already respects when it
makes `C*` a *characterisation* rather than a gate: the atlas maps the existence/stability skeleton of
parameter space (the three axes are all existence/stability quantities), and it may not collapse a
distributional emergence into a coordinate, because a behaviour axis is a *point* per config while the
guild's truth is a *fraction of seeds*. The boundary is enforced mechanically: the guild signal rides on
the evaluator's output as a reported observable, alongside the other non-fitness readings, and is never
summed into fitness nor binned on.

Because each cell's elite is selected on a noisy median-over-seeds, a lucky elite can misrepresent its
cell. The archive tolerates this descriptor noise by design (a soft per-cell acceptance threshold rather
than a single sticky occupant) rather than pretending each cell is a noise-free point.

## The recipe is a projection of the atlas

A single playable [world recipe](../../CONTEXT.md) is still drawn from the search, because the app needs
one world to drop the player into. It is the elite of the highest-fitness live cell — the argmax
projection of the atlas. But the atlas's honest stance is that *many* worlds across the manifold are
viable, so any cell's elite is reachable as a recipe, not only the single best. "The best recipe" is one
pick from a map, not the search's output.
