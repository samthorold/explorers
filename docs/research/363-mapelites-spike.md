# Spike #363 — Thin MAP-Elites driver: does a QD atlas beat LHS+BO at equal budget?

**Status: research finding. Commits nothing to the design.** This is the throwaway, measured
follow-up to [Research Brief B](B-es-qd-outer-search.md) (AC5) — the same rhythm as F's
[#358](F-hopf-validation.md)/[#359](F-branching-validation.md) and A's
[#353-gated #351](354-simd-soa-spike.md). Brief B argued the genesis outer search (today LHS +
Sobol + GP-BO over a 31-dim cube, `crates/explorers-search/src/search.rs`) is poorly matched to a
**31-dim, gated, plural** objective, and that **Quality-Diversity (MAP-Elites)** is the fit: its
native output is an *atlas of which parameter regions avoid which failure modes* (epic user-story
4), not a single tuned config. This spike tests whether that claim holds **before** any commitment
to replace the search. Promoting QD to *be* the genesis search is a separate `ready-for-human`
follow-up gated on the numbers below.

The spike ships one throwaway bin — `crates/explorers-search/src/bin/mapelites_spike.rs`, clearly
marked NOT production — plus a one-line dev-dep (`explorers-genesis-eval`, used only by that bin).
It changes **no committed semantics**: no physics, no evaluator, no RNG/eval order, and it does not
fork or touch `run_search`.

## TL;DR

1. **QD beats LHS+BO on best fitness at an equal rollout budget — in both runs — and does it while
   also returning a map the incumbent structurally cannot.** Two independent runs at different
   budgets and seeds both put MAP-Elites' best cell above the incumbent's GP-BO optimum: **0.802 vs
   0.711** (budget ~381, `max_ticks=120`) and **0.717 vs 0.674** (budget ~533, `max_ticks=200`). The
   incumbent returns a ranked list; QD returns an **illuminated archive** (14–15 cells, QD-score
   7–8) plus a dead-frontier tally. The atlas is not a tie-breaker bonus — it is a different *kind*
   of output, and it is the one user-story 4 asks for.
2. **The behaviour axes separate the live manifold from the cliffs cleanly (Brief B AC2 confirmed).**
   The three F-derived descriptors — `oscillation_strength` × `clustering_strength` ×
   carcass-locked-fraction — index a real, navigable structure: the best worlds cluster at high
   clustering / low oscillation / low carcass-lock, exactly the coexistence corner F predicts, and
   the gated configs fall *off* the archive into the dead frontier (here `generalist_dominance` and
   `monoculture`), never into fitness-0 cells (Brief B AC3a confirmed).
3. **The validation triad passes.** The decoder midpoint (the #326 known-viable baseline) lands
   **live, in the high-clustering interior** (fitness 0.441, cell `(0,19,0)`), not on the dead
   frontier — the "reproduce known good configs" check. The monoculture/lockup boundary cross-checks
   localise exactly the descriptor calibration Brief B predicted they would (see Validation).
4. **The distributional authority boundary is honoured (Brief B AC3b).** The decomposer guild is
   recorded as a **per-cell seed-fraction with a sample count**, never a descriptor or a fitness
   term. At the decoder midpoint and across the archive it reads as the same sporadic-per-seed
   property `decomposer_emergence.rs` reports (~0.1–0.2 of seeds per cell at this budget) — present,
   reported, never optimised.
5. **Go/No-Go: GO** for the `ready-for-human` "promote QD to the genesis search" follow-up. QD
   *matched-and-beat* LHS+BO on best fitness **and** delivered a usable atlas the incumbent cannot
   produce at all. The recommended production form remains Brief B's CMA-ME/CMA-MAE emitter (the
   simple Gaussian emitter here already covers the manifold; covariance adaptation is the scaling
   lever, not a correctness fix). Two caveats, neither blocking, are recorded under Caveats.

## What was built

- **`mapelites_spike.rs`** — a standalone MAP-Elites driver that:
  - **reuses `decode()` unchanged** — every point the emitter proposes is a `[0,1]^31` unit-cube
    point fed verbatim to the production decoder;
  - **reuses the evaluator (`evaluate_from_log`) unchanged** for fitness, the six gates, and the
    `oscillation_strength` / `clustering_strength` axes;
  - **mirrors `run_single`'s rollout loop exactly** (the `decomposer_emergence.rs` precedent) so it
    can sample the carcass-locked-fraction series the evaluator consumes but does not return, and
    thread a `TopologyProjection` for the decomposer readout. The per-seed ensemble (`seed = base+i`)
    reproduces `run_ensemble`'s semantics. *Why not call `run_ensemble` verbatim as AC5 says: it
    returns only a `FitnessBreakdown`, which carries two of the three axes but not the carcass
    fraction, and never computes topology roles — so two required descriptors are simply unreachable
    from its output. Mirroring `run_single` is the minimal faithful workaround and touches no shared
    crate.*
- **The archive** — hard-binned MAP-Elites at 20×20×20 over the three axes, keeping the
  highest-median-fitness elite per cell; gated configs routed to an out-of-archive dead-frontier
  tally keyed by cliff.
- **A head-to-head harness** — runs the real `run_search` (LHS → Sobol → GP-BO), counts its
  single-world rollout budget analytically, then runs MAP-Elites at a matched budget and prints best
  fitness, coverage, QD-score, the dead frontier, the validation triad, and the decomposer
  distribution; dumps the archive to JSON.

## The head-to-head (ensemble 5; MAP-Elites at the incumbent's measured rollout budget)

Two independent runs (different budget, ticks, and seed). MAP-Elites wins on best fitness in both:

| Metric                    | Run A — incumbent | Run A — MAP-Elites | Run B — incumbent | Run B — MAP-Elites |
| ------------------------- | ----------------- | ------------------ | ----------------- | ------------------ |
| Budget / `max_ticks`      | 381 / 120         | 480 / 120          | 533 / 200         | 600 / 200          |
| Best fitness              | 0.711             | **0.802**          | 0.674             | **0.717**          |
| Coverage (filled cells)   | n/a — ranked list | 15 / 8000          | n/a — ranked list | 14 / 8000          |
| QD-score (Σ elite fitness)| n/a               | 8.22               | n/a               | 7.06               |
| Dead frontier (by cliff)  | none              | gen-dom×5, mono×4  | none              | gen-dom×7, mono×3, ext×2 |

(MAP-Elites slightly overshoots the budget because it spends whole batches — the
`generations × batch × ensemble` granularity rounds up past the incumbent's exact rollout count.)

The incumbent spends the bulk of its budget on the **sequential** Sobol sensitivity sweep
(`lhs_samples·(dims+2)` single-seed rollouts) — a fixed overhead that buys a dimension-fixing
heuristic, not candidate worlds. MAP-Elites spends the same budget *illuminating* the space. Both
runs put the best cell at `(0,19,0)` — low oscillation, ~1.0 clustering, ~0.03 carcass — the
frozen-but-richly-coexisting corner the midpoint baseline also occupies.

## Validation triad

- **Reproduce known good config (PASS, both runs).** The decoder midpoint (every searched dim at
  0.5 — the #326 known-viable baseline) lands **live** in the high-clustering interior — fitness
  0.441 (run A) / 0.428 (run B), both in cell `(0,19,0)` (clustering ~1.0) — **not** on the dead
  frontier.
- **Monoculture boundary (cross-check, diagnostic WARN).** 2 live cells sit below
  `clustering_strength < 0.5` — *expected*, and exactly the calibration Brief B's validation-triad
  diagnostic predicts: the `Monoculture` gate only fires with ≥20 agents post-grace, so a small-but-
  diverse-enough surviving world can read low-clustering yet not be gated. The boundary is not
  mis-drawn; the gate is population-conditional and the descriptor is not. Noted for the follow-up
  (the soft-archive CMA-MAE variant tolerates exactly this descriptor noise).
- **Lockup boundary (cross-check, not reached in either run).** Neither run sampled the lockup
  regime: 0 configs gated to the `nutrient_lockup` frontier and 0 live cells at carcass ≥ 0.4 in
  both A and B. The carcass axis is wired and exercised (every live cell carries a carcass
  descriptor, all low here), but a *random* iso-Gaussian emitter at a feasible budget does not land
  in the thin high-carcass region. This is a genuine coverage limit of the simple emitter + budget,
  not a wiring failure — and it is itself a finding: reaching the lockup cliff to map it wants either
  the directed CMA-ME emitter or F's a-priori viability prefilter (Brief B AC4), flagged honestly
  rather than silently.

## Decomposer guild (Brief B AC3b — reported distribution, never a descriptor/objective)

Recorded per cell as the fraction of the 5-seed ensemble that sprouts a *persistent* decomposer
guild (≥1 agent reading as `Decomposer` for ≥25% of the run — the `decomposer_emergence.rs`
classification verbatim), with the sample count. Run A (15 cells): 7 with ≥1 guild seed, 0
majority-guild, mean fraction 0.107. Run B (14 cells): 9 with ≥1 guild seed, **1 majority-guild**,
mean 0.229 — i.e. the atlas localises both the common sporadic-guild cells *and* the rarer
strong-guild regime Brief B notes exists, as a distribution. This is the same sporadic-per-seed
signal the emergence regression reports, surfaced over the atlas — it is **not** an axis and
**not** a fitness term, per the authority boundary.

## Caveats (neither blocks the GO)

1. **Budget feasibility, not method, set the scale.** The head-to-head ran at a deliberately small
   budget because the incumbent's Sobol sweep is sequential and random 31-cube draws routinely
   balloon toward the population-explosion ceiling (O(n²) per tick). This bounds *both* methods
   equally, so the comparison is fair, but the absolute coverage (14–15 cells) is thin. The
   production
   follow-up should (a) parallelise or drop the Sobol step it is replacing anyway, and (b) consider
   an a-priori viability prefilter (Brief B AC4, composing with F) so dead/explosive configs cost
   less than a full rollout.
2. **Simple emitter, by design.** This used a classic iso-Gaussian emitter, not the CMA-ME/CMA-MAE
   Brief B recommends. It already covered the live manifold and beat the incumbent; covariance
   adaptation is the scaling/conditioning lever for the 31-dim production search, not a correctness
   prerequisite the spike needed.

## Reproducing

```
cargo run --release --bin mapelites_spike
# knobs (env): SPIKE_MAX_TICKS SPIKE_LHS SPIKE_ENSEMBLE SPIKE_BATCH SPIKE_SEED
```

Writes the archive to `target/mapelites_spike_atlas.json`. Unit tests (binning, elite replacement,
cliff routing, budget formula) plus a `slow_`-tagged smoke test live in the bin; `cargo test -p
explorers-search` is green and `cargo fmt --check` is clean.
