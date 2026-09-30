# Issue #605: need-gated consumption on `sample:31` and the role-diet census

**Status: measurement. Adds `--founder-aggregation A` to the `reinvasion_barrier` and
`role_diet_census` bins. No trajectory changes when the flag is unset. `explorers-sim`, the search,
the evaluator and the prefilter are unchanged.**

#600 made consumption need-gated: heterotrophy is the most an agent can drain per tick. It drains
`capability / (1 + c·s)`, where `s` is its satiation, the lesser of its reserve and its
nutrient-matched reserve, in ticks of its own maintenance. #603 co-limited `s` with nutrient. The
design rationale (world rules, *Capability and expression are decoupled*) predicts one effect in
particular. A producer that has drifted toward heterotrophy, fed on light, should stop grazing the
newborns beside it, so the kin-grazing cliff #591/#593 found on `sample:31` becomes a slope. This
note measures that on the two instruments that found the cliff, before and after the change.

## TL;DR

1. **Infant kin-grazing did not fall at the resident's own dispersal. It rose.** On `sample:31`,
   infant grazed deaths per birth rose on every arm:

   | arm | before | after |
   |---|---:|---:|
   | `step 0` | 0.07 | 0.17 |
   | `step 1×` | 0.82 | 0.92 |
   | `step 2×` | 0.86 | 0.96 |

   Kin still take essentially all the energy grazed off a step lineage: 0.0094 of 0.0094 E per
   member-tick for `step 1×`, against 0.0071 of 0.0073 before. Infant grazing falls only where
   dispersal already moved the newborns out of kin reach. At dispersal 2 and 4, `step 1×` is 0.23
   and 0.16 (before: 0.35 and 0.25).
2. **The step does not close on the producer.** The paired `Δr` against `step 0` at own dispersal:
   - `step 1×` goes from −0.00082 to **−0.00194**. It loses on every seed, and its lineage ends at
     a median of 1 (max 2).
   - `step 2×` goes from −0.00132 to −0.00101, a gain inside the tie-dominated noise.

   At dispersal 4, #593's one positive result shrinks. `step 2×` beats `step 0` by +0.00018,
   against +0.00062 before. `step 1×` falls from −0.00012 to −0.00118. Nothing invades by the #587
   sign test on either side.
3. **The atlas and the LHS draw agree: kin grazing of producers went slightly up, not down.** On the
   configs that are live on both sides:

   | | kin-grazed share of trait-producer deaths, before | after |
   |---|---:|---:|
   | atlas (86 configs) | 23.1 % | 26.0 % |
   | LHS draw (138 configs) | 19.5 % | 21.0 % |

   Infant producer deaths grazed by kin rose by 24 % (atlas) and 12 % (LHS). Heterotrophs by diet
   are grazed more and more by kin on both sources.
4. **Mixotrophs by investment still live on light.** On live configs, 97.5 % of trait-tagged
   heterotroph agent-samples take ≥ 90 % of lifetime income from light on both sources (before:
   97.2 % atlas, 96.7 % LHS).
   - By diet, no config holds a consumer or decomposer guild on either side.
   - Per seed, the diet read finds one heterotroph guild in 1,475 seeds before (a consumer guild on
     `sample:165`/1001, a monoculture) and one after (a decomposer guild on `sample:45`/1001,
     generalist dominance).
5. **What moved that wasn't expected:**
   - **The resident itself changed, so the after-side invader meets a different resident.** At
     t_inj it has 104 agents against 98, a max heterotrophy of 0.733 against 0.655, and a pool of
     7,092 N against 6,487 N. At the window end the control holds 65 agents against 45, and its
     rate is −0.00032 against −0.00058.
   - **The steps are richer and still lose.** `step 2×` income per member-tick doubles
     (0.89 → 1.83 E), almost all of it photosynthesis. Its births triple (111 → 353), and 338 of its
     339 infant deaths are grazed.
   - **Atlas lockup rises.** Lockup verdicts go from 40 to 52 of 475 seeds, and generalist
     dominance appears (0 → 3).
   - **The LHS live-config death table falls 3×, but only by composition.** Trait-producer deaths
     go 411,350 → 128,085 and the grazed share 72.6 % → 27.0 %. `sample:197` (298k producer deaths,
     90 % grazed) left the live set, becoming monoculture on 5/5 seeds. On matched configs deaths
     rose 4 %.
   - **Sweep cost moved by config.** For example, `sample:38` took 869 s → 164 s and `sample:165`
     737 s → 1,122 s (§5).
