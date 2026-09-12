# Research #434 — Confidence bounds on ensemble verdicts (Hoeffding / Clopper–Pearson / SPRT)

**Status: research finding. Commits nothing to the design.** This note turns the seed-ensemble
verdicts — the scenario suite's `n = 8` (`eval_scenarios --seeds`, #314), the search's in-run `n = 5`
(`SearchConfig::ensemble_size`), and the projection's gated elite refinement at `n = 32`
(`REFINE_ENSEMBLE_SIZE`, #404) — into *statistical model checking with explicit error bounds*: what a
fraction over `n` seeds actually tells us about the true fraction, how large `n` must be for the
claims the design already makes, and whether a sequential probability ratio test (SPRT) should replace
the fixed block. Parent: #430 (formal viability). Every number below is exact (binomial enumeration /
exact Clopper–Pearson via bisection / exact truncated-SPRT dynamic programme); the script is trivial
and reproducible from the formulas quoted.

## TL;DR

1. **At `n = 8` the ensemble says almost nothing about a mid-range fraction.** The exact 95 %
   Clopper–Pearson interval on `coexistence_fraction` is **±0.34 wide at its narrowest and 0.69 wide
   at `k = 4`** (`4/8 → [0.16, 0.84]`). Hoeffding is vacuous (`±0.48`). What `n = 8` *can* say is
   one-sided and extreme: **`8/8` ⇒ `p ≥ 0.63`** (two-sided) / `p ≥ 0.69` (one-sided) at 95 %, which is
   exactly the shape of claim the scenario verdicts make ("all eight seeds agree").
2. **The projection's floor test does not need an interval; it needs a separation.** The floor
   (`COEXISTENCE_FLOOR = 0.5`) is a decision *"is this cell a straddler or robust?"* The refinement's
   `n = 32` separates `p = 0.35` from `p = 0.65` at **α = β ≈ 5 %** (fixed-`n` optimum is `n = 29`),
   which is precisely the #401 straddler (~3/8 ≈ 0.375) versus a cell that coexists on most seeds. It
   does **not** separate 0.4 from 0.6 (`n = 67` needed) and never will separate 0.45 from 0.55 at any
   sane budget (`n = 269`). The two-sided CP interval at `16/32` is `[0.32, 0.68]`.
3. **The modal failure mode at `n = 8` is unreliable unless the mode is dominant.** For a true
   split 0.7/0.3 the observed mode is *wrong or tied* 19 % of the time; for 0.6/0.4, 41 %. Only a
   ≥ 0.9 mode is safe (0.5 %). The suite's current `8/8` unanimity is therefore evidence of a *dominant*
   mode (`p ≥ 0.63`), not proof of a deterministic one.
4. **SPRT: evaluated, not recommended.** A Wald SPRT for `p0 = 0.35` vs `p1 = 0.65`, α = β = 0.05,
   capped at 32, would cut the *mean* refinement draw from 32 to 6–20 seeds — but on the straddler
   band it actually matters for (`p ∈ [0.4, 0.6]`) it saves only 5–9 seeds per cell, and with the
   8-wide parallel seed block the search already runs, it saves *one to two batches*. Refinement is
   ~15 % of a default search's rollouts; SPRT would trim that to ~8 % while introducing a biased
   stopped-sequence estimate into the audit trail, a per-cell variable `refined_sample_count`, and a
   second stopping rule to reason about. **Recommendation: keep the fixed block.** No code shipped.
5. **Concrete recommendations:** (a) keep `REFINE_ENSEMBLE_SIZE = 32` and document it as *the
   0.35-vs-0.65 separator*, not a "tight estimate"; raise to 64 only if the design ever needs
   0.4-vs-0.6; (b) cite the `8/8 ⇒ p ≥ 0.63` bound in the scenario verdicts instead of "eight seeds";
   (c) when `observed.json` is next regenerated, run the suite at `--seeds 32` (`32/32 ⇒ p ≥ 0.89`)
   — a one-flag change, no code; (d) treat the in-run `n = 5` as a ranking heuristic, which the
   design already does — no bound is claimed on it.

