# Issue #439 — formal viability A3: cross-check of the A1/A2 permanence predictions against the atlas and seed ensembles

**Status: research finding, re-taken under #476. Commits nothing.** This note asks
whether the closed-form permanence predictions of [`432-permanence-pc.md`](432-permanence-pc.md)
(A1) and [`437-permanence-alc.md`](437-permanence-alc.md) (A2) agree with what the
committed simulation does, on every atlas live cell and a low-discrepancy sample of the
search box, over a fixed seed ensemble. It presents the confusion matrix predicted ×
observed, the false-positive rate (predicted permanent, observed collapse — the
dangerous direction for a certificate), a per-cell list of every disagreement with a
one-line fault hypothesis, and a verdict under A4's rule. It introduces no mechanism,
functional form, or parameter; it touches nothing in the stepper, the evaluator's
scoring, or the search.

The instrument is `crates/explorers-search/src/bin/permanence_crosscheck.rs` — a
diagnostic bin patterned on `energy_bound_check.rs` / `role_emergence.rs`, not a CI gate.
Its artifact is `target/permanence-crosscheck.json` (gitignored).

## The `χ_C` correction (#482) — landed after this run

The `β` row has since been corrected: `β = χ_C·γ·e·a` with the consumer's biomass
conversion `χ_C = (κ_C + (1 − κ_C)·η_C·s) / (1 − (1 − κ_C)·η_C·(1 − s))` — the twin of
`χ_P`, derived in [`432-permanence-pc.md`](432-permanence-pc.md) — and A2's `Λ` carries
`χ_C·γ·e_C` in its Liebig branch. The bin's `#482 (beta lumping)` fault hypothesis is
gone (`κ_C = 0` no longer zeroes `β`; the only way `β = 0` with a feeding consumer is the
degenerate `χ_C = 0` corner, which now gets the same hypothesis as `χ_P = 0`), and the
example10 pins moved to `β = 0.04095`, `I = 25.77`, `Λ = 90 584`. **The numbers, tables
and appendix below are the pre-#482 run** and still read `β = κ_C·γ·e·a`; the 19 cells
tagged `#482` are expected to become clause-(2) *passes* on the re-run, which is a
separate follow-up (each run is ~20–30 min).

## Change from the previous run

The numbers here were re-taken on the stepper after the six post-atlas fixes
(#444–#453, last at `5a7bede`), on the atlas #474 regenerated on that stepper (82 live
cells, was 56), and with the `r_P` row corrected per #466 (`r_P = χ_P·γ·(F − B_P)`, the
biomass conversion `χ_P` counting both of `κ_P`'s branches). The 200 `sample:i` configs
are the same seed-421 draw, so those labels stay comparable; the `atlas:i` cells are
new. What moved: (i) **A1's one strict false positive is gone** — `sample:12`, extinct
on 8/8 seeds at tick 1 before, now keeps one seed alive to the horizon (7/8), so it is a
mixed cell and A1's strict rate is **0/50** (was 1/44). Every founder still dies on
tick 1 on the seven collapsing seeds, so its fault class is unchanged; the eighth seed
is the fixed stepper's doing, and #485 records that the overdraft cap (#445) alone moves
trajectories, so no single fix is credited. (ii) **A2's strict false positives changed
identity, not kind**: `sample:147` (7/8 lockup before) is now 3/8 lockup and 5/8
monoculture, while `sample:154` (6/8 before) and `sample:192` (7/8 before) now lock up
on 8/8 — three cells of the same fault, the reference lumping's `ι = 1` over-serving the
pile, reshuffled by a seed or two; A2's strict rate is 2/176 (was 2/163). (iii) **The
clause-(1) false negatives are gone and the same cells reappear under clause (2)**: the
`mean_kappa = 0` atlas cells (19 now, 10 before) all clear `ρ > 1` with `r_P` from 0.03
to 0.56, and all predict not-permanent through `I = 0`, because `β = κ_C·γ·e·a` still
carries the lumping #466 removed from `r_P` — filed as #482 and tagged as such in the
disagreement list. (iv) Collapses fell from 301/2048 runs (14.7 %) to 197/2256 (8.7 %),
extinctions from 249 to 76, and the any-seed false-positive rates roughly halved (A1
52 % → 26 %, A2 50 % → 28 %) — the founder floor keeps the whole roster alive past
tick 1, and #474 reads the same rotation off the dead frontier. (v) The #444 tag and
its "excluding tagged cells" denominator are removed from the instrument and this note:
`World::new` floors every founder trait at zero, so no cell can owe a collapse to the
cull (0 of 66 800 founders carry a negative trait). The atlas cells collapse on no seed
in 64/82 and on some seed in 18/82; none collapses 8/8.

## TL;DR

1. **Coverage.** 82 atlas live cells + 200 LHS configs (the seed-421 draw shared with
   `role_emergence` and `energy_bound_check`, so `sample:i` names the same config
   everywhere) × 8 fixed seeds × the 500-tick search horizon = 2256 runs. Two full runs
   produce a byte-identical artifact.
2. **The dangerous direction is empty for A1 and nearly empty for A2 — on the strict
   read.** A1 predicts *permanent* on 50 cells; **none collapses on 8/8 seeds** (strict
   FP 0/50). A2 at its reference lumping predicts *permanent* on 176 cells; **2** collapse
   8/8 (`sample:154`, `sample:192` — both nutrient lockup on every seed with `Λ = 120`
   and `53` at `ι = 1`; strict FP 2/176).
3. **But a quarter of the predicted-permanent cells collapse on *some* seed** (A1: 13/50;
   A2: 50/176). Those are the "no per-seed claim" of the authority boundary made visible:
   1–2 of 8 seeds die by demographic chance (A1 7 cells, A2 32), consumers are lost
   within 25 ticks while producers stand — the example10 signature — (3 / 10), the world
   silts into lockup on a minority of seeds (2 / 5), or every founder dies on tick 1
   (`sample:12`, 1 / 1).
4. **The negative direction — the one a gate would use — is contradicted for clause (2).**
   A1 predicts *not-permanent* on 197 cells, all through `I ≤ 1` (clause (1) fails on
   none), and 195 of them persist on at least one seed (139 on all eight). After
   crediting the 15 cells whose persisting seeds hold no consumer at all (the
   producer-only face, which is what a failed clause 2 predicts), 180 disagreements
   remain: **18 are the #482 lumping** (`κ_C = 0` zeroes `β`, so `I = Λ = 0`), and of the
   rest, 143 carry a **decomposer guild** on their persisting seeds. The consumer that
   cannot invade the standing crop (`I ≤ 1`; median `I` over the box is 0.26) persists
   on the **carcass pile** — the route A2 adds (`Λ > 1`; median `Λ` at the reference
   lumping is 600) and A1's two-compartment map lacks.
5. **Clause (1), the sharpened extinction gate, is untested by the box and uncontradicted.**
   `ρ = F/B_P ≥ 1.9` on every config, so no cell exercises `ρ ≤ 1`; with the #466
   correction no cell fails it either (the ten clause-(1) false negatives of the previous
   run were the lumping, not the theorem, and they are gone).
6. **Verdict under A4's rule, per predicate.** (a) A1 clause (1) `ρ > 1`: zero false
   positives and zero contradictions, but the box never exercises it and its centroid
   form is not a config-only kill (a founder or mutant with less autotrophy has a lower
   `B_P`), so as a gate it reduces to the standing `F ≤ B` — nothing to wire. (b) A1
   clause (2) `I > 1`: the positive direction now meets A4's zero-strict-FP rule (0/50),
   but a prefilter runs the *negative* direction and `I ≤ 1` kills 2 of the 197 cells it
   flags — it would exclude 60 of the 82 atlas live cells. **Characterisation.** (c) A2
   `Λ > 1` at the reference lumping: two strict failures (`sample:154`, `sample:192`), the
   endogenous reach `ι` biting exactly as A2 said it would. **Characterisation, citing
   those cells.** No predicate is gate-grade with all live cells passing; #465 stays
   deferred, now behind #482 as well as the reach bound and carcass floor.

## Reading list this note assumes

