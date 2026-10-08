# Issue #670: the deciding read for #662, fresh atlases with the deficit rule and `c_AH` searched

**Status: measurement, docs only (2026-10-05). The deciding read for #662. Searches two fresh
atlases (seeds 42 and 43) on main `7fc7fb5`, with the flow-3 deficit rule (#666) and the cross-trait
cost `c_AH` in genesis's box over [0, 0.14] (#669). Reads #656/#668's readouts on each, decoded and
at `founder_aggregation = 0`, and applies the need gate's closing rule. No code, design or atlas
change. #662 stays open for its owner's verdict; the recommendations below are the owner's to take.**

A **light-fed mixotroph** is a Producer by recent-income role with effective heterotrophy
`h_eff > 0.05` (#655). Flow 1 ([world rules](../system-design/world-rules.md)) holds it to two
tests: its heterotrophy is **conditional** (it falls where the pool under it is rich), and it drains
a **minority** of the carcass pile on the settled half of a run. #662 committed the autotrophy ×
heterotrophy cost `c_AH × (A·H)^(p/2)` (trade-off #5), working with the deficit rule, as the physics
that should make producer heterotrophy conditional.

## TL;DR

fa 0 = `founder_aggregation = 0`. Seed 42: 99 configs × 5 seeds. Seed 43: 96 × 5.

| run | conditionality: pooled ρ / median per-config ρ (configs ρ < 0, binomial p) | mixotroph share of carcass structure, second half | persisted | lockup | `(k, τ)` region, n = 4 (closest margin) |
|---|---|---:|---:|---:|---|
| #656, decoded | −0.134 / −0.060 (44 of 79, 0.37) | 45.8 % | 89.9 % | 17 (4.3 %) | empty (−0.220) |
| #668, decoded | −0.144 / −0.017 (42 of 79, 0.65) | 45.4 % | 88.6 % | 13 (3.3 %) | empty (−0.119) |
| **#670 seed 42, decoded** | **−0.179 / −0.021 (52 of 99, 0.69)** | **61.6 %** | **90.7 %** | **21 (4.2 %)** | **empty (−0.267)** |
| **#670 seed 43, decoded** | **−0.129 / −0.079 (62 of 96, 0.006)** | **62.0 %** | **89.8 %** | **14 (2.9 %)** | **non-empty, 41 of 276 cells (+0.035)** |
| #656, fa 0 | −0.048 / +0.051 (33 of 79, 0.18) | 48.4 % | 91.9 % | 18 (4.6 %) | empty (−0.078) |
| #668, fa 0 | −0.040 / +0.015 (36 of 79, 0.50) | 52.8 % | 93.2 % | 15 (3.8 %) | empty (−0.030) |
| **#670 seed 42, fa 0** | **−0.165 / −0.026 (55 of 99, 0.31)** | **59.3 %** | **89.9 %** | **25 (5.1 %)** | **empty (−0.226)** |
| **#670 seed 43, fa 0** | **−0.089 / −0.036 (53 of 96, 0.36)** | **62.0 %** | **90.2 %** | **17 (3.5 %)** | **empty (−0.115)** |

**Verdict for #662: on genesis's worlds, the deficit rule with `c_AH` searched does not deliver
flow 1's two tests.**

1. **Conditionality passes by the rule in all four readings, but it is not a clear negative
   relation** (§3). At fa 0 it is the first pass since #655's `b = 1` perturbation. But in three of the four readings the
   share of configs with ρ < 0 is a coin's (p = 0.31–0.69), as in #656 and #668. Only seed 43
   decoded is told apart from a coin (62 of 96, p = 0.006, median −0.079), and its seed-43 fa 0
   twin is not (53 of 96). Within each atlas, a config's `c_AH` does not predict its ρ.
2. **The minority share fails clearly, in all four readings** (§4). It is 59–62 % on the settled
   half, against 45–53 % in #656 and #668. The carcass pile per seed is 52–78 % of #668's.
   Decomposers' drain falls by 37–54 % and the mixotrophs' by 9–42 %, so the mixotrophs' share
   rises. By flow 1's mapping, this is the failure that brings carcass access out of reserve.
3. **The cost does thin kin-killing, and it does not kill worlds** (§5, §6). Mixotroph kin kills
   per 1000 Producer agent-ticks in the second half are 1.4–1.5 (2.4 at seed 43 fa 0), against
   2.6–2.9 under the deficit rule alone. 90–91 % of seeds persist, and lockup is 2.9–5.1 %.
4. **`c_AH` does not pile at the top** (§2). Its median is 0.082 (seed 42) and 0.075 (seed 43),
   with 8 and 10 cells in [0.126, 0.14]. Its spread is what search geometry gives every dimension.
   #668's trigger to widen the top to 0.25 does not fire.

**Closing rule (§8, §10): recommend closing #630/#631 and withdrawing the fullness gate.** The rule
un-defers #630 only if the region is non-empty both decoded and at fa 0. On seed 42 it is empty
in both modes, further from the bars than #668. On seed 43 it is non-empty decoded, the first
non-empty reading since #655's `b = 1` perturbation. But it is empty at fa 0, and the decoded
kept bar rests on 3,228 consumer ticks. One seed of one config carries 55 % of kept's denominator.

**The atlas (§10): replace the committed atlas with seed 42's 34-dimension atlas, in its own slice,
once the owner's #662 verdict settles whether the physics changes again.** Seed 42 is
bit-reproducible and seed 43 is not. 7 of seed 43's rollouts hit the wall-clock budget.

## 1. What ran

| | seed 42 | seed 43 |
|---|---|---|
| Tree | main `7fc7fb5` (#666's deficit rule, #667's `cross_trait_cost`, #669's box with `c_AH` linear over [0, 0.14]). Release binaries pinned in `target/670/bin` | same |
| Search | `explorers-search --batch 32 --generations 10 --ensemble 5 --max-ticks 2000 --seed 42`, the settings of #494, #656 and #663 | `--seed 43` |
| Wall clock | 824 s | 3,858 s |
| Generations | gen 0 (bootstrap) to gen 10/10 | gen 0 to gen 10/10 |
| Rollouts skipped / unfinished | 0 / 0. Bit-reproducible | 0 / **7** (2 in gen 3, 3 in gen 5, 2 in gen 6). **Not** bit-reproducible |
| Live cells (of 8000) | 99 | 96 |
| QD-score / best fitness / median cell fitness | 21.30 / 0.446 / 0.191 | 25.20 / 0.551 / 0.316 |
| Dead configs | 64: lockup 42, extinction 11, monoculture 6, energy death 3, bloom stop 2 | 58: lockup 35, monoculture 10, bloom stop 8, extinction 5 |
| Box | 34 dimensions, `cross_trait_cost` [0, 0.14] last | same |
| Atlas fingerprint (on every readout row) | `aa2662b26da489a3` | `e03608050c196850` |
| Recipe | atlas:90, cell [9, 19, 6], `c_AH` 0.139, `b` 0.717; fitness 0.446, refined 0.297 at n = 32 (coexistence 0.60 → 0.62) | atlas:6, cell [0, 10, 5], `c_AH` 0.081, `b` 0.462; fitness 0.551, refined 0.329 (1.00 → 0.53) |
| Readouts | 526 s: `kin_killer_diet` 127 s / 147 s, `role_diet_census --fullness` 118 s / 133 s (decoded / fa 0) | 860 s: 209 / 235 s, 197 / 218 s |

Behaviour cells in common: 55 between the two seeds, 53 and 51 with the committed atlas.

**Unfinished rollouts.** Seed 43's generations 3, 5 and 6 took 11 m 38 s, 20 m 07 s and 15 m 05 s,
against 30 s to 4 m for the rest. In those generations 7 seed rollouts exhausted the 600 s
wall-clock budget. The search log's last line and the atlas's `rollouts_unfinished` both read 7.
An unfinished seed enters neither the archive nor the dead frontier
(`crates/explorers-search/src/qd.rs`, *Rollout budgets and reproducibility*). Wall clock decides
which seeds those are, so a rerun on another machine, or under other load, can differ. Seed 42 hit
no budget, so it reproduces bit for bit. #656's and #663's searches hit none either.

**Generation count.** Both logs run from `gen 0/10` to `gen 10/10`. That is 11 generations: gen 0
is the bootstrap, and `--generations 10` counts the adaptation generations after it
(`explorers-search --help`). #656 recorded the same 11. Neither log stops at gen 9.

**Readouts** are `target/670/readouts.sh`, a pinned copy of #668's driver: `kin_killer_diet` and
`role_diet_census --fullness` (ungated, `--satiation-sensitivity 0`) on every cell, seeds
1000–1004, T = 2000, `EvalConfig::default()`, network off, decoded and at
`--founder-aggregation 0`. Each config decodes with its own `b`, `s_ref` and `c_AH`. The region is
`role_diet_census --summary --fullness-region` on each run's rows (#637's bars and `τ` grid,
#656's `k` grid: 23 × 12 cells per n).

**Checks.**

- Every row's `cross_trait_cost` equals its cell's `c_AH` decoded from the atlas box (all 390
  config rows).
- The per-config ρ recomputed from the rows' `(h_eff, pool N)` samples matches G's table to 0.0005.
- `role_diet_census`'s kin-kill counts match `kin_killer_diet`'s: 19,296 / 32,206 (seed 42) and
  22,677 / 47,995 (seed 43).
- Every carcass bite is attributed (H's unattributed structure is 0.0 in all four runs). B's
  Producer sample count equals A's.

**Comparisons.** #656 (`target/656/`) read the committed atlas under ratio retention at
`c_AH = 0`. #668 (`target/668/`) read the same worlds under the deficit rule at `c_AH = 0`. Both
are 79 configs × 5 seeds. #663 searched seeds 42 and 43 under ratio retention, `b` in the box and
no `c_AH`: 79 and 80 cells, QD-scores 18.54 and 18.97, no readouts on seed 43. The atlases differ,
so persistence and shares are compared as rates, not paired seeds.

## 2. `c_AH` across the cells

`c_AH = 0.14 × unit[33]` (`seed{42,43}/cah-distribution.md`):

| seed | min | p25 | median | p75 | max | mean |
|---|---:|---:|---:|---:|---:|---:|
| 42 | 0.017 | 0.056 | 0.082 | 0.105 | 0.140 | 0.081 |
| 43 | 0.000 | 0.052 | 0.075 | 0.105 | 0.140 | 0.076 |

| `c_AH` in | [0, 0.014) | [0.014, 0.035) | [0.035, 0.07) | [0.07, 0.105) | [0.105, 0.126) | [0.126, 0.14] |
|---|---:|---:|---:|---:|---:|---:|
| seed 42 | 0 | 11 | 27 | 37 | 16 | 8 |
| seed 43 | 5 | 9 | 31 | 27 | 14 | 10 |

- **It does not pile at the top.** 8 and 10 cells sit in the top bin, and 2 and 5 at the box edge.
  #668 §7's trigger, `c_AH` piling at 0.14, does not fire. The top stays 0.14.
- **Its spread is search geometry.** CMA-MAE starts at the box centre (0.07). The mean unit
  coordinate is 0.08 above centre for seed 42 (10th-largest shift of 34) and 0.04 for seed 43
  (20th). Its standard deviation (0.23, 0.26) and the share of cells within 0.25 of centre (65 %,
  61 %) are the median dimension's. The two seeds do not differ (Mann–Whitney p = 0.39), though
  7 other dimensions do at p < 0.01.
- **Genesis does not select on it, either way.** Spearman of `c_AH` against cell fitness is +0.19
  (p = 0.06) and +0.10 (p = 0.33). Seed 43 keeps cells at 0. Worlds with the cost at break-even or
  twice it are found as readily as worlds without it.
- **Section I's scale holds on the new worlds.** The drainers' median of per-agent ratios
  `2 × income / (A·H)^(p/2)` is 0.100 / 0.097 (seed 42, decoded / fa 0) and 0.101 / 0.136
  (seed 43), against #668's 0.106 / 0.137. The median carried term falls, from 0.23 / 0.21 to
  0.18 / 0.19 and 0.14 / 0.14. Light-fed mixotrophs that never drain in the second half are
  41.7 / 45.7 % and 46.0 / 47.6 %, against 52.1 / 47.2 %.

## 3. Conditionality (G)

Producer `h_eff` against pool N at the cell, second half:

| run | pooled ρ | n | median per-config ρ | configs ρ < 0 | two-sided binomial p |
|---|---:|---:|---:|---:|---:|
| #656, decoded | −0.134 | 405,024 | −0.060 | 44 of 79 | 0.37 |
| #668, decoded | −0.144 | 361,451 | −0.017 | 42 of 79 | 0.65 |
| seed 42, decoded | −0.179 | 429,127 | −0.021 | 52 of 99 | 0.69 |
| seed 43, decoded | −0.129 | 553,159 | −0.079 | 62 of 96 | **0.006** |
| #656, fa 0 | −0.048 | 703,608 | +0.051 | 33 of 79 | 0.18 |
| #668, fa 0 | −0.040 | 656,200 | +0.015 | 36 of 79 | 0.50 |
| seed 42, fa 0 | −0.165 | 636,320 | −0.026 | 55 of 99 | 0.31 |
| seed 43, fa 0 | −0.089 | 729,581 | −0.036 | 53 of 96 | 0.36 |

Mean pool N at the cell by `h_eff` bin:

| `h_eff` bin | s42 dec | s43 dec | s42 fa 0 | s43 fa 0 | #668 dec | #668 fa 0 |
|---|---:|---:|---:|---:|---:|---:|
| [0, 0.05) | 706 | 629 | 611 | 473 | 653 | 518 |
| [0.05, 0.2) | 424 | 597 | 412 | 493 | 428 | 468 |
| [0.2, 0.5) | 445 | 481 | 368 | 444 | 511 | 425 |
| [0.5, 1) | 462 | 486 | 424 | 368 | 530 | 425 |
| ≥ 1 | 623 | 642 | 426 | 393 | 512 | 453 |

- **The rule passes everywhere, at fa 0 for the first time since #655's `b = 1` perturbation.** Pooled ρ is more negative than in
  #656 or #668 in three of four readings, and every median per-config ρ is below zero.
- **But the relation within worlds is still mostly a coin's.** #670 asks for "a clear negative
  relation, not #656's coin flip". Three of the four splits cannot be told from a coin. Their
  medians, −0.02 to −0.04, are within #668's band of zero. The pooled ρ ranks worlds at different
  nutrient levels against each other, which is why the rule also asks for the median.
- **Seed 43 decoded is the one clear reading.** 62 of 96 configs are negative and the median is
  −0.079. Cells within an atlas descend from a few emitter lineages (#663), so 96 configs are not
  96 independent tests and p = 0.006 overstates the evidence. Grouping the cells into 8 clusters
  in unit space (Ward), as a proxy for lineage, the cluster medians are negative in 6 of 8. The same
  atlas at fa 0 gives 53 of 96 (5 of 8 clusters). A relation that holds in one seed in one mode is
  a lead, not the design's pattern.
- **The bins are flat or non-monotone above `h_eff` 0.05**, as in #656 and #668. Pure producers sit
  on the richest ground in three readings. In seed 42 and seed 43 decoded the most heterotrophic
  bin sits on ground as rich as theirs. Only seed 43 at fa 0 falls almost monotonically.
- **Retention is still not conditional on the pool (J).** Per carcass bite, ρ(retained / drained,
  pool N under the drainer) is +0.027 pooled / +0.039 median per config (seed 42, decoded),
  −0.003 / +0.036 (seed 42, fa 0), −0.099 / −0.079 (seed 43, decoded) and −0.122 / +0.052
  (seed 43, fa 0). The configs with ρ < 0 are 42 of 87, 44 of 96, 49 of 90 and 42 of 92 (binomial
  p 0.46–0.83). Light-fed mixotrophs keep 0.91, 1.14, 1.47 and 1.61 units of carcass N per unit
  drained, against #668's 0.97 and 1.01. Carcasses give them 3.9–7.7 % of their nutrient.

## 4. Minority share (H)

Light-fed mixotrophs' share of carcass structure drained:

| run | whole run | second half |
|---|---:|---:|
| #656, decoded / fa 0 | 51.4 / 55.6 % | 45.8 / 48.4 % |
| #668, decoded / fa 0 | 52.8 / 58.9 % | 45.4 / 52.8 % |
| seed 42, decoded / fa 0 | 67.3 / 64.9 % | **61.6 / 59.3 %** |
| seed 43, decoded / fa 0 | 67.5 / 66.9 % | **62.0 / 62.0 %** |

Second half, carcass structure drained per seed:

| run | pile | light-fed mixotrophs | decomposers | consumers | no income yet |
|---|---:|---:|---:|---:|---:|
| #668, decoded | 112.1 | 51.0 (45.4 %) | 38.8 (34.6 %) | 4.7 | 17.6 |
| seed 42, decoded | 64.3 | 39.6 (61.6 %) | 18.4 (28.6 %) | 2.3 | 3.9 |
| seed 43, decoded | 71.3 | 44.3 (62.0 %) | 19.5 (27.3 %) | 1.7 | 5.6 |
| #668, fa 0 | 169.1 | 89.3 (52.8 %) | 54.9 (32.4 %) | 8.4 | 16.1 |
| seed 42, fa 0 | 88.1 | 52.2 (59.3 %) | 25.5 (28.9 %) | 3.2 | 6.8 |
| seed 43, fa 0 | 131.7 | 81.7 (62.0 %) | 34.6 (26.2 %) | 3.2 | 11.7 |

- **It fails by 9–12 points in every reading.** It no longer sits at the bar, as in #656, or on
  one side of it by mode, as in #668. Per config the median share is 60–68 %, and 21–33 configs of
  93–98 are below 50 %. No few worlds carry it: the five largest piles hold 27–38 % of the
  structure.
- **The pile shrinks, and the decomposers lose more of it.** Per seed the pile is 52–78 % of
  #668's. Decomposers drain about half as much (46–63 % of #668's), light-fed mixotrophs 58–91 %, and
  agents with no income yet drain far less. Decomposers by role are 0.4–0.7 % of agent-samples,
  against 0.9–1.2 % in #668.
- **A possible reading, not tested here.** The term charges any agent carrying both machineries,
  not only producers. A decomposer by role that carries autotrophy pays it too, and so does the
  route from a producer lineage to a heterotroph through mixotrophic intermediates. If that is what
  thins the decomposers, the cost works against the minority share by construction. Separating
  this needs the term's charge by role, which no readout records.
- **The light-fed mixotrophs keep 85–89 % of all carcass N retained**, against 80 % in #668.

## 5. Persistence and lockup (E)

| run | persisted | lockup | other failures |
|---|---:|---:|---|
| #656, decoded / fa 0 (of 395) | 355 (89.9 %) / 363 (91.9 %) | 17 / 18 | |
| #668, decoded / fa 0 (of 395) | 350 (88.6 %) / 368 (93.2 %) | 13 / 15 | |
| seed 42, decoded (of 495) | 449 (90.7 %) | 21 (4.2 %) | extinction 19, monoculture 6 |
| seed 42, fa 0 | 445 (89.9 %) | 25 (5.1 %) | extinction 12, monoculture 10, generalist dominance 3 |
| seed 43, decoded (of 480) | 431 (89.8 %) | 14 (2.9 %) | extinction 18, monoculture 17 |
| seed 43, fa 0 | 433 (90.2 %) | 17 (3.5 %) | monoculture 21, generalist dominance 6, extinction 3 |

Persistence does not regress: 89.8–90.7 %, within the 88.6–93.2 % of #656 and #668. Lockup is
2.9–5.1 % of seeds, against 3.3–4.6 %. It is spread over 10–17 configs, at most 3 seeds in any one
except one seed-43 config with 5 at fa 0. Seed 43 has more monocultures (17 and 21 seeds, against
3–10 elsewhere).

## 6. Prevalence and kin-killing (A, kin kills)

| | #656 dec | #668 dec | s42 dec | s43 dec | #656 fa 0 | #668 fa 0 | s42 fa 0 | s43 fa 0 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Producer samples with `h_eff > 0.05` | 24.3 % | 26.4 % | 24.9 % | 22.5 % | 25.8 % | 29.9 % | 25.2 % | 30.0 % |
| mean Producer `h_eff` | 0.120 | 0.138 | 0.116 | 0.110 | 0.118 | 0.149 | 0.115 | 0.129 |
| mixotroph kin kills / 1000 P agent-ticks, second half | 1.84 | 2.87 | 1.44 | 1.54 | 1.65 | 2.57 | 1.48 | 2.39 |

The cost undoes the rise in kin-killing the deficit rule brought (#668 §3), and goes below #656 in
three readings. Prevalence returns to about #656's level, except seed 43 at fa 0.

## 7. `c_AH` within each atlas

Per config: `c_AH` against (a) per-config conditionality ρ, (b) the mixotrophs' second-half share
of carcass structure, (c) light-fed mixotroph prevalence (Producer samples with `h_eff > 0.05`),
(d) the zero-drain share of light-fed mixotrophs (section I), (e) persisted and lockup seeds, and
(f) kin kills per 1000 P agent-ticks.

**Clustering.** Cells within one search descend from a few emitter lineages (#663). The atlas does
not record parentage, so configs are not independent and their p-values overstate. Two guards:

- Each seed is reported apart. A relation is trusted only if it has the same sign, and is not
  negligible, in both seeds and both modes.
- As a lineage proxy, the cells of each atlas are grouped into 8 Ward clusters on their unit
  coordinates (sizes 6–23). Spearman is recomputed on ranks demeaned within cluster. `c_AH` is
  itself clustered: 19 % (seed 42) and 40 % (seed 43) of its variance is between those clusters.

Spearman ρ of `c_AH` against each readout (in brackets, the within-cluster value):

| readout | s42 decoded | s42 fa 0 | s43 decoded | s43 fa 0 | same sign in all four? |
|---|---:|---:|---:|---:|---|
| (a) per-config conditionality ρ | +0.18 (+0.19) | +0.11 (+0.12) | −0.06 (+0.02) | +0.01 (−0.05) | no |
| (b) mixotroph share, second half | −0.13 (−0.10) | −0.18 (−0.11) | +0.03 (+0.06) | +0.02 (+0.09) | no |
| (c) light-fed mixotroph prevalence | −0.02 (+0.03) | +0.03 (+0.09) | −0.01 (−0.13) | +0.06 (−0.00) | no |
| (d) zero-drain share | +0.03 (+0.02) | +0.04 (+0.05) | +0.02 (+0.01) | −0.03 (−0.15) | no |
| (e) persisted seeds | +0.03 (+0.04) | +0.16 (+0.21) | −0.24 (−0.09) | −0.11 (−0.07) | no |
| (e) lockup seeds | +0.03 (+0.01) | −0.02 (−0.02) | +0.11 (+0.08) | +0.15 (+0.14) | no (3 of 4) |
| (f) kin kills / 1000 P agent-ticks | +0.04 (−0.06) | −0.02 (−0.11) | +0.03 (−0.06) | +0.02 (+0.03) | no |

Only one coefficient reaches p < 0.05 unadjusted: seed 43 decoded's persistence, −0.24
(p = 0.02). Its within-cluster value is −0.09 and seed 42 has the opposite sign.

By tercile of `c_AH` (each about 33 configs):

| run | tercile (`c_AH`) | median ρ (configs < 0) | pooled share, second half | median prevalence | median zero-drain | persisted | lockup |
|---|---|---|---:|---:|---:|---:|---:|
| s42 dec | low (0.017–0.064) | −0.131 (23 of 33) | 62.3 % | 30.2 % | 50.0 % | 147 / 165 | 3 |
| | mid (0.066–0.100) | +0.039 (15) | 58.9 % | 35.6 % | 45.5 % | 150 | 10 |
| | high (0.100–0.140) | +0.057 (14) | 64.0 % | 29.1 % | 42.3 % | 152 | 8 |
| s42 fa 0 | low | −0.072 (20) | 62.9 % | 31.1 % | 44.7 % | 142 | 8 |
| | mid | −0.019 (17) | 56.9 % | 31.6 % | 51.2 % | 153 | 9 |
| | high | −0.021 (18) | 54.2 % | 28.2 % | 44.2 % | 150 | 8 |
| s43 dec | low (0–0.059) | −0.126 (21 of 32) | 69.6 % | 27.5 % | 36.3 % | 151 / 160 | 1 |
| | mid (0.059–0.088) | −0.020 (18) | 59.6 % | 36.7 % | 46.8 % | 143 | 6 |
| | high (0.089–0.140) | −0.115 (23) | 59.9 % | 22.4 % | 40.3 % | 137 | 7 |
| s43 fa 0 | low | −0.013 (18) | 67.4 % | 31.2 % | 52.4 % | 148 | 2 |
| | mid | −0.036 (18) | 58.5 % | 40.5 % | 47.8 % | 148 | 4 |
| | high | −0.074 (17) | 62.1 % | 28.3 % | 44.9 % | 137 | 11 |

- **Within an atlas, a world's `c_AH` predicts none of the readouts.** No relation holds in sign
  across both seeds and both modes. If anything, seed 42's worlds are less conditional at higher
  `c_AH` (+0.18 and +0.11), the opposite of the design's expectation. Seed 43 shows nothing.
- **The cost does not selectively thin unused heterotrophy across worlds.** The zero-drain share is
  unrelated to `c_AH` (−0.03 to +0.04). #668 §7 asked for this.
- **Lockup leans weakly upward with `c_AH`** (lowest in the low tercile in three of four runs, and
  ρ +0.11 / +0.15 on seed 43). This is weak and not supported by seed 42 decoded. It is a lead to
  watch, not a finding.
- **Modesty.** These are correlations across 96–99 clustered configs, each with its own 33 other
  parameters. A null here does not show the cost is inert. The atlas-level comparison with #668
  (§4, §6) shows it acts on kin-killing and on the pile. But it does not show conditionality rising
  with `c_AH` within the worlds genesis finds.

## 8. The #637 region

Ungated, bars unchanged (sated ≥ 60 %, kept ≥ 75 %), 23 `k` from 0.048 to 20 × 12 `τ` from 1 to 50.

| run | result, n = 2 / 4 / 8 | closest or default `(k, τ)`, n = 4 | sated (energy side alone) / kept | margin n = 2 / 4 / 8 | kin kills / consumer ticks | kept floor (`k` = 0.048) |
|---|---|---|---|---|---|---:|
| #656, decoded | empty | 2.236, 1 | 45.8 (80.0) / 53.0 % | −0.221 / −0.220 / −0.204 | 18,585 / 5,752 | 37 % |
| #668, decoded | empty | 2.236, 1 | 56.5 (83.5) / 63.1 % | −0.121 / −0.119 / −0.108 | 24,613 / 5,827 | 48 % |
| **s42, decoded** | **empty** | 2.941, 1.427 | 34.6 (66.9) / 48.3 % | **−0.270 / −0.267 / −0.254** | 19,296 / 5,494 | 21 % |
| **s43, decoded** | **non-empty: 53 / 41 / 37 of 276** | **1.293, 17.203 (default)** | 63.5 (98.7) / 78.5 % | **+0.035 / +0.035 / +0.035** | 22,677 / **3,228** | 74 % |
| #656, fa 0 | empty | 2.236, 4.148 | 52.2 (86.4) / 67.5 % | −0.078 / −0.078 / −0.075 | 33,343 / 11,558 | 58 % |
| #668, fa 0 | empty | 2.236, 1 | 57.1 (82.3) / 72.0 % | −0.056 / −0.030 / −0.025 | 46,327 / 12,221 | 55 % |
| **s42, fa 0** | **empty** | 2.941, 1 | 37.4 (69.9) / 54.4 % | **−0.222 / −0.226 / −0.226** | 32,206 / 6,955 | 34 % |
| **s43, fa 0** | **empty** | 1.7, 1 | 54.4 (90.0) / 63.5 % | **−0.127 / −0.115 / −0.063** | 47,995 / 9,764 | 51 % |

- **The closing rule's condition is met on neither atlas.** It asks for a non-empty region both
  decoded and at fa 0. Seed 42 is empty in both modes, further from the bars than #668 (and decoded,
  further than #656). Seed 43 is non-empty decoded and empty at fa 0.
- **Seed 43 decoded is the first non-empty reading since #655's `b = 1` perturbation, and the
  first on an atlas searched under the physics it is read with.** Its default is `k` = 1.293, `τ` = 17.2, the same at n = 2, 4 and 8:
  63.5 % sated (98.7 % on the energy side alone), 78.5 % kept, margin +0.035. Its feasible cells
  form a band at `k` 0.43–1.29 (n = 2) and 0.57–1.29 (n = 4).
- **Its kept bar is thin.** Kept is read over heterotrophs by diet, and seed 43 decoded has the
  fewest of any run: 3,228 consumer ticks, against 5,494–12,221 elsewhere. Kept's denominator is
  their excess production, Σ max(0, x − 1). 55 % of it comes from one seed of one config (atlas:24,
  seed 1003, 159 consumer ticks), 69 % from two seeds, and 83 % from five configs. So the decoded
  region rests on about one world's heterotrophs.
- **The kept floor is a property of which worlds host heterotrophs by diet, and it swings.** At the
  lowest `k`, kept falls to the share of heterotrophs-by-diet's excess production that comes from
  their own light. That is 21 % and 34 % on seed 42, and 74 % and 51 % on seed 43. On seed 43
  decoded the floor sits a point under the bar, so kept clears it from `k` ≈ 0.57 up, while sated
  holds to `k` ≈ 1.29. On seed 42 no `k` comes near.
- **Sated is not the problem.** In every run, low `k` sates 96–99 % of mixotroph kin kills. Kept
  binds, as since #656.

## 9. Verdict for #662

**The design does not deliver flow 1's two tests on genesis's worlds.**

- **Conditionality is not shown as the design means it.** It passes by the rule in all four
  readings, but in three the configs split as a coin would. A world's `c_AH` does not predict its
  ρ, and retention is not conditional on the pool. Seed 43 decoded (62 of 96, median −0.079) is the
  one reading that looks like the pattern, and its fa 0 twin does not.
- **The minority share fails clearly**, by 9–12 points in all four readings, after sitting at the
  bar in #656 and #668. With the cost searched, carcass structure is drained less overall, and
  decomposers lose more of it than mixotrophs do.
- **What the change does deliver:** less kin-killing (1.4–1.5 per 1000 P agent-ticks in three
  readings, against 2.6–2.9 under the deficit rule alone), persistence unchanged, more live cells
  (96–99 against 79–80) and a higher QD-score (21.3–25.2 against 18.5–19.0).

**What flow 1's mapping then indicates, for the owner.**

- The minority share now fails robustly. The mapping says that brings **carcass access out of
  reserve**: an autotrophy-linked cost on carcass digestion.
- Conditionality's lever, the cross-trait term, is in and searched, and does not produce the
  relation within worlds. The design text says a cost alone makes mixotrophy rare, not conditional,
  and that the deficit term supplies the conditional part. #668 §4 and §3 here show the deficit
  does not depend on the pool on these worlds. Without that, the pair cannot make heterotrophy
  conditional. A defensible next step is to ask why a light-fed producer's deficit stays positive
  on rich ground (#668 §4's untested reading: size-scaled uptake keeps a small body's store
  filling slowly whatever the pool), before adding further physics.
- Defensible alternatives: (a) take the strict reading above, open carcass access, and reopen
  conditionality's physics; (b) accept that the carnivorous-plant clause describes an intended
  pattern genesis's worlds do not show, and weaken flow 1's conditionality test to "not running
  backwards" (a design change; #656 §7's reading 3 sets out the cost). This note does not choose.

## 10. Recommendations

**#630/#631 (the closing rule).** **Close #630 and #631 and withdraw the fullness gate**: the
region is not non-empty both decoded and at fa 0 on either atlas, and the rule's bars are not
moved. Kin-killing is then left to recognition and dispersal, and expression is redesigned in its
own pass, as the need gate's status paragraph says. Seed 43 decoded's non-empty region should be
recorded but not used. It is one mode on one seed. Its kept bar rests on about one world's
heterotrophs. The same atlas at fa 0 misses by 0.115, and seed 42 misses by 0.27. If the owner
wants one more read before withdrawing, the cheap one is the region on a third search seed. That
would only matter if it were non-empty in both modes. Nothing here suggests it would be.

**The committed atlas.** Recommend replacing `atlas.json` and `recipe.json` with seed 42's
(`target/670/atlas-seed42.json`, `recipe-seed42.json`), in a dedicated slice like #663, **after
the owner's #662 verdict**. The case:

- The committed atlas was searched before the deficit rule and has no `c_AH` coordinate, so it
  decodes at `c_AH = 0`. Every readout on it now measures a perturbation of physics it was not
  searched under, as #656 §9 argued for its predecessor.
- Seed 42 is bit-reproducible (no budget hit) and records its 34-dimension box. Seed 43 is not
  reproducible (7 unfinished rollouts), so it is not a candidate.
- The two seeds agree on what matters for a baseline: cell count (99, 96), dead frontier led by
  lockup, persistence, and the readouts' verdicts. They disagree only on the region, where neither
  is a candidate default.
- **The cost of re-pinning:** `atlas_search_box` (fingerprint, a 34-dimension box), `evaluator_pin`
  (cells re-chosen), `guild_anchor` (new recipe), and checks that read `atlas.json` or
  `recipe.json` (`prefilter_atlas`, `energy_death_check`, `settling_time`, `radius_sweep`, the app's
  recipe tests). The new recipe would set `cross_trait_cost` = 0.139 explicitly, while the default
  stays 0. The work is #663's again, about one slice.
- **Why wait:** if the owner brings carcass access out of reserve, or otherwise changes the physics,
  the atlas must be searched again. Re-pinning now would be done twice. If the verdict keeps the
  physics as it is, replace then.

## 11. What this does not show

- **Two search seeds, 11 generations each.** The region's split between seeds shows how much one
  draw can mislead. Seed 43's 7 unfinished rollouts mean its atlas is one machine's draw.
- **Correlational and ungated,** as in #656 and #668. G ranks `h_eff` against the pool. It does not
  add nutrient and watch heterotrophy respond.
- **The comparisons are across atlases.** #656 and #668 read 79 worlds searched without the cost.
  Seeds 42 and 43 here are other worlds. The share's rise, the pile's fall and kin-killing's fall are
  rates on different worlds, not paired effects of the cost.
- **The cost's charge by role is not read.** §4's possible reading is untested.
- **The lineage proxy is a proxy.** Ward clusters on unit coordinates approximate emitter lineages.
  The atlas does not record parentage.

## 12. Reproduction

From the repo root, on `7fc7fb5` (or any tree with #666, #667 and #669, and #668's `kin_killer_diet`
sections I and J), with the binaries built into `target/670/bin`
(`explorers-search`, `kin_killer_diet`, `role_diet_census`):

```sh
target/670/run.sh
```

The driver is resumable. For each seed in 42, 43 it searches
(`explorers-search --batch 32 --generations 10 --ensemble 5 --max-ticks 2000 --seed <S>
--output target/670/atlas-seed<S>.json --recipe-output target/670/recipe-seed<S>.json
--checkpoint target/670/seed<S>.ckpt`). It then writes `seed<S>/cah-distribution.md` and runs
`target/670/readouts.sh` with `ATLAS=target/670/atlas-seed<S>.json K=target/670/seed<S>`. Outputs:
`seed<S>/kkd-{decoded,fa0}.{md,jsonl}`, `seed<S>/rd-{decoded,fa0}.jsonl`,
`seed<S>/rd-{decoded,fa0}-region.md`, with timings in `times.log`. Seed 42 reproduces bit for
bit. Seed 43 reproduces only if no rollout reaches the 600 s budget.

§7's per-config analysis reads the rows' `cross_trait_cost`, `tally.ext.het_pool` (ρ),
`tally.ext.routes_second_half.carcass_drained` (buckets: light-fed mixotroph, pure producer,
consumer, decomposer, no income, memo), `tally.ext.producer_bins` (prevalence) and
`tally.ext.lfm_calibration` (zero-drain share), plus E's per-config table. §8's concentration
reads `seeds[].fullness.{consumer_ticks, produced}` in `rd-*.jsonl`.

## References

- [668-cross-trait-calibration.md](668-cross-trait-calibration.md): `c_AH`'s range and the deficit
  rule alone on the committed atlas.
- [656-fresh-atlas-verdict.md](656-fresh-atlas-verdict.md): the same readouts before the deficit
  rule, and #647's verdict.
- [663-atlas-regen-retention.md](663-atlas-regen-retention.md): the committed atlas, seed 43's
  comparison and ancestry clustering.
- [655-retention-readouts.md](655-retention-readouts.md): the instrument and the pass rules.
- [637-fullness-region.md](637-fullness-region.md): the `(k, τ)` region, its bars and grid.
- [World rules](../system-design/world-rules.md): flow 1 (the two tests), flow 3 (the deficit
  rule), the need gate's closing rule, trade-off #5 (the cross-trait term).
