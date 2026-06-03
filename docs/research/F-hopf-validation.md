# Research note — validating Brief F's Hopf reading on a minimal predator-prey example

**Status: research finding (spike verdict). Commits nothing.** This is the verdict
half of issue [#358](https://github.com/) — the cheap, throwaway spike that tests
whether Research Brief F's *operator reading* of the committed tick map `T` is
trustworthy on one minimal example, before the larger investment (F's **move 3**,
the moment-closure genesis objective) is made. It lives outside
`docs/system-design/` for the same reason the brief does: it records investigation
that *informs* a future commitment, and introduces no mechanism, functional form,
or parameter into the world rules.

It is the meeting point of the [validation triad](../system-design/viability.md#place-in-the-validation-triad):
**F predicts** a bifurcation point on one example; the **headless run observes**
the freeze→oscillate transition on the same file. Read this alongside
[`F-mean-field-operator.md`](F-mean-field-operator.md) (AC2 the operator
decomposition, AC5 cross-check 2 the predator-prey construction) and the
`A⇌L⇌C` flux balance in [`viability.md`](../system-design/viability.md).

## What was built

- **A throwaway compartment-ODE + 2-block Jacobian prototype** —
  `crates/explorers-sim/src/bin/hopf_prototype.rs`, clearly marked **not part of
  the production stepper**. It does not touch, fork, or alter the committed scalar
  stepper, the RNG, or the evaluation order. It assembles a 2-compartment
  producer-biomass `P` ↔ consumer-biomass `C` discrete map — the mean-field
  reduction of `state_{t+1} = T(state_t)` onto the frozen↔oscillation coordinate
  (Brief F, AC1) — **from committed fluxes only**, each term landing on a real
  phase:
  - `r_P·P·(1 − P/K_P)` — the **selection/growth diagonal** from photosynthesise
    (`phase.rs:19`) + metabolise (`phase.rs:156`) + grow (`phase.rs:189`).
  - `a·P·C` — the **bilinear trophic term**, *the seat of the Hopf*, from
    resolve_drains (`phase.rs:386`), read as a Holling type-I (mass-action) attack
    rate in the unsaturated regime the proportional split (`phase.rs:489`) reduces
    to.
  - `e = base_trophic_efficiency · exp(−trophic_distance_decay · d)` — the
    **committed kernel** (`lib.rs:770`) over the committed `TraitVector::distance`
    (`lib.rs:73`).
  - `m·C` — the **removal diagonal** from death (`phase.rs:687`).
- **A minimal predator-prey scenario** —
  `scenarios/example10_predator_prey_hopf.json`: one producer cluster (18, a
  sparse slow-reproducing standing crop) + one consumer cluster (3, within feeding
  reach), nothing else. Schema-consistent with `example9`. Carries F's predicted
  crossing in `metadata.rationale`, `probes = frozen_dynamics`,
  `source_issue = 358`.

## F's prediction (the analytic crossing)

At the interior (coexistence) fixed point the 2×2 Jacobian of the discrete map has

```
trace τ = 2 − r_P·P*/K_P ,   det Δ = 1 − r_P·P*/K_P + m·r_P·(1 − P*/K_P)
```

with a **complex-conjugate eigenvalue pair** of modulus `√Δ` (verified: the pair
is a spiral, `τ² < 4Δ`, across the whole interior branch). A discrete-time Hopf —
**Neimark–Sacker** — bifurcation is the pair crossing the unit circle, `Δ = 1`.
The algebra collapses cleanly (`r_P` cancels) to

```
P*/K_P = m/(1+m)   ⟺   base* = (1 + m) / (κ_C · γ · a · K_P · e_dist)
```

On `example10` the prototype derives (from the committed parameters and the two
clusters): `d = 0.959`, `e_dist = exp(−0.5·0.959) = 0.62`, `r_P = 1.30`,
`K_P = 71.1`, `a = 0.423`, `m = 0.113`, `κ_C = 0.45`, `γ = 0.30`, giving

> **F predicts a Neimark–Sacker crossing at `base_trophic_efficiency* = 0.4427`.**
> Below it the interior point is a stable spiral (*frozen* coexistence); above it
> a limit cycle (*oscillation*). The analytic closed form and the numerically
> swept spectral radius agree to 4 d.p. (unit-tested), and the prototype confirms
> the crossing is a genuine complex-pair crossing, not a real-eigenvalue
> (period-doubling/fold) one.

The scenario is committed at `base = 0.8` (above the crossing), so **F predicts
oscillation** on the committed file.

## The observation (the headless sweep)

Sweeping `base_trophic_efficiency` across the predicted crossing through the
existing evaluator —

```
cargo run -p explorers-genesis-eval --bin eval_scenarios -- --seeds 8 scenarios/example10_predator_prey_hopf.json
```

(repeated with each `base` patched in) — reads the committed `oscillation_strength`
metric (`crates/explorers-genesis-eval/src/lib.rs:374`). The result, 8 seeds,
2000 ticks:

| base | modal mode | oscillation_strength (med / min / max) | survives | pop |
|------|-----------|------------------------------------------|----------|-----|
| 0.1 | none | 0.291 / 0.115 / 0.485 | 2000 | 15 |
| 0.2 | none | 0.291 / 0.115 / 0.485 | 2000 | 15 |
| 0.3 | none | 0.333 / 0.115 / 0.485 | 2000 | 15 |
| **0.44** | none | ~0.30 / 0.115 / ~0.49 | 2000 | 15 |
| 0.5 | none | 0.333 / 0.115 / 0.490 | 2000 | 15 |
| 0.6 | none | 0.333 / 0.115 / 0.485 | 2000 | 15 |
| 0.7 | none | 0.323 / 0.115 / 0.485 | 2000 | 15 |
| 0.8 | none | 0.333 / 0.115 / 0.485 | 2000 | 15 |
| 0.9 | none | 0.318 / 0.115 / 0.485 | 2000 | 15 |

> **There is no transition at `base* = 0.4427`.** `oscillation_strength` sits at a
> flat ~0.30 median (0.115 floor) across the entire physical efficiency range —
> identical below and above the predicted crossing.

## Does the observed crossing match F's prediction? No — and *why* is the result

**The observable cannot see the bifurcation, for two compounding reasons, both of
which the disagreement localises precisely (Brief F, AC5: a shift localises the
fault to the contact-Michaelis term or the spatial in-reach geometry).**

1. **The spatial in-reach geometry decouples the predator (primary fault).** A
   per-seed trace shows the **consumers go extinct by tick 2000 at *every* swept
   efficiency** (seed 1: 16 producers, 0 consumers, at `base ∈ {0.1, 0.45, 0.8}`).
   The low-mobility consumer (mobility 0.15, feeding reach 2.75) cannot locate a
   prey field spread 8+ units apart, so its **realised** contact rate is far below
   the well-mixed mass-action `a·P·C` the prototype assumes. The predator-prey
   loop never establishes — there is no interior attractor in the spatial world to
   be a node or a cycle, so there is nothing for the Neimark–Sacker crossing to
   switch. This is the well-mixed-vs-spatial gap exactly: the bilinear term
   `B(n,n)` is an integral over an *in-reach indicator* (Brief F, AC2 §4) that the
   mean field replaces with 1.

2. **Finite-N demographic pulsing sets the noise floor (secondary).** The residual
   ~0.30 `oscillation_strength` is **not** trophic — it is the producers' own
   reproduction-cohort pulsing. A producer-only run of the same field scores
   `oscillation_strength ≈ 0.20` with no consumer present at all. This is the
   demographic stochasticity Brief F's **AC3** says the mean field *erases* by
   construction; here it is the dominant signal in the metric, drowning any
   deterministic skeleton. Tellingly, the canonical "predator-prey oscillation"
   scenarios `example5` and `example8` themselves score `oscillation_strength =
   0.000` in `observed.json` — the metric does not light up for the
   frozen↔oscillation mode it nominally measures, so it is not, at this scale, an
   instrument that can adjudicate F's crossing.

A **trilemma** showed up while authoring the scenario and is itself part of the
finding — every minimal regime hits one horn:

- *Sparse prey + low-mobility consumer* → consumer goes locally extinct (geometry
  decouples). ← the committed file.
- *Dense prey + voracious sessile consumer* → one-shot prey annihilation in ~2
  ticks, then predator starvation (collapse, not a cycle).
- *Mobile consumer* (to realise the mixing) → the committed trophic kernel
  `exp(−decay·d)` penalises mobility as biochemical dissimilarity, inflating `d`
  and pushing the predicted crossing **out of (0, 1]** (F then predicts
  unconditional stability) — even though mobility is exactly what the spatial
  coupling needs.

The contact-Michaelis term (`phase.rs:460`) is **not** strongly implicated: the
mismatch survives any reasonable sustained-contact value, because the binding
constraint is encounter (reach), not extraction ramp.

## Verdict

**F's operator reading is internally trustworthy; its observability on the
individual-based stepper at minimal scale is not established.** The spectral
skeleton is self-consistent and verified: a clean closed-form Neimark–Sacker
crossing, analytic = numerical to 4 d.p., a genuine complex-pair crossing. Nothing
in the spike contradicts the *operator-level* claim that the predator-prey
coupling supplied by `resolve_drains` is the Hopf seat. What the spike establishes
is that the **validation triad's "observe" leg is the weak link** for this mode:
the committed `oscillation_strength` metric, on a small-N spatial world, is
dominated by demographic pulsing and by a spatial in-reach geometry that the
mean-field reading deliberately discards. The predicted–observed disagreement is
therefore the **diagnostic signal the brief and `viability.md` prize**, not a
defect in either lens — and it points at the *observable and the geometry*, not at
the spectral reading.

## Go / no-go for move 3 (the moment-closure branching objective)

**Qualified GO — with one gating prerequisite.**

- **Go**, because the spike's actual job was to decide whether the Hopf skeleton is
  trustworthy *as an operator reading* before the larger investment, and on that
  question the answer is yes: the decomposition lands on real phases, the spectrum
  is closed-form and verified, and the only disagreements localise to the two
  effects F's own authority boundary (AC3) already declares out of scope —
  finite-N stochasticity and per-realisation spatial structure. None of that
  undermines using the spectrum as a *deterministic existence/stability skeleton*,
  which is all move 3 needs from it.
- **Gate**, because move 3 would make F's output a genesis *objective* validated
  against headless runs, and this spike shows the current headless observable
  (`oscillation_strength`) cannot adjudicate a stability bifurcation at this scale.
  **Before move 3, harden the observable**: either a cleaner cycle detector
  (spectral-peak / Fourier power at a non-zero frequency, robust to a flat
  baseline) or larger-N / longer ensembles where the deterministic skeleton
  out-signals demographic noise — and a scenario construction that keeps the
  predator coupled (denser prey or a reach that tracks the field) without inflating
  the trait-distance kernel. Equivalently: spend, in `world-rules`, the
  reach/efficiency relationship so a *mobile* forager is not penalised as
  biochemically distant — the trilemma's third horn is a real under-commitment the
  spike surfaced.

Authority boundary held throughout (Brief F, AC3): nothing here claims a per-seed
distributional verdict. The consumer-extinction observation is reported as an
existence/stability fact about *this* deterministic-on-average construction, not as
a distributional claim about decomposer guilds or coexistence-per-seed.

## Reproduce

```
# F's prediction (throwaway prototype — not the production stepper):
cargo run -p explorers-sim --bin hopf_prototype -- scenarios/example10_predator_prey_hopf.json

# the observation (sweep base_trophic_efficiency across 0.4427 and read oscillation_strength):
cargo run -p explorers-genesis-eval --bin eval_scenarios -- --seeds 8 scenarios/example10_predator_prey_hopf.json
```