A1's map, the ratios `ρ = F/B_P`, `I = β·K_P/m`, the biomass conversion `χ_P` (#466),
hypotheses H1–H2, and the authority boundary; A2's coupled reduction, `Λ`, clauses
(i)–(iii), and its list of endogenous terms (`ι`, `ν`, `q`, `μ_P`);
[`434-ensemble-confidence.md`](434-ensemble-confidence.md) for what `n = 8` can say;
`scenarios/verdicts.md` for the supermajority read; #482 for the consumer-side lumping
(`β = κ_C·γ·e·a`) this run exposes; [`474-atlas-regen-post-fix.md`](474-atlas-regen-post-fix.md)
for the atlas the cells come from; [`433-energy-bound.md`](433-energy-bound.md) § B2 for
the config sourcing this instrument copies.

## Design

### What is predicted, and from what

The predictions are evaluated from the **config alone** — the decoded `WorldParameters`
and `InitialDistribution` — never from the realised population.

- **Representative trait vectors.** A1's coefficients need a producer and a consumer trait
  vector. The instrument uses the **cluster centroids `World::new` seeds**: for
  `initial_cluster_count ≥ 2` and a positive trophic total, a pure producer
  `(α+h, 0, μ, κ, …)` and a pure consumer `(0, α+h, μ, κ, …)` sharing every other
  dimension of the mean (the alternating-cluster rule in `World::new`); for a single
  cluster both are the mean itself — a mixotroph cloud with no distinct consumer
  compartment (30 of 282 configs). `B_P`, `B_C`, `h_C`, `κ_P`, `κ_C`, `θ_P`, `θ_C`, `α_P`
  and the pair's distance `d` are read at those vectors. Founder noise (`trait_covariance`)
  and everything evolution does afterwards are outside the prediction, deliberately.
- **A1** (`predicted_a1`): `permanent` iff `r_P > 0` (clause 1, `ρ > 1`) and `β·K_P > m`
  (clause 2, `I > 1`) and H1 (`0 < m < 1`, `r_P ≤ 2`); `not-permanent` iff either clause
  fails (both are necessary, and the `r_P > 2` caveat keeps clause 2 necessary);
  `undecided` iff both clauses hold but H1 fails (35 cells, all `r_P > 2`, where the
  sufficiency proof does not apply). The coefficient mapping is copied term for term from
  `permanence_prototype.rs` (a bin cannot be imported), with `r_P = χ_P·γ·(F − B_P)` per
  #466, and pinned to that bin's example10 numbers (`ρ = 71.11`, `I = 23.13`,
  `r_P = 1.348`, `m = 0.113`, `β = 0.0368`) by unit test.
