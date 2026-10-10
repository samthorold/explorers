# Issue #764: why the mode 1 mesocosm's decomposer founders die

**Status: diagnosis (2026-10-10). No code or parameter is changed by this note.**

Under #772's override (`trophic_distance_decay` 0.60, so assimilation is about 0.31 on producer litter), the decomposers founded at the recipe's pure heterotroph vertex still die out within about 20 ticks at the recipe's litter density. The owner read #772's result as row 3 of #764's table: efficiency was not the binding cause. This note finds what is.

## TL;DR

A founder's sustained income is about **0.05 energy a tick against a cost of about 0.46**. Three parts of the design compound to make it so:
- a single carcass pays less than upkeep;
- foraging movement cannot find fresh litter;
- reserve is not banked against the gaps between meals.

None of these is a bug. Each is a design question.

## Method

The probe was a throwaway example, `founder_probe`, since deleted. It ran on the #772 mesocosm with recipe.json, `Wear::MESOCOSM` and both arms of the community. It traced the 81 founders per tick:
- their reserve, structure, metabolic cost and reproductive earmark;
- the structure they drained;
- the carcasses within reach and their energy;
- distance moved and births;
- each death's cause: senesced, drained by another agent that tick, or neither.

It also dumped per-agent event ledgers.

The variants were set through `params_mut`:
- contact range ×2;
- base metabolic rate ×0.5;
- growth retention multiplier 20 and 60.

Other variants were set through the recipe's founder means: mobility ×2, ×3 and ×5, and kappa = 1. Two were temporary working-tree edits, reverted afterwards:
- litter ×10 (170 carcasses, 540 energy and 580 nutrient per cell);
- carcass attraction weighted by the carcass's energy.

Seeds 1–5 were used for the litter test, seeds 1–3 elsewhere, and seed 1 for the traces.

**A reading trap.** A carcass `Consumed` event's `energy_delta` is the structure **drained** (`actual_drain`), before flow 7's efficiency. Income is `0.31 ×` that. An early pass of this diagnosis read the drain as income and overstated income about threefold. The numbers below are corrected.

## Findings

**1. One carcass pays less than upkeep.**
- A drainer takes `effective heterotrophy × HETEROTROPHY_STRUCTURE_DRAIN_PER_TICK` from each carcass in reach. That is about 1.0 a tick for a founder.
- It keeps 0.31 of what it drains, so it gains at most **about 0.31 a tick per carcass**.
- Its upkeep is metabolism, about 0.40–0.42 (80 % of it the base rate), plus movement, about 0.04.
- So a founder breaks even only while at least **1.5 non-empty carcasses** are within reach.

**2. There is about one carcass in reach, and it is gone within 3 ticks.**
- Litter stands at 0.17 carcasses per unit area, at the recipe's density.
- A founder's feeding disc has radius `1.07 × 1.28 ≈ 1.37`, and `body_reach_coefficient` is 0, so the disc covers about 5.9 unit².
- At founding there are 1.1–1.2 carcasses within reach, holding about 2.5 energy. By tick 3 they hold 0.4.

**3. Foraging movement cannot find fresh litter.**
- **Attraction ignores how much a carcass holds.** It is `chemotaxis × heterotrophy / dist` times the displacement, so an empty carcass pulls as hard as a fresh one. The same weight applies to every living agent sensed.
- **Empty carcasses linger.** A carcass is removed only when both its energy and its nutrient are 0, and its nutrient leaves only by leaching.
- **Sensing barely exceeds reach.** The sensing range is `0.143 × 14.46 ≈ 2.07`, against a reach of 1.37.
- **The result is a random walk.** A founder is surrounded by drained carcasses whose pulls cancel. What is left is the random-walk term, a step of 0.143 in a random direction, so it stays inside the disc it has emptied.

The test that shows it ran at 10× litter:
- **Unweighted attraction:** 0–1 founders of 81 are alive at tick 50 (seeds 1–5).
- **Attraction weighted by carcass energy:** 35 are alive at tick 50 and 11 at tick 100 (seed 1).

Even weighted, the line dies out by about tick 150. Founders with one carcass in contact still lose reserve, as finding 1 predicts.

**4. Reserve is not banked against gaps.**
- The growth retention buffer is `3.0 × metabolic cost ≈ 1.2`, about 3 ticks of metabolism.
- Every tick, 0.41 of the reserve above it is mobilised, and kappa 0.32 sends most of that to the reproductive earmark. Reproduction draws on the earmark, and metabolism cannot.
- Founders spend about 15 of their 34 founding reserve in the first tick. In seed 1 they found 38 offspring, each as doomed as its parent, and they die at reserve ≈ 0 while still holding 6–10 energy each in the earmark.
- **Kappa = 1** lengthens life from about 20 ticks to 50–100, but no founder persists.
- **A larger buffer** (multiplier 20 or 60, with mobility ×3) buys at most about 100 ticks.

## Ruled out

| Candidate | Test | Result |
|---|---|---|
| Trophic efficiency | #772, decay 0.60 | Row 3: the founders die anyway |
| Feeding reach | contact range ×2 | No change: the larger disc is emptied by tick 2 |
| Kin and conspecific drains | drains on founders, cause of each death | Few founders are drained. Most deaths are by starvation |
| Senescence | `Senesced` events | Almost none |
| Total litter alone | litter ×10 | All founders dead by tick 50 without the movement change |
| Metabolism alone | base rate ×0.5 | All founders dead by tick 50 |
| Mobility alone | ×2, ×3, ×5 | Drain rises in proportion, but all founders are dead by tick 30 |

## What it bears on

These are three design questions, and each needs the literature and a grill before any change:
- **The movement rule** (world rules, *Movement*): should chemotaxis follow the resource rather than its presence? This is the strongest lever measured.
- **The per-carcass drain rate and the decomposer's upkeep.** At decay 0.60, one carcass pays 0.31 against an upkeep of 0.46.
- **Lean-time reserve:** the retention buffer and the locked earmark, against the interval between meals.

The mesocosm's founder phenotype and litter stock are left as they are. The owner's row 3 reading keeps decay at 0.60.
