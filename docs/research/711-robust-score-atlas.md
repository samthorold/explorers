# Issue #711: the re-search under the L² · F score, read by its fixed rule

**Status: measurement, docs only (2026-10-07). The same seed-42 search as #686 with only the
score changed: a cell's fitness is `L² · F`, its live fraction squared times its live seeds' mean
fitness (#709, #710), instead of the plain mean over seeds. The search, the fragility audit and the
census are run and read by the rule fixed in #711 before the run. The committed atlas is not
replaced here. That decision is the owner's (#687).**

## TL;DR

**The robustness rule fails again, but this time the change goes the right way. Against #686,
seed agreement rises from 43.4 % to 47.1 % of worlds and the mean live fraction from 0.864 to
0.887, with quality held (the guard passes). Both still fall short of the committed atlas (57.6 %,
0.900), and far short of the 70 % bar. λ again shows no pull away from 0. Flow 1's signatures are
the strongest yet. The 70 % bar is very demanding: under a simple binomial model, it needs most
worlds to live on at least 97 % of initial conditions.**

| rule | committed (#693) | #686 (mean) | **#711 (L² · F)** | verdict |
|---|---:|---:|---:|---|
| all 10 seeds agree, ≥ 70 % of worlds | 57.6 % | 43.4 % | **47.1 %** (40 of 85) | **FAIL** |
| mean live fraction > 0.900 | 0.900 | 0.864 | **0.887** | **FAIL** |
| guard: median live-seed F ≥ 0.225 | 0.228 | 0.281 | **0.276** | PASS |
| λ: ≤ 12.5 % of cells with `u_λ < 0.25` | – | 16.9 % | **17.6 %** (15 of 85) | **FAIL** |
| λ: ρ(λ, seed agreement) > 0 | – | −0.127 | **−0.114** (p = 0.86) | **FAIL** |

1. **`L² · F` moves robustness the right way, but not far.** Holding the seed, box and physics
   fixed, the square on L buys 3.7 points of seed agreement and 0.023 of live fraction. Flip rates
   at the widest jitter fall from 27.2 % to 23.5 %, and 4 more worlds have the widest basin (37
   against 33). The median live-seed fitness barely moves (0.281 → 0.276), so the gain cost
   almost no quality. The search was also nearly twice as fast (29 against 55 min), because it
   stopped climbing into the dense, slow worlds near the cliffs.
2. **The 70 % bar is close to out of reach for any score at 10 seeds.** A world that lives with
   probability p agrees on all 10 seeds with probability p¹⁰ + (1 − p)¹⁰. That is 0.35 at
   p = 0.90, 0.60 at 0.95 and 0.74 at 0.97. The atlas's mean live fraction of 0.887 is typical
   of worlds at p ≈ 0.9. For 70 % of worlds to agree on fresh seeds, most of them must live at
   p ≥ 0.97: robustness of a different order from anything any atlas here has shown.
3. **Selection overrates the winners only a little.** The search picks cells on its own 10 seeds,
   and the audit reads them on 10 fresh ones. On the fresh seeds the cells score 8 % less on
   average (mean `L² · F` 0.254 → 0.234); 51 cells fall and 34 rise. This is a real winner's
   curse, but small beside the gap to the bar.
4. **λ is again spread like an unselected coordinate,** and its ρ with seed agreement is again
   slightly negative. The bottom quarter of λ is the most robust quarter (12 of 15 cells agree,
   live fraction 0.91). Lockup again survives at high λ: 10 of the 12 lockup cells sit at
   λ ≥ λ_max/16, and two at λ's top.
5. **Flow 1's signatures are the strongest of the three atlases.** Conditionality passes all
   three tests in both modes. Decoded, 64 of 85 configs have ρ < 0 (p < 0.001). The light-fed
   mixotrophs' second-half share is 44.7 % decoded and 52.8 % at fa 0.

## 1. What ran

| | |
|---|---|
| Tree | main `9fc1aa9` (#712: `L² · F`, `LIVE_FRACTION_POWER = 2`). Release binaries pinned in `target/711/bin` |
| Search | `explorers-search --batch 32 --generations 10 --max-ticks 2000 --seed 42 --output target/711/atlas.json --recipe-output target/711/recipe.json --checkpoint target/711/checkpoint.json`: #686's command. The atlas records `scoring: {live_fraction_power: 2}, ensemble_size 10` |
| Search wall clock | 1,742 s (29 min), against #686's 3,285 s. Every generation took 1–5 min; no dense generation |
| Unfinished rollouts | 0 (#686: 4) |
| Atlas | 85 cells (coverage 1.06 %), QD score 21.61, best 0.520. 44 dead configs |
| Recipe | cell [7, 19, 3], refined fitness 0.415 at n = 32, coexistence 1.00 → 0.88. λ = 0.0072 |
| Audit | `env ATLAS=target/711/atlas.json K=target/711/fragility scripts/fragility-audit.sh`: seeds 1000–1009, 8 draws at radii 0.01 / 0.03 / 0.1. 21,250 rollouts in 10,897 s (3h02m). 1 timed out. A few cells cost 9–27 min each (atlas:4, 11, 18, 62, 72, 79, 81) |
| Census | `env ATLAS=target/711/atlas.json K=target/711/census scripts/683-census.sh`: decoded and fa 0, 967 s. No errors |
| Driver | the three steps chained with `.done` markers. The script is in the PR body |
| Reproducibility | not rerun |

Definitions are #686's. *All seeds agree*: the unperturbed cell's seed-only flip rate is 0. *Live
fraction*: the mean over cells of the share of each cell's 10 unperturbed seeds that persist.
*Live-seed F*: the mean fitness of a cell's live audit seeds, with the median over cells.

## 2. Robustness, three atlases side by side

| | committed (#693) | #686 (mean) | #711 (L² · F) |
|---|---:|---:|---:|
| worlds | 99 | 83 | 85 |
| all 10 seeds agree | 57 (57.6 %) | 36 (43.4 %) | 40 (47.1 %) |
| mean live fraction | 0.900 | 0.864 | 0.887 |
| median live-seed F | 0.228 | 0.281 | 0.276 |
| median seed-to-seed fitness sd | 0.149 | 0.157 | 0.164 |
| mean flip rate, r = 0.01 / 0.03 / 0.1 | 11.0 / 13.5 / 19.8 % | 14.2 / 16.6 / 27.2 % | 14.3 / 16.3 / 23.5 % |
| basin width 0 / 0.1 | 22 / 53 | 23 / 33 | 23 / 37 |
| audit rollouts timed out | 0 | 10 | 1 |

**What the bar asks.** All 10 seeds agree with probability p¹⁰ + (1 − p)¹⁰ for a world that lives
with probability p:

| p | 0.80 | 0.90 | 0.95 | 0.97 | 0.98 | 0.99 |
|---|---:|---:|---:|---:|---:|---:|
| P(all 10 agree) | 0.11 | 0.35 | 0.60 | 0.74 | 0.82 | 0.90 |

This treats seeds as independent draws at a fixed p per world. It is a model, not a measurement.

**Selection on the search's own seeds.** Recorded fitness (search seeds) averages 0.254, against
0.234 for the same cells' `L² · F` recomputed on the audit's fresh seeds (median 0.257 → 0.224).
51 cells fall and 34 rise. The search does not store each cell's search-time L, so the
shrinkage cannot be split between L and F here.

## 3. λ

| | #686 | #711 |
|---|---:|---:|
| cells with `u_λ < 0.25` | 14 of 83 (16.9 %) | 15 of 85 (17.6 %) |
| other coordinates' share below 0.25, min / median / max | 4 / 16 / 60 % | 2 / 15 / 48 % |
| median `u_λ` (λ) | 0.588 (0.0035) | 0.653 (0.0043) |
| ρ(λ, seed agreement), one-sided permutation p | −0.127, 0.87 | −0.114, 0.86 |
| ρ(λ, live fraction) | −0.117 | −0.109 |

| `u_λ` | cells | all seeds agree | mean live fraction |
|---|---:|---:|---:|
| [0, 0.25) | 15 | 12 | 0.913 |
| [0.25, 0.5) | 11 | 2 | 0.864 |
| [0.5, 0.75) | 25 | 12 | 0.884 |
| [0.75, 1] | 34 | 14 | 0.885 |

Both clauses fail. As on #686, the search's geometry leaves the median coordinate with about 15 %
below 0.25, under the 25 % that #706's threshold assumed, so the first clause would be hard to
pass even for a selected coordinate. The ρ clause does not depend on that baseline, and it is
negative on both atlases.

## 4. Deaths by cause

| cause | dead frontier: committed / #686 / #711 | atlas worlds' seeds: committed / #686 / #711 |
|---|---|---|
| nutrient lockup | 42 / 30 / 20 | 42 / 35 / 29 |
| extinction | 11 / 19 / 17 | 30 / 40 / 20 |
| monoculture | 6 / 3 / 5 | 17 / 21 / 20 |
| bloom stop | 2 / 9 / 2 | 6 / 5 / 12 |
| energy death | 3 / 6 / 0 | 0 / 12 / 3 |
| generalist dominance | 0 / 1 / 0 | 4 / 0 / 12 |
| total | 64 / 68 / 44 configs | 99 of 990 / 113 of 830 / 96 of 850 seeds |
| lockup's share | 66 / 44 / 45 % | 4.2 / 4.2 / 3.4 % |

The dead frontier shrinks to 44 configs. Lockup leads it again, at the same share as #686. Within
the atlas's worlds, the per-seed failure rate is 11.3 %: between the committed atlas's 10.0 % and
#686's 13.6 %. Extinction and energy death fall back, while generalist dominance and bloom stops
rise.

**Where lockup survives:**

| cell | lockup seeds | `u_λ` | λ |
|---|---:|---:|---:|
| atlas:47 | 7 | 0.215 | 0.0005 |
| atlas:80 | 4 | 0.000 | 0 |
| atlas:57, 62, 68 | 3 each | 0.46–0.77 | 0.0021–0.0060 |
| atlas:29, 82 | 2 each | 1.000, 0.447 | 0.0100, 0.0020 |
| atlas:36, 51, 67, 77, 81 | 1 each | 0.39–1.00 | 0.0015–0.0100 |

The two heaviest lockup cells this time sit at or near λ = 0, which is where leaching can't help.
The rest sit at high λ, as on #686, where the nutrient left locked is the share bound in
structure.

## 5. Reported, not required

**Flow 1** (#682's verdict block, seeds 1000–1004, T = 2000), decoded / fa 0:

| | committed (#683) | #686 | #711 |
|---|---|---|---|
| (1) median per-config ρ | −0.021 / −0.026 | −0.104 / −0.085 | −0.158 / −0.071 |
| (2) configs with ρ < 0, binomial p | 52 / 55 of 99, 0.69 / 0.31 | 54 / 51 of 83, 0.008 / 0.048 | 64 / 53 of 85, < 0.001 / 0.029 |
| (3) lineage clusters with median ρ < 0 | 5 / 5 of 8 | 7 / 5 of 8 | 7 / 5 of 8 |
| **conditionality** | FAIL / FAIL | PASS / PASS | **PASS / PASS** |
| **minority share**, second half | 61.6 / 59.3 % | 48.8 / 51.6 % | **44.7 / 52.8 %** |
| persisted seeds | 449 / 445 of 495 | 357 / 362 of 415 | 389 / 384 of 425 (91.5 / 90.4 %) |
| nutrient lockup seeds | – | 15 / 8 | 11 / 11 |

**Mobile autotrophy** (#683's census), runs to the horizon where mobile producers hold, decoded /
fa 0:

| effective mobility > | committed | #686 | #711 |
|---|---|---|---|
| 0.05 | 88 / 455, 149 / 458 | 125 / 367, 161 / 374 | 146 / 403, 196 / 406 |
| 0.3 | 49 / 455, 65 / 458 | 99 / 367, 101 / 374 | 113 / 403, 130 / 406 |
| 0.5 | 33 / 455, 41 / 458 | 66 / 367, 70 / 374 | 95 / 403, 89 / 406 |

**Fitness, coexistence, guilds** (each atlas under its own score, so the fitness rows don't
compare across columns):

| | committed | #686 | #711 |
|---|---:|---:|---:|
| median cell fitness (recorded) | 0.191 | 0.229 | 0.257 |
| best cell fitness (recorded) | 0.446 | 0.573 | 0.520 |
| mean coexistence fraction | 0.505 | 0.530 | 0.609 |
| median carcass axis | 0.107 | 0.109 | 0.093 |
| cells with a decomposer or consumer guild | 0 | 0 | 0 |

## 6. What this does not show

- **Whether a larger power or more seeds would clear the bar.** One search per score and one seed
  per search. The effect of k = 2 against k = 1 (+3.7 points of agreement) is within the range a
  different search seed might move it, which is unmeasured.
- **Whether 70 % is the right bar.** The binomial model in §2 says it asks for worlds at p ≥ 0.97.
  Whether genesis's worlds can be that robust under this physics at all, at any score, is open.
  The committed atlas, the most robust of the three, sits at 57.6 %.
- **Why the committed atlas is the most robust.** It was searched under the old surplus gate
  with `c_AH` in the box, and ranked on the median seed of 5, so it differs in physics, box and
  score at once. These runs hold physics and box fixed and change only the score, so they
  cannot say what made it more robust.
- **That leaching does not help robustness.** The search does not select for λ, and lockup
  persists both at λ ≈ 0 and at high λ. Neither run tests leaching against a world held fixed.
