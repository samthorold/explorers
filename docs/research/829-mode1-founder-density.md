# Issue #829: the pre-registered founder density reading for mode 1

**Status: done (2026-10-13). `n` = 5 passes, but the prediction fails in its specialist half, and follow-up 2 fires.** By #814's combined rule, `n` = 20 (0 of 5 seeds) and 10 (0 of 5) fail, and `n` = 5 passes (3 of 5). Producers persist in all 15 runs. Founders do not persist at 20, as predicted: all are dead by tick 17. They persist at 10 and at 5, in 5 of 5 seeds each. But drainers below autotrophy 0.1, the specialists, take none of the arm's carcass drain at any point, as in #814, #822 and #826. By the rule, **follow-up 2 fires at `n` = 10 and 5: founders persist, but the guild fails (10) or holds without specialists (5). Recruitment is the limit, and reproduction is grilled next.** Follow-ups 1, 3 and 4 do not fire. Founder income ran above the pre-registered arithmetic, at about 0.45, 1.04 and 2.2 times upkeep against 0.4, 0.8 and 1.6. At founding it matches the arithmetic; the excess builds over the first few ticks. Every founder still dies, by tick 92 at `n` = 10 and by tick 558 at 5, even while earning more than its founding upkeep. The reports do not say why. No default changed.

## The question

#826 found that 81 founders share the founding stand's saturated yield, so each earns about a quarter of its upkeep at any reach. The stand's litter ratio at one founder a cell is 22, about the domain's peak; the domain founds at about 170 or more ([fungi](../ecology/fungi.md), *Decomposer biomass against its supply*; CONTEXT.md, **Litter ratio**). This reading sets the number of decomposer founders by the litter ratio, with body reach at the value where their catchments tile the stand on average, and #814's four rules on and the starvation cascade off. The pre-registration is mode 1's *Founder density* bullet in `docs/system-design/reference-modes.md` (PR #828), agreed by the owner on 2026-10-13. The design is the *Decomposer founders* bullet.

## The option

`reference_mode --decomposer-founders N` (commit 4eca3dd) seeds `N` decomposer founders, at least 1, at uniform random positions like every founder, in place of one a cell (81). The founding litter is placed before the founders, and producers come first in the roster, so producers and litter stand where they do at the default. Without the flag the world is unchanged, state for state, and nothing is recorded; with it, the summary header and the artifact record `N`. It refuses 0 and `--no-decomposers`. The tests cover the count honoured, the default world unchanged for the same seed, producers and litter unmoved by the count, and the CLI flag.

**The conversion does not depend on the count.** `--body-reach-pieces` reads the founders' mean effective heterotrophy and structure, which are the recipe's at the heterotroph vertex whatever their number, and the litter density the run builds. So `k` still counts pieces at the founders' mean, and the reports record the same `h` (1.0734), `S` (2.4588) and `ρ` (0.17 per unit area) at every point.

## The runs

Release build of `reference_mode` at 4eca3dd, frozen as a copy for the sweep. Every run is paired, at 1× litter, with both starvation-cascade switches off:

```sh
reference_mode --seed $s --clear-at 1500 --ticks 1500 \
  --size-scaled-base-rate --established-producer-structure 6.937 --living-resistance \
  --litter-half-life 1 --enzyme-half-saturation 1 \
  --decomposer-founders $n --body-reach-pieces $k \
  --out runs/n${n}_s${s}/report.json
```

`n` = 20, 10 and 5, seeds 1–5: 15 paired runs, on a 90 × 90 torus (area 8,100) of 81 nutrient cells.

