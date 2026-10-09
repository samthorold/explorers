# Issue #766: recalibrating the mode-1 mesocosm's wear under the new law

**Status: measurement and new defaults (2026-10-09). The instrument is `reference_mode`
(`crates/explorers-search/src/bin/reference_mode.rs`) on the #764 mesocosm (9 × 9 torus, 3 × 3
centre patch, standing carcasses). Every run used the committed `recipe.json`, producers and
decomposers, 3,000 ticks and a sample every 25 ticks. This supersedes the old repair law's
calibration, [753-mode1-wear-calibration.md](753-mode1-wear-calibration.md), which found no rate
that met both criteria.**

**Caveat.** The decomposer founders still die by about tick 18 (#764, owner question open there),
so every run here is in effect a producer world. The calibration may need revisiting once the
decomposer founders persist.

## TL;DR

- **Eight of the twelve guided points meet both criteria, and the old default does not.** The
  mesocosm's defaults (`Wear::MESOCOSM`) are now **`wear_rate` 0, `use_wear_rate` 0.0007,
  `ρ` 0.03, `η` 0.022** (point P12).
- **At P12, established producers live a median of 229 ticks** (10 seeds pooled; per seed 216–242),
  under the 277-tick leaching half-life. The cut-off for "established" is a death older than
  **50 ticks** (`ESTABLISHMENT_AGE`, the same cut-off #753 used). 75 % of established deaths are
  senescent; the rest are starvation or structural.
- **The control persists at P12 in 10 seeds of 10.** The centre patch holds producers at every
  25-tick sample of the second half (ticks 1,500–3,000), with at least 9–26 producers at its
  thinnest per seed. The second half is 1,500 ticks, over six median lifespans.
- **The old default (wear 0.2, ρ 1, η 0.01) fails persistence.** Its lifespans are in range (141),
  but loss of function at its high equilibrium wear (autotrophy works at about half strength)
  thins the world to 3–11 producers, and the patch persists in 1 seed of 5.
- **Wear from use, not from the baseline, gives the longer lifespans and the strongest
  persistence.** Points with baseline wear alone land at 139–158 ticks; points with use-wear alone
  at 193–232. A mixed point (P13, half of each) also meets both criteria, at 177 ticks.
- **Lifespans past establishment are close to exponential.** Their coefficient of variation is
  0.87–1.24; the slowest repair (`ρ` 0.03, relaxation ~95 ticks) gives the lowest. The hazard
  barely rises with age, so a cohort does not die together. That is not one of this issue's
  criteria, but colonisation overshoot depends on cohort synchrony (see *Open*).

## What the code reports

`reference_mode` now sets all four wear parameters (`--wear-rate`, `--use-wear-rate`,
`--repair-rate`, `--senescence-hazard`; defaults `Wear::MESOCOSM`). Its report adds
`senescent_lifespans`, the ages of producers whose death the stepper marked `Senesced`, and its
summary prints a line over established producers:

```text
established (age > 50): deaths N | median M | senescent S (share)
```

## The closed-form guide

From world rules, *Somatic wear*: wear on each functional trait relaxes to `w* = a / (ρκ)` with
time constant `τ = 1/(ρκ)`, and the senescent hazard approaches `η·W*` (`W*` summed over the
functional traits). With the hazard rising as `η·W*·(1 − e^(−t/τ))`, survival is
`exp(−η·W*·(t − τ(1 − e^(−t/τ))))`. The grid solves that for `η` so that the median age at death
of producers alive at 50 ticks is 200 ticks, the middle of the window below the half-life.

Inputs, for the producer founders and the mesocosm's producers:
- **Nominal traits:** autotrophy 1.073, mobility 0.143 (heterotrophy 0), so a baseline
  `a = wear_rate × 1.217`.
- **Kappa:** 0.325 at founding, rising to 0.45–0.75 under selection with wear on (throwaway
  probe). The guide uses κ = 0.35.
- **Throughput:** about 4.5 energy a tick captured per producer, and 0.3 spent moving (throwaway
  probe, seed 1, mean over ticks 0–1,000). So use-wear alone gives `a ≈ use_wear_rate × 4.5`,
  nearly all of it on autotrophy.
- **Loss of function:** effective autotrophy is `e^(−w_A*)`. A `W*` of 0.075 keeps it at 0.93–0.94,
  and 0.3 at 0.74–0.77. The old default sits at `W*` ≈ 0.70, about 0.5.

The guide predicts the old default's established median at 150 ticks; it measures 141.

## The grid

Twelve points: three repair rates (relaxation time `τ` ≈ 3, 29 and 95 ticks at κ = 0.35) × two
equilibrium wear levels (`W*` 0.075 and 0.3) × two sources (baseline wear alone, use-wear alone),
with `η` from the guide. Plus the old default (D), and a mixed point (P13) added after the grid,
with half of P12's `W*` from each source. Seeds 1–5 at every point, seeds 1–10 at P12 and P13.

```sh
# one line per run: name wear_rate use_wear_rate rho eta seed, 8 at a time
xargs -P 8 -L 1 sh -c 'target/agent/release/reference_mode --wear-rate $1 --use-wear-rate $2 \
  --repair-rate $3 --senescence-hazard $4 --seed $5 --out target/reference-mode/766/$0-s$5.json'
```

| point | τ (ticks) | `W*` | wear_rate | use_wear_rate | ρ | η |
|---|---:|---:|---:|---:|---:|---:|
| D (old default) | 3 | 0.70 | 0.2 | 0 | 1 | 0.01 |
| P1 | 3 | 0.075 | 0.02 | 0 | 1 | 0.06 |
| P2 | 3 | 0.3 | 0.09 | 0 | 1 | 0.015 |
| P3 | 3 | 0.075 | 0 | 0.006 | 1 | 0.06 |
| P4 | 3 | 0.3 | 0 | 0.024 | 1 | 0.015 |
| P5 | 29 | 0.075 | 0.002 | 0 | 0.1 | 0.06 |
| P6 | 29 | 0.3 | 0.009 | 0 | 0.1 | 0.015 |
| P7 | 29 | 0.075 | 0 | 0.0006 | 0.1 | 0.06 |
| P8 | 29 | 0.3 | 0 | 0.0024 | 0.1 | 0.015 |
| P9 | 95 | 0.075 | 0.00065 | 0 | 0.03 | 0.09 |
| P10 | 95 | 0.3 | 0.0026 | 0 | 0.03 | 0.022 |
| P11 | 95 | 0.075 | 0 | 0.00018 | 0.03 | 0.09 |
| **P12** | 95 | 0.3 | 0 | 0.0007 | 0.03 | 0.022 |
| P13 | 95 | 0.3 | 0.0013 | 0.00035 | 0.03 | 0.022 |

## Results

Definitions:
- **Established lifespan:** age at death of a producer (income read) that died older than 50
  ticks, world-wide, pooled over seeds; nearest-rank median. Per-seed medians in brackets.
- **Senescent share:** of established producer deaths, the share marked `Senesced`; in brackets,
  of all producer deaths.
- **Persists:** the centre 3 × 3 patch holds at least one producer at every sample from tick
  1,500 to 3,000. "Fewest" is the patch's minimum producer count over that half, per seed.

| point | est. deaths | median est. lifespan (per seed) | senescent share, est. (all) | patch occupied, 2nd-half samples | fewest in patch, 2nd half (per seed) | persists | world producers at 3,000 (per seed) |
|---|---:|---|---:|---:|---|---:|---|
| D | 2,287 | 141 (139, 148, 147, 146, 137) | 0.89 (0.31) | 0.57 | 0, 0, 0, 3, 0 | 1/5 | 3, 5, 11, 7, 4 |
| P1 | 3,723 | 157 (160, 162, 168, 148, 152) | 0.82 (0.27) | 0.57 | 0, 6, 0, 2, 0 | 2/5 | 13, 54, 24, 47, 74 |
| P2 | 2,690 | 158 (154, 167, 149, 162, 158) | 0.84 (0.25) | 0.82 | 0, 0, 1, 0, 0 | 1/5 | 7, 13, 4, 6, 17 |
| P3 | 7,964 | 193 (194, 208, 183, 196, 192) | 0.76 (0.22) | 1.00 | 0, 8, 17, 21, 20 | 4/5 | 165, 128, 210, 184, 135 |
| P4 | 8,886 | 204 (203, 211, 190, 212, 207) | 0.76 (0.22) | 1.00 | 15, 21, 5, 5, 20 | 5/5 | 161, 142, 168, 102, 137 |
| P5 | 5,935 | 150 (153, 151, 150, 141, 154) | 0.78 (0.23) | 0.97 | 8, 12, 5, 4, 0 | 4/5 | 70, 83, 81, 63, 55 |
| P6 | 6,457 | 139 (141, 134, 145, 136, 146) | 0.76 (0.25) | 1.00 | 1, 1, 12, 6, 1 | 5/5 | 65, 87, 63, 120, 57 |
| P7 | 8,931 | 203 (201, 210, 195, 205, 199) | 0.73 (0.20) | 1.00 | 11, 24, 13, 15, 19 | 5/5 | 158, 202, 243, 174, 181 |
| P8 | 10,005 | 224 (219, 233, 218, 221, 228) | 0.73 (0.20) | 1.00 | 22, 23, 28, 25, 9 | 5/5 | 261, 215, 227, 210, 234 |
| P9 | 7,543 | 151 (145, 153, 150, 151, 159) | 0.78 (0.25) | 1.00 | 3, 5, 2, 2, 1 | 5/5 | 70, 107, 133, 72, 41 |
| P10 | 10,329 | 155 (156, 157, 156, 156, 151) | 0.79 (0.30) | 1.00 | 7, 15, 15, 3, 6 | 5/5 | 70, 112, 165, 92, 126 |
| P11 | 10,124 | 213 (211, 215, 217, 201, 225) | 0.75 (0.21) | 1.00 | 9, 14, 9, 8, 17 | 5/5 | 209, 156, 246, 144, 191 |
| **P12**, seeds 1–10 | 21,657 | **229** (227, 234, 236, 220, 242, 227, 234, 216, 223, 230) | **0.75** (0.22) | 1.00 | 9, 16, 18, 12, 26, 18, 18, 15, 12, 18 | **10/10** | 185, 200, 244, 217, 235, 186, 235, 181, 194, 240 |
| P13, seeds 1–10 | 23,403 | 177 (176, 181, 179, 165, 180, 172, 188, 176, 183, 168) | 0.78 (0.27) | 1.00 | 27, 17, 5, 19, 19, 2, 3, 21, 9, 14 | 10/10 | 178, 147, 171, 205, 162, 214, 189, 182, 177, 238 |

The shape of the lifespans (pooled over the same seeds). "CV" is the coefficient of variation of
age past the 50-tick cut-off: 1 for an exponential, below 1 for deaths bunched around an age.

| point | est. p10 | est. median | est. p90 | CV past cut-off | median of all producer deaths | share of producer deaths established |
|---|---:|---:|---:|---:|---:|---:|
| D | 62 | 141 | 399 | 1.18 | 14 | 0.16 |
| P1 | 62 | 157 | 512 | 1.24 | 13 | 0.19 |
| P2 | 62 | 158 | 484 | 1.24 | 13 | 0.15 |
| P3 | 65 | 193 | 515 | 1.01 | 15 | 0.25 |
| P4 | 65 | 204 | 537 | 0.96 | 15 | 0.26 |
| P5 | 63 | 150 | 505 | 1.22 | 14 | 0.23 |
| P6 | 60 | 139 | 465 | 1.23 | 17 | 0.25 |
| P7 | 66 | 203 | 536 | 0.97 | 14 | 0.25 |
| P8 | 67 | 224 | 608 | 0.96 | 15 | 0.26 |
| P9 | 63 | 151 | 400 | 1.08 | 16 | 0.28 |
| P10 | 64 | 155 | 401 | 1.05 | 19 | 0.33 |
| P11 | 69 | 213 | 503 | 0.90 | 15 | 0.27 |
| P12 (10 seeds) | 71 | 229 | 540 | 0.89 | 15 | 0.28 |
| P13 (10 seeds) | 66 | 177 | 442 | 0.96 | 17 | 0.31 |

What the tables show:
- **Persistence is set by loss of function, not by lifespan.** Lifespans barely separate D, P1 and
  P2 from the rest, but those three run at short relaxation with baseline wear and hold a fraction
  of the producers. A throwaway producers-only probe (seed 1) self-thins with wear off too, but
  slowly: 64 producers at tick 2,000 with wear off, against 7 at the old default.
- **Use-wear spares seedlings.** A seedling captures little, so under use-wear it wears little and
  its hazard is small. Baseline wear charges it from birth by its nominal traits. That is the
  likely reason use-wear points sit about 50 ticks above the guide's baseline-wear points at the
  same `W*` and `η`.
- **Seedling deaths dominate every count.** 67–85 % of producer deaths come before 50 ticks, at a
  median age of 13–19, and only a fifth to a third of all producer deaths are senescent. Seedlings die
  of starvation or structural failure, under shading. Hence the cut-off.
- **The guide places the window well.** All twelve points land at 139–232 ticks against a target of
  200, so no point fell outside the few-hundred-tick window or above the half-life.

## The choice

**P12: `wear_rate` 0, `use_wear_rate` 0.0007, `ρ` 0.03, `η` 0.022**, now `Wear::MESOCOSM`.

Of the eight points that meet both criteria on five seeds (P4, P6–P12), P12:
- has its median nearest the middle of the window (229 ticks; P6, P9 and P10 sit near 150, the
  low end of "a few hundred");
- persists with the largest margin, together with P8 (no seed's patch falls below 9 producers);
- has the slowest relaxation, so the hazard rises over a lifespan, and the least exponential
  lifespans (CV 0.89);
- persisted in all ten seeds.

Its wear comes from use alone, which is the law's activity-dependent property: a producer ages
with the light it puts through its apparatus. Wear is not off. The baseline term is unused at
this point. P13, which splits the same `W*` between baseline and use, also meets both criteria
(177 ticks, 10/10 seeds) and is the alternative if the baseline term should carry part of the
wear.

## Runtime

Wall-clock rises with the roster. At the old default the world thins to a handful of producers
and a 3,000-tick run took 44 s (#764). At P12 it holds ~200 producers, and a run takes 122 s alone
(seed 1, release build), about 3.5 min each with 8 sharing 8 cores. The 65-run grid took 24 min 36 s, 8 at a time on 8 cores; the 15 runs of the
second batch, 8 min.

## Open

- **Cohort synchrony.** Lifespans past establishment are near-exponential at every point
  (CV 0.87–1.24). Colonisation overshoot's peak and decline come from an even-aged cohort dying
  over a short span, so the paired reading should check whether this hazard gives a decline at
  all. Slower relaxation helps a little; a hazard that rises more steeply with age would need a
  change to the law, which this issue does not make.
- **Decomposers.** The decomposer founders die at once (#764). With them persisting, carcass
  return changes and producer persistence may change with it. Re-run P12, and P11–P13, then.
