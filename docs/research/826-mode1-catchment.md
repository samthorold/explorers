# Issue #826: the pre-registered catchment reading for mode 1

**Status: done (2026-10-13). Two of three points pass, but the prediction fails in its second half, and follow-up 3 fires.** By #814's combined rule, `R` = 170 fails (1 of 5 seeds), and `R` = 500 (4 of 5) and 1,400 (5 of 5) pass. Producers persist in all 15 runs. But drainers below autotrophy 0.1, the specialists, take none of the arm's carcass drain at any point. The guild that holds is made of heterotroph-dominant mixotrophs at autotrophy 0.1 or more. Every founder is dead at tick 14 in every run, so founders do not persist at `R` ≥ 500. By the rule, **follow-up 3 fires: catchment is not the binding cause, and the founder budget is diagnosed before anything else is grilled.** Follow-ups 1, 2 and 4 do not fire. The diagnosis is below. The founders' catchments overlap, so all 81 founders share the founding stand's saturated yield, and that caps a founder's income at about a quarter of its upkeep at any reach. A founder's income moves by 11 % while `R` grows 8-fold. No default changed, and no code changed.

## The question

#814 and #822 found the specialist's litter income 3 to 40 times short of its upkeep at every reach up to 30 pieces. #822's founders survived on the earmark, but no guild formed. Mode 1's *Feeding reach* bullet gives the domain's magnitude for a saprotroph's catchment as a ratio: about 170 to 1,400 times its own biomass in litter. The 30 pieces read so far are `R ≈ 39`. This reading sets body reach by that ratio, at the domain's ends and in between, with #814's four rules on and the starvation cascade off. The only change from #814 is reach. The pre-registration is mode 1's *Catchment* bullet in `docs/system-design/reference-modes.md` (PR #825), agreed by the owner on 2026-10-12.

## The runs

Release build of `reference_mode` at main after #825 (commit 9ad02df), frozen as a copy for the sweep. Every run is paired, at 1× litter, with both starvation-cascade switches off:

```sh
reference_mode --seed $s --clear-at 1500 --ticks 1500 \
  --size-scaled-base-rate --established-producer-structure 6.937 --living-resistance \
  --litter-half-life 1 --enzyme-half-saturation 1 --body-reach-pieces $k \
  --out runs/R${R}_s${s}/report.json
```

`R` = 170, 500 and 1,400, seeds 1–5: 15 paired runs. Each run seeds 81 decomposer founders, at the pure heterotroph vertex, on a 90 × 90 torus of 81 nutrient cells.

**The conversion.** `R` counts the founding stand's litter in reach per unit of a founder's structure at founding. A carcass on the founding stand holds 54/17 = 3.1765 structure, and a founder holds `S` = 2.4588. So `R = k × 3.1765 / 2.4588 ≈ 1.292 k`, and `k = R × 2.4588 / 3.1765`. No `R` option was added. `k` was passed directly, rounded to 0.1:

| `R` | k (pieces) | body reach coefficient `b` | founder reach at founding |
|---:|---:|---:|---:|
| 170 | 131.6 | 8.507 | 15.70 |
| 500 | 387.0 | 15.174 | 26.92 |
| 1,400 | 1,083.7 | 25.944 | 45.05 |

`--body-reach-pieces` converts k by flow 3's reach (`h` 1.0734, contact range coefficient 1.2836, litter density `ρ` 0.17 per unit area). No point clamped. The reach at `R` = 1,400 is half the torus's width, as the pre-registration expected.

**The pilot.** Seed 1 at `R` = 1,400, 300 ticks, unforked, took 18.8 s of wall-clock (18.7 s CPU), alone. The guild held over ticks 150–300 (fewest 9, share of carcass drain 0.907), and producers persisted. Drainers below autotrophy 0.1 took 0.815 of the carcass drain over the 300 ticks. A paired run steps 4,500 ticks, so a straight-line projection gave about 5 minutes. Allowing for the carcass pile's growth, the estimate was 5–15 minutes a run, well under the two-hour bound, so the sweep went ahead without consulting the owner. That estimate was 3–10 times short (*Runtime*, below). A second probe, seed 6 at `R` = 1,400 for 600 ticks, unforked and outside the sweep's seeds, took 106 s (93 s CPU). Its guild held over ticks 300–600 (fewest 12, share 0.894), and drainers below autotrophy 0.1 took 0.899 of the carcass drain over the 600 ticks.

