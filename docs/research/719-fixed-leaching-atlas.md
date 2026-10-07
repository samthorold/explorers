# Issue #719: a fresh search on the fixed-λ box, read by a recipe-centred rule

**Status: measurement, docs only (2026-10-07). A seed-42 search on main's default box since #716,
the 33-coordinate untaxed box with the leaching rate held at 0.0025 outside it, scored by `L² · F`
and projected by #715's robust pick. The command is #711's, with only the box changed. The
fragility audit and the census were run as #711 ran them. The atlas is read by the rule fixed in #719
before the run. Replacing the committed atlas is #687's job.**

## TL;DR

**The atlas passes the rule. Its pick is the top cell by fitness, [5, 19, 1], which coexists on
all 32 refined seeds and lives on all 10 fresh audit seeds at a refined fitness of 0.704: the most
robust and the best recipe any of these atlases has offered. #687 therefore replaces the
committed atlas with this one. The map around the pick is less robust than #711's and of higher
quality. Flow 1's signatures fail on it, and for the first time producer heterotrophy runs
backwards across the pooled worlds, in the recipe's own world too.**

| clause | reading | verdict |
|---|---|---|
| 1. the pick clears 0.90, not the fallback | [5, 19, 1] (atlas:31), refined coexistence 32/32 = 1.00, after 1 refinement | **PASS** |
| 2. the pick's refined fitness ≥ 0.238 | 0.704 (committed recipe 0.297; #711's reprojected pick 0.331) | **PASS** |
| 3. no unfinished rollouts, or the pick unaffected | 5 search rollouts hit the budget, none in the atlas: all 78 cells rest on 10 finished seeds, and the pick's 10 search and 32 refinement seeds all finished | **PASS** (qualified) |

1. **The recipe is robust and good.** The search's best cell cleared the floor on the first
   refinement. On the audit's fresh seeds 1000–1009 it lives on all 10 (live-seed fitness
   0.67–0.81), and no seed flips under jitter at radius 0.01 or 0.03 (basin 0.03). It persists
   on all 5 census seeds in both modes. It runs at `λ = 0.0025`, which the atlas records as
   `fixed`.
