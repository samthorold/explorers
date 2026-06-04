# Research Brief B — ES / Quality-Diversity outer search (atlas of viable worlds)

**Status: research finding. Commits nothing.** This is the Cluster-B deep-dive for epic
[#345](https://github.com/) (issue [#347](https://github.com/)). It is a recommendation, not an
implementation, and deliberately lives outside `docs/system-design/` — the system-design layer is
self-justifying and commits the design; this layer records investigation that *informs* a future
commitment. It proposes a *search method* for the outer loop (`explorers-search`), changing nothing
about the physics, the trait space, or the failure-mode definitions. It reasons from a read of the
current search (`crates/explorers-search/`), the genesis evaluator
(`crates/explorers-genesis-eval/`), and the two sibling briefs that have already reported — A
([`A-vectorised-stepper.md`](A-vectorised-stepper.md)) and F
([`F-mean-field-operator.md`](F-mean-field-operator.md)).

This brief inherits the epic's framing: two nested loops (the inner Darwinian rollout vs. the outer
parameter hunt — this brief is the *outer*), the cluster interlock (F sharpens the objective → A
makes evaluating it cheap → **B maps where it holds** → E amortises it), and the three-lens
validation triad (this brief deepens **lens 3, genesis search**).

## TL;DR

1. **The BO→ES economics flip is real in *direction* but modest in *magnitude*, and — crucially —
   the strongest case for the switch is not the throughput change at all. It is the *objective
   structure*.** Brief A's outcome was narrower than this brief's parent presumed: the GPU/JAX batch
   regime (10⁴–10⁶ evals/generation) was ruled **NO-GO**, and the throughput win that shipped is
   rayon over (config × seed) — **~cores× (10–50×), each evaluation still seconds**, not the
   accelerator regime QDax/EvoJAX assume. That is enough to *feed a population-based method* (which
   only needs a batch per generation, and that batch is exactly the rayon-parallel unit #353 already
   delivers) but **not** enough to make naive large-batch ES dominate the incumbent on raw sample
   count. The decisive arguments are instead: (a) the search is **31-dimensional**, right at the
   ceiling where GP-BO is known to weaken — and the code already concedes this, fixing
   low-sensitivity dims via Sobol indices before the BO step; (b) the objective is **gated, plural,
   and deceptive** (six hard zero-fitness cliffs around a thin viable manifold), which violates the
   smooth-scalar assumption GP-BO and scalarised ES are both built on. → **AC1.**

2. **Quality-Diversity (MAP-Elites) is the right tool, and F supplies its behaviour axes.** The
   parent brief guessed *"oscillation amplitude × cluster count × trophic balance."* F's central
   finding — *which reduced coordinate each failure mode lives on* — tells us *why* exactly three
   axes suffice and *which three*: the dynamics failure modes partition onto three independent
   operator objects, so the natural QD descriptor space is **(i) oscillation strength** (the
   frozen↔oscillation Hopf axis), **(ii) clustering strength** (the monoculture↔coexistence
   branching axis), **(iii) carcass-locked nutrient fraction** (the energy-death/nutrient-lockup flux
   axis). All three are *already computed cheaply per rollout* by the evaluator. Axis (iii) is a
   concrete improvement F enables over the parent's "trophic balance": `trophic_balance_score` is
   **decomposer-blind by design**, so it cannot index the very flux cliff F shows lives on the
   carcass compartment. → **AC2.**

3. **The gated objective is what *kills* scalarised ES and what *makes* QD — and the distributional
   modes impose the same authority boundary on B that they impose on F.** Six terminal cliffs collapse
   fitness to a flat zero across most of the 31-cube; a scalarised ES climbing that surface gets *no
   gradient* across the dead desert (the deceptive-landscape failure). QD's behaviour descriptors
   supply *secondary* signal where fitness is flat — illumination finds stepping stones an optimiser
   cannot. **But** the sporadic-per-seed decomposer guild (forms in ~1/5 of surviving seeds at the
   decoder midpoint; `decomposer_emergence.rs`) is a *distributional* property: it must be a per-cell
   *reported distribution*, never a fitness target or a sharp descriptor. This is the B-level mirror
   of F's authority boundary — QD maps the *existence/stability* skeleton of parameter space; it must
   not collapse a distributional emergence into a coordinate. → **AC3.**

