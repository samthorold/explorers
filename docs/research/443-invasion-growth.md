# Issue #443 — formal viability D1: the invasion-growth-rate instrument (mutual invasibility as the coexistence oracle)

**Status: research finding. Commits the instrument and one observability addition to the
event log; changes no trajectory.** This note asks Chesson's question of every atlas live
cell: can each trophic role invade the resident web when rare? It builds the instrument
that measures the answer — a lineage-tracked per-capita growth rate of an injected
cohort over a 500-tick window, eight seeds per cell, three roles, two injection
protocols — turns it into a per-cell verdict, puts that verdict next to the atlas's own
`coexistence_fraction`, and tabulates the observed invasion sign against the sign the
A1/A2 reduction ([`432-permanence-pc.md`](432-permanence-pc.md),
[`437-permanence-alc.md`](437-permanence-alc.md)) predicts at the same trait vectors. It
introduces no mechanism, functional form, or parameter; it touches nothing in the
stepper's dynamics, the evaluator, or the search. Nothing in `docs/system-design/` is
edited.

The instrument is `crates/explorers-search/src/bin/invasion_growth.rs`, patterned on
`permanence_crosscheck.rs` / `energy_bound_check.rs` (config sourcing, seed block,
artifact, env selectors). Its artifact is `target/invasion-growth.json` (gitignored).
The atlas is the one #474 regenerated on the fixed stepper (PR #483, search run at commit
`5a7bede`): 82 live cells.

## TL;DR

1. **Coverage.** 82 atlas live cells × 3 roles × 8 fixed seeds × 2 injection arms, plus a
   control per (cell, seed): 656 resident runs to `t_inj = 500` (646 reached it; 10 seeds
   across seven low-fitness cells went extinct first), 3 750 forked windows of up to 500
   ticks (646 control, 1 938 `intact`, 1 166 `removed`; the `removed` arm reuses the
   `intact` run when the role is absent). ~19 min wall-clock in release; two full runs give a byte-identical artifact.
2. **The headline: by the invasion criterion, no multi-role atlas cell coexists.** 54 of the
   82 cells carry at least two roles in the resident web at `t_inj` (P+D 33, P+C+D 18,
   P+C 3); 49 of those are "coexisting" by the atlas's `coexistence_fraction ≥ 0.5`
   (27 of them at `1.0`). **0 / 54 pass the criterion — on either arm, strict or relaxed.**
   Every cell that does pass (8 strict / 15 by median on the `removed` arm; 0 / 3 on the
   `intact` arm) is a **producer-only web**, where the criterion asks only whether the
   producer can re-invade its own emptied world. So of the 70 atlas cells the projection
   would call coexisting, **0 are coexisting by mutual invasibility** once the trivial
   producer-only passes are set aside (8 with them).
3. **The heterotroph roles never invade when rare.** Consumers are present in 21 cells and
   invade in 0 (by median, either arm); decomposers are present in 51 and invade in 1 (by
   median, `atlas:9`, 4/8 seeds — not strict). It is not instant death: the injected
   heterotroph cohorts reproduce in 122/193 (consumer) and 258/338 (decomposer)
   role-present runs, but their lineages shrink — births do not cover deaths — and the
   *resident* heterotrophs are doing the same thing: over the control window the resident
   consumers shrink or vanish on 92 + 55 of the 193 seeds that had them and grow on 12;
   decomposers shrink or vanish on 100 + 37 of 338 and grow on 50. **At the 500-tick
   search horizon the atlas's heterotroph guilds are transients, not attractors** — the
   "coexistence" `coexistence_fraction` reads (no failure mode, separated trait clusters
   or a positive coexistence duration) is a snapshot of a web whose heterotrophs are on
   their way out.
4. **The producer's invasibility depends on the protocol, and the difference is itself a
   finding.** On the `intact` arm (inject into the standing web, the issue's literal
   protocol) the producer invades in 25/82 cells by median and 2 strict; on the `removed`
   arm (cull the producers first, Chesson's "community without the species") 59 by median
   and 25 strict. Injected into a standing crop of its own phenotype the founder cohort
   dies out in 132 of 646 runs (median extinction tick 80) — an established producer web
   excludes its own kind (light competition against grown residents; the newcomer's
   founder-scale body), a priority effect the mean-field map has no coordinate for.
5. **Against the reduction.** Producer: `λ_P(𝓥) > 1` predicted on every cell; observed
   positive on 59/82 (`removed`) — the 23 disagreements are the cells whose resident is a
   handful of agents (median resident population 6; 22 of 23 below 30) where an 8-agent
   cohort is not rare and finite-size death dominates. Consumer: `λ_C(K_P)` agrees on
   13/21, disagrees in the predicted-positive direction on 8, never in the other. Decomposer:
   `λ_H(𝓛) > 1` predicted on 47 + 3 split of 51 and observed on 1 — the A2 reference
   lumping `ι = 1` over-serves the pile by one to four orders (`Λ` from 6 to 21 000),
   exactly the fault #439 already named for its two strict false positives. No row is
   attributable to #482 (the `κ_C ≈ 0` tag never decides a disagreement here, because the
   reduction over-predicts rather than under-predicts the heterotrophs).
6. **Rarity is not achieved on a third of the cells.** Median resident population at
   `t_inj` is 22 agents; on 31 cells the 8-agent cohort is at least half the resident. The
   instrument reports the ratio; the criterion is read as stated but those cells are
   finite-size, not invasion, experiments.
7. **Re-read (§4, 2026-09-13): the resident heterotrophs are single sessile
   mixotrophic individuals, a quarter of them sterile at the `kappa` clamp — not guilds.**
   The horizon question of §3 is closed (2000-tick evidence grows no guild in the atlas);
   the rarity question is moot (`N = 1`); the 761 canonical-vertex injections are a
   protocol artefact (sessile, non-photosynthetic vertex starves by tick 13). Heterotroph
   populations of ~20 do exist in ~2 % of unselected configs and the search selected
   against them. What follows is a guild observable in the evaluator and two protocol
   fixes to this bin, not a longer window.

## 1. Method

### 1.1 Resident, injection, arms

Per (cell, seed): `World::new` on the cell's decoded `unit` (via `decode` over
`default_ranges`), stepped to `t_inj = SearchConfig::default().max_ticks = 500` with the
genesis loop's early stops (empty roster; population above `EvalConfig::max_population`).
At `t_inj` a `TopologyProjection` over the whole log classifies the roster with
`topology::trophic_roles` (producer by trait; consumer / decomposer by realised diet) and
each role's **realised centroid** — the mean trait vector of the agents in that role — is
read. A role absent from the resident gets the config's **canonical pure vertex**: the
cluster centroid `World::new` seeds for that compartment, pure producer `(α+h, 0, …)` or
pure heterotroph `(0, α+h, …)` around the config mean (the decomposer is a heterotroph by
trait, its role realised by diet, so it shares the heterotroph vertex); the record says
which was used (`centroid_source`). Over the 646 seeds: producer realised 635 /
canonical 11, consumer 193 / 453, decomposer 338 / 308.

The resident world is then **forked** — `World` is now `Clone`, and a clone steps
identically to its original (every draw is keyed on world state, #376; the sim test
`a_cloned_world_steps_identically_to_its_original` pins it) — into:

- a **control**: the resident stepped through the window untouched;
- per role × arm, an **injection**: a cohort of `INVADER_COHORT = 8` agents at the
  centroid, positions uniform over the world from a stream keyed on (seed, role),
  provisioned exactly as founders are (`initial_energy_per_agent` split by
  `provision_initial_reserve_structure`; the birth structure's bound nutrient drawn from
  the pool under the agent, free store empty).

