# Issue #624: does the surplus gate sate the killers and keep consumers reproducing?

**Status: measurement; the gate fails, so #607 does not start. Adds two readouts to
`reinvasion_barrier` and `role_diet_census` (commit `02e2590`): the killing grazers banded by their
pre-growth surplus expression, and reproduction (births and earmark fill) by diet group. Both are
observer-only: every table the tools printed before is identical with and without them.
`explorers-sim`, the search, the evaluator and the prefilter are unchanged.**

#621 settled satiation as the surplus above the retention buffer, co-limited by free nutrient and
read before growth. #622 set its default sensitivity `c = 33` from the light-fed mixotrophs' surplus,
and #623 put the gate into the stepper. This slice measures the gate against the three conditions
the issue sets for starting #607, with #619's default cell as the post-#612 baseline.

## TL;DR

**Verdict: FAIL.** Condition 1 passes, condition 2 fails narrowly for `step 2×`, and condition 3
fails clearly. The orchestrator returns to design.

| condition | bar | measured at the defaults (c = 33, D = 0.5) | verdict |
|---|---|---|---|
| 1. Light-fed killers sated | most `step` kin killers on `sample:31` at E < 0.5 | `step 1×` 68 %, `step 2×` 62 % (#619: most held under 1 tick, hungry) | **pass** |
| 2. Infant kin-grazing per birth halves | ≤ 0.405 / ≤ 0.455 (#619: 0.81 / 0.91) | 0.27 / 0.46 | **fail** (`step 2×` misses by 0.004) |
| 3. Consumers keep reproducing | heterotroph-by-diet earmark fill and births do not fall against c = 0 | atlas earmark fill falls 18× (`founder_aggregation = 0`) and 39× (decoded); births per agent-sample fall 4.6× and 38× | **fail** |

1. **The light-fed killers are sated, but they still kill.** About two thirds of the steps' kin
   killers sit past half expression (E < 0.5), against a mostly hungry population in #619. They
   express little, but a newborn's structure is so small that even a low expressed drain kills it.
   The other third have no surplus at all (s = 0, E = 1).
2. **Infant kin-grazing per birth falls by 67 % for `step 1×` and 49.6 % for `step 2×`.** At `D = 1.0`
   both halve (0.15 / 0.18). With the gate off (`c = 0`) it is 0.96 / 0.98.
3. **The gate starves the consumers' earmark.** On the atlas, agents that live as heterotrophs by
   diet are 8× (`founder_aggregation = 0`) to 20× (decoded) more numerous under the gate. But their
   mean earmark fill per sample falls from 1.33 to 0.073 and from 2.22 to 0.057. Under the gate,
   54–58 % of consumers by income sit at zero surplus, against 15–16 % ungated. The gate shuts as soon
   as a consumer holds any surplus, and the grow phase fills the earmark from that surplus.
4. **Nothing invades by the #587 criterion,** in any of the 8 (c, D) cells or the 3 modes. This is
   not part of the gate.
5. **What it implies.** `c = 1 / s₂₅⁺` was calibrated on light-fed mixotrophs, whose surplus is small
   and whose photosynthesis the gate does not touch. The same `c` applies to diet-fed heterotrophs,
   whose only route to a surplus is the drain the gate closes. One gate scale cannot serve both (§6).

## 1. What ran

| | |
|---|---|
| Tree | First sweep: `main` at `ff61a81` (after #623). Readout rerun: `02e2590` (this branch). Release builds. |
| Founder aggregation | `sample:31` pinned to 0 (`--founder-aggregation 0`), as #605/#606/#619. Atlas both at 0 and as decoded. |
| Grid | `c` ∈ {0, 11, 33, 99} × `D` ∈ {0.5, 1.0}. (33, 0.5) is the default, and unset flags reproduce it. `c = 0` is the ungated control. 11 and 99 are ⅓× and 3× the default. |
| `reinvasion_barrier` | `sample:31`, seeds 1000–1007 (8/8 reach tick 1000 in every cell), injection at tick 1000, window 1000, cohort 8. Plain (#587), `--accounting` (#591) and `--dispersal` (#593) modes. |
| `role_diet_census` | Atlas (`atlas.json`, 95 cells), seeds 1000–1004, T = 2000, `EvalConfig::default()`, at the default `c` and at `c = 0`. |
| Machine | 8-core laptop, one process at a time. |

The first sweep ran every cell. The readout rerun repeated the accounting and dispersal modes at
`D = 0.5` (unset, `c` ∈ {0, 11, 99}) and all four atlas passes. `D = 1.0` killer bands were
therefore not read. Every table the first sweep printed is reproduced line for line by the rerun.

**Commands** (cwd is the repo root; drivers `target/624/run.sh` and `target/624/run2.sh`):

```sh
cargo build --release -p explorers-search --bin reinvasion_barrier --bin role_diet_census

# sample:31: MODE in "" --accounting --dispersal; C in 0 11 33 99; D in 0.5 1.0
./target/release/reinvasion_barrier sample:31 $MODE --founder-aggregation 0 --out target/624/rb-unset-<mode>.json
./target/release/reinvasion_barrier sample:31 $MODE --founder-aggregation 0 \
    --satiation-sensitivity $C --recognition-distance $D --out target/624/rb-c$C-d$D-<mode>.json

# atlas: FAFLAG in "" "--founder-aggregation 0"; CFLAG in "" "--satiation-sensitivity 0"
C=$(seq -s, -f 'atlas:%g' 0 94)
./target/release/role_diet_census --atlas atlas.json --configs "$C" $FAFLAG $CFLAG --out target/624/v2/rd-atlas-fa<0|decoded>-c<default|0>.jsonl
./target/release/role_diet_census --summary --out target/624/v2/rd-atlas-<…>.jsonl
```

**Wall clock.** The laptop idle-slept and ran under swap pressure during both sweeps, so identical
runs took 12 s or about 1,000 s (see *A sweep that stops producing rows may be asleep* in
`docs/agents/instrument-runtimes.md`). The long figures are sleep, not work. The CPU-realistic
figures are from the runs that were not interrupted:

| run | time |
|---|---:|
| `reinvasion_barrier` plain / accounting / dispersal | 19–21 s / 12–15 s / 20–25 s |
| `role_diet_census`, atlas, decoded (either c) | 37–42 s |
| `role_diet_census`, atlas, `founder_aggregation = 0`, `c = 0` | 78 s |
| `cargo test --workspace` (with build) | 404 s; 896 pass, 0 fail |

Other runs logged 166–2,906 s (`target/624/times.log`, `target/624/v2/times.log`).

**Reproduction check.** Unset flags print exactly the (33, 0.5) cell's tables in all three modes.
The readout rerun reproduces the first sweep's `sample:31` tables and atlas summaries exactly; it
only adds the two new tables.

**The pins change the whole world.** As in #619, each cell's resident evolves under its own (c, D)
from tick 0. At injection the resident has 88–128 agents (114 at the default, 100 in #619's
baseline), and its control rate is −0.00029 to −0.00063 (−0.00045). Compare cells as whole worlds.

## 2. Condition 1: are the light-fed killers sated?

**Reading the condition.** "Sated" means a high surplus, which means reduced expression. "At or past
half expression" therefore means `E ≤ 0.5`: the gate is closed at least halfway. The new table's
`kin E ≥ 0.5` column counts the complement, killers still on the hungry side. The passing share is
the `E < 0.1` and `0.1–0.5` bands together. (The tables' header prose said the opposite: "E ≥ 0.5 =
at or past half expression". This branch corrects it to "E ≥ 0.5 = at most half gated (hungry side);
E < 0.5 = past half expression (sated side)". The column names are unchanged.)

Kin kill pairs (grazer and victim both lineage members), summed over 8 seeds, own dispersal,
banded by the grazer's expression `E = 1 / (1 + c·s)` at its pre-growth surplus:

| c (D = 0.5) | arm | kin pairs | E < 0.1 | 0.1–0.5 | 0.5–0.9 | ≥ 0.9 | **E < 0.5** | s = 0 |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| 0 | `step 1×` | 142 | 0 | 0 | 0 | 142 | 0 % | 2 % |
| 0 | `step 2×` | 196 | 0 | 0 | 0 | 196 | 0 % | 3 % |
| 11 | `step 1×` | 203 | 65 | 54 | 6 | 78 | 59 % | 37 % |
| 11 | `step 2×` | 91 | 29 | 9 | 11 | 42 | 42 % | 44 % |
| **33** | **`step 1×`** | 209 | 122 | 20 | 8 | 59 | **68 %** | 24 % |
| **33** | **`step 2×`** | 287 | 150 | 29 | 20 | 88 | **62 %** | 22 % |
| 99 | `step 1×` | 156 | 79 | 7 | 2 | 68 | 55 % | 42 % |
| 99 | `step 2×` | 179 | 97 | 4 | 6 | 72 | 56 % | 40 % |

- **Pass at the defaults.** 68 % and 62 % of the kin killers are past half expression. In #619, most
  of them held under one tick of reserve on the old yardstick and so were hungry.
- **`c = 0` passes nothing, trivially:** every killer expresses E = 1.
- **The killers split in two.** Most kill from deep in the gate (E < 0.1). Nearly everyone else holds
  no surplus at all (s = 0, E = 1). Few sit between. Raising `c` to 99 does not deepen the gate (E < 0.1:
  51–54 % of kin pairs, against 52–58 % at `c = 33`); it raises the s = 0 share (22–24 % → 40–42 %).
- **Sated killers still kill.** A killer at E < 0.1 drains under a tenth of its capability, but a
  newborn's structure is small enough that this still kills it. Past half expression, recognition spares an
  identical target completely but a less similar one only partly, so these kills fall on kin that
  differ in trait.

**On the atlas** (all configs; `E < 0.5` = the two lowest bands over all kin pairs):

| | kin pairs | E < 0.5 | s = 0 | c = 0: kin pairs, s = 0 |
|---|---:|---:|---:|---:|
| `founder_aggregation = 0` | 80,783 | 42 % (29,086 + 4,667) | 55 % | 56,495, 8 % |
| decoded | 32,749 | 32 % (8,848 + 1,668) | 63 % | 23,309, 8 % |

The atlas does not enter the condition, but it shows the same split more strongly: most kin killing
on the atlas is by grazers with no surplus. Ungated, only 8 % of kin killers hold none.

## 3. Condition 2: infant kin-grazing per birth

Infant grazed deaths (≤ 50 ticks) per birth, `step 1×` / `step 2×`, own dispersal, uniform
placement (`--dispersal` mode). #619's default cell was 0.81 / 0.91, so the bar is ≤ 0.405 /
≤ 0.455.

| c \ D | 0.5 | 1.0 |
|---|---|---|
| 0 | 0.96 / 0.98 | 0.92 / 0.82 |
| 11 | 0.45 / **0.28** | **0.26 / 0.40** |
| **33** | **0.27** / 0.46 | **0.15 / 0.18** |
| 99 | **0.21 / 0.35** | **0.17 / 0.34** |

(Bold: under the bar.)

- **Fail at the defaults, narrowly.** `step 1×` falls 67 % (40 infant grazed deaths over 149 births).
  `step 2×` falls 49.6 %, to 0.459 (83 over 181), just above the 0.455 bar.
- **The gate does the work.** Ungated (`c = 0`), infant kin-grazing is near total (0.82–0.98), worse
  than #619's baseline. Every gated cell is at most about half the ungated figure.
- **The response is not monotone in `c`.** At `D = 0.5`, `c = 11` halves `step 2×` but not `step 1×`,
  and `c = 33` the reverse. `c = 99` passes both. At `D = 1.0`, every `c ≥ 11` passes both. Each cell
  rests on 8 seeds and 50–180 births, so differences of a few hundredths are noise. The defaults' 0.46
  against a 0.455 bar is a miss within that noise.
- **Kin-grazed energy** per member-tick at the defaults is 0.0026 / 0.0037, against 0.0065 / 0.0148
  ungated and 0.0038 / 0.0071 in #619's default cell.

## 4. Condition 3: do consumers keep reproducing?

**Tolerance.** A fall counts if it lies outside the seed-to-seed spread: the bootstrap 95 % interval
over the 475 seeds of the pooled mean earmark fill. Here the gated and ungated intervals are far
apart, so the verdict does not depend on the tolerance chosen.

**The atlas** ("Reproduction by diet group (#624)", all configs). Births are booked to each parent's
diet group at the birth. Earmark fill is the energy the grow phase moved into the reproductive
earmark in the step before each second-half agent-sample, as a mean per sample.

| | births, c = 33 / c = 0 | agent-samples | births per sample | mean earmark fill (bootstrap 95 %) |
|---|---|---|---|---|
| **heterotrophs by diet, `founder_aggregation = 0`** | 137 / 80 | 6,271 / 787 | 0.022 / 0.102 | **0.073** [0.052, 0.100] / **1.325** [0.79, 2.08] |
| **heterotrophs by diet, decoded** | 22 / 43 | 5,214 / 267 | 0.004 / 0.161 | **0.057** [0.036, 0.085] / **2.224** [0.75, 4.73] |
| light-fed heterotrophs, `founder_aggregation = 0` | 28,138 / 15,468 | 55,227 / 49,263 | 0.51 / 0.31 | 2.28 / 2.21 |
| light-fed heterotrophs, decoded | 18,111 / 6,685 | 47,981 / 24,152 | 0.38 / 0.28 | 2.06 / 2.26 |
| trait producers, `founder_aggregation = 0` | 165,189 / 130,203 | 495,488 / 447,551 | 0.33 / 0.29 | 1.53 / 1.53 |
| trait producers, decoded | 69,625 / 62,189 | 182,331 / 171,106 | 0.38 / 0.36 | 1.48 / 1.42 |

- **Fail.** Heterotrophs by diet fill their earmark 18× less at `founder_aggregation = 0` and 39×
  less decoded. Births per agent-sample fall 4.6× and 38×. Decoded, absolute births halve (43 → 22).
  At `founder_aggregation = 0` they rise (80 → 137), because 8× more agents live as heterotrophs by
  diet. Live configs only agree (fill 0.075 against 1.35, and 0.057 against 2.50).
- **More agents live as consumers, and barely reproduce.** Realised consumer-diet samples rise to
  3,902 from 135 (`founder_aggregation = 0`) and to 3,394 from 39 (decoded). Seeds with any
  heterotroph-by-diet sample rise from 96 to 151 and from 44 to 108. Seeds where such agents give any
  birth go from 22 to 39 and from 11 to 14. No config holds a consumer or decomposer guild either way.
- **Light-fed agents reproduce more under the gate.** Light-fed heterotrophs give 1.8–2.7× more
  births, and producers give 12–27 % more. Their earmark fill does not move.
- **Why: the gate pins consumers at zero surplus.** By recent income, consumers sit at `s = 0` on 58 %
  of samples (`founder_aggregation = 0`) and 54 % (decoded) under the gate, against 16 % and 15 %
  ungated. Their median surplus is 0 against 3.0 and 0.64 ticks. The grow phase fills the earmark
  from the reserve it mobilises above the buffer, which is exactly the surplus the gate reads. A
  consumer's only income is the drain, which closes as soon as a surplus appears, so its surplus
  cannot build. A light-fed agent's photosynthesis is not gated, so its surplus builds regardless.
  This matches #622's warning: at `c = 33` a consumer at its median surplus (0.8 ticks) expresses
  about 3 %.

**`sample:31`'s resident consumers** (control fork, agents of role consumer or decomposer at tick
1000; dispersal-mode accounting):

| cell | seeds with any | member-ticks | earmark fill | births |
|---|---:|---:|---:|---:|
| c = 33, D = 0.5 (default) | 6 | 382 | 0.018 | 1 |
| c = 0, D = 0.5 | 1 | 788 | 0.016 | 0 |
| c = 0, D = 1.0 | 2 | 553 | 0.214 | 5 |
| c = 11, D = 0.5 / 1.0 | 3 / 4 | 62 / 748 | 0.041 / 0.034 | 1 / 2 |
| c = 33, D = 1.0 | 6 | 452 | 0.031 | 2 |
| c = 99, D = 0.5 / 1.0 | 3 / 5 | 160 / 80 | 0.019 / 0.036 | 0 / 1 |

At `D = 0.5` there is no fall against `c = 0`, and no signal either: the ungated resident has one
seed with consumers and no births. At `D = 1.0` the ungated cell's 0.21 fill and 5 births against
0.03 and 2 at `c = 33` point the same way as the atlas, on two seeds. The resident does not decide
condition 3; the atlas does.

## 5. Invasion (#587; not part of the gate)

**No arm invades in any cell or mode.** Plain mode has 10 arms × 8 cells, and no sign-test interval
clears zero. At the defaults, the best plain step arm is `step 4×` / uniform: median −0.00069, 1/8 seeds
positive. The founder heterotroph (`full`) dies out on 8/8 seeds in every cell, at a median tick of 8–12.

In dispersal mode at the defaults, the `step 1×` dispersal-2 and dispersal-4 arms reach medians of
+0.00066 and +0.00069 (6/8 seeds positive), and `step 2×` dispersal 4 reaches +0.00026 (6/8). Their
intervals still reach −0.00277. Ungated (`c = 0`), every plain step arm is at −0.00139 or below with
0/8 seeds positive.

## 6. Verdict and what it implies

**Gate: FAIL.**
- **Condition 1 passes:** 68 % / 62 % of the steps' kin killers are past half expression.
- **Condition 2 fails narrowly:** `step 1×` 0.27 passes; `step 2×` 0.46 misses the 0.455 bar.
- **Condition 3 fails clearly:** heterotrophs by diet fill their earmark 18–39× less than ungated,
  and their births per sample fall 4.6–38×.

Per the issue, #607 does not start, and the orchestrator returns to design. The evidence for that
design session:

- **One gate scale serves two populations with opposite needs.** `c = 1 / s₂₅⁺` was calibrated on
  light-fed mixotrophs, whose positive surplus is small (`s₂₅⁺` 0.03–0.07 ticks), one to two orders below
  the 0.6–3 ticks ungated consumers hold at their median, so their gate shuts at a tiny surplus. That is the intent: their
  heterotrophy is supposed to be latent. The same `c` applies to diet-fed heterotrophs. For them,
  any surplus closes the drain that is their only income, so they hover at `s = 0` and the grow
  phase has nothing above the buffer to put into the earmark. Raising or lowering `c` moves one
  cut-off for both.
- **The calibration is not a fixed point.** Under the gate, the census's own `1 / s₂₅⁺` reads 10.7
  (`founder_aggregation = 0`) and 8.3 (decoded), against 15.9 and 43.3 ungated. The mixotrophs'
  surplus moves with the gate, as #622 §5 predicted.

Options worth a grill-with-docs, with what the data say about each:

- **(a) A lower `c`.** The `c = 11` cell bears on this. Condition 2 is split there (`step 2×` 0.28
  halves, `step 1×` 0.45 does not), and condition 1 is weaker (59 % / 42 %). Condition 3 was not
  measured at `c = 11`. Because the mechanism is that any surplus closes the gate, a lower `c` only
  raises the surplus a consumer can hold before its drain halves: from 0.03 ticks to 0.09. That is
  still far below the 0.6–3 ticks ungated consumers hold at their median. Nothing here suggests a
  `c` that passes condition 3 without losing condition 1.
- **(b) Scale surplus per agent by its own reproductive need** rather than by maintenance, for
  example the gap to its reproduction threshold or earmark target. A consumer short of its earmark
  would then read as hungry while it still holds a surplus over maintenance. A light-fed mixotroph
  whose photosynthesis fills its earmark would read as sated. This targets the failing mechanism
  directly. It was not measured here.
- **(c) Gate only living targets, and leave carcass intake ungated.** The data show carcass intake
  is gated today and falls with `c`: `step 2×`'s carcass gain is 0.0133 E per member-tick at `c = 0`,
  0.0048 at `c = 33` and 0.0024 at `c = 99`. Kin-grazing is a drain on living targets only, so
  ungating carcasses would not touch conditions 1–2. Whether carcass income alone would refill
  consumers' earmarks is not measured. The heterotrophs by diet include decomposers, and their
  fill is low too.

## 7. What this does not show

- **No LHS draw.** The atlas result on condition 3 is unambiguous, so the LHS pass (about 2 h) was
  not run. If wanted:

  ```sh
  ./target/release/role_diet_census --founder-aggregation 0 --out target/624/rd-lhs-fa0.jsonl
  ./target/release/role_diet_census --founder-aggregation 0 --satiation-sensitivity 0 --out target/624/rd-lhs-fa0-c0.jsonl
  ./target/release/role_diet_census --summary --out target/624/rd-lhs-fa0.jsonl
  ```

  Split `--configs` over two processes so `sample:20` does not hold the rest
  (`docs/agents/instrument-runtimes.md`).
- **Condition 3 on the atlas is at two `c` values only** (33 and 0). The `c` = 11 and 99 cells were
  read only on `sample:31`, whose resident consumers are too few to decide it.
- **No killer bands at `D = 1.0`.** The readout rerun covered `D = 0.5` only.
- **One config and 8 seeds for conditions 1 and 2.** Many invasion cells are decided by extinction
  ties at `r = −0.00277`.
- **Diet groups are lifetime diet, roles are recent income.** The reproduction table uses the
  census's lifetime diet groups (the deaths table's groups). The surplus rows use the evaluator's
  recent-income roles (#599).

## 8. Instrument

Commit `02e2590`, in `explorers-search`:

- **Killer expression bands.** `PreStep::killer_readings` reads each killing grazer's satiation off
  the grown roster (the #606 band, unchanged) and its surplus off the metabolised roster, with
  `E = 1 / (1 + c·s)` at the run's `c`. `GrazerHunger` gains expression bands (E < 0.1, 0.1–0.5,
  0.5–0.9, ≥ 0.9) and a zero-surplus count per kinship. Both are serde-defaulted, so old rows read
  back. `reinvasion_barrier` and `role_diet_census` print "Killing grazers at the pre-growth surplus
  read (#624)".
- **Reproduction by diet group.** `SeedDiet` gains a reproduction table: births booked to each
  parent's diet group at the birth (one `diet_group` classifier, shared with the deaths table), and
  the mean earmark fill per second-half agent-sample (`PreStep::earmark_fills`: the reproductive
  reserve after growth less before it, the accounting's earmark fill).
- **Tests:** `grazer_hunger_bands_pairs_by_surplus_expression`,
  `grazer_hunger_reads_back_without_expression_bands`,
  `killer_readings_read_surplus_on_the_pre_growth_roster`,
  `earmark_fills_are_the_grow_phase_s_repro_share_of_mobilised_surplus`,
  `ledger_books_births_by_the_parent_s_diet_group`,
  `reproduction_by_diet_means_earmark_fill_over_samples`, `rollout_records_reproduction_by_diet_group`.
- **This commit** corrects the expression table's header prose (and the matching doc comments in
  `grazer_hunger.rs`): `E ≥ 0.5` is the hungry side of half expression, not past it. No column name
  or test-pinned string changed.
- **Artifacts:** `target/624/` (first sweep: `rb-{unset,cC-dD}-{plain,accounting,dispersal}.{json,txt}`,
  `rd-atlas-fa{0,decoded}-c{default,0}{.jsonl,-summary.txt}`, `times.log`, `run.sh`) and
  `target/624/v2/` (readout rerun, `workspace-tests.txt`, `times.log`; driver `target/624/run2.sh`).