4. **Recommendation: replace LHS+GP-BO with a CMA-MAE archive (CMA-ME family), not pure scalarised ES
   and not classical multi-objective.** CMA-ME because the search is 31-dim continuous and
   ill-conditioned (the Sobol prefilter is evidence of that), and CMA-ES's covariance adaptation
   handles the conditioning *automatically* — subsuming the manual "fix low-sensitivity dims" hack
   into the emitter. QD (not Pareto/NSGA) because user-story 4 wants an **atlas of which regions
   avoid which failure modes** — a map from parameter space onto behaviour, which is QD's native
   output, not a trade-off front. It **composes with A** precisely because QD needs only a
   batch-per-generation, which is the (config × seed) rayon unit #353 ships — so it runs *now*,
   without the GPU regime A rejected — and it **composes with F** (F's coordinates are the axes; F's
   a-priori viability gates can prefilter dead configs before they cost a rollout) and **with E** (the
   QD archive's diverse coverage is the training set brief E's surrogate needs). → **AC4.**

5. **Next move: a thin MAP-Elites spike, measured against the incumbent at equal rollout budget.**
   Reuse `decode()` and `run_ensemble()` unchanged; archive over the three F-derived axes; cell
   fitness = the existing 5-component score; route the six cliffs to an out-of-archive "dead" tally
   that *is itself the atlas*. Validate by reproducing known good/bad configs and checking the
   archive's monoculture/lockup boundaries against the evaluator's gates. Go/no-go gate: does QD beat
   LHS+BO on best-fitness-at-equal-budget **and** produce a usable viability atlas? → **AC5.**

## The baseline this brief reasons from (what the search actually is today)

All citations are against `crates/explorers-search/` and `crates/explorers-genesis-eval/` at the time
of writing.

- **The search is a two-stage LHS → GP-BO pipeline** (`search.rs`, `run_search`). Stage 1: Latin
  Hypercube sampling (default 50 samples) over a **31-dimensional unit hypercube**, each sample
  decoded by `decode()` to a fully-specified `(WorldParameters, InitialDistribution)` and scored by
  `explorers_genesis::run_ensemble`. Stage 2: a Gaussian-Process surrogate (RBF kernel, `gp.rs`) with
  Expected-Improvement acquisition (`bayesopt.rs`, ~200-candidate random search per step, default 10
  iterations). Between the stages, **Sobol sensitivity indices fix the low-sensitivity dimensions**
  (threshold 0.05) so BO only varies the dims that move the objective — an explicit concession that
  31-dim BO is strained.
- **The parameter space is 31 free dimensions** (`default_ranges()`), spanning world physics
  (solar flux, trophic efficiency/decay, metabolic and maintenance costs, mutation rate/magnitude,
  world extent, …) and the founding `InitialDistribution` (trait means, covariance, cluster count,
  per-agent energy). Eight further fields are inherited fixed from `viable_baseline()`.