6. **Reading.** The need gate as tuned (`satiation_sensitivity = 0.1`) does not make a near-producer
   mixotroph's heterotrophy latent toward its newborns on these instruments. The cliff #591/#593
   measured is still a cliff. Why is not measured here: these instruments do not read satiation or
   expressed drain. The natural next step is an in-tree control at `satiation_sensitivity = 0`, the
   flat limit the world rules keep for comparison (§7).

## 1. What ran

| | |
|---|---|
| Before | `f6a1c1f` (main after #599's income-role read, before #600). Unmodified worktree. |
| After | `c095f04` (main after #600 need-gating and #603 nutrient co-limitation, before #604 recognition) plus `529ed16` (this issue's `--founder-aggregation` flag). #601 founder aggregation and #602 (trophic balance retired from fitness) also lie between the two. |
| Founder aggregation | Pinned to 0 on the after side (`--founder-aggregation 0`). See below. |
| `reinvasion_barrier` | `sample:31`, seeds 1000–1007 (8/8 reached tick 1000 on both sides), injection at tick 1000, window 1000, cohort 8. The #587 arms (plain), #591 arms (`--accounting`) and #593 arms (`--dispersal`, dispersal own/1/2/4). |
| `role_diet_census` | Atlas (`atlas.json`, 95 cells) and the seed-421 LHS draw (200 configs). Seeds 1000–1004, T = 2000, `EvalConfig::default()`, as #596. |
| Machine | 8-core laptop, release builds. The two LHS sweeps ran concurrently. |

**Why pin founder aggregation.**
- Since #601, `search::decode` founds every decoded world at `DEFAULT_FOUNDER_AGGREGATION = 0.8`.
  `sample:N` and `atlas:N` both decode, so at `c095f04` every input of both instruments would found
  in tight patches, while at `f6a1c1f` they found uniformly.
- It matters on `sample:31`. Unpinned, the after-side resident reaches tick 1000 with a free pool of
  16,144 N against 7,092 N pinned, and a pile share of 0.36 against 0.29.
- At `a = 0` the sim returns the founder's uniform draw unchanged. The patch centres come from their
  own keyed stream, so the main stream is untouched. The sim test
  `at_the_well_mixed_end_placement_is_the_uniform_scatter` pins that.
- So the pinned after side founds exactly as the before side does. The remaining difference is the
  #600/#603 consumption physics, plus #602, which changes no verdict or trajectory.

**Commands** (cwd is each worktree root; each tree built with
`cargo build --release -p explorers-search --bin reinvasion_barrier --bin role_diet_census`):

```sh
# before: /Users/sam/Projects/explorers-605-before @ f6a1c1f
./target/release/reinvasion_barrier sample:31 --out target/605/rb-plain.json
./target/release/reinvasion_barrier sample:31 --accounting --out target/605/rb-accounting.json
./target/release/reinvasion_barrier sample:31 --dispersal --out target/605/rb-dispersal.json
./target/release/role_diet_census --atlas atlas.json --configs atlas:0,…,atlas:94 --out target/605/rd-atlas.jsonl
./target/release/role_diet_census --out target/605/rd-sample.jsonl

# after: /Users/sam/Projects/explorers-605-after @ c095f04 + 529ed16
./target/release/reinvasion_barrier sample:31 --founder-aggregation 0 --out target/605/rb-plain.json
./target/release/reinvasion_barrier sample:31 --accounting --founder-aggregation 0 --out target/605/rb-accounting.json
./target/release/reinvasion_barrier sample:31 --dispersal --founder-aggregation 0 --out target/605/rb-dispersal.json
./target/release/role_diet_census --atlas atlas.json --configs atlas:0,…,atlas:94 --founder-aggregation 0 --out target/605/rd-atlas.jsonl
./target/release/role_diet_census --founder-aggregation 0 --out target/605/rd-sample.jsonl

# summaries, each side
./target/release/role_diet_census --summary --out target/605/rd-atlas.jsonl
./target/release/role_diet_census --summary --out target/605/rd-sample.jsonl
```

`atlas.json` is byte-identical in the two trees. The LHS sweeps were resumed from pilot rows
(`sample:0`–`19`), which the census skips on resume.

**Wall clock:**

| run | before | after |
|---|---:|---:|
| `reinvasion_barrier` plain / accounting / dispersal | 11 s / 7 s / 14 s | 10 s / 7 s / 13 s |
| `role_diet_census`, atlas | 48 s | 48 s |
| `role_diet_census`, LHS | 2 h 05 min (7,490 s + 7 s pilot) | 1 h 46 min (6,342 s + 7 s pilot) |

**Reproduction checks.**
- The before-side census reproduces #596's tables digit for digit: the atlas death table, and the
  LHS live confusion and death tables.
- The before-side `sample:31` resident and control lines reproduce #593's (control rate −0.00058).
- The step arms do not reproduce #593 exactly. #599 changed how the resident producer centroid is
  read, from the trait tag to the income role. The arm figures here are therefore compared only
  with each other.

## 2. `sample:31`: demography

The #593 dispersal table, before → after. `Δ vs step 0` is the median over seeds of the per-seed
rate difference against `step 0` at the same dispersal. Infant grazed is per birth. Kin grazing is
median E per member-tick. Lineage end is median / max. There are 8 seeds per cell.

| arm | dispersal | median r | r > 0 | Δ vs step 0 | infant grazed per birth | grazed by kin | energy gate met | lineage end |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| step 0 | own | −0.00013 → −0.00038 | 3 → 2 | 0 | 0.07 → 0.17 | 0.0004 → 0.0017 | 0.173 → 0.095 | 7 / 21 → 5.5 / 26 |
| step 0 | 1 | +0.00001 → +0.00049 | 4 → 6 | 0 | 0.13 → 0.14 | 0.0009 → 0.0008 | 0.169 → 0.143 | 9 / 27 → 13 / 34 |
| step 0 | 2 | −0.00007 → +0.00022 | 3 → 5 | 0 | 0.15 → 0.06 | 0.0002 → 0.0007 | 0.216 → 0.136 | 7.5 / 36 → 10 / 37 |
| step 0 | 4 | −0.00008 → +0.00024 | 4 → 4 | 0 | 0.04 → 0.03 | 0.0000 → 0.0001 | 0.214 → 0.187 | 8 / 34 → 10.5 / 32 |
| step 1× | own | −0.00188 → −0.00208 | 0 → 0 | −0.00082 → **−0.00194** | 0.82 → **0.92** | 0.0071 → 0.0094 | 0.031 → 0.061 | 1.5 / 6 → 1 / 2 |
| step 1× | 1 | −0.00139 → −0.00104 | 1 → 0 | −0.00132 → −0.00131 | 0.62 → 0.71 | 0.0025 → 0.0060 | 0.041 → 0.066 | 2 / 14 → 3 / 8 |
| step 1× | 2 | −0.00084 → −0.00118 | 2 → 2 | −0.00077 → −0.00107 | 0.35 → 0.23 | 0.0030 → 0.0021 | 0.085 → 0.102 | 3.5 / 19 → 2.5 / 13 |
| step 1× | 4 | +0.00004 → −0.00139 | 4 → 3 | −0.00012 → −0.00118 | 0.25 → 0.16 | 0.0029 → 0.0028 | 0.107 → 0.075 | 8.5 / 27 → 2.5 / 37 |
| step 2× | own | −0.00208 → −0.00139 | 1 → 0 | −0.00132 → −0.00101 | 0.86 → **0.96** | 0.0125 → 0.0111 | 0.042 → 0.018 | 1 / 9 → 2 / 7 |
| step 2× | 1 | −0.00173 → −0.00118 | 0 → 1 | −0.00132 → −0.00154 | 0.81 → 0.80 | 0.0060 → 0.0062 | 0.093 → 0.016 | 1.5 / 7 → 2.5 / 9 |
| step 2× | 2 | −0.00058 → −0.00013 | 2 → 3 | −0.00039 → −0.00071 | 0.54 → 0.47 | 0.0056 → 0.0054 | 0.076 → 0.080 | 4.5 / 10 → 7 / 25 |
| step 2× | 4 | +0.00056 → +0.00032 | 5 → 6 | +0.00062 → +0.00018 | 0.26 → 0.29 | 0.0049 → 0.0038 | 0.063 → 0.079 | 14 / 33 → 11 / 35 |

**Kin grazing of the steps does not fall.** At own dispersal, kin take 0.0094 of the 0.0094 E per
member-tick grazed off `step 1×`, and 0.0111 of 0.0113 off `step 2×`. Before, the figures were
0.0071 of 0.0073 and 0.0125 of 0.0128. For `step 2×` the counts are stark: 353 births (111
before), 396 deaths, 362 grazed to death, and 338 of 339 infant deaths grazed.

**The producer loses from the change too.**
- `step 0`'s own-dispersal infant grazing more than doubles (0.07 → 0.17), and its kin-grazed
  energy quadruples.
- Its dispersal arms improve (5/8 and 6/8 seeds positive at dispersal 1 and 2). The steps do not
  keep pace.

**The steps are phenotypically the same on both sides.** Built on the after-side resident centroid,
their heterotrophy is 0.363 and 0.675 (before 0.355 and 0.668). Photosynthetic absorption is 1.12
(before 1.16), and kappa is 0.145 (before 0.156).

**Ties.** Many cells still have several seeds extinct inside the window at
`r = ln(½/8)/1000 = −0.00277`. The per-cell medians move on 1–2 seeds. The own-dispersal `step 1×`
result is not a tie artefact: its interval is [−0.00277, −0.00139], and 0/8 seeds are positive on
both sides.

## 3. `sample:31`: energy and lockup

Accounting (#591), median over seeds of per-member-tick means, own dispersal:

| arm | photosynthesis | living gain | carcass gain | income | net | earmark fill | energy gate met |
|---|---:|---:|---:|---:|---:|---:|---:|
| step 0 | 0.807 → 0.763 | 0.0001 → 0.0005 | 0.0001 → 0.0008 | 0.809 → 0.766 | 0.431 → 0.418 | 0.48 → 0.46 | 0.173 → 0.095 |
| step 1× | 1.054 → 1.325 | 0.0039 → 0.0043 | 0.0097 → 0.0080 | 1.062 → 1.340 | 0.665 → 0.788 | 0.76 → 0.92 | 0.031 → 0.061 |
| step 2× | 0.879 → 1.805 | 0.0055 → 0.0065 | 0.0059 → 0.0142 | 0.890 → 1.827 | 0.615 → 1.095 | 0.73 → 1.24 | 0.042 → 0.018 |

- **Living gain is not damped.** The step's grazing income, the drain the gate should suppress in a
  light-fed agent, is 0.0043 against 0.0039 before (`step 1×`) and 0.0065 against 0.0055
  (`step 2×`).
- **Energy was not the step's problem before, and it is less of one now.** Both steps' net rises;
  `step 2×`'s nearly doubles. As in #591/#593, the steps lose on recruitment.

**Lockup readout.** Carcass nutrient drawdown at the window end (control − arm, median over seeds):

| | own | 1 | 2 | 4 |
|---|---:|---:|---:|---:|
| step 0 | −231 → +32 | −47 → −621 | +1062 → −438 | −540 → −1149 |
| step 1× | +818 → −804 | +735 → +416 | +278 → +71 | +263 → +563 |
| step 2× | +140 → +1262 | +623 → +982 | −789 → +925 | **+3503 → +1162** |

- **The control piles are unchanged.** They hold 40,475 N before and 40,523 N after. The control
  free pool is 152 N → 777 N.
- **The arms' readings are mixed.** As in #593, these are percent-level moves of mixed sign on 8
  seeds.
  - `step 2×` now draws the pile down at every dispersal level. Its lineage still does not grow.
  - The dispersal-4 standout #593 found shrinks by two thirds.
  - All step arms but `step 1×` at dispersal 4 gain 34–643 N of free pool against the control;
    before, only the dispersal-2 and dispersal-4 `step 2×` arms did.
  - `step 0` loses about 400 N of free pool against its control at every level.

## 4. The role-diet census

**Mixotrophs by investment** (the retired trait tag: heterotrophy > photosynthetic absorption). The
table gives their light share of lifetime income over second-half agent-samples, with the number of
agent-samples in brackets:

| light share ≥ 0.9 | before | after |
|---|---:|---:|
| atlas, all 95 configs | 97.2 % (42,109) | 97.3 % (45,951) |
| atlas, live configs | 97.2 % (41,336; 92 configs) | 97.5 % (42,857; 88 configs) |
| LHS, all 200 configs | 82.8 % (180,378) | 82.2 % (225,890) |
| LHS, live configs | 96.7 % (59,721; 141 configs) | 97.5 % (66,360; 145 configs) |

There are slightly more mixotrophs by investment, and they are no less light-fed. The diet
agreement of tagged consumers on live configs falls to 0.8 % (atlas; 1.4 % before) and 0.9 % (LHS;
1.4 % before).

**Kin-grazed producer deaths** (trait producers, deaths over whole runs):

| | deaths | infant | grazed | kin share of grazed | kin-grazed share of deaths | infant kin-grazed |
|---|---:|---:|---:|---:|---:|---:|
| atlas, all | 115,201 → 125,076 | 83.9 → 84.4 % | 30.4 → 31.7 % | 77.7 → 78.6 % | 23.6 → 24.9 % | 25,957 → 29,958 |
| atlas, live on both (86) | 95,874 → 104,184 | 84.7 → 85.9 % | 30.3 → 32.9 % | 76.4 → 79.1 % | 23.1 → **26.0 %** | 21,138 → 26,109 |
| LHS, all | 2,474,749 → 2,203,802 | 94.1 → 93.7 % | 75.5 → 74.0 % | 33.5 → 32.0 % | 25.3 → 23.7 % | 616,171 → 513,428 |
| LHS, live on both (138) | 109,796 → 114,435 | 89.1 → 89.3 % | 27.2 → 28.1 % | 71.6 → 74.6 % | 19.5 → **21.0 %** | 20,585 → 23,111 |

- **The all-config LHS row falls.** That is the dense failing worlds getting a little less grazed.
  On the matched live configs, kin grazing of producers rises on both sources.
- **Heterotrophs by diet** (almost all infants) go the same way. Their kin share of grazing rises
  from 87.8 % to 91.4 % (atlas, all) and from 82.0 % to 85.6 % (LHS, matched live).
- **The census's own "live configs" LHS table overstates the change.** Its trait-producer deaths go
  411,350 → 128,085, with 72.6 % → 27.0 % grazed. That is a change of membership:
  - `sample:197` was live on 3/5 seeds before, with 297,993 producer deaths, 90 % of them grazed.
    After, it is monoculture on 5/5, so it is out of the pool.
  - `sample:16` and `58` also leave. `sample:27`, `50`, `53`, `66`, `120`, `161` and `171` join.
  - Read the matched rows above instead.
- **#596's §6 LHS death table is the live-config pool.** It is labelled "all configs", but its
  figures (411,350 producer deaths, 72.6 % grazed) are the live-config pool. The all-config pool is
  2,474,749 deaths, 75.5 % grazed, 33.5 % of it by kin. Its atlas table is the all-config pool, as
  labelled.

**Guild counts by role.** Seeds holding each guild (producer / consumer / decomposer):

| | tag, before | tag, after | diet, before | diet, after |
|---|---:|---:|---:|---:|
| atlas (475 seeds) | 213 / 1 / 4 | 206 / 1 / 6 | 241 / 0 / 0 | 248 / 0 / 0 |
| LHS (1,000 seeds) | 248 / 3 / 21 | 262 / 7 / 28 | 288 / 1 / 0 | 312 / 0 / 1 |

- **Configs holding a heterotroph guild on at least half their seeds, by diet:** none, on either
  side.
- **By tag,** decomposer-guild configs go 3 → 5 (LHS). The tag's artefact grows a little with the
  extra mixotrophs by investment.
- **Configs holding a producer guild by diet go up:** atlas 46 → 55, LHS 53 → 57.

**Verdicts** (seeds):

| | live | lockup | extinction | monoculture | energy death | generalist dominance |
|---|---:|---:|---:|---:|---:|---:|
| atlas | 427 → 416 | 40 → 52 | 8 → 4 | 0 → 0 | 0 → 0 | 0 → 3 |
| LHS | 695 → 701 | 175 → 172 | 73 → 60 | 34 → 41 | 18 → 20 | 5 → 6 |

- **Atlas.** The atlas loses live configs `atlas:11`, `58`, `62`, `64`, `76` and `83` and gains
  `31` and `39`.
- **Generalist dominance.** It appears on the atlas under need-gating, on 3 seeds. That is too few
  seeds to read.

## 5. Sweep cost by config

The per-config wall clock on the LHS draw (5 seeds in parallel), before → after, for the configs
that cost more than a minute on either side:

| config | before | after |
|---|---:|---:|
| `sample:165` | 737 s | 1,122 s |
| `sample:20` | 954 s | 345 s |
| `sample:24` | 892 s | 725 s |
| `sample:147` | 816 s | 890 s |
| `sample:38` | 869 s | 164 s |
| `sample:129` | 607 s | 670 s |
| `sample:154` | 589 s | 144 s |
| `sample:169` | 356 s | 527 s |
| `sample:197` | 350 s | 526 s |
| `sample:100` | 240 s | 201 s |
| `sample:151` | 150 s | 133 s |
| `sample:96` | 63 s | 80 s |

The sums are 6,935 s before and 5,787 s after. The dense set is still the one #505/#508 named
(`sample:169` included). `sample:20`, `38` and `154` got much lighter; `165`, `169`
and `197` got heavier. The atlas costs 48 s on both sides. The instrument-runtimes figure for
`role_diet_census` (LHS ~90 min) holds as an order of magnitude on both sides.

## 6. Reading

Against the issue's questions:

- **Does infant kin-grazing fall?** No, not where it matters. At the resident's own dispersal it
  rises for the producer and both steps. Across the atlas and LHS live worlds, the kin-grazed share
  of producer deaths rises by 1.5–3 points. It falls only in arms whose newborns dispersal has
  already moved out of kin reach.
- **Does the step close on the producer?** No. `step 1×` falls further behind (−0.00082 →
  −0.00194). `step 2×` is unchanged within noise. The dispersal-4 lead #593 found for `step 2×`
  shrinks from +0.00062 to +0.00018.
- **What moved unexpectedly:**
  - The steps' energy economics improved while their recruitment did not.
  - The resident at injection is itself a different population.
  - Atlas lockup rose (40 → 52 seeds).
  - Per-config cost moved by up to 5× in both directions.

The world rules justify need-gating by this mechanism: a light-fed near-producer's heterotrophy
becomes mostly latent, so it stops grazing its newborns. On these instruments, at the default
`satiation_sensitivity = 0.1`, the downstream consequence is not observed. The grazing income of the
steps (living gain) is not damped either. So on `sample:31` the gate is not suppressing the drain.
The data do not say which of these is true:
- the step agents are rarely sated (little reserve against maintenance, for example with
  `kappa ≈ 0.15` routing most mobilised energy to reproduction);
- the gate's scale is too weak at `c = 0.1`;
- the post-change resident changes the context enough to hide the effect.

## 7. What this does not show

- **Satiation itself.** Neither instrument reads a consumer's satiation or its expressed drain
  against its capability, so the mechanism behind §6 is not traced.
- **The flat limit in-tree.** `satiation_sensitivity = 0` is the ungated drain, kept in the world
  rules for comparison. An after-tree run at `c = 0` with everything else fixed would separate the
  gate from the rest of what changed between `f6a1c1f` and `c095f04`, the resident included. It needs
  a small flag and is not run here. It is the obvious follow-up, together with a satiation readout
  in the lineage accountant.
- **The same resident.** The design forks each side's own resident at tick 1000. An after-side
  invader therefore meets a resident that evolved under need-gating. A fork-from-common-state
  comparison would need a checkpoint shared across trees, which the instruments do not support.
- **One config, 8 seeds, for the invasion side.** As in #593, several cells are decided by
  extinction ties.
- **Founder aggregation at its design default.** Everything here is at `a = 0`. The unpinned
  after-side `sample:31` run (`a = 0.8`) is not analysed. It is kept at
  `target/605/rb-dispersal-agg08.*` in the after tree.
- **#604 recognition** is not in either tree. #606 measures it.

## 8. Instrument

- `crates/explorers-search/src/config_source.rs`:
  - `with_founder_aggregation(config, Option<f32>)` pins a resolved world's
    `founder_aggregation` and leaves everything else as decoded;
  - `parse_founder_aggregation` accepts values in [0, 1] only.
- `reinvasion_barrier --founder-aggregation A`:
  - records the pin as the artifact's `founder_aggregation`, left out when unset, so earlier
    artifacts read back unchanged;
  - `--merge` refuses chunks with different pins.
- `role_diet_census --founder-aggregation A`:
  - pins every world in `run_row`;
  - records the pin on each row, left out when unset.
- Tests:
  - the pin overrides only `founder_aggregation`, and the flag's range is enforced;
  - `reinvasion_barrier`: CLI parsing, the artifact's record and round trip, and the merge guard;
  - `role_diet_census`: the pin reaches both the world and the row, and changes the rollout; an
    unpinned row serialises without it.
- Artifacts are under `target/605/` in each worktree:
  - `rb-{plain,accounting,dispersal}.{json,md}`;
  - `rd-{atlas,sample}.jsonl` and `rd-{atlas,sample}-summary.md`.
