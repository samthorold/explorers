# Issue #764: standing carcasses and a 3 × 3 patch in the mode-1 mesocosm

**Status: measurement (2026-10-09). The mesocosm (`explorers_search::mesocosm`) is now a 9 × 9-cell
torus, its perturbed and reported patch is the 3 × 3 block of cells at the centre, and every cell
starts with standing carcasses drawn from the pool. The instrument is `reference_mode`. Every run
used the committed `recipe.json`, the provisional wear rate 0.2, and the recipe's placeholder repair
rate and senescence hazard (ρ = 1, η = 0.01, #763).**

## TL;DR

- **The wider patch holds producers early, but not for the whole run.** With seed 1 the 3 × 3
  patch holds 4–20 producers from tick 25 to tick 500 (seeds 1–5: 4–34). The single centre cell
  held 0–6 before. Over the second half (ticks 1,500–3,000) the patch has producers in 7 % to
  100 % of samples, depending on the seed. The whole world thins to 3–11 producers by tick 3,000.
- **The decomposers still do not persist past 200 ticks. The carcass stock is not the cause.** With
  the settled stock, the decomposer founders are gone by tick 18 (seed 1). Ten times the stock, and
  a wear rate of 0.01 instead of 0.2, change nothing. The cause is the founders' phenotype. A
  decomposer on the recipe's pure heterotroph vertex converts producer litter at a trophic
  efficiency of **0.021**, because the litter's traits sit at the opposite trophic vertex. The
  recipe's settled carcass stock is producer litter (energy-weighted autotrophy 1.02–1.13,
  heterotrophy 0.03–0.07), so it is no better food. The acceptance criterion "decomposers persist
  past the first 200 ticks" is **not met**. This is a design question for the owner (posted on
  #764), not something a carcass stock can fix.
- **Conservation holds from tick 0.** The litter's nutrient is drawn from the pool after the
  founders bind theirs, so total nutrient at creation equals the pool (test
  `conservation_holds_from_tick_0_with_the_standing_carcasses`).

## The carcass stock and where it comes from

The stock is a settled recipe world's, scaled to one cell's area. The committed recipe was run from
its own initial distribution (`World::from_recipe`, its 81-cell, 83.3-unit world), seeds 1–10, and
its standing carcasses were read every 50 ticks over ticks 1,500–2,000, the settled horizon's last
quarter. Per cell area (100 square units) they average:

| seed | carcasses | energy | nutrient |
|---:|---:|---:|---:|
| 1 | 16.4 | 69.9 | 63.4 |
| 2 | 19.3 | 47.7 | 47.6 |
| 3 | 14.8 | 64.2 | 69.6 |
| 4 | 18.1 | 36.2 | 41.6 |
| 5 | 16.7 | 66.2 | 90.0 |
| 6 | 13.2 | 62.3 | 56.3 |
| 7 | 19.8 | 55.2 | 61.6 |
| 8 | 17.0 | 36.3 | 39.0 |
| 9 | 14.8 | 44.4 | 55.3 |
| 10 | 19.1 | 57.2 | 54.2 |
| **mean** | **16.9** | **54.0** | **57.9** |

So every mesocosm cell gets `CARCASSES_PER_CELL = 17` carcasses holding `CARCASS_ENERGY_PER_CELL = 54`
energy and `CARCASS_NUTRIENT_PER_CELL = 58` nutrient, split equally (3.18 energy, 3.41 nutrient
each). They are placed uniformly at random within the cell, from the seed, before the founders, so
both communities stand the same litter and the same producers. They carry the producer founders'
trait vector, because a producer stand's litter is producer litter, and the settled recipe world's
litter is too (above). The 81 cells hold 1,377 carcasses, 4,374 energy and 4,698 nutrient.

