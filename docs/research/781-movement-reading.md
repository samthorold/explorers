# Issue #781: the movement rule's pre-registered reading, at 1× and 10× litter

**Status: measurement (2026-10-10). #779's reading was run as pre-registered on the #772 mesocosm (decay 0.60, #770 wear defaults) under #780's movement rule (world rules, *Foraging movement follows what a resource releases*). No threshold, decay, movement rule, founder phenotype, litter default or other physics was changed.**

## Verdict, by the fixed rule

| Arm | Prediction | Result | Row that applies |
|---|---|---|---|
| 1× litter (the committed mesocosm) | Founders still die out in at least 3 of 5 seeds. Mean founder income over ticks 5–20 rises above today's ~0.05 | Founders die out in **5 of 5** seeds (the last by tick 24). Mean founder income over ticks 5–20 is **0.119–0.159** per living founder-tick | **The prediction holds.** Neither failure clause applies: founders do not persist, so #764 does not close on movement alone, and income rose, so the implementation is not flagged as wrong |
| 10× litter (`--litter-x 10`) | At least 20 of 81 founders alive at tick 50 in at least 3 of 5 seeds | 20 or more alive at tick 50 in **2 of 5** seeds (23 and 22; the others 18, 16, 16) | **The prediction fails: "Grill the aim weight again"** |

The result is not ambiguous under the table. The two readings of "die out" (below) agree at 1×, and so do the two readings of income.

## Method

Pre-registered in #779 and restated in #781:
- **Runs.** Each seed ran paired, as #772 did: `reference_mode --seed S --clear-at 1500 --ticks 1500`. It settles for 1,500 ticks, forks into a control and a perturbed arm, and runs each to tick 3,000. The 10× arm adds `--litter-x 10`. Seeds 1–5, release build, everything else at its default: the committed `mode1.json`, decay 0.60, and #770's wear.
- **Founders** are the agents at the pure heterotroph vertex at founding, tracked by id: the 81 decomposer founders, one per cell.
- **Founders alive at tick T** counts those ids still living at the world's tick T. Ticks 20, 50 and 100 all fall in the settle, before the fork, so the two arms share them. Seeds are read from the control arm, as in #772. The perturbed arm gives the same counts.
- **Founders die out** means no founder, by id, is alive at the end of the run. This follows #772's usage ("the pure-vertex founders' line died out within 50 ticks"). Under the by-id definition a long-lived line could still lose every founder to senescence, so the founders' line is reported beside it: the founders plus every agent born to a parent in the line. At 1× both are extinct in every seed, so the reading does not depend on which is used.
- **Income** is energy *gained*: the structure each founder drain took, times the trophic efficiency the stepper applied to that drain. A `Consumed` event's `energy_delta` is the structure drained, before efficiency (`764-founder-death-diagnosis.md`, *A reading trap*). The readout recomputes the applied efficiency exactly, as `trophic_transfer_efficiency(founder traits, target traits)`, from the traits of the carcass or agent the event names, as they stood before the step. Carcasses created in a step are not drained until the next, so this is the efficiency the stepper used. A unit test pins it against an independent full-log read.
- **Mean founder income over ticks 5–20** is the energy the founders gained in the steps ending at ticks 5 through 20 (16 steps), divided by the founders alive as each of those steps began: income per living founder-tick. That matches the diagnosis's "a founder's sustained income". The table also gives the same gain spread over all 81 founders for all 16 ticks, with the dead counted at 0. Both clear 0.05 in every seed, so the verdict does not depend on which is used.
- **"Today's ~0.05"** is the pre-#780 figure from the #764 diagnosis, measured by its since-deleted probe on the same mesocosm at decay 0.60. The comparison is against that figure, as pre-registered. No same-code baseline was run on `11aa051`.
- **The litter multiplier.** `--litter-x N` stands N × 17 carcasses per cell, each the size of a 1× carcass, so the cell holds N × 54 energy and N × 58 nutrient. The extra litter is placed after the founders, so at a given seed the founders stand in the same places at 1× and 10×. At 1× the world is the committed mesocosm, carcass for carcass.

## Results

The control arm's readout, from each run's `.md` summary (*Founders* table). Income is per living founder-tick over ticks 5–20, with the structure drained in brackets. "Per founder" spreads the same gain over all 81 founders. "Last death" is the tick the last founder died. "Line at 3000" is the founders' line alive at the end of the control arm.

