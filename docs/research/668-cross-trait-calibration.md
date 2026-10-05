# Issue #668: the top of `c_AH`'s range, and the deficit rule on the committed atlas

**Status: measurement, docs only (2026-10-05). Measures the top of the search range for the
cross-trait cost `c_AH` (trade-off #5), as the design asks. Gives a provisional read of the flow-3
deficit rule (#666) on its own, on the committed atlas, against #656 on the same worlds. Probes
`c_AH` at the recommended top. The instrument commits (`a2ee9d0`, `cd68a4b`) add sections I and J
and a `--cross-trait-cost` flag to `kin_killer_diet`. `explorers-sim`, the search and the evaluator
are unchanged. No design change. #662's deciding read is #670; this one is provisional.**

The design ([world rules](../system-design/world-rules.md), trade-off #5) sets the range's top as
the `c_AH` at which a typical light-fed mixotroph's cross-trait cost, `c_AH × (A·H)^(p/2)`, is
about twice its median drain income. It reads that on the committed atlas, under the deficit rule,
at `c_AH = 0`. A **light-fed mixotroph** is a Producer by recent-income role with effective
heterotrophy `h_eff > 0.05` (#655).

## TL;DR

**Recommended `c_max` = 0.14.** Typical is defined over light-fed mixotrophs that drain at all, as
the median of per-agent ratios `2 × income_i / (A·H)_i^(p/2)`. One agent is one mixotroph lifetime
in one seed, over the second-half ticks it spent as a light-fed mixotroph. That reading is 0.106
decoded and 0.137 at fa 0. 0.14 is the larger, rounded up. The alternatives:

| reading of "typical" | decoded | fa 0 |
|---|---:|---:|
| all light-fed mixotrophs, ratio of medians `2 × median(income) / median(term)` | 0 | 0.0033 |
| all light-fed mixotrophs, median of per-agent ratios | 0 | 0.0024 |
| drainers only (income > 0), ratio of medians | 0.102 | 0.124 |
| **drainers only, median of per-agent ratios** | **0.106** | **0.137** |
| per-config median of the drainers' per-agent ratio: p25 / median / p75 over configs | 0.014 / 0.118 / 0.203 | 0.031 / 0.102 / 0.261 |

The all-agent readings collapse to zero because about half of light-fed mixotrophs never drain in
the second half: 52.1 % decoded, 47.2 % at fa 0. A range top of 0 is not a range. §2 argues the
choice.

**The deficit rule alone, against #656 on the same worlds** (fa 0 = `founder_aggregation = 0`; 79
configs × 5 seeds):

| mode | conditionality: pooled ρ / median per-config ρ (configs ρ < 0) | mixotroph share of carcass structure, whole run / second half | persisted | lockup | `(k, τ)` region, closest margin (n = 4) |
|---|---|---|---|---|---|
| #656, decoded | PASS −0.134 / −0.060 (44) | FAIL 51.4 % / 45.8 % | 355 | 17 | empty, −0.220 |
| **#668, decoded** | **PASS −0.144 / −0.017 (42)** | **FAIL 52.8 % / 45.4 %** | **350** | **13** | **empty, −0.119** |
| #656, fa 0 | FAIL −0.048 / +0.051 (33) | FAIL 55.6 % / 48.4 % | 363 | 18 | empty, −0.078 |
| **#668, fa 0** | **FAIL −0.040 / +0.015 (36)** | **FAIL 58.9 % / 52.8 %** | **368** | **15** | **empty, −0.030** |
| probe `c_AH = 0.14`, decoded | PASS −0.168 / −0.048 (47) | FAIL 56.5 % / 49.9 % | 361 | 14 | not read |
| probe `c_AH = 0.14`, fa 0 | FAIL −0.009 / +0.085 (31) | FAIL 58.7 % / 51.7 % | 363 | 18 | not read |

1. **The deficit rule does what it says to retention** (§4). Light-fed mixotrophs keep 8.1×
   (decoded) and 6.8× (fa 0) as much carcass nutrient in the second half as under #656's rule.
   Per unit of carcass structure drained, they keep 0.97 and 1.01 against 0.14 and 0.18. Carcasses
   now supply 4.3 % and 4.2 % of their nutrient, against 0.6 % and 0.8 %.
2. **But retention does not depend on the pool under the drainer** (§4). Per bite, ρ(retained /
   drained, pool N) is +0.016 pooled and +0.023 median per config decoded, and −0.034 and +0.065
   at fa 0. The rule was meant to make a drain's nutrient worth more on poor ground. On these
   worlds it makes drains worth more everywhere.
3. **Conditionality is still not shown** (§3). Under the deficit rule alone the median per-config
   ρ is −0.017 decoded and +0.015 at fa 0. 42 and 36 of 79 configs are negative, a coin's split
   (two-sided binomial p = 0.65 and 0.50). The decoded pass is the pooled ρ ranking worlds
   against each other, as in #656.
4. **The minority share moves the wrong way at fa 0** (§3). On the settled half, where flow 1
   reads it, it is 45.4 % decoded (passes) and 52.8 % at fa 0 (fails, up from 48.4 %). Mixotrophy
   and kin-killing rise. Producers with `h_eff > 0.05` go from 24.3 % to 26.4 % of Producer
   samples decoded and from 25.8 % to 29.9 % at fa 0. Mixotroph kin kills per 1000 Producer
   agent-ticks rise from 1.84 to 2.87 and from 1.65 to 2.57 in the second half.
5. **Persistence holds.** 88.6 % and 93.2 % of seeds persist, against 89.9 % and 91.9 %. Lockup is
   13 and 15 seeds, against 17 and 18.
6. **The fullness region is still empty, but closer** (§5). The closest margin at n = 4 is −0.119
   decoded and −0.030 at fa 0, against −0.220 and −0.078. Kept still binds decoded. At fa 0 both
   bars are within 3 points at `k = 2.236`, `τ = 1`.
7. **The probe shows the term bites** (§6). At `c_AH = 0.14` light-fed mixotroph lifetimes fall by
   34 % decoded and 12 % at fa 0. Kin kills per 1000 Producer agent-ticks fall back to 1.93 and
   1.78 in the second half. Persistence holds. Conditionality moves the right way decoded (median
   ρ −0.048, 47 of 79 negative, p = 0.12) and the wrong way at fa 0 (+0.085, 31 of 79, p = 0.07).
   The minority share does not improve. One value of `c_AH` on every world is a perturbation, not
   the searched result; #670 decides.

## 1. What ran

| | |
|---|---|
| Atlas | the committed `atlas.json` (79 cells, searched in #663), byte-identical to `target/656/atlas.json`. Fingerprint `9c79856550a0151e` on every row |
| Tree | `cd68a4b` (branch `issue-668-cross-trait-calibration`): main `0c36082` (#666's deficit rule, #667's `cross_trait_cost`, default 0) plus the instrument commits `a2ee9d0` and `cd68a4b`. Release build, binaries copied to `target/668/bin` |
| Diagnostics | `kin_killer_diet` and `role_diet_census --fullness` (ungated, `--satiation-sensitivity 0`) on all 79 cells, seeds 1000–1004 (395 seeds), T = 2000, `EvalConfig::default()`, network off. Decoded and at `--founder-aggregation 0`. Each config decodes with its own `b` and `s_ref`. These are #656's settings |
| Region | `role_diet_census --summary --fullness-region` on each run's rows. #637's bars and `τ` grid, #656's extended `k` grid (23 × 12 cells per n) |
| Probe | `kin_killer_diet --cross-trait-cost 0.14`, decoded and fa 0. The region was not re-read at the probe |
| Baseline | #656's readouts on the same worlds, under ratio retention (before #666): `target/656/kkd-*.md`, `rd-*-region.md` |
| Machine | 8-core laptop, one process at a time |

**Commands** (cwd is the repo root; the driver is `target/668/readouts.sh`):

```sh
C=$(seq -s, -f 'atlas:%g' 0 78)
FA0="--founder-aggregation 0"
target/668/bin/kin_killer_diet --atlas atlas.json --configs $C [$FA0] \
    --out target/668/kkd-{decoded,fa0}.jsonl > target/668/kkd-{decoded,fa0}.md
target/668/bin/role_diet_census --atlas atlas.json --configs $C [$FA0] \
    --satiation-sensitivity 0 --fullness --out target/668/rd-{decoded,fa0}.jsonl
target/668/bin/role_diet_census --summary --fullness-region \
    --out target/668/rd-{decoded,fa0}.jsonl > target/668/rd-{decoded,fa0}-region.md
# probe
target/668/bin/kin_killer_diet --atlas atlas.json --configs $C [$FA0] --cross-trait-cost 0.14 \
    --out target/668/kkd-{decoded,fa0}-cmax.jsonl > target/668/kkd-{decoded,fa0}-cmax.md
```

**Wall clock** (`target/668/times.log`). Build 17 s. `kin_killer_diet` 239 s decoded and 204 s
fa 0. `role_diet_census --fullness` 225 s and 188 s. Region reads under 1 s. Probe 229 s decoded
and 173 s fa 0. In total 1258 s. The four readouts #656 also ran took 856 s here against 557 s
there. The worlds are busier under the deficit rule, with more drains and kin kills (§3), and the
instrument does more per row (sections I and J).

**Checks.**

- `role_diet_census`'s kin-kill counts match `kin_killer_diet`'s: 24,613 decoded and 46,327 at
  fa 0.
- Every carcass bite is attributed (H's unattributed structure is 0.0 in all four runs).
- B's Producer sample count equals A's in all four runs.

## 2. Calibration (section I)

**The readout.** The unit is one light-fed mixotroph per seed, an agent lifetime, over the
second-half ticks it spent as one. **Income** is its mean drain energy gained per such tick,
living and carcass, from `realised_bites`. A tick without a drain counts as 0. **Term** is
`(A·H)^(p/2)` on its raw autotrophy and heterotrophy, with its config's
`maintenance_cost_exponent` `p`. This is the expression `phase::metabolic_cost` multiplies by
`c_AH`. `c_max` is the `c_AH` at which `c_AH × term = 2 × income`, read two ways: (a) the ratio of
medians, `2 × median(income) / median(term)`; (b) the median of per-agent ratios,
`median_i(2 × income_i / term_i)`.

| | decoded | fa 0 |
|---|---:|---:|
| light-fed mixotrophs (agent lifetimes) | 11,739 | 12,916 |
| with zero drain income | 6,120 (52.1 %) | 6,091 (47.2 %) |
| income per tick, p25 / median / p75, all | 0 / 0 / 0.0093 | 0 / 0.0003 / 0.0140 |
| term, p25 / median / p75, all | 0.097 / 0.229 / 0.519 | 0.073 / 0.206 / 0.517 |
| `c_max` (a) / (b), all | 0 / 0 | 0.0033 / 0.0024 |
| drainers (income > 0) | 5,619 | 6,825 |
| income per tick, p25 / median / p75, drainers | 0.0028 / 0.0103 / 0.0301 | 0.0032 / 0.0124 / 0.0380 |
| term median, drainers | 0.200 | 0.200 |
| `c_max` (a) / (b), drainers | 0.102 / 0.106 | 0.124 / 0.137 |

**Per config** (drainers only; 77 configs decoded and 79 at fa 0 have a drainer):

| reading | min | p10 | p25 | median | p75 | p90 | max |
|---|---:|---:|---:|---:|---:|---:|---:|
| (a), decoded | 0.0003 | 0.004 | 0.018 | 0.101 | 0.195 | 0.547 | 4.09 |
| (b), decoded | 0.0003 | 0.004 | 0.014 | 0.118 | 0.203 | 0.600 | 4.40 |
| (a), fa 0 | 0.0008 | 0.013 | 0.027 | 0.083 | 0.234 | 0.482 | 1.49 |
| (b), fa 0 | 0.0008 | 0.011 | 0.031 | 0.102 | 0.261 | 0.444 | 1.30 |

Over all agents, 33–34 of 79 configs have a per-config `c_max` (a) of exactly 0.

**Why drainers, not all agents.** The design's sentence assumes a typical light-fed mixotroph has
a drain income to set the cost against. On this atlas the median one does not: more than half never
drain in the second half, so the median income is 0 or close to it, and any `c_AH > 0` exceeds twice
it. Read literally, the rule gives a top of 0, which is no range at all. The drainers are the agents
whose heterotrophy is in use, and the rule's intent is a cost that heterotrophy's own return can
pay where it is used. Reading the scale on them keeps that intent.

**What the zero-drain half means.** They pay the cost with no drain income to set against it. Under
any `c_AH > 0` their heterotrophy is a pure tax. That is the point of a cost on co-investment:
heterotrophy that is carried but not used should be selected away, leaving it where drains pay.
They are not a reason to lower the top. They are the population the term is meant to thin.

**Why (b) at fa 0, rounded up to 0.14.** The four drainer readings fall in a narrow band,
0.102–0.137. The term's median is the same in both modes (0.200), so the band is the income's
spread. (b) is the more typical of the two definitions: it pairs each agent's cost with its own
income, while (a) pairs the median income with the median term, which may come from different
agents. Rounding the largest reading up to 0.14 puts every pooled reading inside the range. On a
linear range `[0, 0.14]` the box's centre, where CMA-MAE starts (#656 §2), is 0.07. There a typical
drainer's cost is about its own drain income, the break-even point. So the search starts at
break-even and can reach twice it.

**The alternative.** A top at the per-config p75 (0.20–0.26) would reach the 15–17 configs per
mode whose own drainers' `c_max` exceeds 0.28. It would also put most of the range above where the
typical world's cost is twice its drainers' income. The design's rule is about the typical
mixotroph, not the extremes, so 0.14 is recommended. If #670 finds `c_AH` piling at the top, the
top is the first thing to widen. About 30 of 77–79 configs per mode have a drainer `c_max` above
0.14.

**It is stable under the cost.** At the probe (`c_AH = 0.14`) the drainers' readings are 0.153 /
0.147 decoded and 0.100 / 0.113 at fa 0, the same band.

## 3. The deficit rule alone against #656

**Conditionality (G).** Producer `h_eff` against pool N at the cell, second half:

| setting | pooled ρ | n | median per-config ρ | configs ρ < 0 (binomial p) |
|---|---:|---:|---:|---:|
| #656, decoded | −0.134 | 405,024 | −0.060 | 44 of 79 (0.37) |
| #668, decoded | −0.144 | 361,451 | −0.017 | 42 of 79 (0.65) |
| #656, fa 0 | −0.048 | 703,608 | +0.051 | 33 of 79 (0.18) |
| #668, fa 0 | −0.040 | 656,200 | +0.015 | 36 of 79 (0.50) |

Mean pool N at the cell by `h_eff` bin:

| `h_eff` bin | #656 dec | #668 dec | #656 fa 0 | #668 fa 0 |
|---|---:|---:|---:|---:|
| [0, 0.05) | 703 | 653 | 512 | 518 |
| [0.05, 0.2) | 593 | 428 | 434 | 468 |
| [0.2, 0.5) | 452 | 511 | 426 | 425 |
| [0.5, 1) | 544 | 530 | 473 | 425 |
| ≥ 1 | 532 | 512 | 467 | 453 |

The deficit rule alone does not bring conditionality. The medians move towards zero in both modes,
and the split of configs stays a coin's. Pure producers still sit on the richest ground decoded,
but above `h_eff` 0.05 the pool is flat across bins. This is what the design predicts for the
deficit term without the cost (trade-off #5: "a cost alone would thin mixotrophy everywhere
alike"; the deficit alone makes drains pay everywhere alike, §4).

**Minority share (H).** Light-fed mixotrophs' share of carcass structure drained:

| setting | whole run | second half |
|---|---:|---:|
| #656, decoded | 51.4 % | 45.8 % |
| #668, decoded | 52.8 % | 45.4 % |
| #656, fa 0 | 55.6 % | 48.4 % |
| #668, fa 0 | 58.9 % | 52.8 % |

Flow 1 reads the share on the settled half. On that reading it passes decoded and now fails at
fa 0, by 2.8 points. The instrument's rule asks for both halves and fails in both modes. Decomposers
drain 34.6 % and 32.4 % of structure in the second half, against 38.0 % and 35.1 %. With drains
worth more nutrient, light-fed mixotrophs take a little more of the pile at fa 0.

**Prevalence and kin-killing (A, kin kills).**

| | #656 dec | #668 dec | #656 fa 0 | #668 fa 0 |
|---|---:|---:|---:|---:|
| Producer samples with `h_eff > 0.05` | 24.3 % | 26.4 % | 25.8 % | 29.9 % |
| mean Producer `h_eff` | 0.120 | 0.138 | 0.118 | 0.149 |
| mixotroph kin kills per 1000 P agent-ticks, second half | 1.84 | 2.87 | 1.65 | 2.57 |

Heterotrophy pays producers more under the deficit rule, so they carry more of it and kill more
kin. The cross-trait cost is meant to work against exactly this.

**Persistence and lockup (E).** Decoded: 350 of 395 persist (88.6 %), against 355. Lockup 13,
extinction 22, monoculture 7, generalist dominance 3. At fa 0: 368 (93.2 %), against 363. Lockup
15, extinction 5, monoculture 3, generalist dominance 3, energy death 1. Lockup falls by 4 and 3
seeds, which is the direction a rule that keeps more carcass nutrient in living bodies might be
expected to push, but it is within seed noise. Persistence does not regress.

## 4. Carcass nutrient retained (sections C and J)

Second half, light-fed mixotrophs' carcass bites. The #656 figures are its section C on disk
(`target/656/kkd-*.md`). #656's markdown predates section J, so its per-bite ρ was never computed
and is not reported here.

| | #656 dec | #668 dec | #656 fa 0 | #668 fa 0 |
|---|---:|---:|---:|---:|
| carcass structure drained | 17,461 | 20,129 | 29,268 | 35,292 |
| carcass N retained | 2,408 | 19,570 | 5,254 | 35,541 |
| retained per unit drained | 0.138 | 0.972 | 0.180 | 1.007 |
| % of the N their bites release that they keep | 1.9 % | 14.8 % | 2.5 % | 12.3 % |
| carcass share of their N acquired | 0.6 % | 4.3 % | 0.8 % | 4.2 % |
| their share of all retained carcass N | 50.0 % | 79.9 % | 60.5 % | 80.7 % |

- **How much more.** 8.1× decoded and 6.8× at fa 0 in total. Per unit drained it is 7.0× and
  5.6×. Pool uptake still supplies about 95 % of their nutrient. Decomposers keep more too, but
  less so: 1.4× decoded (1,624 to 2,313) and 1.9× at fa 0 (2,358 to 4,506). Light-fed mixotrophs
  now hold four-fifths of the carcass nutrient any role retains.
- **Spent-carcass bites.** Some bites drain no structure and take only a spent carcass's
  remaining nutrient: 2,926 bites retaining 1,758 decoded (9.0 % of the total) and 11,264 retaining
  1,718 at fa 0 (4.8 %). They carry no ratio. Without them, retained per unit drained is 0.885
  and 0.958.
- **It does not depend on the pool under the drainer.** Per bite, ρ(retained / drained, pool N in
  the drainer's cell before the step):

  | | pooled ρ (n) | median per-config ρ (configs, ρ < 0) |
  |---|---|---|
  | decoded | +0.016 (65,358) | +0.023 (70, 33) |
  | fa 0 | −0.034 (118,421) | +0.065 (78, 33) |

  The flow-3 design expects a negative relation. A drainer on rich ground should have its store
  filled by uptake, its deficit at zero, and keep only `ratio × energy gained`. None shows. Per
  config, the retained-per-drained ratio runs from 0.004 to 34. Which world a drainer is in
  matters far more than the pool under it.
- **A possible reading, not tested here.** The deficit compares the nutrient a producer's surplus
  could bind with its free store. A light-fed producer's surplus is large wherever it is lit, so
  its deficit may stay positive even on rich ground. Under size-scaled uptake (`b` ≈ 0.6) a small
  body's store fills slowly whatever the pool. That 40 % of pure producers are nutrient-limited at a
  mean pool of 653 decoded fits this. If it holds, the pool is not what sets the deficit on these
  worlds, and the deficit rule cannot supply conditionality alone. The cost has to.

## 5. The #637 region

Ungated, bars unchanged (sated ≥ 60 %, kept ≥ 75 %), 23 `k` values from 0.048 to 20, 12 `τ`
values from 1 to 50.

| setting | result | closest `(k, τ)`, n = 4 | sated (energy side alone) / kept | margin n = 2 / 4 / 8 | kin kills / consumer ticks |
|---|---|---|---|---|---|
| #656, decoded | empty | 2.236, 1 | 45.8 % (80.0 %) / 53.0 % | −0.221 / −0.220 / −0.204 | 18,585 / 5,752 |
| **#668, decoded** | **empty** | 2.236, 1 | 56.5 % (83.5 %) / 63.1 % | **−0.121 / −0.119 / −0.108** | 24,613 / 5,827 |
| #656, fa 0 | empty | 2.236, 4.148 | 52.2 % (86.4 %) / 67.5 % | −0.078 / −0.078 / −0.075 | 33,343 / 11,558 |
| **#668, fa 0** | **empty** | 2.236, 1 | 57.1 % (82.3 %) / 72.0 % | **−0.056 / −0.030 / −0.025** | 46,327 / 12,221 |

(n = 2 decoded's closest point is `k` 2.941, `τ` 2.037; n = 8 fa 0's is `k` 2.236, `τ` 1.427.)

- **Closer in both modes, still empty.** Decoded the margin halves. At fa 0 both bars are within
  3 points at one grid cell, which is closer than any run on genesis's worlds before it.
- **Kept still binds decoded.** Its floor at low `k` is 47–48 % (37 % under #656). Kept reaches
  75 % only at `k ≥ 3.9` with `τ = 1`, where sated is 32 %. At fa 0 the floor is 55 %, and kept
  reaches 75 % at `k = 2.941` (sated 43–48 %).
- **Why closer.** Both bars moved. Decoded, the kept floor rose by 10 points: the floor is the
  share of heterotrophs by diet's excess production that comes from their own light, so they now
  lean less on their drains. At the closest cell, sated rose by 11 points decoded and 5 at fa 0.
  This note does not separate the causes.
- **The region was not read at the probe.** #670 reads it on the atlas searched with `c_AH` in the
  box, as the closing rule asks.

## 6. The probe at `c_AH = 0.14`

Every config at one `c_AH`, on worlds searched without the term. This is a perturbation, not what
genesis would find. It shows the term bites at the recommended top, not what it selects for.

| | decoded, 0 | decoded, 0.14 | fa 0, 0 | fa 0, 0.14 |
|---|---:|---:|---:|---:|
| light-fed mixotroph lifetimes (I) | 11,739 | 7,734 (−34 %) | 12,916 | 11,317 (−12 %) |
| of them with zero drain income | 52.1 % | 40.8 % | 47.2 % | 53.9 % |
| light-fed mixotroph share of agent-samples (A) | 25.5 % | 20.6 % | 29.2 % | 26.0 % |
| Producer `h_eff`: mean / % > 0.5 | 0.138 / 10.1 % | 0.109 / 8.0 % | 0.149 / 10.8 % | 0.139 / 10.1 % |
| median term `(A·H)^(p/2)`, all | 0.229 | 0.152 | 0.206 | 0.221 |
| mixotroph kin kills per 1000 P agent-ticks, second half | 2.87 | 1.93 | 2.57 | 1.78 |
| conditionality: pooled / median ρ (configs < 0) | −0.144 / −0.017 (42) | −0.168 / −0.048 (47) | −0.040 / +0.015 (36) | −0.009 / +0.085 (31) |
| minority share, whole / second half | 52.8 / 45.4 % | 56.5 / 49.9 % | 58.9 / 52.8 % | 58.7 / 51.7 % |
| carcass N retained per unit drained (J) | 0.972 | 1.041 | 1.007 | 1.008 |
| J: ρ(retained / drained, pool N), pooled / median | +0.016 / +0.023 | +0.091 / +0.048 | −0.034 / +0.065 | −0.127 / +0.121 |
| persisted / lockup | 350 / 13 | 361 / 14 | 368 / 15 | 363 / 18 |

- **It bites.** Mixotroph lifetimes, prevalence, mean `h_eff`, the median carried term and
  kin-killing all fall decoded. At fa 0 they fall too, less, though the median term does not.
  Kin-killing returns to about #656's level (1.84 and 1.65).
- **It does not kill worlds.** Persistence rises decoded (361) and is unchanged at fa 0 (363).
  Lockup is 14 and 18, within the range of the runs above.
- **Conditionality splits by mode.** Decoded it moves the right way: median −0.048, 47 of 79
  configs negative (p = 0.12). At fa 0 it moves the wrong way: median +0.085, 31 of 79 (p = 0.07).
  Neither is a clear relation. A uniform cost on worlds not searched under it thins mixotrophy, as
  the design warns a cost alone would. Whether the search finds worlds where the two act together
  is #670's question.
- **Decoded, the zero-drain share falls** (52 % to 41 %). The cost removes the non-drainers first,
  which is the selection §2 describes. At fa 0 it rises (47 % to 54 %), which this note does not
  explain.
- **The minority share does not improve.** Decoded it rises to 49.9 % in the second half. The pile
  shrinks (31,784 drained against 44,294), and decomposers' and no-income agents' drains fall more
  than the mixotrophs'.

## 7. For #669 and #670

- **#669:** put `cross_trait_cost` in the box, linear over **[0, 0.14]**. The default stays 0, and
  the committed atlas decodes with `c_AH = 0`.
- **#670** should read, beyond its listed readouts:
  - J's ρ(retained / drained, pool N), to see whether searched worlds make retention conditional.
    The deficit rule alone does not (§4).
  - The zero-drain share of light-fed mixotrophs against each cell's `c_AH`: the cost should thin
    carried, unused heterotrophy.
  - Whether `c_AH` piles at 0.14. If it does, widen the top (§2's alternative, about 0.25) before
    reading the verdict.
  - The fa 0 minority share. It now fails on the settled half without the cost (52.8 %), so
    carcass access may come out of reserve (flow 1) unless the searched worlds bring it back.
- **The fullness-gate region** is closer under the deficit rule, at fa 0 by 3 points. #670's read
  under the closing rule is no longer a foregone conclusion.

## 8. What this does not show

- **It is provisional.** The deficit rule is measured on worlds searched without it. #670 searches
  with both changes.
- **The calibration reads `c_AH = 0` worlds.** Under the cost the population changes (§6), and the
  drainers' readings stay in the same band, but the top is a scale, not an optimum.
- **One draw of `p` per config.** The term uses each config's own `p` (1.5–3), so `c_max` mixes
  worlds where the term has different curvature. Per-config spread (§2) is wide for this reason
  among others.
- **Correlational, ungated, five seeds per config.** As in #656 §10. The minority share's 2.8-point
  miss at fa 0 and the persistence differences are within the range seed noise has moved similar
  readouts before.
- **#656's per-bite retention ρ is not on disk.** §4 compares totals only.

## 9. Reproduction

From the repo root, on `cd68a4b` (or any tree whose `kin_killer_diet` has sections I and J and
`--cross-trait-cost`), with `atlas.json` at fingerprint `9c79856550a0151e`:

```sh
target/668/readouts.sh                  # c_AH = 0: kkd and rd, decoded and fa 0, regions
env CMAX=0.14 target/668/readouts.sh    # the probe; finished steps are skipped
```

The driver is resumable and builds once into `target/668/bin`. Outputs go to `target/668/`:
`kkd-{decoded,fa0}{,-cmax}.{md,jsonl}`, `rd-{decoded,fa0}.jsonl`, `rd-{decoded,fa0}-region.md`,
with timings in `times.log`.

## References

- [656-fresh-atlas-verdict.md](656-fresh-atlas-verdict.md): the same readouts on the same worlds
  before the deficit rule.
- [663-atlas-regen-retention.md](663-atlas-regen-retention.md): the committed atlas.
- [655-retention-readouts.md](655-retention-readouts.md): the instrument and the pass rules.
- [637-fullness-region.md](637-fullness-region.md): the `(k, τ)` region, its bars and grid.
- [World rules](../system-design/world-rules.md): flow 1 (the two tests), flow 3 (the deficit
  rule), the need gate's closing rule, trade-off #5 (the cross-trait term and its range).
