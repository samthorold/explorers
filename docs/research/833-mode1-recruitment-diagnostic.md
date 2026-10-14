# Issue #833: the recruitment diagnostic for mode 1

**Status: done (2026-10-14). A plain diagnostic: no prediction, no rule, no verdict.** It re-reads #829's 15 paired runs, which are identical field for field, with an observer-only readout of the founder line. It answers #829's four open questions.
- **Why founders earning above upkeep die.** It is not one cause. At `n` = 20 and 10, 149 of 150 founders die on reserve. Each spends its founding reserve on growth and one reproductive attempt within its first 15 ticks, and keeps only a buffer of about three times upkeep. At 20 its income is below upkeep. At 10 its upkeep grows with its body (1.3–1.5 times founding) until it about equals income (0.91–1.07 times), and the buffer runs out. At `n` = 5, 23 of 25 die by age: the senescence hazard, at ages 53–558. Summed over their lives as lived, that hazard expects 18.0 such deaths. Their income is still 1.24–1.76 times their grown upkeep when they die.
- **When they reproduce.** 172 of the 175 founders make their first attempt at tick 3–15, from their founding reserve, not from income. At `n` = 20 and 10 that is the only attempt. At 5 later attempts come every 100 or so ticks, from tick 101 on. About two in three attempts leave no offspring. The nutrient earmark (#822) delays the first attempt only at `n` = 20. After it the energy earmark is the binding gate at every point. At 5 the nutrient earmark reaches 25–173 against its threshold of 1.
- **How the line leaves the heterotroph vertex: by crossing with producers.** Of 24 births that first take the line off the vertex, 22 have a parent outside the line, at autotrophy 0.86–1.63. Two are by mutation. The off-vertex members then breed among themselves. The line is mostly off the vertex by tick 50 at `n` = 20, by 50–100 at 10, and by 300–1,000 at 5.
- **When the pure-vertex drain stops, and whose drain the guild's is.** The line's last pure-vertex member dies at tick 22–25 at `n` = 20, at 64–92 at 10 and at 328–1,461 at 5. There is no pure-vertex drain in either arm of any run from tick 1,475 on. Of the guild at `n` = 5, seeds 3–5:
  - In seeds 3 and 5 it is not the founders'. The line is extinct by tick 791 and 328.
  - In seed 4 the line takes 0.44 of the carcass drain over ticks 2,250–3,000. It holds 4–12 heterotroph-dominant members against the guild's fewest of 6, at autotrophy 0.91–1.12, and they descend through crossings with producers.

Set beside the domain (*For the ecology read*), three of these run against [fungi](../ecology/fungi.md) and [trophic roles](../ecology/trophic-roles.md): the age-tied hazard, upkeep in proportion to the whole body, and crossing with producers. No default changed.

## The question

#829 fired its follow-up 2 at `n` = 10 and 5: founders persist, but the guild fails or holds without specialists, so recruitment is the limit, and reproduction is grilled next. Its reports could not say why founders earning above their founding upkeep die, when they reproduce, whether the nutrient earmark holds them back, or how their line leaves the heterotroph vertex (#829, *For follow-up 2*). The grill measures before it designs. This is that measurement, as mode 1's *Recruitment diagnostic* bullet in `docs/system-design/reference-modes.md` sets it out (PR #832). Its findings go to the ecology read (#831) and the reproduction grill. It makes no prediction and applies no rule.

## The slice

Commit c0a6d64 adds an observer-only `recruitment` readout to each arm's report, `report.control.recruitment` and `report.perturbed.recruitment`. It is always on when there are founders, with no flag and no stepper change.
- **`founders`**: each founder, every step it lives. The series are reserve, energy earmark (`repro_reserve`), nutrient earmark (`repro_nutrient`), free nutrient, structure, peak structure, mean wear over the three functional traits, upkeep, and its own income gained (after efficiency) and drained (before it). Upkeep is `metabolic_cost` as the step began. Entry `i` is the step ending at tick `first_tick + i + 1`, and the state is the founder's as that step began. At death, the tick and the cause.
- **`births`**: every birth with a parent in the founder line. Each records its tick, offspring and sexual or asexual; each parent's id, line membership, autotrophy and heterotrophy; and the offspring's autotrophy, heterotrophy and nutrient endowment.
- **`deaths`**: each line member's death. It records the id, tick, age, cause, autotrophy, heterotrophy and offspring count.
- **`carcass_drain`**: per sample interval of the arm, the carcass drain split into founder line or not, and pure vertex or not.

**The cause of death.** It is *senesced* if a `Senesced` event preceded the `Died`. It is *reserve* if unallocated reserve was at or below upkeep entering the step. Otherwise it is *structural*. Autotrophy is nominal `photosynthetic_absorption`, and *pure vertex* means autotrophy below 0.1, as `arm_carcass_drain` splits it. A birth joins the line if either parent is in it. Births, deaths and founder lives run from the world's founding in both arms, since paired arms share the settle's history. Six tests in `mesocosm::tests` cover the readout:
- the readout follows each founder tick by tick and leaves the rest of the report alone;
- the three causes are classified correctly;
- a line birth records both parents' membership;
- line deaths are recorded;
- the lineage split sums to the existing carcass drain.

**Upkeep.** The readout's upkeep is metabolic only. It is 0.1821 at founding. #829's 0.1987 adds the full-stride movement cost, 0.0165, from the *Size-scaled base rate* table. Ratios below are against the readout's own upkeep unless they name 0.1987.

## The runs

Release build of `reference_mode` at c0a6d64, frozen as `target/833-sweep/reference_mode`. These are #829's commands exactly: 15 paired runs.

```sh
reference_mode --seed $s --clear-at 1500 --ticks 1500 \
  --size-scaled-base-rate --established-producer-structure 6.937 --living-resistance \
  --litter-half-life 1 --enzyme-half-saturation 1 \
  --decomposer-founders $n --body-reach-pieces $k \
  --out runs/n${n}_s${s}/report.json
```

`n` = 20, 10 and 5 with `k` = 68.85, 137.7 and 275.4, seeds 1–5, paired. They ran through `xargs -P 5` under `caffeinate -i`; the owner approved `caffeinate` and waived the two-hour stop.

**The check: identity.** `scripts/identity.py` walks every field of #829's frozen reports (`target/829-sweep/runs/`) and compares it with the same path in the new report. It ignores only keys #833 adds. All 15 runs are identical. Every #829 figure stands, and the new readouts describe exactly #829's worlds.

**Cross-checks.** `analyze.py x` re-derives #829's numbers from the new readouts, and they agree:
- First and last founder death ticks agree in all 15 runs. They are 14–17 at `n` = 20; 17–75, 15–64, 17–92, 16–64 and 16–84 at 10; and 53–550, 71–487, 112–550, 133–558 and 116–328 at 5.
- The line reconstructed from births and deaths equals `line_alive_at_end` in every run.
- The founders' summed drain equals #829's *founders' own drain* (74–89, 191–272, 1,912–4,489).
- The arm's lineage intervals sum to `arm_carcass_drain` to within 1e-7.
- Founder death ticks are identical in both arms.

## The readout

All figures are the control arm's unless they say otherwise. The perturbed arm shares everything before tick 1,500, and its founders all die before then.
- **Reproductive attempts.** An attempt is a step in which the energy earmark is discharged. The stepper takes it whole, and a Poisson draw on fecundity 0.35 can then give no offspring. A birth event marks a successful attempt. The readout records the state as each step began, so a founder that fills both earmarks and reproduces within one step is never seen gated. Attempts are counted from the discharge, which catches that case.
- **Gates.** These come from `mode1.json`: `reproduction_energy_threshold` 14.562065 and `reproduction_nutrient_threshold` 1.0. A founder is *gated* entering a step when both earmarks are at or above them.
- **The senescence hazard** is 0.022 times total wear over the three functional traits, so 0.066 times the readout's mean wear, a tick. Summed over each founder's life as lived, it is the number of senescent deaths the hazard expects.
- **The line's departures from the vertex.**
  - A *departure* is an off-vertex offspring whose in-line parents are all on the vertex.
  - It is by *crossing* if a parent is outside the line, and by *mutation* otherwise.
  - The line is *mostly off the vertex* from the first 50-tick mark at which fewer than half its living members are pure vertex.
- **Heterotroph-dominant** means heterotrophy above autotrophy, the guild predicate's test.

## a. Why founders earning above upkeep die

| `n` | founders | senesced | reserve | structural | age at death | upkeep at death / founding | structure at death (from 2.46) | gained / upkeep, last 10 steps | reserve at death |
|---:|---:|---:|---:|---:|---|---|---|---|---|
| 20 | 100 | 0 | 100 | 0 | 14–17 | 1.19–1.32 | 3.07–3.51 | 0.24–0.60 | 0.06–0.24 |
| 10 | 50 | 1 | 49 | 0 | 15–92 (median 20) | 1.31–1.49 | 3.44–4.03 | 0.47–1.07 | 0.13–0.33 |
| 5 | 25 | 23 | 2 | 0 | 53–558 (median 281 senesced) | 1.56–7.74 | 4.26–24.17 | 0.97–1.76 | 0.27–6.69 |

No founder dies structurally, and none dies below its peak structure. Every founder's structure at death is its peak.

**The founding reserve goes in the first six ticks.** Each founder starts with 25.76 of reserve. The stepper mobilises 0.41 of the reserve above a retention of about three times upkeep each tick. 0.675 of that (1 − kappa) goes to the energy earmark and the rest to growth. By the start of step 7 the reserve is 2.3–2.8 in every founder read, and the body has grown to 3.1–4.1 from 2.46. From then on the reserve sits near the retention buffer. Income above upkeep goes on to the earmark and to growth, not to the reserve.

**At `n` = 20 every founder dies on reserve at ticks 14–17.** Its income is 0.24–0.60 of its own upkeep over its last ten steps. Its upkeep is already 1.19–1.32 times founding, because it grew. Its first attempt took 15–21 of energy into the earmark. Its founding reserve alone would have covered a deficit of about 0.15 a tick for over 150 ticks. That is arithmetic on the readout, not a run. So, at this point, the death tick is set by how fast the founding reserve is spent, not by income. #829 found income nearly doubled from #826 and the death tick barely moved, and this explains it.

**At `n` = 10, 49 of 50 die on reserve.** The ones that outlive tick 30 earn about their own upkeep at death: 0.91–1.07 times over the last ten steps. They earn more than founding upkeep (#829: 1.32–1.44 times 0.1987 over ticks 20–100), but their upkeep has grown 1.31–1.49 times with a body of 3.4–4.0. They have grown until upkeep met income, and their reserve buffer, 0.13–0.33 at death, cannot absorb a lean tick. One founder (seed 1) senesced at 53.

**At `n` = 5, 23 of 25 die by age.** They die at ages 53–558, while earning 1.24–1.76 times their own upkeep. Their wear at death is 0.014–0.139, a hazard of 0.0009–0.0092 a tick. Summed over their lives as lived, the hazard expects 18.0 senescent deaths, against 23 observed, so the deaths are what the hazard gives. Wear accrues with use, which here is drain, so the largest founders wear fastest. The long-lived ones reach structure 18–24, upkeep 1.0–1.4 a tick (5.7–7.7 times founding) and wear 0.10–0.14. The other two (seeds 3 and 4) die on reserve at 180 and 200, at 0.97–1.02 of their own upkeep, as at `n` = 10. #829 noted that these lifespans bracket the established producers' median (311–346); the senescent median here is 281.

**So upkeep does grow as founders grow, and it explains the deaths at `n` = 10.** It tracks structure. Under the size-scaled base rate the base is proportional to structure. The hazard is senescence at `n` = 5 only.

## b. The nutrient earmark and the timing of reproduction

| `n` | founders | attempted | first attempt, tick | attempts | with a birth | founders with a birth | later attempts (ticks) | steps entering with energy earmark only / nutrient only / both |
|---:|---:|---:|---|---:|---:|---:|---|---|
| 20 | 100 | 97 | 4–13 | 97 | 35 | 35 | none | 296 / 480 / 151 |
| 10 | 50 | 50 | 3–15 | 50 | 17 | 17 | none | 11 / 1,069 / 164 |
| 5 | 25 | 25 | 3–15 | 79 | 16 | 11 | 54, ticks 101–552 | 0 / 6,909 / 249 |

**The first attempt is made from the founding reserve, at tick 3–15.** Every founder at `n` = 10 and 5 makes one, and 97 of 100 at 20. The earmark at the first attempt is 12.0–21.4, nearly all of it mobilised founding reserve: income by then is 0.07–0.5 a tick. At `n` = 20 and 10 it is the only attempt. At 5 a founder refills its energy earmark from income in about 100 ticks, and 54 later attempts follow at ticks 101–552, up to 10 for one founder.

**About two in three attempts leave no offspring:** 62 of 97 at `n` = 20, 33 of 50 at 10, 63 of 79 at 5. An asexual brood is Poisson on fecundity 0.35, so it is empty 70 % of the time, and the earmark is spent either way. At `n` = 5, 8 of 25 first attempts and 8 of 54 later ones leave a birth. Only 11 of the 25 founders ever have offspring.

**Which gate binds.**
- **At `n` = 20 the nutrient earmark delays the first attempt.** Founders enter 296 steps with the energy earmark full and the nutrient one short. It is the only point where that happens much. Their nutrient income is low: the earmark is 0.05–2.2 at death.
- **After the first attempt, the energy earmark binds at every point.** Founders enter 480, 1,069 and 6,909 steps with the nutrient earmark full and the energy one short. At `n` = 5 the nutrient earmark reaches 25–173 by death, 25 to 170 times its threshold. A birth donates it whole, so a founder's late offspring carry a nutrient endowment of up to 156, against 0.5–5 for an early one.

**A gated founder can wait.** Founders entered 151, 164 and 249 steps with both gates met, against 97, 50 and 79 attempts, so many gated steps pass without one. Each step a gated founder rolls its asexual propensity, 0.15 in the recipe. Failing that it needs a compatible, gated mate within its mate reach. So both gates met is not reproduction.

**The mates.** Of the founders' own 79 births, 37 are asexual and 5 are founder × founder. 37 are with a mate outside the line, at autotrophy 0.86–1.63, so producers or producer-line mixotrophs. A pure-vertex founder and a recipe producer are 1.52 apart in trait space, inside the compatibility distance of 3.81. Producers are many and gated, so a founder failing its asexual roll usually finds a producer as its nearest compatible mate.

**Against the income trajectory.** At `n` = 20 and 10 the only attempt comes before income has settled, from stores. At `n` = 5 income per founder rises over its life (#829: 2–9 times founding upkeep). Later attempts come as the large founders' income outruns their upkeep, and none of the 23 senescent founders had income below upkeep in its last ten steps. No founder at any point reproduces in response to falling income.

## c. How the line leaves the heterotroph vertex

| `n` | seed | line births | asexual / sexual in line / crossing | departures: crossing / mutation | first crossing | line mostly off vertex from | last pure-vertex member born, alive to | line at end | heterotroph-dominant in line at end |
|---:|---:|---:|---|---|---:|---:|---|---:|---:|
| 20 | 1 | 285 | 190 / 68 / 27 | 4 / 0 | 6 | 50 | 7, 23 | 40 | 0 |
| 20 | 2 | 246 | 141 / 35 / 70 | 2 / 0 | 6 | 50 | 7, 22 | 21 | 0 |
| 20 | 3 | 5 | 0 / 2 / 3 | 0 / 0 | 7 | - | 11, 25 | 0 | 0 |
| 20 | 4 | 469 | 233 / 140 / 96 | 3 / 1 | 6 | 50 | 11, 24 | 62 | 1 |
| 20 | 5 | 4 | 1 / 1 / 2 | 0 / 0 | 6 | - | 8, 23 | 0 | 0 |
| 10 | 1 | 394 | 308 / 61 / 25 | 2 / 0 | 6 | 100 | 7, 84 | 37 | 0 |
| 10 | 2 | 172 | 76 / 40 / 56 | 1 / 0 | 6 | 50 | 6, 64 | 13 | 0 |
| 10 | 3 | 1 | 1 / 0 / 0 | 0 / 0 | - | - | 4, 92 | 0 | 0 |
| 10 | 4 | 367 | 80 / 239 / 48 | 1 / 1 | 6 | 50 | 10, 64 | 40 | 23 |
| 10 | 5 | 1 | 1 / 0 / 0 | 0 / 0 | - | - | 3, 84 | 0 | 0 |
| 5 | 1 | 228 | 147 / 18 / 63 | 4 / 0 | 7 | 300 | 633, 898 | 28 | 0 |
| 5 | 2 | 22 | 9 / 2 / 11 | 3 / 0 | 326 | 1,000 | 1,163, 1,461 | 0 | 0 |
| 5 | 3 | 2 | 1 / 0 / 1 | 0 / 0 | 6 | - | 436, 791 | 0 | 0 |
| 5 | 4 | 179 | 78 / 85 / 16 | 2 / 0 | 390 | 650 | 493, 847 | 26 | 10 |
| 5 | 5 | 1 | 1 / 0 / 0 | 0 / 0 | - | - | 3, 328 | 0 | 0 |

**The line leaves the vertex by crossing with producers, not by drift.**
- 22 of the 24 departures have a parent outside the line, at autotrophy 0.86–1.63. Two are by mutation, one each at `n` = 20 seed 4 and `n` = 10 seed 4.
- Of the line's 2,376 births, 1,958 have no parent outside it, and 1,911 of those are off-vertex because their in-line parents already are.
- A hybrid of a founder and a recipe producer takes each trait from one parent, so it is producer-like, decomposer-like, both or neither.

**The pure-vertex part of the line does not establish.** The founders' pure-vertex offspring die young, mostly on reserve. Median age is 11–14 at `n` = 20 and 10. At `n` = 5 it is 73 and 88 in seeds 1 and 2, the longest 611. Every line alive at tick 3,000 is wholly off the vertex.

**By when.**
- At `n` = 20 the line is all off the vertex by tick 50, in the three seeds where it survives. Its last pure-vertex member dies at 22–25.
- At `n` = 10 it is mostly off by 50–100. The last pure-vertex member dies at 64–92, and it is a founder in four of the five seeds.
- At `n` = 5 the founders outlive most of their pure-vertex offspring. The line is mostly off the vertex from 300, 650 and 1,000 in seeds 1, 4 and 2. In seeds 3 and 5 it never left and died out at 791 and 328.

**What the line becomes.** In 5 of the 8 lines alive at the end it is producer-like, with no member heterotroph-dominant; `n` = 20 seed 4 has one. In the other two, `n` = 10 seed 4 and `n` = 5 seed 4, it holds 23 and 10 heterotroph-dominant members at autotrophy 1.00–1.37 and 0.91–1.12. Those are 23 of the world's 24 and 10 of its 13 heterotroph-dominant agents at the end. They are mixotrophs at about the producers' autotrophy, with higher heterotrophy, not specialists.

## d. When the pure-vertex drain stops, and whose drain the guild's is

**The pure-vertex drain stops with the line's last pure-vertex member:** by tick 25 at `n` = 20, by 92 at 10, and at 328–1,461 at 5. The per-interval split covers the arms only, from the interval ending at tick 1,500. In it, pure-vertex drain is 0 in every interval of both arms of all 15 runs, for the line and for others. Before the clearing, pure-vertex drain beyond the founders' own is 2–38 at `n` = 20 and 10, and 7–5,959 at 5 (#829). That window is not split by lineage. The line's pure-vertex members are its only drainers below 0.1 that the readout sees, and the largest such drain, seed 2 at 5,959, goes with the latest pure-vertex member, alive to 1,461.

**The founder line's share of the carcass drain, ticks 2,250–3,000:**

| `n` | seed | control: line share | guild holds (fewest), share | line heterotroph-dominant, 2nd half | perturbed: line share | perturbed line at end |
|---:|---:|---:|---|---|---:|---:|
| 20 | 1 / 2 / 3 / 4 / 5 | 0.027 / 0.001 / 0 / 0.145 / 0 | no (0) in all | 0, 0, 0, 0–2, 0 | 0.028 / 0.020 / 0 / 0.348 / 0 | 12 / 39 / 0 / 40 / 0 |
| 10 | 1 / 2 / 3 / 4 / 5 | 0.025 / 0.028 / 0 / 0.666 / 0 | no (0) in all | 0, 0, 0, 20–36, 0 | 0.035 / 0.017 / 0 / 0.769 / 0 | 44 / 11 / 0 / 34 / 0 |
| 5 | 1 | 0.098 | no (0), 0.000 | 0 | 0.002 | 3 |
| 5 | 2 | 0 | no (0), 0.022 | 0 | 0 | 0 |
| 5 | 3 | 0 | **yes (10), 0.297** | 0 | 0 | 0 |
| 5 | 4 | 0.440 | **yes (6), 0.179** | 4–12 | 0.421 | 18 |
| 5 | 5 | 0 | **yes (8), 0.284** | 0 | 0 | 0 |

**The guild at `n` = 5, seeds 3–5.**
- **Seeds 3 and 5:** the line's last member dies at tick 791 and 328, so the guild over ticks 2,250–3,000 does not descend from the founders. The founder line takes none of the drain.
- **Seed 4:** the line takes 0.440 of the carcass drain and holds 4–12 heterotroph-dominant members on the second half's samples, against the guild's fewest of 6. At the end, 10 of the world's 13 heterotroph-dominant agents are the line's. The guild there is very likely largely the founders' line, but its members are mixotrophs at autotrophy 0.91–1.12. They descend from two crossings with producers, the first at tick 390, and their pure-vertex ancestors are gone by 847. The readout does not tag guild members by lineage, so the overlap is inferred, not read.

**`n` = 10 seed 4 is the line's strongest showing:** 0.666 of the drain and 20–36 heterotroph-dominant members on the second half's samples. Yet its guild fails on fewest members (0), as #829 recorded. The line's heterotroph-dominant mixotrophs drain carcasses but are not counted as guild members on every sample. The guild needs the decomposer role as well as heterotroph dominance; this was not read further.

**The perturbed arm.** The line is alive at the end in the same 8 runs as in the control. Its second-half share differs from the control's by up to 0.20 (`n` = 20 seed 4, 0.348 against 0.145). At `n` = 5 seed 4 it is 0.421 against 0.440, but the perturbed guild fails there (fewest 2), as #829 recorded.

## Observations beyond the questions

- **#829's two death groups at `n` = 10, read per founder.** The 32 founders dying before tick 30 earned 0.47–0.92 of their grown upkeep over their last ten steps. The 18 later ones earned 0.91–1.07. #829 inferred from the timing that random placement left some founders on more-shared litter; the per-founder income agrees.
- **The founders' offspring carry the earmark's nutrient.** Early offspring get an endowment of 0.5–5. Late ones at `n` = 5 get 100–156, the whole of an earmark grown 25–170 times its threshold.
- **Line deaths beyond the founders are mostly on reserve:** 1,426 of 2,109. 682 are senescent and one is structural. The senescent ones die at a median age of 282–585 by run, about the producers' lifespan, since most such members are producer-like.

## For the ecology read

Each finding beside the domain facts in [fungi](../ecology/fungi.md), *The saprotroph's life history*, and [trophic roles](../ecology/trophic-roles.md), *A decomposer lineage stays a decomposer*. These are observations for the reproduction grill to take up, not decisions.

1. **Ageing.**
   - *The world:* at `n` = 5, founders that out-earn their upkeep die by age at 53–558 ticks, median 281, about a producer cohort's lifespan. The hazard is the producers' own, 0.022 times wear, and wear accrues with use, so the largest, best-fed founders age fastest.
   - *The domain:* a saprotroph genet has no age-tied hazard in most species. Senescence is expected only on substrates limited in space and time (the dung fungi), and what kills a genet is ecological: its resource running out, a rival, grazing. A resource-unit-restricted genet lives about as long as its piece of litter, an inference, not a measurement.
   - *The discrepancy:* the world gives the decomposer founder a plant's ageing. In the domain its death would come from its litter.
2. **Upkeep as it grows.**
   - *The world:* upkeep is proportional to structure. A founder grows until upkeep about equals income, 1.3–1.5 times founding at `n` = 10 and up to 7.7 times at 5, and then dies on reserve at the first lean step. That is the death of 49 of the 50 founders at `n` = 10.
   - *The domain:* upkeep tracks the living, active part. Old mycelium is emptied, recycled or abandoned, older mycelium respires less, and most of the mycelium in far-decomposed litter is dead. So the cost of keeping a mycelium grows more slowly than the mycelium. The domain gives no exponent.
   - *The discrepancy:* the world charges the whole body. The domain charges a share that falls as the body grows.
3. **When it reproduces, and from what.**
   - *The world:* the founding reserve goes to a first attempt in the first 3–15 ticks, before the founder has grown into its litter. At `n` = 20 and 10 it is the only attempt. Later attempts at `n` = 5 come while income is rising. About two in three attempts leave no offspring, and the earmark is spent either way.
   - *The domain:* mushroom-forming fungi fruit when their nutrients run out, and only the youngest mycelium keeps the competence. Theory has a saprotroph on one patch grow first and switch to spores before the patch is used up. Leaf-litter microfungi sporulate while they grow. Fruiting is provisioned from stores and from material moved through the network.
   - *The discrepancy:* the world's founder reproduces at founding and grows second, the reverse of the domain's grow-then-switch, and not in response to depletion. Provisioning from stores agrees with the domain, but here the store is the founding reserve, spent before the first lean time.
4. **The nutrient earmark.**
   - *The world:* the nutrient gate delays the first attempt only at `n` = 20. Afterwards the energy earmark binds, and at `n` = 5 the nutrient earmark reaches 25–173 against a threshold of 1. Late offspring carry it all.
   - *The domain:* a litter saprotroph's fruit body is several times richer in nitrogen than its litter, and which element limits fruiting is open.
   - *Not a discrepancy:* the world has founders gather nutrient far beyond what reproduction asks, and then hand it all to one offspring. The domain neither confirms nor rules this out.
5. **Crossing with producers.**
   - *The world:* the founder line leaves the heterotroph vertex by crossing with producers, in 22 of 24 departures. Founders mate with producers in 37 of their 79 births, because a founder and a recipe producer are 1.52 apart in trait space, within the compatibility distance of 3.81. Every surviving line is descended from such crosses and wholly off the vertex.
   - *The domain:* isolation between fungi and plants is categorical, with no cross known. Trait distance stands in for isolation only within a clade, an inference. A saprotroph has no route to autotrophy over a stand's timescale.
   - *The discrepancy:* this is the plainest one. In the domain, constraint alone holds a decomposer lineage at the heterotroph end. In this world no constraint does, and crossing is the main route off.
6. **Drift.**
   - *The world:* mutation took the line off the vertex twice in 15 runs.
   - *The domain:* no short route to photosynthesis.
   - *Agreement as far as it goes:* drift is not what moves the line here.

## Runtime

15 paired runs, release build, on 8 cores, through `xargs -P 5` under `caffeinate -i` (approved by the owner), `n` = 5 first, on mains power. The sweep ran from about 11:08 to 12:30 on 2026-10-14: about 82 minutes of wall-clock and 21,640 s of CPU (6.0 CPU-hours). Per run, CPU (wall-clock):
- `n` = 20: 855–1,118 s (936–1,262 s). 4,834 s of CPU for the five.
- `n` = 10: 1,463–1,756 s (1,560–1,935 s). 7,807 s.
- `n` = 5: 1,644–2,016 s (1,752–2,151 s). 8,998 s.

Wall-clock was 1.07–1.16 times CPU in every run. The readout costs nothing measurable: total CPU is within 1 % of #829's 21,892 s. A `report.json` is 1.5–2.3 MB, against #829's 1.5 MB.

## Reproducing

The frozen binary, scripts and per-run reports are under `target/833-sweep/` (not committed). To rebuild:

```sh
cargo build --release -p explorers-reference-modes --bin reference_mode
mkdir -p target/833-sweep && cp target/release/reference_mode target/833-sweep/
# one paired run per line "n seed"; k = 68.85, 137.7, 275.4 for n = 20, 10, 5
caffeinate -i xargs -P 5 -L 1 sh target/833-sweep/scripts/run.sh < target/833-sweep/jobs.txt
python3 -I target/833-sweep/scripts/identity.py
python3 -I target/833-sweep/scripts/analyze.py all fresh
```

`run.sh N SEED` maps `n` to k, skips a run whose `report.md` exists, and runs the command above under `/usr/bin/time -p` into `runs/n${N}_s${SEED}/`. `identity.py` compares every field of #829's reports with the new ones. `analyze.py` writes `rows.json` and prints a section per question: `a`, `b`, `c` and `d`, with `x` for the cross-checks, `e` for the mates and the line at the end, and `seeds` for the table below.

## Appendix: per point and seed

The control arm. The columns:
- Attempts are earmark discharges by founders. Births are summed per founder, so a founder × founder birth counts twice; there are 5 such births in all, at `n` = 20.
- The line-birth kinds are as in *c*.
- Line share is the founder line's share of all carcass drain over ticks 2,250–3,000.
- Runtime is wall-clock, with CPU in brackets.

| `n` | seed | founder deaths: senesced / reserve / structural | first–last death | attempts (births) | first attempt | line births: asexual / sexual in line / crossing | line mostly off vertex from | last pure-vertex member alive to | line at end (control / perturbed) | line share of carcass drain, 2,250–3,000 | guild holds (fewest), share | runtime s (CPU) |
|---:|---:|---|---|---|---|---|---:|---:|---|---:|---|---:|
| 20 | 1 | 0 / 20 / 0 | 14–16 | 19 (12) | 5–10 | 190 / 68 / 27 | 50 | 23 | 40 / 12 | 0.027 | no (0), 0.027 | 1,099 (951) |
| 20 | 2 | 0 / 20 / 0 | 14–16 | 20 (7) | 5–10 | 141 / 35 / 70 | 50 | 22 | 21 / 39 | 0.001 | no (0), 0.024 | 1,262 (1,118) |
| 20 | 3 | 0 / 20 / 0 | 14–17 | 19 (7) | 4–12 | 0 / 2 / 3 | - | 25 | 0 / 0 | 0.000 | no (0), 0.001 | 969 (855) |
| 20 | 4 | 0 / 20 / 0 | 14–15 | 19 (9) | 5–11 | 233 / 140 / 96 | 50 | 24 | 62 / 40 | 0.145 | no (0), 0.026 | 1,143 (1,039) |
| 20 | 5 | 0 / 20 / 0 | 14–16 | 20 (5) | 5–13 | 1 / 1 / 2 | - | 23 | 0 / 0 | 0.000 | no (0), 0.000 | 936 (871) |
| 10 | 1 | 1 / 9 / 0 | 17–75 | 10 (5) | 4–15 | 308 / 61 / 25 | 100 | 84 | 37 / 44 | 0.025 | no (0), 0.063 | 1,560 (1,463) |
| 10 | 2 | 0 / 10 / 0 | 15–64 | 10 (4) | 4–12 | 76 / 40 / 56 | 50 | 64 | 13 / 11 | 0.028 | no (0), 0.000 | 1,613 (1,507) |
| 10 | 3 | 0 / 10 / 0 | 17–92 | 10 (1) | 4–12 | 1 / 0 / 0 | - | 92 | 0 / 0 | 0.000 | no (0), 0.025 | 1,744 (1,616) |
| 10 | 4 | 0 / 10 / 0 | 16–64 | 10 (12) | 3–15 | 80 / 239 / 48 | 50 | 64 | 40 / 34 | 0.666 | no (0), 0.070 | 1,935 (1,756) |
| 10 | 5 | 0 / 10 / 0 | 16–84 | 10 (1) | 3–11 | 1 / 0 / 0 | - | 84 | 0 / 0 | 0.000 | no (0), 0.000 | 1,597 (1,465) |
| 5 | 1 | 5 / 0 / 0 | 53–550 | 19 (8) | 4–7 | 147 / 18 / 63 | 300 | 898 | 28 / 3 | 0.098 | no (0), 0.000 | 2,060 (1,930) |
| 5 | 2 | 5 / 0 / 0 | 71–487 | 16 (2) | 3–10 | 9 / 2 / 11 | 1,000 | 1,461 | 0 / 0 | 0.000 | no (0), 0.022 | 1,786 (1,676) |
| 5 | 3 | 4 / 1 / 0 | 112–550 | 13 (2) | 6–12 | 1 / 0 / 1 | - | 791 | 0 / 0 | 0.000 | yes (10), 0.297 | 2,151 (2,016) |
| 5 | 4 | 4 / 1 / 0 | 133–558 | 22 (8) | 3–15 | 78 / 85 / 16 | 650 | 847 | 26 / 18 | 0.440 | yes (6), 0.179 | 1,752 (1,644) |
| 5 | 5 | 5 / 0 / 0 | 116–328 | 9 (1) | 3–11 | 1 / 0 / 0 | - | 328 | 0 / 0 | 0.000 | yes (8), 0.284 | 1,845 (1,732) |
