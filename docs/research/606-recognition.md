# Issue #606: recognition on `sample:31` and the role-diet census

**Status: measurement. Adds a killing-grazer hunger readout and a parent–offspring distance readout
to `role_diet_census` and to `reinvasion_barrier`'s lineage accountant
(`explorers_search::grazer_hunger`). No trajectory changes: on every arm, every pre-existing table
and census row is identical with the readout on. `explorers-sim`, the search, the evaluator and the
prefilter are unchanged.**

#604 added **recognition**: a consumer withholds `r·w(d)` of its capability from a living target
that resembles it. Here `w(d) = (1 − (d/D)²)²` inside the recognition distance `D = 0.5`, and the
restraint is `r = ½`. Hunger relaxes the restraint only partially. The design rationale
(world rules, *Recognition*) predicts that a near-producer's newborns, a small mutation from it,
stop being its prey, so the #591/#593 kin-grazing cliff on `sample:31` should go. #605 found that
need-gating alone did not remove that cliff. This note measures recognition on top of need-gating,
with the same instruments and format. It compares three arms:
- the original baseline;
- need-gating only;
- recognition.

## TL;DR

1. **Nothing invades by the #587 criterion** (the arm's median `r > 0` *and* its sign-test interval
   clears zero).
   - At the resident's own dispersal, every step arm has a negative median (−0.00139 to −0.00173) and 0/8 seeds positive.
   - The best arm is still `step 2×` at dispersal 4. Its median is +0.00027 with 5/8 seeds
     positive, but its interval is [−0.00277, +0.00148].
2. **Infant kin-grazing of the steps is barely touched.** At own dispersal, infant grazed deaths
   per birth are:

   | arm | baseline | need-gating | recognition |
   |---|---:|---:|---:|
   | `step 1×` | 0.82 | 0.92 | 0.81 |
   | `step 2×` | 0.86 | 0.96 | 0.93 |

   Kin still take 0.0093 of the 0.0094 E per member-tick grazed off `step 2×`.
3. **The step's paired lead over the producer is an artefact of a producer collapse.**
   - `step 2×` now beats `step 0` at own dispersal by +0.00035, against −0.00101 under need-gating.
   - `step 2×` did not improve. The `step 0` cohort collapsed: its median `r` went from −0.00038 to
     −0.00139, with 0/8 seeds positive, births 202 → 91 and member-ticks 6,554 → 3,255.
4. **Within-cluster predation remains, and it is hunger-driven.** The new readout reads each killing
   grazer's satiation in the drain pass, exactly.
   - On the configs live on all three arms, 88–93 % of kin kills are by a grazer under half
     expression (< 10 ticks of maintenance in its scarcer currency), against 83–87 % on the older
     arms.
   - Kills of near-identical kin (`d < 0.1`) are 99.7 % hungry under recognition, against about 87 %
     before. The physics requires that.
   - **But the grazers are hungry on every arm, the baseline included.** Killers sit mostly at 1–5
     ticks of reserve. *Inference (§6):* the grow phase mobilises reserve above
     `growth_retention_multiplier` ticks of maintenance, so agents are rarely sated. The need gate
     then sits near expression 0.7–0.9, and recognition's restraint toward an identical newborn
     leaves about a quarter of capability, which is enough to kill a newborn.
5. **Kin grazing went up, not down.**
   - The kin-grazed share of trait-producer deaths on configs live on all three arms:

     | | baseline | need-gating | recognition |
     |---|---:|---:|---:|
     | atlas (84 configs) | 23.2 % | 26.1 % | 26.8 % |
     | LHS draw (138 configs) | 19.5 % | 21.0 % | 23.1 % |

   - Kin kill pairs rise on both (atlas 38.5k → 53.0k → 61.1k).
   - About a quarter to a third of kin kills are of kin at `d ≥ D`, where recognition does not
     apply.
6. **No consumer or decomposer guild by diet.**
   - No config holds one on ≥ half its seeds, on either source, on any arm.
   - Per seed, there is one heterotroph-by-diet guild in 1,475 on each side, and recognition's is
     the same seed as need-gating's (`sample:45`/1001, a decomposer guild under generalist
     dominance).
7. **`D = 0.5` is not "ten mutation steps" in the searched worlds.** `sample:31` mutates at
   σ = 0.312 (rate 0.169), so `D` is 1.6σ.
   - Its parent–offspring distance has rms 0.32 (analytic √(7pσ²) = 0.34), and 14 % of births land
     beyond `D`.
   - Births beyond `D`: 25 % on the atlas, 20–21 % on the matched LHS configs, and 39 % on all LHS
     configs.