## 1. What the ensembles claim

Three places reduce a seed block to a fraction and read that fraction as a verdict:

| site | `n` | statistic | how it is read |
|---|---|---|---|
| `eval_scenarios` → `verdicts.md` (#314) | 8 (`--seeds`, default) | modal failure mode; fraction of seeds matching the prediction | supermajority read, "all eight agree" |
| `run_qd` in-run ensemble (`SearchConfig::ensemble_size`) | 5 | `coexistence_fraction` (`is_coexisting` per seed), median-seed fitness | ranking key + descriptor; the pre-#404 floor read this |
| gated elite refinement (`refined_best_recipe`, #404) | 32 (`REFINE_ENSEMBLE_SIZE`) | `refined_coexistence_fraction ≥ COEXISTENCE_FLOOR = 0.5` | *the* pick gate — binary decision |

All three are a binomial proportion `k/n` with `p` the true per-seed probability under the config,
because seeds are independent draws (`base_seed.wrapping_add(i)`, disjoint RNG streams — #350). The
question "how confident is the verdict?" is therefore exactly a binomial-interval question.

## 2. Intervals on a fraction at `n = 8`

**Hoeffding** (distribution-free): with probability ≥ 1 − α, `|k/n − p| ≤ √(ln(2/α) / 2n)`.
At α = 0.05:

| `n` | 5 | 8 | 16 | 32 | 64 | 128 | 185 |
|---|---|---|---|---|---|---|---|
| half-width | ±0.61 | **±0.48** | ±0.34 | ±0.24 | ±0.17 | ±0.12 | ±0.10 |

At `n = 8` the Hoeffding interval is `[k/8 − 0.48, k/8 + 0.48]` — wider than `[0, 1]` for every `k`
from 1 to 7 after clipping. It is the wrong tool for a small block; it is quoted because it is the
bound the issue names and because it is the one that generalises to bounded non-binary statistics
(the score medians), where it is equally vacuous at `n = 8`.

**Clopper–Pearson** (exact, two-sided 95 %), `n = 8`, all `k`:

| `k` | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|---|
| `k/8` | 0.000 | 0.125 | 0.250 | 0.375 | 0.500 | 0.625 | 0.750 | 0.875 | 1.000 |
| lower | 0.000 | 0.003 | 0.032 | 0.085 | 0.157 | 0.245 | 0.349 | 0.473 | **0.631** |
| upper | **0.369** | 0.527 | 0.651 | 0.755 | 0.843 | 0.915 | 0.968 | 0.997 | 1.000 |
| width | 0.37 | 0.52 | 0.62 | 0.67 | **0.69** | 0.67 | 0.62 | 0.52 | 0.37 |

Plain reading: an observed `4/8` is consistent with anything from a 16 % to an 84 % coexistence
rate. The #401 straddler's independent re-evaluation at ~`3/8` was consistent with `p ∈ [0.09, 0.76]`
— the projection was right to distrust it, but `n = 8` could not have *cleared* it either.

**The one claim `n = 8` supports is unanimity.** `8/8` gives a 95 % two-sided lower bound of
`p ≥ 0.63`; the one-sided bound (the natural one for "the mode is dominant") is `p ≥ 0.05^(1/8) =
0.69`. To push a unanimous block to a stronger claim:

| unanimous `k = n` implies (one-sided 95 %) | `p ≥ 0.69` | `p ≥ 0.80` | `p ≥ 0.90` | `p ≥ 0.95` |
|---|---|---|---|---|
| `n` required | 8 | 14 | 29 | 59 |

For reference the in-run `n = 5` is worse still: `5/5 ⇒ p ≥ 0.48` two-sided (`0.55` one-sided), and
`3/5 = 0.6` — the value the #401 leader showed in-run — has CP `[0.15, 0.95]`.

## 3. The modal failure mode as a multinomial

The verdicts table cites the *modal* failure mode (`n/8`). The right question is: with what
probability is the observed mode not the true mode? Exact enumeration of the multinomial, counting
ties as failures (a tie is not a read):

| true mode distribution | `n = 8` | `n = 32` |
|---|---|---|
| 0.9 / 0.1 | **0.005** | 0.000 |
| 0.7 / 0.3 | **0.19** | 0.014 |
| 0.6 / 0.4 | **0.41** | 0.17 |
| 0.5 / 0.3 / 0.2 | 0.41 | 0.13 |
| 0.5 / 0.25 / 0.25 | 0.40 | 0.10 |
| 0.4 / 0.35 / 0.25 | 0.63 | 0.46 |
| 0.34 / 0.33 / 0.33 | 0.75 | 0.69 |

So a modal read at `n = 8` is trustworthy only when the mode is dominant (≥ 0.9); at a 0.7/0.3 split
one verdict in five would be wrong. The current suite is unanimous on every scenario, which by §2 puts
each true mode at `≥ 0.63` — consistent with a dominant mode but not proving one. A scenario that ever
shows a `5/8` or `6/8` mode should be read as *undecided*, not as a supermajority; the verdicts header
now says so.

## 4. What the projection's gate actually relies on

The floor is a *decision*, `k/n ≥ 0.5`, not an estimate. Its operating characteristic — the
probability a cell with true rate `p` clears — for the fixed rule at each `n`:

| true `p` | 0.2 | 0.3 | 0.35 | 0.4 | 0.45 | 0.5 | 0.55 | 0.6 | 0.65 | 0.7 | 0.8 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `n = 5` (pre-#404 in-run) | 0.06 | 0.16 | 0.24 | 0.32 | 0.41 | 0.50 | 0.59 | 0.68 | 0.76 | 0.84 | 0.94 |
| `n = 8` | 0.06 | 0.19 | 0.29 | 0.41 | 0.52 | 0.64 | 0.74 | 0.83 | 0.89 | 0.94 | 0.99 |
| `n = 32` (refinement) | 0.00 | 0.01 | **0.06** | 0.16 | 0.35 | 0.57 | 0.77 | 0.91 | **0.97** | 0.99 | 1.00 |
| `n = 64` | 0.00 | 0.00 | 0.01 | 0.07 | 0.25 | 0.55 | 0.82 | 0.96 | 1.00 | 1.00 | 1.00 |

Read the `n = 5` row against #401: a straddler at `p = 0.35` cleared the in-run floor one time in
four, and a robust `p = 0.65` cell *failed* it one time in four. That is the failure the refinement
was built to close, and at `n = 32` those become 6 % and 3 %.

**Required `n`, stated as a separation problem.** The smallest fixed `n` (with its optimal
threshold `c`, "clear iff `k ≥ c`") that distinguishes a straddler at `p0` from a robust cell at `p1`
with α = β = 0.05:

| `p0` vs `p1` | 0.25 / 0.75 | 0.30 / 0.70 | **0.35 / 0.65** | 0.40 / 0.60 | 0.45 / 0.55 |
|---|---|---|---|---|---|
| `n` (threshold `c`) | 9 (5) | 17 (9) | **29 (15)** | 67 (34) | 269 (135) |

`REFINE_ENSEMBLE_SIZE = 32` is therefore *the 0.35-vs-0.65 separator*, sitting just above the
`n = 29` optimum — a good match for the #401 straddler (~0.375 on the independent draw) against a
cell that coexists on two seeds in three. Its doc comment calls the refined fraction a "tight
estimate near the bifurcation"; it is not an estimate at all in the interval sense (`16/32 →
[0.32, 0.68]`), and the design should not lean on it as one. If the design ever needs to gate on a
0.4-vs-0.6 distinction it needs `n = 64`; 0.45-vs-0.55 is out of reach and should not be attempted —
a cell that close to the floor *is* a straddler for the purpose of the projection, and the honest
handling is the existing argmax fallback with a warning.

**Required `n` for interval widths, for completeness** (95 %, at the worst case `k/n = 0.5`):

| CP two-sided width | 0.36 | 0.30 | 0.26 | 0.21 | 0.18 | 0.15 | 0.10 |
|---|---|---|---|---|---|---|---|
| `n` | 32 | 48 | 64 | 96 | 128 | 192 | 384 |
| Hoeffding half-width | ±0.30 | ±0.25 | ±0.20 | ±0.15 | ±0.10 | ±0.05 | |
| `n` | 21 | 30 | 47 | 82 | 185 | 738 | |

## 5. SPRT — the adaptive alternative

The gated refinement approximates a sequential test: a small in-run draw, then a larger independent
draw only for the top-K. Wald's SPRT is the exact version: test `H0: p = p0` vs `H1: p = p1`, after
each seed update `Z = k·ln(p1/p0) + (n−k)·ln((1−p1)/(1−p0))`, stop and *clear* when `Z ≥ ln((1−β)/α)`,
stop and *fail* when `Z ≤ ln(β/(1−α))`, else draw another seed; cap at `n_max` and fall back to the
fixed rule `k/n ≥ 0.5`. For `p0 = 0.35`, `p1 = 0.65`, α = β = 0.05 the log-likelihood step is
`±0.619` per seed and the boundaries are `±2.944`, i.e. **stop when successes lead failures by 5 (or
trail by 5)**. Exact operating characteristic and average sample number (ASN) with the cap at 32,
computed by dynamic programme over `(k, n−k)`:

| true `p` | 0.1 | 0.2 | 0.3 | 0.35 | 0.4 | 0.5 | 0.6 | 0.65 | 0.7 | 0.8 | 0.9 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| seed-at-a-time: E[n] | 6.2 | 8.3 | 12.0 | 14.5 | 17.1 | 19.8 | 17.1 | 14.5 | 12.0 | 8.3 | 6.2 |
| seed-at-a-time: P(clear) | 0.00 | 0.00 | 0.02 | 0.07 | 0.17 | 0.54 | 0.87 | 0.95 | 0.98 | 1.00 | 1.00 |
| batches of 8: E[n] | 9.5 | 12.7 | 17.6 | 20.7 | 23.6 | 26.5 | 23.6 | 20.7 | 17.6 | 12.7 | 9.5 |
| batches of 8: P(clear) | 0.00 | 0.00 | 0.02 | 0.06 | 0.16 | 0.56 | 0.90 | 0.97 | 0.99 | 1.00 | 1.00 |

(Compare the fixed `n = 32` row of §4: the error rates are the same to within a point — the cap
makes the truncated SPRT's OC essentially the fixed rule's — so the *only* thing SPRT buys is the
smaller `E[n]`.)

**Why the batch row is the relevant one.** `run_ensemble` runs its seed block in parallel
(`into_par_iter`, #350). A seed-at-a-time SPRT serialises the block and forfeits that; the honest
comparison is a batch-sequential SPRT that draws one parallel batch (8 seeds on an 8-wide machine),
tests, and draws again. On that row the saving against 32 is:

- clear-cut cells (`p ≤ 0.2` or `≥ 0.8`): 32 → ~10–13 seeds, i.e. **3 batches saved of 4**;
- the band the gate exists for (`p ∈ [0.4, 0.6]`): 32 → 24–27 seeds, i.e. **≤ 1 batch saved**.

**Cost context.** A default search (`--batch 32 --generations 10 --ensemble 5`) runs
`32 × 11 × 5 = 1760` search rollouts and `10 × 32 = 320` refinement rollouts — refinement is ~15 % of
the total. If every top-K cell were clear-cut, SPRT would take refinement to ~110 rollouts (~6 % of
total); on a leaderboard of straddlers it would take it to ~260 (~13 %). The realistic saving is
**5–10 % of a search's rollouts**.

**What it would cost in return.**

1. *A biased estimate in the audit trail.* The fraction from an optionally-stopped sequence is
   biased away from the boundary (stopping early on a run of successes overstates `p`);
   `RefinedCell::refined_coexistence_fraction` would no longer be an unbiased proportion, and the
   `recorded` vs `refined` comparison the projection reports for auditability would compare unlike
   quantities.
2. *A variable `refined_sample_count` per cell* — the audit trail currently reads "32 seeds, k
   coexisted"; it would read "stopped at 16 with a lead of 5", which is a harder read and a second
   thing to explain in `genesis-search.md`.
3. *Seed-block determinism.* It stays fixed and contiguous (the SPRT consumes the same
   `base + offset + rank·n_max` block, in order, and simply stops early), so the determinism
   invariant holds — but the rank-`r` block would need to be reserved at `n_max` width regardless,
   so there is no seed-layout simplification either.
4. *A second stopping rule* (SPRT boundary + cap + fallback) living beside the fixed block behind an
   `Option` on `EnsembleConfig`, default-off, and a byte-identity test to prove the default path is
   untouched — real code and a real test surface for a single-digit-percent budget win.

**Verdict: not worth it at this budget.** Sequential testing earns its complexity when the per-sample
cost is high *and* most decisions are clear-cut *and* the decision is the only output. Here the cap
dominates (`n_max = 32` is already close to the fixed optimum of 29), the decisions that matter are
near the boundary where SPRT saves least, the per-cell fraction is reported for audit and must stay an
unbiased proportion, and the whole refinement stage is 15 % of the run. If the top-K ever grows to
the point that refinement dominates the search — say `REFINE_TOP_K` of 50+ — revisit; the numbers
above are the ones to plug in. **Step 3 of the issue (implementation) is deliberately skipped.**

## 6. Recommendation

**Fixed `n`. No SPRT.** Specifically:

| site | today | recommendation | why |
|---|---|---|---|
| refinement `REFINE_ENSEMBLE_SIZE` | 32 | **keep 32**; re-describe it as the 0.35-vs-0.65 separator at α = β ≈ 5 % (doc comment updated in this PR) | `n = 29` is the fixed optimum for that separation; the design's straddler is ~0.375 |
| scenario suite `--seeds` | 8 | **cite the bound** (`8/8 ⇒ p ≥ 0.63`) now; run at `--seeds 32` (`32/32 ⇒ p ≥ 0.89`) at the next `observed.json` regeneration | the suite is cheap; a mode read at `n = 8` is only safe for a ≥ 0.9 mode; regenerating the artifact triggers a re-judge and is out of scope here |
| in-run `ensemble_size` | 5 | **leave** | it ranks and bins; no confidence claim is made on it and the refinement exists precisely because none can be |
| `COEXISTENCE_FLOOR` | 0.5 | leave | the gate's OC at `n = 32` is what §4 tabulates; a floor at 0.5 with a 0.35/0.65 separator is coherent |

Nothing in the stepper or the evaluator's scoring changes; nothing in `run_ensemble` changes. The
one code touch is a doc comment on `REFINE_ENSEMBLE_SIZE` so the constant's justification matches
the arithmetic.

## Appendix — formulas

- Hoeffding: `P(|k/n − p| ≥ ε) ≤ 2·exp(−2nε²)` ⇒ `ε = √(ln(2/α)/(2n))`, `n = ln(2/α)/(2ε²)`.
- Clopper–Pearson (two-sided 1 − α): lower `= inf{p : P(Bin(n,p) ≥ k) > α/2}`, upper
  `= sup{p : P(Bin(n,p) ≤ k) > α/2}`; `k = n` lower bound `= (α/2)^(1/n)`, one-sided `α^(1/n)`.
  Computed here by bisection on the exact binomial CDF — no beta-quantile dependency needed.
- Fixed-`n` separation: smallest `n` with a threshold `c` such that `P(Bin(n,p0) ≥ c) ≤ α` and
  `P(Bin(n,p1) < c) ≤ β`.
- Wald SPRT boundaries `A = ln((1−β)/α)`, `B = ln(β/(1−α))`; per-seed log-LR `ln(p1/p0)` on
  success, `ln((1−p1)/(1−p0))` on failure; truncated at `n_max` with the fixed rule as fallback.
  OC and ASN by exact forward recursion over `(successes, failures)` states, batch width 1 or 8.