**The conversion.** The founding stand holds 81 × 17 = 1,377 carcasses, 81 × 54 = 4,374 structure. Founders tile it on average when each one's reach covers `A / n`: reach `r = √(8,100 / (π n))`. The litter in that reach is `π r² ρ = 8,100 × 0.17 / n = 1,377 / n` pieces, so `k = 1,377 / n`. The litter ratio is the stand's structure against the founders' combined structure, `L = 4,374 / (2.4588 n)`. At the tiling reach it equals #826's `R = k × 3.1765 / 2.4588`, since one founder's catchment is then the stand's share per founder. `k` was passed directly, to two decimals:

| `n` | `L` | k (pieces) | body reach coefficient `b` | founder reach at founding |
|---:|---:|---:|---:|---:|
| 20 | 88.9 | 68.85 | 5.927 | 11.354 |
| 10 | 177.9 | 137.7 | 8.721 | 16.057 |
| 5 | 355.8 | 275.4 | 12.673 | 22.708 |

No point clamped. The reach at `n` = 5 is half the torus's width.

**The pilot.** `n` = 5, seed 1, as a full paired run, the point expected to cost most. It took 2,344 s of wall-clock and 1,977 s of CPU (33 CPU-minutes), well under the two-hour bound. The owner waived the stop-and-report for this sweep and approved `caffeinate -i`, so the pilot ran concurrently with the rest: it started alone, and the other 14 runs followed seconds later, five at a time beside it. It is one of the 15 runs.

**The readout.** As in #826, every figure is the control arm's. Founders die long before the clearing at tick 1,500, so the perturbed arm's founder readout is the same; its death ticks are identical in all 15 runs (checked).
- Guild verdict, fewest members, share of carcass drain and living share come from `report.control.guild`, over the arm's second half (ticks 2,250–3,000).
- The **specialist share** is 1 − the mixotroph share of `arm_carcass_drain`, the carcass drain over the arm (ticks 1,500–3,000), split at autotrophy 0.1. The **competition share** is that mixotroph share.
- Founders alive, the founder line alive at the end, and income over ticks 5–20 (gained, with drained in brackets, per living founder-tick) come from the summary's *Founders* table. Death ticks and per-tick income come from `report.control.founders`.
- Upkeep is the size-scaled decomposer founder's at founding, 0.1987, from the *Size-scaled base rate* table.
- `S_p` is the median established producer structure over the control's second half.
- Runtime is `/usr/bin/time -p`: wall-clock (`real`), with CPU (`user`) in brackets.

**The analysis.** `scripts/analyze.py` reads the per-run reports. For each point it counts seeds where the guild passes, holds and producers persist; seeds with a founder alive at tick 50; and seeds with a specialist share above 0.5. A point *passes*, founders *persist* and specialists *take most of the drain* each at 3 of 5 seeds. Follow-up 1 fires if `n` = 5 passes with specialists taking most of the drain. Follow-up 2 fires where founders persist and the guild fails or specialists do not take most of the drain. Follow-up 3 fires if founders do not persist at `n` = 5. Follow-up 4 names any point and seed whose producers did not persist.

## The verdict, by the fixed rule

| `n` | point passes | guild holds | producers persist | founders persist (seeds with one alive at 50) | specialists take > half (seeds) | median specialist share | median guild share | `S_p` median (range) |
|---:|---|---|---|---|---|---:|---:|---|
| 20 | no (0/5) | 0/5 | 5/5 | no (0/5) | no (0/5) | 0.000 | 0.024 | 7.64 (7.20–8.11) |
| 10 | no (0/5) | 0/5 | 5/5 | **yes (5/5)** | no (0/5) | 0.000 | 0.025 | 8.22 (7.76–8.97) |
| 5 | **yes (3/5)** | 3/5 | 5/5 | **yes (5/5)** | no (0/5) | 0.000 | 0.179 | 7.79 (7.74–8.30) |

