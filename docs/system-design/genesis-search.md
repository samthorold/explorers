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

## The atlas maps the settled community, not the founder bloom

Every rollout the search scores starts from a founder cohort and passes through a **pioneer bloom** —
the producers colonise the empty world, peak, and then fall back as their own mortality feeds the
carcass flux that the heterotroph niche lives on ([disturbance and succession](../ecology/disturbance-and-succession.md)).
The heterotroph guilds are a **later successional stage**: on the strongest known guild configs the
producers fall 2–3× and the decomposers rise 2–18× *after* the bloom peaks, and a guild that reads on
7 / 8 seeds over the settled phase reads on 0 / 8 at peak bloom
([443 §6.2](../research/443-invasion-growth.md)).

The atlas characterises the **settled community** — the post-bloom stage the player is dropped into —
not the bloom. The reason is what the axes are for: they are existence/stability coordinates, and the
existence or stability of a colonisation transient is not the thing the design maps; the failure modes
the frontier tallies (frozen, monoculture, lockup) are properties of where the ecology *settles*, and
a bloom-stage read cannot tell "the search selects against guilds" from "the guild forms after the
search stops looking". A bloom-stage map is therefore a map of the wrong object, however cheap. The
rollout horizon is consequently a **design quantity set by the ecology's own settling time**, not a
search tuning knob.

**One rollout, one settled window.** Each rollout runs to the horizon `T`; the terminal gates
(extinction, energy death, lockup) fire whenever they fire along the way, so the dead frontier is the
frontier of the *whole* trajectory — a world that blooms and then locks up is on the lockup cliff,
which a bloom-length rollout could not see. Every behaviour axis and every reported distribution (the
guild fractions, the coexistence fraction) is read off the **settled window `(T/2, T]`** — the same
window the guild predicate is defined on — not off the bloom or the whole run. The window changes
*when* each axis is read, not *what* it reads: `clustering_strength` stays the existence read on the
final snapshot (now a settled tick; the existence-vs-persistence pair with `coexistence_duration` in
[expected properties](expected-properties.md) is deliberate and a window-typical dip would blur it);
`oscillation_strength` reads the window's share series (a thousand settled ticks instead of a few
hundred of bloom — the demographic-pulsing contamination #358 found at the short horizon is a
bloom-stage artefact); the carcass-locked fraction stays the trailing mean the lockup gate reads, so
axis and gate cannot diverge — if the dead pool has not settled by `T − window` the horizon is too
short, not the read. The two halves are not separable: a bloom-length rollout with a "longer guild-only read" bolted on still has to run
the world to `T` to see the guild, saves only the per-tick observation (which is not where the cost
is), and leaves the axes reading the bloom while the guild reads the settled stage — a map whose
coordinates and whose reported distributions describe different objects.