Two arms. **`intact`** injects into the resident as it stands — the issue's literal
protocol; the injected role's own residents stay. **`removed`** first removes the injected
role's residents (`World::retain_agents`, a new state-edit API alongside `add_agent`),
returning each body's `nutrient_total` to the pool under it so the residual web keeps the
cell's nutrient budget (`N_total` is an A2 coefficient); this is Chesson's "the community
without the species". When the role is absent the two arms coincide and the `removed` row
reuses the `intact` one.

### 1.2 Lineage tracking (the tracking method)

`Agent` has no parent field and the `Reproduced` event names the parent(s) but not the
offspring, whose ids are handed out after a world-state sort, so a marker id range would
identify only the first generation. The observability addition is a per-offspring
**`Born` event** — `source` = the offspring's final world id, `target` = the parent (the
seed parent of a pair), a new `Event::second_parent` = the mate for sexual births —
emitted by `World::step` when it assigns ids, from a `parents` vector
`resolve_reproduction` now returns alongside `offspring`. It is a raw descent fact like
the existing `target_was_carcass`; it changes no trajectory (golden digests in
`trait_unit_anchors.rs`, `soa_differential`, `headless_*` all unchanged; the SoA twin has
no reproduction path to mirror). Two sim tests pin the event's contract (asexual and
sexual births).

The bin's `Lineage` starts as the cohort's id set and absorbs the log tail after each tick:
a `Born` whose `target` **or** `second_parent` is a member adds its `source`. Membership is
permanent; the live lineage at tick `t` is the members on the roster (so every death route
counts, not only the ones that log `Died`). Sexual crosses with a resident mate are members
by this either-parent rule and are counted separately (`cross_births`; 9 318 of 81 621
lineage births on the `intact` arm) so the rule's inflation is visible. The per-capita
rate is

```
r = ln( max(N_end, ½) / N_0 ) / ticks_run
```

over the 500-tick window (`ticks_run < 500` only when the world empties or explodes; an
extinct lineage reads at half an agent so its rate is a finite negative number, and only
the sign enters the criterion). The lineage size every 25 ticks and the extinction tick
are in the record.

### 1.3 Criterion and the interval

Per (cell, role, arm) over the seeds that reached `t_inj`: the role **invades** when the
median of the 8 rates is > 0 **and** the interval on the median does not straddle zero.
The interval is the distribution-free order-statistic interval `[x_(k), x_(n+1−k)]` — the
sign test, i.e. #434's binomial computation applied to the sign of each seed's rate. At
`n = 8` the narrowest ≥ 95 % interval is `[x_(1), x_(8)]` (coverage 99.2 %; `[x_(2), x_(7)]`
is only 93.0 %), so "does not straddle zero" means **every seed agrees with the median's
sign**; below `n = 6` no interval exists and the strict verdict cannot fire. #434's exact
Clopper–Pearson lower bound on the positive-seed fraction is recorded alongside (pinned
to its table: `8/8 → 0.631`, `7/8 → 0.473`). The relaxed read — median > 0 only — is
reported next to the strict one.

A role is **present** in a cell when it is present at `t_inj` on at least half the seeds
that reached `t_inj`. A cell is **coexisting-by-criterion** on an arm when every present
role invades. A cell with one role present passes as soon as that role can invade its own
emptied world; the tables therefore carry the multi-role subset separately.

### 1.4 Predicted signs