- **The guild.** It holds only at `n` = 5, in seeds 3, 4 and 5, with fewest members 10, 6 and 8 and shares of the arm's second-half carcass drain of 0.297, 0.179 and 0.284. Its fewest members are 0 in every other run. Its share is 0.000–0.027 at `n` = 20 and 0.000–0.070 at 10.
- **Competition.** The mixotroph share is 1.000 in every run. `arm_carcass_drain.pure_vertex` is exactly 0.0 in both arms of all 15 runs.
- **The check.** The guild's living drain is 0 in every run, so its living share is 0.000 wherever it drained (at `n` = 20 seed 5 and `n` = 10 seeds 2 and 5 it drained under 2 structure).
- **Producers.** They persist in all 15 runs, with 658–870 at the control's end.
- **Founders.** At `n` = 20 all die at ticks 14–17, so none is alive at tick 20. At `n` = 10, 3–6 of 10 are alive at tick 20, 1–2 at tick 50 and none at 100; the last dies at tick 64–92. At `n` = 5 all five are alive at tick 50 in every seed, and 4–5 at tick 100; the last dies at tick 328–558.

## The prediction and its result

*At `n` = 5, founders persist, the point passes, and specialists take more than half the carcass drain. At `n` = 20, founders do not persist.*

- **`n` = 20 holds.** Founders do not persist (0 of 5).
- **`n` = 5 holds in two parts and fails in the third.** Founders persist (5 of 5), and the point passes (3 of 5). But the specialist share is 0.000 in every seed.

So the prediction fails, in its specialist half. `n` = 10 carries none: founders persist there, and the point fails.

## What follows, by the rule

1. **`n` = 5 passes and specialists take most of the drain: does not fire.** `n` = 5 passes, but specialists take none of the drain. The guild that holds is not the specialists' (*Who the guild is*, below). Setting the decomposer founders' count by the litter ratio is not recommended on this reading.
2. **Founders persist, but the guild fails or holds without specialists: fires**, at `n` = 10, where founders persist and the guild fails (0 of 5), and at `n` = 5, where founders persist and the guild holds without specialists. **Recruitment is the limit, and reproduction is grilled next:** the nutrient earmark a founder fills only by draining (#822), the timing of reproduction, and why the founder line leaves the heterotroph vertex. What these reports can say about it is below.
3. **Founders do not persist at `n` = 5: does not fire.** They persist in 5 of 5 seeds.
4. **Producers collapse at any point: does not fire.** They persist in all 15 runs.

These are recorded, not acted on. No grill has run, no issue has been filed, and no default changed.

## For follow-up 2: what the reports show about recruitment

The reports record each founder's death tick, the founders' summed income and drain per tick while any lives, the founder line alive at the end, the world's heterotroph-dominant count at the end, and carcass drain split at autotrophy 0.1 over the whole run and over the arm. They do not record births or their parents, the line's size over time, its members' traits, the cause of any death, or per-founder income.

### When the founders died, and why at `n` = 10

Income per living founder-tick, as a multiple of founding upkeep (0.1987), by window of ticks:

| `n` | 0–5 | 5–20 | 20–50 | 50–100 | 100–200 | 200–300 | 300–400 | 400–558 |
|---:|---|---|---|---|---|---|---|---|
| 20 | 0.40–0.45 | 0.41–0.48 | - | - | - | - | - | - |
| 10 | 0.86–1.01 | 0.99–1.09 | 1.35–1.44 | 1.32–1.39 | - | - | - | - |
| 5 | 1.97–2.07 | 2.16–2.28 | 2.15–2.22 | 2.07–2.28 | 2.30–2.62 | 3.52–4.46 | 4.94–7.34 | 7.51–9.05 |

**At `n` = 10 the founders die in two groups.** 4–8 of the 10 die at ticks 15–25 (7, 4, 8, 7 and 6 by seed), the same time as every founder at `n` = 20 (14–17) and #826 (14). The mean income over ticks 5–20 is about upkeep (0.99–1.09 times), so roughly half the founders earned less than upkeep; random placement leaves some founders on more-shared litter than others. The readout gives only the mean, so this is inferred from the timing, not read per founder. The survivors then earn 1.32–1.44 times founding upkeep through tick 100, and still die, at ticks 30–92. So the later deaths at `n` = 10 are not explained by income against founding upkeep.

**At `n` = 5 the founders also die above upkeep.** The first death is at tick 53–133 and the last at 328–558, while income per living founder runs at 2–9 times founding upkeep, rising as neighbours die and producer litter accumulates. Their lifespans bracket the established producers' median lifespan (311–346 ticks in the control's second half).