- **Evaluation is a seed ensemble reduced to a median.** `run_ensemble` runs `ensemble_size` (default
  5) rollouts on independent RNG streams (`base_seed.wrapping_add(i)`), each to `max_ticks` (default
  500) or extinction, and returns the **median fitness** across seeds. Both the ensemble loop and the
  LHS sweep are **rayon-parallel** (#350, #353), with a bit-identity test against a sequential
  reference (`parallel_ensemble_matches_sequential_reference`).
- **The objective is six hard gates wrapped around a five-component scalar** (`explorers-genesis-eval`,
  `evaluate_from_log`). The gates, each forcing **fitness = 0.0**: *extinction* (no agents),
  *population explosion* (`> max_population`, 10 000), *energy death* (free-energy collapse to <10% of
  peak over a trailing window), *nutrient lockup* (carcass-locked fraction ≥40% and not receding),
  *monoculture* (`clustering_strength < 0.5` with ≥20 agents post-grace), *generalist dominance*
  (≥50% of energy in photo-and-hetero clusters). If all gates pass:
  `fitness = 0.2·oscillation_strength + 0.2·clustering_strength + 0.2·coexistence_duration +
  0.2·turnover_score + 0.2·trophic_balance_score`. A grace period (20% of `max_ticks`) suppresses
  early transients.
- **The decomposer guild is a sporadic-per-seed distributional property**, asserted by
  `decomposer_emergence.rs` (`slow_persistent_decomposer_guild_forms_across_ensemble`): on the decoder
  midpoint, ~48/50 seeds survive, a decomposer role appears in ~1/3 of survivors, and a *persistent*
  guild forms in ~1/5 (≥4 of the surviving seeds is the assertion floor; ~10/48 observed). This is the
  property `expected-properties.md` calls *"distributional: confirmed across seed ensembles, sporadic
  per-seed, never guaranteed on a single run."*

Two structural facts fall straight out of this baseline and drive the whole brief: the search is
**31-dim** (GP-BO's weak regime) and the objective is **six terminal cliffs around a thin manifold**
(no method built on a smooth scalar is well-matched).

## AC1 — The BO→ES economics flip, conditioned on brief A's outcome

**The parent brief's premise was conditional — *"if* brief A makes evaluations cheap and massively
parallel, the economics flip toward ES." A has now reported, and the condition is only *partly*
met.** A's findings, precisely:

- The **free, fork-less throughput win shipped**: rayon over (config × seed) (#353) — **~cores×
  (10–50×), bit-identical, zero design delta.**
- The **inner-stepper GPU/JAX rewrite — the only path to the 10⁴–10⁶-evals-per-generation accelerator
  regime — was ruled NO-GO** (spike #354): the single-rollout latency floor lives in the irregular
  spatial-query/reduction phases, not the elementwise phases a cheap bit-identical vectorisation could
  touch; the design-delta commitment that rewrite needed (#351) was closed NOT_PLANNED.

So the *realised* throughput regime is **~cores× speedup, each evaluation still ~seconds** (500 ticks
× 5 seeds). This matters for the flip in two opposite directions:

**Why the throughput change *alone* does not justify pure large-batch ES.** Accelerator-native ES
(OpenAI-ES, popsize 10³–10⁴; the EvoJAX/QDax economics) earns its keep by trading sample-efficiency
for embarrassingly-parallel throughput — sensible when a generation of 10⁴ evals costs the same
wall-clock as one. At ~cores× with seconds-per-eval, a "generation" affordably holds ~10²–10³ evals,
not 10⁴–10⁶. In that regime GP-BO's sample-efficiency is *not* obviously dominated: the incumbent
finds its optimum in 50 + 10·(BO budget) ≈ a few hundred evals. **A pure-ES argument that rests only
on "evals are now cheap" is therefore weak — A did not deliver the regime that argument needs.**

**Why the flip is nonetheless correct — for reasons of *objective structure*, not throughput.** Two
properties of the *problem* (independent of A) make the incumbent a poor fit and a population-based
illuminator a good one:

1. **Dimensionality.** GP-BO is reliable to ~20 dims and degrades past ~50 (cubic-in-observations GP
   cost; the curse of dimensionality in the acquisition surface). The search is **31-dim** — squarely
   in the degraded band. The code's own Sobol-prefilter-then-fix-dims step is a workaround for exactly
   this weakness, and a brittle one: it fixes dims *globally* on a single midpoint sensitivity
   estimate, discarding any dimension whose importance is *conditional* on others (an interaction the
   one-shot Sobol index cannot see). A covariance-adapting evolution strategy (CMA-ES) handles
   anisotropic, correlated 31-dim landscapes *natively* — it learns the relevant subspace as a
   rotation of its sampling covariance, replacing the manual prefilter with something that adapts as
   the search moves.

2. **The objective is gated, plural, and deceptive** (developed in AC3). GP-BO models a smooth scalar;
   ours is a scalar with six discontinuous zero-cliffs around a thin viable manifold, *and* the thing
   we actually want is plural ("avoid *various* failure modes," user-story 4), not a single maximised
   number. Both GP-BO and scalarised ES inherit the smooth-single-objective assumption. QD does not.

**Conclusion (AC1).** The flip is justified — but the load-bearing reason is the **31-dim, gated,
plural objective**, with A's ~cores× as the *enabling* (not the *deciding*) factor: it is exactly
enough to feed a population-based method (a batch per generation = the rayon unit #353 ships) and not
more. The honest framing for the follow-up issue: *"we are not switching to ES because evals got
cheap (they got cheaper, modestly); we are switching to QD because the objective is plural and gated
and the space is 31-dim, and A's throughput is sufficient to feed QD without the GPU regime A ruled
out."*

## AC2 — QD behaviour-space axes (F supplies them)

A MAP-Elites archive is defined by its **behaviour descriptors** — low-dimensional features computed
per evaluation that the archive bins on, keeping the best-fitness elite per cell. Good descriptors
must be (a) **cheap** (computed every rollout), (b) **defined even on failures** (QD maps the whole
space, including the dead regions), and (c) **separating** of the failure modes (the atlas is only
useful if its cells correspond to distinct ecological regimes).

The parent brief proposed, by eye, *"oscillation amplitude × cluster count × trophic balance."* **F's
central finding makes this principled and corrects the third axis.** F shows the dynamics failure
modes partition onto exactly three *independent operator objects*: a scalar/low-block **flux
balance** (extinction, explosion, energy death, nutrient lockup), a low-block **Jacobian spectrum**
(frozen↔oscillation Hopf), and the **specification-subspace shape** (monoculture↔coexistence
branching; generalist dominance). That partition is *why three axes suffice and which three they are*:

| QD axis | Cheap descriptor (computed today) | F's operator object it indexes | Failure modes it separates |
|---|---|---|---|
| **1 — oscillation strength** | `oscillation_strength` (lineage autocorrelation), already in `FitnessBreakdown` | leading eigenvalue *pair* of the 2-block Jacobian (Hopf crossing) | **frozen** (low) ↔ healthy **oscillation** (mid) |
| **2 — clustering strength** | `clustering_strength` (dip statistic on trait-distance), already computed | single- vs multi-peak stationary `n(θ)`; invasion-fitness sign on the specification subspace | **monoculture** (low) ↔ **coexistence** (high); **generalist dominance** reads as a distinct low-clustering signature |
| **3 — carcass-locked nutrient fraction** | the carcass-locked fraction the **nutrient-lockup gate already computes** (`LOCK_FRACTION`) | the three-compartment flux balance `A⇌L⇌C`; stationary `C* = death_flux/decomp_turnover` (F, AC4) | **nutrient lockup / energy death** (high `C`) ↔ healthy throughput (low `C`) |

**Why axis 3 is `carcass-locked fraction`, not `trophic_balance` (the brief's improvement over the
parent).** The evaluator comment is explicit that `trophic_balance_score` is **decomposer-blind** — it
reads the producer-vs-consumer *energy* share and cannot see the dead-vs-living distinction, because
decomposer-ness is a behavioural role, not a heritable trait to score. But the energy-death/lockup
cliff *lives on exactly that dead pool* (F, AC4: the `n→c→A` chain). So `trophic_balance` would put the
healthy-throughput worlds and the about-to-lock-up worlds in the *same* cell — it cannot separate the
mode it is meant to map. The carcass-locked fraction is the descriptor that *does* index that cliff,
it is *already computed* by the lockup gate, and it is the cheap observable of F's `C*` order
parameter. This is a clean instance of F sharpening the objective for B to map.

**On dimensionality of the archive.** Three axes at a modest resolution (e.g. 20 × 20 × 20 = 8 000
cells) is well within reach at ~cores× throughput — MAP-Elites fills an archive incrementally over
many generations, not in one batch, so 8 000 cells does not mean 8 000 simultaneous evals. If three
proves too coarse, the natural refinement is *not* more axes (which dilutes coverage exponentially)
but a CMA-MAE *soft* archive that tolerates finer binning by smoothing the per-cell threshold.

**A-priori-defined-on-failure note.** Axes 1 and 2 degenerate on terminal failures (a world extinct
by tick 3 has no meaningful oscillation or clustering). The design consequence is in AC3: terminal
cliffs do not get a behaviour cell at all — they are tallied to the atlas's "dead" boundary, which is
*itself a deliverable* (which parameter regions die, and how).

## AC3 — How the gated / distributional objective interacts with ES/QD selection

There are **two distinct interactions**, and conflating them is the trap.

### (a) The *gated* structure — six terminal cliffs — is what defeats scalarised ES and favours QD

Four of the six gates (*extinction, explosion, energy death, nutrient lockup*) are **terminal cliffs**:
a config that hits them scores a flat **0.0** regardless of *how badly* it failed. Across the 31-cube,
the viable manifold is thin and most of the volume is this zero-fitness desert. For a **scalarised ES**
(or GP-BO), that desert is **deceptive**: a population of candidates that all score 0.0 yields *no
selection gradient* — the strategy has nothing to climb, and which direction leads toward the viable
manifold is invisible from fitness alone. This is the textbook failure mode ES/CMA-ES share with
gradient methods on needle-in-haystack objectives.

**QD dissolves exactly this.** MAP-Elites selects not on fitness-rank but on *novelty in behaviour
space*: a config that scores 0.0 but lands in a previously-empty behaviour cell is *kept* and *bred
from*. The behaviour descriptors supply **secondary signal where fitness is flat**, so QD finds
stepping-stones across the dead desert that an optimiser cannot. The two *soft* gates (*monoculture,
generalist dominance*) reinforce this: they are not "the world is broken" so much as "the world is
alive but undiverse" — and those are *precisely the low-clustering cells of axis 2*. QD maps them as a
*region of the atlas* rather than culling them blindly; the boundary between the monoculture region and
the coexistence region *is* the user-story-4 deliverable.

**Design consequence for the archive's valid region.** The terminal cliffs should be handled as an
*out-of-bounds* tally, not as fitness-0 elites competing for behaviour cells (a fitness-0 extinct
config has degenerate descriptors and would pollute cells it does not belong in). The archive maps the
*surviving* region; the cliffs are recorded as the *frontier* of that region — "here is where
parameter space stops being survivable, and which cliff it hits when it does." That frontier is the
atlas's most valuable layer for the system designer.

### (b) The *distributional* structure — sporadic-per-seed — imposes F's authority boundary on B too

The objective is a **median over 5 seeds**, and at least one expected property — the decomposer guild —
is *sporadic per seed* (forms in ~1/5 of survivors; `decomposer_emergence.rs`). This interacts with QD
selection in a way that mirrors F's authority boundary one level up:

- **Median-over-5 is a noisy fitness *and* a noisy descriptor.** QD archives are known to be vulnerable
  to descriptor noise: a config can land in a cell by *luck* of its 5 seeds and, because elites are
  sticky (they are only displaced by a higher-fitness occupant of the *same* cell), a lucky elite
  persists and misrepresents that cell. The mitigations are known — more seeds per eval (costlier),
  archive-elite *re-evaluation* (Deep-Grid / stochastic-domain MAP-Elites), or treating each cell's
  contents as a *sample* rather than a point. The recommendation (AC4) flags this as a first-class
  design choice for the spike, not an afterthought.
- **The decomposer guild must be a *reported per-cell distribution*, never a descriptor or a fitness
  target.** This is the B-analogue of F's hard authority boundary. F may not arbitrate the
  distributional modes because mean-field erases per-seed realisations; **B may not collapse them into
  a coordinate**, because a behaviour axis is a *point* per config and the guild's truth is a
  *fraction of seeds*. The correct treatment: the QD archive maps the *existence/stability* skeleton
  (the three axes above, all existence/stability quantities), and the decomposer guild is reported as a
  **per-cell statistic** — "of the seeds whose configs occupy this cell, what fraction sprout a
  persistent guild." That keeps B inside the same discipline F and the validation triad already
  enforce: the atlas reports the distributional emergence as a *distribution*, it does not optimise
  for it or pretend it is a deterministic feature of a cell.

**One-line boundary (the B mirror of F's):** QD maps *which regions of parameter space avoid which
failure modes* (existence/stability — its native strength); it must *report* the sporadic-per-seed
decomposer guild as a per-cell distribution and must *never* make it a descriptor or an objective.

## AC4 — Recommendation: scalarised ES vs. multi-objective vs. QD

**Recommended: a Quality-Diversity archive from the CMA-ME family (specifically CMA-MAE), replacing the
LHS + GP-BO pipeline for the global exploration/atlas job.** Reasoning, against the three candidates:

- **Reject pure scalarised ES** (OpenAI-ES / vanilla CMA-ES on the scalar fitness). It inherits GP-BO's
  smooth-single-objective assumption, has no defence against the deceptive zero-fitness desert (AC3a),
  and — per AC1 — the throughput regime A actually delivered does not give it the sample budget that
  is its only advantage. It optimises for *one tuned config* when the user wants *a map*.
- **Reject classical multi-objective (NSGA-II / MO-CMA-ES).** A Pareto front over the five fitness
  components answers "what are the achievable trade-offs," which is *not* user-story 4. The designer
  wants *which regions of **parameter** space* avoid which failure modes — a map from the 31-cube onto
  behaviour, which a Pareto front (a set of non-dominated *objective* vectors, parameter-space-blind)
  does not provide.
- **Adopt QD / MAP-Elites, with a CMA-ME emitter (CMA-MAE for the soft archive).** QD's native output
  *is* the atlas (user-story 4). The **CMA-ME** emitter matters because the search is 31-dim and
  ill-conditioned: it runs CMA-ES-style covariance-adapting emitters that *fill the archive* rather
  than maximise a scalar, getting CMA-ES's handling of the anisotropic landscape (subsuming the Sobol
  prefilter) *and* MAP-Elites' illumination of the gated, plural objective. **CMA-MAE** (the
  soft-archive variant) additionally tolerates the descriptor noise of AC3b and finer binning without
  the brittleness of hard-threshold MAP-Elites.

**How it composes with the rest of the cluster (the interlock, made concrete):**

- **With A (the data engine).** QD needs only a *batch per generation*, and that batch is exactly the
  (config × seed) unit #353 already parallelises with rayon. So QD is **feedable now, at A's realised
  ~cores×, without the GPU regime A ruled out** — the single most important compositional fact, because
  it means B does not re-open A's NO-GO. Each generation's emitter batch fans out over rayon; the
  archive accumulates across generations.
- **With F (the objective and the prefilter).** F supplies the three behaviour axes (AC2) — and,
  beyond that, F's a-priori viability gates (lens 1) can **prefilter** obviously-dead configs *before*
  they cost a rollout (lens 3), the validation triad's cheap-feeds-expensive ordering. F's
  distance-to-bifurcation order parameters (its `C*`, its Jacobian crossing) can later become *direct*
  descriptors, sharper than the rollout observables.
- **With E (the consumer).** The QD archive's *diverse* coverage — including the failure regions — is
  exactly the training set brief E's surrogate needs so it is not blind away from the optimum. This is
  why E is sequenced *after* B (see #349): B is E's data engine.

**Keep GP-BO for refinement, not exploration.** The recommendation is not "delete the GP." It is
"GP-BO is the wrong tool for the *global, gated, 31-dim* job." A natural division of labour: QD owns
the global atlas; a GP/surrogate (brief E) *refines within* a promising cell once QD has localised it.
That is the surrogate-assisted-evolution pattern the parent brief names, and it falls out of the B→E
sequencing rather than being designed in.

## AC5 — Recommended next move (build-issue-ready)

**A thin MAP-Elites spike, validated against the incumbent at equal rollout budget — the throwaway,
measurement-first slice, exactly the shape of A's #354 and F's #358/#359 spikes.** It commits nothing;
it answers one go/no-go question and produces the first atlas.

Scope (deliberately minimal — reuse everything that exists):

1. **Reuse `decode()` and `run_ensemble()` unchanged.** The spike is a new search *driver*; the
   parameter encoding, the physics, and the evaluator are untouched. The emitter proposes points in the
   same `[0,1]³¹` cube `decode()` already consumes.
2. **Archive over the three F-derived axes** (AC2): `oscillation_strength` × `clustering_strength` ×
   `carcass_locked_fraction`, each already computed per rollout. Start hard-binned MAP-Elites at coarse
   resolution (e.g. 20³); only reach for CMA-MAE's soft archive if noise/coverage demands it.
3. **Cell fitness = the existing 5-component score**; the **six terminal/soft gates route to an
   out-of-archive frontier tally** (AC3a) — which is itself the atlas's dead-region layer.
4. **Handle the distributional mode honestly** (AC3b): record per-cell the *fraction of seeds* that
   sprout a persistent decomposer guild; do **not** make it a descriptor or a fitness term. Decide the
   noise treatment up front (start with the incumbent's 5-seed median + per-cell sample count; escalate
   to elite re-evaluation only if lucky-elite drift shows up).
5. **Run on rayon** — emitter batch per generation = the #353 parallel (config × seed) unit.

Validation (the AC's "reproduce known good/bad configs; do the atlas's boundaries match the gates"):

- **Reproduce the incumbent's wins.** The decoder midpoint and any known viable baseline must land in
  the archive's high-fitness interior, not the dead frontier.
- **Boundary cross-check.** The archive's monoculture region (low axis-2 cells) must coincide with the
  `clustering_strength < 0.5` gate; the lockup frontier (high axis-3) must coincide with the
  `LOCK_FRACTION` gate. Agreement validates the axes; disagreement localises a miscalibrated descriptor
  — the same diagnostic discipline the validation triad prizes.
- **Head-to-head.** At an *equal rollout budget* to a current LHS+BO run, compare (i) best fitness
  found and (ii) archive coverage / QD-score. The go/no-go: **does QD match-or-beat LHS+BO on best
  fitness *and* deliver a usable atlas the incumbent cannot produce at all?**

**Framing for the follow-up issue.** Two issues, sequenced like F's:

- **(1) Thin MAP-Elites spike** (`ready-for-agent`): the throwaway driver above, reporting the
  head-to-head numbers and the first atlas. Independent of any design delta — it changes no committed
  semantics, only adds a search driver. Blocked on nothing.
- **(2) Promote QD to the genesis search** (`ready-for-human`, route through `/grill-with-docs`):
  *only if* (1)'s numbers justify it. Replacing the search method is a system-design-adjacent decision
  (it changes what genesis *returns* — an atlas, not a ranked list — which downstream tooling and the
  expected-properties workflow inherit), so it is gated on the spike's measured result, exactly as A's
  #351 was gated on #353's measured ceiling.

This keeps B inside the project's established rhythm: a cheap, falsifiable spike on a shared object
first; the committing decision second, only with data — and the spike's own numbers retire or justify
the larger investment, never a hunch.
