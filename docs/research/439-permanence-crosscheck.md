# Issue #439 — formal viability A3: cross-check of the A1/A2 permanence predictions against the atlas and seed ensembles

**Status: research finding. Commits nothing.** This note asks whether the closed-form
permanence predictions of [`432-permanence-pc.md`](432-permanence-pc.md) (A1) and
[`437-permanence-alc.md`](437-permanence-alc.md) (A2) agree with what the committed
simulation does, on every atlas live cell and a low-discrepancy sample of the search
box, over a fixed seed ensemble. It presents the confusion matrix predicted × observed,
the false-positive rate (predicted permanent, observed collapse — the dangerous
direction) raw and on the honest denominator that excludes the #444 founder artefact,
a per-cell list of every disagreement with a one-line fault hypothesis, and a verdict
that gates A4. It introduces no mechanism, functional form, or parameter; it touches
nothing in `docs/system-design/` and nothing in the stepper, the evaluator's scoring, or
the search.

The instrument is `crates/explorers-search/src/bin/permanence_crosscheck.rs` — a
diagnostic bin patterned on `energy_bound_check.rs` / `role_emergence.rs`, not a CI gate.
Its artifact is `target/permanence-crosscheck.json` (gitignored).

## TL;DR

1. **Coverage.** 56 atlas live cells + 200 LHS configs (the seed-421 draw shared with
   `role_emergence` and `energy_bound_check`, so `sample:i` names the same config
   everywhere) × 8 fixed seeds × the 500-tick search horizon = 2048 runs. Two full runs
   produce a byte-identical artifact.