- **A2** (`predicted_a2`): the coupled reduction's clauses (i) `min(r_P, α_P/θ_P) > μ_P`,
  (ii) `Λ > 1`, (iii) the heteroclinic product, evaluated at the **reference lumping A2's
  own sweep uses** — `ι = 1`, `q = ν = θ_P` (producer carcasses, no free store),
  `μ_P = 0.02`, `e_C = e` — and pinned to example10 (`Λ = 81 334`). `not-permanent` only
  where a clause fails **for every value of the endogenous terms**:
  `min(r_P, α_P/θ_P) ≤ 0` (no `μ_P ≥ 0` rescues it) or `Λ = 0` (no heterotrophy, or
  `κ_C = 0` — the #482 lumping). `undecided` where a clause fails only at the reference,
  or H1 fails. Note that **A2's theorem does not contain A1's clause (2)**: in the coupled
  map the producer-only face flows into the lockup corner, so the consumer's escape route
  is the pile, not the standing crop. The two columns are therefore different predicates,
  not a nested pair.

### What is observed

Each (config, seed) is driven through the genesis step loop exactly as
`explorers_genesis::run_single` does — same per-tick observations, same early stops
(empty world, `> max_population`), same `evaluate_from_log` — and the evaluator's terminal
failure mode is the run's outcome. **Observed collapse** = `extinction`, `energy-death` or
`nutrient-lockup` (the extinction boundary attracted, on the energy or the nutrient side);
`none`, `monoculture`, `generalist-dominance` and `explosion` have a living population
bounded away from zero at the horizon and are read as **persist**. Over the 2256 runs the
modes are: none 1894, monoculture 157, nutrient-lockup 120, extinction 76,
generalist-dominance 8, energy-death 1, explosion 0 — 197 collapses, 8.7 %.

Per config the ensemble reduces to the **supermajority read** of `scenarios/verdicts.md`:
`collapse` when 8/8 seeds collapse, `persist` when 0/8 do, `mixed` otherwise (with the
fraction kept). Per #434, `8/8` bounds the true per-seed rate at `p ≥ 0.63`; a `1/8` or
`2/8` split says only that the rate is not zero — at `k = 1`, the exact 95 % interval is
`[0.003, 0.53]`. The instrument also records, per seed, the producer/consumer split of the
roster after tick 1 and at the horizon (`α ≥ h` is a producer, the prototype's rule), the
evaluator's `has_decomposer_guild`, the first tick without a consumer while producers
stood, and the peak population. Founder traits are floored at zero by `World::new`
(#444), so there is no negative-founder count and no compartment-dead-on-arrival tag:
a population of 0 after tick 1 is the peak-relative death threshold's doing, not a cull.

### How prediction and observation are related

| predicted | observed | class |
|---|---|---|
| permanent | collapse (8/8) | **false positive** — the dangerous direction for a certificate |
| permanent | mixed | **false positive (mixed)** — reported separately; the mean field makes no per-seed claim |
| permanent | persist | agree |
| not-permanent | collapse | agree |
| not-permanent | persist / mixed, and every persisting seed has **no consumer** at the horizon | **producer-only** — agrees in kind: the pair is not permanent, the producer face is (A1's clause 2 says exactly this) |
| not-permanent | persist / mixed, consumers present | **false negative** (/ mixed) — the safe direction for a certificate, the unsafe one for a gate |
| undecided | any | undecided |

**Fault hypotheses** are assigned mechanically, first matching rule wins, from A1's
authority boundary plus the one known coefficient error:

- *false positive / mixed*: *first-tick floor* (median population after tick 1 on the
  collapsing seeds is 0: every founder died on tick 1) → *reduction: lockup* (modal
  collapse mode is nutrient lockup — outside A1's coordinates, A2's endogenous terms) →
  *reduction: energy death* → *spatial* (median first tick without a consumer while
  producers stood ≤ 25 — reach never met the crop) → *demographic-stochastic* (a mixed
  split with none of the above) → *finite-size* (peak population never left the founder
  scale) → *reduction: unclassified*.
- *false negative / mixed*: *reduction: `χ_P = 0`* (clause 1 fails with `ρ > 1`; never
  fires) → *reduction: extinction gate* (clause 1 fails with `ρ ≤ 1`; never fires) →
  **`#482` (β lumping)** (`κ_C = 0` at the consumer centroid zeroes `β`, so `I = Λ = 0`
  although `ρ > 1` — the consumer twin of the `r_P` lumping #466 corrected) → *reduction:
  single mixotroph centroid* (no consumer compartment to evaluate) → *reduction: carcass
  route* (a decomposer guild on a persisting seed) → *reduction: realised traits departed
  the centroid* (consumers persisted with no guild).

## Results

### Confusion matrices

**A1** — `ρ > 1` and `I > 1` at the seeded centroids (rows predicted, columns observed):

| A1 | collapse (8/8) | mixed | persist (0/8) | total |
|---|---|---|---|---|
| permanent | 0 | 13 | 37 | 50 |
| not-permanent | 2 | 56 | 139 | 197 |
| undecided | 0 | 9 | 26 | 35 |
| total | 2 | 78 | 202 | 282 |

**A2** — clauses (i)–(iii) at the reference lumping:

| A2 | collapse (8/8) | mixed | persist (0/8) | total |
|---|---|---|---|---|
| permanent | 2 | 48 | 126 | 176 |
| not-permanent | 0 | 2 | 17 | 19 |
| undecided | 0 | 28 | 59 | 87 |
| total | 2 | 78 | 202 | 282 |

Only **2 of 282 configs collapse on all eight seeds** and 202 persist on all eight; the
search box (as `default_ranges` draws it) is overwhelmingly a persisting region, and the
question this note can actually answer is about the 78 mixed cells and the two hard
collapses. The atlas cells, being live cells, collapse on no seed in 64/82 and on some
seed in 18/82. On the atlas A1 predicts permanent on 16 cells, not-permanent on 60 and
undecided on 6; A2 permanent on 45, not-permanent on 19 (the `κ_C = 0` cells) and
undecided on 18.

### False-positive rate — the dangerous direction

| | predicted permanent | strict FP (8/8 collapse) | any-seed FP (≥ 1 seed collapses) |
|---|---|---|---|
| A1 | 50 | **0** (0 %) | 13 (26 %) |
| A2 | 176 | **2** (1.1 %) | 50 (28 %) |

There is no "excluding #444" row any more: the denominator is honest by construction.

The strict false positives, both A2 only (A1 says not-permanent on each, `I = 0.05` and
`0.02`, and so agrees with the collapse):

- **`sample:154`**: 21 founders, 8/8 `nutrient-lockup` at the horizon, peak population
  1661 — a large, predation-heavy world that silts its nutrient into the dead pool.
  `Λ = 120` at the reference lumping says the lockup corner repels; the reference sets
  `ι = 1` (every carcass within every consumer's reach), and this is a cell where that
  over-serving bites. It was 6/8 lockup on the previous run.
- **`sample:192`**: 10 founders, 8/8 `nutrient-lockup`, peak population 411, `Λ = 53`,
  `κ_C = 0.05`. Same fault; 7/8 on the previous run.
- **`sample:147`**, the previous run's strict failure (`Λ = 18.8`), is now 3/8 lockup
  and 5/8 monoculture — a mixed cell of the same class. The three together are the
  empirical twin of A2's own example10 reading and the reason A2 calls `Λ` a
  characterisation, not a gate.

### The mixed cells under a permanent prediction

The 13 A1 cells and 48 A2 cells that lose 1–7 of 8 seeds are, by A4's rule, false
positives on the any-seed read. By fault class (A1 / A2):

| fault hypothesis | A1 mixed cells | A2 mixed cells |
|---|---|---|
| demographic-stochastic — 1–5 seeds collapse, no structural signature | 7 | 32 |
| spatial — consumers lost by tick ≤ 25 while producers stood (example10) | 3 | 10 |
| reduction — nutrient lockup on a minority of seeds (A1's coordinates lack the ledger; A2's `ι/ν/q` are endogenous) | 2 | 5 |
| reduction — first-tick floor: every founder dies on tick 1 (`sample:12`, 7/8) | 1 | 1 |

The A1 split by collapse count is 1/8 × 5, 2/8 × 4, 3/8, 4/8, 5/8, 7/8 × 1 each; A2's is
1/8 × 21, 2/8 × 10, 3/8 × 8, 4/8 × 5, 5/8 × 3, 7/8 × 1 — mostly single-seed losses, which
#434 says an `n = 8` block cannot distinguish from a per-seed rate anywhere below ~0.5.
These are the authority boundary's "no demographic noise / no space" clauses operating
as described, and they are the reason `I > 1` cannot be read as a per-seed promise. Full
per-cell rows are in the appendix.

### The negative direction

A1 predicts **not-permanent on 197 cells — every one through clause (2); 2 collapse
(8/8), 139 persist on every seed, 56 are mixed.** After the producer-only credit (15
cells whose persisting seeds carry no consumer at the horizon — the producer face alone,
which is what a failed clause 2 says), 131 false negatives and 49 mixed remain. Their
hypotheses:

| fault hypothesis | cells |
|---|---|
| carcass route — a decomposer guild present on persisting seeds; `I ≤ 1` but `Λ ≫ 1` | 143 |
| `#482` (β lumping) — `κ_C = 0` zeroes `β`, so `I = Λ = 0` although `ρ` is 12–529 (18 atlas cells, 16 persist 8/8) | 18 |
| single mixotroph centroid — no consumer compartment to evaluate | 11 |
| consumers persisted with no guild — realised traits departed the centroid | 8 |

Two things follow.

**A1's clause (2) is not the system's boundary.** `I = κ_C·γ·base·e^{−λd}·h_C·F/(B_P·B_C)`
exceeds 1 on only 85 of 282 configs (median 0.26), because the pair distance `d` between a
pure producer and a pure consumer centroid is `√2·(α+h)` plus the shared dimensions, and
`trophic_distance_decay` runs to 5 — the living-prey kernel is small across most of the
box. Yet heterotrophs persist almost everywhere, and the evaluator sees a decomposer
guild on 172 of the 180 disagreeing cells. This is A2's finding read off the IBM: the
pile, not the crop, is the heterotroph's escape route, and `Λ` (`∝ N_total/ν`, with
`N_total = 50 000` fixed by the viable baseline and not searched) is in the hundreds to
tens of thousands at the reference lumping. A gate that killed `I ≤ 1` would kill 195 of
the 197 cells it flagged — 60 of the 82 atlas live cells among them; the "mean-field
non-permanent ⇒ the expected trajectory goes extinct" reading of A1's authority boundary
holds for the *pair on the living-prey route*, not for the world.

**`κ_C = 0` is the same lumping error, one compartment over.** All 19 atlas cells at
`mean_kappa = 0` (the search box's lower edge, which QD evidently likes — 10 of 56 cells
before, 19 of 82 now) clear clause (1) since #466 (`r_P` from 0.03 to 0.56, `ρ` from 12
to 529) and fail clause (2) with `I = 0` exactly, because `β = κ_C·γ·e·a` reads the
consumer's growth as its somatic share only — at `κ_C = 0` the whole surplus becomes
offspring biomass the map should carry and the coefficient drops. A2 inherits it
(`Λ ∝ min(κ_C·γ·e_C, q/θ_C) = 0`), which is why A2's 19 not-permanent cells are exactly
these. 17 persist on 8/8 seeds, 2 are mixed, and one is producer-only. The theorem is
untouched; the `β` row wants the `χ_C` twin of `χ_P`, which is #482's scope, not this
note's. Until then clause (2) carries a spurious conjunct `κ_C > 0`, and every one of
these cells is tagged `#482` in the appendix rather than counted as a permanence error.

### A2's not-permanent and undecided cells

A2 calls only 19 cells not-permanent — the 19 `κ_C = 0` cells above, through `Λ = 0`. 87
cells are undecided: 80 for H1 (`r_P > 2`), 7 for `Λ ≤ 1` at the reference lumping;
clause (i) holds on every cell (with the #466 `r_P` no producer sits in `(0, μ_P]`), and
(iii) fails alone on none. None of the undecided cells collapses 8/8; 28 are mixed.

## What eight seeds can and cannot say here

The strict read (`8/8` / `0/8`) is the only one #434 licenses at `n = 8`, and on that read
the dangerous direction has no cell for A1 and two for A2 with a named cause — an
over-served pile. The any-seed read is where the honest uncertainty sits: a `1/8` loss is
consistent with a true per-seed collapse rate anywhere in `[0.003, 0.53]`, so the 5 A1
cells losing exactly one seed may be robust worlds with a rare bad draw or coin-flips —
`n = 32` (the projection's refinement block) would separate 0.35 from 0.65 at 5 %, and is
the right next instrument if the mixed cells are ever to be resolved rather than tallied.
Nothing in this note depends on resolving them: the verdict below is the same whether
every mixed cell is read as a false positive or as noise, because the negative direction,
not the positive one, is what disqualifies clause (2) as a gate. The `sample:12`
reshuffle (8/8 → 7/8) is the cautionary case in the other direction: a strict false
positive at `n = 8` is one surviving seed away from a mixed cell.

## Verdict — characterisation, per predicate

Applying A4's rule (*zero false positives → gate-grade; any FP → promote as
characterisation citing the failure cells*), per predicate:

- **A1 clause (1), `ρ = F/B_P > 1` (the sharpened extinction gate): uncontradicted,
  unexercised, not wired.** No cell fails it since #466 and no cell contradicts it, but
  no cell exercises it either (`ρ ≥ 1.9` throughout the box, so it would exclude nothing
  the standing `F ≤ B` gate does not). Its centroid form is also not a config-only kill:
  `B_P` is read at the pure-producer centroid, and a founder drawn or a mutant evolved
  with less autotrophy pays less maintenance, so `ρ ≤ 1` at the centroid does not prove
  extinction the way `F ≤ B` does. The sound config-only version *is* `F ≤ B`. There is
  nothing to wire.
- **A1 clause (2), `I > 1`: characterisation only.** The positive direction now meets
  A4's rule — strict FP 0/50 (`sample:12` survives one seed on the fixed stepper) — and
  13/50 on the any-seed read. But a prefilter runs the negative direction, and `I ≤ 1`
  kills 2 of the 197 cells it flags; 60 of the 82 atlas live cells fail it and persist.
  It characterises whether the *living-prey* route is open — a useful descriptor for the
  atlas, not a gate. Its `κ_C = 0` conjunct is #482's lumping and must be dropped before
  `I` is read on those cells at all.
- **A2 `Λ > 1` at the reference lumping: characterisation only, citing `sample:154` and
  `sample:192`.** Two genuine strict failures (and `sample:147` mixed), where the
  endogenous reach `ι` is the term that bit — exactly A2's stated contingency. Its
  config-only kills (`Λ = 0`) fire on the 19 `κ_C = 0` cells and are all #482; its
  undecided band is a third of the box.
- **Not gate-grade with all live cells passing:** no predicate. A1's conjunction fails
  60 live cells; A2's fails 19 (all #482) and is undecided on 18; the two A2 strict
  failures are `sample` cells, not live cells, which is what makes them counter-examples
  to any `Λ` gate drawn at the reference lumping.

For the standing follow-ups: carry `I` and `Λ` as atlas descriptors (per cell, at the
seeded centroids) rather than gates; correct the `β` row (#482) before reading either on
a `κ_C = 0` cell; and, if a gate on the pile route is ever wanted, take A2's reserve
remedy (a reach bound `ῑ` and a carcass floor `ν̲`) rather than the reference lumping —
`sample:154` and `sample:192` are the counter-examples that would falsify any `Λ` gate
drawn without them. #465 (the deferred prefilter permanence gate) stays deferred on
those grounds, with #482 added to its blockers.

## Reproduce

```
cargo run --release -p explorers-search --bin permanence_crosscheck            # ~30 min, 2256 runs
cargo test -p explorers-search --bin permanence_crosscheck                     # pure parts, < 1 s
PERMANENCE_CROSSCHECK_CONFIGS=sample:154,sample:192,sample:12 PERMANENCE_CROSSCHECK_SEEDS=8 \
  cargo run --release -p explorers-search --bin permanence_crosscheck          # the A2 strict FPs and the A1 near-miss
```

Artifact: `target/permanence-crosscheck.json` — per config the two verdicts with every
coefficient, the per-seed outcomes, the aggregate, and the agreement class and
hypothesis; plus the summary printed to stdout. Determinism: two consecutive full runs
give byte-identical artifacts (SHA-1 compared; one config per rayon task, seeds in an
order-stable inner collect).

## Deliverables against the acceptance criteria

- Instrument runs deterministically and covers the atlas live cells + ≥ 200 sampled
  configs × ≥ 8 seeds — 82 + 200 × 8, identical artifact across two runs.
- Confusion matrix and false-positive rate — *Results*, A1 and A2, strict and any-seed.
- Every disagreeing config listed with a fault hypothesis — *Appendix*, with the #482
  cells tagged.
- Explicit verdict — *characterisation*, per predicate, with A4's rule applied and the
  reason no predicate is wired.
- No assertions on emergent values beyond a smoke check — the bin's tests pin the
  coefficient mapping to the prototype's example10 numbers and the pure aggregation
  rules; the one run-producing test asserts only that records were produced.
- No change to the stepper, evaluator scoring, or search — the only code is the bin.

## Appendix — every disagreement

Columns: predicted, observed (collapsing seeds / n), modal terminal mode, `ρ`, `I`, `Λ` at
the reference lumping, `κ_C` at the consumer centroid, hypothesis. Generated from the
artifact; `atlas:i` is the i-th live cell of `atlas.json`, `sample:i` the i-th point of
the seed-421 LHS draw.

### A1 — false positives (strict and mixed)

| cell | predicted | observed | modal mode | ρ | I | Λ | κ_C | fault hypothesis |
|---|---|---|---|---|---|---|---|---|
| `atlas:7` | permanent | mixed (2/8) | none | 24.7 | 1.46 | 3.06e+03 | 0.41 | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:42` | permanent | mixed (1/8) | none | 246.9 | 111.69 | 2.55e+04 | 0.34 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:52` | permanent | mixed (1/8) | none | 25.4 | 1.44 | 2.63e+03 | 0.38 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:12` | permanent | mixed (7/8) | extinction | 11.2 | 2.09 | 7.58e+03 | 0.33 | reduction: every founder dies on tick 1 whatever its trait sign - a first-tick per-body floor (the peak-relative death threshold, endowment, embodiment) the biomass map has no coordinate for |
| `sample:14` | permanent | mixed (5/8) | nutrient-lockup | 81.1 | 1.17 | 986 | 0.43 | reduction: nutrient lockup is outside A1's coordinates (A2's iota/nu/q are endogenous) |
| `sample:55` | permanent | mixed (4/8) | nutrient-lockup | 141.4 | 3.16 | 5.9e+03 | 0.11 | reduction: nutrient lockup is outside A1's coordinates (A2's iota/nu/q are endogenous) |
| `sample:96` | permanent | mixed (2/8) | monoculture | 168.2 | 3.85 | 2.01e+03 | 0.16 | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:103` | permanent | mixed (1/8) | none | 54.9 | 3.25 | 2.84e+03 | 0.45 | spatial: consumers lost by tick 22 while producers stood - reach never met the crop (example10 signature) |
| `sample:120` | permanent | mixed (3/8) | none | 5.6 | 1.39 | 1.03e+04 | 0.82 | spatial: consumers lost by tick 17 while producers stood - reach never met the crop (example10 signature) |
| `sample:131` | permanent | mixed (2/8) | none | 52.6 | 11.87 | 2.9e+04 | 0.37 | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:136` | permanent | mixed (1/8) | none | 257.6 | 1.24 | 464 | 0.25 | spatial: consumers lost by tick 21 while producers stood - reach never met the crop (example10 signature) |
| `sample:172` | permanent | mixed (1/8) | none | 29.3 | 2.41 | 5.91e+03 | 0.98 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:180` | permanent | mixed (2/8) | none | 56.8 | 39.76 | 5.58e+04 | 0.56 | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |

### A1 — false negatives (strict and mixed)

| cell | predicted | observed | modal mode | ρ | I | Λ | κ_C | fault hypothesis |
|---|---|---|---|---|---|---|---|---|
| `atlas:0` | not-permanent | mixed (3/8) | none | 12.5 | 0.27 | 1.23e+03 | 0.33 | reduction: consumers persisted on 5/5 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `atlas:3` | not-permanent | persist (0/8) | monoculture | 529.3 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 529.3 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 2/8 seeds |
| `atlas:4` | not-permanent | mixed (3/8) | none | 14.4 | 0.09 | 368 | 0.37 | reduction: consumers persisted on 2/5 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `atlas:5` | not-permanent | persist (0/8) | monoculture | 26.2 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 26.2 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 6/8 seeds |
| `atlas:8` | not-permanent | mixed (1/8) | none | 35.9 | 0.06 | 115 | 0.41 | reduction: consumers persisted on 4/7 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `atlas:10` | not-permanent | mixed (2/8) | none | 20.7 | 0.01 | 22.5 | 0.06 | reduction: consumers persisted on 1/6 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `atlas:11` | not-permanent | persist (0/8) | none | 36.4 | 0.10 | 144 | 0.57 | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 7 of them - A2's carcass route, which A1's map lacks |
| `atlas:14` | not-permanent | persist (0/8) | none | 52.1 | 0.84 | 856 | 0.25 | reduction: consumers persisted on 2/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `atlas:15` | not-permanent | persist (0/8) | monoculture | 66.5 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 66.5 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 7/8 seeds |
| `atlas:16` | not-permanent | mixed (1/8) | none | 89.8 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 89.8 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 2/7 seeds |
| `atlas:17` | not-permanent | persist (0/8) | none | 49.5 | 0.05 | 100 | 0.73 | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `atlas:19` | not-permanent | mixed (2/8) | none | 15.7 | 0.27 | 976 | 0.09 | reduction: consumers persisted on 5/6 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `atlas:20` | not-permanent | persist (0/8) | none | 74.1 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 74.1 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 7/8 seeds |
| `atlas:21` | not-permanent | persist (0/8) | none | 24.6 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 24.6 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 8/8 seeds |
| `atlas:22` | not-permanent | persist (0/8) | none | 74.7 | 0.15 | 100 | 0.12 | reduction: consumers persisted on 1/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `atlas:23` | not-permanent | persist (0/8) | none | 14.7 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 14.7 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 3/8 seeds |
| `atlas:24` | not-permanent | persist (0/8) | none | 50.0 | 0.33 | 650 | 0.56 | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `atlas:25` | not-permanent | mixed (1/8) | none | 37.7 | 0.59 | 578 | 0.36 | reduction: consumers persisted on 5/7 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `atlas:26` | not-permanent | persist (0/8) | monoculture | 79.9 | 0.51 | 356 | 0.64 | reduction: consumers persisted on 1/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `atlas:27` | not-permanent | persist (0/8) | none | 23.3 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 23.3 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 4/8 seeds |
| `atlas:29` | not-permanent | persist (0/8) | none | 47.1 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 47.1 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 8/8 seeds |
| `atlas:30` | not-permanent | persist (0/8) | none | 26.5 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 26.5 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 3/8 seeds |
| `atlas:31` | not-permanent | persist (0/8) | none | 42.2 | 0.72 | 963 | 0.27 | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `atlas:32` | not-permanent | persist (0/8) | none | 39.8 | 0.08 | 82.8 | 0.14 | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 6 of them - A2's carcass route, which A1's map lacks |
| `atlas:33` | not-permanent | mixed (1/8) | none | 20.8 | 0.14 | 646 | 0.48 | reduction: consumers persisted on 3/7 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `atlas:34` | not-permanent | persist (0/8) | none | 30.5 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 30.5 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 6/8 seeds |
| `atlas:35` | not-permanent | persist (0/8) | none | 32.4 | 0.47 | 1.46e+03 | 0.61 | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `atlas:36` | not-permanent | mixed (2/8) | none | 23.7 | 0.07 | 352 | 0.34 | reduction: consumers persisted on 5/6 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `atlas:38` | not-permanent | persist (0/8) | none | 24.5 | 0.08 | 133 | 0.20 | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 7 of them - A2's carcass route, which A1's map lacks |
| `atlas:39` | not-permanent | mixed (1/8) | none | 21.0 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 21.0 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 3/7 seeds |
| `atlas:40` | not-permanent | persist (0/8) | none | 63.5 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 63.5 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 4/8 seeds |
| `atlas:41` | not-permanent | persist (0/8) | none | 32.5 | 0.18 | 307 | 0.34 | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `atlas:43` | not-permanent | persist (0/8) | none | 13.1 | 0.11 | 502 | 0.35 | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `atlas:44` | not-permanent | persist (0/8) | none | 25.5 | 0.14 | 508 | 0.53 | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `atlas:45` | not-permanent | persist (0/8) | none | 43.2 | 0.37 | 805 | 0.15 | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `atlas:46` | not-permanent | persist (0/8) | none | 35.4 | 0.02 | 22.2 | 0.21 | reduction: consumers persisted on 2/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `atlas:49` | not-permanent | persist (0/8) | none | 43.5 | 0.15 | 243 | 0.04 | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `atlas:50` | not-permanent | mixed (2/8) | monoculture | 56.8 | 0.13 | 189 | 0.19 | reduction: consumers persisted on 5/6 seeds despite I <= 1 and no decomposer guild - realised traits departed the centroid (evolution / draw) |
| `atlas:53` | not-permanent | persist (0/8) | none | 34.4 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 34.4 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 6/8 seeds |
| `atlas:56` | not-permanent | persist (0/8) | none | 41.5 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 41.5 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 5/8 seeds |
| `atlas:58` | not-permanent | persist (0/8) | none | 30.3 | 0.12 | 246 | 0.77 | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `atlas:59` | not-permanent | persist (0/8) | none | 39.0 | 0.33 | 432 | 0.20 | reduction: consumers persisted on 1/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `atlas:60` | not-permanent | persist (0/8) | none | 22.0 | 0.20 | 505 | 0.55 | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `atlas:61` | not-permanent | persist (0/8) | none | 37.8 | 0.24 | 481 | 0.53 | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `atlas:64` | not-permanent | mixed (1/8) | none | 18.1 | 0.01 | 30.2 | 0.15 | reduction: consumers persisted on 2/7 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `atlas:65` | not-permanent | mixed (1/8) | none | 72.1 | 0.11 | 188 | 0.57 | reduction: consumers persisted on 6/7 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `atlas:66` | not-permanent | persist (0/8) | none | 52.1 | 0.08 | 98.8 | 0.55 | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `atlas:68` | not-permanent | persist (0/8) | none | 33.6 | 0.02 | 22.6 | 0.02 | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `atlas:69` | not-permanent | persist (0/8) | none | 28.3 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 28.3 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 5/8 seeds |
| `atlas:70` | not-permanent | mixed (1/8) | none | 63.9 | 0.03 | 57.9 | 0.41 | reduction: consumers persisted on 5/7 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `atlas:71` | not-permanent | persist (0/8) | none | 23.4 | 0.48 | 2.17e+03 | 0.72 | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `atlas:73` | not-permanent | persist (0/8) | none | 116.8 | 0.92 | 551 | 0.23 | reduction: consumers persisted on 1/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `atlas:74` | not-permanent | persist (0/8) | none | 12.1 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 12.1 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 6/8 seeds |
| `atlas:77` | not-permanent | persist (0/8) | none | 21.0 | 0.04 | 113 | 0.10 | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `atlas:79` | not-permanent | persist (0/8) | none | 13.7 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 13.7 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 3/8 seeds |
| `atlas:80` | not-permanent | persist (0/8) | none | 20.5 | 0.01 | 15.9 | 0.05 | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:1` | not-permanent | persist (0/8) | none | 55.0 | 0.22 | 193 | 0.98 | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 7 of them - A2's carcass route, which A1's map lacks |
| `sample:2` | not-permanent | persist (0/8) | none | 16.9 | 0.02 | 102 | 0.73 | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 8 of them - A2's carcass route, which A1's map lacks |
| `sample:3` | not-permanent | mixed (4/8) | nutrient-lockup | 21.4 | 0.03 | 102 | 0.80 | reduction: consumers persisted on 2/4 seeds despite I <= 1 and no decomposer guild - realised traits departed the centroid (evolution / draw) |
| `sample:5` | not-permanent | persist (0/8) | none | 3.6 | 0.01 | 139 | 0.09 | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:6` | not-permanent | persist (0/8) | none | 15.7 | 0.72 | 3.83e+03 | 0.63 | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 7/8 persisting seeds |
| `sample:8` | not-permanent | persist (0/8) | none | 19.0 | 0.01 | 225 | 0.94 | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 7 of them - A2's carcass route, which A1's map lacks |
| `sample:9` | not-permanent | mixed (3/8) | none | 48.8 | 0.45 | 1.05e+03 | 0.31 | reduction: consumers persisted on 2/5 seeds despite I <= 1 and no decomposer guild - realised traits departed the centroid (evolution / draw) |
| `sample:10` | not-permanent | persist (0/8) | none | 20.3 | 0.33 | 1.94e+03 | 0.45 | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:11` | not-permanent | persist (0/8) | none | 7.7 | 0.34 | 2.46e+03 | 0.42 | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:13` | not-permanent | persist (0/8) | none | 28.9 | 0.00 | 4.22 | 0.03 | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 7 of them - A2's carcass route, which A1's map lacks |
| `sample:16` | not-permanent | mixed (4/8) | none | 69.4 | 0.81 | 664 | 0.39 | reduction: consumers persisted on 4/4 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:17` | not-permanent | persist (0/8) | none | 80.8 | 0.06 | 93.7 | 0.66 | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:18` | not-permanent | persist (0/8) | none | 17.2 | 0.01 | 30.5 | 0.69 | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 7 of them - A2's carcass route, which A1's map lacks |
| `sample:19` | not-permanent | persist (0/8) | none | 73.1 | 0.37 | 377 | 0.73 | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:21` | not-permanent | persist (0/8) | none | 20.3 | 0.02 | 105 | 0.18 | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:22` | not-permanent | persist (0/8) | monoculture | 9.4 | 0.11 | 980 | 0.34 | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:23` | not-permanent | persist (0/8) | none | 50.8 | 0.07 | 173 | 0.17 | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:24` | not-permanent | persist (0/8) | none | 30.9 | 0.10 | 412 | 0.99 | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:25` | not-permanent | persist (0/8) | none | 83.8 | 0.30 | 337 | 0.89 | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:26` | not-permanent | mixed (2/8) | none | 29.0 | 0.00 | 9.65 | 0.50 | reduction: consumers persisted on 1/6 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:30` | not-permanent | persist (0/8) | none | 30.9 | 0.11 | 197 | 0.51 | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:31` | not-permanent | persist (0/8) | none | 86.1 | 0.18 | 339 | 0.01 | reduction: consumers persisted on 1/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:34` | not-permanent | persist (0/8) | none | 9.5 | 0.46 | 8.21e+03 | 0.41 | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:35` | not-permanent | persist (0/8) | none | 22.9 | 0.15 | 339 | 0.82 | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:41` | not-permanent | mixed (1/8) | none | 41.8 | 0.05 | 82.4 | 0.57 | reduction: consumers persisted on 4/7 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:43` | not-permanent | persist (0/8) | none | 21.1 | 0.21 | 701 | 0.88 | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:44` | not-permanent | persist (0/8) | none | 82.8 | 0.02 | 18.8 | 0.41 | reduction: consumers persisted on 1/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:45` | not-permanent | mixed (4/8) | none | 35.2 | 0.03 | 71.4 | 1.00 | reduction: consumers persisted on 4/4 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:46` | not-permanent | mixed (4/8) | extinction | 18.2 | 0.06 | 283 | 0.36 | reduction: consumers persisted on 3/4 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:47` | not-permanent | mixed (2/8) | none | 40.3 | 0.10 | 283 | 0.38 | reduction: consumers persisted on 2/6 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:48` | not-permanent | persist (0/8) | none | 13.9 | 0.11 | 358 | 0.34 | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:49` | not-permanent | persist (0/8) | none | 92.4 | 0.39 | 330 | 0.44 | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:51` | not-permanent | persist (0/8) | none | 31.4 | 0.00 | 3.25 | 0.46 | reduction: consumers persisted on 2/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:52` | not-permanent | persist (0/8) | none | 9.8 | 0.00 | 22.4 | 0.76 | reduction: consumers persisted on 2/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:53` | not-permanent | mixed (4/8) | nutrient-lockup | 28.3 | 0.01 | 25.4 | 0.02 | reduction: consumers persisted on 3/4 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:57` | not-permanent | persist (0/8) | none | 50.7 | 0.05 | 75.7 | 0.30 | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:62` | not-permanent | persist (0/8) | none | 7.5 | 0.00 | 0.811 | 0.21 | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:64` | not-permanent | persist (0/8) | none | 67.1 | 0.09 | 74.3 | 0.47 | reduction: consumers persisted on 1/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:65` | not-permanent | mixed (2/8) | none | 31.8 | 0.16 | 645 | 0.66 | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 1/6 persisting seeds |
| `sample:68` | not-permanent | mixed (5/8) | nutrient-lockup | 95.9 | 0.11 | 48.4 | 0.24 | reduction: consumers persisted on 2/3 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:69` | not-permanent | persist (0/8) | none | 11.7 | 0.76 | 7.18e+03 | 0.60 | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:70` | not-permanent | persist (0/8) | none | 2.7 | 0.00 | 14.2 | 0.32 | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:72` | not-permanent | mixed (3/8) | none | 91.1 | 0.14 | 91.4 | 0.57 | reduction: consumers persisted on 4/5 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:73` | not-permanent | persist (0/8) | none | 30.5 | 0.09 | 225 | 0.84 | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 6 of them - A2's carcass route, which A1's map lacks |
| `sample:74` | not-permanent | persist (0/8) | none | 13.0 | 0.07 | 1.19e+03 | 0.22 | reduction: consumers persisted on 2/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:76` | not-permanent | persist (0/8) | none | 8.0 | 0.37 | 6.26e+03 | 0.29 | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:78` | not-permanent | mixed (1/8) | none | 14.8 | 0.02 | 56.1 | 0.07 | reduction: consumers persisted on 7/7 seeds despite I <= 1, with a decomposer guild on 7 of them - A2's carcass route, which A1's map lacks |
| `sample:80` | not-permanent | persist (0/8) | none | 9.4 | 0.00 | 9.52 | 0.01 | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:81` | not-permanent | persist (0/8) | none | 5.8 | 0.02 | 390 | 0.83 | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 7/8 persisting seeds |
| `sample:82` | not-permanent | persist (0/8) | none | 13.8 | 0.31 | 2.23e+03 | 0.86 | reduction: consumers persisted on 2/8 seeds despite I <= 1 and no decomposer guild - realised traits departed the centroid (evolution / draw) |
| `sample:83` | not-permanent | mixed (1/8) | none | 16.2 | 0.30 | 2.62e+03 | 0.27 | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 2/7 persisting seeds |
| `sample:86` | not-permanent | persist (0/8) | none | 57.2 | 0.55 | 517 | 0.55 | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:87` | not-permanent | persist (0/8) | none | 23.4 | 0.62 | 3.98e+03 | 0.31 | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 7/8 persisting seeds |
| `sample:88` | not-permanent | persist (0/8) | none | 45.3 | 0.46 | 589 | 0.23 | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:90` | not-permanent | persist (0/8) | none | 10.6 | 0.01 | 57.2 | 0.14 | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 8 of them - A2's carcass route, which A1's map lacks |
| `sample:91` | not-permanent | persist (0/8) | none | 85.2 | 0.10 | 61.2 | 0.67 | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:92` | not-permanent | persist (0/8) | none | 33.9 | 0.99 | 3.09e+03 | 0.75 | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 6/8 persisting seeds |
| `sample:93` | not-permanent | mixed (1/8) | none | 21.5 | 0.16 | 696 | 0.67 | reduction: consumers persisted on 5/7 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:94` | not-permanent | mixed (1/8) | none | 28.6 | 0.16 | 1.27e+03 | 0.10 | reduction: consumers persisted on 1/7 seeds despite I <= 1 and no decomposer guild - realised traits departed the centroid (evolution / draw) |
| `sample:98` | not-permanent | mixed (2/8) | none | 38.5 | 0.00 | 0.156 | 0.00 | reduction: consumers persisted on 3/6 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:99` | not-permanent | persist (0/8) | none | 39.9 | 0.03 | 85.4 | 0.11 | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:100` | not-permanent | mixed (1/8) | none | 11.2 | 0.03 | 162 | 0.06 | reduction: consumers persisted on 7/7 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:101` | not-permanent | mixed (1/8) | none | 31.3 | 0.03 | 67.4 | 0.50 | reduction: consumers persisted on 2/7 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:104` | not-permanent | mixed (3/8) | none | 30.0 | 0.00 | 0.221 | 0.20 | reduction: consumers persisted on 1/5 seeds despite I <= 1 and no decomposer guild - realised traits departed the centroid (evolution / draw) |
| `sample:108` | not-permanent | persist (0/8) | none | 18.5 | 0.01 | 28.4 | 0.42 | reduction: consumers persisted on 3/8 seeds despite I <= 1 and no decomposer guild - realised traits departed the centroid (evolution / draw) |
| `sample:110` | not-permanent | persist (0/8) | none | 8.5 | 0.43 | 6.79e+03 | 0.84 | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 8 of them - A2's carcass route, which A1's map lacks |
| `sample:112` | not-permanent | persist (0/8) | none | 48.4 | 0.41 | 1.49e+03 | 0.91 | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:113` | not-permanent | persist (0/8) | none | 59.4 | 0.36 | 599 | 0.26 | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:117` | not-permanent | persist (0/8) | none | 1.9 | 0.08 | 3.74e+03 | 0.90 | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:118` | not-permanent | persist (0/8) | none | 9.7 | 0.45 | 5.35e+03 | 0.22 | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 6/8 persisting seeds |
| `sample:119` | not-permanent | persist (0/8) | none | 5.9 | 0.00 | 0.429 | 0.48 | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:121` | not-permanent | persist (0/8) | none | 11.6 | 0.00 | 37.1 | 0.59 | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 6 of them - A2's carcass route, which A1's map lacks |
| `sample:122` | not-permanent | persist (0/8) | none | 26.5 | 0.22 | 458 | 0.08 | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:124` | not-permanent | persist (0/8) | none | 26.0 | 0.00 | 0.643 | 0.87 | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 6 of them - A2's carcass route, which A1's map lacks |
| `sample:125` | not-permanent | persist (0/8) | none | 24.6 | 0.01 | 27 | 0.32 | reduction: consumers persisted on 1/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:126` | not-permanent | persist (0/8) | none | 18.0 | 0.12 | 495 | 0.40 | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:127` | not-permanent | persist (0/8) | none | 31.1 | 0.17 | 345 | 0.29 | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:128` | not-permanent | persist (0/8) | none | 29.1 | 0.78 | 1.42e+03 | 0.87 | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:129` | not-permanent | mixed (1/8) | monoculture | 16.4 | 0.00 | 18.4 | 0.64 | reduction: consumers persisted on 7/7 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:132` | not-permanent | mixed (1/8) | none | 177.1 | 0.80 | 408 | 0.09 | reduction: consumers persisted on 5/7 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:133` | not-permanent | mixed (1/8) | none | 25.9 | 0.72 | 1.24e+03 | 0.77 | reduction: consumers persisted on 6/7 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:135` | not-permanent | persist (0/8) | none | 8.5 | 0.01 | 95.3 | 0.05 | reduction: consumers persisted on 2/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:137` | not-permanent | persist (0/8) | none | 43.7 | 0.50 | 1.75e+03 | 0.13 | reduction: consumers persisted on 2/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:138` | not-permanent | persist (0/8) | none | 25.1 | 0.01 | 14.6 | 0.81 | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:140` | not-permanent | persist (0/8) | none | 47.3 | 0.27 | 641 | 0.54 | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:141` | not-permanent | persist (0/8) | none | 13.4 | 0.33 | 6.32e+03 | 0.91 | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:144` | not-permanent | persist (0/8) | none | 14.0 | 0.38 | 2.45e+03 | 0.12 | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:145` | not-permanent | persist (0/8) | none | 4.6 | 0.86 | 1.53e+04 | 0.70 | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:146` | not-permanent | mixed (5/8) | nutrient-lockup | 45.0 | 0.25 | 571 | 0.06 | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 3/3 persisting seeds |
| `sample:148` | not-permanent | persist (0/8) | none | 35.5 | 0.31 | 638 | 0.03 | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 7/8 persisting seeds |
| `sample:149` | not-permanent | persist (0/8) | none | 53.1 | 0.01 | 27.8 | 0.77 | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:151` | not-permanent | mixed (3/8) | none | 26.6 | 0.00 | 0.00511 | 0.20 | reduction: consumers persisted on 4/5 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:152` | not-permanent | persist (0/8) | none | 9.6 | 0.30 | 2.8e+03 | 0.17 | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:153` | not-permanent | persist (0/8) | none | 40.2 | 0.35 | 1.77e+03 | 0.68 | reduction: consumers persisted on 2/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:155` | not-permanent | persist (0/8) | none | 33.7 | 0.04 | 61.7 | 0.33 | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:161` | not-permanent | mixed (1/8) | none | 17.9 | 0.02 | 103 | 0.47 | reduction: consumers persisted on 6/7 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:162` | not-permanent | persist (0/8) | none | 11.6 | 0.12 | 611 | 0.49 | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:163` | not-permanent | persist (0/8) | none | 73.2 | 0.55 | 876 | 0.88 | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:165` | not-permanent | persist (0/8) | monoculture | 66.4 | 0.20 | 212 | 0.28 | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:166` | not-permanent | persist (0/8) | none | 70.4 | 0.22 | 1.13e+03 | 0.35 | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:167` | not-permanent | persist (0/8) | none | 85.5 | 0.03 | 38.1 | 0.35 | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:168` | not-permanent | persist (0/8) | none | 4.5 | 0.05 | 666 | 0.04 | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 7 of them - A2's carcass route, which A1's map lacks |
| `sample:169` | not-permanent | mixed (1/8) | monoculture | 32.6 | 0.00 | 6.92 | 0.15 | reduction: consumers persisted on 6/7 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:171` | not-permanent | mixed (4/8) | none | 21.2 | 0.03 | 534 | 0.52 | reduction: consumers persisted on 1/4 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:173` | not-permanent | mixed (1/8) | none | 10.3 | 0.01 | 80 | 0.68 | reduction: consumers persisted on 4/7 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:174` | not-permanent | mixed (4/8) | extinction | 46.0 | 0.36 | 601 | 0.72 | reduction: consumers persisted on 3/4 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:175` | not-permanent | mixed (2/8) | none | 68.8 | 0.89 | 553 | 0.72 | reduction: consumers persisted on 5/6 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:177` | not-permanent | persist (0/8) | none | 34.4 | 0.11 | 303 | 0.63 | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:178` | not-permanent | persist (0/8) | none | 45.7 | 0.01 | 30.2 | 0.14 | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:179` | not-permanent | persist (0/8) | none | 14.0 | 0.45 | 2.87e+03 | 0.30 | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 3/8 persisting seeds |
| `sample:181` | not-permanent | persist (0/8) | none | 28.9 | 0.35 | 1.54e+03 | 0.07 | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:182` | not-permanent | persist (0/8) | none | 44.9 | 0.03 | 65.9 | 0.40 | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 7 of them - A2's carcass route, which A1's map lacks |
| `sample:183` | not-permanent | persist (0/8) | none | 24.9 | 0.67 | 1.52e+03 | 0.16 | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:185` | not-permanent | persist (0/8) | none | 9.5 | 0.10 | 1.11e+03 | 0.53 | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 5/8 persisting seeds |
| `sample:186` | not-permanent | mixed (3/8) | none | 47.8 | 0.21 | 583 | 0.26 | reduction: consumers persisted on 3/5 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:187` | not-permanent | persist (0/8) | none | 47.0 | 0.16 | 181 | 0.13 | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:188` | not-permanent | persist (0/8) | none | 16.6 | 0.28 | 3.94e+03 | 0.90 | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 6 of them - A2's carcass route, which A1's map lacks |
| `sample:189` | not-permanent | mixed (1/8) | none | 9.0 | 0.41 | 6.6e+03 | 0.92 | reduction: consumers persisted on 5/7 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:191` | not-permanent | persist (0/8) | none | 8.4 | 0.00 | 3.18 | 0.37 | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:193` | not-permanent | mixed (1/8) | none | 16.5 | 0.41 | 1.51e+03 | 0.62 | reduction: consumers persisted on 1/7 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:196` | not-permanent | persist (0/8) | monoculture | 11.7 | 0.00 | 0.807 | 0.86 | reduction: consumers persisted on 4/8 seeds despite I <= 1 and no decomposer guild - realised traits departed the centroid (evolution / draw) |
| `sample:197` | not-permanent | mixed (1/8) | monoculture | 45.0 | 0.18 | 1.29e+03 | 0.44 | reduction: consumers persisted on 6/7 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:198` | not-permanent | persist (0/8) | none | 7.0 | 0.06 | 1.56e+03 | 0.36 | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:199` | not-permanent | persist (0/8) | none | 19.4 | 0.08 | 199 | 0.61 | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |

### A2 — false positives (strict and mixed)

| cell | predicted | observed | modal mode | ρ | I | Λ | κ_C | fault hypothesis |
|---|---|---|---|---|---|---|---|---|
| `atlas:0` | permanent | mixed (3/8) | none | 12.5 | 0.27 | 1.23e+03 | 0.33 | demographic-stochastic: 3/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:4` | permanent | mixed (3/8) | none | 14.4 | 0.09 | 368 | 0.37 | demographic-stochastic: 3/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:7` | permanent | mixed (2/8) | none | 24.7 | 1.46 | 3.06e+03 | 0.41 | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:10` | permanent | mixed (2/8) | none | 20.7 | 0.01 | 22.5 | 0.06 | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:19` | permanent | mixed (2/8) | none | 15.7 | 0.27 | 976 | 0.09 | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:33` | permanent | mixed (1/8) | none | 20.8 | 0.14 | 646 | 0.48 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:36` | permanent | mixed (2/8) | none | 23.7 | 0.07 | 352 | 0.34 | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:42` | permanent | mixed (1/8) | none | 246.9 | 111.69 | 2.55e+04 | 0.34 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:50` | permanent | mixed (2/8) | monoculture | 56.8 | 0.13 | 189 | 0.19 | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:52` | permanent | mixed (1/8) | none | 25.4 | 1.44 | 2.63e+03 | 0.38 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:64` | permanent | mixed (1/8) | none | 18.1 | 0.01 | 30.2 | 0.15 | spatial: consumers lost by tick 14 while producers stood - reach never met the crop (example10 signature) |
| `sample:9` | permanent | mixed (3/8) | none | 48.8 | 0.45 | 1.05e+03 | 0.31 | demographic-stochastic: 3/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:12` | permanent | mixed (7/8) | extinction | 11.2 | 2.09 | 7.58e+03 | 0.33 | reduction: every founder dies on tick 1 whatever its trait sign - a first-tick per-body floor (the peak-relative death threshold, endowment, embodiment) the biomass map has no coordinate for |
| `sample:14` | permanent | mixed (5/8) | nutrient-lockup | 81.1 | 1.17 | 986 | 0.43 | reduction: nutrient lockup is outside A1's coordinates (A2's iota/nu/q are endogenous) |
| `sample:16` | permanent | mixed (4/8) | none | 69.4 | 0.81 | 664 | 0.39 | demographic-stochastic: 4/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:26` | permanent | mixed (2/8) | none | 29.0 | 0.00 | 9.65 | 0.50 | spatial: consumers lost by tick 13 while producers stood - reach never met the crop (example10 signature) |
| `sample:27` | permanent | mixed (1/8) | none | 27.9 | 0.01 | 9.84 | 0.21 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:46` | permanent | mixed (4/8) | extinction | 18.2 | 0.06 | 283 | 0.36 | spatial: consumers lost by tick 15 while producers stood - reach never met the crop (example10 signature) |
| `sample:47` | permanent | mixed (2/8) | none | 40.3 | 0.10 | 283 | 0.38 | spatial: consumers lost by tick 10 while producers stood - reach never met the crop (example10 signature) |
| `sample:53` | permanent | mixed (4/8) | nutrient-lockup | 28.3 | 0.01 | 25.4 | 0.02 | reduction: nutrient lockup is outside A1's coordinates (A2's iota/nu/q are endogenous) |
| `sample:55` | permanent | mixed (4/8) | nutrient-lockup | 141.4 | 3.16 | 5.9e+03 | 0.11 | reduction: nutrient lockup is outside A1's coordinates (A2's iota/nu/q are endogenous) |
| `sample:68` | permanent | mixed (5/8) | nutrient-lockup | 95.9 | 0.11 | 48.4 | 0.24 | reduction: nutrient lockup is outside A1's coordinates (A2's iota/nu/q are endogenous) |
| `sample:78` | permanent | mixed (1/8) | none | 14.8 | 0.02 | 56.1 | 0.07 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:83` | permanent | mixed (1/8) | none | 16.2 | 0.30 | 2.62e+03 | 0.27 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:93` | permanent | mixed (1/8) | none | 21.5 | 0.16 | 696 | 0.67 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:94` | permanent | mixed (1/8) | none | 28.6 | 0.16 | 1.27e+03 | 0.10 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:96` | permanent | mixed (2/8) | monoculture | 168.2 | 3.85 | 2.01e+03 | 0.16 | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:100` | permanent | mixed (1/8) | none | 11.2 | 0.03 | 162 | 0.06 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:101` | permanent | mixed (1/8) | none | 31.3 | 0.03 | 67.4 | 0.50 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:103` | permanent | mixed (1/8) | none | 54.9 | 3.25 | 2.84e+03 | 0.45 | spatial: consumers lost by tick 22 while producers stood - reach never met the crop (example10 signature) |
| `sample:120` | permanent | mixed (3/8) | none | 5.6 | 1.39 | 1.03e+04 | 0.82 | spatial: consumers lost by tick 17 while producers stood - reach never met the crop (example10 signature) |
| `sample:129` | permanent | mixed (1/8) | monoculture | 16.4 | 0.00 | 18.4 | 0.64 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:131` | permanent | mixed (2/8) | none | 52.6 | 11.87 | 2.9e+04 | 0.37 | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:132` | permanent | mixed (1/8) | none | 177.1 | 0.80 | 408 | 0.09 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:136` | permanent | mixed (1/8) | none | 257.6 | 1.24 | 464 | 0.25 | spatial: consumers lost by tick 21 while producers stood - reach never met the crop (example10 signature) |
| `sample:142` | permanent | mixed (3/8) | none | 23.0 | 0.01 | 17.1 | 0.18 | spatial: consumers lost by tick 13 while producers stood - reach never met the crop (example10 signature) |
| `sample:146` | permanent | mixed (5/8) | nutrient-lockup | 45.0 | 0.25 | 571 | 0.06 | reduction: nutrient lockup is outside A1's coordinates (A2's iota/nu/q are endogenous) |
| `sample:147` | permanent | mixed (3/8) | monoculture | 30.8 | 0.01 | 18.8 | 0.04 | demographic-stochastic: 3/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:154` | permanent | collapse (8/8) | nutrient-lockup | 40.4 | 0.05 | 120 | 0.27 | reduction: nutrient lockup is outside A1's coordinates (A2's iota/nu/q are endogenous) |
| `sample:158` | permanent | mixed (3/8) | none | 9.8 | 0.00 | 10.6 | 0.08 | spatial: consumers lost by tick 12 while producers stood - reach never met the crop (example10 signature) |
| `sample:161` | permanent | mixed (1/8) | none | 17.9 | 0.02 | 103 | 0.47 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:169` | permanent | mixed (1/8) | monoculture | 32.6 | 0.00 | 6.92 | 0.15 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:171` | permanent | mixed (4/8) | none | 21.2 | 0.03 | 534 | 0.52 | demographic-stochastic: 4/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:172` | permanent | mixed (1/8) | none | 29.3 | 2.41 | 5.91e+03 | 0.98 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:173` | permanent | mixed (1/8) | none | 10.3 | 0.01 | 80 | 0.68 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:180` | permanent | mixed (2/8) | none | 56.8 | 39.76 | 5.58e+04 | 0.56 | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:186` | permanent | mixed (3/8) | none | 47.8 | 0.21 | 583 | 0.26 | spatial: consumers lost by tick 13 while producers stood - reach never met the crop (example10 signature) |
| `sample:189` | permanent | mixed (1/8) | none | 9.0 | 0.41 | 6.6e+03 | 0.92 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:192` | permanent | collapse (8/8) | nutrient-lockup | 35.6 | 0.02 | 53.2 | 0.05 | reduction: nutrient lockup is outside A1's coordinates (A2's iota/nu/q are endogenous) |
| `sample:193` | permanent | mixed (1/8) | none | 16.5 | 0.41 | 1.51e+03 | 0.62 | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |

### A2 — false negatives (strict and mixed)

| cell | predicted | observed | modal mode | ρ | I | Λ | κ_C | fault hypothesis |
|---|---|---|---|---|---|---|---|---|
| `atlas:3` | not-permanent | persist (0/8) | monoculture | 529.3 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 529.3 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 2/8 seeds |
| `atlas:5` | not-permanent | persist (0/8) | monoculture | 26.2 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 26.2 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 6/8 seeds |
| `atlas:15` | not-permanent | persist (0/8) | monoculture | 66.5 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 66.5 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 7/8 seeds |
| `atlas:16` | not-permanent | mixed (1/8) | none | 89.8 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 89.8 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 2/7 seeds |
| `atlas:20` | not-permanent | persist (0/8) | none | 74.1 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 74.1 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 7/8 seeds |
| `atlas:21` | not-permanent | persist (0/8) | none | 24.6 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 24.6 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 8/8 seeds |
| `atlas:23` | not-permanent | persist (0/8) | none | 14.7 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 14.7 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 3/8 seeds |
| `atlas:27` | not-permanent | persist (0/8) | none | 23.3 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 23.3 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 4/8 seeds |
| `atlas:29` | not-permanent | persist (0/8) | none | 47.1 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 47.1 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 8/8 seeds |
| `atlas:30` | not-permanent | persist (0/8) | none | 26.5 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 26.5 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 3/8 seeds |
| `atlas:34` | not-permanent | persist (0/8) | none | 30.5 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 30.5 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 6/8 seeds |
| `atlas:39` | not-permanent | mixed (1/8) | none | 21.0 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 21.0 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 3/7 seeds |
| `atlas:40` | not-permanent | persist (0/8) | none | 63.5 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 63.5 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 4/8 seeds |
| `atlas:53` | not-permanent | persist (0/8) | none | 34.4 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 34.4 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 6/8 seeds |
| `atlas:56` | not-permanent | persist (0/8) | none | 41.5 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 41.5 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 5/8 seeds |
| `atlas:69` | not-permanent | persist (0/8) | none | 28.3 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 28.3 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 5/8 seeds |
| `atlas:74` | not-permanent | persist (0/8) | none | 12.1 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 12.1 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 6/8 seeds |
| `atlas:79` | not-permanent | persist (0/8) | none | 13.7 | 0.00 | 0 | 0.00 | #482 (beta lumping): kappa_C = 0 zeroes beta = kappa_C*gamma*e*a, so I = Lambda = 0 although rho = 13.7 > 1 - the consumer twin of the r_P lumping #466 corrected; the reproductive branch still becomes offspring biomass; consumers persisted on 3/8 seeds |