**The frontier costs a bloom, the atlas costs the horizon.** A rollout that hits a terminal gate is
tallied and stopped where it dies; only a world still alive past the bloom is carried to `T`. Nothing
is *scored* before `T` — an early stop is a frontier entry, never a cell — so the map is the same map
as if every rollout ran the full horizon; the search simply does not pay a settled-community price
for a world that has no settled community. This is the tick-0 prefilter's interlock moved along the
trajectory, and it is why the longer horizon does not force the map to get coarser: cutting
generations or batch to hold wall-clock trades the atlas's resolution for its correctness, and
shrinking the seed ensemble makes the median-over-seeds selection signal noisier exactly where
[the projection](#the-recipe-is-a-projection-of-the-atlas) already struggles. Neither is taken.

The two dead-pool gates are read *incrementally* for this: on the series-so-far, exactly the
horizon definitions with the reference starting at the grace, at the lockup-window cadence, so a
world that collapses at tick `t` and stays collapsed stops at the first window boundary past
`t + window` (`early_stop` in the evaluator; extinction and explosion read the count every tick).
But the gates as defined are **not proven irreversible** — a world flagged at tick 600 might
recover by `T` — so the stop carries the same falsification interlock the prefilter has: a
configurable fraction of stopped rollouts (`early_stop_crosscheck_fraction`, default 5 %, drawn
deterministically from the rollout seed) is carried to `T` anyway and re-verdicted on the full
series, and every stopped-dead / alive-at-`T` disagreement is surfaced beside the prefilter and
bifurcation disagreements (`early_stop_disagreements`), never swallowed. The carry changes only
what is *recorded*: a carried rollout's own verdict stays the gate's, so the atlas is the same map
at any carry fraction. A non-empty list localises a gate firing on a reversible collapse — the
evidence for tightening the gate's definition, not for lengthening the grace.

**The gates' reference excludes the founder transient, by a measured tick count.** Lockup is read as
a trailing window against the trajectory's earlier history (the low the dead pool once reached);
energy death is read against a history-free reference, the config's sustainable stock ([expected
properties](expected-properties.md#energy-death)), but its trailing window is likewise taken from the
grace on. Tick 0 is not part of either read: the founder
cohort is *provisioned*, so its stock is an artefact of the founder budget rather than anything the
ecology produced, and for the first few tens of ticks — until photosynthetic income overtakes the
provisioning — the series describes the budget, not the world. The reference therefore starts after a
**grace** of fixed tick count, sized from the *provisioning transient* — the tick at which the living
stock first exceeds its tick-0 value, measured across the atlas's live cells and a low-discrepancy
sample of the search box, taken at an upper quantile — and recorded here beside the measurement that
produced it. It is an ecological constant, not a fraction of the horizon: a longer rollout does not
make the founder budget last longer, and a grace scaled to the horizon would blind the gates to a
world that genuinely dies early. It is also not per-run: a world that never recovers its founder stock
would then never acquire a reference and would pass as live. When the stepper changes materially the
transient is re-measured, the same way [viability](viability.md)'s tightness claims are re-checked
against the atlas and the search box.

**The horizon is the settling time, measured the same way.** `T` is set so that the settled window
`(T/2, T]` opens *after* succession has run: the measurement is, per run, the tick at which the
producer count last leaves the band it holds over the run's final stretch (succession is the producer
ceding stock to the heterotroph niche, so the producer series is where it reads), taken at an upper
quantile across the atlas's live cells and a low-discrepancy sample of the search box, with `T/2`
placed above it. The evidence that it is a succession timescale and not a threshold is
[443 §6.2–6.3](../research/443-invasion-growth.md): at 1000 ticks decomposers are still rising
2–18× on the strongest guild configs, at 2000 the control arm only drifts, and the guild's own
doubling time where it lives is ~700 ticks. A round number chosen for convenience would carry no
procedure to re-run when the stepper changes; this one is re-measured with the grace, off the same
long rollouts, since both are transient ticks of the same trajectory.

*Current values — measured by `crates/explorers-search/src/bin/settling_time.rs` (one resumable
JSON-lines row per config; the summary restates the procedure beside the quantiles) over the 82 atlas
live cells and the 200-point LHS sample of the search box, 8 seeds each, at a 3000-tick horizon: 2256
runs, 1480 live (mode `none`, reached the horizon), the rest tallied by mode
([505](../research/505-settling-time.md)). Both quantiles are nearest-rank over the live runs, atlas
and sample pooled.* **Grace = 260 ticks** (`EvalConfig::grace_ticks`): the 90th percentile of the
provisioning-transient tick over the 1357 live runs whose stock re-exceeds the founder budget, rounded
up to 10 — the distribution is 50 / 90 / 95 / max = **2 / 253 / 514 / 2586** pooled (atlas, n = 401:
1 / 198 / 419 / 1885; sample, n = 956: 3 / 296 / 551 / 2586); 123 live runs (8 %) never re-exceed
tick 0, the case that rules out a per-run grace. **Horizon: `T = 2000` stands as the working value;
the settling read did not measure it.** The producer-settling tick (last tick outside ±20 % of the
final-500-tick mean) reads 50 / 90 / 95 / max = **1677 / 2981 / 3000 / 3000** pooled over the 1407
live runs with producers in the tail (atlas, n = 396: 2021 / 2989 / 3000 / 3000; sample, n = 1011:
1527 / 2973 / 3000 / 3000), so the rule *smallest multiple of 500 with `T/2` above the 90th
percentile* returns 6000 — but the statistic is saturated, not the ecology unsettled: 29 % of live
runs leave the band *inside* the 500-tick tail the band is defined from (the median settled producer
count is 3, so ±20 % is narrower than one agent and every birth or death breaks it), the upper
quantiles sit at the horizon on every sub-population (tail count ≥ 20: 90th percentile 3000), and
the reading would move with any horizon it was run at. `SearchConfig::max_ticks` is therefore not
moved on it; the band statistic is re-designed (a band in absolute agents, or a smoothed series)
before `T` is read again, and #492 / 443 §6.3 remain the evidence the working value rests on.*