2. **The dangerous direction is nearly empty — on the strict read.** A1 predicts
   *permanent* on 44 cells; **1** collapses on 8/8 seeds (`sample:12`), and that one is a
   first-tick founder floor the biomass map has no coordinate for (every founder dies on
   tick 1, negative-trait or not), not a permanence error — it is *not* a #444 artefact and
   stays on the honest denominator: **strict FP 1/44 raw, 1/42 excluding #444-tagged
   cells.** A2 at its reference lumping predicts *permanent* on 163 cells; 2 collapse 8/8 —
   `sample:12` again and `sample:147`, a nutrient lockup where `Λ = 18.8` says the corner
   should repel (2/163 raw, 2/157 excluding #444).
3. **But ~half of the predicted-permanent cells collapse on *some* seed** (A1: 23/44 raw,
   21/42 excluding #444; A2: 82/163, 76/157). Those are the "no per-seed claim" of the
   authority boundary made visible: 1–4 of 8 seeds die by demographic chance (13 A1 cells),
   by spatial reach failing to meet the crop before the consumers starve — the example10
   signature — (7 cells), or by the #444 cull emptying a compartment at tick 1 (2 cells).
4. **The negative direction — the one a gate would use — is contradicted for clause (2).**
   A1 predicts *not-permanent* on 180 cells, and 179 of them persist on at least one seed
   (92 on all eight). After crediting the 9 cells whose persisting seeds hold no consumer
   at all (the producer-only face, which is what a failed clause 2 predicts), 170
   disagreements remain — and 155 of them carry a **decomposer guild** on their persisting
   seeds. The consumer that cannot invade the standing crop (`I ≤ 1`; median `I` over the
   box is 0.21) persists on the **carcass pile** — exactly the route A2 adds (`Λ > 1`;
   median `Λ` at the reference lumping is 562) and A1's two-compartment map lacks. A1's
   clause (2) is therefore **not** a kill condition for the system, only for the
   living-prey route, and A2's `Λ` replaces it.
5. **Clause (1), the sharpened extinction gate, is untested by the box.** `ρ = F/B_P ≥ 1.9`
   on every config (`F ≥ 1`, `B_P ≤ 0.6`), so no cell exercises `ρ ≤ 1`. The 10 cells on
   which clause (1) *does* fail have `ρ ≫ 1` and `κ_P = 0`: the lumping
   `r_P = κ_P·γ·(F − B_P)` reads the producer's growth as its somatic share only, and at
   `κ_P = 0` calls a world non-permanent that persists on 8/8 seeds (7 cells) — the
   reproductive share still becomes offspring biomass. That is a **reduction error** in
   A1's coefficient table, recorded here for A4, not a fault of the theorem.
6. **Verdict: promote-with-caveat.** (a) Promote A1 clause (1) as the affirmative form of
   the extinction gate with the `κ_P` lumping corrected (`r_P` must count the whole net
   income that becomes biomass, not `κ_P` of it) and with the explicit caveat that the
   search box never exercises it; (b) promote A1 clause (2) and A2's `Λ` **as
   characterisation only** — `I > 1` certifies nothing per seed (half of its cells lose a
   seed) and `I ≤ 1` kills nothing (the pile route); `Λ > 1` at the reference lumping has
   one genuine failure cell (`sample:147`) where the endogenous reach `ι` is what bit, as
   A2 said it would. Neither is gate-grade under A4's zero-false-positive rule: the strict
   read has one failure per column on the honest denominator (`sample:12`, a tick-0 regime
   exit), and the any-seed read has ~50 %.

## Reading list this note assumes

A1's map, the ratios `ρ = F/B_P`, `I = β·K_P/m`, hypotheses H1–H2, and the authority
boundary; A2's coupled reduction, `Λ`, clauses (i)–(iii), and its list of endogenous terms
(`ι`, `ν`, `q`, `μ_P`); [`434-ensemble-confidence.md`](434-ensemble-confidence.md) for what
`n = 8` can say; `scenarios/verdicts.md` for the supermajority read; the #444 thread
(founder traits sampled negative and culled silently at tick 1; 57 % of founders across the
box) and its #438 comments; [`433-energy-bound.md`](433-energy-bound.md) § B2 for the
config sourcing this instrument copies.

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
  compartment (36 of 256 configs). `B_P`, `B_C`, `h_C`, `κ_P`, `κ_C`, `θ_P`, `θ_C`, `α_P`
  and the pair's distance `d` are read at those vectors. Founder noise (`trait_covariance`)
  and everything evolution does afterwards are outside the prediction, deliberately.
- **A1** (`predicted_a1`): `permanent` iff `r_P > 0` (clause 1, `ρ > 1`) and `β·K_P > m`
  (clause 2, `I > 1`) and H1 (`0 < m < 1`, `r_P ≤ 2`); `not-permanent` iff either clause
  fails (both are necessary, and the `r_P > 2` caveat keeps clause 2 necessary);
  `undecided` iff both clauses hold but H1 fails (32 cells, all `r_P > 2`, where the
  sufficiency proof does not apply). The coefficient mapping is copied term for term from
  `permanence_prototype.rs` (a bin cannot be imported) and pinned to that bin's example10
  numbers (`ρ = 71.11`, `I = 23.13`, `r_P = 1.301`, `m = 0.113`, `β = 0.0368`) by unit test.
- **A2** (`predicted_a2`): the coupled reduction's clauses (i) `min(r_P, α_P/θ_P) > μ_P`,
  (ii) `Λ > 1`, (iii) the heteroclinic product, evaluated at the **reference lumping A2's
  own sweep uses** — `ι = 1`, `q = ν = θ_P` (producer carcasses, no free store),
  `μ_P = 0.02`, `e_C = e` — and pinned to example10 (`Λ = 81 334`, product `7.53 > 0.0024`).
  `not-permanent` only where a clause fails **for every value of the endogenous terms**:
  `min(r_P, α_P/θ_P) ≤ 0` (no `μ_P ≥ 0` rescues it) or `Λ = 0` (no heterotrophy).
  `undecided` where a clause fails only at the reference, or H1 fails. Note that **A2's
  theorem does not contain A1's clause (2)**: in the coupled map the producer-only face
  flows into the lockup corner, so the consumer's escape route is the pile, not the
  standing crop. The two columns are therefore different predicates, not a nested pair.

### What is observed

Each (config, seed) is driven through the genesis step loop exactly as
`explorers_genesis::run_single` does — same per-tick observations, same early stops
(empty world, `> max_population`), same `evaluate_from_log` — and the evaluator's terminal
failure mode is the run's outcome. **Observed collapse** = `extinction`, `energy-death` or
`nutrient-lockup` (the extinction boundary attracted, on the energy or the nutrient side);
`none`, `monoculture`, `generalist-dominance` and `explosion` have a living population
bounded away from zero at the horizon and are read as **persist**. Over the 2048 runs the
modes are: none 1682, extinction 249, monoculture 59, nutrient-lockup 45, energy-death 7,
generalist-dominance 6, explosion 0 — 301 collapses, 14.7 %.

Per config the ensemble reduces to the **supermajority read** of `scenarios/verdicts.md`:
`collapse` when 8/8 seeds collapse, `persist` when 0/8 do, `mixed` otherwise (with the
fraction kept). Per #434, `8/8` bounds the true per-seed rate at `p ≥ 0.63`; a `1/8` or
`2/8` split says only that the rate is not zero — at `k = 1`, the exact 95 % interval is
`[0.003, 0.53]`. The instrument also records, per seed, the producer/consumer split of the
roster after tick 1 and at the horizon (`α ≥ h` is a producer, the prototype's rule), the
evaluator's `has_decomposer_guild`, the first tick without a consumer while producers
stood, the peak population, and the #444 exposure.

### How prediction and observation are related

| predicted | observed | class |
|---|---|---|
| permanent | collapse (8/8) | **false positive** — the dangerous direction |
| permanent | mixed | **false positive (mixed)** — reported separately; the mean field makes no per-seed claim |
| permanent | persist | agree |
| not-permanent | collapse | agree |
| not-permanent | persist / mixed, and every persisting seed has **no consumer** at the horizon | **producer-only** — agrees in kind: the pair is not permanent, the producer face is (A1's clause 2 says exactly this) |
| not-permanent | persist / mixed, consumers present | **false negative** (/ mixed) — the safe direction for a certificate, the unsafe one for a gate |
| undecided | any | undecided |

**The #444 tag.** A run is *compartment-DOA* when it seeded at least one negative-trait
founder, a whole compartment the prediction was evaluated on (producers, or consumers when
the config has a distinct consumer centroid) has no member left after tick 1, **and** the
tick-1 losses are no larger than the negative-trait founder count — i.e. the loss is
explained by the #444 cull. A cell is `#444`-tagged when a majority of its collapsing seeds
are compartment-DOA. On such a cell the IBM never held the pair the mean field was
evaluated on, so the collapse is an artefact of #444, not a permanence error; the
false-positive rate is reported with and without those cells. Exposure for context: 2014
of 2048 runs seeded a negative-trait founder (the #438 figure), 111 runs lost a whole
compartment to the cull alone, and 16 cells are tagged.

**Fault hypotheses** are assigned mechanically, first matching rule wins, from A1's
authority boundary plus the two known artefacts:

- *false positive / mixed*: `#444` (tagged) → *first-tick floor* (median population after
  tick 1 on the collapsing seeds is 0: every founder died on tick 1 whatever its sign) →
  *reduction: lockup* (modal collapse mode is nutrient lockup — outside A1's coordinates,
  A2's endogenous terms) → *reduction: energy death* → *spatial* (median first tick without
  a consumer while producers stood ≤ 25 — reach never met the crop) →
  *demographic-stochastic* (a mixed split with none of the above) → *finite-size* (peak
  population never left the founder scale) → *reduction: unclassified*.
- *false negative / mixed*: *reduction: `κ_P = 0`* (clause 1 fails with `ρ > 1`) →
  *reduction: extinction gate* (clause 1 fails with `ρ ≤ 1`; never fires here) → *reduction:
  single mixotroph centroid* (no consumer compartment to evaluate) → *reduction: carcass
  route* (a decomposer guild on a persisting seed) → *reduction: realised traits departed
  the centroid* (consumers persisted with no guild).

## Results

### Confusion matrices

**A1** — `ρ > 1` and `I > 1` at the seeded centroids (rows predicted, columns observed):

| A1 | collapse (8/8) | mixed | persist (0/8) | total |
|---|---|---|---|---|
| permanent | 1 | 22 | 21 | 44 |
| not-permanent | 1 | 87 | 92 | 180 |
| undecided | 0 | 13 | 19 | 32 |
| total | 2 | 122 | 132 | 256 |

**A2** — clauses (i)–(iii) at the reference lumping:

| A2 | collapse (8/8) | mixed | persist (0/8) | total |
|---|---|---|---|---|
| permanent | 2 | 80 | 81 | 163 |
| not-permanent | 0 | 3 | 7 | 10 |
| undecided | 0 | 39 | 44 | 83 |
| total | 2 | 122 | 132 | 256 |

Only **2 of 256 configs collapse on all eight seeds** and 132 persist on all eight; the
search box (as `default_ranges` draws it) is overwhelmingly a persisting region, and the
question this note can actually answer is about the 122 mixed cells and the two hard
collapses. The atlas cells, being live cells, collapse on no seed in 41/56 and on some seed
in 15/56.

### False-positive rate — the dangerous direction

| | predicted permanent | strict FP (8/8 collapse) | any-seed FP (≥ 1 seed collapses) |
|---|---|---|---|
| A1, raw | 44 | **1** (2.3 %) | 23 (52 %) |
| A1, excluding #444-tagged cells | 42 | **1 (2.4 %)** | 21 (50 %) |
| A2, raw | 163 | **2** (1.2 %) | 82 (50 %) |
| A2, excluding #444-tagged cells | 157 | **2 (1.3 %)** | 76 (48 %) |

The strict false positives:

- **`sample:12`** (A1 and A2): 31 founders, 8/8 extinct **on tick 1**. Between 12 and 19
  founders per seed carry a negative trait (#444), but all 31 die — the non-negative ones
  too. The config's `trait_covariance` is large enough that the founders are
  near-unit-fragility generalists, and the peak-relative death threshold (#312) kills them
  on their first tick of structure loss. The biomass map has no per-body coordinate at all,
  so this is a **reduction regime exit at tick 0** rather than a permanence error: the map's
  `P(0) > 0` never existed. It is *not* tagged #444 (the tick-1 losses exceed the cull), and
  it is counted against the predictions on the honest denominator.
- **`sample:147`** (A2 only; A1 says not-permanent, `I = 0.015`, and so agrees with the
  collapse): 7/8 `nutrient-lockup`, 1/8 extinction, peak population 657–948 — a large,
  predation-heavy world that silts its nutrient into the dead pool. `Λ = 18.8` at the
  reference lumping says the lockup corner repels; the reference sets `ι = 1` (every
  carcass within every consumer's reach), and this is the cell where that over-serving
  bit. It is the empirical twin of A2's own example10 reading and the reason A2 calls `Λ`
  a characterisation, not a gate.

### The mixed cells under a permanent prediction

The 22 A1 cells and 80 A2 cells that lose 1–7 of 8 seeds are, by A4's rule, false
positives on the any-seed read. By fault class (A1 / A2):

| fault hypothesis | A1 mixed cells | A2 mixed cells |
|---|---|---|
| `#444` — a compartment emptied by the tick-1 cull on a majority of the collapsing seeds | 2 | 6 |
| demographic-stochastic — 1–5 seeds collapse, no structural signature | 13 | 45 |
| spatial — consumers lost by tick ≤ 25 while producers stood (example10) | 7 | 22 |
| reduction — nutrient lockup (A1's coordinates lack the ledger; A2's `ι/ν/q` are endogenous) | 0 | 3 |
| reduction — first-tick floor: every founder dies on tick 1 whatever its trait sign | 0 | 4 |

The A1 split by collapse count is 1/8 × 9, 2/8 × 8, 3/8 × 3, 4/8 × 2 — mostly single-seed
losses, which #434 says an `n = 8` block cannot distinguish from a per-seed rate anywhere
below ~0.5. These are the authority boundary's "no demographic noise / no space" clauses
operating as described, and they are the reason `I > 1` cannot be read as a per-seed
promise. Full per-cell rows are in the appendix.

### The negative direction

A1 predicts **not-permanent on 180 cells; 1 collapses (8/8), 92 persist on every seed, 87
are mixed.** After the producer-only credit (9 cells whose persisting seeds carry no
consumer at the horizon — the producer face alone, which is what a failed clause 2 says),
90 false negatives and 80 mixed remain. Their hypotheses:

| fault hypothesis | cells |
|---|---|
| carcass route — a decomposer guild present on persisting seeds; `I ≤ 1` but `Λ ≫ 1` | 138 |
| `κ_P = 0` zeroes `r_P` although `ρ > 1` (10 atlas cells; 7 persist 8/8) | 10 |
| single mixotroph centroid — no consumer compartment to evaluate | 12 |
| consumers persisted with no guild — realised traits departed the centroid | 10 |

Two things follow.

**A1's clause (2) is not the system's boundary.** `I = κ_C·γ·base·e^{−λd}·h_C·F/(B_P·B_C)`
exceeds 1 on only 76 of 256 configs (median 0.21), because the pair distance `d` between a
pure producer and a pure consumer centroid is `√2·(α+h)` plus the shared dimensions, and
`trophic_distance_decay` runs to 5 — the living-prey kernel is small across most of the
box. Yet heterotrophs persist almost everywhere, and the evaluator sees a decomposer
guild on 138 of the 170 disagreeing cells. This is A2's finding read off
the IBM: the pile, not the crop, is the heterotroph's escape route, and `Λ`
(`∝ N_total/ν`, with `N_total = 50 000` fixed by the viable baseline and not searched) is
in the hundreds to tens of thousands at the reference lumping. A gate that killed `I ≤ 1`
would kill 179 of the 180 cells it flagged; the "mean-field non-permanent ⇒ the expected
trajectory goes extinct" reading of A1's authority boundary holds for the *pair on the
living-prey route*, not for the world.

**`κ_P = 0` is a lumping error.** All ten clause-(1) failures are atlas cells with
`mean_kappa = 0.0` (the search box's lower edge, which QD evidently likes), `ρ` from 17 to
474, persisting on 8/8 seeds in seven of them. A1 lumps `r_P = κ_P·γ·(F − B_P)`: the
somatic share of net income. At `κ_P = 0` the whole surplus is earmarked for reproduction
and becomes offspring — biomass the map is supposed to carry ("whether structure sits in
one body or two is invisible to it"), but which the coefficient drops. The theorem is
untouched; the table row for `r_P` is what A4 should correct (`γ·(F − B_P)` up to the
reproduction efficiency, with `κ_P` governing only how the biomass is split). Until then
the extinction clause is `F > B_P` **and** `κ_P > 0`, and the second conjunct is spurious.

### A2's not-permanent and undecided cells

A2 calls only 10 cells not-permanent — the same ten `κ_P = 0` cells (clause (i)'s
`min(r_P, α_P/θ_P) ≤ 0` inherits the lumping error). 83 cells are undecided: 69 for H1
(`r_P > 2`); of the other 14, clause (i) holds only for `μ_P < 0.02` in 6 (a producer whose
Liebig-limited invasion rate sits in `(0, 0.02]`), `Λ ≤ 1` at the reference in 8 (one cell
both), and (iii) alone fails in 1. None of the undecided cells collapses 8/8; 39 are mixed.

## What eight seeds can and cannot say here

The strict read (`8/8` / `0/8`) is the only one #434 licenses at `n = 8`, and on that read
the dangerous direction has one cell (A1) and two (A2) with a named cause — a tick-0
regime exit, and an over-served pile. The
any-seed read is where the honest uncertainty sits: a `1/8` loss is consistent with a true
per-seed collapse rate anywhere in `[0.003, 0.53]`, so the 9 A1 cells losing exactly one seed
may be robust worlds with a rare bad draw or coin-flips — `n = 32` (the projection's
refinement block) would separate 0.35 from 0.65 at 5 %, and is the right next instrument
if A4 wants the mixed cells resolved rather than tallied. Nothing in this note depends on
resolving them: the verdict below is the same whether every mixed cell is read as a false
positive or as noise, because the negative direction, not the positive one, is what
disqualifies clause (2) as a gate.

## Verdict — promote-with-caveat

Applying A4's rule (*zero false positives → gate-grade; any FP → promote as
characterisation citing the failure cells*), per predicate:

- **A1 clause (1), `ρ = F/B_P > 1` (the sharpened extinction gate): promote, with two
  caveats.** No cell contradicts it, but no cell exercises it either (`ρ ≥ 1.9` throughout
  the box); and its `κ_P` conjunct is a lumping error that produces 10 false negatives on
  atlas cells and must be dropped before promotion. The negative direction (`ρ ≤ 1 ⇒`
  extinction) inherits the standing gate's proof and needs no new evidence; the positive
  direction promises nothing, as the mixed cells show.
- **A1 clause (2), `I > 1`: promote as characterisation only.** Strict FP is 1/42 on the
  honest denominator (`sample:12`, a tick-0 regime exit) and 21/42 on the any-seed read,
  and the negative direction is wrong on 179/180 cells because the coupled system has a
  second route. It characterises whether
  the *living-prey* route is open — a useful descriptor for the atlas, not a gate.
- **A2 `Λ > 1` at the reference lumping: promote as characterisation only, citing
  `sample:147`.** One genuine strict failure, where the endogenous reach `ι` is the term
  that bit — exactly A2's stated contingency. Its config-only kills (`Λ = 0`) fire on no
  cell in the box; its undecided band is a third of the box.
- **Not gate-grade:** the conjunction A1 ∧ A2 as a whole. Its one strict false positive
  on the honest denominator is a world the map never had (`P(0)` gone on tick 1), its
  any-seed false-positive rate is ~50 %, and the box never tests its extinction clause.

For A4, the actionable items are: correct A1's `r_P` row; carry `I` and `Λ` as atlas
descriptors (per cell, at the seeded centroids) rather than gates; and, if a gate on
the pile route is ever wanted, take A2's reserve remedy (a reach bound `ῑ` and a carcass
floor `ν̲`) rather than the reference lumping — `sample:147` is the counter-example that
would falsify any `Λ` gate drawn without them.

## Reproduce

```
cargo run --release -p explorers-search --bin permanence_crosscheck            # ~10 min, 2048 runs
cargo test -p explorers-search --bin permanence_crosscheck                     # pure parts, < 1 s
PERMANENCE_CROSSCHECK_CONFIGS=sample:147,sample:12 PERMANENCE_CROSSCHECK_SEEDS=8 \
  cargo run --release -p explorers-search --bin permanence_crosscheck          # the two strict FPs
```

Artifact: `target/permanence-crosscheck.json` — per config the two verdicts with every
coefficient, the per-seed outcomes, the aggregate, and the agreement class and
hypothesis; plus the summary printed to stdout. Determinism: two consecutive full runs
give byte-identical artifacts (SHA-1 compared; one config per rayon task, seeds in an
order-stable inner collect).

## Deliverables against the acceptance criteria

- Instrument runs deterministically and covers the atlas live cells + ≥ 200 sampled
  configs × ≥ 8 seeds — 56 + 200 × 8, identical artifact across two runs.
- Confusion matrix and false-positive rate — *Results*, A1 and A2, raw and excluding
  #444-tagged cells, strict and any-seed.
- Every disagreeing config listed with a fault hypothesis — *Appendix*.
- Explicit verdict — *promote-with-caveat*, per predicate, with the A4 actions.
- No assertions on emergent values beyond a smoke check — the bin's tests pin the
  coefficient mapping to the prototype's example10 numbers and the pure aggregation
  rules; the one run-producing test asserts only that records were produced.
- No change to the stepper, evaluator scoring, or search — the only code is the bin (and
  a `explorers-genesis-eval` dependency line in the search crate so the bin can call
  `evaluate_from_log` as `run_single` does).

## Appendix — every disagreement

Columns: predicted, observed (collapsing seeds / n), modal terminal mode, `ρ`, `I`, `Λ` at
the reference lumping, #444 tag, hypothesis. Generated from the artifact; `atlas:i` is the
i-th live cell of `atlas.json`, `sample:i` the i-th point of the seed-421 LHS draw.

### A1 — false positives (strict and mixed)

| cell | predicted | observed | modal mode | ρ | I | Λ | #444 | fault hypothesis |
|---|---|---|---|---|---|---|---|---|
| `atlas:5` | permanent | mixed (1/8) | none | 22.6 | 5.03 | 1.45e+04 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:30` | permanent | mixed (1/8) | none | 65.7 | 2.08 | 7.01e+03 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:35` | permanent | mixed (3/8) | none | 100.5 | 21.69 | 8.23e+03 |  | spatial: consumers lost by tick 12 while producers stood - reach never met the crop (example10 signature) |
| `sample:12` | permanent | collapse (8/8) | extinction | 11.2 | 2.09 | 7.58e+03 |  | reduction: every founder dies on tick 1 whatever its trait sign - a first-tick per-body floor (the peak-relative death threshold, endowment, embodiment) the biomass map has no coordinate for |
| `sample:14` | permanent | mixed (1/8) | none | 81.1 | 1.17 | 986 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:29` | permanent | mixed (4/8) | none | 32.9 | 1.11 | 2.73e+03 |  | demographic-stochastic: 4/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:38` | permanent | mixed (1/8) | none | 29.7 | 2.65 | 4.62e+03 |  | spatial: consumers lost by tick 23 while producers stood - reach never met the crop (example10 signature) |
| `sample:50` | permanent | mixed (1/8) | none | 22.7 | 3.12 | 9.75e+03 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:55` | permanent | mixed (2/8) | none | 141.4 | 3.16 | 5.9e+03 |  | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:59` | permanent | mixed (1/8) | none | 115.9 | 8.87 | 7.01e+03 | yes | #444 artefact: a compartment had no non-negative founder after tick 1 on 1/1 collapsing seeds |
| `sample:75` | permanent | mixed (2/8) | none | 176.2 | 25.29 | 9.78e+03 |  | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:79` | permanent | mixed (1/8) | none | 29.2 | 3.72 | 7.42e+03 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:95` | permanent | mixed (2/8) | none | 148.5 | 9.06 | 4.61e+03 |  | spatial: consumers lost by tick 5 while producers stood - reach never met the crop (example10 signature) |
| `sample:96` | permanent | mixed (2/8) | none | 168.2 | 3.85 | 2.01e+03 |  | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:105` | permanent | mixed (1/8) | none | 29.0 | 1.75 | 9.16e+03 |  | spatial: consumers lost by tick 16 while producers stood - reach never met the crop (example10 signature) |
| `sample:106` | permanent | mixed (3/8) | none | 6.2 | 1.10 | 1.68e+04 |  | demographic-stochastic: 3/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:107` | permanent | mixed (1/8) | none | 91.3 | 2.47 | 2.05e+03 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:120` | permanent | mixed (2/8) | none | 5.6 | 1.39 | 1.03e+04 |  | spatial: consumers lost by tick 11 while producers stood - reach never met the crop (example10 signature) |
| `sample:131` | permanent | mixed (4/8) | none | 52.6 | 11.87 | 2.9e+04 | yes | #444 artefact: a compartment had no non-negative founder after tick 1 on 3/4 collapsing seeds |
| `sample:136` | permanent | mixed (2/8) | none | 257.6 | 1.24 | 464 |  | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:172` | permanent | mixed (2/8) | none | 29.3 | 2.41 | 5.91e+03 |  | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:176` | permanent | mixed (2/8) | none | 177.2 | 1.54 | 554 |  | spatial: consumers lost by tick 1 while producers stood - reach never met the crop (example10 signature) |
| `sample:180` | permanent | mixed (3/8) | none | 56.8 | 39.76 | 5.58e+04 |  | spatial: consumers lost by tick 15 while producers stood - reach never met the crop (example10 signature) |

### A2 — false positives (strict and mixed)

| cell | predicted | observed | modal mode | ρ | I | Λ | #444 | fault hypothesis |
|---|---|---|---|---|---|---|---|---|
| `atlas:5` | permanent | mixed (1/8) | none | 22.6 | 5.03 | 1.45e+04 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:10` | permanent | mixed (1/8) | none | 32.2 | 0.01 | 6.9 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:13` | permanent | mixed (4/8) | none | 16.3 | 0.00 | 4.95 |  | demographic-stochastic: 4/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:21` | permanent | mixed (1/8) | none | 37.8 | 0.24 | 481 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:24` | permanent | mixed (1/8) | none | 44.1 | 0.68 | 954 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:30` | permanent | mixed (1/8) | none | 65.7 | 2.08 | 7.01e+03 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:31` | permanent | mixed (2/8) | none | 65.9 | 0.30 | 378 |  | spatial: consumers lost by tick 8 while producers stood - reach never met the crop (example10 signature) |
| `atlas:35` | permanent | mixed (3/8) | none | 100.5 | 21.69 | 8.23e+03 |  | spatial: consumers lost by tick 12 while producers stood - reach never met the crop (example10 signature) |
| `atlas:36` | permanent | mixed (1/8) | none | 73.9 | 0.12 | 68 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:41` | permanent | mixed (1/8) | none | 27.7 | 0.20 | 417 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `atlas:47` | permanent | mixed (1/8) | none | 77.4 | 0.08 | 43.7 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:7` | permanent | mixed (1/8) | none | 16.6 | 0.05 | 245 |  | spatial: consumers lost by tick 9 while producers stood - reach never met the crop (example10 signature) |
| `sample:8` | permanent | mixed (1/8) | none | 19.0 | 0.01 | 225 |  | reduction: every founder dies on tick 1 whatever its trait sign - a first-tick per-body floor (the peak-relative death threshold, endowment, embodiment) the biomass map has no coordinate for |
| `sample:9` | permanent | mixed (5/8) | extinction | 48.8 | 0.45 | 1.05e+03 |  | spatial: consumers lost by tick 1 while producers stood - reach never met the crop (example10 signature) |
| `sample:11` | permanent | mixed (3/8) | none | 7.7 | 0.34 | 2.46e+03 |  | demographic-stochastic: 3/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:12` | permanent | collapse (8/8) | extinction | 11.2 | 2.09 | 7.58e+03 |  | reduction: every founder dies on tick 1 whatever its trait sign - a first-tick per-body floor (the peak-relative death threshold, endowment, embodiment) the biomass map has no coordinate for |
| `sample:14` | permanent | mixed (1/8) | none | 81.1 | 1.17 | 986 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:22` | permanent | mixed (1/8) | none | 9.4 | 0.11 | 980 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:26` | permanent | mixed (2/8) | none | 29.0 | 0.00 | 9.65 |  | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:27` | permanent | mixed (1/8) | none | 27.9 | 0.01 | 9.84 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:29` | permanent | mixed (4/8) | none | 32.9 | 1.11 | 2.73e+03 |  | demographic-stochastic: 4/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:30` | permanent | mixed (1/8) | none | 30.9 | 0.11 | 197 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:38` | permanent | mixed (1/8) | none | 29.7 | 2.65 | 4.62e+03 |  | spatial: consumers lost by tick 23 while producers stood - reach never met the crop (example10 signature) |
| `sample:43` | permanent | mixed (2/8) | none | 21.1 | 0.21 | 701 |  | spatial: consumers lost by tick 4 while producers stood - reach never met the crop (example10 signature) |
| `sample:46` | permanent | mixed (5/8) | extinction | 18.2 | 0.06 | 283 |  | spatial: consumers lost by tick 7 while producers stood - reach never met the crop (example10 signature) |
| `sample:47` | permanent | mixed (4/8) | none | 40.3 | 0.10 | 283 | yes | #444 artefact: a compartment had no non-negative founder after tick 1 on 4/4 collapsing seeds |
| `sample:49` | permanent | mixed (2/8) | none | 92.4 | 0.39 | 330 |  | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:50` | permanent | mixed (1/8) | none | 22.7 | 3.12 | 9.75e+03 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:52` | permanent | mixed (1/8) | none | 9.8 | 0.00 | 22.4 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:53` | permanent | mixed (1/8) | none | 28.3 | 0.01 | 25.4 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:55` | permanent | mixed (2/8) | none | 141.4 | 3.16 | 5.9e+03 |  | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:59` | permanent | mixed (1/8) | none | 115.9 | 8.87 | 7.01e+03 | yes | #444 artefact: a compartment had no non-negative founder after tick 1 on 1/1 collapsing seeds |
| `sample:64` | permanent | mixed (3/8) | none | 67.1 | 0.09 | 74.3 |  | demographic-stochastic: 3/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:68` | permanent | mixed (3/8) | none | 95.9 | 0.11 | 48.4 |  | demographic-stochastic: 3/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:70` | permanent | mixed (2/8) | none | 2.7 | 0.00 | 14.2 |  | spatial: consumers lost by tick 17 while producers stood - reach never met the crop (example10 signature) |
| `sample:75` | permanent | mixed (2/8) | none | 176.2 | 25.29 | 9.78e+03 |  | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:76` | permanent | mixed (2/8) | none | 8.0 | 0.37 | 6.26e+03 |  | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:79` | permanent | mixed (1/8) | none | 29.2 | 3.72 | 7.42e+03 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:82` | permanent | mixed (3/8) | none | 13.8 | 0.31 | 2.23e+03 |  | spatial: consumers lost by tick 12 while producers stood - reach never met the crop (example10 signature) |
| `sample:83` | permanent | mixed (3/8) | none | 16.2 | 0.30 | 2.62e+03 |  | demographic-stochastic: 3/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:88` | permanent | mixed (1/8) | none | 45.3 | 0.46 | 589 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:93` | permanent | mixed (2/8) | none | 21.5 | 0.16 | 696 |  | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:94` | permanent | mixed (3/8) | none | 28.6 | 0.16 | 1.27e+03 |  | reduction: every founder dies on tick 1 whatever its trait sign - a first-tick per-body floor (the peak-relative death threshold, endowment, embodiment) the biomass map has no coordinate for |
| `sample:95` | permanent | mixed (2/8) | none | 148.5 | 9.06 | 4.61e+03 |  | spatial: consumers lost by tick 5 while producers stood - reach never met the crop (example10 signature) |
| `sample:96` | permanent | mixed (2/8) | none | 168.2 | 3.85 | 2.01e+03 |  | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:99` | permanent | mixed (5/8) | extinction | 39.9 | 0.03 | 85.4 |  | spatial: consumers lost by tick 6 while producers stood - reach never met the crop (example10 signature) |
| `sample:100` | permanent | mixed (2/8) | none | 11.2 | 0.03 | 162 |  | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:101` | permanent | mixed (3/8) | none | 31.3 | 0.03 | 67.4 |  | demographic-stochastic: 3/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:105` | permanent | mixed (1/8) | none | 29.0 | 1.75 | 9.16e+03 |  | spatial: consumers lost by tick 16 while producers stood - reach never met the crop (example10 signature) |
| `sample:106` | permanent | mixed (3/8) | none | 6.2 | 1.10 | 1.68e+04 |  | demographic-stochastic: 3/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:107` | permanent | mixed (1/8) | none | 91.3 | 2.47 | 2.05e+03 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:108` | permanent | mixed (2/8) | none | 18.5 | 0.01 | 28.4 |  | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:118` | permanent | mixed (1/8) | none | 9.7 | 0.45 | 5.35e+03 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:120` | permanent | mixed (2/8) | none | 5.6 | 1.39 | 1.03e+04 |  | spatial: consumers lost by tick 11 while producers stood - reach never met the crop (example10 signature) |
| `sample:122` | permanent | mixed (2/8) | none | 26.5 | 0.22 | 458 |  | spatial: consumers lost by tick 11 while producers stood - reach never met the crop (example10 signature) |
| `sample:127` | permanent | mixed (1/8) | none | 31.1 | 0.17 | 345 | yes | #444 artefact: a compartment had no non-negative founder after tick 1 on 1/1 collapsing seeds |
| `sample:129` | permanent | mixed (4/8) | nutrient-lockup | 16.4 | 0.00 | 18.4 |  | reduction: nutrient lockup is outside A1's coordinates (A2's iota/nu/q are endogenous) |
| `sample:131` | permanent | mixed (4/8) | none | 52.6 | 11.87 | 2.9e+04 | yes | #444 artefact: a compartment had no non-negative founder after tick 1 on 3/4 collapsing seeds |
| `sample:135` | permanent | mixed (5/8) | extinction | 8.5 | 0.01 | 95.3 |  | demographic-stochastic: 5/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:136` | permanent | mixed (2/8) | none | 257.6 | 1.24 | 464 |  | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:142` | permanent | mixed (4/8) | none | 23.0 | 0.01 | 17.1 |  | spatial: consumers lost by tick 17 while producers stood - reach never met the crop (example10 signature) |
| `sample:145` | permanent | mixed (2/8) | none | 4.6 | 0.86 | 1.53e+04 |  | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:146` | permanent | mixed (3/8) | none | 45.0 | 0.25 | 571 |  | demographic-stochastic: 3/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:147` | permanent | collapse (8/8) | nutrient-lockup | 30.8 | 0.01 | 18.8 |  | reduction: nutrient lockup is outside A1's coordinates (A2's iota/nu/q are endogenous) |
| `sample:152` | permanent | mixed (4/8) | extinction | 9.6 | 0.30 | 2.8e+03 |  | spatial: consumers lost by tick 1 while producers stood - reach never met the crop (example10 signature) |
| `sample:153` | permanent | mixed (5/8) | extinction | 40.2 | 0.35 | 1.77e+03 |  | demographic-stochastic: 5/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:154` | permanent | mixed (6/8) | nutrient-lockup | 40.4 | 0.05 | 120 |  | reduction: nutrient lockup is outside A1's coordinates (A2's iota/nu/q are endogenous) |
| `sample:155` | permanent | mixed (1/8) | none | 33.7 | 0.04 | 61.7 | yes | #444 artefact: a compartment had no non-negative founder after tick 1 on 1/1 collapsing seeds |
| `sample:158` | permanent | mixed (5/8) | extinction | 9.8 | 0.00 | 10.6 |  | spatial: consumers lost by tick 10 while producers stood - reach never met the crop (example10 signature) |
| `sample:165` | permanent | mixed (2/8) | none | 66.4 | 0.20 | 212 |  | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:169` | permanent | mixed (3/8) | none | 32.6 | 0.00 | 6.92 | yes | #444 artefact: a compartment had no non-negative founder after tick 1 on 2/3 collapsing seeds |
| `sample:171` | permanent | mixed (4/8) | none | 21.2 | 0.03 | 534 |  | reduction: every founder dies on tick 1 whatever its trait sign - a first-tick per-body floor (the peak-relative death threshold, endowment, embodiment) the biomass map has no coordinate for |
| `sample:172` | permanent | mixed (2/8) | none | 29.3 | 2.41 | 5.91e+03 |  | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:173` | permanent | mixed (2/8) | none | 10.3 | 0.01 | 80 |  | spatial: consumers lost by tick 20 while producers stood - reach never met the crop (example10 signature) |
| `sample:176` | permanent | mixed (2/8) | none | 177.2 | 1.54 | 554 |  | spatial: consumers lost by tick 1 while producers stood - reach never met the crop (example10 signature) |
| `sample:177` | permanent | mixed (2/8) | none | 34.4 | 0.11 | 303 |  | demographic-stochastic: 2/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:178` | permanent | mixed (2/8) | none | 45.7 | 0.01 | 30.2 |  | spatial: consumers lost by tick 23 while producers stood - reach never met the crop (example10 signature) |
| `sample:180` | permanent | mixed (3/8) | none | 56.8 | 39.76 | 5.58e+04 |  | spatial: consumers lost by tick 15 while producers stood - reach never met the crop (example10 signature) |
| `sample:186` | permanent | mixed (1/8) | none | 47.8 | 0.21 | 583 |  | reduction: every founder dies on tick 1 whatever its trait sign - a first-tick per-body floor (the peak-relative death threshold, endowment, embodiment) the biomass map has no coordinate for |
| `sample:187` | permanent | mixed (1/8) | none | 47.0 | 0.16 | 181 |  | demographic-stochastic: 1/8 seeds collapse - a per-seed split the mean field does not resolve |
| `sample:192` | permanent | mixed (7/8) | nutrient-lockup | 35.6 | 0.02 | 53.2 |  | reduction: nutrient lockup is outside A1's coordinates (A2's iota/nu/q are endogenous) |
| `sample:193` | permanent | mixed (1/8) | none | 16.5 | 0.41 | 1.51e+03 |  | spatial: consumers lost by tick 21 while producers stood - reach never met the crop (example10 signature) |

### A1 — false negatives (strict and mixed)

| cell | predicted | observed | modal mode | ρ | I | Λ | #444 | fault hypothesis |
|---|---|---|---|---|---|---|---|---|
| `atlas:1` | not-permanent | persist (0/8) | none | 20.5 | 0.01 | 15.9 |  | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `atlas:2` | not-permanent | persist (0/8) | none | 37.4 | 0.05 | 78.6 |  | reduction: consumers persisted on 2/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `atlas:3` | not-permanent | persist (0/8) | none | 22.2 | 0.04 | 71 |  | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `atlas:6` | not-permanent | persist (0/8) | none | 88.3 | 0.21 | 175 |  | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `atlas:7` | not-permanent | persist (0/8) | none | 62.8 | 0.00 | 1.95 |  | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `atlas:8` | not-permanent | persist (0/8) | none | 26.5 | 0.00 | 0 |  | reduction: kappa_P = 0 zeroes r_P = kappa_P*gamma*(F - B_P) although rho = 26.5 > 1 - the reproductive share still becomes offspring biomass (the map's kappa lumping, not the extinction gate) |
| `atlas:9` | not-permanent | persist (0/8) | none | 6.9 | 0.02 | 112 |  | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 7 of them - A2's carcass route, which A1's map lacks |
| `atlas:10` | not-permanent | mixed (1/8) | none | 32.2 | 0.01 | 6.9 |  | reduction: consumers persisted on 5/7 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `atlas:11` | not-permanent | persist (0/8) | none | 18.4 | 0.00 | 0 |  | reduction: kappa_P = 0 zeroes r_P = kappa_P*gamma*(F - B_P) although rho = 18.4 > 1 - the reproductive share still becomes offspring biomass (the map's kappa lumping, not the extinction gate) |
| `atlas:12` | not-permanent | persist (0/8) | none | 29.1 | 0.00 | 0 |  | reduction: kappa_P = 0 zeroes r_P = kappa_P*gamma*(F - B_P) although rho = 29.1 > 1 - the reproductive share still becomes offspring biomass (the map's kappa lumping, not the extinction gate) |
| `atlas:13` | not-permanent | mixed (4/8) | none | 16.3 | 0.00 | 4.95 |  | reduction: consumers persisted on 3/4 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `atlas:15` | not-permanent | persist (0/8) | monoculture | 163.8 | 0.00 | 0 |  | reduction: kappa_P = 0 zeroes r_P = kappa_P*gamma*(F - B_P) although rho = 163.8 > 1 - the reproductive share still becomes offspring biomass (the map's kappa lumping, not the extinction gate) |
| `atlas:17` | not-permanent | persist (0/8) | none | 66.5 | 0.00 | 0 |  | reduction: kappa_P = 0 zeroes r_P = kappa_P*gamma*(F - B_P) although rho = 66.5 > 1 - the reproductive share still becomes offspring biomass (the map's kappa lumping, not the extinction gate) |
| `atlas:18` | not-permanent | persist (0/8) | none | 28.0 | 0.00 | 1.72 |  | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `atlas:19` | not-permanent | persist (0/8) | none | 34.4 | 0.02 | 27.5 |  | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `atlas:20` | not-permanent | persist (0/8) | none | 32.6 | 0.00 | 0.0783 |  | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `atlas:21` | not-permanent | mixed (1/8) | none | 37.8 | 0.24 | 481 |  | reduction: consumers persisted on 5/7 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `atlas:23` | not-permanent | persist (0/8) | none | 26.2 | 0.15 | 446 |  | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 6 of them - A2's carcass route, which A1's map lacks |
| `atlas:24` | not-permanent | mixed (1/8) | none | 44.1 | 0.68 | 954 |  | reduction: consumers persisted on 2/7 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `atlas:25` | not-permanent | persist (0/8) | none | 18.6 | 0.00 | 2.98 |  | reduction: consumers persisted on 1/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `atlas:26` | not-permanent | persist (0/8) | none | 14.3 | 0.01 | 35.4 |  | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 6 of them - A2's carcass route, which A1's map lacks |
| `atlas:27` | not-permanent | persist (0/8) | none | 23.2 | 0.03 | 57.3 |  | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `atlas:28` | not-permanent | mixed (2/8) | none | 474.0 | 0.00 | 0 |  | reduction: kappa_P = 0 zeroes r_P = kappa_P*gamma*(F - B_P) although rho = 474.0 > 1 - the reproductive share still becomes offspring biomass (the map's kappa lumping, not the extinction gate) |
| `atlas:29` | not-permanent | persist (0/8) | none | 26.5 | 0.03 | 48 |  | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `atlas:31` | not-permanent | mixed (2/8) | none | 65.9 | 0.30 | 378 |  | reduction: consumers persisted on 3/6 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `atlas:32` | not-permanent | persist (0/8) | none | 95.8 | 0.01 | 3.78 |  | reduction: consumers persisted on 1/8 seeds despite I <= 1 and no decomposer guild - realised traits departed the centroid (evolution / draw) |
| `atlas:34` | not-permanent | persist (0/8) | none | 17.5 | 0.01 | 15.8 |  | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `atlas:36` | not-permanent | mixed (1/8) | none | 73.9 | 0.12 | 68 |  | reduction: consumers persisted on 5/7 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `atlas:37` | not-permanent | mixed (1/8) | none | 26.8 | 0.00 | 0 |  | reduction: kappa_P = 0 zeroes r_P = kappa_P*gamma*(F - B_P) although rho = 26.8 > 1 - the reproductive share still becomes offspring biomass (the map's kappa lumping, not the extinction gate) |
| `atlas:38` | not-permanent | persist (0/8) | none | 13.2 | 0.66 | 2.86e+03 |  | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 5/8 persisting seeds |
| `atlas:40` | not-permanent | mixed (1/8) | none | 17.3 | 0.00 | 0 |  | reduction: kappa_P = 0 zeroes r_P = kappa_P*gamma*(F - B_P) although rho = 17.3 > 1 - the reproductive share still becomes offspring biomass (the map's kappa lumping, not the extinction gate) |
| `atlas:41` | not-permanent | mixed (1/8) | none | 27.7 | 0.20 | 417 |  | reduction: consumers persisted on 6/7 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `atlas:43` | not-permanent | persist (0/8) | none | 24.1 | 0.01 | 19.5 |  | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 6 of them - A2's carcass route, which A1's map lacks |
| `atlas:44` | not-permanent | persist (0/8) | none | 52.1 | 0.08 | 98.8 |  | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `atlas:45` | not-permanent | persist (0/8) | none | 155.4 | 0.83 | 341 |  | reduction: consumers persisted on 2/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `atlas:46` | not-permanent | persist (0/8) | none | 10.8 | 0.01 | 50 |  | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 6 of them - A2's carcass route, which A1's map lacks |
| `atlas:47` | not-permanent | mixed (1/8) | none | 77.4 | 0.08 | 43.7 |  | reduction: consumers persisted on 1/7 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `atlas:48` | not-permanent | persist (0/8) | none | 34.7 | 0.00 | 0 |  | reduction: kappa_P = 0 zeroes r_P = kappa_P*gamma*(F - B_P) although rho = 34.7 > 1 - the reproductive share still becomes offspring biomass (the map's kappa lumping, not the extinction gate) |
| `atlas:49` | not-permanent | persist (0/8) | none | 15.6 | 0.01 | 34.1 |  | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `atlas:52` | not-permanent | persist (0/8) | none | 20.5 | 0.01 | 15.9 |  | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `atlas:54` | not-permanent | persist (0/8) | none | 35.8 | 0.00 | 0 |  | reduction: kappa_P = 0 zeroes r_P = kappa_P*gamma*(F - B_P) although rho = 35.8 > 1 - the reproductive share still becomes offspring biomass (the map's kappa lumping, not the extinction gate) |
| `sample:1` | not-permanent | persist (0/8) | none | 55.0 | 0.22 | 193 |  | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 7 of them - A2's carcass route, which A1's map lacks |
| `sample:2` | not-permanent | persist (0/8) | none | 16.9 | 0.02 | 102 |  | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 8 of them - A2's carcass route, which A1's map lacks |
| `sample:3` | not-permanent | mixed (1/8) | none | 21.4 | 0.03 | 102 |  | reduction: consumers persisted on 3/7 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:5` | not-permanent | mixed (1/8) | none | 3.6 | 0.01 | 139 |  | reduction: consumers persisted on 6/7 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:6` | not-permanent | persist (0/8) | none | 15.7 | 0.72 | 3.83e+03 |  | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 7/8 persisting seeds |
| `sample:7` | not-permanent | mixed (1/8) | none | 16.6 | 0.05 | 245 |  | reduction: consumers persisted on 1/7 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:8` | not-permanent | mixed (1/8) | none | 19.0 | 0.01 | 225 |  | reduction: consumers persisted on 5/7 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:9` | not-permanent | mixed (5/8) | extinction | 48.8 | 0.45 | 1.05e+03 |  | reduction: consumers persisted on 2/3 seeds despite I <= 1 and no decomposer guild - realised traits departed the centroid (evolution / draw) |
| `sample:10` | not-permanent | persist (0/8) | none | 20.3 | 0.33 | 1.94e+03 |  | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:11` | not-permanent | mixed (3/8) | none | 7.7 | 0.34 | 2.46e+03 |  | reduction: consumers persisted on 3/5 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:13` | not-permanent | persist (0/8) | none | 28.9 | 0.00 | 4.22 |  | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:16` | not-permanent | persist (0/8) | none | 69.4 | 0.81 | 664 |  | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:17` | not-permanent | mixed (1/8) | none | 80.8 | 0.06 | 93.7 |  | reduction: consumers persisted on 4/7 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:18` | not-permanent | persist (0/8) | none | 17.2 | 0.01 | 30.5 |  | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 7 of them - A2's carcass route, which A1's map lacks |
| `sample:19` | not-permanent | mixed (1/8) | none | 73.1 | 0.37 | 377 | yes | reduction: consumers persisted on 4/7 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:21` | not-permanent | persist (0/8) | none | 20.3 | 0.02 | 105 |  | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:22` | not-permanent | mixed (1/8) | none | 9.4 | 0.11 | 980 |  | reduction: consumers persisted on 3/7 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:23` | not-permanent | persist (0/8) | none | 50.8 | 0.07 | 173 |  | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:24` | not-permanent | mixed (1/8) | none | 30.9 | 0.10 | 412 | yes | reduction: consumers persisted on 3/7 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:25` | not-permanent | mixed (1/8) | none | 83.8 | 0.30 | 337 | yes | reduction: consumers persisted on 5/7 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:26` | not-permanent | mixed (2/8) | none | 29.0 | 0.00 | 9.65 |  | reduction: consumers persisted on 2/6 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:27` | not-permanent | mixed (1/8) | none | 27.9 | 0.01 | 9.84 |  | reduction: consumers persisted on 3/7 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:30` | not-permanent | mixed (1/8) | none | 30.9 | 0.11 | 197 |  | reduction: consumers persisted on 4/7 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:31` | not-permanent | persist (0/8) | none | 86.1 | 0.18 | 339 |  | reduction: consumers persisted on 1/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:34` | not-permanent | persist (0/8) | none | 9.5 | 0.46 | 8.21e+03 |  | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:35` | not-permanent | mixed (3/8) | none | 22.9 | 0.15 | 339 |  | reduction: consumers persisted on 3/5 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:41` | not-permanent | mixed (3/8) | none | 41.8 | 0.05 | 82.4 | yes | reduction: consumers persisted on 2/5 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:43` | not-permanent | mixed (2/8) | none | 21.1 | 0.21 | 701 |  | reduction: consumers persisted on 4/6 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:44` | not-permanent | persist (0/8) | none | 82.8 | 0.02 | 18.8 |  | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:45` | not-permanent | mixed (1/8) | none | 35.2 | 0.03 | 71.4 |  | reduction: consumers persisted on 5/7 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:46` | not-permanent | mixed (5/8) | extinction | 18.2 | 0.06 | 283 |  | reduction: consumers persisted on 3/3 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:48` | not-permanent | persist (0/8) | none | 13.9 | 0.11 | 358 |  | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:49` | not-permanent | mixed (2/8) | none | 92.4 | 0.39 | 330 |  | reduction: consumers persisted on 5/6 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:51` | not-permanent | mixed (1/8) | none | 31.4 | 0.00 | 3.25 |  | reduction: consumers persisted on 4/7 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:52` | not-permanent | mixed (1/8) | none | 9.8 | 0.00 | 22.4 |  | reduction: consumers persisted on 1/7 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:53` | not-permanent | mixed (1/8) | none | 28.3 | 0.01 | 25.4 |  | reduction: consumers persisted on 6/7 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:57` | not-permanent | persist (0/8) | none | 50.7 | 0.05 | 75.7 |  | reduction: consumers persisted on 1/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:62` | not-permanent | mixed (1/8) | none | 7.5 | 0.00 | 0.811 |  | reduction: consumers persisted on 4/7 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:63` | not-permanent | persist (0/8) | none | 53.5 | 0.00 | 3.25 |  | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:64` | not-permanent | mixed (3/8) | none | 67.1 | 0.09 | 74.3 |  | reduction: consumers persisted on 2/5 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:65` | not-permanent | persist (0/8) | none | 31.8 | 0.16 | 645 |  | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 3/8 persisting seeds |
| `sample:67` | not-permanent | persist (0/8) | none | 15.4 | 0.00 | 0.0321 |  | reduction: consumers persisted on 1/8 seeds despite I <= 1 and no decomposer guild - realised traits departed the centroid (evolution / draw) |
| `sample:68` | not-permanent | mixed (3/8) | none | 95.9 | 0.11 | 48.4 |  | reduction: consumers persisted on 5/5 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:69` | not-permanent | persist (0/8) | none | 11.7 | 0.76 | 7.18e+03 |  | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:70` | not-permanent | mixed (2/8) | none | 2.7 | 0.00 | 14.2 |  | reduction: consumers persisted on 3/6 seeds despite I <= 1 and no decomposer guild - realised traits departed the centroid (evolution / draw) |
| `sample:72` | not-permanent | mixed (3/8) | none | 91.1 | 0.14 | 91.4 |  | reduction: consumers persisted on 5/5 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:73` | not-permanent | mixed (1/8) | none | 30.5 | 0.09 | 225 |  | reduction: consumers persisted on 6/7 seeds despite I <= 1, with a decomposer guild on 6 of them - A2's carcass route, which A1's map lacks |
| `sample:74` | not-permanent | persist (0/8) | none | 13.0 | 0.07 | 1.19e+03 |  | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:76` | not-permanent | mixed (2/8) | none | 8.0 | 0.37 | 6.26e+03 |  | reduction: consumers persisted on 3/6 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:78` | not-permanent | persist (0/8) | none | 14.8 | 0.02 | 56.1 |  | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 6 of them - A2's carcass route, which A1's map lacks |
| `sample:80` | not-permanent | mixed (1/8) | none | 9.4 | 0.00 | 9.52 | yes | reduction: consumers persisted on 4/7 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:81` | not-permanent | persist (0/8) | none | 5.8 | 0.02 | 390 |  | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 2/8 persisting seeds |
| `sample:82` | not-permanent | mixed (3/8) | none | 13.8 | 0.31 | 2.23e+03 |  | reduction: consumers persisted on 2/5 seeds despite I <= 1 and no decomposer guild - realised traits departed the centroid (evolution / draw) |
| `sample:83` | not-permanent | mixed (3/8) | none | 16.2 | 0.30 | 2.62e+03 |  | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 1/5 persisting seeds |
| `sample:86` | not-permanent | persist (0/8) | none | 57.2 | 0.55 | 517 |  | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:87` | not-permanent | persist (0/8) | none | 23.4 | 0.62 | 3.98e+03 |  | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 7/8 persisting seeds |
| `sample:88` | not-permanent | mixed (1/8) | none | 45.3 | 0.46 | 589 |  | reduction: consumers persisted on 6/7 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:90` | not-permanent | persist (0/8) | none | 10.6 | 0.01 | 57.2 |  | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:91` | not-permanent | persist (0/8) | none | 85.2 | 0.10 | 61.2 |  | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:92` | not-permanent | persist (0/8) | none | 33.9 | 0.99 | 3.09e+03 |  | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 6/8 persisting seeds |
| `sample:93` | not-permanent | mixed (2/8) | none | 21.5 | 0.16 | 696 |  | reduction: consumers persisted on 4/6 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:94` | not-permanent | mixed (3/8) | none | 28.6 | 0.16 | 1.27e+03 |  | reduction: consumers persisted on 1/5 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:98` | not-permanent | mixed (2/8) | none | 38.5 | 0.00 | 0.156 |  | reduction: consumers persisted on 5/6 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:99` | not-permanent | mixed (5/8) | extinction | 39.9 | 0.03 | 85.4 |  | reduction: consumers persisted on 2/3 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:100` | not-permanent | mixed (2/8) | none | 11.2 | 0.03 | 162 |  | reduction: consumers persisted on 5/6 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:101` | not-permanent | mixed (3/8) | none | 31.3 | 0.03 | 67.4 |  | reduction: consumers persisted on 1/5 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:104` | not-permanent | mixed (4/8) | none | 30.0 | 0.00 | 0.221 |  | reduction: consumers persisted on 2/4 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:108` | not-permanent | mixed (2/8) | none | 18.5 | 0.01 | 28.4 |  | reduction: consumers persisted on 3/6 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:110` | not-permanent | persist (0/8) | none | 8.5 | 0.43 | 6.79e+03 |  | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 8 of them - A2's carcass route, which A1's map lacks |
| `sample:112` | not-permanent | persist (0/8) | none | 48.4 | 0.41 | 1.49e+03 |  | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:113` | not-permanent | persist (0/8) | none | 59.4 | 0.36 | 599 |  | reduction: consumers persisted on 3/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:117` | not-permanent | persist (0/8) | none | 1.9 | 0.08 | 3.74e+03 |  | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:118` | not-permanent | mixed (1/8) | none | 9.7 | 0.45 | 5.35e+03 |  | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 4/7 persisting seeds |
| `sample:119` | not-permanent | mixed (3/8) | none | 5.9 | 0.00 | 0.429 |  | reduction: consumers persisted on 2/5 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:121` | not-permanent | persist (0/8) | none | 11.6 | 0.00 | 37.1 |  | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:122` | not-permanent | mixed (2/8) | none | 26.5 | 0.22 | 458 |  | reduction: consumers persisted on 3/6 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:124` | not-permanent | persist (0/8) | none | 26.0 | 0.00 | 0.643 |  | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 8 of them - A2's carcass route, which A1's map lacks |
| `sample:125` | not-permanent | persist (0/8) | none | 24.6 | 0.01 | 27 |  | reduction: consumers persisted on 1/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:126` | not-permanent | persist (0/8) | none | 18.0 | 0.12 | 495 |  | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:127` | not-permanent | mixed (1/8) | none | 31.1 | 0.17 | 345 | yes | reduction: consumers persisted on 4/7 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:128` | not-permanent | mixed (3/8) | none | 29.1 | 0.78 | 1.42e+03 | yes | reduction: consumers persisted on 3/5 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:129` | not-permanent | mixed (4/8) | nutrient-lockup | 16.4 | 0.00 | 18.4 |  | reduction: consumers persisted on 4/4 seeds despite I <= 1 and no decomposer guild - realised traits departed the centroid (evolution / draw) |
| `sample:130` | not-permanent | persist (0/8) | monoculture | 44.4 | 0.26 | 605 |  | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:132` | not-permanent | persist (0/8) | none | 177.1 | 0.80 | 408 |  | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:133` | not-permanent | mixed (5/8) | extinction | 25.9 | 0.72 | 1.24e+03 |  | reduction: consumers persisted on 3/3 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:135` | not-permanent | mixed (5/8) | extinction | 8.5 | 0.01 | 95.3 |  | reduction: consumers persisted on 2/3 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:137` | not-permanent | persist (0/8) | none | 43.7 | 0.50 | 1.75e+03 |  | reduction: consumers persisted on 2/8 seeds despite I <= 1 and no decomposer guild - realised traits departed the centroid (evolution / draw) |
| `sample:138` | not-permanent | mixed (1/8) | none | 25.1 | 0.01 | 14.6 |  | reduction: consumers persisted on 3/7 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:140` | not-permanent | persist (0/8) | none | 47.3 | 0.27 | 641 |  | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:141` | not-permanent | persist (0/8) | none | 13.4 | 0.33 | 6.32e+03 |  | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:144` | not-permanent | persist (0/8) | none | 14.0 | 0.38 | 2.45e+03 |  | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 6 of them - A2's carcass route, which A1's map lacks |
| `sample:145` | not-permanent | mixed (2/8) | none | 4.6 | 0.86 | 1.53e+04 |  | reduction: consumers persisted on 6/6 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:146` | not-permanent | mixed (3/8) | none | 45.0 | 0.25 | 571 |  | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 2/5 persisting seeds |
| `sample:148` | not-permanent | persist (0/8) | none | 35.5 | 0.31 | 638 |  | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 7/8 persisting seeds |
| `sample:149` | not-permanent | mixed (3/8) | none | 53.1 | 0.01 | 27.8 | yes | reduction: consumers persisted on 1/5 seeds despite I <= 1 and no decomposer guild - realised traits departed the centroid (evolution / draw) |
| `sample:151` | not-permanent | persist (0/8) | none | 26.6 | 0.00 | 0.00511 |  | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:153` | not-permanent | mixed (5/8) | extinction | 40.2 | 0.35 | 1.77e+03 |  | reduction: consumers persisted on 2/3 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:155` | not-permanent | mixed (1/8) | none | 33.7 | 0.04 | 61.7 | yes | reduction: consumers persisted on 3/7 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:157` | not-permanent | persist (0/8) | none | 4.3 | 0.01 | 205 |  | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:161` | not-permanent | persist (0/8) | none | 17.9 | 0.02 | 103 |  | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 7 of them - A2's carcass route, which A1's map lacks |
| `sample:162` | not-permanent | persist (0/8) | none | 11.6 | 0.12 | 611 |  | reduction: consumers persisted on 2/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:163` | not-permanent | persist (0/8) | none | 73.2 | 0.55 | 876 |  | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:165` | not-permanent | mixed (2/8) | none | 66.4 | 0.20 | 212 |  | reduction: consumers persisted on 5/6 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:166` | not-permanent | persist (0/8) | none | 70.4 | 0.22 | 1.13e+03 |  | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:167` | not-permanent | persist (0/8) | none | 85.5 | 0.03 | 38.1 |  | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:168` | not-permanent | persist (0/8) | none | 4.5 | 0.05 | 666 |  | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:169` | not-permanent | mixed (3/8) | none | 32.6 | 0.00 | 6.92 | yes | reduction: consumers persisted on 2/5 seeds despite I <= 1 and no decomposer guild - realised traits departed the centroid (evolution / draw) |
| `sample:170` | not-permanent | mixed (7/8) | extinction | 37.5 | 0.36 | 1.45e+03 |  | reduction: consumers persisted on 1/1 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:171` | not-permanent | mixed (4/8) | none | 21.2 | 0.03 | 534 |  | reduction: consumers persisted on 1/4 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:173` | not-permanent | mixed (2/8) | none | 10.3 | 0.01 | 80 |  | reduction: consumers persisted on 4/6 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:174` | not-permanent | mixed (2/8) | none | 46.0 | 0.36 | 601 |  | reduction: consumers persisted on 6/6 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:175` | not-permanent | persist (0/8) | none | 68.8 | 0.89 | 553 |  | reduction: consumers persisted on 8/8 seeds despite I <= 1, with a decomposer guild on 7 of them - A2's carcass route, which A1's map lacks |
| `sample:177` | not-permanent | mixed (2/8) | none | 34.4 | 0.11 | 303 |  | reduction: consumers persisted on 3/6 seeds despite I <= 1 and no decomposer guild - realised traits departed the centroid (evolution / draw) |
| `sample:178` | not-permanent | mixed (2/8) | none | 45.7 | 0.01 | 30.2 |  | reduction: consumers persisted on 3/6 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:179` | not-permanent | persist (0/8) | none | 14.0 | 0.45 | 2.87e+03 |  | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 2/8 persisting seeds |
| `sample:181` | not-permanent | persist (0/8) | none | 28.9 | 0.35 | 1.54e+03 |  | reduction: consumers persisted on 5/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:182` | not-permanent | persist (0/8) | none | 44.9 | 0.03 | 65.9 |  | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:183` | not-permanent | persist (0/8) | none | 24.9 | 0.67 | 1.52e+03 |  | reduction: consumers persisted on 2/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:185` | not-permanent | persist (0/8) | none | 9.5 | 0.10 | 1.11e+03 |  | reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on 7/8 persisting seeds |
| `sample:186` | not-permanent | mixed (1/8) | none | 47.8 | 0.21 | 583 |  | reduction: consumers persisted on 5/7 seeds despite I <= 1, with a decomposer guild on 3 of them - A2's carcass route, which A1's map lacks |
| `sample:187` | not-permanent | mixed (1/8) | none | 47.0 | 0.16 | 181 |  | reduction: consumers persisted on 3/7 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:188` | not-permanent | persist (0/8) | none | 16.6 | 0.28 | 3.94e+03 |  | reduction: consumers persisted on 7/8 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:189` | not-permanent | persist (0/8) | none | 9.0 | 0.41 | 6.6e+03 |  | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:191` | not-permanent | persist (0/8) | none | 8.4 | 0.00 | 3.18 |  | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |
| `sample:193` | not-permanent | mixed (1/8) | none | 16.5 | 0.41 | 1.51e+03 |  | reduction: consumers persisted on 2/7 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:196` | not-permanent | mixed (1/8) | none | 11.7 | 0.00 | 0.807 |  | reduction: consumers persisted on 2/7 seeds despite I <= 1, with a decomposer guild on 2 of them - A2's carcass route, which A1's map lacks |
| `sample:197` | not-permanent | mixed (4/8) | none | 45.0 | 0.18 | 1.29e+03 |  | reduction: consumers persisted on 1/4 seeds despite I <= 1, with a decomposer guild on 1 of them - A2's carcass route, which A1's map lacks |
| `sample:198` | not-permanent | persist (0/8) | none | 7.0 | 0.06 | 1.56e+03 |  | reduction: consumers persisted on 4/8 seeds despite I <= 1, with a decomposer guild on 4 of them - A2's carcass route, which A1's map lacks |
| `sample:199` | not-permanent | persist (0/8) | none | 19.4 | 0.08 | 199 |  | reduction: consumers persisted on 6/8 seeds despite I <= 1, with a decomposer guild on 5 of them - A2's carcass route, which A1's map lacks |

### A2 — false negatives

| cell | predicted | observed | modal mode | ρ | I | Λ | #444 | fault hypothesis |
|---|---|---|---|---|---|---|---|---|
| `atlas:8` | not-permanent | persist (0/8) | none | 26.5 | 0.00 | 0 |  | reduction: kappa_P = 0 zeroes r_P = kappa_P*gamma*(F - B_P) although rho = 26.5 > 1 - the reproductive share still becomes offspring biomass (the map's kappa lumping, not the extinction gate) |
| `atlas:11` | not-permanent | persist (0/8) | none | 18.4 | 0.00 | 0 |  | reduction: kappa_P = 0 zeroes r_P = kappa_P*gamma*(F - B_P) although rho = 18.4 > 1 - the reproductive share still becomes offspring biomass (the map's kappa lumping, not the extinction gate) |
| `atlas:12` | not-permanent | persist (0/8) | none | 29.1 | 0.00 | 0 |  | reduction: kappa_P = 0 zeroes r_P = kappa_P*gamma*(F - B_P) although rho = 29.1 > 1 - the reproductive share still becomes offspring biomass (the map's kappa lumping, not the extinction gate) |
| `atlas:15` | not-permanent | persist (0/8) | monoculture | 163.8 | 0.00 | 0 |  | reduction: kappa_P = 0 zeroes r_P = kappa_P*gamma*(F - B_P) although rho = 163.8 > 1 - the reproductive share still becomes offspring biomass (the map's kappa lumping, not the extinction gate) |
| `atlas:17` | not-permanent | persist (0/8) | none | 66.5 | 0.00 | 0 |  | reduction: kappa_P = 0 zeroes r_P = kappa_P*gamma*(F - B_P) although rho = 66.5 > 1 - the reproductive share still becomes offspring biomass (the map's kappa lumping, not the extinction gate) |
| `atlas:28` | not-permanent | mixed (2/8) | none | 474.0 | 0.00 | 0 |  | reduction: kappa_P = 0 zeroes r_P = kappa_P*gamma*(F - B_P) although rho = 474.0 > 1 - the reproductive share still becomes offspring biomass (the map's kappa lumping, not the extinction gate) |
| `atlas:37` | not-permanent | mixed (1/8) | none | 26.8 | 0.00 | 0 |  | reduction: kappa_P = 0 zeroes r_P = kappa_P*gamma*(F - B_P) although rho = 26.8 > 1 - the reproductive share still becomes offspring biomass (the map's kappa lumping, not the extinction gate) |
| `atlas:40` | not-permanent | mixed (1/8) | none | 17.3 | 0.00 | 0 |  | reduction: kappa_P = 0 zeroes r_P = kappa_P*gamma*(F - B_P) although rho = 17.3 > 1 - the reproductive share still becomes offspring biomass (the map's kappa lumping, not the extinction gate) |
| `atlas:48` | not-permanent | persist (0/8) | none | 34.7 | 0.00 | 0 |  | reduction: kappa_P = 0 zeroes r_P = kappa_P*gamma*(F - B_P) although rho = 34.7 > 1 - the reproductive share still becomes offspring biomass (the map's kappa lumping, not the extinction gate) |
| `atlas:54` | not-permanent | persist (0/8) | none | 35.8 | 0.00 | 0 |  | reduction: kappa_P = 0 zeroes r_P = kappa_P*gamma*(F - B_P) although rho = 35.8 > 1 - the reproductive share still becomes offspring biomass (the map's kappa lumping, not the extinction gate) |
