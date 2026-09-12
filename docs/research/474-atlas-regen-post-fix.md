# Issue #474 — regenerate the atlas and recipe on the fixed stepper (post #444–#453)

**Status: data regeneration with a before/after reading. Commits `atlas.json` and
`recipe.json`; changes no code.** Six stepper fixes landed after the committed atlas was
produced — founder trait floor (#444), metabolic overdraft (#445), nutrient bind (#446),
chemotaxis order (#451), drain stoichiometry order (#452), carcass nutrient release
(#453). #444 alone changed 57 % of founders across the search box, so every live cell,
the dead frontier and the projected recipe were computed on physics that no longer
exists. This note records the regeneration and reads the shift, attributing to a fix only
what the numbers support. Search behaviour, ranges, the prefilter and the evaluator are
untouched (acceptance criterion 5); the research bins that consume the atlas
(`permanence_crosscheck`, `energy_bound_check`, `role_emergence`, `step_profile`) are
re-run under #476, not here, and `439-`/`433-` are left as written.

## TL;DR

1. **The search behaves.** 82 / 8000 cells live (1.025 %), QD-score 38.98, best fitness
   0.799; 51 dead configs. No population-ceiling explosion, no energy-death gating, and
   the n = 32 refinement found a robust cell on its first pick. The item-4 stop condition
   did not fire.
2. **More live cells, higher fitness, same live *config* rate.** Live cells 56 → 82
   (+46 %), QD-score 22.09 → 38.98, best fitness 0.674 → 0.799, median live fitness
   0.485 → 0.510. But the search budget is fixed at 32 × 11 = 352 configs and the dead
   frontier moved only 48 → 51, so live *configs* are 304 → 301: the fixed physics does
   not make more configs viable, it spreads the viable ones over more behaviour cells
   (carcass axis 6 → 8 occupied bins, oscillation and clustering similar) and lifts the
   elite in each.
3. **The dead frontier's composition rotated from extinction to monoculture.** Extinction
   19 → 10, monoculture 15 → 29, nutrient lockup 12 → 11, generalist dominance 2 → 1. The
   direction is exactly what #444 predicts — founders that used to be culled on tick 1
   (NaN maintenance from a negative trait) now live, and a population that lives but does
   not diversify is a monoculture, not an extinction — and it is the only shift with a
   named cause; see §4.
4. **Coexistence is more robust.** `coexistence_fraction ≥ 0.8` on 32 / 56 (57 %) → 58 / 82
   (71 %) live cells; mean 0.72 → 0.77; `== 1.0` on 25 → 36. Cells at `0.0` went 1 → 4.
5. **The recipe moved and is now a refined pick.** Old: cell [9, 13, 4], fitness 0.674,
   `coexistence_fraction` 0.6 at n = 5, *never refined* (the #404 refinement landed after
   `recipe.json` was last written in #399). New: cell [5, 19, 7], fitness 0.799,
   coexistence 0.80 (n = 5) → **0.91 (n = 32)**, refined fitness 0.722. It is a producer-led
   world (`photosynthetic_absorption` mean 1.0, `heterotrophy` 0.09) with three founder
   clusters, low mutation rate (0.01) and a deep carcass layer (carcass descriptor 0.36,
   `decomposer_fraction` 0.6).
6. **Every top-10 cell sits at clustering index 19** (`clustering_strength ≥ 0.95` — an
   empty valley between two peaks in the pairwise trait-distance histogram, i.e. fully
   separated trait clusters). This is an intensification, not a new phenomenon: the old
   top-10 had 7 / 10 at index 19 and the old best cell was at 13; overall 24 / 56 (43 %) →
   39 / 82 (48 %) of live cells are at 19, and mean clustering 0.63 → 0.74. The fitness
   objective rewards clustering, so the elite concentrating at its ceiling is the search
   working as designed; the reading is that the fixed physics reaches the ceiling more
   often.

## 1. Provenance

| | before | after |
|---|---|---|
| `atlas.json` from | PR #405 (`e5d6c63`, 2026-06-08), regenerated for `coexistence_fraction` | this PR |
| `recipe.json` from | PR #399 (`d1c1ae4`, 2026-06-07); #405 left it unchanged | this PR |
| stepper commit | pre-#444 | `5a7bede` (last of the six fixes, #473; the four later PRs #478–#481 are byte-identical constant-naming and a research-doc correction) |
| command | `explorers-search --batch 32 --generations 10 --ensemble 5 --max-ticks 500 --seed 42` | same, with the #434 defaults `--refine-top-k 10 --refine-ensemble 32` spelled out |
| wall-clock | — | ≈ 20 min (release build, clean detached worktree), exit 0 |
| configs evaluated | 32 × (10 + 1) = 352 | 352 |
| `rollouts_skipped` / a-priori frontier / prefilter disagreements | 0 / {} / 0 | 0 / {} / 0 |

The search's stderr (refinement table, coverage, top cells, dead frontier, bifurcation
cross-check summary) is reproduced in §6. Raw before/after numbers were extracted with a
throwaway Python script over the two JSON files; nothing in the repo depends on it.

## 2. Atlas summary

| | before | after | Δ |
|---|---|---|---|
| live cells (of 8000) | 56 (0.70 %) | 82 (1.03 %) | +26 |
| dead configs | 48 | 51 | +3 |
| live configs (352 − dead) | 304 | 301 | −3 |
| QD-score (Σ elite fitness) | 22.09 | 38.98 | +16.9 |
| best fitness | 0.674 | 0.799 | +0.125 |
| median live fitness | 0.485 | 0.510 | +0.025 |
| mean live fitness | 0.394 | 0.475 | +0.081 |
| min live fitness | 0.017 | 0.020 | — |
| shared cells (same index live in both) | — | 35 | 21 old-only, 47 new-only |
| bifurcation cross-check disagreements | 416 | 316 (112 validated-regime branching, 204 weak-observable oscillation) | −100 |

Every live cell in both atlases has `sample_count = 5`.

### Occupied behaviour-axis bins (20 per axis)

| axis | before: distinct bins | after: distinct bins | before histogram | after histogram |
|---|---|---|---|---|
| oscillation (0) | 13 (0–12) | 12 (0–11) | 0:1 1:2 2:3 3:4 4:7 5:6 6:10 7:8 8:4 9:4 10:4 11:2 12:1 | 0:5 1:2 2:4 3:7 4:8 5:13 6:9 7:11 8:8 9:6 10:7 11:2 |
| clustering (1) | 10 | 11 | 0:16 10:2 11:3 12:1 13:2 14:2 16:2 17:1 18:3 **19:24** | 0:12 7:1 10:5 12:3 13:7 14:4 15:4 16:3 17:2 18:2 **19:39** |
| carcass (2) | 6 (0–5) | 8 (0–7) | 0:30 1:14 2:5 3:3 4:3 5:1 | 0:30 1:19 2:16 3:8 4:4 5:2 6:2 7:1 |

Mean descriptors over live cells: oscillation 0.326 → 0.310, clustering 0.633 → 0.737,
carcass 0.066 → 0.100. The carcass axis is where the new coverage is: bins 2–7 hold 12
cells before and 33 after. The clustering axis is bimodal in both atlases (a block at 0 —
`clustering_strength` reads exactly 0 below n ≈ 4 or with no valley — and a block at 19)
and the 19 block grew from 43 % to 48 % of live cells.

### Per-seed distributions

| `coexistence_fraction` | 0.0 | 0.2 | 0.4 | 0.6 | 0.8 | 1.0 | mean | median | ≥ 0.8 |
|---|---|---|---|---|---|---|---|---|---|
| before (56) | 1 | 5 | 10 | 8 | 7 | 25 | 0.721 | 0.8 | 32 (57 %) |
| after (82) | 4 | 3 | 5 | 12 | 22 | 36 | 0.773 | 0.8 | 58 (71 %) |

| `decomposer_fraction` | 0.0 | 0.2 | 0.4 | 0.6 | 0.8 | 1.0 |
|---|---|---|---|---|---|---|
| before (56) | 9 | 12 | 15 | 13 | 3 | 4 |
| after (82) | 11 | 21 | 20 | 17 | 10 | 3 |

### Dead frontier (configs by cliff)

| cliff | before | after | Δ |
|---|---|---|---|
| extinction | 19 | 10 | −9 |
| monoculture | 15 | 29 | +14 |
| nutrient lockup | 12 | 11 | −1 |
| generalist dominance | 2 | 1 | −1 |
| population explosion | 0 | 0 | 0 |
| energy death | 0 | 0 | 0 |
| **total** | **48** | **51** | **+3** |

### Top-10 live cells

Before (#405 atlas):

| cell | fitness | osc | clus | carcass | decomposer | coexist |
|---|---|---|---|---|---|---|
| [9, 13, 4] | 0.674 | 0.481 | 0.690 | 0.224 | 0.6 | 0.6 |
| [7, 19, 3] | 0.645 | 0.358 | 1.000 | 0.163 | 0.2 | 1.0 |
| [5, 11, 5] | 0.627 | 0.270 | 0.592 | 0.276 | 0.4 | 1.0 |
| [5, 19, 3] | 0.612 | 0.291 | 1.000 | 0.188 | 0.0 | 1.0 |
| [6, 19, 4] | 0.601 | 0.349 | 1.000 | 0.242 | 0.0 | 1.0 |
| [5, 19, 2] | 0.596 | 0.252 | 1.000 | 0.128 | 0.2 | 1.0 |
| [7, 19, 2] | 0.595 | 0.388 | 1.000 | 0.132 | 0.6 | 1.0 |
| [7, 19, 1] | 0.587 | 0.376 | 1.000 | 0.052 | 0.2 | 1.0 |
| [5, 19, 0] | 0.581 | 0.272 | 1.000 | 0.036 | 0.4 | 1.0 |
| [8, 17, 2] | 0.573 | 0.425 | 0.889 | 0.119 | 0.4 | 1.0 |

After (this PR), with the n = 32 refinement column from the search log:

| cell | fitness | osc | clus | carcass | decomposer | coexist n=5 | coexist n=32 | refined fitness |
|---|---|---|---|---|---|---|---|---|
| **[5, 19, 7]** | **0.799** | 0.271 | 1.000 | 0.359 | 0.6 | 0.80 | **0.91** | 0.722 |
| [4, 19, 3] | 0.767 | 0.218 | 1.000 | 0.197 | 0.8 | 1.00 | 0.94 | 0.508 |
| [10, 19, 4] | 0.739 | 0.516 | 1.000 | 0.229 | 0.2 | 0.80 | 1.00 | 0.598 |
| [10, 19, 2] | 0.738 | 0.537 | 1.000 | 0.118 | 0.0 | 1.00 | 0.97 | 0.506 |
| [6, 19, 0] | 0.722 | 0.304 | 1.000 | 0.048 | 0.6 | 1.00 | 0.97 | 0.605 |
| [3, 19, 3] | 0.716 | 0.182 | 1.000 | 0.157 | 0.8 | 1.00 | 1.00 | 0.696 |
| [2, 19, 0] | 0.710 | 0.108 | 1.000 | 0.049 | 1.0 | 1.00 | 0.94 | 0.748 |
| [7, 19, 3] | 0.707 | 0.384 | 0.995 | 0.189 | 0.4 | 0.80 | 0.84 | 0.574 |
| [8, 19, 0] | 0.704 | 0.436 | 1.000 | 0.032 | 0.6 | 0.80 | 0.84 | 0.599 |
| [4, 19, 2] | 0.680 | 0.226 | 1.000 | 0.122 | 0.8 | 0.80 | 0.81 | 0.521 |

All ten refined cells clear `COEXISTENCE_FLOOR = 0.5` (✓ in the log); the recipe is the
highest in-run-fitness cell among them, as the projection specifies. Note the refined
fitness is below the in-run fitness on nine of ten cells (0.02–0.26 lower), the expected
regression-to-the-mean of a 5-seed elite re-drawn at 32 seeds; [2, 19, 0] is the only one
that *rose* (0.710 → 0.748) and [4, 19, 3] the most optimistic elite (0.767 → 0.508).

## 3. Recipe

| | before ([9, 13, 4]) | after ([5, 19, 7]) |
|---|---|---|
| in-run fitness (n = 5) | 0.674 | 0.799 |
| `coexistence_fraction` n = 5 | 0.6 | 0.8 |
| `coexistence_fraction` n = 32 | not refined | 0.91 |
| refined fitness (n = 32) | — | 0.722 |
| descriptors (osc, clus, carcass) | 0.48, 0.69, 0.22 | 0.27, 1.00, 0.36 |
| `decomposer_fraction` | 0.6 | 0.6 |

Parameters that moved materially (full diff is `git diff` on `recipe.json`; unchanged
fixed values omitted):

| parameter | before | after |
|---|---|---|
| `solar_flux_magnitude` | 18.72 | 10.59 |
| `base_trophic_efficiency` | 0.638 | 0.263 |
| `trophic_distance_decay` | 3.54 | 0.394 |
| `reproduction_efficiency` | 0.802 | 0.564 |
| `base_metabolic_rate` | 0.184 | 0.277 |
| `movement_cost_coefficient` | 0.0735 | 0.0173 |
| `sensing_range_coefficient` | 1.0 | 7.35 |
| `reproduction_energy_threshold` | 27.9 | 49.3 |
| `mutation_rate` | 0.184 | 0.01 (range floor) |
| `contact_range_coefficient` | 2.22 | 4.30 |
| `world_extent` | 77.1 | 99.6 |
| `light_competition_radius` | 7.12 | 8.92 |
| `photo_maintenance_cost` | 0.0729 | 0.1 (range ceiling) |
| `heterotrophy_maintenance_cost` | 0.0426 | 0.0210 |
| `base_nutrient_ratio` | 0.336 | 0.5 (range ceiling) |
| `specification_nutrient_coefficient` | 0.467 | 0.326 |
| `reproductive_compatibility_distance` | 1.76 | 3.73 |
| `maintenance_cost_exponent` | 2.71 | 3.0 (range ceiling) |
| `growth_retention_multiplier` | 1.22 | 1.41 |
| `reserve_mobilisation_rate` | 0.406 | 0.05 (range floor) |
| `offspring_structure_fraction` | 0.352 | 0.457 |
| founder `photosynthetic_absorption` | 0.588 | 1.0 (ceiling) |
| founder `heterotrophy` | 0.465 | 0.092 |
| founder `asexual_propensity` | 0.325 | 1.0 (ceiling) |
| founder `dispersal` | 0.891 | 0.629 |
| `trait_covariance` | 0.607 | 0.816 |
| `initial_cluster_count` | 1 | 3 |
| `initial_energy_per_agent` | 19.0 | 37.2 |

The new recipe is a different world, not a re-tuned one: a low-flux, low-trophic-
efficiency, producer-founded world with three founder clusters, near-zero mutation, a
large sensing range and slow reserve mobilisation, whose descriptors say the trait
clusters stay fully separated (clustering 1.0) and a third of the biomass is carcass at
the horizon. Six of its coordinates sit on a range boundary (`mutation_rate`,
`photo_maintenance_cost`, `base_nutrient_ratio`, `maintenance_cost_exponent`,
`reserve_mobilisation_rate`, two founder traits at 1.0) — the atlas top-10 has always
sat on boundaries and this note does not read that as a reason to move the box (item 4).

## 4. Attribution

Six fixes, one search. The regeneration is a single seed-42 run on the new physics
against a single seed-42 run on the old, and the outer search is itself a trajectory
(CMA-MAE emitters condition on what they found), so *any* change to the stepper — even a
last-ulp one — re-routes which configs get evaluated after generation 1. A cell-by-cell
diff therefore cannot be read as "this fix caused this cell"; only aggregate shifts with
a mechanism that predicts their direction are attributable. What the six fixes do to a
trajectory, from their PR descriptions:

| fix | what changed in the stepper | expected effect on a trajectory |
|---|---|---|
| #444 founder trait floor (`fc52e74`) | founder traits sampled as centroid + Normal are floored at 0; previously 57 % of search-box founders carried a negative trait → NaN maintenance under a non-integer exponent (agent silently dropped on tick 1) or a negative charge under an odd one | **large, systematic**: founders that died on tick 1 now live; seeds with no negative draws are byte-identical |
| #445 metabolic overdraft (`be21bf0`) | charge capped at available reserve; death path previously floored the negative reserve to 0 | **none on the trajectory** — the agent dies at the same tick with the same state; only the dissipation ledger entry changes |
| #446 nutrient bind (`e08c2cc`) | nutrient-limited growth binds the store to exactly 0 instead of −1 ulp | ulp-level; measure-zero unless a downstream comparison sits on the sign |
| #451 chemotaxis order (`2301455`) | neighbours sensed at tick-start positions, attraction summed in stable-id order | trajectory-changing for every config with mobile sensing agents; no systematic direction |
| #452 drain stoichiometry order (`15b193b`) | stoichiometric need at tick-start structure; drains in stable-id order; spent carcasses stay targets while they hold stock | trajectory-changing wherever consumption occurs; spent carcasses now mineralise instead of stranding nutrient → *less* stranded nutrient, if anything |
| #453 carcass nutrient release (`5a7bede`) | consumer shares taken against tick-start carcass nutrient; rounding remainder mineralises on exhaustion | as #452, smaller |

What the numbers support:

- **Extinction 19 → 10 and monoculture 15 → 29 are attributable to #444.** Under the old
  physics a config whose founders drew negative traits lost a large fraction of them on
  tick 1 — with founding populations of tens of agents and 57 % of founders affected, that
  was routinely enough to push a marginal config to extinction within 500 ticks. Under
  the new physics those founders live; a founding population that lives but whose trait
  clusters merge (or whose consumer compartment never gets going) ends as a monoculture,
  which the evaluator scores as a cliff, not as extinction. The total dead count barely
  moved (48 → 51), consistent with the same *configs* being marginal and the cliff they
  fall off being relabelled. This is the direction #444's PR predicted ("the
  full-crosscheck test runs 160 ticks so it can observe the guaranteed extinction now
  that founders survive their first tick") and no other fix has a mechanism that moves
  configs *between* those two cliffs.
- **Nutrient lockup 12 → 11 is not evidence for or against #452/#453.** Those fixes
  release stranded carcass nutrient, which would predict *fewer* lockups; a change of one
  on a base of twelve is within the outer search's re-routing noise.
- **The +26 live cells, +0.125 best fitness and the carcass-axis spread (6 → 8 occupied
  bins, 12 → 33 cells in bins ≥ 2) are consistent with #444 but not attributable by
  measurement here.** Founders that live are more biomass, more carcass and more
  trait variance from tick 1 — every direction the fitness objective and the carcass
  descriptor reward — but the same outcome is reachable by the outer search simply
  finding a better basin on a re-routed trajectory. Separating those would need the
  #444-only stepper (the six fixes are sequential commits, so it is a one-line checkout
  and a 20-minute run); it is not done here because item 4 says not to tune, and a
  finer attribution changes no decision.
- **#445 contributes nothing to any of these numbers**, by construction (it changes the
  ledger, not the state). Any doc that attributes an atlas shift to #445 is wrong.
- **The clustering-19 concentration** (§2, TL;DR 6) is intensified, not created: the old
  atlas already had 7 / 10 of its top-10 and 43 % of its live cells there. Not
  attributed.
- **Bifurcation cross-check disagreements 416 → 316** track the change in which configs
  were evaluated and are not read further; the #358/#359 gating on the observables is
  unchanged.

## 5. Tests

- `crates/explorers-search/tests/prefilter_atlas.rs` (new in #480) decodes every live
  cell of the committed atlas through the search's decoder and asserts it clears the
  energy-death gate. Passes on the new atlas without change — the gate did not move and
  no live cell sits near it.
- `cargo test --workspace`: 583 passed, 0 failed, 0 ignored — identical to the pre-PR
  baseline. **No test pinned an emergent number from the old atlas**, so nothing was
  re-pinned. (The `grep` for `atlas.json` in test code finds only `prefilter_atlas.rs`;
  the four research bins read it at run time and are #476's re-run.)
- The root-level `test_fitness.rs` is deleted: it was a stray from 2026-05-24 that used
  stale field names, was not a Cargo target and was referenced nowhere.

## 6. Search log (stderr, verbatim)

```
Running QD genesis search (CMA-MAE atlas)...
  Batch size: 32
  Generations: 10
  Ensemble size: 5
  Max ticks: 500
  Seed: 42

Refining top-10 live cells at ensemble n=32 (independent seeds)...
Refined cells (recorded → refined coexistence fraction):
  cell [5, 19, 7]: fitness=0.7986 coexist 0.80 → 0.91 (n=32) refined_fit=0.7222 ✓
  cell [4, 19, 3]: fitness=0.7673 coexist 1.00 → 0.94 (n=32) refined_fit=0.5080 ✓
  cell [10, 19, 4]: fitness=0.7388 coexist 0.80 → 1.00 (n=32) refined_fit=0.5977 ✓
  cell [10, 19, 2]: fitness=0.7378 coexist 1.00 → 0.97 (n=32) refined_fit=0.5063 ✓
  cell [6, 19, 0]: fitness=0.7216 coexist 1.00 → 0.97 (n=32) refined_fit=0.6053 ✓
  cell [3, 19, 3]: fitness=0.7160 coexist 1.00 → 1.00 (n=32) refined_fit=0.6959 ✓
  cell [2, 19, 0]: fitness=0.7099 coexist 1.00 → 0.94 (n=32) refined_fit=0.7476 ✓
  cell [7, 19, 3]: fitness=0.7068 coexist 0.80 → 0.84 (n=32) refined_fit=0.5743 ✓
  cell [8, 19, 0]: fitness=0.7037 coexist 0.80 → 0.84 (n=32) refined_fit=0.5985 ✓
  cell [4, 19, 2]: fitness=0.6803 coexist 0.80 → 0.81 (n=32) refined_fit=0.5213 ✓
  (72 lower-fitness live cell(s) below the top-10 cut were not refined)
Recipe (best refined-robust live cell) written to ../regen-474-recipe.json
Atlas written to ../regen-474-atlas.json

Coverage: 82 / 8000 cells (1.025%)
QD-score (Σ elite fitness): 38.980
Best fitness: 0.7986

Top live cells (by fitness):
  cell [5, 19, 7]: fitness=0.7986 osc=0.271 clus=1.000 carcass=0.359 decomposer_frac=0.60 coexist_frac=0.80 (n=5)
  cell [4, 19, 3]: fitness=0.7673 osc=0.218 clus=1.000 carcass=0.197 decomposer_frac=0.80 coexist_frac=1.00 (n=5)
  cell [10, 19, 4]: fitness=0.7388 osc=0.516 clus=1.000 carcass=0.229 decomposer_frac=0.20 coexist_frac=0.80 (n=5)
  cell [10, 19, 2]: fitness=0.7378 osc=0.537 clus=1.000 carcass=0.118 decomposer_frac=0.00 coexist_frac=1.00 (n=5)
  cell [6, 19, 0]: fitness=0.7216 osc=0.304 clus=1.000 carcass=0.048 decomposer_frac=0.60 coexist_frac=1.00 (n=5)
  cell [3, 19, 3]: fitness=0.7160 osc=0.182 clus=1.000 carcass=0.157 decomposer_frac=0.80 coexist_frac=1.00 (n=5)
  cell [2, 19, 0]: fitness=0.7099 osc=0.108 clus=1.000 carcass=0.049 decomposer_frac=1.00 coexist_frac=1.00 (n=5)
  cell [7, 19, 3]: fitness=0.7068 osc=0.384 clus=0.995 carcass=0.189 decomposer_frac=0.40 coexist_frac=0.80 (n=5)
  cell [8, 19, 0]: fitness=0.7037 osc=0.436 clus=1.000 carcass=0.032 decomposer_frac=0.60 coexist_frac=0.80 (n=5)
  cell [4, 19, 2]: fitness=0.6803 osc=0.226 clus=1.000 carcass=0.122 decomposer_frac=0.80 coexist_frac=0.80 (n=5)

Dead frontier (configs by cliff):
  monoculture            29
  nutrient_lockup        11
  extinction             10
  generalist_dominance   1
  (total dead configs)   51

Predicted bifurcation coordinates (live-cell spread):
  oscillation |lambda|-1  min=-0.2957 mean=+0.0097 max=+0.2600
  branching D             min=+0.0068 mean=+5.0979 max=+29.6614
Bifurcation cross-check disagreements: 316 total (112 validated-regime, 204 weak-observable)
  (a weak-observable disagreement localises to the genesis observable, not F's reading; objective-promotion stays gated on observable-hardening — #358/#359.)
```