| arm | seed | alive at 20 | alive at 50 | alive at 100 | alive at 3000 | last death | income 5–20: gained (drained) | per founder | line at 3000 | runtime |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1× | 1 | 1 | 0 | 0 | 0 | 22 | 0.119 (0.371) | 0.066 | 0 | 255 s |
| 1× | 2 | 1 | 0 | 0 | 0 | 24 | 0.139 (0.421) | 0.082 | 0 | 236 s |
| 1× | 3 | 2 | 0 | 0 | 0 | 21 | 0.159 (0.488) | 0.092 | 0 | 230 s |
| 1× | 4 | 1 | 0 | 0 | 0 | 24 | 0.129 (0.402) | 0.074 | 0 | 199 s |
| 1× | 5 | 2 | 0 | 0 | 0 | 24 | 0.150 (0.445) | 0.089 | 0 | 203 s |
| 10× | 1 | 57 | **18** | 2 | 0 | 129 | 0.548 (1.741) | 0.499 | 69 | 1,186 s |
| 10× | 2 | 62 | **23** | 1 | 0 | 105 | 0.588 (1.858) | 0.544 | 19 | 321 s |
| 10× | 3 | 62 | **16** | 1 | 0 | 116 | 0.607 (1.922) | 0.542 | 0 | 1,209 s |
| 10× | 4 | 51 | **16** | 0 | 0 | 97 | 0.536 (1.698) | 0.483 | 0 | 979 s |
| 10× | 5 | 55 | **22** | 2 | 0 | 131 | 0.583 (1.845) | 0.540 | 8 | 706 s |

**Row assignment.**
1. **1×.** Founders are extinct by tick 24 in 5 of 5 seeds, which is at least 3, so they "still die out". Mean income over ticks 5–20 is 0.119–0.159, above ~0.05 in every seed. Both parts of the prediction hold.
2. **10×.** Founders alive at tick 50 are 18, 23, 16, 16 and 22. Seeds 2 and 5 reach 20, which is 2 of 5, short of the 3 the prediction needs. The prediction fails, and the table's failure clause is *grill the aim weight again*.

## Supplementary (outside the pre-registration)

- **Gained over drained is 0.31–0.32 in every run.** That is flow 7's efficiency for a founder on producer litter, `0.78 · exp(−0.60 · 1.52) ≈ 0.31`. Reading the drain as income would have overstated income about threefold, which is the reading trap.
- **The 10× founders outlive the 1× founders by about 100 ticks.** The last 10× founder dies at ticks 97–131, against 21–24 at 1×. No founder, by id, lives to tick 3,000 in either arm.
- **The founders' line at 3,000 under 10× litter.** Seeds 1, 2 and 5 have 69, 19 and 8 agents of the founders' line alive in the control arm. The line counts descent through either parent, so it includes offspring of sexual births with a producer-line mate. Heterotroph-dominant agents alive at 3,000 in those controls are 0, 19 and 0 (seed 2's world had collapsed to 13 producers). So this is not a persisting pure-vertex line, except perhaps in seed 2's control.
- **The decomposer reading from #772 is unchanged in shape.** Income-role decomposers persist in every arm (1–24 at tick 3,000). Agents with autotrophy ≥ 0.1 take 0.986–0.990 of the whole-horizon carcass drain at 1× and 0.899–0.931 at 10×, and all of it after the fork in every run.

## Runtime

The ten paired runs (4,500 ticks each) ran five at a time on 8 cores, release build. The first minutes overlapped one `cargo test --workspace`. At 1×, each run took 199–255 s. At 10×, each took 321–1,209 s. The 10× runs' cost tracks how long the extra litter stands, not the roster, and a run is roughly 3–5 times a 1× run. A 200-tick 10× pilot (seed 1) took 32 s, against 1 s at 1×: the early ticks, with 13,770 standing carcasses, cost about thirty times as much.

## Reproducing

```sh
cargo build --release -p explorers-reference-modes --bin reference_mode
# per arm and seed (x = 1 or 10, s = 1..5), five at a time:
target/release/reference_mode --seed $s --clear-at 1500 --ticks 1500 --litter-x $x \
  --out target/781/runs/x$x-s$s.json
```

Each `.md` summary beside its JSON carries the *Founders* table per arm: founders alive at ticks 20, 50 and 100 and at the end, the line alive at the end, and mean income over ticks 5–20 as energy gained, with the drain beside it.
