# Research Brief E — Surrogate / emulator of the genesis landscape

**Status: research finding + recommendation. Commits nothing to the design.** This is the
findings half of issue [#349](https://github.com/samthorold/explorers/issues/349), the last of the
four child briefs of epic [#345](https://github.com/samthorold/explorers/issues/345). It inherits
the epic's two-loops framing, the cluster interlock, and the validation-triad mapping. It is
written *after* its three upstream briefs have not only reported but **shipped to production**:

- **A** (#346) → [A-vectorised-stepper.md](A-vectorised-stepper.md)
- **B** (#347) → [B-es-qd-outer-search.md](B-es-qd-outer-search.md), now the production search
  (`crates/explorers-search/src/qd.rs`, #365/#369), with the F-composed prefilter
  (`crates/explorers-search/src/prefilter.rs`, #370) and the directed lockup emitter (#371/#367).
- **F** (#348) → [F-mean-field-operator.md](F-mean-field-operator.md), with its order parameters
  built and validated: Hopf onset (#358, [F-hopf-validation.md](F-hopf-validation.md)), branching
  (#359, [F-branching-validation.md](F-branching-validation.md)), and the three-compartment flux
  balance promoted into [`viability.md`](../system-design/viability.md) (#357).

Because all three landed, E's question is no longer the one the original brief posed. The
re-scoping note on #349 already moved it; this brief confronts the moved question head-on, with a
**measurement** rather than an argument.

## TL;DR

1. **The crux is not "is a differentiable surrogate possible" — it is "does a surrogate amortise a
   cost that is actually the bottleneck."** It is not. The thing E1 was meant to amortise (F's
   distance-to-bifurcation objective) is now **closed-form-cheap** — measured at **2–3 orders of
   magnitude below a single rollout** (see [Measurement](#the-measurement-the-spine-of-the-verdict)).
   You cannot usefully amortise arithmetic that already costs microseconds. **E1-as-amortise-F is a
   No-Go.**

2. **The one version of E1 that *would* amortise the real cost — predicting fitness/gates without
   rolling out — is mostly pre-empted by what already shipped.** The F-composed **prefilter** kills
   provably-dead configs *before* the rollout for free (#370); F's **indicators** give the
   dynamics-failure boundaries for ~milliseconds without a rollout (#358/#359). What remains that
   only a rollout yields is the *distributional* residue — the actual fitness score and the
   sporadic-per-seed decomposer guild — and the authority boundary (inherited from B/F) **forbids**
   E from learning that residue as a deterministic feature. So E1's marginal value over
   *prefilter + F-indicators* is thin, and it sits exactly where a surrogate is least trustworthy.

3. **E1 vs E2, made concrete.** *E1* (surrogate of the outer params→fitness map) is safe only as
   **in-cell refinement** within an already-localised QD cell (B's division of labour), never as a
   global arbiter. *E2* (emulator of the inner rollout) **drifts exactly at bifurcations** — and F
   now *pinpoints* those boundaries (its validated Hopf/branching crossings), so E2's unreliable
   zone is no longer fuzzy: it is the precise set of cells adjacent to F's predicted crossings. E2
   is safe for *coarse interior sketching*, **never the final arbiter at a boundary** — and since F
   already hands you those boundaries cheaply, E2's coarse sketch has little left to buy.

4. **The distance-to-bifurcation objective (AC3) is achievable and worthwhile — but it is F's
   deliverable, already done, not E's.** F built and validated it. E adds value only if predicting
   it *cheaply across param space* beats evaluating F's operator directly. The measurement says it
   does not: F's operator is already cheaper than the rollout E would train on.

5. **Recommendation (AC4): defer E — not "never," but "not now, and not next."** The conditions
   that would make E pay off do not hold today (F's per-eval cost is not the bottleneck; the
   prefilter already shrinks the rollout budget; the atlas is low-dimensional). Pursue A's *inner*
   GPU stepper and densify the QD atlas first. E becomes worth revisiting only against a specific,
   named trigger ([AC5](#ac5--recommended-next-move-build-issue-ready)).

6. **When revisited, the first move is a falsifiable spike, not a build:** train E1 on the
   production `atlas.json`, hold out the cells adjacent to F's predicted crossings, and measure
   surrogate error *in-interior vs near-boundary*. GO only if near-boundary error stays under the
   gate margin **and** train+eval amortisation beats prefilter+rollout at the target budget. This
   bakes the near-boundary-drift caveat and the authority boundary into the go/no-go.

---

## The measurement (the spine of the verdict)

The whole recommendation turns on one comparison the re-scoping note flagged but left open: *is
F's operator cheap enough that a surrogate amortising it buys little?* Measured on this machine
(release builds, macOS, warm cache; wall-clock, reported as order-of-magnitude, not a benchmark):

| What | What it costs | Source |
| --- | --- | --- |
| **F Hopf indicator** (the distance-to-Hopf objective) | **< 5 ms** — closed-form; the throwaway bin runs a *96-point sweep* in under 10 ms, so a single-point objective is sub-millisecond | `crates/explorers-sim/src/bin/hopf_prototype.rs` (`analytic_crossing`, a handful of flops) |
| **F branching indicator** (distance-to-branching) | **~0.28 s for the full diagnostic** (a 40-point bifurcation sweep + the singular-point flow + layer-3 density). A **single-point** objective — one flow + one Hessian + one invasion margin, no sweep — is **~10 ms** | `crates/explorers-sim/src/bin/branching_detector.rs` |
| **One genesis rollout** (single seed, `max_ticks` 200) | **~170 ms wall (~355 ms CPU)** | measured: production search `crates/explorers-search/src/main.rs` |
| **One config evaluation** (5-seed ensemble) | **~0.85 s wall (~1.8 s CPU)** | same |

Method for the rollout figure: a small production QD run (`--max-ticks 200 --ensemble 5 --batch
16 --generations 4`) took **67.9 s wall / 142 s CPU** (~2× rayon parallelism on this box) over
~80 configs × 5 seeds ≈ 400 single-seed rollouts → ~170 ms/seed-rollout, ~0.85 s per 5-seed
config.

**The ratios:**

- F Hopf objective : config rollout ≈ **5 ms : 850 ms ≈ 1 : 170**.
- F branching single-point objective : config rollout ≈ **10 ms : 850 ms ≈ 1 : 85**.
- Even the *full branching diagnostic* (0.28 s, far more work than a per-config objective needs) :
  config rollout ≈ **1 : 3**.

**Two structural facts make the gap a floor, not a ceiling:**

1. **F's cost is flat in T and N.** The mean-field operators do *no per-tick agent loop* — they
   read lumped coefficients and solve a small fixed-size spectral problem. The rollout, by
   contrast, is O(N²) per tick over T ticks; the #363 spike already records that random configs
   "balloon toward the population-explosion ceiling (O(n²) per tick)." As genesis pushes toward
   richer worlds (higher `N_max`) or longer horizons (to resolve slow oscillation onset), the
   rollout cost climbs while F's stays put. The ratio *widens*.

2. **A's free throughput already attacked the same cost from the other side.** A's recommended,
   shipped win is rayon over (config × seed) — ~cores× (10–50×), bit-identical, zero divergence
   risk (A-vectorised-stepper.md, AC3). The expensive rollout is already being parallelised; a
   surrogate would be competing with that, not with a naive serial loop.

**Conclusion of the measurement:** a surrogate trained to predict F's distance-to-bifurcation
would amortise a cost that is *already two-to-three orders of magnitude below the rollout it learns
from*, and is *not the search's bottleneck*. The amortisation buys microseconds while adding a
training pipeline, a staleness liability, and a model that is least accurate precisely at the
boundaries F exists to locate. That is a bad trade. The original E1 framing is retired by its own
upstream success.

---

## AC1 — E1 vs E2: what each buys, and where each is safe vs unreliable

### E1 — surrogate of the *outer* map (params → fitness)

The existing GP (`crates/explorers-search/src/gp.rs`, `bayesopt.rs`) already *is* an E1 in spirit —
though its GP-BO search path was retired when QD was promoted (#365; `search.rs` notes "the
LHS/Sobol/GP-BO path is retired"). The brief asks whether to scale that surrogate idea (neural /
Bayesian-NN / deep-kernel) for higher dimension and cheap gradient-based acquisition. Three
sub-positions, only the third defensible:

- **E1 as amortiser of F's objective — No-Go.** Covered by the measurement: nothing to amortise.

- **E1 as amortiser of the *rollout* (predict fitness/gates directly, skip the sim) — thin,
  mostly pre-empted.** This *is* the version that targets the real cost (0.85 s/config). But the
  cheap pre-rollout signal it would provide is already provided:
  - the **prefilter** (#370) routes provably-dead configs to the dead frontier *for free* — the a
    priori existence gates from `viability.md`. A surrogate adds nothing on those.
  - F's **indicators** give the dynamics-failure boundaries (Hopf, branching) for ~10 ms without a
    rollout — sharper than a learned fit, with no training set and no staleness.
  - What is *left* — the scalar fitness value and the decomposer-guild distribution — is the
    **distributional residue** the authority boundary (below) says E must not learn deterministically.
  So E1-over-the-rollout's marginal value above *prefilter + F-indicators* is small, and it
  concentrates in exactly the region (near boundaries, and on per-seed-sporadic properties) where a
  surrogate is least reliable.

- **E1 as in-cell refinement — the one safe niche.** B already carved the division of labour:
  *"QD owns the global atlas; a GP/surrogate (brief E) refines within a promising cell once QD has
  localised it"* (B-es-qd-outer-search.md). Here E1 is the textbook surrogate-assisted-evolution
  move, and gradient-based acquisition is genuinely additive — F gives a *value* at a point; a
  differentiable surrogate gives a *gradient*. This is safe because (a) within a single localised,
  live cell the map is smooth and far from the gated cliffs, and (b) the surrogate never arbiters
  existence/stability — it only ranks candidates QD has already certified live. **Caveat:** QD's
  CMA-MAE emitter already does covariance adaptation, so the *incremental* gain over the shipped
  emitter is modest; this is a refinement, not a step-change.

**Where E1 is safe:** smoothing/refining *within* a localised live cell. **Where E1 is unreliable:**
as a global arbiter, anywhere near a gate boundary, and on any distributional property.

### E2 — emulator of the *inner* rollout (neural-ODE / sequence model)

E2 predicts trajectory summaries from (params, seed) without running T ticks. Its load-bearing
flaw was stated in the parent epic and the brief: **it drifts exactly near bifurcations** — where
the dynamics are most sensitive and where the failure-mode boundary actually lives.

F sharpens this from a caveat into a *coordinate*. F's validated crossings (the Neimark–Sacker
Hopf crossing in `base_trophic_efficiency`; the distance-to-branching zero) name the precise
boundaries E2 would drift across. So E2's unreliable zone is no longer a vague "near
bifurcations" — it is **the set of cells whose F-indicator sits within a band of zero.** That makes
E2 *auditable* (you can mask its predictions there) but also *redundant*: in exactly the region
where you would most want a cheap predictor, F already gives you a cheap, physics-grounded one, and
in the interior the rollout is cheap enough (and A-parallelised) that a learned emulator's coarse
sketch saves little.

**Where E2 is safe:** coarse, interior landscape sketching, far from any F crossing. **Where E2 is
unreliable:** at any boundary — definitionally, and now precisely locatable. **Verdict:** E2 has no
live use case today; its only safe region is the one where the alternative (F + parallelised
rollout) is already adequate.

---

## AC2 — How E composes with A/B (data engine) and F (reduced coordinates)

The composition is real and already wired — which is *why* E is now well-defined enough to decline:

- **Fed by B's atlas (the data engine).** The production search dumps `atlas.json` with exactly the
  structure E needs: `cells` (live elites across the three behaviour axes), `dead_frontier`
  (observed deaths by cliff), and `dead_frontier_apriori` (prefilter kills). This is the *diverse*
  coverage — including the failure regions — that the brief insists on so the surrogate "is not
  blind away from the optimum." Before #363 this was abstract; it is now a file on disk. **If E is
  ever built, this is its training set, and it already exists.**

- **Fed by A's throughput.** A's free (config × seed) rayon parallelism (10–50×) and the eventual
  inner GPU stepper are what make it cheap to *generate* held-out rollouts for surrogate validation
  — and, more decisively, what make the rollout cheap enough that a surrogate's value shrinks. A is
  both E's data engine *and* E's competitor.

- **Structured by F's reduced coordinates.** F's AC1 finding — each failure mode lives on a
  low-dimensional subspace (Hopf on a 2-compartment coupling / 2×2 Jacobian; branching and
  generalist-dominance on the 3-dim specification subspace autotrophy × heterotrophy × mobility) —
  is exactly the physics-informed feature set a surrogate would want as inputs instead of the raw
  31-dim cube. **But the same finding is what undercuts E:** if the objective already factors
  through a 2-D or 3-D operator that is closed-form to evaluate, there is little a neural surrogate
  over those same coordinates can learn that the operator does not already give exactly.

The interlock the epic describes (F sharpens the objective → A makes it cheap → B maps it → E
amortises the intractable parts) has a hole at the last link: **after F + A + B, there is no
intractable part left for E to amortise.** F made the objective tractable; A/B made evaluating it
cheap and parallel. E was the amortiser of an intractability that the other three briefs removed.

---

## AC3 — Is a distance-to-bifurcation objective achievable and worthwhile?

**Achievable: yes — and already built, by F, not E.** The re-scoping note is correct: the order
parameters exist and compute on real scenarios (Hopf #358, branching #359, flux #357). The
branching detector even emits a *signed distance-to-branching scalar* (its layer 4) as an explicit
candidate genesis objective.

**Worthwhile: yes — as F's deliverable.** Replacing eyeballed fitness with a computable
distance-to-bifurcation is precisely the affirmative-viability move `viability.md` points toward,
and it is cheap (the measurement). The natural next step is to wire F's distance-to-bifurcation in
as a **QD behaviour descriptor or objective** — but that is a *Cluster F/B* follow-up (compose F's
indicator into the search, as #366 did for the existence gates), **not an E surrogate**.

**Worthwhile *as a surrogate target* (E's actual question): no.** A surrogate predicting F's
distance-to-bifurcation amortises a sub-millisecond-to-10 ms computation. There is no economic case.

---

## AC4 — Recommendation: pursue now, after A/B/F, or not at all?

**Defer.** Concretely: *not now, not next; revisit only against a named trigger.* Reasoning:

1. **The bottleneck E targets has moved.** E1's premise was that the params→fitness map (or F's
   objective) is expensive enough to amortise. After F (cheap operator), the prefilter (free
   dead-config rejection), and A (parallel rollouts), neither is the binding cost.

2. **E's safe niches are narrow and partly already covered.** In-cell refinement (E1) is real but
   incremental over the shipped CMA-MAE emitter; coarse interior sketching (E2) is dominated by
   F + parallel rollout; everything else runs into the authority boundary or the near-boundary
   drift.

3. **E carries ongoing cost the others don't.** A surrogate is a training pipeline, a held-out
   validation regime, and a staleness liability (it must be retrained as the physics/evaluator
   evolve — and they are actively evolving). The other three reframings are *analyses* or
   *parallelisations* with no model to keep fresh. The maintenance asymmetry matters (epic
   user-story 8/9).

4. **It is genuinely sequenced last for a reason.** The brief's own "most actionable once A/B and F
   report" was right; what it could not predict is that A/B/F would report *so completely* that they
   would absorb E's mandate.

**This is a "defer," not a "never."** The honest revisit triggers (any one):

- A single QD generation's **rollout budget becomes the binding constraint** *even after* the
  prefilter and a built inner GPU stepper — i.e. you are rollout-bound, not search-operator-bound.
- The **behaviour/objective space grows** beyond what F's low-dimensional operators cover (a failure
  mode is discovered that does *not* factor through a 2–3-D subspace), so F stops being cheap-exact.
- You want **gradient-based acquisition inside dense cells** and measure that CMA-MAE's covariance
  adaptation is leaving ranking performance on the table.

---

## AC5 — Recommended next move (build-issue-ready)

**No build now.** Update #349's "Blocked by" framing to record the *defer* and the triggers above,
and write the follow-up only when a trigger fires. When it does, the follow-up is a **throwaway,
measurement-first spike** — the same rhythm as A's #354, F's #358/#359, and B's #363 — that
commits nothing and answers one go/no-go:

> **Spike: does an E1 surrogate beat prefilter+rollout, *and* stay honest at the boundary?**
>
> 1. **Train set:** the production `atlas.json` — `cells` + `dead_frontier` + `dead_frontier_apriori`
>    (the diverse, failure-region-inclusive coverage the brief demands). No new data generation
>    needed; B's atlas is the data engine.
> 2. **Inputs:** F's reduced coordinates (the 2-D Hopf coupling, the 3-D specification subspace),
>    not the raw 31-cube — the physics-informed feature structuring.
> 3. **Target:** fitness / gate-pass. **Explicitly not** the decomposer-guild fraction — that is a
>    per-seed-sporadic distributional property the authority boundary forbids learning as a
>    deterministic feature (it stays a reported distribution, never a surrogate output).
> 4. **The decisive test — near-boundary drift, made concrete:** hold out the cells whose F-indicator
>    sits within a band of its zero crossing (the Hopf / branching boundaries). Measure surrogate
>    error **in-interior vs near-boundary** separately.
> 5. **Go/No-Go:** GO only if (a) near-boundary error stays **below the gate margin** (the surrogate
>    does not misclassify a boundary cell as live/dead), **and** (b) train + eval cost amortises
>    against just running prefilter + (parallel) rollout at the target search budget. If either
>    fails — which the measurement predicts is likely — the result *retires* E, exactly as a spike
>    should.

Framing for the follow-up issue: this is a `ready-for-agent` research spike, blocked on nothing
once a revisit trigger fires, touching no committed semantics (a throwaway driver that reads
`atlas.json`). It either justifies the larger E investment with numbers or closes E for good — never
a hunch.

---

## Validation discipline (how this finding could be falsified)

Per the validation triad and the "example file as connective tissue" principle, E's claims are
falsifiable on the shared `scenarios/` objects:

- **The measurement is reproducible** (commands in [the measurement section](#the-measurement-the-spine-of-the-verdict));
  re-run on the inner-GPU stepper once A's #351 lands — if the rollout drops below F's operator
  cost, the central ratio flips and the "defer" should be re-examined. *That is the primary
  falsifier.*
- **The near-boundary-drift caveat** is falsifiable by the AC5 spike's held-out boundary cells: if
  a surrogate predicts gate-pass within the gate margin *even adjacent to F's crossings*, E2/E1
  earn back authority near boundaries and the position softens.
- **The authority boundary is non-negotiable:** if any E variant is found to predict the
  decomposer-guild fraction as a deterministic feature, that is a defect in E, not a capability —
  the guild is "confirmed across seed ensembles, sporadic per-seed, never guaranteed on a single
  run" (`expected-properties.md`), the same line B and F draw.