**The readout.** Every figure is the control arm's, as in #814 and #822. Founders die long before the clearing at tick 1,500, so the perturbed arm's founder readout is the same, and its death ticks are identical in all 15 runs (checked).
- Guild verdict, fewest members, share of carcass drain and living share come from `report.control.guild`, over the arm's second half (ticks 2,250–3,000).
- The **specialist share** is 1 − the mixotroph share of `arm_carcass_drain`, the carcass drain over the whole arm (ticks 1,500–3,000), split at autotrophy 0.1. The **competition share** in #814's sense is that mixotroph share.
- Founders alive, the founder line alive at the end, and income over ticks 5–20 (gained, with drained in brackets, per living founder-tick) come from the summary's *Founders* table.
- Upkeep is the size-scaled decomposer founder's, from the *Size-scaled base rate* table.
- `S_p` is the median established producer structure over the control's second half.
- Runtime is `/usr/bin/time -p`: wall-clock (`real`), with CPU (`user`) in brackets.

**The analysis.** It is read from the per-run reports. For each point it counts seeds where the guild passes, holds and producers persist, and seeds with a founder alive at tick 50. It also counts seeds with a specialist share above 0.5, and takes the nearest-rank median of the specialist and guild shares. A point *passes*, founders *persist* and specialists *take most of the drain* each at 3 of 5 seeds. The prediction is then read at `R` = 500 and 1,400. Follow-up 1 fires if at least 2 points both pass and have specialists taking most of the drain. Follow-up 2 fires where founders persist and the guild fails. Follow-up 3 fires where `R` ≥ 500 and founders do not persist. Follow-up 4 names any point and seed whose producers did not persist.

## The verdict, by the fixed rule

| `R` | point passes | guild holds | producers persist | founders persist (seeds with one alive at 50) | specialists take > half (seeds) | median specialist share | median guild share | `S_p` median (range) |
|---:|---|---|---|---|---|---:|---:|---|
| 170 | no (1/5) | 1/5 | 5/5 | no (0/5) | no (0/5) | 0.000 | 0.086 | 7.88 (7.54–8.20) |
| 500 | **yes (4/5)** | 4/5 | 5/5 | no (0/5) | no (0/5) | 0.000 | 0.157 | 7.44 (6.90–8.62) |
| 1,400 | **yes (5/5)** | 5/5 | 5/5 | no (0/5) | no (0/5) | 0.000 | 0.191 | 8.05 (7.58–8.95) |

**2 of 3 points pass, which is "many".**
- **The guild.** It holds in 1, 4 and 5 seeds by `R`. Where it holds, its fewest members are 5–17. Its share of the arm's second-half carcass drain is 0.000–0.205 at `R` = 170, 0.032–0.321 at 500 and 0.157–0.593 at 1,400. It rises with reach, but reaches half only in `R` = 1,400, seed 3.
- **Competition.** The mixotroph share is 1.000 in every run. Drainers below autotrophy 0.1 drained nothing in the arm, in either arm of any run, as in #814 and #822.
- **The check.** The guild's living share is 0.000 wherever it drained, and undefined in `R` = 170, seed 2, where it drained nothing. Living resistance spares the stand at every reach.
- **Producers.** They persist in all 15 runs, with 637–847 at the control's end.
- **Founders.** All 81 die at tick 14 in every run, so none is alive at tick 20, 50 or 100. The founder line, meaning the founders and every agent with a parent in it, is alive at the end of all 15 runs, with 13–175 agents.

## The prediction and its result

