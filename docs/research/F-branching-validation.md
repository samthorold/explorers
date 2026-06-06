# Research note — validating Brief F's branching reading of monoculture↔coexistence

**Status: research finding (spike verdict). Commits nothing.** This is the verdict
half of issue [#359](https://github.com/) — Research Brief F's **move 3**, the cheap
*affirmative* **monoculture↔coexistence branching detector**, built as a throwaway
prototype and validated against the observed clustering boundary before any larger
investment (the detector becoming a genesis *objective*). It is the sibling of the
Hopf spike [#358](https://github.com/) ([`F-hopf-validation.md`](F-hopf-validation.md)),
whose *"Qualified GO"* verdict gated this one. Like the brief, it lives outside
`docs/system-design/`: it records investigation that *informs* a future commitment and
introduces no mechanism, functional form, or parameter into the world rules.

It is the meeting point of the [validation triad](../system-design/viability.md#place-in-the-validation-triad):
**F predicts** a branching direction on a scenario's initial condition; the **headless
run observes** `clustering_strength` / `coexistence_duration` on the same file; genesis
*locates* it. Read this alongside [`F-mean-field-operator.md`](F-mean-field-operator.md)
(AC1 row 5 — monoculture↔coexistence is *the one genuinely distribution-shaped mode*;
AC2 the operator decomposition; AC3 the authority boundary) and the
`A⇌L⇌C` flux balance in [`viability.md`](../system-design/viability.md).

## What was built

- **A throwaway moment-closure + invasion-fitness prototype** —
  `crates/explorers-sim/src/bin/branching_detector.rs`, clearly marked **not part of
  the production stepper**. It does not touch, fork, or alter the committed scalar
  stepper, the RNG, or the evaluation order. It lifts the agent cloud to a density on
  the 3-dim **specification subspace** (autotrophy `a` × heterotrophy `h` × mobility
  `m`) — the subspace that defines trophic *role* (CONTEXT.md) — and reads its branching
  structure. Four layers, **each term landing on a real phase**:
  - **Layer 1 — moment closure.** Per resident compartment, trait **mean** `μ ∈ ℝ³` +
    **covariance** `Σ ∈ ℝ³ˣ³` (the Price-equation / quantitative-genetics view).
    `dμ/dt = Σ·∇g(μ;E)` moves the mean down the **selection gradient**; the
    **mutation–diffusion** injection `M = mutation_rate · mutation_magnitude² ·
    birth_rate · I₃` (from the per-trait Gaussian mutation at `resolve_reproduction`,
    `phase.rs:1049`) feeds the covariance via `dΣ/dt = Σ·H·Σ + M`, `H = ∇²g`.
  - **Layer 2 — the branching detector.** The cheap signal is the sign of disruptive vs
    stabilising selection at the resident singular point, read off the committed
    per-capita net growth rate `g(θ;E)` — assembled from photosynthesise (`phase.rs:19`,
    density-dependent light share) + resolve_drains (`phase.rs:386`) through the
    **committed kernel** `base_trophic_efficiency · exp(−trophic_distance_decay · d)`
    (`lib.rs:770`) over `TraitVector::distance` (`lib.rs:73`) + metabolise
    (`phase.rs:156`, superlinear maintenance) + grow (`phase.rs:189`, κ·`growth_efficiency`).
    Gradient and Hessian by **central finite differences over `g`**, restricted to the
    3-dim subspace — *never* forming the 7-dim histogram.
  - **Layer 3 — targeted confirmation.** *Only* when layer 2 flags branching, a coarse
    8×8 density on (autotrophy × heterotrophy) is evolved to stationarity under the AC2
    operator (selection diagonal + mutation diffusion + bilinear trophic) and its
    marginals are read for multi-peakedness by a valley-depth test (the
    `clustering_strength` idea, `genesis-eval/src/lib.rs:246`). Never the dense 7-dim grid.
  - **Layer 4 — candidate genesis objective.** A signed **distance-to-branching** scalar
    `D` (zero = at the bifurcation), with the authority caveat documented alongside.
  All four are unit-tested (six tests: the branching signal flips sign across a hand-built
  stabilising→disruptive landscape; the mutation injection is positive-definite; the
  covariance leading eigenvalue grows under disruptive curvature; the mean flow descends the
  gradient; a heterotroph mutant taps the producer niche in the committed `g`; the valley test
  separates a synthetic bimodal marginal from a unimodal one).
- **A minimal branching scenario** —
  `scenarios/example11_branching_coexistence.json`: one producer cluster (8,
  high-autotrophy, sessile) + one consumer cluster (4, high-heterotrophy, lightly mobile)
  **co-located** within feeding reach (the deliberate contrast with `example8`, where a
  dispersed prey field decouples the loop). `trophic_distance_decay = 1.0` places the
  predicted branching crossing mid-range; committed at `base_trophic_efficiency = 0.8`,
  just **above** the predicted crossing — the analogue of `example10` committing above its
  Hopf crossing. Schema-consistent with `example10`. `probes = monoculture`,
  `prediction = live`, `source_issue = 359`.

## F's prediction (the branching reading)

The decisive cheap object is **invasion fitness** `s(y;x) = g(y; E*(x))` — the per-capita
growth of a rare mutant `y` in the environment the resident `x` sets — restricted to the
spec subspace. The detector reports two complementary reads plus a confirmation:

- **Layer 2a — the local Gaussian curvature.** Flow the moment-closure mean to its
  gradient-zero singular point `μ*` in the producer-set field, and read the leading
  eigenvalue of the Hessian `H = ∇²g(μ*)`. Positive ⇒ disruptive (the single peak splits);
  negative ⇒ stabilising (the peak holds).
- **Layer 2b — the invasibility margin (the branching signal).** The
  resident-relative invasion fitness of a rare heterotroph into the producer
  monoculture, `D = max_y [g(y; E_prod) − g(x_prod; E_prod)]` over the consumer half of
  the trophic axis. `D > 0` ⇒ a consumer can invade ⇒ a second peak nucleates (branching
  → coexistence); `D ≤ 0` ⇒ the consumer niche is closed (monoculture). `D` is the
  signed **distance-to-branching** scalar; sweeping `base_trophic_efficiency` locates the
  crossing `base*` where `D = 0` — F's **falsifiable** monoculture↔coexistence boundary.
- **Layer 3 — peak confirmation**, run only when `D > 0`.

The reads on the three validation configs (`cargo run … branching_detector`):

| scenario | clusters seeded | 2a local `λ_max` | 2b margin `D` | crossing `base*` | layer-3 peaks | **F's read** |
|---|---|---|---|---|---|---|
| `example4` (coexistence pole) | producer + consumer | −0.297 (stabilising) | **+0.381** | 0.082 | 2 / 2 | **branching → coexistence** |
| `example8` (monoculture pole) | producer + consumer | −0.334 (stabilising) | **+0.181** | 0.044 | 2 / 2 | **branching** (disagrees — see below) |
| `example11` (near boundary) | producer + consumer | −0.040 (stabilising) | **+0.022** | **0.628** | 2 / 2 | **branching, near boundary** |

> **The two reads split, and the split is the finding.** The local Gaussian curvature
> (2a) reads **stabilising everywhere** — at *every* established cluster the single-peak
> Hessian is negative-definite, because an established peak *is* a fitness maximum that
> holds together. The invasibility margin (2b) reads **branching** wherever a consumer can
> tap the producer biomass. These are not contradictory: 2a asks "does *this* peak split?"
> and 2b asks "can a *second* peak nucleate?" — and **only 2b can see the second niche.**
> The local-curvature read is **blind by construction** to a frequency-dependent second
> peak; that is exactly Brief F's predicted weak point, and exactly why layers 2b + 3 exist.

`example11` is committed at `base = 0.8`, just above its predicted crossing `base* = 0.628`,
so **F predicts branching (coexistence) — near the boundary** (`D = +0.022`).

## The observation (the headless evaluator)

The committed evaluator (`eval_scenarios`, the same machinery genesis uses) reads
`clustering_strength` (the dip-test multi-peak signature) and `coexistence_duration` over
an 8-seed ensemble. From `scenarios/observed.json` (`example4`, `example8`) and a fresh
`example11` run:

| scenario | modal mode | clustering_strength (med / min / max) | coexistence_duration (med / max) | final pop (med) | **observed read** |
|---|---|---|---|---|---|
| `example4` | none (8/8) | **1.000** / 1.0 / 1.0 | 0.446 / — | 8 | **coexistence** |
| `example8` | none (8/8) | **0.000** / 0.0 / 0.0 | 0.000 / — | 3 | **monoculture** (consumers fail) |
| `example11` | none (8/8) | 0.000 / 0.0 / **1.0** | **0.516** / 0.991 | 2 | **borderline coexistence** |

> `example4` is unambiguously multi-peak (clustering 1.0). `example8` collapses to a single
> producer guild (clustering 0.0, consumers do not establish). `example11` sits genuinely
> *on the boundary*: the consumer **does coexist** for a substantial fraction of the run
> (`coexistence_duration` median 0.516, up to 0.991), but `clustering_strength` cannot lock
> in because the surviving population is tiny (median 2, below the metric's n ≥ 4 floor) —
> some seeds reach clustering 1.0, most read 0.

## Does the observed boundary match F's prediction? Mostly — and the disagreement localises

- **`example4` (coexistence): AGREEMENT.** F reads branching (`D = +0.381`, the largest of
  the three); the run is multi-peak (clustering 1.0). Confidence-raising.
- **`example11` (near boundary): AGREEMENT, qualified.** F reads branching but *near the
  crossing* (`D = +0.022`, `base* = 0.628`, committed `base = 0.8`); the run shows borderline
  coexistence — the consumer establishes and coexists for half the run, but the population is
  too small for the clustering metric to register. The detector's *graded* scalar tracks the
  observation: `example11`'s `D` (0.022) is an order of magnitude below `example4`'s (0.381),
  i.e. F itself places `example11` close to the boundary, which is where the run lands.
- **`example8` (monoculture): DISAGREEMENT, and it localises precisely.** F reads branching
  (`D = +0.181`); the run is monoculture (clustering 0.0, consumers go extinct). This is the
  **diagnostic signal the triad prizes** (Brief F, AC5), not a defect in either lens, and it
  localises to two effects **the mean field discards by construction (AC3)** — the same two
  the Hopf spike (#358) surfaced:
  1. **Spatial in-reach geometry (primary).** `example8`'s consumers are mobile over a prey
     field dispersed across the world; their *realised* contact rate is far below the
     well-mixed mass-action `B(n,n)` the invasibility margin assumes. The bilinear term is an
     integral over an in-reach indicator (Brief F, AC2 §4) the mean field replaces with 1.
     (**Re-validation caveat, #380:** under the contact-duration ramp these mobile consumers
     could not feed *at all* — #379 — so the `example8` monoculture read is confounded and
     should be re-run now that the binary-reach drain lets mobile consumers feed; the in-reach
     geometry argument itself is unaffected, as it concerns encounter, not the drain magnitude.)
     `example11` was built **co-located** precisely to close this gap — and there F's branching
     read does match a (borderline) observed coexistence, isolating geometry as the `example8`
     fault.
  2. **Wear-mediated structural fragility (secondary).** `example8` runs with `wear_rate = 0.1`
     **on**; the mean field carries no wear. Structural fragility is exactly the
     mortality-mediated force Brief F flags (AC1 generalist-dominance row, AC3) as *"the weakest
     and most provisional"* read — it *"acts through a peak-relative structural death threshold
     whose bite depends on the realised structure trajectory, which a mean field smooths."*
     `example4` and `example11` run with `wear_rate = 0` and agree; `example8` carries wear and
     disagrees — a clean isolation of the second fault.

## The two flagged weak points — assessed with evidence

Brief F names two specific places the branching detector is expected to be weak. This spike
tested both:

1. **Gaussian closure is blind at the branching point.** *Confirmed, and covered.* The local
   Hessian (2a) reads **stabilising on all three configs** (`λ_max` = −0.30 / −0.33 / −0.04),
   including the unambiguously multi-peak `example4`. A unimodal moment model cannot, by
   construction, see the second peak. The deliverable shows the gap is **covered** two ways:
   (a) the **invasibility margin** (2b) reads the second niche directly via invasion fitness,
   not curvature; and (b) the **layer-3 coarse density** evolves the full AC2 operator and reads
   **2 peaks** on both marginals for every branching-flagged config — the targeted 8×8 grid
   resolves exactly the bimodality the Gaussian closure erases. This is the intended division of
   labour: cheap moment view to *flag*, coarse bin to *confirm*.
2. **Generalist-dominance / fragility smooths poorly.** *Confirmed as the dominant `example8`
   fault, and recorded as a hard caveat.* The detector cannot see wear-mediated structural
   death (it carries no wear, no peak-relative threshold). Where that force is the binding
   constraint on whether a second guild establishes — `example8` — F's branching read is wrong
   in the conservative direction (it says a niche is open that fragility actually closes). The
   caveat stands as Brief F states it: **F's verdict on fragility-limited coexistence is the
   weakest of the set and must not be trusted against a wear-on world.**

## Authority boundary (Brief F, AC3) — held throughout

Every claim here is an **existence/multiplicity** claim about the deterministic-on-average
skeleton: *is there a second viable peak; in which direction does selection point.* Nothing
claims a **per-seed distributional** outcome. In particular the spike never claims a
sporadic-per-seed decomposer guild — `expected-properties.md`'s *"confirmed across seed
ensembles, sporadic per-seed, never guaranteed on a single run"* — which remains genesis
search's to report. The `example8` consumer-extinction observation is read as an
existence/stability fact about *this* construction, not as a distributional verdict. Where the
detector and the observed boundary disagree, that is the prized diagnostic signal, not a
failure of either lens.

## Verdict

**The branching skeleton is internally trustworthy and its disagreements localise cleanly; it
is ready to become a genesis *objective* — gated on the same observable-hardening #358 already
flagged.**

- **The operator reading lands on real phases and the math is self-consistent.** Every term in
  `g` is a committed flux; the invasion-fitness margin is a textbook adaptive-dynamics
  invasibility test; the bifurcation sweep produces a clean `D = 0` crossing in
  `base_trophic_efficiency`; all six unit tests pass.
- **The cheap detector's graded output tracks the observed boundary where the mean field is
  valid.** `example4` (rich, co-located, wear-off) agrees strongly; `example11` (co-located,
  wear-off, near the crossing) agrees at the boundary; the lone disagreement (`example8`)
  localises *exactly* to the two effects F's own authority boundary declares out of scope —
  spatial reach and wear-mediated fragility.
- **Both of Brief F's predicted weak points reproduced**, and the deliverable shows the
  intended cover for the first (invasibility + coarse-bin confirmation) and records the second
  as a hard caveat (fragility-limited coexistence is not F's to call).

**Go / no-go for promoting the distance-to-branching objective: Qualified GO**, with the same
gate as #358. The scalar `D` is a usable *direction*-of-coexistence signal **on configs where
the coupling is geometrically realised and fragility is not the binding constraint**. Before it
becomes a trusted genesis objective: (a) the headless observable must distinguish
*borderline-coexistence-but-small-N* (`example11`) from *true monoculture* — `clustering_strength`
silently zeroes below n = 4, so `coexistence_duration` or a population-robust multi-peak test
should carry the read; and (b) the spatial in-reach geometry and a wear penalty should enter the
objective's environment, or the objective should be **restricted to the co-located, wear-off
regime where this spike validated it**. None of this undermines using `D` as F's cheap
*deterministic branching skeleton*, which is all move 3 needs from it.

## Reproduce

```
# F's prediction (throwaway prototype — not the production stepper):
cargo run -p explorers-sim --bin branching_detector -- scenarios/example4.json                    # coexistence pole
cargo run -p explorers-sim --bin branching_detector -- scenarios/example8.json                    # monoculture pole
cargo run -p explorers-sim --bin branching_detector -- scenarios/example11_branching_coexistence.json  # near boundary

# the observation (clustering_strength / coexistence_duration over the seed ensemble):
cargo run -p explorers-genesis-eval --bin eval_scenarios -- --seeds 8 scenarios/example11_branching_coexistence.json
# (example4 / example8 are already in scenarios/observed.json)

# the unit tests (branching signal sign-flip, PD mutation injection, covariance growth,
# mean-flow descent, valley test):
cargo test -p explorers-sim --bin branching_detector
```
