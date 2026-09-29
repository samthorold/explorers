# Issue #589: how much of the search box leaves heterotrophs unable to cover maintenance on carcass income?

**Status: measurement. Adds a closed-form module (`explorers_search::heterotroph_margin`)
and a research bin (`heterotroph_margin`); changes no stepper, evaluator, search box or
prefilter. `explorers-sim` is unchanged.** [#587](587-reinvasion-barrier.md) found that on
`sample:31` the pure heterotroph keeps `e ≈ 0.02` of a producer carcass, so its carcass-intake
ceiling (≈ 0.022 E/tick) sits below its base and heterotrophy maintenance (≈ 0.034 E/tick).
This note computes the same comparison for every config of the #509 sweep and of the current
atlas. It asks three things: how much of the box has no heterotroph phenotype that pays, whether
that predicts lockup or guild absence beyond `light_competition_radius`, and which box axes set
it.

## TL;DR

1. **On one carcass in reach, 84 % of the box has no phenotype on the producer→heterotroph line
   that covers its maintenance.** On the unbiased LHS sample this is 168 of 200 configs. It is
   67/82 on the #509 atlas and 89/95 on the current atlas. The pure heterotroph (#587's `full`
   phenotype) fails on 188/200 (94 %).
2. **The margin predicts nothing about outcomes.** Its Spearman correlation with a config's
   lockup, live, consumer and guild fractions is within ±0.10 on the LHS sample. Its
   cross-validated `R²` alone is below zero on every outcome. Adding it to the raw coordinates or
   to the radius changes `R²` by at most 0.01 either way: lockup goes 0.357 → 0.360 on the raw
   32 and 0.206 → 0.196 on the radius alone. Lockup is 0.17 where the margin is ≤ 0 and 0.18
   where it is positive.
3. **The margin is almost entirely `base_metabolic_rate`.** Its Spearman correlation with the
   margin is −0.86, and it alone gives cross-validated `R²` 0.64 for the margin. With
   `base_trophic_efficiency` and `trophic_distance_decay` added, `R²` reaches 0.75, which is all
   32 coordinates give. `heterotrophy_maintenance_cost` barely matters. Leave out the base term
   and every config (198/198) has a heterotrophy step that repays its own maintenance on one
   carcass.
4. **`sample:31` is in the minority where the line pays.** Mixotrophs up to 90 % of the way to
   the pure heterotroph cover base and heterotrophy maintenance on one carcass. The best point is
   `t = 0.29`, with margin +0.055 E/tick. Only the pure corner fails. That is #587's finding, and
   this config's low base rate (0.0117, near the box floor) is what makes it a kernel story.
   Most of the box fails on the base rate, not the kernel. Worlds that fail by that measure,
   including #462's three live baselines `sample:110`, `18` and `188` (margins −0.39 to −0.47),
   hold consumers at `T` on 7 or 8 of 8 seeds.
5. **Reading: the third one.** The margin is ≤ 0 across most of the box, but it carries no
   information beyond the radius, or beyond anything else. A single-carcass income-vs-maintenance
   comparison is not what separates lockup or heterotroph-absent worlds from the rest. Where
   consumers persist, they persist on the emergent terms this closed form holds at one: carcasses
   and prey in reach at once, and the reserve. That is viability's resolved finding
   ("decomposer-return floor") seen from the data side. The `sample:31` pure-corner failure is
   a feature of that config, not the box's lockup mechanism. Recorded, and stopped.

## 1. What is computed

For a config, the **line** runs in trait space from the founder producer centroid to the pure
heterotroph with the same trophic budget `B = mean_photosynthetic_absorption + mean_heterotrophy`.
Every other trait is held at the founder mean. On a multi-cluster founding the producer end is the
producer cluster `World::new` seeds: `(photo B, heterotrophy 0)`, the `representative_clusters`
rule `permanence_crosscheck` records. A single-cluster founding starts from the mean. The pure
heterotroph end is #587's `full` phenotype, `founder_heterotroph_centroid`. Every phenotype on
the line targets **the producer centroid's carcass**, which keeps its dead agent's trait vector.

At `t ∈ {0, 0.001, …, 1}` (1001 grid points), with `h(t)` the phenotype's heterotrophy and
`d(t)` its trait distance to the producer carcass, the terms are:

