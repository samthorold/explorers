# Issue #686: the re-search under robustness scoring and leaching, read by its fixed rule

**Status: measurement, docs only (2026-10-06). A fresh seed-42 atlas searched under main's
physics: ungated expression, recognition restraint 1, `c_AH` latent at 0, and leaching with λ in
the box on a square scale. It is scored by the mean over 10 seeds with dead seeds at 0. The atlas
is audited for fragility as #693 audited the committed one, and read by the rule fixed in #686
(the λ clause restated by #706 before the run). The committed atlas is not replaced here. That
decision is the owner's (#687).**

## TL;DR

**Both rules fail. Robustness got worse, not better, and λ shows no pull away from 0. Lockup
fell as a share of the dead frontier but not within the atlas's own worlds, and the lockup that
remains sits at high λ in worlds with high stoichiometric ratios. Flow 1's signatures came out
stronger.**

| rule | reading | verdict |
|---|---|---|
| **robustness improved:** all 10 unperturbed seeds agree in ≥ 70 % of worlds | 43.4 % (36 of 83), down from 57.6 % (57 of 99) | **FAIL** |
| **and** the atlas-wide mean live fraction rises | 0.864, down from 0.900 | **FAIL** |
| **leaching buys robustness:** ≤ 12.5 % of cells with `u_λ < 0.25` | 16.9 % (14 of 83) | **FAIL** |
| **and** Spearman ρ(λ, seed agreement) > 0 | −0.127 (one-sided permutation p = 0.87) | **FAIL** |
| deaths: lockup's share of the dead frontier | 44 % (30 of 68), down from 66 % (42 of 64) | falls, as expected |
| deaths: lockup among the atlas worlds' own seeds | 4.2 % (35 of 830), the same as before (42 of 990) | does not fall |

1. **The new atlas is less robust than the committed one, on every audit measure.** Its flip
   rates under jitter are higher at every radius (14.2 / 16.6 / 27.2 % at r = 0.01 / 0.03 / 0.1,
   against 11.0 / 13.5 / 19.8 %), and fewer of its worlds have a wide basin (33 of 83 against 53
   of 99). Scoring by the mean over 10 seeds did not steer the search toward robust worlds.
   Fitness rose instead: median cell fitness is 0.229, against 0.200 for the committed atlas
   re-scored by the same mean (#699), and the best cell is 0.573 against 0.445. This is #693's
   pattern again: the search climbs toward high-scoring worlds that sit near cliffs.
2. **λ is spread like an unselected coordinate.** Its share of cells below `u = 0.25` (16.9 %) is
   the median of the other 33 coordinates (16 %), and its ρ with seed agreement is slightly negative.
   The cells nearest λ = 0 are, if anything, the most robust: the bottom quarter has a mean live
   fraction of 0.93, against 0.82–0.88 above.
3. **The lockup that remains is not the kind leaching reaches.** Of the 12 cells with lockup
   seeds, 9 have λ ≥ λ_max/16 and 7 have λ ≥ λ_max/4. One locks up on 8 of 10 seeds at
   λ = 0.0034 (atlas:81), and one at λ's top, 0.01 (atlas:54). Against the lockup-free cells,
   the lockup-prone ones sit highest on `base_nutrient_ratio` (median `u` 0.84 vs 0.56) and
   `specification_nutrient_coefficient` (0.87 vs 0.60). Those two set the body's stoichiometric
   ratio, so more of a carcass's nutrient is bound in its structure, which leaching by design
   cannot release.
4. **Flow 1's signatures are stronger,** though they are reported, not required. Conditionality
   now passes all three tests in both modes, where on the committed atlas it failed the binomial
   test. The light-fed mixotrophs' second-half share is 48.8 % decoded and 51.6 % at fa 0, against
   61.6 / 59.3 %. Mobile producers hold in 27 % of runs, against 11 %.

## 1. What ran

| | |
|---|---|
| Tree | main `9cc934c` (#707). Release binaries pinned in `target/686/bin` (`explorers-search`, `fragility_audit`, `kin_killer_diet`) |
| Search | `explorers-search --batch 32 --generations 10 --max-ticks 2000 --seed 42 --output target/686/atlas.json --recipe-output target/686/recipe.json --checkpoint target/686/checkpoint.json`. Defaults otherwise: ensemble 10, mean over seeds (no top-up), bloom stop 300:10, refinement of the top 10 at n = 32 |
| Search wall clock | 3,285 s (55 min). Generations 7 and 10 took 11m43s and 12m20s; every other generation took 1–5 min. The pre-run estimate of 18–27 min (#699) missed these dense generations |
| Unfinished rollouts | 4 seed rollouts hit the 600 s budget (2 in each of generations 7 and 10), in neither the cells nor the frontier. 0 skipped |
| Atlas | 83 cells (coverage 1.04 %), QD score 20.58, best 0.573. 68 dead configs. Fingerprint `8452989377844692258`. Records `scoring: mean_over_seeds, ensemble_size 10` and λ's square scale |
| Recipe | cell [0, 19, 4], refined fitness 0.562 at n = 32. Its λ is 0.01, the top of the box |
| Audit | `env ATLAS=target/686/atlas.json K=target/686/fragility scripts/fragility-audit.sh`: as #693, seeds 1000–1009, 8 draws at radii 0.01 / 0.03 / 0.1, the search's rollout settings. 83 × 25 rows × 10 seeds = 20,750 rollouts in 8,218 s (2h17m), against #693's 1,842 s for 99 cells. 10 rollouts timed out |
| Census | `env ATLAS=target/686/atlas.json K=target/686/census scripts/683-census.sh`: seeds 1000–1004, T = 2000, decoded (322 s) and at fa 0 (301 s). No errors |
| Reproducibility | not rerun. The search is deterministic in its seed, but a second run costs another hour |

**Definitions.** *Seeds agree*: the unperturbed cell's 10 seeds all give the same verdict, so
its seed-only flip rate is 0. *Live fraction*: the share of the unperturbed cell's 10 seeds that
persist, as a mean over cells. *Seed agreement*, for ρ: 1 − the seed-only flip rate. Both
baselines are recomputed from #693's `target/fragility/summary.json`, and they reproduce its 57
of 99.

## 2. Robustness

| | committed (#693) | new (#686) |
|---|---:|---:|
| worlds | 99 | 83 |
| all 10 seeds agree | 57 (57.6 %) | 36 (43.4 %) |
| mean live fraction | 0.900 | 0.864 |
| worlds with live as the modal verdict | 96 | 80 |
| median seed-to-seed fitness sd | 0.149 | 0.157 |
| mean flip rate, r = 0.01 / 0.03 / 0.1 | 11.0 / 13.5 / 19.8 % | 14.2 / 16.6 / 27.2 % |
| basin width 0 / 0.1 | 22 / 53 | 23 / 33 |

The rule wants agreement to rise to ≥ 70 % **and** the live fraction to rise. Both fell. The
fall is not a cut-off artefact: the new atlas has fewer worlds at the robust end (basin 0.1) and
the same number at the fragile end (basin 0), out of fewer worlds.

## 3. λ

The read in `u`, the coordinate (λ = 0.01 · u²), where a search that does not select on λ
spreads it as it spreads any coordinate.

| | value |
|---|---:|
| cells with `u_λ` < 0.25 (λ < λ_max/16) | 14 of 83 (16.9 %) |
| the same share over the other 33 coordinates: min / median / max | 4 / 16 / 60 % |
| median `u_λ` (λ) | 0.588 (0.0035) |
| cells pinned at a bound, λ / every coordinate | 13 of 83 (15.7 %) / 313 of 2,822 (11.1 %) |
| ρ(λ, seed agreement) | −0.127 (one-sided permutation p = 0.87, 5,000 shuffles) |
| ρ(λ, live fraction) / ρ(λ, flip rate at r = 0.1) | −0.117 / +0.083 |
| λ's rank among the 34 coordinates by ρ with agreement (1 = most negative) | 7 |

| `u_λ` | cells | all seeds agree | mean live fraction |
|---|---:|---:|---:|
| [0, 0.25) | 14 | 6 | 0.929 |
| [0.25, 0.5) | 19 | 9 | 0.879 |
| [0.5, 0.75) | 26 | 9 | 0.815 |
| [0.75, 1] | 24 | 12 | 0.867 |

**The threshold's assumed baseline is too high.** #706 set 12.5 % as half of the 25 % an even
spread gives. The search does not spread coordinates evenly: it pulls them toward the middle, so
the median coordinate has only 16 % below 0.25. Against that baseline, λ is unremarkable either
way. The ρ clause does not depend on the baseline, and it is negative. The verdict holds on both
clauses.

## 4. Deaths by cause

| cause | committed: dead frontier | new: dead frontier | committed: atlas worlds' seeds | new: atlas worlds' seeds |
|---|---:|---:|---:|---:|
| nutrient lockup | 42 (66 %) | 30 (44 %) | 42 (4.2 %) | 35 (4.2 %) |
| extinction | 11 | 19 | 30 | 40 |
| bloom stop | 2 | 9 | 6 | 5 |
| energy death | 3 | 6 | 0 | 12 |
| monoculture | 6 | 3 | 17 | 21 |
| generalist dominance | 0 | 1 | 4 | 0 |
| total | 64 configs | 68 configs | 99 of 990 seeds | 113 of 830 seeds |

Lockup falls from two-thirds of the dead frontier to under half, as the rule expected if
leaching works. It still leads the frontier, ahead of extinction (19). Within the atlas's own
worlds, though, lockup's per-seed rate is unchanged, and extinction and energy death rose. The
new atlas fails more often (13.6 % of seeds against 10.0 %), and in more ways.

**Where lockup survives.** Per-seed lockups in the atlas's worlds, at radius 0:

| cell | lockup seeds | `u_λ` | λ |
|---|---:|---:|---:|
| atlas:81 | 8 | 0.586 | 0.0034 |
| atlas:40 | 6 | 0.488 | 0.0024 |
| atlas:56 | 6 | 0.702 | 0.0049 |
| atlas:48 | 4 | 0.538 | 0.0029 |
| atlas:39 | 2 | 0.128 | 0.0002 |
| atlas:54 | 2 | 1.000 | 0.0100 |
| atlas:70 | 2 | 0.614 | 0.0038 |
| atlas:7, 24, 31, 76, 79 | 1 each | 0.00–0.59 | 0–0.0034 |

#700's sweep found nearly every lockup on the committed atlas gone by λ_max/4. These worlds lock
up above it. Lockup-prone cells (≥ 2 lockup seeds, n = 7) against lockup-free ones (n = 71), the
largest gaps in median `u`:

| coordinate | lockup-prone | lockup-free |
|---|---:|---:|
| `base_nutrient_ratio` | 0.844 | 0.556 |
| `world_extent` | 0.731 | 0.447 |
| `specification_nutrient_coefficient` | 0.873 | 0.596 |
| `reserve_mobilisation_rate` | 0.333 | 0.541 |
| `solar_flux_magnitude` | 0.719 | 0.522 |

The two stoichiometric coefficients lead. A body with a high nutrient ratio binds more of a
carcass's nutrient in its structure, and leaching releases only what lies above that ratio
(world-rules.md, *The form*). The rest leaves only through a drain. With 7 cells this is
suggestive, not shown. A λ sweep over these cells, with the bound and leachable nutrient read
separately, would test it.

## 5. Reported, not required

**Flow 1** (`kin_killer_diet`, #682's verdict block), seeds 1000–1004, T = 2000:

| | committed decoded / fa 0 (#683) | new decoded / fa 0 |
|---|---|---|
| (1) median per-config ρ | −0.021 / −0.026 PASS | −0.104 / −0.085 PASS |
| (2) configs with ρ < 0 vs a coin | 52 / 55 of 99, p 0.69 / 0.32 FAIL | 54 / 51 of 83, p 0.008 / 0.048 PASS |
| (3) lineage clusters with median ρ < 0 | 5 / 5 of 8 PASS | 7 / 5 of 8 PASS |
| **conditionality** | **FAIL / FAIL** | **PASS / PASS** |
| pooled ρ | −0.179 / −0.165 | −0.168 / −0.160 |
| **minority share**, second half | 61.6 / 59.3 % FAIL | 48.8 % PASS / 51.6 % FAIL |
| persisted seeds | 449 / 445 of 495 | 357 / 362 of 415 (86.0 / 87.2 %) |

**Mobile autotrophy** (#683's census, section L), runs to the horizon where mobile producers
hold (≥ 5 above the threshold on every second-half sample tick, with births):

| effective mobility > | committed decoded / fa 0 | new decoded / fa 0 |
|---|---|---|
| 0.05 | 88 / 455, 149 / 458 | 125 / 367, 161 / 374 |
| 0.3 | 49 / 455, 65 / 458 | 99 / 367, 101 / 374 |
| 0.5 | 33 / 455, 41 / 458 | 66 / 367, 70 / 374 |
| producer samples above 0.3 | 53.0 / 46.5 % | 79.6 / 69.2 % |

**Mobile producers and robustness** (for #648's trigger): over the 83 worlds, ρ between the share
of producer samples above 0.3 and seed agreement is −0.107 decoded and −0.178 at fa 0. Against
median producer mobility it is −0.118 and −0.213. That is a weak tendency for worlds with more
mobile producers to disagree across seeds more. At n = 83, ρ must pass about ±0.22 to clear
p = 0.05, and none does.

**Fitness, coexistence, guilds** (the cell fields; the committed atlas's fitness is the median
of 5, so the #699 re-scoring is shown too):

| | committed | committed, re-scored mean of 10 (#699) | new |
|---|---:|---:|---:|
| median cell fitness | 0.191 | 0.200 | 0.229 |
| best cell fitness | 0.446 | 0.445 | 0.573 |
| mean coexistence fraction | 0.505 | – | 0.530 |
| median carcass axis | 0.107 | – | 0.109 |
| cells with a decomposer or consumer guild | 0 | – | 0 |

## 6. What this does not show

- **Which change made the atlas less robust.** The new atlas differs from the committed one in
  scoring (mean of 10), box (`c_AH` out, λ in) and search draw at once. The committed atlas was
  also *searched* under the old surplus gate, though both were *audited* under today's physics.
  One seed per search cannot separate these effects, and seed-to-seed spread between searches
  is unmeasured here.
- **That leaching does nothing for robustness.** It shows the search did not select for it.
  Lockup fell in the dead frontier. The remaining lockups sit where leaching, as specified, has
  little to act on.
- **Why scoring by the mean over seeds did not find robust worlds.** Dead seeds score 0, but
  fitness still rewards oscillation, turnover and coexistence, and a world that is live on 7
  seeds with high fitness can outscore one that is live on 10 with low fitness. The mean
  sharpened the climb rather than changing its target. Whether robustness should enter the score
  directly (for example, a cell's live fraction as a gate or a factor) is a design question for
  the owner.