8. **What moved unexpectedly:**
   - **Generalist dominance rises:** LHS 5 → 6 → 16 seeds, atlas 0 → 3 → 5.
   - **`sample:20` becomes live on 3/5 seeds** (monoculture on 3–4/5 before). Its world is denser
     (143k second-half agent-samples, against 67k–104k), and it cost about 65 min of census time.
   - **The resident's own heterotrophs live longer on the `sample:31` control.** They have 546
     member-ticks against 162.

## 1. What ran

| arm | tree | physics |
|---|---|---|
| **baseline** | `f6a1c1f` + `b76a994` (worktree `explorers-605-before`) | main after #599's income-role read; no need gate, no recognition |
| **need-gating** | `c095f04` + `403587a` + `aee92c8` (worktree `explorers-605-after`) | #600 need-gating + #603 nutrient co-limitation |
| **recognition** | `ca37c7b` + `4e3aa55` (worktree `explorers-606-recognition`) | need-gating + #604 recognition (`D = 0.5`, `r = ½`) |

- **Readout commits.** `4e3aa55` is this issue's readout. `b76a994` and `aee92c8` are cherry-picks
  of it onto the older trees; their tests drop the terms those trees lack (recognition; the need
  gate).
- **Nothing else differs.** Between `403587a` and `ca37c7b` the only physics change is #604. The
  rest is the #605 note, test and app fixtures gaining the new parameter, `scenarios/observed.json`
  and docs.
- **The founder aggregation is pinned to 0 on the need-gating and recognition arms**
  (`--founder-aggregation 0`), as in #605.
  - Since #601, `search::decode` founds every decoded world in patches
    (`DEFAULT_FOUNDER_AGGREGATION = 0.8`). The baseline tree founds uniformly.
  - At `a = 0` the sim returns the uniform draw unchanged, so pinning makes the three arms found
    identically.
  - The baseline has no flag and runs unpinned.