*At `R` = 500 and 1,400 the points pass, and specialists take more than half the carcass drain there.*

- **The first half holds.** Both points pass.
- **The second half fails.** The specialist share is 0.000 at both points, in every seed.

So the prediction fails. `R` = 170 carries none; it fails, 1 of 5.

## What follows, by the rule

1. **Many points pass and specialists take most of the drain: does not fire.** Many points pass, but specialists take none of the drain at any point. The guild that holds is not the specialists' (*Who the guild is*, below). Catchment is not shown to be the binding limit, and body reach anchored by the ratio is not recommended for adoption on this reading.
2. **Founders persist but the guild fails: does not fire.** Founders persist at no point.
3. **Founders do not persist at `R` ≥ 500: fires**, at `R` = 500 and 1,400 (0 of 5 seeds each). Catchment is not the binding cause after all. **The founder budget is diagnosed before anything else is grilled.** The data here already give that diagnosis (*The founder budget*, below).
4. **Producers collapse at any point: does not fire.** They persist in all 15 runs.

These are recorded, not acted on. No grill has run, no issue has been filed, and no default changed.

## The founder budget: catchments overlap

Founder income over ticks 5–20 is 0.044–0.045 gained a living founder-tick at `R` = 170, 0.047–0.048 at 500 and 0.049 at 1,400. Drained, it is 0.140–0.143, 0.151–0.153 and 0.156. Upkeep is 0.199 at every point. So income is 22–25 % of upkeep, and it moves by 11 % while `R` grows 8-fold and reach 2.9-fold. Assimilation is 0.31 of what is drained.

**What sets it.** Under saturating decomposition (world rules, *Carcass decomposition saturates in its decomposers*), a carcass yields `D = k_d · S · Σc / (K + Σc)` a tick, shared among its drainers in proportion to their capability. Every carcass yields at most `k_d · S`. The founding stand holds 81 × 54 = 4,374 structure, so it yields at most 0.003027 × 4,374 = 13.24 a tick, however far the founders reach. All 81 founders together drained, on their first step:

| `R` | reach | founders whose reach covers a point (`π r² × 81 / 8,100`) | saturation `n / (n + 1)` | predicted drain, all founders | measured, step 0 | measured, mean of steps 5–13 |
|---:|---:|---:|---:|---:|---:|---:|
| 170 | 15.70 | 7.7 | 0.886 | 11.73 | 11.52–11.61 | 11.34–11.54 |
| 500 | 26.92 | 22.8 | 0.958 | 12.68 | 12.66–12.67 | 12.17–12.39 |
| 1,400 | 45.05 | 63.8 | 0.985 | 13.04 | 13.03 | 12.59–12.61 |

The prediction takes each drainer at `c = K`, one founder's capability, and ignores producer litter and other drainers. It matches to within 2 %. So at every point the founders already drain the stand at 87–98 % of its ceiling. More reach only adds founders to each carcass, and they split the same yield.

**The ceiling.** Shared 81 ways, the stand's yield is at most 0.163 drained a founder-tick, or 0.051 gained at 0.31. That is 26 % of upkeep, at any reach. At saturation the stand could carry about 21 founders (13.24 × 0.31 / 0.199), not 81.

**Why the pre-registration expected break-even at `R` = 170.** The *Feeding reach* bullet's budget, about 0.0048 a carcass from about 135 carcasses, is a lone founder on each carcass: `k_d × 3.18 × c / (K + c)` with `c = K` is 0.0048. Its catchment counts the litter in one founder's reach. On the founding stand, though, each carcass at `R` = 170 is in about 7.7 founders' reach and pays each about `k_d × 3.18 / 8.7` = 0.0011. Across 131.6 carcasses that is 0.145 drained, which matches the 0.140–0.143 measured. The ratio that reaches a founder is `R / n`: 170 / 7.7, 500 / 22.8 and 1,400 / 63.8 are all about 22. That is the stand's litter per founder structure, 54 / 2.46 = 21.96, set by founder density and not by reach. It is about an eighth of the domain's lower end. CONTEXT.md's *Catchment* already says catchments overlap; the pre-registered `R` did not account for it.

