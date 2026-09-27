# Issue #561: is there a search-box narrowing worth making the default?

**Status: measurement on existing data. Changes nothing in the stepper, evaluator
or search, and adds no instrument.** #559's narrowed box (the ten-dim core at full
width, the other 22 dims cut to bands of 0.25 of their span around the
known-viable baseline) illuminated less than the full box on both search seeds,
and recorded about 1.7× as many nutrient-lockup dead configs
([genesis-search.md](../system-design/genesis-search.md), *The search box*). The
design doc read the lockup excess as a **centre effect**: the band centres sit in
a lockup-prone part of the box. This note asks:

1. Which of the 22 narrowed dims drive the behaviour descriptors, and in particular
   the clustering axis's intermediate bins?
2. Is the lockup excess a centre effect? Would re-centring the bands fix it?
3. Is there a narrowing worth the four-run test for promotion to the default?

Stepper and search at `1a36235`.

## TL;DR

1. **No narrowed dim is a strong behaviour driver.** The behaviour signal is in
   the core. Clustering follows `mean_mobility` (Spearman −0.38) and `world_extent`
   (+0.23), and carcass follows `contact_range_coefficient` (−0.37) and
   `initial_population_size` (+0.25). The largest narrowed-dim correlations are
   0.17–0.22 against a permutation threshold of 0.14 (§2). There is no small set
   of dims that could be kept wide to win the coverage back.
2. **It is not a centre effect.** On the 395 LHS configs, lockup under
   baseline-centred bands and under midpoint-centred bands differs by −0.02
   (90 % interval −0.13 to +0.07). A linear model, which can only see centres,
   puts the two boxes 0.01 apart. The standouts in the per-dim band rates do not
   replicate across the two draws (§3).
3. **It is a width effect, spread thinly.** Narrowing the 22 dims around *any*
   centre raises predicted lockup by about 0.05 and lowers the live fraction by
   about 0.06–0.08. The configs nearest the band centres have the most lockup,
   whichever centre is used. No single dim carries more than about 0.02 of it
   (§3, §4).
4. **The penalty starts well before 0.25.** At a band width of 0.75 predicted
   lockup is already +0.03 and live −0.03, rising to +0.04 and −0.06 at width
   0.5 (§4).
5. **Recommendation: no narrowing is worth the four-run test.** Keep the full box
   as the default. Keep `narrowed_ranges()` as the opt-in it already is, and
   correct the design doc's explanation of its cost (§5).

## 1. Data and method

| | |
|---|---|
| atlases | full box: `atlas.json` (seed 42, SHA-1 `3428e4e8…`), `target/atlas-fullbox-s43.json` (`7113da06…`). Narrowed: `target/atlas-559.json` (seed 42, `0f8bf36f…`), `target/atlas-559-s43.json` (seed 43, `57b07c6e…`). The last one did not exist when #561 was filed. It now does, so its cells are read from the atlas, not the checkpoint |
| LHS draws | `target/permanence-crosscheck.jsonl` (seed 421, `e92eb370…`) and `target/permanence-crosscheck-9421.jsonl` (`da1d44f1…`): the same files and the same drop rule as [462-held-out-check.md](462-held-out-check.md). 395 configs with ≥ 1 finished seed, 3079 finished seeds. Unit vectors from `config_source::sample_draw(421 \| 9421, 32)`. The decode was checked against each row's `initial_population_size` and `initial_cluster_count` |
| coordinates | every cell and config is expressed in **full-box** unit coordinates: narrowed-atlas cells are decoded over their recorded `search_box`, then re-normalised over `default_ranges()`. A narrowed dim's band is then an interval of width 0.25 in these coordinates |
| targets | per config, over finished seeds: the **lockup** fraction (`nutrient-lockup`) and the **live** fraction (mode `none`) |
| §2 | Spearman correlation of each unit coordinate with each descriptor, over the 188 full-box elites. The threshold is the 95th percentile of \|ρ\| under 2000 permutations |
| §3–4 models | seed-weighted ridge on the 32 coordinates, with (**additive-quadratic**) and without (**linear**) a `(u − 0.5)²` term per dim. λ by 10-fold CV (5 shuffles). Box averages are taken over 30k–200k uniform draws with the core at full width. Intervals come from 300 bootstraps of the configs, refitting each time |
| tool | throwaway Python/NumPy scripts and a one-off example, not committed (the [474](474-atlas-regen-post-fix.md) precedent) |