## Authority boundary: the heterotroph guilds are reported, never optimised

A **decomposer** is a behavioural role read from an agent's trait vector and diet, confirmed across seed
ensembles but **sporadic per seed** — a persistent guild forms in only a fraction of surviving runs
([expected properties](expected-properties.md); CONTEXT.md, *Decomposition*). The atlas therefore records
it as a **per-cell distribution** — the fraction of a cell's seed ensemble that holds a
**[heterotroph guild](../../CONTEXT.md)** of that role, with the sample count — and **never** as a
behaviour axis or a fitness term. The same read is reported for the **consumer** role
(`consumer_fraction` beside `decomposer_fraction`).

A guild is a *population*, not a role tag on one agent (#490; the #443 re-read found the atlas's resident
"guild" to be a single sessile mixotroph on most seeds, a quarter of them sterile at the `kappa` clamp).
Membership is the evaluator's existing `trophic_roles` read — heterotroph by trait, consumer/decomposer
by realised diet — taken on the rollout's roster snapshots (the `coexistence_sample_interval` cadence)
over the **second half** of the run. The guild holds when the role's count reaches `GUILD_MIN_SIZE = 5`
on **every** such sample *and* at least one `Born` event in that window names a member as a parent —
sustained size rules out the lone long-lived individual, recruitment rules out a sterile founder cohort
sitting at exactly the floor. Both values are starting points, not settled thresholds: regenerating the
atlas with the read reported, and counting the cells that pass, is what tests them and decides whether
the observable ever becomes a binning axis or folds into `coexistence_fraction`. Until then it is
reported only.

This is the same existence-vs-distributional boundary [viability](viability.md) already respects when it
makes `C*` a *characterisation* rather than a gate: the atlas maps the existence/stability skeleton of
parameter space (the three axes are all existence/stability quantities), and it may not collapse a
distributional emergence into a coordinate, because a behaviour axis is a *point* per config while the
guild's truth is a *fraction of seeds*. The boundary is enforced mechanically: the guild signal rides on
the evaluator's output as a reported observable, alongside the other non-fitness readings, and is never
summed into fitness nor binned on.

The same boundary decides what a **terminal gate** zeroes. A gated world has no meaningful behaviour
*coordinate* (above), so every scored descriptor is zeroed and the world gets no cell — but an
observable is not a coordinate. A world gated on its **terminal** read ran the whole horizon, so its
settled window `(T/2, T]` was classified like any other and the guild predicate is as computable, and
as true, there as on a live world; zeroing it was a measurement artefact that hid real guilds inside
monoculture and lockup cells (#527, measured in
[519-guild-read-at-settled-horizon.md](../research/519-guild-read-at-settled-horizon.md)). So the guild
flags are carried through the horizon gate and the scored descriptors are not. A rollout stopped
**before** the horizon is the other case: it has no settled window to read, so its flags report `false`
and "no guild" is indistinguishable from "not read" — accepted rather than made tri-state.

A **further** reported per-seed distribution rides under the *same* boundary: the **coexistence
fraction** — the share of a cell's seed ensemble that lands in the coexisting regime (alive, and either
clustering or coexisting; the `||` is the #359 small-N disjunction so clustering's silent zero below
n≈4 does not under-count). Like the decomposer fraction it is recorded with the sample count and is
**never** a behaviour axis nor a fitness term; the monoculture↔coexistence *axis* is still the median
seed's `clustering_strength`. Note that a coexisting seed need not hold a guild: `coexistence_fraction`
reads separated trait clusters, which one heterotrophy-dominant individual already supplies — the guild
fractions are what say whether a second trophic *level* is present. What the fraction adds is visibility into how a cell's ensemble splits
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
only**. They never read the heterotroph guilds or any per-seed distributional property (the decomposer and
consumer fractions, the coexistence fraction), are never summed into fitness, and are never a binning axis — the
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
refined. The refinement size is a *separator*, not an estimator: at n = 32 the floor rule tells a
straddler at p ≈ 0.35 from a robust cell at p ≈ 0.65 with ≈ 5 % error either way, but its two-sided
interval at 16/32 is still [0.32, 0.68], and a sequential (SPRT) alternative was evaluated and not
adopted — the arithmetic is in [`docs/research/434-ensemble-confidence.md`](../research/434-ensemble-confidence.md).
