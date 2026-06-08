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

A **second** reported per-seed distribution rides under the *same* boundary: the **coexistence
fraction** — the share of a cell's seed ensemble that lands in the coexisting regime (alive, and either
clustering or coexisting; the `||` is the #359 small-N disjunction so clustering's silent zero below
n≈4 does not under-count). Like the decomposer fraction it is recorded with the sample count and is
**never** a behaviour axis nor a fitness term; the monoculture↔coexistence *axis* is still the median
seed's `clustering_strength`. What the fraction adds is visibility into how a cell's ensemble splits
across the regime — the raw material the projection reads (below).

Because each cell's elite is selected on a noisy median-over-seeds, a lucky elite can misrepresent its
cell. The archive tolerates this descriptor noise by design (a soft per-cell acceptance threshold rather
than a single sticky occupant) rather than pretending each cell is a noise-free point. The coexistence
fraction makes that noise *visible*: a cell whose median seed coexists on a lucky 5-seed draw but whose
ensemble mostly monocultures reads a low fraction, and the projection (below) now reads it.

## Predicted bifurcation coordinates and the cross-check

The atlas's three behaviour axes are *observed* — read off a rollout's per-seed breakdown. Two of the
dynamics failure modes also admit a cheap *closed-form prediction* of which side of their bifurcation a
config sits on, lifted from the two validated research spikes ([#358 Hopf](../research/F-hopf-validation.md),
[#359 branching](../research/F-branching-validation.md)) into `crates/explorers-search/src/bifurcation.rs`:

- **`oscillation_distance` = `|λ| − 1`** of the living-mass↔available-pool 2×2 Jacobian (Brief F AC1's
  self-contained coupling) — `< 0` frozen, `> 0` limit cycle.
- **`branching_distance` = `D`**, the adaptive-dynamics invasion margin of a rare heterotroph into the
  founder monoculture — `> 0` coexistence, `< 0` monoculture.

Both are **descriptors, not objectives, and not binning axes.** This is the deliberate choice, and it is
the one the committed research permits. Both #358 and #359 returned a *Qualified GO* that explicitly
gates objective-promotion on work this design does **not** yet do: hardening the genesis observables
(#358 — `oscillation_strength` is flat and demographic-pulsing-dominated at search scale; #359 —
`clustering_strength` silently zeroes below n=4) and putting spatial in-reach geometry plus a wear
penalty into the objective's environment. So the readings enter as per-cell descriptors plus a
predicted-vs-observed cross-check, never summed into fitness — exactly as [viability](viability.md) keeps
`C*` a *characterisation* rather than a gate. **Objective-promotion remains gated** on that
observable-hardening (#358/#359); the cross-check's regime tag (below) makes the gate's status legible on
every sweep.

**Reduced coordinates for a single-founder config.** A QD config carries one founder *mean* trait vector
— no producer/consumer pair (that is emergent). So branching `D` is computed on the founder mean
directly (sweep a rare heterotroph against the founder-as-producer in the founder-monoculture
environment), and the Hopf reading uses the living-biomass↔available-pool coupling rather than the
prototype's producer↔consumer pair (which needs two clusters a config lacks, and whose observable #358
showed decouples at scale anyway). Both reuse the committed kernel (`trophic_transfer_efficiency`) and
`TraitVector::distance` verbatim.

**Conductance-aware under flow 9.** The [reserve mobilisation rate](world-rules.md) `f` (flow 9) sets
how fast above-buffer reserve is mobilised into the growth flow. Both readings are aware of it, and the
asymmetry between them is a direct consequence of reserve conservation. At any reserve fixed point the
mobilised flow per tick *equals* net income `I − b` (inflow must balance outflow when reserve returns to
the same value each tick), so `f` only sets where reserve sits (`R* = ρb + (I − b)/f`, larger as `f → 0`)
and how fast it relaxes (`~1/f`), **not** the steady-state throughput. The selection diagonals are
steady-state objects — invasion fitness and Jacobian eigenvalues are asymptotic, read after the reserve
transient — so the **branching margin `D` and the interior fixed-point location are f-invariant**: a
naïve scalar `f` on `net_energy` would be wrong, because the throughput the operator reads is net income
regardless of `f`. The **oscillation reading is not** f-invariant in the same way: the conductance rate
turns reserve into a genuine slow state on the consumer's income→structure pathway (the flow `f`
governs), so the Hopf coupling is a three-compartment available-pool↔reserve↔living-mass map whose third
eigenvalue (`≈ 1 − f`) and lagged structure-building shift the **Hopf boundary** with `f` even though the
fixed point does not move. The reduction is exact at `f = 1` (reserve is slaved within one tick and the
map collapses to the two-compartment pool↔living-mass form). The reserve buffer sits only on the consumer
growth pathway: the producer's photosynthetic income refills reserve every tick, so the pool-fill
diagonal is f-invisible (flow 9 names this producer/consumer asymmetry), and maintenance stays lumped on
the living-mass decay term rather than drawn from reserve, which is what keeps the `f = 1` reduction
exact. Because the oscillation observable is `WeakObservable` at current scale, the predicted-vs-observed
crosscheck across `f` is exercised for the branching axis (its realised steady-state rate is directly
measurable and reads f-invariant); for the oscillation axis the rollout crosscheck stays gated on the
hardened cycle-detector, and the descriptor is held to its analytic invariants (fixed-point
f-invariance, exact `f = 1` reduction, a monotone boundary shift in `f`).

**The cross-check, regime-tagged.** For every *live* config the predicted sign is compared against the
observed behaviour-axis boundary, and every disagreement is surfaced (`bifurcation_disagreements`),
never swallowed — the validation-triad cross-check both spikes prize. Each disagreement carries a
**regime**:

- `Validated` — the observable is trustworthy here, so the disagreement implicates F's spectral reading.
- `WeakObservable` — the observable is known-weak here, so the disagreement localises to the
  observable (or its geometry), not F. The **branching** axis is `WeakObservable` exactly on #359's
  small-N borderline (`clustering_strength == 0` while `coexistence_duration > 0`), else `Validated`
  (wear is off for every searched config, satisfying #359's other validated-regime condition). The
  **oscillation** axis is *constant* `WeakObservable` at current genesis scale — #358's verdict is that
  `oscillation_strength` cannot adjudicate the Hopf crossing — and flips to `Validated` only once a
  hardened cycle-detector lands (a separate issue).

**Authority boundary.** Like the three axes and `C*`, these readings arbitrate **existence/stability
only**. They never read the decomposer guild or any per-seed distributional property (the decomposer
fraction, the coexistence fraction), are never summed into fitness, and are never a binning axis — the
same existence-vs-distributional boundary the rest of the atlas respects.

## The recipe is a projection of the atlas

A single playable [world recipe](../../CONTEXT.md) is still drawn from the search, because the app needs
one world to drop the player into. It is the elite of the highest-fitness live cell **that clears the
coexistence-fraction floor** — most of its seed ensemble coexists (`COEXISTENCE_FLOOR = 0.5`,
operationalizing CONTEXT.md's bar *"accepted only when most runs in the ensemble produce sensible
worlds"*). When no live cell clears the floor the projection **falls back to plain argmax-fitness**, so a
live atlas always yields a world; the search warns when it had to.

The why is #401: the 5-seed median that ranks cells is high-variance near the monoculture↔coexistence
bifurcation, so a **straddler** — a cell that coexists on only a minority of initial conditions — can win
a lucky draw and top the leaderboard while its typical outcome is monoculture (the live #401 leader scored
fitness 0.67 yet re-evaluated to median 0 over an independent 8-seed ensemble, coexisting on ~3/8 ICs).
This is **selection only** — not binning, not fitness: the straddler is still a recorded cell with its
real fitness; only the *recipe pick* reads the floor, so the atlas map stays untouched and now visibly
honest. The atlas's honest stance is that *many* worlds across the manifold are viable, so any cell's
elite is reachable as a recipe, not only the projected one. "The best recipe" is one pick from a map, not
the search's output.

**Gated elite refinement hardens the pick (#404).** The floor above reads the *same* in-run 5-seed
ensemble that ranks the cell, and that estimate is itself high-variance near the bifurcation — so a lucky
5-seed draw can both top the leaderboard *and* clear the floor (the #401 leader read 0.60 = 3/5 in-run yet
re-evaluated to ~3/8 over an independent draw). Before projecting, the search therefore **re-evaluates the
top-K live cells** (K = `REFINE_TOP_K`, small) at a **larger, independent ensemble** (`REFINE_ENSEMBLE_SIZE`,
≫ 5) and applies the floor to that **refined** fraction. The refinement seeds are deterministic but drawn
far above any seed the search used (offset `2^40`), so the re-evaluation is an *independent* draw, not a
re-read of the in-run seeds, and a fixed `(atlas, seed)` refines bit-reproducibly. The pick is the
highest **recorded**-fitness top-K cell whose refined fraction clears the floor; it falls back to plain
argmax-fitness (with a warning) when none does. This stays inside the authority boundary: refinement
**never** rewrites the atlas map's binning or per-cell fitness — the recorded fitness remains the ranking
key, the refined fraction feeds only the pick, and the straddler stays a recorded cell. Its cost is
bounded (top-K only) and logged, including the lower-fitness live cells below the cut that were not
refined.