At each seed's centroids the A1/A2 boundary eigenvalues give a predicted sign per role,
coefficients copied from `permanence_crosscheck.rs` (with #466's `r_P = χ_P·γ·(F − B_P)`)
and pinned to its example10 numbers by unit test:

| role | eigenvalue | sign of | note |
|---|---|---|---|
| producer | `λ_P(𝓥) = 1 + min(r_P, α_P/θ_P) − μ_P` (A2 i) | `min(r_P, α_P/θ_P) − 0.02` | the virgin pool; `λ_P(𝓛) = 1 − μ_P < 1` always, so the lockup corner is not a producer-invasion question |
| consumer | `λ_C(K_P) = 1 + β·K_P − m` (A1 clause 2) | `I − 1` | the standing crop |
| decomposer | `λ_H(𝓛) = 1 + σ·N_total·min(κ_C·γ·e_C, q/θ_C) − m` (A2 ii) | `Λ − 1` | the carcass pile, at A2's reference lumping `ι = 1`, `q = ν = θ_P` |

**The `χ_C` correction (#482), landed after this run.** When these numbers were taken
the consumer's and decomposer's `β` / `Λ` still carried the `κ_C` lumping, so a
heterotroph centroid with `κ < 0.05` was tagged `affected_by_482` and a
predicted-negative / observed-positive row on such a centroid was classed `affected-482`
rather than as a disagreement. The bin now reads `β = χ_C·γ·e·a` and
`Λ ∝ min(χ_C·γ·e_C, q/θ_C)` with the consumer's biomass conversion `χ_C` (derived in
[`432-permanence-pc.md`](432-permanence-pc.md); example10 pins `I = 25.77`, `Λ = 90 584`),
the tag and the `affected-482` agreement class are removed (the sign tallies are five-way:
agree / pred+ obs− / pred− obs+ / split / absent), and the tables below — pre-#482 — are
to be re-taken as a separate follow-up. Since #482 never decided a row here (§2.5), the
headline is not expected to move. Per (cell, role, arm) the predicted sign is the majority
over seeds (`predicted-split` when tied) and the observed sign is the median's.

### 1.5 Determinism and provenance

Seeds `1000..1008` per cell (the sibling bins' block); cohort positions keyed on
(seed, role); one rayon task per cell with an order-stable per-seed collect. Two
consecutive full runs of the same binary both give SHA-1
`1fb77ca233112e716e4e23da0fff071e1d323da6`. A third run after a pure refactor of the bin
(a return struct; the multi-role summary fields) reproduces every cell record
byte-for-byte (the `cells` array compared as JSON); its artifact, the one the tables
here are generated from, is SHA-1 `5f2a9ede3463ddacc83fae5548b5d6294863013d`.
Wall-clock ≈ 19 min per full run on the development machine (release, rayon), peak
RSS ≈ 0.6 GB on a two-cell subset.

## 2. Results

### 2.1 The criterion against `coexistence_fraction`

`n = 82` cells; "atlas coexisting" is `coexistence_fraction ≥ COEXISTENCE_FLOOR = 0.5`
(70 cells). Strict = every present role: median > 0 and interval clear of zero; median =
every present role: median > 0.

| arm | all cells: strict / median | atlas-coexisting (70): strict / median | not atlas-coexisting (12): strict / median | multi-role (54): strict / median | atlas-coexisting ∧ multi-role (49): strict / median |
|---|---|---|---|---|---|
| `intact` | 0 / 3 | 0 / 3 | 0 / 0 | **0 / 0** | **0 / 0** |
| `removed` | 8 / 15 | 8 / 15 | 0 / 0 | **0 / 0** | **0 / 0** |

By `coexistence_fraction` band (`removed` arm; `intact` in brackets):

| band | cells | strict | by median |
|---|---|---|---|
| `== 1.0` | 36 | 3 (0) | 6 (2) |
| `≥ 0.8` | 58 | 6 (0) | 13 (3) |
| `≥ 0.5` | 70 | 8 (0) | 15 (3) |
| `< 0.5` | 12 | 0 (0) | 0 (0) |

Every pass is a producer-only web: `atlas:2, 3, 10, 14, 26, 46, 59, 68` strict on
`removed`; `atlas:10, 63, 75` by median on `intact`. The role sets present at `t_inj`
(majority of seeds): P only 28 cells, P+D 33, P+C+D 18, P+C 3.

### 2.2 Per role

| role | present in | `intact`: strict / median | `removed`: strict / median |
|---|---|---|---|
| producer | 82 | 2 / 25 | 25 / 59 |
| consumer | 21 | 0 / 1 | 0 / 0 |
| decomposer | 51 | 0 / 1 | 0 / 1 |

What the heterotroph cohorts do (role-present runs, i.e. seeds where the role was in the
resident): consumer 193 runs — 122 reproduce, 15 go extinct within the window (median
tick 49), 35 end with `r > 0`; decomposer 338 runs — 258 reproduce, 69 extinct (median
tick 38), 46 with `r > 0`. Their centroids are not the degenerate ones: median `κ` 0.63
(consumer) / 0.52 (decomposer), median heterotrophy 0.95 / 1.06, median fecundity
0.42 / 0.35 — though 57 of the 193 consumer centroids and 51 of the 338 decomposer
centroids sit at `κ ≥ 0.95`, a heterotroph that funds nothing to reproduction.

### 2.3 The resident's response (control drift and displacement)

The control tells the resident's own drift over the window. Per-capita rate of the
resident role count from `t_inj` to `t_inj + 500`, over the seeds that had the role at
`t_inj`:

| role | seeds with role | grew | shrank | unchanged | lost entirely | seeds with positive control rate |
|---|---|---|---|---|---|---|
| producers | 635 | 174 | 376 | 85 | 5 | 174 |
| consumers | 193 | 12 | 92 | 89 | 55 | 12 |
| decomposers | 338 | 50 | 100 | 188 | 37 | 50 |

Consumers are lost outright on 55 of 193 seeds and grow on 12; decomposers lost on 37 of
338 and grow on 50. This is the same fact the invader lineages report from the other side:
the heterotroph guilds present at the search horizon are declining transients on most
seeds.

Displacement — median over seeds of (resident role count at the end of the injected
window − the control's), P / C / D, and the seeds on which some resident role ended below
the control:

| arm | injected | median Δ P / C / D | seeds with any resident loss |
|---|---|---|---|
| `intact` | producer | −1 / 0 / 0 | 413 / 646 |
| `intact` | consumer | 0 / 0 / 0 | 307 / 646 |
| `intact` | decomposer | 0 / 0 / 0 | 335 / 646 |
| `removed` | producer | −18 / 0 / 0 | 638 / 646 |
| `removed` | consumer | 0 / 0 / 0 | 341 / 646 |
| `removed` | decomposer | 0 / 0 / 0 | 438 / 646 |

(On the `removed` arm the "resident" producers are the culled ones, so the −18 is the cull
itself, not a displacement.) Injected heterotrophs displace nothing at the median; an
injected producer cohort into an intact web costs the resident one producer at the median
— the newcomers and the residents compete for the same light and then mostly the
newcomers lose (§2.4).

### 2.4 The intact-vs-removed gap on the producer

On `intact`, the producer cohort at the resident's own centroid goes extinct in 132 of 646
runs (median extinction tick 80, quartiles 34–170) and reaches `r > 0` on 25 cells; on
`removed` it goes extinct in 10 and reaches `r > 0` on 59. Whether an intact web admits its
own phenotype tracks the resident's size: by resident-population tercile (`≤ 13`,
`14–32`, `≥ 33` agents) the producer's `intact` median is positive on 1, 9 and 16 of the
27–28 cells. A small, saturated producer web — founders' light competition radius
covering the world — has no room for a founder-scale newcomer; a large one does. Note the
`removed` arm hands the invader the culled residents' nutrient back in the pool, i.e. it
measures invasion of the *virgin* end (A2's `𝓥`), which is exactly the eigenvalue it is
compared with in §2.5.

### 2.5 Predicted vs observed sign (A1/A2 at the injected centroids)

Tallies over cells: agree / predicted + observed − / predicted − observed + / #482 /
split / absent.

| role | eigenvalue | `intact` | `removed` |
|---|---|---|---|
| producer | `λ_P(𝓥)` | 25 / 57 / 0 / 0 / 0 / 0 | 59 / 23 / 0 / 0 / 0 / 0 |
| consumer | `λ_C(K_P)` | 12 / 8 / 1 / 0 / 0 / 61 | 13 / 8 / 0 / 0 / 0 / 61 |
| decomposer | `λ_H(𝓛)` | 1 / 47 / 0 / 0 / 3 / 31 | 1 / 47 / 0 / 0 / 3 / 31 |

Reading the `removed` column (the arm that measures the boundary the eigenvalues are
about):

- **Producer, 59/82 agree.** `λ_P(𝓥) > 1` on every cell (`r_P` from 0.03 to 4.95 at the
  injected centroids, `μ_P = 0.02`). The 23 predicted-positive / observed-negative cells
  have median resident populations `[1, 2, 2, 2, 2, 3, 3, 3, 4, 6, 6, 6, 7, 8, 8, 9, 9, 10,
  11, 13, 17, 26, 186]` against a median of 30 on the agreeing cells: a world that holds a
  handful of producers at `t_inj` does not hold eight more — finite size, not the
  reduction's sign.
- **Consumer, 13/21 agree, 8 predicted-positive / observed-negative, none the other way.**
  Where `I < 1` the consumer cohort indeed shrinks (13 cells, `I` from 0.00 to 0.29). Where
  `I > 1` (`atlas:9, 13, 29, 51, 55, 57, 62, 76`; `I` from 1.5 to 29) the cohort
  still shrinks. These are A1's authority boundary made visible from the invasion side:
  clause (2) is necessary, and here it is not sufficient — the standing crop is there but
  the realised encounter (reach, spatial mixing) does not deliver `β·K_P` to a rare
  consumer. The one `intact` predicted-negative / observed-positive row (`atlas:78`, `I ≈
  0.2`, 4/8 seeds positive, median +0.0001) is a coin-flip median, not a contradiction.
- **Decomposer, 1/51 agree.** `Λ` at the reference lumping is 6 to 21 000 on the 47
  predicted-positive cells and the decomposer cohort shrinks on all of them; the one
  agreement, `atlas:9` (`Λ ≈ 2 500`), is a 4/8-seed median of +0.0008. The
  reference `ι = 1` (every carcass within reach) over-serves the pile by orders of
  magnitude — the fault #439 named for `sample:154` / `sample:192` — and the invasion
  instrument says it is not two cells' fault but the lumping's on every cell. `λ_H(𝓛)` at
  `ι = 1` is not a usable predictor of decomposer invasion; a realised `ι` (carcasses
  within contact range per decomposer) would be the next step.
- **#482** decides no row: the `κ_C ≈ 0` tag fires on a majority of seeds only on
  `atlas:15, 56, 74` (consumer, 5/8 each — agreements, predicted and observed negative),
  `atlas:49` (decomposer, 6/8 — predicted positive regardless) and `atlas:40` (5/8, split);
  elsewhere on 1–4 seeds. The lumping matters where it would *under*-predict; here
  the reduction over-predicts the heterotrophs.

## 3. Characterisation

**What the instrument establishes.** A deterministic, lineage-tracked mutual-invasibility
read is now available for any cell or config, at ~15 s per (cell, 8 seeds, 2 arms) in
release. The tracking is by descent off the log, not by role headcount, so an injected
cohort that fails while the resident role persists is told apart from one that succeeds
by replacing it.

**What it says about the atlas.** `coexistence_fraction` and mutual invasibility disagree
on every multi-role cell. `coexistence_fraction` reads "no failure mode and separated
trait clusters or a positive coexistence duration" at tick 500; the invasion criterion
reads "each present role grows back from rare". The 49 atlas-coexisting multi-role cells
have heterotroph guilds that are declining in the control and that cannot re-establish
from a cohort, on either protocol. Under the issue's intended use — the criterion as
*primary evidence for coexistence, replacing hand-curated scenarios* — the atlas currently
offers no cell that qualifies. The evidence points at the horizon and at the heterotroph
economy rather than at the instrument: the resident heterotrophs are transients at
`t = 500`, and both the resident and the invader heterotrophs reproduce but not fast enough.
Whether a longer resident phase (`role_emergence` runs to 2000) finds a quasi-stationary
web with a self-sustaining heterotroph guild is the obvious next measurement; the bin's
`t_inj` and `window` are a two-constant change.

**Which protocol to read.** The `removed` arm is Chesson's; the `intact` arm is the
issue's. They agree on the headline (0/54 multi-role) and differ on the producer, where
the difference is a priority effect worth knowing about (§2.4). Future runs should report
both; a criterion built on `intact` alone would reject producer-only webs that are
perfectly viable.

**Rarity.** With median resident populations of 22 agents, an 8-agent cohort is not rare on
a third of the cells. The criterion is still read as stated, but those cells' rates are
finite-size draws. A cohort of 5 would be rarer and noisier; the record carries the
resident population so the reader can condition on the ratio.

**No wiring.** As in #439, nothing here gates the search or the evaluator. The criterion is
an instrument reading, and it currently says the atlas's "coexistence" descriptor and
mutual invasibility measure different things.

## 4. Re-read (2026-09-13): the resident heterotrophs are individuals, not guilds

**Status: addendum. No new runs; every number below is read off the artifact this note
is generated from (SHA-1 `5f2a9ede…`) or off `target/role-emergence.json` (#421, run
2026-06-15 on the pre-#444 stepper and the pre-#474 atlas — indicative, not
authoritative).** §3 posed the open question as *horizon*: is 500 ticks too short for the
heterotroph guild to be quasi-stationary, and is "rare" achievable when the median
resident population is 22? This section answers it from the data already in hand. The
answer is that the question was mis-posed: there is no heterotroph guild in the atlas for
a horizon to stabilise or a cohort to be rare against.

### 4.1 The resident heterotroph "guild" is one agent

Per seed that reached `t_inj` with the role present, the resident count at `t = 500`:

| role | seeds with role | count = 1 | = 2 | = 3–4 | ≥ 5 |
|---|---|---|---|---|---|
| consumers | 193 | 119 | 41 | 20 | 13 |
| decomposers | 338 | 157 | 81 | 75 | 25 |

Per *cell* (median over seeds, among the 21 / 51 role-present cells of §1.3): no
consumer-present cell has a median guild of 3 or more; 4 of the 51 decomposer-present cells
do (`[5,19,0]` 4.0, `[5,19,7]` 3.5, `[8,0,0]` 3.0, `[6,13,1]` 3.0). Seven consumer cells
and fifteen decomposer cells are "present" with a *median of zero* — present on exactly half
the seeds, at one agent.

What the single agent does over the control window: consumers — still 1 on 70 seeds, lost
on 45, grew on 4; decomposers — still 1 on 110, lost on 31, grew on 16. §2.3's "declining
transients" is therefore mostly this: one long-lived individual that does not reproduce
and sometimes dies. It is not a population in decline; there is no population.

### 4.2 What the resident heterotrophs are

`trophic_roles` reads producer vs heterotroph by trait (`photosynthetic_absorption ≥
heterotrophy` → producer) and, among heterotrophs, consumer vs decomposer by realised
diet, with a non-eater defaulting to consumer. The realised heterotroph centroids the
instrument injected at (§1.1) are heterotrophy-dominant but far from pure:

| role | n | `photosynthetic_absorption` median (q1–q3) | `< 0.1` | `mobility` median | `kappa ≥ 0.99` |
|---|---|---|---|---|---|
| producer | 635 | 1.11 (0.87–1.41) | 0 | — | 13 |
| consumer | 193 | 0.43 (0.22–0.64) | 20 | 0.00 | 52 |
| decomposer | 338 | 0.46 (0.28–0.73) | 11 | 0.07 | 45 |

They photosynthesise at roughly 40 % of the producer's investment, are sessile, and a
quarter of them sit at the `kappa` clamp. `kappa = 1` routes the whole mobilised flow to
soma and none to the reproductive earmark (`repro_nutrient += amount · (1 − kappa)`,
`explorers-sim/src/lib.rs`), so a `kappa = 1` agent is sterile by construction. The
picture consistent with the artifact — not directly confirmed, since it does not carry
per-agent consumption — is a sessile mixotroph that lives on its own photosynthesis, eats
whatever dies beside it (or nothing), and never reproduces. That is an individual with a
role tag, not a trophic level.

### 4.3 The injected cohort does the same thing

Splitting the `intact`-arm heterotroph injections by centroid source:

| role, source | n | extinct | flat (constant over the last 250 ticks) | grew | declining / fluctuating |
|---|---|---|---|---|---|
| consumer, canonical | 453 | 452 | 0 | 0 | 1 |
| consumer, realised | 193 | 15 | 69 | 35 | 74 |
| decomposer, canonical | 308 | 307 | 1 | 0 | 0 |
| decomposer, realised | 338 | 69 | 103 | 46 | 120 |

On realised centroids the modal outcome is **flat**: alive at 500, constant count, median
births 0 (consumer) / 3 (decomposer). Conditioning on `kappa`: the `kappa ≥ 0.99`
consumer cohorts are flat 38 / 52 (decomposer 28 / 45), exactly the sterile-founder
signature. The `grew` cohorts sit at `kappa` 0.44 / 0.36 and are the only ones with
births in the tens.

The injected lineage's rate and the resident guild's control rate on the same seed do not
track (Pearson `r = 0.09` consumer, `0.11` decomposer; sign agreement 79 / 193 and
104 / 338), and the invader's rate does not move with the resident's count (invader
`r > 0` on 9 / 10 / 16 of 64 consumer seeds by resident-count tercile, 10 / 9 / 27 of
~113 decomposer seeds). There is no density signal in either direction, which is what one
expects when the "resident population" is one agent.

### 4.4 The canonical-vertex injections measure nothing

The canonical pure heterotroph vertex is `(0, α+h, 0, …)` — no photosynthesis and, as the
sessile vertex, **mobility 0**. Of the 761 cohorts injected at it (the role-absent seeds),
759 die with zero births at a median tick of 13 (q1–q3 8–20). A sessile heterotroph with
no photosynthesis cannot reach food and starves on its founding provision. These 761 of
the 3 750 windows are a protocol artefact and should be excluded from every count above
them in this note; none of the headline verdicts change (they were already 0 / 54), but
§2.2's "consumers present in 21 cells and invade in 0" was never a 21-cell result — it is
a 21-cell result on the *realised* rows only, and the absent-role rows should read
*not testable*.

### 4.5 The mean-field signs do not separate the outcomes

Per realised injection, `eigen_excess` at the centroid by observed shape: consumer
−0.24 (flat) / −0.14 (grew) / −0.11 (declining) / −0.20 (extinct); decomposer +142
(flat) / +210 (grew) / +194 (declining) / +119 (extinct). The consumer reduction predicts
negative regardless, the decomposer reduction (at `ι = 1`) predicts strongly positive
regardless; neither reads the `kappa`-clamp or the guild-of-one, because neither has a
coordinate for them. §2.5's agreement table stands as written; it is just not
informative here.

### 4.6 Longer horizons do not grow a guild — but unselected configs sometimes have one

`role-emergence.json` (#421) carries terminal role counts at `t = 2000` on 8 seeds for
the then-atlas (56 configs) and the seed-421 LHS draw (198 configs), on the pre-fix
stepper:

| set | configs | median heterotrophs (C + D) ≥ 5 at `t = 2000` | ≥ 2 |
|---|---|---|---|
| atlas | 56 | 1 | 1 |
| LHS `sample:` | 198 | 4 | 17 |

The four: `sample:55` (median P 42 / C 5.5 / D 20.5, 6 / 8 seeds survive), `sample:20`
(31 / 0.5 / 21.5), `sample:129` (436 / 0.5 / 19.5), `sample:127` (3 / 0 / 6). Decomposer
guilds of ~20 that persist to 2000 on most seeds exist in the parameter space, at roughly
2 % of a random draw; a consumer guild of ≥ 5 appears in one config. The atlas, selected
on `coexistence_fraction`, contains essentially none — the objective rewarded the
straggler configs (separated clusters read off one or two individuals) over the ones with
a heterotroph population. This is the pre-fix stepper and the pre-#474 atlas, so the
specific configs need re-checking on `main`; the shape of the result — populations exist
unselected and are absent selected — is the finding.

### 4.7 What this closes and what follows

- **The horizon question is closed, not deferred.** Running the atlas resident to 2000
  before injecting would show one mixotroph living or dying, not a hidden attractor
  (§4.6, atlas row). Not worth a run.
- **The rarity question is moot for heterotrophs.** The resident *is* rare (`N = 1`) and
  is not growing. The 8-vs-22 concern in §3 applies to the producer rows only.
- **The atlas's "coexisting multi-role" cells are producer monocultures plus one or two
  sessile heterotrophy-dominant individuals.** `coexistence_fraction` reads separated
  trait clusters — a `kappa = 1`, `heterotrophy = 1.2` individual *is* far from the
  producer cluster — and mutual invasibility reads no second trophic level. Both are
  correct about what they measure; they disagree because there was never a heterotroph
  population to agree about. This is the same fault #486 names at the evaluator, one
  level up: roles are being read off individuals, not guilds.
- **The instrument needs two protocol changes** before its verdicts mean what the issue
  intended: (i) an absent role is *not testable* — no canonical-vertex injection (§4.4);
  (ii) "present" should require a guild, not a role tag on ≥ half the seeds (§4.1).
- **The evaluator needs a guild observable** — reported, like `decomposer_fraction`, not
  an objective or binning axis in the first instance. Proposed shape, with starting
  values rather than settled thresholds: heterotroph-by-trait (the existing
  `trophic_roles` read), count ≥ 5 on every sampled tick over the second half of the run,
  at least one birth inside the guild over that window (rules out a long-lived sterile
  founder cohort at exactly 5). Regenerating the atlas with it reported, and counting the
  cells that pass, is what tests the thresholds and decides whether the observable becomes
  a binning axis or folds into `coexistence_fraction`.
- **The positive control** for the amended criterion is `sample:55 / 20 / 129 / 127` on
  the fixed stepper — the first test of whether mutual invasibility can pass anywhere.
  §4.6 is pre-#444 data: **re-run those four to 2000 on `main` first** (a `role_emergence`
  run restricted to them, minutes not tens of minutes) and confirm the guild is still
  there before spending an invasion run on any of them.
- **The §4.2 mixotroph reading should be confirmed, not just stated.** The artifact has no
  per-agent consumption; whether the flat, zero-birth heterotroph cohorts ever emit a
  `Consumed` event is a one-cell, seconds-long check against the event log that belongs
  in the bin-fix work, before this section is cited as the explanation.
## 5. Do the flat cohorts eat? (2026-09-13, #491)

**Status: addendum. Subset run only — `atlas:0,atlas:1` × seeds 1000–1001 × both arms
on the #491 protocol (absent roles `not_testable`, no canonical-vertex injection;
`lineage_consumed_events` added to every injection). Every realised-centroid
`growth_rate` / `lineage_series` is byte-identical to the pre-#491 run of the same subset,
so the numbers below are §4's cohorts, now with their diet.** §4.2 read the flat,
zero-birth heterotroph cohorts as sessile mixotrophs that never eat; the artifact could
not confirm it. The flat consumer cohort on `atlas:0` seed 1000 (centroid `α 0.13`,
`h 1.34`, `mobility 0.34`, `kappa 1.0`; `lineage_series` `[8, 6, 6, …, 6]`, zero births,
identical on both arms) logs **33 `Consumed` events over the 500-tick window — 21 on
living targets, 12 on carcasses** — about five bites per surviving agent in 500 ticks,
against 756 / 797 for the same seed's producer cohort of 8–14. So the letter of §4.2 is
wrong (they do eat) and its substance stands: at ~0.01 events per agent-tick consumption
is not what keeps six agents alive for 475 ticks, and at `kappa = 1` the cohort is sterile
by construction — a flat cohort is a founder cohort that neither breeds nor feeds to any
purpose. Whether photosynthesis at `α = 0.13` is the balance of the budget is still not
in the record (it carries counts, not `energy_delta`), so that half of §4.2 remains a
reading. The other flat shape is different in kind: the decomposer cohorts flat at
**one** agent (`atlas:0` seed 1001 `removed`, `atlas:1` seed 1001 `removed`; `kappa` 0.0
/ 0.72, 16 / 15 births early, then a lone survivor) log 318 and 368 events — a single
agent eating continuously, not a sterile cohort. "Flat" in §4.3 conflates the two, and
the `lineage_consumed_events` field is what separates them on the next full run. As a
selector check only (one seed, `intact` arm), `sample:55` — §4.6's positive control — ran
with all three roles present at `t_inj` (P 1441 / C 4 / D 6) and its consumer cohort went
8 → 62 with 12 617 living / 17 077 carcass events; a single seed, not a verdict.

## Deliverables against the acceptance criteria

- **Deterministic across two runs; covers every atlas live cell × 3 roles × ≥ 8 seeds** —
  82 × 3 × 8 on two arms plus control; identical SHA-1 across two full runs (§1.5).
- **Lineage-tracked invader growth rate, not raw role count (tracking method stated)** —
  §1.2: `Born` events with `second_parent`, the either-parent descent rule, live lineage
  = members on the roster; `cross_births` reported.
- **Per-cell criterion verdict and comparison with `coexistence_fraction`** — §2.1 and the
  appendix table; the artifact carries every verdict and rate summary.
- **Predicted-vs-observed invasion sign table against A1/A2** — §2.5; per-cell in the
  appendix; #482 handled as a tag.
- **No change to stepper, evaluator, or search behaviour** — the sim changes are the `Born`
  event (trajectories byte-identical under the golden digests), `Clone` on `World` and
  its parts, `retain_agents`, and `Clone` on `TopologyProjection`; the bin is the only
  other code.

## Appendix — every cell

Columns: `atlas:i` (the i-th live cell of `atlas.json`), `coexistence_fraction`, median
resident population at `t_inj`; per role and arm the median rate × 10³ over seeds
(positive seeds / seeds), with **+** = invades (strict), + = median positive, − = median
non-positive, and a bracketed entry = role absent from the resident (canonical vertex
injected; not required by the verdict); the cell verdict per arm; and the
predicted-vs-observed sign per role on the `removed` arm (✓ agree, ✗ (+/−) predicted
positive observed negative, ✗ (−/+) the reverse, · absent, split = predicted sign differs
across seeds).

| cell | cf | pop | P intact | C intact | D intact | verdict intact | P removed | C removed | D removed | verdict removed | sign P/C/D (removed) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `atlas:0` | 0.6 | 186 | -5.5 (1/8) − | (-5.5 (0/8) −) | -4.9 (2/8) − | no | -0.3 (2/8) − | (-5.5 (0/8) −) | -4.9 (1/8) − | no | ✗ (+/−) / · / ✗ (+/−) |
| `atlas:1` | 0.2 | 2 | -4.9 (0/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | -1.7 (1/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | ✗ (+/−) / · / · |
| `atlas:2` | 1.0 | 34 | -5.5 (0/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | +2.7 (8/8) **+** | (-5.5 (0/8) −) | (-5.5 (0/8) −) | strict | ✓ / · / · |
| `atlas:3` | 0.6 | 63 | -5.5 (1/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | +4.0 (8/8) **+** | (-5.5 (0/8) −) | (-5.5 (0/8) −) | strict | ✓ / · / · |
| `atlas:4` | 0.0 | 1 | -5.5 (0/5) − | (-5.5 (0/5) −) | (-5.5 (0/5) −) | no | -4.2 (0/5) − | (-5.5 (0/5) −) | (-5.5 (0/5) −) | no | ✗ (+/−) / · / · |
| `atlas:5` | 0.6 | 150 | +4.5 (7/8) + | (-5.5 (0/8) −) | -3.5 (1/8) − | no | +6.6 (8/8) **+** | (-5.5 (0/8) −) | -3.5 (1/8) − | no | ✓ / · / ✗ (+/−) |
| `atlas:6` | 1.0 | 24 | -0.1 (3/8) − | -0.8 (0/8) − | -2.8 (0/8) − | no | +0.2 (4/8) + | -0.6 (1/8) − | -2.4 (0/8) − | no | ✓ / ✓ / ✗ (+/−) |
| `atlas:7` | 0.0 | 2 | -4.2 (0/6) − | (-5.5 (0/6) −) | (-5.5 (0/6) −) | no | -0.8 (1/6) − | (-5.5 (0/6) −) | (-5.5 (0/6) −) | no | ✗ (+/−) / · / · |
| `atlas:8` | 0.4 | 2 | -2.8 (0/7) − | (-5.5 (0/7) −) | -5.5 (0/7) − | no | -1.4 (1/7) − | (-5.5 (0/7) −) | -5.5 (0/7) − | no | ✗ (+/−) / · / ✗ (+/−) |
| `atlas:9` | 1.0 | 183 | +3.0 (7/8) + | -0.4 (3/8) − | +0.7 (6/8) + | no | +5.0 (8/8) **+** | -0.3 (3/8) − | +0.8 (4/8) + | no | ✓ / ✗ (+/−) / ✓ |
| `atlas:10` | 0.8 | 100 | +0.4 (4/8) + | (-5.5 (1/8) −) | (-5.5 (1/8) −) | median | +4.6 (8/8) **+** | (-5.5 (1/8) −) | (-5.5 (1/8) −) | strict | ✓ / · / · |
| `atlas:11` | 1.0 | 36 | -0.6 (2/8) − | (-5.5 (0/8) −) | -3.8 (0/8) − | no | +2.0 (8/8) **+** | (-5.5 (0/8) −) | -5.5 (0/8) − | no | ✓ / · / ✗ (+/−) |
| `atlas:12` | 1.0 | 56 | +3.7 (7/8) + | (-5.5 (2/8) −) | -4.2 (2/8) − | no | +4.7 (8/8) **+** | (-5.5 (2/8) −) | -3.5 (1/8) − | no | ✓ / · / ✗ (+/−) |
| `atlas:13` | 1.0 | 54 | +1.0 (6/8) + | -1.4 (0/8) − | -3.5 (1/8) − | no | +3.4 (8/8) **+** | -1.4 (0/8) − | -3.5 (1/8) − | no | ✓ / ✗ (+/−) / ✗ (+/−) |
| `atlas:14` | 0.8 | 17 | -4.2 (0/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | +0.6 (8/8) **+** | (-5.5 (0/8) −) | (-5.5 (0/8) −) | strict | ✓ / · / · |
| `atlas:15` | 0.4 | 74 | -5.5 (2/8) − | -5.5 (0/8) − | -5.5 (1/8) − | no | +3.8 (8/8) **+** | -5.5 (1/8) − | -1.7 (2/8) − | no | ✓ / ✓ / ✗ (+/−) |
| `atlas:16` | 0.8 | 21 | -4.9 (2/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | +1.7 (6/8) + | (-5.5 (0/8) −) | (-5.5 (0/8) −) | median | ✓ / · / · |
| `atlas:17` | 1.0 | 14 | -1.1 (0/8) − | (-5.5 (0/8) −) | -4.9 (0/8) − | no | +0.9 (8/8) **+** | (-5.5 (0/8) −) | -4.2 (0/8) − | no | ✓ / · / ✗ (+/−) |
| `atlas:18` | 0.6 | 54 | -2.4 (2/8) − | (-5.5 (0/8) −) | -5.5 (1/8) − | no | +4.4 (7/8) + | (-5.5 (0/8) −) | -4.9 (1/8) − | no | ✓ / · / ✗ (+/−) |
| `atlas:19` | 0.8 | 107 | +0.5 (5/8) + | (-5.5 (0/8) −) | -2.1 (1/8) − | no | +1.4 (6/8) + | (-5.5 (0/8) −) | -4.2 (1/8) − | no | ✓ / · / ✗ (+/−) |
| `atlas:20` | 1.0 | 77 | -1.5 (3/8) − | (-5.5 (0/8) −) | -3.5 (0/8) − | no | +4.1 (7/8) + | (-5.5 (0/8) −) | -4.2 (0/8) − | no | ✓ / · / ✗ (+/−) |
| `atlas:21` | 0.6 | 52 | +0.3 (4/8) + | -0.0 (4/8) − | -2.4 (0/8) − | no | +1.6 (8/8) **+** | -0.0 (4/8) − | -1.1 (2/8) − | no | ✓ / ✓ / ✗ (+/−) |
| `atlas:22` | 1.0 | 11 | -5.5 (1/8) − | (-5.5 (1/8) −) | (-5.5 (0/8) −) | no | +0.2 (6/8) + | (-5.5 (1/8) −) | (-5.5 (1/8) −) | median | ✓ / · / · |
| `atlas:23` | 1.0 | 10 | -1.7 (0/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | -0.6 (4/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | ✗ (+/−) / · / · |
| `atlas:24` | 0.8 | 15 | +0.9 (7/8) + | -4.9 (1/8) − | -5.5 (2/8) − | no | +1.0 (8/8) **+** | -3.5 (2/8) − | -5.5 (2/8) − | no | ✓ / ✓ / ✗ (+/−) |
| `atlas:25` | 0.4 | 20 | +0.0 (3/7) − | (-5.5 (0/7) −) | -5.5 (0/7) − | no | +1.6 (5/7) + | (-5.5 (0/7) −) | -5.5 (0/7) − | no | ✓ / · / ✗ (+/−) |
| `atlas:26` | 0.6 | 85 | -0.2 (4/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | +5.7 (8/8) **+** | (-5.5 (0/8) −) | (-5.5 (0/8) −) | strict | ✓ / · / · |
| `atlas:27` | 0.8 | 16 | -0.1 (2/8) − | (-5.5 (0/8) −) | -5.5 (0/8) − | no | +1.4 (7/8) + | (-5.5 (0/8) −) | -5.5 (0/8) − | no | ✓ / · / split |
| `atlas:28` | 1.0 | 17 | -3.5 (0/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | -1.2 (1/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | ✗ (+/−) / · / · |
| `atlas:29` | 0.8 | 58 | +0.7 (6/8) + | -4.9 (0/8) − | -0.7 (2/8) − | no | +2.9 (8/8) **+** | -4.9 (1/8) − | -0.5 (3/8) − | no | ✓ / ✗ (+/−) / ✗ (+/−) |
| `atlas:30` | 0.8 | 21 | -0.8 (2/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | +0.6 (5/8) + | (-5.5 (0/8) −) | (-5.5 (0/8) −) | median | ✓ / · / · |
| `atlas:31` | 1.0 | 11 | -3.1 (1/8) − | (-5.5 (0/8) −) | -4.9 (0/8) − | no | +0.8 (6/8) + | (-5.5 (0/8) −) | -4.9 (0/8) − | no | ✓ / · / ✗ (+/−) |
| `atlas:32` | 0.8 | 14 | -1.4 (1/8) − | (-5.5 (0/8) −) | -4.2 (1/8) − | no | +0.5 (6/8) + | (-5.5 (0/8) −) | -2.8 (1/8) − | no | ✓ / · / ✗ (+/−) |
| `atlas:33` | 0.6 | 2 | -1.4 (2/7) − | (-5.5 (1/7) −) | (-5.5 (0/7) −) | no | -0.6 (2/7) − | (-5.5 (0/7) −) | (-5.5 (0/7) −) | no | ✗ (+/−) / · / · |
| `atlas:34` | 0.6 | 26 | -3.1 (1/8) − | (-5.5 (1/8) −) | -5.5 (0/8) − | no | +0.0 (3/8) − | (-5.5 (1/8) −) | -2.2 (1/8) − | no | ✗ (+/−) / · / ✗ (+/−) |
| `atlas:35` | 1.0 | 9 | -2.8 (1/8) − | (-5.5 (0/8) −) | -5.5 (0/8) − | no | -0.3 (1/8) − | (-5.5 (0/8) −) | -4.9 (0/8) − | no | ✗ (+/−) / · / ✗ (+/−) |
| `atlas:36` | 0.8 | 46 | -1.4 (3/8) − | -4.2 (0/8) − | -4.2 (0/8) − | no | +0.9 (6/8) + | -4.2 (0/8) − | -4.2 (0/8) − | no | ✓ / ✓ / ✗ (+/−) |
| `atlas:37` | 0.8 | 30 | +0.9 (6/8) + | (-5.5 (0/8) −) | -4.9 (1/8) − | no | +3.2 (7/8) + | (-5.5 (0/8) −) | -4.9 (1/8) − | no | ✓ / · / ✗ (+/−) |
| `atlas:38` | 1.0 | 30 | +0.2 (4/8) + | (-5.5 (0/8) −) | -2.0 (0/8) − | no | +1.9 (7/8) + | (-5.5 (0/8) −) | -2.0 (0/8) − | no | ✓ / · / ✗ (+/−) |
| `atlas:39` | 0.2 | 3 | -1.4 (1/7) − | (-5.5 (0/7) −) | (-5.5 (0/7) −) | no | -0.3 (2/7) − | (-5.5 (0/7) −) | (-5.5 (0/7) −) | no | ✗ (+/−) / · / · |
| `atlas:40` | 1.0 | 11 | +0.2 (4/8) + | (-5.5 (0/8) −) | -3.8 (2/8) − | no | +1.1 (7/8) + | (-5.5 (1/8) −) | -3.5 (2/8) − | no | ✓ / · / split |
| `atlas:41` | 1.0 | 17 | -0.6 (2/8) − | -5.5 (1/8) − | -4.2 (0/8) − | no | +0.9 (6/8) + | -5.5 (0/8) − | -4.2 (0/8) − | no | ✓ / ✓ / ✗ (+/−) |
| `atlas:42` | 0.6 | 175 | +1.7 (5/8) + | (-5.5 (2/8) −) | -3.2 (3/8) − | no | +5.3 (7/8) + | (-5.5 (2/8) −) | -3.2 (3/8) − | no | ✓ / · / ✗ (+/−) |
| `atlas:43` | 1.0 | 8 | -1.4 (2/8) − | -4.2 (0/8) − | -3.5 (0/8) − | no | +0.1 (4/8) + | -4.2 (0/8) − | -3.5 (0/8) − | no | ✓ / ✓ / ✗ (+/−) |
| `atlas:44` | 1.0 | 8 | -0.6 (0/8) − | (-5.5 (0/8) −) | -5.5 (1/8) − | no | +0.0 (3/8) − | (-5.5 (0/8) −) | -5.5 (2/8) − | no | ✗ (+/−) / · / ✗ (+/−) |
| `atlas:45` | 1.0 | 28 | -1.7 (2/8) − | (-5.5 (0/8) −) | -2.4 (0/8) − | no | +0.4 (5/8) + | (-5.5 (0/8) −) | -1.7 (0/8) − | no | ✓ / · / ✗ (+/−) |
| `atlas:46` | 1.0 | 25 | -2.0 (3/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | +2.5 (8/8) **+** | (-5.5 (0/8) −) | (-5.5 (0/8) −) | strict | ✓ / · / · |
| `atlas:47` | 0.8 | 6 | -0.6 (1/8) − | (-5.5 (0/8) −) | -4.2 (0/8) − | no | -0.1 (3/8) − | (-5.5 (0/8) −) | -3.8 (0/8) − | no | ✗ (+/−) / · / ✗ (+/−) |
| `atlas:48` | 0.8 | 18 | -0.4 (2/8) − | (-5.5 (1/8) −) | -5.5 (1/8) − | no | +0.6 (5/8) + | (-5.5 (0/8) −) | -5.5 (0/8) − | no | ✓ / · / ✗ (+/−) |
| `atlas:49` | 1.0 | 16 | +0.1 (4/8) + | (-5.5 (0/8) −) | -4.2 (0/8) − | no | +0.3 (5/8) + | (-5.5 (0/8) −) | -4.2 (0/8) − | no | ✓ / · / ✗ (+/−) |
| `atlas:50` | 0.6 | 105 | -1.7 (1/8) − | (-5.5 (2/8) −) | -4.2 (0/8) − | no | +4.7 (7/8) + | (-5.5 (2/8) −) | -5.5 (1/8) − | no | ✓ / · / ✗ (+/−) |
| `atlas:51` | 1.0 | 38 | +2.0 (7/8) + | -2.4 (0/8) − | -0.9 (1/8) − | no | +2.7 (8/8) **+** | -2.0 (0/8) − | -0.4 (2/8) − | no | ✓ / ✗ (+/−) / ✗ (+/−) |
| `atlas:52` | 0.8 | 47 | +0.6 (5/8) + | (-5.5 (0/8) −) | -2.8 (0/8) − | no | +3.2 (8/8) **+** | (-5.5 (0/8) −) | -2.4 (0/8) − | no | ✓ / · / ✗ (+/−) |
| `atlas:53` | 1.0 | 19 | +0.9 (6/8) + | (-5.5 (2/8) −) | -0.2 (4/8) − | no | +1.3 (7/8) + | (-5.5 (3/8) −) | -0.5 (3/8) − | no | ✓ / · / split |
| `atlas:54` | 0.2 | 9 | -4.9 (0/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | -2.0 (0/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | ✗ (+/−) / · / · |
| `atlas:55` | 1.0 | 20 | -1.0 (3/8) − | -2.4 (0/8) − | -2.0 (1/8) − | no | +0.9 (6/8) + | -3.5 (0/8) − | -1.7 (0/8) − | no | ✓ / ✗ (+/−) / ✗ (+/−) |
| `atlas:56` | 0.8 | 34 | +1.6 (8/8) **+** | -2.3 (4/8) − | (-5.5 (3/8) −) | no | +2.3 (8/8) **+** | -2.4 (4/8) − | (-5.5 (2/8) −) | no | ✓ / ✓ / · |
| `atlas:57` | 1.0 | 38 | +1.6 (6/8) + | -0.8 (1/8) − | -0.1 (2/8) − | no | +2.1 (7/8) + | -0.6 (2/8) − | -0.1 (3/8) − | no | ✓ / ✗ (+/−) / ✗ (+/−) |
| `atlas:58` | 0.6 | 3 | -2.0 (0/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | -0.9 (0/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | ✗ (+/−) / · / · |
| `atlas:59` | 1.0 | 2616 | -5.5 (0/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | +9.8 (8/8) **+** | (-5.5 (0/8) −) | (-5.5 (0/8) −) | strict | ✓ / · / · |
| `atlas:60` | 0.8 | 7 | -0.1 (3/8) − | -1.9 (0/8) − | (-5.5 (0/8) −) | no | -0.0 (4/8) − | -1.9 (0/8) − | (-5.5 (0/8) −) | no | ✗ (+/−) / ✓ / · |
| `atlas:61` | 1.0 | 11 | -0.4 (3/8) − | -3.5 (1/8) − | -3.5 (0/8) − | no | +0.2 (4/8) + | -4.2 (1/8) − | -3.5 (0/8) − | no | ✓ / ✓ / ✗ (+/−) |
| `atlas:62` | 1.0 | 33 | +2.4 (6/8) + | -4.2 (1/8) − | -5.5 (0/8) − | no | +3.5 (8/8) **+** | -3.8 (1/8) − | -4.9 (0/8) − | no | ✓ / ✗ (+/−) / ✗ (+/−) |
| `atlas:63` | 1.0 | 30 | +1.3 (5/8) + | (-5.5 (1/8) −) | (-5.5 (1/8) −) | median | +1.8 (7/8) + | (-5.5 (1/8) −) | (-5.5 (0/8) −) | median | ✓ / · / · |
| `atlas:64` | 0.0 | 8 | -1.4 (2/7) − | (-5.5 (0/7) −) | (-5.5 (0/7) −) | no | +0.0 (2/7) − | (-5.5 (0/7) −) | (-5.5 (0/7) −) | no | ✗ (+/−) / · / · |
| `atlas:65` | 0.8 | 43 | +1.0 (6/8) + | -1.4 (0/8) − | -5.5 (1/8) − | no | +2.3 (8/8) **+** | -1.4 (0/8) − | -4.2 (2/8) − | no | ✓ / ✓ / ✗ (+/−) |
| `atlas:66` | 0.8 | 10 | -1.7 (1/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | +0.3 (5/8) + | (-5.5 (0/8) −) | (-5.5 (0/8) −) | median | ✓ / · / · |
| `atlas:67` | 0.6 | 3 | -5.5 (0/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | -0.6 (1/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | ✗ (+/−) / · / · |
| `atlas:68` | 0.8 | 64 | -0.0 (4/8) − | (-5.5 (0/8) −) | (-5.5 (1/8) −) | no | +2.8 (8/8) **+** | (-5.5 (0/8) −) | (-5.5 (0/8) −) | strict | ✓ / · / · |
| `atlas:69` | 1.0 | 13 | -1.2 (2/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | +0.0 (3/8) − | (-5.5 (1/8) −) | (-5.5 (0/8) −) | no | ✗ (+/−) / · / · |
| `atlas:70` | 1.0 | 24 | -3.1 (1/8) − | (-5.5 (0/8) −) | -4.9 (0/8) − | no | +1.1 (6/8) + | (-5.5 (0/8) −) | -4.9 (0/8) − | no | ✓ / · / ✗ (+/−) |
| `atlas:71` | 0.4 | 6 | -4.2 (0/8) − | (-5.5 (0/8) −) | -5.5 (0/8) − | no | -1.2 (0/8) − | (-5.5 (0/8) −) | -4.9 (0/8) − | no | ✗ (+/−) / · / ✗ (+/−) |
| `atlas:72` | 1.0 | 23 | -2.8 (1/8) − | (-5.5 (0/8) −) | -4.9 (1/8) − | no | +0.7 (5/8) + | (-5.5 (0/8) −) | -4.9 (1/8) − | no | ✓ / · / ✗ (+/−) |
| `atlas:73` | 0.8 | 27 | -5.5 (1/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | +2.5 (7/8) + | (-5.5 (0/8) −) | (-5.5 (0/8) −) | median | ✓ / · / · |
| `atlas:74` | 1.0 | 19 | -1.4 (1/8) − | -5.5 (0/8) − | (-5.5 (0/8) −) | no | +0.5 (5/8) + | -4.9 (0/8) − | (-5.5 (0/8) −) | no | ✓ / ✓ / · |
| `atlas:75` | 1.0 | 28 | +2.6 (6/8) + | (-5.5 (1/8) −) | (-5.5 (0/8) −) | median | +3.1 (7/8) + | (-5.5 (1/8) −) | (-5.5 (1/8) −) | median | ✓ / · / · |
| `atlas:76` | 1.0 | 37 | +1.3 (6/8) + | -2.8 (0/8) − | -4.9 (0/8) − | no | +2.3 (7/8) + | -2.4 (0/8) − | -4.9 (0/8) − | no | ✓ / ✗ (+/−) / ✗ (+/−) |
| `atlas:77` | 0.4 | 6 | -1.0 (1/8) − | (-5.5 (0/8) −) | -4.2 (0/8) − | no | -0.3 (2/8) − | (-5.5 (0/8) −) | -4.2 (0/8) − | no | ✗ (+/−) / · / ✗ (+/−) |
| `atlas:78` | 0.8 | 29 | +1.7 (8/8) **+** | +0.1 (4/8) + | -2.0 (1/8) − | no | +2.2 (8/8) **+** | -0.3 (3/8) − | -2.0 (1/8) − | no | ✓ / ✓ / ✗ (+/−) |
| `atlas:79` | 0.0 | 4 | -2.4 (1/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | -1.2 (1/8) − | (-5.5 (0/8) −) | (-5.5 (0/8) −) | no | ✗ (+/−) / · / · |
| `atlas:80` | 1.0 | 11 | -1.7 (0/8) − | (-5.5 (0/8) −) | -4.9 (1/8) − | no | +0.0 (3/8) − | (-5.5 (0/8) −) | -4.9 (0/8) − | no | ✗ (+/−) / · / ✗ (+/−) |
| `atlas:81` | 0.8 | 33 | -0.1 (3/8) − | (-5.5 (0/8) −) | -1.7 (0/8) − | no | +2.2 (8/8) **+** | (-5.5 (0/8) −) | -2.0 (0/8) − | no | ✓ / · / ✗ (+/−) |