**The reports cannot say why founders above upkeep die.** The candidates, none tested here: death by age, whose hazard is the senescence hazard (0.022) times the agent's wear and runs beside death by loss of function (CONTEXT.md, *Death by age*); upkeep that grows as a founder grows, since 0.1987 is the founding body's; the nutrient a founder needs, which it takes only by draining (#822's earmark); and the cost of reproducing.

At `n` = 20 the founders die at ticks 14–17 on 0.41–0.48 of upkeep, against #826's tick 14 on 0.22–0.25. Nearly doubling income below upkeep moved the death tick by at most 3 ticks. The reports do not explain that either.

### Whether the founders reproduced

Births and their parents are not in the reports. The founder line at the end is: the founders and every living agent with a parent in the line. With every founder dead, a line alive at the end means at least one birth had a founder parent. So founders reproduced in at least 8 of 15 runs: `n` = 20 seeds 1, 2 and 4 (line 40, 21, 62), `n` = 10 seeds 1, 2 and 4 (37, 13, 40) and `n` = 5 seeds 1 and 4 (28, 26). Even at `n` = 20, where they lived 14–17 ticks on under half their upkeep, the line survives in 3 of 5. In the other 7 runs the line is 0 at the end; whether the founders never reproduced or their line died out is not recorded.

**The line leaves the heterotroph vertex again.** Wherever it survives, it outnumbers all the world's heterotroph-dominant agents at the end (40 against 2, 21 against 6, 62 against 2 at `n` = 20; 37 against 10, 13 against 0, 40 against 24 at 10; 28 against 4 and 26 against 13 at 5). So at least 13–60 of the line's members, by run, have autotrophy at or above their heterotrophy, as #826 found. Whether by mutation or by crossing with producers (a birth joins the line if either parent is in it) is not recorded.

### When the pure-vertex drain stopped

`carcass_drain − arm_carcass_drain` is the drain over ticks 0–1,500. Over that window, drainers below autotrophy 0.1 took 0.003–0.006 of the carcass drain at `n` = 20, 0.006–0.009 at 10 and 0.052–0.278 at 5. The founders are among them, and their own drain is in the founder readout, so the rest is drain by other pure-vertex agents, presumably the founders' offspring (the readout does not tag drains by lineage):

| `n` | founders' own drain | other pure-vertex drain, ticks 0–1,500 |
|---:|---|---|
| 20 | 74–89 | 8–17 |
| 10 | 191–272 | 2–38 |
| 5 | 1,912–4,489 | 7–5,959 (by seed: 3,353, 5,959, 329, 1,501, 7) |

- At `n` = 20 and 10 the founders' pure-vertex offspring drained almost nothing: 82–99 % of the pre-clearing pure-vertex drain is the founders' own.
- At `n` = 5 they drained thousands in seeds 1, 2 and 4, and little in 3 and 5. The guild failed in seeds 1 and 2 and held in 3, 4 and 5, so where pure-vertex offspring drained most the guild did not hold. With five seeds this is an observation, not a finding.
- From tick 1,500 on, no agent below autotrophy 0.1 drained anything, in either arm of any run. When between the last founder's death and tick 1,500 the pure-vertex drain stopped is not recorded.

### Who the guild is at `n` = 5, seeds 3–5

The guild predicate (`is_guild_member`) is the decomposer role with heterotrophy above autotrophy, with no autotrophy threshold. The arm's pure-vertex drain is 0, so every guild member that drained has autotrophy of at least 0.1 and below its heterotrophy. **The guild here is heterotroph-dominant mixotrophs, not specialists,** as in #826. In seeds 3 and 5 the founder line is 0 at the end, so that guild is not the founders' descendants. In seed 4 the line is 26 against 13 heterotroph-dominant agents, so whether the guild descends from the founders is not established.

### What a reproduction grill would need to read

The reports leave open the questions follow-up 2 hands on. Births by parent and tick, the line's size and traits over time, each founder's own income and cause of death, and drain by lineage are all observable in the stepper's event log (the observer already logs births and the line), but none is serialised. This is recorded for the grill, not filed.

## Income against the pre-registered arithmetic

The pre-registration's budget: a founder breaks even at 0.1987 / 0.3144 = 0.632 drained a tick (assimilation 0.3144 in every run). Alone on a carcass at `c = K` it drains `k_d × 3.1765 / 2` = 0.00481 a tick, so a lone founder breaks even on about 131 carcasses, the pre-registration's "about 135". At the tiling reach a lone founder would earn 0.52, 1.05 and 2.09 times upkeep at `n` = 20, 10 and 5. Random overlap was taken to cost about a fifth, giving 0.4, 0.8 and 1.6.

For scattered founders at the tiling reach, the other founders covering a carcass are Poisson with mean 1, and under saturating decomposition a founder's share of a carcass with `j` others is `k_d S / (2 + j)`. Its mean is 0.736 of the lone founder's: overlap costs about a quarter, not a fifth. The predicted drain per founder is then 0.244, 0.487 and 0.974 a tick.

| `n` | predicted drain, random overlap | measured drain, step 0 | measured drain, ticks 5–20 | measured income / upkeep, step 0 | measured, ticks 5–20 | pre-registered |
|---:|---:|---:|---:|---:|---:|---:|
| 20 | 0.244 | 0.239–0.262 | 0.259–0.302 | 0.38–0.41 | 0.41–0.48 | 0.4 |
| 10 | 0.487 | 0.478–0.563 | 0.624–0.684 | 0.76–0.89 | 0.99–1.09 | 0.8 |
| 5 | 0.974 | 1.029–1.118 | 1.369–1.437 | 1.63–1.77 | 2.16–2.28 | 1.6 |

**At founding the arithmetic holds,** within −2 % to +16 % of the random-overlap drain, which ignores producer litter and producer-line drainers. **The excess builds over the first few ticks:** drain per living founder rises 8–15 % from step 0 to ticks 5–20 at `n` = 20, 16–32 % at 10 and 22–36 % at 5. The reports do not say why. The candidates are founders moving toward less-shared litter, founders growing (reach and capability scale with structure), and producer litter added to the stand. Neighbours' deaths cannot explain it at `n` = 5 or 20, where no founder dies before tick 53 and 14. So overlap at founding costs about what the stepper predicts, and the founders then recover part of it.

## Observations beyond the rule, not verdicts

- **The perturbed arm's guild holds less often.** At `n` = 5 it holds only in seed 5 (fewest 13); seeds 3 and 4 fall to 2. The rule reads the control arm, as #814, #822 and #826 did.
- **`S_p` drifted up,** to 7.20–8.97 against #814's 6.937, as in #826. `b` was not re-iterated.
- **Fewer founders with more reach drain more of the stand early, and its pure-vertex share before the clearing rises.** It is 0.05–0.28 at `n` = 5, against 0.03–0.30 at #826's `R` = 1,400 with 81 founders.

## Runtime

15 paired runs, release build, on 8 cores: the pilot first and then the other 14 through `xargs -P 5` beside it, six at a time, under `caffeinate -i` (approved by the owner), on mains power. The sweep ran from about 17:17 to 18:38 on 2026-10-13: about 81 minutes of wall-clock and 21,892 s of CPU (6.1 CPU-hours). Per run, CPU (wall-clock):
- `n` = 20: 840–1,083 s (923–1,174 s). 4,794 s of CPU for the five.
- `n` = 10: 1,461–1,723 s (1,651–1,959 s). 7,791 s.
- `n` = 5: 1,699–2,089 s (2,008–2,468 s). 9,306 s.

Wall-clock was 1.05–1.19 times CPU in every run, with no sleep, against #826's 1.7–1.8 in its first wave and 4.8–11.8 later, uncaffeinated and on a draining battery. CPU tracks reach, not the founder count: `n` = 10 at k = 137.7 costs about what #826's `R` = 170 (k = 131.6, 81 founders) did. A `report.json` is about 1.5 MB.

## Reproducing

The frozen binary, scripts and per-run reports are under `target/829-sweep/` (not committed). To rebuild:

```sh
cargo build --release -p explorers-reference-modes --bin reference_mode
mkdir -p target/829-sweep && cp target/release/reference_mode target/829-sweep/
sh target/829-sweep/scripts/run.sh 5 1 &   # the pilot
# one paired run per line "n seed"; k = 68.85, 137.7, 275.4 for n = 20, 10, 5
xargs -P 5 -L 1 sh target/829-sweep/scripts/run.sh < target/829-sweep/jobs.txt
```

`run.sh N SEED` maps `n` to k, skips a run whose `report.md` exists, and runs the command above under `/usr/bin/time -p` into `runs/n${N}_s${SEED}/`. `analyze.py` does the per-run extraction and per-point rule described under *The readout* and *The analysis*; with the argument `seeds` it prints the core of the per-seed table below. The income windows, death ticks and pure-vertex drain split were read from the same `report.json` files.

## Appendix: per point and seed

The control arm. The columns:
- Guild share is the guild's share of all carcass drain over the arm's second half.
- Specialist share is the share of the arm's carcass drain taken by drainers below autotrophy 0.1.
- Living share is the guild's living drain over its total.
- Producers in brackets are the control's at its end.
- Founders alive are counts of the `n` founders; deaths are the first and last founder death ticks.
- Line is the founder line alive at the end; heterotroph-dominant is the world's count at the end.
- Pre-clearing specialist share is the share of carcass drain over ticks 0–1,500 taken by drainers below autotrophy 0.1, and the next column splits that drain into the founders' own and the rest.
- Income is per living founder-tick over ticks 5–20, gained (drained).
- Runtime is wall-clock, with CPU in brackets.

| `n` | seed | guild holds (fewest) | guild share | specialist share | living share | producers persist (at end) | pass | founders alive 20 / 50 / 100 | founder deaths | line at end | heterotroph-dominant at end | pre-clearing specialist share | pre-clearing pure-vertex drain: founders / other | income 5–20: gained (drained) | upkeep | decomposers at end | `S_p` | reach | runtime s (CPU) |
|---:|---:|---|---:|---:|---:|---|---|---|---|---:|---:|---:|---|---|---:|---:|---:|---:|---:|
| 20 | 1 | no (0) | 0.027 | 0.000 | 0.000 | yes (870) | no | 0 / 0 / 0 | 14–16 | 40 | 2 | 0.005 | 74 / 17 | 0.081 (0.259) | 0.199 | 4 | 7.197 | 11.35 | 1,052 (960) |
| 20 | 2 | no (0) | 0.024 | 0.000 | 0.000 | yes (753) | no | 0 / 0 / 0 | 14–16 | 21 | 6 | 0.004 | 86 / 8 | 0.093 (0.296) | 0.199 | 5 | 7.636 | 11.35 | 1,174 (1,083) |
| 20 | 3 | no (0) | 0.001 | 0.000 | 0.000 | yes (743) | no | 0 / 0 / 0 | 14–17 | 0 | 0 | 0.006 | 89 / 17 | 0.095 (0.302) | 0.199 | 4 | 8.086 | 11.35 | 923 (840) |
| 20 | 4 | no (0) | 0.026 | 0.000 | 0.000 | yes (832) | no | 0 / 0 / 0 | 14–15 | 62 | 2 | 0.003 | 83 / 8 | 0.090 (0.287) | 0.199 | 0 | 8.109 | 11.35 | 1,076 (1,018) |
| 20 | 5 | no (0) | 0.000 | 0.000 | 0.000 | yes (858) | no | 0 / 0 / 0 | 14–16 | 0 | 0 | 0.005 | 80 / 9 | 0.087 (0.277) | 0.199 | 2 | 7.479 | 11.35 | 942 (893) |
| 10 | 1 | no (0) | 0.063 | 0.000 | 0.000 | yes (708) | no | 3 / 2 / 0 | 17–75 | 37 | 10 | 0.009 | 209 / 38 | 0.205 (0.653) | 0.199 | 2 | 7.906 | 16.06 | 1,760 (1,505) |
| 10 | 2 | no (0) | 0.000 | 0.000 | 0.000 | yes (749) | no | 6 / 2 / 0 | 15–64 | 13 | 0 | 0.009 | 272 / 6 | 0.215 (0.684) | 0.199 | 0 | 8.220 | 16.06 | 1,709 (1,499) |
| 10 | 3 | no (0) | 0.025 | 0.000 | 0.000 | yes (692) | no | 5 / 2 / 0 | 17–92 | 0 | 1 | 0.009 | 240 / 3 | 0.211 (0.670) | 0.199 | 4 | 8.772 | 16.06 | 1,819 (1,603) |
| 10 | 4 | no (0) | 0.070 | 0.000 | 0.000 | yes (783) | no | 5 / 1 / 0 | 16–64 | 40 | 24 | 0.006 | 191 / 20 | 0.206 (0.653) | 0.199 | 4 | 7.764 | 16.06 | 1,959 (1,723) |
| 10 | 5 | no (0) | 0.000 | 0.000 | 0.000 | yes (658) | no | 6 / 1 / 0 | 16–84 | 0 | 4 | 0.007 | 210 / 2 | 0.196 (0.624) | 0.199 | 0 | 8.966 | 16.06 | 1,651 (1,461) |
| 5 | 1 | no (0) | 0.000 | 0.000 | 0.000 | yes (794) | no | 5 / 5 / 4 | 53–550 | 28 | 4 | 0.236 | 4,405 / 3,353 | 0.433 (1.376) | 0.199 | 0 | 7.785 | 22.71 | 2,344 (1,977) |
| 5 | 2 | no (0) | 0.022 | 0.000 | 0.000 | yes (708) | no | 5 / 5 / 4 | 71–487 | 0 | 2 | 0.278 | 3,321 / 5,959 | 0.447 (1.421) | 0.199 | 6 | 7.735 | 22.71 | 2,055 (1,738) |
| 5 | 3 | yes (10) | 0.297 | 0.000 | 0.000 | yes (762) | yes | 5 / 5 / 5 | 112–550 | 0 | 21 | 0.085 | 2,634 / 329 | 0.452 (1.437) | 0.199 | 17 | 7.748 | 22.71 | 2,468 (2,089) |
| 5 | 4 | yes (6) | 0.179 | 0.000 | 0.000 | yes (709) | yes | 5 / 5 / 5 | 133–558 | 26 | 13 | 0.168 | 4,489 / 1,501 | 0.431 (1.369) | 0.199 | 23 | 8.299 | 22.71 | 2,008 (1,699) |
| 5 | 5 | yes (8) | 0.284 | 0.000 | 0.000 | yes (725) | yes | 5 / 5 / 5 | 116–328 | 0 | 11 | 0.052 | 1,912 / 7 | 0.433 (1.377) | 0.199 | 15 | 8.162 | 22.71 | 2,134 (1,804) |