2. **The map is less robust than #711's.** 41.0 % of worlds agree on all 10 audit seeds (#711
   47.1 %), the mean live fraction is 0.849 (#711 0.887), and flip rates under jitter are a few
   points higher at every radius. The median live-seed fitness rises from 0.276 to 0.334. These
   are readouts since #714, not bars. With the seed and score held fixed, the box and the λ
   regime both changed, so the run cannot say which moved them (§6).
3. **Flow 1's signatures fail, and heterotrophy runs backwards.** Conditionality fails in both
   modes on all three tests. The pooled ρ is positive (+0.087 decoded, +0.021 at fa 0), where the
   committed atlas and #711 both had it clearly negative. In the recipe's own world, ρ is +0.630
   decoded and +0.367 at fa 0. Light-fed mixotrophs drain 60 % of the settled half's carcass
   structure. Flow 1 reads these as signatures, not requirements (world-rules.md, flow 1), so
   they do not enter the rule. They are flagged for the owner as the readout this run most
   changed.
4. **Lockup does not fall at fixed λ.** It leads the dead frontier again (25 of 56 configs), and
   3.97 % of the atlas worlds' audit seeds lock up, against 3.4 % on #711. Monoculture and
   generalist dominance rise among the worlds' seeds, as #700 found monoculture rising at every
   λ > 0.
5. **The search and the audit were slow.** The search took 1h32m and the audit 3h42m. Dense
   worlds dominated both (§1).

## 1. What ran

| | |
|---|---|
| Tree | main `1ff1fdd` (#716: λ fixed at 0.0025 outside the box; #715: the robust pick). Search binary pinned in `target/719/bin`; the audit and census drivers pin their own |
| Search | `explorers-search --batch 32 --generations 10 --max-ticks 2000 --seed 42 --output target/719/atlas.json --recipe-output target/719/recipe.json --checkpoint target/719/checkpoint.json`. The atlas records `fixed: {leaching_rate: 0.0025}` and `scoring: {live_fraction_power: 2}`, ensemble 10 |
| Search wall clock | 5,513 s (1h32m), against #711's 1,742 s and #686's 3,285 s. Four dense generations took 10m27s, 28m25s, 15m26s and 19m34s; the other seven 44 s – 4m51s |
| Unfinished rollouts | 5: 3 in generation 8, 2 in generation 10. None is in a recorded cell (TL;DR, clause 3) |
| Atlas | 78 cells (coverage 0.975 %), QD score 21.38, best 0.708. 56 dead configs |
| Recipe | cell [5, 19, 1] (atlas:31), recorded fitness 0.708, refined 0.704 at n = 32, coexistence 1.00 → 1.00. Picked after 1 refinement; the 77 lower cells were not refined. `λ = 0.0025` |
| Audit | `env ATLAS=target/719/atlas.json K=target/719/fragility scripts/fragility-audit.sh`: seeds 1000–1009, 8 draws at radii 0.01 / 0.03 / 0.1. 19,500 rollouts in 13,313 s (3h42m). 6 timed out. Ten cells cost 5–34 min each and 9,075 s between them: atlas:0, 4, 7, 23, 26, 36, 50, 53, 63, 74 |
| Census | `env ATLAS=target/719/atlas.json K=target/719/census scripts/683-census.sh`: decoded and fa 0, 1,423 s. No errors |
| Driver | the three steps chained with `.done` markers (`target/719/run.sh`), under `caffeinate` |

Definitions are #686's and #711's. *All seeds agree*: the unperturbed cell's 10 audit seeds share
one verdict. *Live fraction*: the mean over cells of the share of each cell's 10 unperturbed
seeds that persist. *Live-seed F*: the mean fitness of a cell's live audit seeds, with the median
over cells. The script that reads them reproduces #711's published figures from `target/711`
exactly.

## 2. Robustness, four atlases side by side

| | committed (#693) | #686 (mean) | #711 (L² · F) | **#719 (L² · F, fixed λ)** |
|---|---:|---:|---:|---:|
| worlds | 99 | 83 | 85 | **78** |
| all 10 seeds agree | 57 (57.6 %) | 36 (43.4 %) | 40 (47.1 %) | **32 (41.0 %)** |
| mean live fraction | 0.900 | 0.864 | 0.887 | **0.849** |
| median live-seed F | 0.228 | 0.281 | 0.276 | **0.334** |
| median seed-to-seed fitness sd | 0.149 | 0.157 | 0.164 | **0.191** |
| mean flip rate, r = 0.01 / 0.03 / 0.1 | 11.0 / 13.5 / 19.8 % | 14.2 / 16.6 / 27.2 % | 14.3 / 16.3 / 23.5 % | **17.0 / 19.2 / 27.2 %** |
| basin width 0 / 0.1 | 22 / 53 | 23 / 33 | 23 / 37 | **29 / 28** |
| audit rollouts timed out | 0 | 10 | 1 | **6** |

**Selection on the search's own seeds.** Recorded fitness averages 0.274, against 0.248 for the
same cells' `L² · F` recomputed on the audit's fresh seeds (median 0.260 → 0.235). 50 cells fall
and 28 rise. The winner's curse is the same size as #711's (8 % and 9 %).

**The fitness and robustness of the map trade off, the pick does not.** Live-seed F is the
highest of the four atlases and agreement the lowest. Fragility still falls with fitness across
cells (ρ(seed-only flip rate, fitness) = −0.349), and the pick, the fittest cell, is among the 32
whose seeds all agree.

## 3. λ

λ is not a coordinate of this box, so #706's clause has nothing to read. Every world ran at
`λ = 0.0025`. #711's worlds spread λ over `[0, 0.01]` (median 0.0043), and its bottom quarter
`λ < 0.000625` was its most robust (12 of 15 cells agree, live fraction 0.91). The worlds here
all sit above that quarter.

## 4. Deaths by cause

| cause | dead frontier: committed / #711 / **#719** | atlas worlds' seeds: committed / #711 / **#719** |
|---|---|---|
| nutrient lockup | 42 / 20 / **25** | 42 / 29 / **31** |
| extinction | 11 / 17 / **9** | 30 / 20 / **23** |
| monoculture | 6 / 5 / **9** | 17 / 20 / **29** |
| bloom stop | 2 / 2 / **11** | 6 / 12 / **4** |
| energy death | 3 / 0 / **1** | 0 / 3 / **2** |
| generalist dominance | 0 / 0 / **1** | 4 / 12 / **29** |
| total | 64 / 44 / **56** configs | 99 of 990 / 96 of 850 / **118 of 780** seeds |
| lockup's share | 66 / 45 / **45** % | 4.2 / 3.4 / **4.0** % |

The per-seed failure rate within the atlas's worlds is 15.1 %, against 11.3 % on #711 and 10.0 %
on the committed atlas. Lockup is flat. The rise is in monoculture and generalist dominance, the
two failures where one kind of organism takes the world.

**Where lockup survives:** atlas:23 and 77 (5 seeds each), 26 and 66 (4 each), 6, 47 and 75
(2 each), and seven cells with 1. Two of them, 23 and 77, are the only cells whose modal verdict
is lockup. Under leaching the design expects the lockup that persists to be nutrient bound in
structure, in worlds with high stoichiometric ratios (world-rules.md, *The rate*). That is not
what these cells show: their median `base_nutrient_ratio` coordinate is 0.48, below the other
cells' 0.61. The run does not say what locks them.

## 5. Reported, not required

**Flow 1** (#682's verdict block, seeds 1000–1004, T = 2000), decoded / fa 0:

| | committed (#683) | #711 | **#719** |
|---|---|---|---|
| (1) median per-config ρ | −0.021 / −0.026 | −0.158 / −0.071 | **−0.010 / +0.033** |
| (2) configs with ρ < 0, binomial p | 52 / 55 of 99, 0.69 / 0.32 | 64 / 53 of 85, < 0.001 / 0.029 | **41 / 35 of 78, 0.73 / 0.43** |
| (3) lineage clusters with median ρ < 0 | 5 / 5 of 8 | 7 / 5 of 8 | **3 / 3 of 8** |
| **conditionality** | FAIL / FAIL | PASS / PASS | **FAIL / FAIL** |
| pooled ρ (does not run backwards if ≤ 0) | −0.179 / −0.165 | −0.143 / −0.125 | **+0.087 / +0.021** |
| **minority share**, second half | 61.6 / 59.3 % | 44.7 / 52.8 % | **60.1 / 59.5 %** |
| persisted seeds | 449 / 445 of 495 | 389 / 384 of 425 | **333 / 343 of 390 (85.4 / 87.9 %)** |
| nutrient lockup seeds | – | 11 / 11 | **15 / 8** |

The recipe's world reads ρ = +0.630 decoded (16,837 producer samples) and +0.367 at fa 0: its most
heterotrophic producers sit on its richest ground, the arrangement world-rules.md says the
carnivorous-plant clause does not justify (*Satiation is co-limited*). It kills kin at 8.6 per
1,000 producer agent-ticks decoded (4.1 at fa 0), above the atlas's median config (2.2 decoded;
#711's 1.7), within its range (0.07–61).

**Mobile autotrophy** (#683's census), runs to the horizon where mobile producers hold, decoded /
fa 0:

| effective mobility > | committed | #711 | **#719** |
|---|---|---|---|
| 0.05 | 88 / 455, 149 / 458 | 146 / 403, 196 / 406 | **138 / 362, 175 / 377** |
| 0.3 | 49 / 455, 65 / 458 | 113 / 403, 130 / 406 | **75 / 362, 80 / 377** |
| 0.5 | 33 / 455, 41 / 458 | 95 / 403, 89 / 406 | **51 / 362, 50 / 377** |

**Fitness, coexistence, guilds** (each atlas under its own score; the committed column's
fitness rows don't compare):

| | committed | #711 | **#719** |
|---|---:|---:|---:|
| median cell fitness (recorded) | 0.191 | 0.257 | **0.260** |
| best cell fitness (recorded) | 0.446 | 0.520 | **0.708** |
| mean coexistence fraction | 0.505 | 0.609 | **0.629** |
| median carcass axis | 0.107 | 0.093 | **0.092** |
| cells with a decomposer or consumer guild | 0 | 0 | **1** |

The one guild cell is atlas:26 ([4, 19, 5]): a decomposer guild on 1 of its 10 search seeds. It is
also one of the lockup cells and the slowest audit cell (34 min).

## 6. What this does not show

- **What moved the map.** Against #711 the box lost its λ coordinate and every world moved to
  `λ = 0.0025`, so the emitters' geometry and the physics changed at once. The flip in flow 1,
  the lower agreement and the higher F could come from either, or from the different archive a
  33-dimensional search builds from the same seed. A second search seed would show how much is
  the draw; a fixed-world comparison at `λ = 0` and `0.0025` would show how much is the rate.
- **That the recipe stays robust under play.** Robustness to perturbation mid-run is what the game
  turns on, and it is read on settled worlds, not here (genesis-search.md, *Genesis selects for
  worlds that are sensible across initial conditions*).
- **Why heterotrophy runs backwards.** The census reports the pooled and per-config ρ. It does not
  separate the drain economics behind them. Flow 1's signatures are readouts; their failure does
  not by itself bring physics out of reserve (world-rules.md, flow 1).
- **Reproducibility.** The search was not rerun. Its 5 unfinished rollouts depend on wall clock,
  so a faster or slower machine could finish them and enter different cells. None of the atlas's
  cells rests on one.