**So the binding limit on the founders is founder density against the stand's saturated yield, not reach.** Below about 21 founders on this stand, or with about 3.9 times its litter for 81, a founder at saturation would break even. That is the founder budget diagnosis follow-up 3 asks for, read from these runs. What to do with it is for the next grill, not this reading.

## Who the guild is: specialist share against guild share

The guild takes 12–59 % of the arm's second-half carcass drain wherever it holds, while specialists take 0 %. These do not conflict, because the two readouts draw different lines. From `crates/explorers-reference-modes/src/mesocosm.rs`:
- **The specialist split** is `MIXOTROPH_AUTOTROPHY` = 0.1. Each carcass drain is booked as `pure_vertex` when the drainer's autotrophy (`photosynthetic_absorption`) is below 0.1, else as `mixotroph` (`book_carcass_drain`). `arm_carcass_drain` is that split over the arm.
- **The guild predicate** is `is_guild_member`: the decomposer role, and heterotrophy greater than autotrophy. It has no autotrophy threshold of its own (*Feeding reach*, *The reading*: "with no new threshold").

`arm_carcass_drain.pure_vertex` is exactly 0.0 in both arms of all 15 runs. So no agent with autotrophy below 0.1 drained any carcass from tick 1,500 on. Every guild member that drained in the arm had autotrophy of at least 0.1 and below its heterotrophy. **The guild here is heterotroph-dominant mixotrophs, not specialists.** This is the same membership #814 and #822 found for their small guild shares. Here those members hold a guild.

**When the specialists' drain stopped.** It stopped before the clearing. `carcass_drain − arm_carcass_drain` is the drain over ticks 0–1,500. Over that window, drainers below autotrophy 0.1 took 0.005–0.019 of the carcass drain at `R` = 170 (174–608 structure). At 500 they took 0.007–0.236 (248–8,810), and at 1,400 0.030–0.299 (1,047–10,453).
- At `R` = 170 that is about what the founders themselves drained before tick 14 (about 11.5 a tick for 14 ticks, about 160), so their pure-vertex offspring drained little.
- At `R` ≥ 500 pure-vertex agents drained thousands in some seeds, and in the pilots at `R` = 1,400 they took 0.82–0.90 of all carcass drain over the first 300–600 ticks.
- By tick 1,500 they drained nothing.

The reports do not say when between tick 600 and 1,500 it stopped, or why.

**Where the founder line went.** In every run the founder line at the end outnumbers all the heterotroph-dominant agents in the world: 13–175 in the line, against 0–34 heterotroph-dominant. In `R` = 1,400, seed 4, it is 175 against 25. So at least part of the line, 2–150 agents by run, has autotrophy at or above its heterotrophy. The line has moved up the autotrophy axis, by mutation or by crossing with producers (a birth joins the line if either parent is in it). The readouts do not record each guild member's lineage or traits. So whether the guild's members descend from the founders, or from producer-line mixotrophs that gained heterotrophy, is not established here.

## Observations beyond the rule, not verdicts

- **Reach helps the guild even without specialists.** Its share and how often it holds both rise with `R`, though every member is at autotrophy ≥ 0.1. Reach scales with effective heterotrophy, so a heterotroph-dominant mixotroph gains catchment from the body-reach coefficient too.
- **`S_p` drifted up.** Every run used `b` set at #814's `S_p` of 6.937. The control's `S_p` came out at 6.90–8.95, above 6.937 in 14 of 15 runs (`R` = 500, seed 1, at 6.900 is the exception). #822's neither and shrinking arms read 6.53–8.03. This reading did not iterate `b` to its own `S_p`, as #822 did not.
- **The founders die together.** All 81 die at tick 14 in every run, at every `R`. #814 saw tick 14–16 and #822's neither arm a median of 14–15. Reach changes their income by 11 % and their death tick not at all.
- **R = 170's one passing seed** (seed 3) held with fewest 5 and share 0.205. The other four have fewest 0–1.