- **`reinvasion_barrier`:** `sample:31`, seeds 1000–1007 (8/8 reached tick 1000 on every arm),
  injection at tick 1000, window 1000, cohort 8. It ran in plain (#587), `--accounting` (#591) and
  `--dispersal` (#593: dispersal own/1/2/4) modes.
- **`role_diet_census`:** the atlas (`atlas.json`, 95 cells) and the seed-421 LHS draw (200 configs),
  seeds 1000–1004, T = 2000, `EvalConfig::default()`. A one-config census of `sample:31` was also
  run.
- **Machine:** 8-core laptop, release builds.

**Commands.** The cwd is each worktree root. `PIN` is `--founder-aggregation 0` on the need-gating
and recognition arms and empty on the baseline. Each tree was built with
`cargo build --release -p explorers-search --bin reinvasion_barrier --bin role_diet_census`.

```sh
./target/release/reinvasion_barrier sample:31 $PIN --out target/606/rb-plain.json
./target/release/reinvasion_barrier sample:31 --accounting $PIN --out target/606/rb-accounting.json
./target/release/reinvasion_barrier sample:31 --dispersal $PIN --out target/606/rb-dispersal.json
./target/release/role_diet_census --atlas atlas.json --configs atlas:0,…,atlas:94 $PIN --out target/606/rd-atlas.jsonl
./target/release/role_diet_census --configs sample:31 $PIN --out target/606/rd-s31.jsonl
./target/release/role_diet_census $PIN --out target/606/rd-sample.jsonl
./target/release/role_diet_census --summary --out target/606/<file>.jsonl
```

**How the recognition LHS sweep ran.** `sample:20` held the first process for 65 min. A second
process ran `sample:21`–`199` into `rd-sample-rest.jsonl`, and the first was stopped once `sample:20`
landed. The merged file `rd-sample-merged.jsonl` takes rows 0–20 from the first process and 21–199
from the second. The first process had already finished `sample:21`–`23`, and those three rows are
byte-identical in the two files.

**Reproduction checks.**
- **No trajectory change.** With the readout on, all 95 atlas rows and all 200 LHS rows on the
  baseline and need-gating arms are identical to their #605 artefacts once the two new fields are
  removed.
- **Barrier tables unchanged.** Every pre-existing `reinvasion_barrier` table reproduces #605's on
  those arms.
- **Recognition arm.** Its atlas rows and barrier tables are identical with and without the readout.

**Wall clock:**

| run | baseline | need-gating | recognition |
|---|---:|---:|---:|
| `reinvasion_barrier` plain / accounting / dispersal | 11 s / 7 s / 14 s (#605) | 10 s / 7 s / 13 s (#605) | 13 s / 8 s / 16 s |
| `role_diet_census`, atlas | 48 s (#605) | 48 s (#605) | 56 s without the readout, 60 s with it |
| `role_diet_census`, LHS, real | 3 h 27 min | 2 h 56 min | 1 h 18 min (0–23) + 2 h 50 min (21–199), overlapping |
| LHS, sum of per-config times | 11,666 s | 9,774 s | 14,123 s |

The three LHS sweeps ran **concurrently**, with up to four processes on 8 cores, so their per-config
times are inflated against #605's solo runs (on configs 20–199, the sum was 6,928 s solo against
11,653 s here on the baseline). Compare the arms with each other only. The readout costs about 5 %
(atlas 56 s → 60 s).

## 2. The readout

`explorers_search::grazer_hunger`:

- **`PreStep`** captures the roster and nutrient grid before a step. It replays the tick's first
  four phases (photosynthesise, absorb nutrients, metabolise, grow) on a copy, using the stepper's
  own public phase functions. The result is the state the drain pass read. A test reproduces every
  living drain a lone consumer took from a target that could cover it, as
  `h_eff · u_H · max(0, E − r·w(d))` read off the replayed roster, **bit for bit**. The test fails if
  the grow replay is dropped.
- **`Satiation`** reads an agent's reserve and its nutrient-matched energy in ticks of its own
  maintenance. The need gate reads the lesser. Its expression is pinned against the stepper's
  `consumption_expression`. It is computed independently of the stepper so it also works on the
  baseline tree, which has no gate.
- **`GrazerHunger`** tallies (grazer, victim) pairs by:
  - kinship;
  - trait distance, in the bands `[0, 0.1)`, `[0.1, 0.25)`, `[0.25, 0.5)`, `[0.5, 1)`, `≥ 1`;
  - satiation, in the bands 0–1, 1–5, 5–10, 10–20, 20–50 and ≥ 50 ticks;
  - whether the grazer is nutrient-limited.

  "Hungry" means < 10 ticks: under half expression at `c = 0.1`. That is the point from which
  recognition spares an identical target completely.
- **`role_diet_census`** books every grazed death's grazers (kin = parent, offspring or sibling) and
  every birth's trait distance to each parent.
- **`reinvasion_barrier`** books the grazers of members grazed to death. Kin means the grazer is
  itself a member.

## 3. `sample:31`: invasion and demography

The #593 dispersal table across the three arms. Each cell reads baseline → need-gating →
recognition. `Δ vs step 0` is the median over seeds of the per-seed rate difference against
`step 0` at the same dispersal. There are 8 seeds per cell.

| arm | dispersal | median r | r > 0 | Δ vs step 0 | infant grazed per birth | grazed by kin (E / member-tick) |
|---|---:|---:|---:|---:|---:|---:|
| step 0 | own | −0.00013 → −0.00038 → **−0.00139** | 3 → 2 → **0** | 0 | 0.07 → 0.17 → 0.18 | 0.0004 → 0.0017 → 0.0001 |
| step 0 | 4 | −0.00008 → +0.00024 → −0.00139 | 4 → 4 → 1 | 0 | 0.04 → 0.03 → 0.00 | 0.0000 → 0.0001 → 0.0000 |
| step 1× | own | −0.00188 → −0.00208 → −0.00173 | 0 → 0 → 0 | −0.00082 → −0.00194 → −0.00035 | 0.82 → 0.92 → 0.81 | 0.0071 → 0.0094 → 0.0038 |
| step 1× | 2 | −0.00084 → −0.00118 → −0.00139 | 2 → 2 → 1 | −0.00077 → −0.00107 → +0.00069 | 0.35 → 0.23 → 0.30 | 0.0030 → 0.0021 → 0.0026 |
| step 1× | 4 | +0.00004 → −0.00139 → −0.00021 | 4 → 3 → 2 | −0.00012 → −0.00118 → +0.00057 | 0.25 → 0.16 → 0.08 | 0.0029 → 0.0028 → 0.0018 |
| step 2× | own | −0.00208 → −0.00139 → −0.00173 | 1 → 0 → 0 | −0.00132 → −0.00101 → **+0.00035** | 0.86 → 0.96 → 0.93 | 0.0125 → 0.0111 → 0.0093 |
| step 2× | 1 | −0.00173 → −0.00118 → −0.00139 | 0 → 1 → 0 | −0.00132 → −0.00154 → −0.00029 | 0.81 → 0.80 → 0.85 | 0.0060 → 0.0062 → 0.0067 |
| step 2× | 2 | −0.00058 → −0.00013 → −0.00098 | 2 → 3 → 1 | −0.00039 → −0.00071 → +0.00014 | 0.54 → 0.47 → 0.65 | 0.0056 → 0.0054 → 0.0075 |
| step 2× | 4 | +0.00056 → +0.00032 → +0.00027 | 5 → 6 → 5 | +0.00062 → +0.00018 → +0.00071 | 0.26 → 0.29 → 0.34 | 0.0049 → 0.0038 → 0.0041 |

**Invasion, by the #587 criterion: no arm invades on any of the three arms.**
- On the recognition arm, no step arm at own dispersal has a positive median.
- The only positive medians are `step 2×` at dispersal 4 (+0.00027, interval [−0.00277, +0.00148])
  and, on the plain mode's placements, none.
- The full founder heterotroph still dies out on 8/8 seeds within about 10 ticks, at both
  placements.

**The positive `Δ vs step 0` values come from a collapsing producer, not a rising step.**
- The `step 0` cohort, the resident producer centroid with phenotype unchanged across arms
  (photosynthetic absorption 1.12, heterotrophy 0.04), loses its edge under recognition:

  | `step 0` cohort | baseline | need-gating | recognition |
  |---|---:|---:|---:|
  | member-ticks | 9,698 | 6,554 | 3,255 |
  | births | 179 | 202 | 91 |

  Its median is −0.00139 at every dispersal level.
- Its energy is not the problem: its net is 0.49 E per member-tick (0.42 under need-gating), and
  its energy gate is met on 17 % of member-ticks.
- Its nutrient earmark sits at 17 N (41 N under need-gating), as on the baseline.
- The resident it meets has less free nutrient at injection: 5,557 N, against 7,092 N under
  need-gating and 6,487 N on the baseline.
- Why the producer cohort fails under recognition is not traced here.
- **Read the paired deltas against that collapse.** A step "beating" a producer that itself declines
  at `r = −0.00139` is not a step that establishes.

**What remains of infant kin-grazing.**
- At own dispersal, `step 1×` loses 0.81 infants per birth to grazing (0.92 under need-gating), and
  `step 2×` loses 0.93 (0.96).
- Kin still take essentially all of it: 0.0038 of 0.0041 E per member-tick off `step 1×`, and 0.0093
  of 0.0094 off `step 2×`.
- Recognition halves `step 1×`'s kin-grazed energy (0.0094 → 0.0038). It does not change its infant
  mortality.

**The residual #593 could not explain.** Moving newborns out of kin reach (dispersal 4) still helps
more than recognition does:
- `step 1×`'s infant grazing per birth falls to 0.08 at dispersal 4.
- `step 2×` at dispersal 4 is still the only cell with most seeds positive.

The steps' energy economics (accounting mode, own dispersal, median E per member-tick, baseline →
need-gating → recognition):

| arm | photosynthesis | living gain | carcass gain | net | energy gate met |
|---|---:|---:|---:|---:|---:|
| step 0 | 0.807 → 0.763 → 0.808 | 0.0001 → 0.0005 → 0.0001 | 0.0001 → 0.0008 → 0.0001 | 0.431 → 0.418 → 0.486 | 0.173 → 0.095 → 0.171 |
| step 1× | 1.054 → 1.325 → 0.439 | 0.0039 → 0.0043 → 0.0025 | 0.0097 → 0.0080 → 0.0060 | 0.665 → 0.788 → 0.285 | 0.031 → 0.061 → 0.044 |
| step 2× | 0.879 → 1.805 → 1.519 | 0.0055 → 0.0065 → 0.0052 | 0.0059 → 0.0142 → 0.0123 | 0.615 → 1.095 → 0.970 | 0.042 → 0.018 → 0.026 |

- **Living gain falls a little under recognition**, which is the drain recognition targets.
- **`step 1×`'s photosynthesis drops by two thirds** (1.33 → 0.44). Its lineage crowds into its own
  shade: member-ticks rise 1,360 → 2,436 on similar births (84 against 98). The energy-gate rate
  stays low on every arm, as #591 found.

**The resident's own heterotrophs live longer on the control fork.** They are role consumer or
decomposer at injection.
- They have 546 member-ticks (162 under need-gating), 38 births and photosynthesis of 2.09 E per
  member-tick. They are mostly light-fed.
- All 38 of their infant deaths are grazed, and kin take all of the energy grazed off them.
- The control's carcass nutrient at the window end is 41,792 N (40,523 under need-gating), and its
  pool is 408 N (777).

## 4. Killing grazers: is within-cluster predation hunger-driven?

**`sample:31` lineages** (`--dispersal`, (grazer, member) pairs of members grazed to death, summed
over 8 seeds; own dispersal; baseline → need-gating → recognition):

| arm | kin pairs | hungry | kin within `d < 0.5` | nutrient-limited | s 1–5 ticks |
|---|---:|---:|---:|---:|---:|
| step 0 | 42 → 63 → 26 | 95 → 94 → 100 % | 52 → 63 → 31 % | 2 → 13 → 0 % | 34 → 48 → 25 |
| step 1× | 218 → 118 → 97 | 82 → 93 → 99 % | 85 → 85 → 90 % | 11 → 24 → 10 % | 116 → 63 → 73 |
| step 2× | 156 → 484 → 269 | 67 → 74 → 81 % | 81 → 83 → 80 % | 1 → 10 → 13 % | 90 → 278 → 141 |

Non-kin killers of members are 88–100 % hungry in every cell. At dispersal 1–4 the step lineages'
kin killers are 85–100 % hungry on every arm.

**Census.** The table gives kin kill pairs, all victims, on configs live on all three arms, with
each cell reading baseline → need-gating → recognition:

| | pairs | hungry (< 10 ticks) | within `d < 0.5` | at `d < 0.1`: hungry | at `d ≥ 0.5`: share of kin pairs | nutrient-limited |
|---|---:|---:|---:|---:|---:|---:|
| atlas (84) | 38,535 → 53,009 → 61,138 | 84.3 → 82.7 → **88.4 %** | 62.6 → 60.0 → 64.6 % | 86.1 → 88.0 → **99.7 %** | 37.4 → 40.0 → 35.4 % | 30.6 → 29.0 → 31.8 % |
| LHS (138) | 41,635 → 51,981 → 59,729 | 86.8 → 87.4 → **92.5 %** | 74.8 → 73.9 → 72.5 % | 86.7 → 88.7 → **99.7 %** | 25.2 → 26.1 → 27.5 % | 32.3 → 32.5 → 36.1 % |

- **Within-cluster predation remains.** On the matched configs, 65–73 % of kin kills under
  recognition are within `D`. There are more of them than under need-gating: atlas 31.8k → 39.5k,
  LHS 38.4k → 43.3k.
- **It is hunger-driven where recognition bites.**
  - At `d < 0.1`, `w > 0.92`. A grazer then drains only if `E > 0.46`, i.e. `s < 11.7` ticks.
    99.7 % of such kills are hungry, against 86–89 % on the arms without recognition.
  - Farther out, recognition's grip weakens, and the hungry share falls to 75–88 %.
- **Hunger is the ordinary state, not a recognition effect.**
  - On every arm, 27–36 % of kin killers hold under 1 tick of reserve, and 33–40 % hold 1–5.
  - On the matched LHS configs, the satiation distribution of recognition's kin killers is:

    | s (ticks) | 0–1 | 1–5 | 5–10 | 10–20 | 20–50 | ≥ 50 |
    |---|---:|---:|---:|---:|---:|---:|
    | share of kin killers | 36 % | 40 % | 17 % | 5 % | 2 % | 0.4 % |

  - About a third are nutrient-limited, and the rest energy-limited.
  - On `sample:31` the steps' killers are 76–100 % energy-limited.
- **Non-kin killers** are 90–94 % hungry on every arm.
- **Recognition shifts within-trait-cluster non-kin kills.** It does not reduce them. Under
  recognition, 17 % of non-kin kill pairs on the matched LHS configs are within `D` (12–15 % on the
  older arms), and 23 % on the atlas (13–14 %).

## 5. The role-diet census

**Guilds by diet.**
- No config holds a consumer or decomposer guild on at least half its seeds, on either source, on
  any arm.
- Per seed, the heterotroph-by-diet guilds are:
  - baseline: one consumer guild (`sample:165`/1001, monoculture);
  - need-gating: one decomposer guild (`sample:45`/1001, generalist dominance);
  - recognition: the same `sample:45`/1001.
- The atlas has none on any arm.
- Producer-guild configs by diet: atlas 46 → 55 → 54, LHS 53 → 57 → 58.

**Seeds holding each guild, by tag and by diet** (producer / consumer / decomposer):

| | tag | diet |
|---|---|---|
| atlas (475 seeds) | 213/1/4 → 206/1/6 → 207/1/9 | 241/0/0 → 248/0/0 → 250/0/0 |
| LHS (1,000 seeds) | 248/3/21 → 262/7/28 → 253/11/36 | 288/1/0 → 312/0/1 → 319/0/1 |

The retired trait tag's spurious heterotroph guilds keep growing.

**Mixotrophs by investment still live on light.** The share of trait-tagged heterotroph
agent-samples taking ≥ 90 % of lifetime income from light:

| | baseline | need-gating | recognition |
|---|---:|---:|---:|
| atlas, live configs | 97.2 % | 97.5 % | 96.6 % (52,278 samples) |
| LHS, matched | 98.6 % | 97.5 % | 96.9 % (69,009 samples) |

**Kin-grazed producer deaths** (trait producers, whole runs, matched live configs):

| | deaths | grazed | kin share of grazed | kin-grazed share of deaths | infant kin-grazed |
|---|---:|---:|---:|---:|---:|
| atlas (84) | 92,977 → 101,156 → 103,430 | 30.4 → 33.0 → 34.5 % | 76.6 → 79.2 → 77.6 % | 23.2 → 26.1 → **26.8 %** | 20,626 → 25,462 → 26,572 |
| LHS (138) | 109,796 → 114,435 → 109,456 | 27.2 → 28.1 → 30.8 % | 71.6 → 74.6 → 75.2 % | 19.5 → 21.0 → **23.1 %** | 20,585 → 23,111 → 24,274 |

- **Heterotrophs by diet**, almost all infants, are grazed by kin about as often: LHS matched
  82.0 → 85.6 → 86.4 %.
- **Light-fed mixotrophs by investment are grazed by kin more:**

  | kin share of grazed | baseline | need-gating | recognition |
  |---|---:|---:|---:|
  | LHS matched | 56.5 % | 62.9 % | 74.3 % |
  | atlas, all | 67.7 % | 75.2 % | 86.0 % |

- **Read the matched rows.** The census's own LHS "live configs" pool is dominated by membership:
  - Under recognition `sample:20` joins the live set, live on 3/5 seeds. Its 641k producer deaths
    are 85 % of that pool's 755k.
  - `sample:58` also joins. `sample:50`, `53` and `66` leave.

**Verdicts** (seeds):

| | live | lockup | extinction | monoculture | energy death | generalist dominance |
|---|---:|---:|---:|---:|---:|---:|
| atlas | 427 → 416 → 416 | 40 → 52 → 46 | 8 → 4 → 7 | 0 → 0 → 1 | 0 → 0 → 0 | 0 → 3 → **5** |
| LHS | 695 → 701 → 700 | 175 → 172 → 169 | 73 → 60 → 53 | 34 → 41 → 41 | 18 → 20 → 21 | 5 → 6 → **16** |

Live atlas configs are 92 → 88 → 89. Against need-gating, recognition gains `atlas:62`, `76` and
`83` and loses `atlas:34` and `87`.

## 6. Why the grazers are hungry (inference)

*This section is inference from the readout and the grow phase's code. It is not separately
measured.*

- **The stepper's mechanism.** The grow phase mobilises
  `reserve_mobilisation_rate × (reserve − growth_retention_multiplier × maintenance)` each tick into
  structure and the reproductive earmark. The need gate reads satiation as
  `reserve / maintenance`, in the same units.
- **On `sample:31`** (`growth_retention_multiplier = 2.68`, `reserve_mobilisation_rate = 0.82`),
  an agent ends each grow phase near 2.7 ticks of reserve plus 18 % of any excess. It enters the
  drain pass there, with `E = 1/(1 + 0.1·s) ≈ 0.75–0.8`.
- **What the readout shows.** Killers sit at 1–5 ticks on every arm, the pre-gate baseline
  included, so this is not a response to the gate.
- **The consequence.** The gate's half-expression point (10 ticks) lies above where the retention
  buffer holds the reserve. Recognition's full sparing starts at that same point, so a typical agent
  never reaches it. Toward an identical newborn it still expresses `E − ½ ≈ 0.27` of capability. A
  newborn's structure is tiny (trade-off #9), so that drain kills it.
- **The searched box.** `growth_retention_multiplier` ranges over [1, 5], so the cap sits at 1–5
  ticks across the box, the band most killers occupy.

This answers #605's open question in favour of "the step agents are rarely sated" over "the gate is
too weak". The two are not exclusive: a larger `satiation_sensitivity` moves half expression down
toward the retention buffer. But the yardstick the gate reads is held near a fixed point by
another phase.

## 7. Recognition distance against parent–offspring distance

**Analytic.** An asexual birth mutates each of the 7 traits with probability `p` by `N(0, σ²)`,
floored at 0. Away from the floors, `E[d²] = 7pσ²`, and a birth is a clone with probability
`(1 − p)⁷`.

At `sample:31` (`σ = 0.312`, `p = 0.169`):
- the rms parent–offspring distance is 0.34;
- 27 % of births are clones;
- a one-trait mutation moves 0.25 on average;
- 89 % of one-trait mutations land inside `D = 0.5`, which is 1.6σ.

**Measured** (the census, every birth's distance to each parent):

| | births | mean | rms | `< 0.1` | beyond `D` |
|---|---:|---:|---:|---:|---:|
| `sample:31`, recognition (5 seeds, to lockup) | 1,463 | 0.223 | 0.318 | 40 % | 14 % |
| atlas, all configs | 165,763 | 0.328 | 0.506 | 38 % | 25 % |
| LHS, matched live configs | 137,400 | 0.292 | 0.456 | 39 % | 21 % |
| LHS, all configs | 4.1 M | 0.466 | 0.617 | 19 % | 39 % |

The arms differ by a few points only. The world rules justified `D = 0.5` as "ten mutation steps
(`mutation_magnitude` 0.05)". The box searches `mutation_magnitude` over [0.01, 0.5], so in the
searched worlds `D` is one to a few steps. A fifth to two fifths of newborns are born outside their
parent's recognition. That matches §4: a quarter to a third of kin kills are at `d ≥ D`.

## 8. Sweep cost by config

Per-config wall clock on the LHS draw, under concurrent load (§1), for configs above a minute on
any arm. The verdict string is per seed: `l` live, `m` monoculture, `n` nutrient lockup, `e`
extinction or energy death, `g` generalist dominance. Agent-samples are the second-half role samples
summed over seeds, a roster proxy.

| config | baseline | need-gating | recognition | agent-samples b / n / r | verdicts b / n / r |
|---|---:|---:|---:|---:|---|
| `sample:20` | 2,259 s | 787 s | **3,900 s** | 104k / 67k / **143k** | mmlml / mmmmg / lllmg |
| `sample:24` | 1,449 s | 1,688 s | 1,646 s | 107k / 127k / 90k | ememe ×3 |
| `sample:154` | 1,048 s | 176 s | **1,486 s** | 15k / 0 / **89k** | nnnnn / nnnnn / mnnnn |
| `sample:129` | 887 s | 950 s | 1,154 s | 152k / 167k / 180k | nnnmm / nnnmm / nmnmm |
| `sample:147` | 1,259 s | 1,349 s | 1,079 s | 207k / 205k / 218k | mmmnm / mmmmn / mnmmm |
| `sample:165` | 1,192 s | 1,737 s | 991 s | 257k / 350k / 264k | mmmml / mmmmm / mmmml |
| `sample:38` | 1,256 s | 302 s | 764 s | 347k / 307k / 402k | mmmmm / gmmmn / mlmmg |
| `sample:55` | 49 s | 57 s | **515 s** | 0 / 0 / **94k** | nnnnn / nnnnn / gnlgn |
| `sample:100` | 390 s | 298 s | 508 s | 208k / 190k / 158k | mnglm / mnmnm / gngng |
| `sample:169` | 529 s | 828 s | 500 s | 117k / 116k / 115k | mmlml / mmlmm / mmlmm |
| `sample:146` | 18 s | 15 s | **498 s** | 1k / 0 / **98k** | nnnln / nnnnn / nnnmm |
| `sample:197` | 638 s | 966 s | 359 s | 118k / 94k / 95k | mmlll / mmnmm / mmnmm |
| `sample:96` | 90 s | 103 s | 150 s | 20k / 33k / 37k | |
| `sample:61` | 0 s | 0 s | 138 s | 0 / 0 / 30k | eeeee / eeeee / meeee |
| `sample:151` | 200 s | 170 s | 94 s | 22k / 24k / 22k | lnnln ×3 |

- **Recognition's newly heavy configs carry larger, longer-lived rosters.** On `sample:20`, `55`,
  `61`, `146` and `154`, agent-samples rise from ~0–100k to 30k–143k, and early stops come later
  (`sample:55` mean termination tick 440 → 1,360; `sample:146` 490 → 1,010).
- **Reading.** Spared kin keep worlds that used to lock up or die early alive and dense for longer.
  The lighter configs (`165`, `197`, `151`) are dense on every arm and change little in roster.
- **Atlas.** It costs about a minute on every arm.

## 9. What this does not show

- **The satiation mechanism, measured directly.** §6 is inference. An in-tree control is needed:
  `growth_retention_multiplier` raised, or the gate reading satiation against something other than
  maintenance.
- **Why the producer cohort collapses under recognition** on `sample:31` (§3). The resident differs
  at injection (less free nutrient, longer-lived resident heterotrophs). Both sides fork their own
  resident, as in #605, so a common-state comparison is not available.
- **Recognition at other `D` or `r`.** Everything here is at `D = 0.5`, `r = ½`,
  `satiation_sensitivity = 0.1`.
- **One config and 8 seeds for invasion.** Several cells are decided by extinction ties at
  `r = −0.00277`.
- **The baseline's founder placement.** The baseline is unpinned, but it predates #601 and founds
  uniformly, the same as the pinned arms.

## 10. Recommendations

The orchestrator and owner have agreed the next steps.

1. **#612, the founder nutrient floor, first.**
2. **Then a quick `sample:31` experiment.** Override `satiation_sensitivity` (e.g. 1.0) and
   `recognition_distance` (e.g. 1.0–1.5), and re-run the three `reinvasion_barrier` modes with this
   readout. This tells whether these are **scale** problems, fixable by putting both parameters
   into #607's search box, or a **structural** one.

**This note's view: expect a structural component.**
- **`satiation_sensitivity`.** Raising it to 1.0 moves half expression to 1 tick, inside the 1–5
  tick band where killers sit. It should cut kin kills sharply if §6 is right. If it does, the
  "scale" fix works.
  - But it works by making every agent nearly sated at the retention buffer. That damps *all*
    consumption, and with it the heterotrophy a consumer guild needs.
  - A gate whose yardstick (reserve against maintenance) is pinned by the grow phase's retention
    buffer cannot separate "fed" from "hungry" at any sensitivity. It can only move the cut-off
    relative to a quantity that barely varies.
- **`recognition_distance`.** Raising it to 1.0–1.5 should cover the 20–40 % of births beyond
  `D = 0.5`. At 1.4 it reaches the producer–consumer separation the world rules use to say
  recognition does not shelter prey. Watch whether a raised `D` starts protecting the steps' prey
  as well as their kin.
- **Reading the experiment.**
  - If kin kills at `d < D` stay hungry and frequent at `c = 1.0`, the problem is structural. The
    satiation yardstick against the retention buffer is a design question for the world rules
    (§6).
  - If they vanish *and* the steps invade, both parameters belong in #607's box.
  - **Read the killers' satiation bands, not just `r`.**

## 11. Instrument

- **`crates/explorers-search/src/grazer_hunger.rs`** (new): `PreStep`, `Satiation`, `GrazerHunger`
  and `TraitDistances`, with tests:
  - the replay reproduces the stepper's living drains bit for bit (`sample:31`, 120 ticks);
  - `Satiation`'s expression equals the stepper's need gate;
  - the banding and merging behave as specified;
  - parent–offspring distances are banded with the right mean and rms.
- **`role_diet.rs`.** `DietLedger::ingest` takes the tick's `PreStep` and books each grazed death's
  grazers before any death is attributed, so a grazer that died the same tick is still on the
  ledger. `SeedDiet` gains `kills` and `births`, serde-defaulted, so earlier rows read back. A test
  pins:
  - at least one pair per grazed death and a kin pair per kin-grazed death;
  - birth distances are read;
  - the result is deterministic.
- **`energy_accounting.rs`.** `EnergyAccount` gains `killers`, serde-defaulted. The reconciliation
  helper asserts a killer for every grazed-to-death member. A new test on a tight `sample:31`
  cohort pins kin killers.
- **`role_diet_census`** pools and prints the killers and births tables. **`reinvasion_barrier`**
  prints a killers table after the accounting tables.
- **Artifacts**, under `target/606/` in each worktree:
  - `rb-{plain,accounting,dispersal}.{json,md}`;
  - `rd-atlas.jsonl` and its summary;
  - `rd-s31.jsonl` and its summary;
  - `rd-sample.jsonl` and its summary; on the recognition arm, `rd-sample-merged.jsonl` and
    `rd-sample-merged-summary.md`.