**Nutrient.** World rules, *Founders bind nutrient at world creation*: the litter's nutrient is
drawn from the pool with the founders' nearest-first rule (`NutrientGrid::draw_nearest`), carcass by
carcass, after the founders have drawn theirs. Each cell's pool (720 at founding) covers its own
founders and litter, so the draw is the in-place subtraction. The sim's seeded carcasses
(`WorldRecipe::carcasses`, #311) still conjure their nutrient. The mesocosm builder does the draw
itself, with no sim change.

## Seed 1, 3,000 ticks

`reference_mode --seed 1` (defaults: wear 0.2, a sample every 25 ticks). The patch columns cover the
3 × 3 block. "Decomposers" are the whole world's agents that the income read calls decomposers. At
tick 0 nothing has an income yet, so both counts read 0.

| tick | patch producers | structure | patch pool N | patch carcass N | births | deaths | mean age | decomposers (world) |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 | 0 | 0.0 | 5919.09 | 522.00 | 0 | 0 | - | 0 |
| 25 | 20 | 73.6 | 5871.16 | 520.56 | 10 | 3 | 18 | 0 |
| 50 | 16 | 93.0 | 5815.03 | 552.28 | 5 | 9 | 40 | 0 |
| 75 | 16 | 116.8 | 5754.87 | 606.07 | 11 | 13 | 48 | 0 |
| 100 | 15 | 110.0 | 5702.31 | 653.68 | 11 | 10 | 47 | 0 |
| 125 | 13 | 122.7 | 5659.07 | 681.15 | 8 | 10 | 52 | 0 |
| 150 | 11 | 140.3 | 5614.29 | 688.62 | 6 | 5 | 75 | 0 |
| 175 | 11 | 195.7 | 5563.05 | 719.48 | 6 | 7 | 100 | 0 |
| 200 | 8 | 156.7 | 5509.28 | 791.01 | 6 | 9 | 112 | 0 |
| 225 | 9 | 151.8 | 5474.77 | 795.99 | 5 | 4 | 94 | 0 |
| 250 | 7 | 172.2 | 5452.42 | 800.85 | 4 | 4 | 130 | 0 |
| 275 | 6 | 153.9 | 5423.16 | 815.72 | 1 | 2 | 149 | 0 |
| 300 | 5 | 125.1 | 5399.81 | 811.49 | 2 | 2 | 151 | 0 |
| 500 | 6 | 149.0 | 5199.60 | 1087.08 | 4 | 2 | 107 | 0 |
| 750 | 1 | 68.0 | 5326.01 | 1092.69 | 0 | 1 | 163 | 0 |
| 1000 | 0 | 0.0 | 5382.93 | 1010.82 | 1 | 1 | - | 0 |
| 1250 | 0 | 0.0 | 5435.39 | 1177.04 | 2 | 0 | - | 0 |
| 1500 | 2 | 175.2 | 5316.62 | 1196.05 | 4 | 2 | 239 | 1 |
| 1750 | 4 | 45.0 | 5140.19 | 1444.95 | 3 | 1 | 40 | 0 |
| 2000 | 6 | 97.2 | 4964.16 | 1574.28 | 4 | 1 | 65 | 0 |
| 2250 | 3 | 140.1 | 4833.90 | 1682.56 | 3 | 3 | 167 | 0 |
| 2500 | 0 | 0.0 | 4796.87 | 1823.26 | 1 | 2 | - | 0 |
| 3000 | 0 | 0.0 | 4911.53 | 1707.99 | 0 | 0 | - | 0 |

The world-wide roster at tick 3,000 is 3 producers and no decomposers. Producer lifespans: 3,305
deaths, median 13 ticks, p90 89, max 1,544.

**The first 30 ticks** (`--ticks 30 --sample-every 2`), world-wide decomposers by income role:

| tick | 2 | 4 | 6 | 8 | 10 | 12 | 14 | 16 | 18 | 20–28 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| decomposers | 53 | 55 | 57 | 55 | 56 | 4 | 3 | 3 | 0 | 0 |

Of the 81 decomposer founders, most die at ticks 11–12. The odd decomposer the income read finds
later (at most 1–2 in any sample, seed 1) is a mutant offspring of the producer line.

**Seeds 2–5**, same settings:

| seed | patch producers, ticks 25–500 (min–max) | 2nd half: share of samples with patch producers (max count) | samples with any decomposer (max) | world roster at 3,000 (P / D) |
|---:|---|---:|---:|---|
| 1 | 4–20 | 0.66 (6) | 14 (2) | 3 / 0 |
| 2 | 7–34 | 0.64 (6) | 38 (5) | 5 / 0 |
| 3 | 6–28 | 0.49 (4) | 23 (2) | 11 / 1 |
| 4 | 11–27 | 1.00 (7) | 10 (2) | 7 / 0 |
| 5 | 6–30 | 0.07 (1) | 14 (2) | 4 / 0 |

## Why the decomposer founders die

These readings come from throwaway probes on seed 1 (not committed). They follow one decomposer
founder tick by tick.

- **They eat, but gain almost nothing.** A founder drains the carcasses in reach (contact reach
  `heterotrophy × contact_range_coefficient` ≈ 1.37) at ~13 energy a tick. The trophic transfer
  efficiency is `0.78 × exp(−2.38 × d)`, and the distance between the decomposer vertex
  (autotrophy 0, heterotrophy 1.07) and producer litter (1.07, 0) is 1.52. So the efficiency is
  0.021, against 0.78 for litter of its own kind. Its reserve falls by 6–7 a tick from tick 1, as
  it would with no food.
- **The reserve goes into reproduction.** About 6.7 a tick moves into the reproductive reserve.
  A founder that took nutrient from litter reaches the thresholds and gives birth at about tick 4.
  Its offspring sit on the same vertex and starve the same way. A founder with no litter in reach
  never earns the nutrient earmark (1.0), so its reproductive reserve sits there unspent, about 20
  by tick 8. Either way, the founders die at ticks 11–12 with their reserve spent.
- **More litter does not help.** With 10 × the stock (170 carcasses, 540 energy, 580 nutrient per
  cell), or 10 × the energy on 17 carcasses, every decomposer founder is still gone by tick 25. The
  same holds at a wear rate of 0.05 and of 0.01. At 0.2 the heterotrophy wear also climbs past 0.5
  by tick 5, which shrinks reach (`exp(−wear)`) and adds repair cost, but that only speeds up a
  death that comes anyway.
- **The settled recipe world's heterotrophs are mixotrophs.** In the same settled runs, the
  heterotroph-dominant agents carry autotrophy 0.02–0.18 (for example (0.11, 0.85), (0.12, 1.27),
  (0.17, 1.14) as autotrophy, heterotrophy). In a probe that moved 0.11 or 0.2 of the decomposer
  founders' trophic investment to autotrophy, some heterotroph-dominant agents were alive at tick
  600 in 3 of 4 runs. This probe counted heterotroph-dominant agents by traits, not decomposers by
  income, so it does not show that they live as decomposers. It shows only that the pure vertex is
  the problem.

So "a carcass stock large enough to carry its decomposers" does not exist for this founder
phenotype. Only a different decomposer founder could be carried, for example one at the settled
world's mixotroph phenotype, or litter that is not producer litter. Both change the design (the
founders take the recipe's trophic vertices), so neither is made here. The question is on #764.

## Other observations

- **The world thins under the provisional wear.** World-wide producers fall from 162 founders to
  3–11 by tick 3,000 in every seed, while the patch pool stays near 5,000 (of 5,919 at founding). So
  the producers are not nutrient-limited. Recalibrating wear under the new law (#766) is the next
  lever for persistence. The control should be judged again after that.
- **Nutrient drifts ~10 units in 3,000 ticks.** Summed in f64, the world's total nutrient (pool,
  agents, carcasses) goes from 58,336.13 at founding to 58,326.92 at tick 3,000 (seed 1), −1.6e-4
  relative. On the 5 × 5 mesocosm before #764 the paired run's nutrient residual was 2.2 of ~18,000
  (1.2e-4), so the drift is not new. It scales with the number of stores the stepper updates in
  f32, and the pile here grows to ~4,800 carcasses. The paired run's residuals (`--clear-at 1500`,
  seed 1) are +14.7 and +17.4 nutrient and −46 and −29 energy, over 4,500 ticks from the start.
  Founding itself is exact to f32 rounding. This is not addressed here.
- **Runtime.** 44 s for 3,000 ticks (seed 1, release), 121 s paired, against 5.5 s and 18.7 s
  before. The cost follows the carcass count, because every mobile agent's move reads every carcass
  (`docs/agents/instrument-runtimes.md`).

## Reproducing

```sh
cargo build --release -p explorers-search --bin reference_mode
target/release/reference_mode --seed 1 --out target/reference-mode/764/s1.json
target/release/reference_mode --seed 1 --ticks 30 --sample-every 2 --out target/reference-mode/764/s1-early.json
```

The settled stock was read with a throwaway probe: `World::from_recipe(&recipe, seed)` stepped to
tick 2,000. At every 50th tick from 1,500 it summed `carcasses().len()`, carcass energy and carcass
nutrient. Each sum was scaled by `cell_size² / world_extent²` and then averaged.