## Runtime

15 paired runs, release build, six at a time on 8 cores through `xargs -P 6`, the `R` = 1,400 jobs first. The owner declined `caffeinate -i` for this sweep, and the laptop ran low on battery mid-sweep. The sweep ran from 22:27 on 2026-10-12 to 06:15 on 2026-10-13: 7 h 47 min (28,047 s) of wall-clock. It took 32,039 s of CPU (8.9 CPU-hours). Per run, CPU (wall-clock):
- `R` = 170: 1,142–1,556 s (8,930–14,383 s). 6,707 s of CPU for the five.
- `R` = 500: 1,869–2,616 s (4,714–12,817 s). 10,613 s.
- `R` = 1,400: 2,652–3,161 s (4,786–5,344 s). 14,719 s.

Wall-clock was 1.7–1.8 times CPU in the first wave (the five `R` = 1,400 runs and `R` = 500, seed 1). That is already above #822's 4–11 %, at the same parallelism. In the later runs it was 4.8–11.8 times CPU, which points to sleep or throttling. CPU, not wall-clock, is the cost to plan by. CPU grows with reach, 1.6 times from `R` = 170 to 500 and 1.4 times from 500 to 1,400, unlike #796, where reach collapsed the stand. A `report.json` is about 1.2 MB at `R` = 1,400.

**The pilot missed the cost by 3 to 10 times.** It projected 5–15 minutes a run from 300 ticks at 18.8 s. The `R` = 1,400 runs took 44–53 CPU-minutes. Early ticks are cheap because the carcass pile is still small. A 300-tick pilot doesn't sample the settled pile that sets a paired run's cost. #814's pilot made the same miss.

## Reproducing

The frozen binary, scripts and per-run reports are under `target/826-sweep/` (not committed). To rebuild:

```sh
cargo build --release -p explorers-reference-modes --bin reference_mode
mkdir -p target/826-sweep && cp target/release/reference_mode target/826-sweep/
# one paired run per line "R seed", R = 1400 first; k = 131.6, 387.0, 1083.7 for R = 170, 500, 1400
xargs -P 6 -L 1 sh target/826-sweep/scripts/run.sh < target/826-sweep/jobs.txt
```

`run.sh R SEED` maps `R` to k, skips a run whose `report.md` exists, and runs the command above under `/usr/bin/time -p` into `runs/R${R}_s${SEED}/`. `analyze.py` does the per-run extraction and per-point rule described under *The readout* and *The analysis*. With the argument `seeds` it prints the per-seed table below.

## Appendix: per point and seed

The control arm. The columns:
- Guild share is the guild's share of all carcass drain over the arm's second half.
- Specialist share is the share of the arm's carcass drain taken by drainers below autotrophy 0.1.
- Living share is the guild's living drain over its total, or "-" where it drained nothing.
- Producers in brackets are the control's at its end.
- Founders alive are counts of the 81 founders.
- Line is the founder line alive at the end.
- Income is per living founder-tick over ticks 5–20, gained (drained).
- Pre-clearing specialist share is the share of carcass drain over ticks 0–1,500 taken by drainers below autotrophy 0.1.
- Heterotroph-dominant is the world's count at the end.
- Runtime is wall-clock, with CPU in brackets.