The additive-quadratic lockup model's CV R² is 0.49 within the pooled draws and
0.33 on 421 → 9421 transfer. The live model scores 0.42 and 0.32. The models are
fit to rank boxes, not to predict single configs.

## 2. Behaviour drivers (Q1)

The atlas cell counts show the clustering loss:

| atlas | clustering bin 0 | intermediate (1–18) | bin 19 |
|---|---|---|---|
| full, 42 | 38 | 8 | 49 |
| full, 43 | 32 | 10 | 51 |
| narrowed, 42 | 35 | 0 | 43 |
| narrowed, 43 | 29 | 3 | 52 |

The narrowed box loses most of the intermediate clustering cells. On seed 42 it
also loses some of the top bin.

The full-box elites do not sit near the narrowed box. **None of the 188 is inside
it.** Each lies outside the band on 12–22 of the 22 narrowed dims (median 16).
Uniform sampling would give 16.5. On the narrowed dims the elites are spread
nearly as widely as uniform: the per-dim sd in their own box's units is 0.25,
against 0.29 for uniform. The emitter starts at the box centre, which accounts
for the mild centre bias.

Correlations with |ρ| above the permutation threshold of 0.14:

| descriptor | core dims | narrowed dims |
|---|---|---|
| clustering | `mean_mobility` −0.38, `world_extent` +0.23 | `mean_photosynthetic_absorption` +0.20, `offspring_structure_fraction` +0.20, `mean_asexual_propensity` −0.17, `movement_cost_coefficient` −0.17 |
| carcass | `contact_range_coefficient` −0.37, `initial_population_size` +0.25, `solar_flux_magnitude` +0.16, `trait_covariance` −0.15 | `specification_nutrient_coefficient` +0.22, `base_nutrient_ratio` +0.17, `mean_photosynthetic_absorption` +0.17, `heterotrophy_maintenance_cost` −0.16, `offspring_structure_fraction` +0.15, `initial_energy_per_agent` +0.15 |
| oscillation | none | none |

This makes 66 narrowed-dim tests, with about 3 false positives expected at this
threshold. About 10 pass, all of them weak. The candidates #561 named do not
stand out: `mutation_rate` (0.00 against clustering),
`mutation_magnitude` (+0.02), `reproductive_compatibility_distance` (+0.12). The
nutrient pair are the only named candidates to show up, on carcass.

The 18 intermediate-clustering elites are too few to support a per-dim read. On
their median position, `trophic_distance_decay` (0.66) and `base_nutrient_ratio`
(0.75) sit well above their bands (0.06–0.31), and none of the 18 is in either
band. But every band at the edge of a range sees few elites, because of the
emitter's centre bias.

## 3. Centre or width? (Q2)

**Per dim, band against band.** The seed-weighted lockup rate of the LHS configs
inside each baseline-centred band, compared with a midpoint band of the same
width, is flat for most dims. The standouts in the pooled table are
`reserve_mobilisation_rate` (top quarter 0.20 against 0.10),
`reproduction_efficiency` (+0.08) and `growth_retention_multiplier` (−0.07).
None of them replicates across draws. `reserve_mobilisation_rate`'s top-quintile
lockup is 0.25 on 421 but 0.16 on 9421. `reproduction_efficiency`'s quintile
profile has no consistent shape on either draw.

**Multivariate, bootstrapped.** Model-predicted rates are averaged over each box:

| comparison | Δ lockup [90 %] | Δ live [90 %] |
|---|---|---|
| baseline-centred narrowed − full | +0.05 [−0.02, +0.13] | −0.08 [−0.16, +0.02] |
| midpoint-centred narrowed − full | +0.08 [+0.00, +0.15] | −0.07 [−0.17, +0.02] |
| baseline − midpoint | −0.02 [−0.13, +0.07] | 0.00 [−0.12, +0.12] |

The linear model can only move the mean, so it can only see a centre effect. It
puts full, baseline-narrowed and midpoint-narrowed at 0.175, 0.185 and 0.176
lockup. Re-centring on the full-box elites' medians gives the same predicted
lockup as the midpoint (0.285 against 0.285 in the point fit), which is expected:
those medians sit near 0.5.

**Model-free.** Split the 395 configs by rms distance to the band centres over
the 22 narrowed dims. The nearest quartile has the most lockup:

| centre | lockup by distance quartile (near → far) | Spearman (distance, lockup) [90 %] |
|---|---|---|
| baseline | 0.199, 0.161, 0.136, 0.157 | −0.08 [−0.16, +0.01] |
| midpoint | 0.245, 0.091, 0.166, 0.148 | −0.06 [−0.15, +0.02] |

The data agrees across three readings: moving the bands does not help, and
pulling in toward a centre, any centre, costs a little. Each piece is marginal on
its own, but they point the same way, and they match the search: lockup up on
both seeds, and whole clustering bins gone.

## 4. How much of the width can go? (Q3)

Bootstrapped additive-quadratic ridge, baseline centres, core at full width:

| band width | Δ lockup vs full [90 %] | Δ live vs full [90 %] |
|---|---|---|
| 0.75 | +0.028 [−0.005, +0.057] | −0.030 [−0.070, +0.013] |
| 0.50 | +0.044 [−0.005, +0.101] | −0.064 [−0.127, +0.012] |
| 0.25 | +0.047 [−0.026, +0.137] | −0.061 [−0.151, +0.049] |

The cost starts with the outer quarter of the range. No width buys concentration
for free. The width effect is diffuse: with the other dims at full width,
narrowing one dim to a midpoint band costs at most 0.02 lockup
(`initial_energy_per_agent` 0.022, `initial_cluster_count` 0.017,
`growth_retention_multiplier` 0.017), and the 22 together cost 0.075. There is
no short list of "keep these wide" dims that would buy back most of the loss.

## 5. Recommendation

No narrowing passes the bar that would justify the four-run promotion test
(coverage and QD-score within seed spread of the full box, on two seeds):

- a **re-centred** box does not address the cause, since the cause is not the
  centre;
- a **wider** band pays the same kind of cost, only less of it, and saves less;
- a **selective** narrowing that keeps a few more dims wide has no dims to pick,
  because the effect is spread across all 22.

Keep the full box as the default and `narrowed_ranges()` as an opt-in. Correct
the design doc: the narrowed box costs coverage and adds lockup because it cuts
the tails of 22 weakly-acting dims, not because its centres are badly placed.

The narrowed box's one gain was the higher best elite on seed 43. That is the
concentration a small box buys, and it could still suit a *refinement* stage
around a known elite, where coverage is not the goal. That is a separate
question and is not tested here.

## 6. Caveats

- **Elites are not a random sample.** §2 reads correlations off QD elites, which
  are selected for fitness and cell novelty. The §3–4 reading uses the unbiased
  LHS draws, but those carry no behaviour descriptors. So the lockup conclusion
  is on firm ground, and the behaviour-driver conclusion is a screen.
- **Additive models.** Interactions are unmeasured, as in the #462 notes. A
  narrowing that works only through an interaction would not show up here. The
  model-free distance split is the check that does not assume additivity, and
  it agrees.
- **Uniform-box averages are not search counts.** The search's 1.7× lockup ratio
  counts dead configs among an emitter's proposals, which are neither uniform
  nor independent. The models give about 1.25× for a uniform draw. They agree
  on the direction, not the size.
- **Two search seeds per box, one batch/generation setting.** This note does not
  add runs. It explains the four that exist.