| term | formula | source |
|---|---|---|
| efficiency `e` | `base_trophic_efficiency · exp(−trophic_distance_decay · d)` | `explorers_sim::trophic_transfer_efficiency`, flow 7 |
| intake ceiling | `h · u_H · e`, with `u_H = 1` | the binary-reach drain on one carcass in reach, as #587's bin computes it |
| maintenance (#587's terms) | `base_metabolic_rate + h^x · heterotrophy_maintenance_cost` | flow 8; #587's `0.0117 + 1.086^1.80 × 0.0194` |
| **margin** | intake ceiling − maintenance | |
| strict margin | margin − (`photo^x·c_P + mobility^x·c_M + asexual^x·c_A`) | the other trait terms of flow 8 |
| break-even count `k*` | maintenance ÷ intake ceiling | carcasses in reach per tick needed to pay |

A config's **best margin** is the maximum over the line, and it has **no positive margin** when
the best is ≤ 0. The read also records where the best point sits (`best_t`) and the last `t` with
a positive margin (`positive_reach`). It records the best point among nominal heterotrophs
(`heterotrophy > photo`, `topology::trophic_roles`' split, i.e. `t > 0.5`), the pure
heterotroph's terms, and the smallest `k*` on the line.

**Terms left out**, named as the issue asks:

- **Carcass multiplicity.** The drain is per target in reach, so `k` carcasses in reach give
  `k ×` the ceiling. The margin is set at `k = 1`, as #587's is. `k*` reports what the line
  would need.
- **Living prey.** It is drained by the same kernel.
- **Photosynthetic income** of a mixotroph on the line. The margin asks whether the carcass pays,
  not whether the phenotype lives.
- **The finite energy per carcass.**
- **Wear**, which lowers effective heterotrophy.
- **Structure and somatic maintenance, growth retention and repair**, which are body-dependent.
- **The κ split.** #587's founders died with 80 % of their reserve earmarked.

The strict margin adds back the photosynthesis, mobility and asexual-propensity trait terms.

## 2. The `sample:31` pin

`HeterotrophLine::from_founders(sample:31)` at `t = 1` gives:

- `e = 0.0224` (rounds to #587's 0.02);
- maintenance `0.0342` (#587: 0.034);
- intake ceiling `0.0243`;
- margin −0.0099.

#587's table reads 0.021 and 0.022 because its `e` is against the *resident* producer centroid
at tick 1000, as a median over 8 seeds. That centroid has drifted in mobility, κ, fecundity and
dispersal as well as in the trophic traits, so it sits a little further off. The founder centroid
used here is the one the issue specifies.

A second test feeds seed 1000's recorded resident centroid (copied from #587's artifact) through
the same `margin_terms`. It reproduces that seed's recorded `e = 0.023537695` and ceiling
`0.02555098` bit for bit, so this is the computation #587 made. Both are pinned in
`heterotroph_margin::tests`.

## 3. What ran

| | |
|---|---|
| sweep outcomes | `target/permanence-crosscheck.jsonl` (#509; SHA-1 `e92eb370…`, the file #462 read): 282 configs × 8 seeds at `T = 2000`, per-seed terminal mode, consumers and decomposer guild. Outcomes are seed fractions over finished seeds, as in #462 |
| #509 atlas | the sweep's `atlas:i` are the 82 cells of the atlas **before #545 regenerated it**: `git show c86c456^:atlas.json`. The bin checks every joined row. Each of the 282 recorded founder producer centroids equals the one decoded here, and the join refuses the current `atlas.json` at `atlas:0` |
| current atlas | `atlas.json` (95 cells, full box). It records `decomposer_fraction`, `consumer_fraction` and `coexistence_fraction`, not lockup |
| commands | `heterotroph_margin --atlas <c86c456^ atlas> --sweep target/permanence-crosscheck.jsonl --out target/heterotroph-margin-509.jsonl`; `heterotroph_margin --configs atlas:0,…,atlas:94 --out target/heterotroph-margin-atlas.jsonl`; `heterotroph_margin --slice X,Y` for §6 |
| wall clock | under a second per run. The output is byte-identical on a re-run |
| statistics | a throwaway NumPy script over the JSON-lines output, following #462's procedure: ridge on z-scored features, 10-fold CV with fixed folds (seeded permutation), best of λ ∈ {0.1, 1, 10, 100}. The script is not committed (the #462 / #474 precedent) |

The sweep's outcomes are from `c68b467` code. #541 and #543 have changed the stepper and evaluator
since then, and #462's radius sweep re-run showed what that moves. The margin itself is closed
form and current.

## 4. How much of the box

| population | n | no positive margin | strict | none among nominal heterotrophs | pure heterotroph ≤ 0 | median best margin | median `k*` |
|---|---:|---:|---:|---:|---:|---:|---:|
| LHS sample (seed 421) | 200 | **168 (0.84)** | 173 (0.87) | 173 (0.87) | 188 (0.94) | −0.168 | 3.5 |
| #509 atlas (pre-#545) | 82 | 67 (0.82) | 68 (0.83) | 68 (0.83) | 74 (0.90) | −0.180 | 3.7 |
| current atlas | 95 | 89 (0.94) | 92 (0.97) | 92 (0.97) | 92 (0.97) | −0.176 | 4.1 |

Where the line does pay (32 LHS configs), the median best point is `t = 0.39`, a mixotroph. The
median `positive_reach` is 0.83, so on those configs most of the line toward the heterotroph
pays too. On 16 LHS configs the best point is the producer itself (`t = 0`): no heterotrophy step
pays for its own cost there.

## 5. Against the outcomes

### Cross-tab

LHS sample, the 198 configs with a finished seed. Mean per-config seed fractions:

| | n | live | lockup | consumers | guild |
|---|---:|---:|---:|---:|---:|
| margin > 0 | 32 | 0.632 | 0.180 | 0.354 | 0.012 |
| margin ≤ 0 | 166 | 0.715 | 0.170 | 0.351 | 0.004 |
| Spearman (best margin, outcome) | | −0.05 | −0.01 | −0.06 | +0.02 |

| best-margin quartile | n | live | lockup | consumers | guild |
|---|---:|---:|---:|---:|---:|
| Q1 [−0.471, −0.284] | 50 | 0.688 | 0.198 | 0.363 | 0.007 |
| Q2 [−0.280, −0.168] | 49 | 0.798 | 0.100 | 0.405 | 0.003 |
| Q3 [−0.168, −0.040] | 49 | 0.659 | 0.201 | 0.310 | 0.003 |
| Q4 [−0.039, +0.445] | 50 | 0.662 | 0.185 | 0.329 | 0.007 |

No monotone trend appears in any column. The margin is ≤ 0 on 29 of the 35 configs that lock up
on at least half their seeds, and on 104 of the 125 that never lock up: the same 83 % as the box.
It is ≤ 0 on 59 of the 69 configs that hold consumers on at least half their seeds. The guild is
too rare to read: 7 configs have any guild seed, and 4 of them have margin ≤ 0.

Stratified by radius (unit < 0.4, the bloom-then-lockup half):

- margin > 0: 13 configs, lockup 0.39;
- margin ≤ 0: 65 configs, lockup 0.32.

The positive-margin side locks up slightly *more*, the opposite of the hypothesis, on a small n.
Pooled with the #509 atlas (n = 279) the picture is the same: lockup 0.23 vs 0.20, Spearman
+0.005.

Every #509 lockup cell #587 named as a candidate other than `sample:31` has margin ≤ 0:

| cell | best margin | pure heterotroph margin | `base_metabolic_rate` | lockup | consumers |
|---|---:|---:|---:|---:|---:|
| `sample:31` | **+0.055** (`t = 0.29`, reach 0.90) | −0.010 | 0.012 | 1.00 | 0.00 |
| `atlas:2` (pre-#545) | −0.257 | −0.283 | 0.385 | 1.00 | 0.00 |
| `sample:154` | −0.444 | −0.465 | 0.468 | 1.00 (1 seed) | 0.00 |
| `sample:158` | −0.326 | −0.419 | 0.393 | 0.75 | 0.00 |
| `sample:192` | −0.455 | −0.476 | 0.484 | 1.00 | 0.00 |
| `sample:110` (#462 baseline) | −0.392 | −0.426 | 0.492 | 0.00 | 1.00 |
| `sample:18` (#462 baseline) | −0.471 | −0.520 | 0.499 | 0.00 | 1.00 |
| `sample:188` (#462 baseline) | −0.406 | −0.511 | 0.479 | 0.00 | 0.88 |

The last three rows are the point. By this closed form they are as heterotroph-starved as the
lockup cells, and they hold consumers on nearly every seed.

On the **current atlas**, 89 of 95 cells have no positive margin. `consumer_fraction` is 0 on
every cell, so there is nothing to cross. `decomposer_fraction` is positive on 4 cells, all with
margin ≤ 0 (Spearman −0.28, on 6 positive-margin cells). `coexistence_fraction` is 0.57 on the
positive-margin cells and 0.63 on the rest.

### Explained variance

LHS sample, n = 198. Cross-validated `R²` (ridge, best λ). "margin" is the best margin on the
line, and "margin3" adds the best nominal-heterotroph margin and the pure-heterotroph margin.

| outcome | radius | `log m²` | margin | margin3 | radius + margin | `log m²` + margin | raw32 | raw32 + margin | raw32 + margin3 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| live | 0.309 | 0.225 | −0.023 | −0.020 | 0.306 | 0.224 | 0.401 | 0.394 | 0.392 |
| lockup | 0.206 | 0.160 | −0.026 | −0.028 | 0.196 | 0.150 | 0.357 | 0.360 | 0.360 |
| consumers | −0.012 | −0.010 | −0.020 | −0.018 | −0.023 | −0.022 | 0.230 | 0.227 | 0.223 |
| guild | −0.004 | 0.006 | −0.018 | −0.025 | −0.015 | −0.007 | −0.084 | −0.084 | −0.086 |

The baselines agree with #462 (raw32 live 0.38 / lockup 0.34; `log m²` 0.23 / 0.14). The small
differences come from the fold assignment, which #462 did not record. Across five fold seeds,
lockup `R²` for raw32 spans 0.339–0.381 and for raw32 + margin 0.343–0.385. The margin's
contribution (+0.003 to +0.009) is inside that spread, and it is negative against the radius
alone. Pooled (n = 279) the result is the same: lockup raw32 0.411 → 0.412; radius 0.164 → 0.160.

As a sensitivity check, the margin without the base term (best margin + `base_metabolic_rate`,
since the base term is constant along the line) is positive on all 198 configs. Its Spearman
correlation with lockup is −0.15, and it moves raw32's `R²` by under 0.01.

## 6. Axis map

Spearman correlation of the best margin with each unit coordinate over the LHS sample, largest
first:

| axis | Spearman |
|---|---:|
| `base_metabolic_rate` | −0.857 |
| `base_trophic_efficiency` | +0.273 |
| `trophic_distance_decay` | −0.217 |
| `mutation_magnitude` | −0.185 (chance: the margin does not read it) |
| `initial_population_size` | +0.132 (chance, likewise) |

Cross-validated `R²` for the margin:

- `base_metabolic_rate` alone: 0.635;
- plus `base_trophic_efficiency` and `trophic_distance_decay`: 0.752;
- plus `heterotrophy_maintenance_cost`, the trophic means, the exponent and the cluster count:
  0.775;
- all 32 coordinates: 0.756.

The margin is a three-axis quantity, and the base rate dominates it. The issue expected
`heterotrophy_maintenance_cost` to matter; it does not. The best point sits at small `h`, where
`h^x · c_H` with `x ≥ 1.5` is tiny.

Slices from `heterotroph_margin --slice X,Y` hold every other axis at the box median (unit 0.5:
`base_metabolic_rate` 0.255, `trophic_distance_decay` 2.55, `base_trophic_efficiency` 0.5,
`heterotrophy_maintenance_cost` 0.051, budget 1.0, exponent 2.25). Each entry is the best margin
in E/tick.

`trophic_distance_decay` (rows) × `base_trophic_efficiency` (columns), at the median base rate:

| | 0.1 | 0.3 | 0.5 | 0.7 | 0.9 |
|---|---:|---:|---:|---:|---:|
| 0.100 | −0.214 | −0.045 | +0.129 | +0.302 | +0.476 |
| 0.713 | −0.235 | −0.173 | −0.105 | −0.035 | +0.036 |
| 1.325 | −0.241 | −0.205 | −0.166 | −0.128 | −0.089 |
| 2.550 | −0.247 | −0.227 | −0.206 | −0.186 | −0.166 |
| 5.000 | −0.250 | −0.240 | −0.230 | −0.219 | −0.209 |

`trophic_distance_decay` (rows) × `base_metabolic_rate` (columns):

| | 0.010 | 0.071 | 0.133 | 0.255 | 0.378 | 0.500 |
|---|---:|---:|---:|---:|---:|---:|
| 0.100 | +0.374 | +0.312 | +0.251 | +0.129 | +0.006 | −0.116 |
| 0.713 | +0.140 | +0.079 | +0.018 | −0.105 | −0.227 | −0.350 |
| 1.325 | +0.079 | +0.017 | −0.044 | −0.166 | −0.289 | −0.411 |
| 2.550 | +0.039 | −0.023 | −0.084 | −0.206 | −0.329 | −0.451 |
| 5.000 | +0.015 | −0.046 | −0.107 | −0.230 | −0.352 | −0.475 |

At the box median the zero crossing lies at the flat-kernel edge: `trophic_distance_decay ≲ 0.7`
with `base_trophic_efficiency ≳ 0.8`, or `≲ 0.1` with `≳ 0.35`. Along the base-rate axis it lies
at `base_metabolic_rate ≈ 0.05` for a median kernel, rising to ≈ 0.38 for the flattest. Across
`heterotrophy_maintenance_cost` ∈ [0.001, 0.1] the margin moves by at most 0.005 at any base
rate. The margin's zero crossing is essentially one line, "base rate below what one carcass keeps
at the best point", and at the median kernel one carcass keeps about 0.04–0.05 E/tick.

## 7. Reading

**The third reading holds.** The margin is ≤ 0 across most of the box, which is the first half of
reading 1. But it explains nothing about lockup, live, consumer or guild fractions, alone or on
top of the radius or the raw coordinates. It is also not the minority-overlapping-lockup pattern
of reading 2: the lockup configs are no more heterotroph-starved by this measure than the rest.

Two things make the single-carcass margin uninformative:

- **It is mostly the base rate.** `base_metabolic_rate` is paid by producers and heterotrophs
  alike, so a world where one carcass cannot pay it is a world where every body is expensive, not
  one that singles out heterotrophs. The heterotroph-specific part (the heterotrophy step against
  its own cost) is positive everywhere.
- **Consumers persist where the margin says they cannot.** They do so on the terms the closed form
  holds at one: several carcasses or prey in reach at once (median `k* ≈ 3.5`), photosynthesis
  carried along, and reserve. Those are the emergent reach and carcass-richness terms viability's
  resolved finding names as the reason lockup has no clean gate. This sweep shows them from the
  data side: a per-parameter income-vs-maintenance floor at `k = 1` does not track outcomes.

`sample:31` is idiosyncratic in the opposite direction from the issue's framing. It is one of the
few configs where the carcass *does* pay a mixotroph (base rate 0.012), and its lockup comes with
no consumers at all. Its pure-heterotroph failure (#587) is the kernel pricing the far corner, and
it is not a box-wide lockup mechanism.

## 8. Recommendations (not acted on)

- **No change to the search box's trophic ranges or the kernel's shape on this evidence.** The
  margin that would motivate it predicts nothing. If a design discussion about heterotroph
  economics is wanted anyway, the lever this data points to is `base_metabolic_rate`'s range
  (0.01–0.5), which sets 64 % of the margin's variance, not `trophic_distance_decay`.
- **A decomposer-return floor stays un-drawable as a gate.** Drawn at `k = 1`, it would call 84 %
  of the box heterotroph-starved, including worlds that hold consumers on every seed. That is
  viability's resolved finding, now measured.
- **What would answer `sample:31`'s lockup** is the `k*` question on its own run: how many
  carcasses a resident near-producer has in reach, against the 0.20 it needs. That needs
  `reinvasion_barrier`'s run, not a closed form. It was not run here.

## 9. What this does not cover

- **The #509 outcomes are from pre-#541/#543 code** (`c68b467`). No sweep was re-run. The current
  atlas's own fractions are current, but it records no lockup, and its `consumer_fraction` is 0
  on every cell.
- **The #462 radius sweep** (`target/radius-sweep-v2.jsonl`) moves one axis on four baselines. The
  margin does not read the radius, so each baseline's margin is constant across its levels, and
  nothing is added by crossing it.
- **The held-out draw** (`sample@9421`, `target/permanence-crosscheck-9421-*.jsonl`) was not read.
  The bin takes `--configs sample@9421:i` if wanted.
- **No simulation spot check.** The margin carried no signal to follow up.
- **Only one line.** Phenotypes off the producer→heterotroph line (other mobility, body size or κ)
  and targets other than the founder producer carcass (a heterotroph carcass, a drifted resident)
  are not searched. Moving the other traits only adds distance, so it cannot raise `e`. It can
  lower the other trait maintenance, which the (#587-comparable) margin does not charge.

## 10. Instrument

`crates/explorers-search/src/heterotroph_margin.rs` holds the closed form:

- `HeterotrophLine::from_founders(dist)`, `phenotype(t)`, `terms_at(t, params)` and `read(params)`;
- `margin_terms(phenotype, carcass, params)`.

`crates/explorers-search/src/bin/heterotroph_margin.rs` writes one JSON line per config:

- the unit vector and the margin's axes;
- the founder producer;
- the line read;
- the atlas cell's recorded fractions;
- the joined sweep outcome.

It takes the shared `config_source` selectors through `--configs` and decodes atlas cells over the
atlas's own box. It reads legacy atlases, those without `consumer_fraction` or a recorded box.
`--sweep` refuses a sweep whose founder producers do not match the decoded configs.
`--slice X,Y` prints a 9 × 9 slice at the box medians.

The tests are the `sample:31` pin (founder terms and #587's recorded seed-1000 values) plus smoke
tests. The smoke tests cover line dominance, a constructed positive case, the strict and
break-even arithmetic, reach and the nominal-heterotroph split, sweep-fraction arithmetic, the
join and its refusal, row order and determinism, the slice and the CLI.