| `R` | seed | guild holds (fewest) | guild share | specialist share | living share | producers persist (at end) | pass | founders alive 20 / 50 / 100 | line at end | heterotroph-dominant at end | pre-clearing specialist share | income 5–20: gained (drained) | upkeep | decomposers at end | `S_p` | reach | runtime s (CPU) |
|---:|---:|---|---:|---:|---:|---|---|---|---:|---:|---:|---|---:|---:|---:|---:|---:|
| 170 | 1 | no (1) | 0.097 | 0.000 | 0.000 | yes (763) | no | 0 / 0 / 0 | 76 | 8 | 0.006 | 0.044 (0.141) | 0.199 | 6 | 8.146 | 15.70 | 9264 (1556) |
| 170 | 2 | no (0) | 0.000 | 0.000 | - | yes (740) | no | 0 / 0 / 0 | 21 | 0 | 0.007 | 0.044 (0.140) | 0.199 | 3 | 7.880 | 15.70 | 8930 (1453) |
| 170 | 3 | yes (5) | 0.205 | 0.000 | 0.000 | yes (807) | yes | 0 / 0 / 0 | 25 | 17 | 0.005 | 0.044 (0.140) | 0.199 | 9 | 8.200 | 15.70 | 14383 (1355) |
| 170 | 4 | no (0) | 0.086 | 0.000 | 0.000 | yes (741) | no | 0 / 0 / 0 | 13 | 11 | 0.019 | 0.044 (0.141) | 0.199 | 5 | 7.542 | 15.70 | 13772 (1200) |
| 170 | 5 | no (0) | 0.000 | 0.000 | 0.000 | yes (751) | no | 0 / 0 / 0 | 17 | 0 | 0.006 | 0.045 (0.143) | 0.199 | 0 | 7.715 | 15.70 | 13483 (1142) |
| 500 | 1 | no (0) | 0.032 | 0.000 | 0.000 | yes (836) | no | 0 / 0 / 0 | 24 | 7 | 0.154 | 0.048 (0.153) | 0.199 | 6 | 6.900 | 26.92 | 4714 (2616) |
| 500 | 2 | yes (5) | 0.122 | 0.000 | 0.000 | yes (682) | yes | 0 / 0 / 0 | 62 | 11 | 0.007 | 0.048 (0.153) | 0.199 | 16 | 8.086 | 26.92 | 8938 (1869) |
| 500 | 3 | yes (10) | 0.321 | 0.000 | 0.000 | yes (660) | yes | 0 / 0 / 0 | 43 | 18 | 0.236 | 0.048 (0.153) | 0.199 | 15 | 8.618 | 26.92 | 9767 (2010) |
| 500 | 4 | yes (8) | 0.219 | 0.000 | 0.000 | yes (847) | yes | 0 / 0 / 0 | 59 | 17 | 0.015 | 0.047 (0.151) | 0.199 | 14 | 7.213 | 26.92 | 12087 (2128) |
| 500 | 5 | yes (5) | 0.157 | 0.000 | 0.000 | yes (772) | yes | 0 / 0 / 0 | 81 | 17 | 0.007 | 0.048 (0.152) | 0.199 | 32 | 7.438 | 26.92 | 12817 (1991) |
| 1400 | 1 | yes (7) | 0.173 | 0.000 | 0.000 | yes (760) | yes | 0 / 0 / 0 | 22 | 16 | 0.299 | 0.049 (0.156) | 0.199 | 17 | 7.579 | 45.05 | 5344 (3161) |
| 1400 | 2 | yes (8) | 0.191 | 0.000 | 0.000 | yes (775) | yes | 0 / 0 / 0 | 20 | 16 | 0.159 | 0.049 (0.156) | 0.199 | 10 | 7.583 | 45.05 | 5277 (3090) |
| 1400 | 3 | yes (17) | 0.593 | 0.000 | 0.000 | yes (664) | yes | 0 / 0 / 0 | 70 | 34 | 0.118 | 0.049 (0.156) | 0.199 | 27 | 8.048 | 45.05 | 4786 (2652) |
| 1400 | 4 | yes (7) | 0.257 | 0.000 | 0.000 | yes (637) | yes | 0 / 0 / 0 | 175 | 25 | 0.030 | 0.049 (0.156) | 0.199 | 24 | 8.673 | 45.05 | 4968 (2810) |
| 1400 | 5 | yes (9) | 0.157 | 0.000 | 0.000 | yes (689) | yes | 0 / 0 / 0 | 84 | 29 | 0.185 | 0.049 (0.156) | 0.199 | 23 | 8.951 | 45.05 | 5191 (3007) |
