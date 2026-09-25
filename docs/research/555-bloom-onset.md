# Issue #555: what decides whether a bloom starts?

**Status: measurement on existing data. Changes nothing in the stepper, evaluator
or search, and adds no instrument.** [`462-amplitude-read.md`](462-amplitude-read.md)
found that 52 % of the seed-421 LHS sample never blooms: the median peak
population equals the founder count, so the bloom factor is exactly 1.000. It
also found that the raw 32-dim box predicts *whether* a config blooms much
better than the amplitude pair does (CV `R²` 0.44 against 0.29). The amplitude
law `log₁₀(peak/founders) = 0.40·log₁₀ m² + 0.29·log₁₀ F − 0.64` says how big a
bloom gets, but little about whether one starts. This note asks three
questions, all answerable without new rollouts:

1. What predicts bloom onset? Which raw dims carry raw32's advantage, and is
   there a compact physical form for it, as `m²` turned out to be for amplitude?
2. How do the worlds that never bloom end? How much of that does the tick-1
   regime exit (#517, `sample:12`) account for?
3. Does adding the onset coordinate(s) to the six-dim reduced set close the gap
   to raw32 on live and lockup?

The data is `target/permanence-crosscheck.jsonl` (#509: 282 configs × 8 seeds
at `T = 2000`, SHA-1 `e92eb3709796251ea6e3c0dd6d506b777b16b731`), with the
stepper and search at `c7c2e7b`. **Every figure is within-sample on the seed-421
draw.** The held-out check needs the seed-9421 draw that #553 (merged as #556)
makes addressable (`sample@9421:i`); see §6.

## TL;DR

1. **The method matches the #462 notes.** On their label (a config blooms iff
   its median bloom factor is above 1), `log m²` / `+ log F` / raw32 read CV
   `R²` **0.24 / 0.29 / 0.43**, against the note's 0.24 / 0.29 / 0.44. The fits
   below use a finer label: the fraction of a config's finished seeds that
   bloom (`peak_population > founders`). On that label the same three sets read
   0.30 / 0.36 / **0.59**.
2. **Onset is decided by founder crowding, not by bloom headroom in `m²`
   alone.** The strongest single raw dim is `initial_population_size`
   (Spearman **−0.43**): more founders make a bloom *less* likely. The compact
   form is one number, the **founder headroom**
   `H = log₁₀(ρ·m²/N₀) = log₁₀(F·m²/(B_P·N₀))`. It is the light ceiling's tile
   count times the producer invasion ratio, per founder. Alone it reads **0.47**,
   more than `log m² + log F` (0.36), and the free fit over its four parts
   (0.47) does no better. Onset is a switch in `H`. Configs with
   `ρ·m²/N₀ < 30` almost never bloom (onset fraction 0.01–0.14), those above
   ~300 mostly do (0.81), and the logistic midpoint sits at `ρ·m²/N₀ ≈ 140`.
3. **The second onset coordinate is founder grazing, and it is also what the
   tick-1 exit is.** The **founder grazing cover** `G` is the founders'
   combined feeding-reach area as a fraction of the world,
   `Σ π(h_i·c)²/L²` at the cluster centroids. It predicts the fraction of
   founders lost on tick 1 at CV `R²` **0.63**, better than raw32's 0.59
   (Spearman +0.88). The founder endowment predicts nothing
   (`E₀(1−s)/B_P`, Spearman −0.08). **Three numbers match the box:** `H + G`
   reads 0.56, and `H + G + reproduction_energy_threshold` reads **0.59**
   against raw32's **0.59** (per-seed AUC **0.886** against 0.877). A logistic
   link fits the switch better, and there the three numbers beat the box:
   **0.67** against 0.63.
4. **Non-blooming worlds mostly do not die. They thin to a live remnant.** Of
   853 non-blooming finished seeds, **86 %** are verdict `none` (live) at
   `T = 2000`. 72 % end with under a quarter of their founders (median three
   agents), and only 3 seeds (0.4 %) hold ≥ 75 % of the cohort. Extinction is
   12 % (median tick 438). Energy death, lockup and generalist dominance
   together are 1.6 %. **The tick-1 regime exit is 10 seeds (1.2 %)** on three
   configs (`sample:12` 7/8, `sample:171` 2/8, `sample:58` 1/8), all at the
   95th–99th percentile of `G`. A consequence for #462: **two-thirds of all
   "live" verdicts in the sample (735 / 1092) are worlds that never grew past
   their founders.**
5. **Q3: the onset coordinates add a little on outcome, and a swapped six-set
   covers every target.** The six-dim set plus `H, G` reads live **0.45** /
   lockup **0.37**, against the six-dim set's 0.43 / 0.35 and raw32's
   0.40 / 0.37. That gain of +0.02 is inside the shuffle spread. The useful
   result is a swap: **`{H, G, r, mean_mobility, mean_kappa,
   contact_range_coefficient}`**, which replaces `log m², log F` by `H, G`, stays
   at six numbers and reads **onset 0.57 / live 0.45 / lockup 0.37 / bloom
   0.46**, against raw32's 0.59 / 0.40 / 0.37 / 0.38. That is the first
   reduced set in the #462 line to match or beat the box on all four targets
   at once.

## 1. What ran

| | |
|---|---|
| outcomes | `target/permanence-crosscheck.jsonl` (#509), SHA-1 `e92eb37…`; the **198 LHS `sample` configs** with ≥ 1 finished seed (`sample:20` and `sample:129` have none). Seeds with mode `timeout` (62 in the sample) or `eval_timeout` (none present) are dropped per seed and never counted as an outcome. This leaves 1538 finished seeds |
| coordinates | the seed-421 unit vectors from `explorers_search::config_source::sampled_units(32)` (equivalently `sample_draw(421, 32)`), emitted by a one-off example that printed `default_ranges()` names and `sampled_units(names.len())` as JSON. Decoded dims are `min + u·(max − min)` over `search::default_ranges()`, which is `decode`'s linear map; `initial_population_size` and `initial_cluster_count` are rounded as `decode` rounds them |
| onset label | **per seed:** blooms iff `peak_population > founders` (peak is taken over post-step counts from tick 1, starting at `founders`, so `peak ≥ founders` always; the minimum ratio is exactly 1.000). **Per config:** the fraction of finished seeds that bloom (**onset fraction**, mean 0.46, sd 0.41; 33 % of configs are 0/n, 23 % n/n). **The fits use the onset fraction.** The #462 note's binary label (median bloom factor > 1) is reported for the reproduction (§2), and per-seed AUC is reported beside the fraction |
| outcomes (Q3) | per-config seed fractions **live** (mode `none`) and **lockup** (`nutrient-lockup`), and `log₁₀` bloom factor (median peak / median founders), as in the #462 notes |
| derived | `m = ⌊√2·L/r⌋ + 1`; `ρ = F/B_P` and `B_P` read off the row (`a1.rho`, `a1.b_p`: the producer centroid's per-tick maintenance, which the sweep computes); **founder headroom** `H = log₁₀(ρ·m²/N₀)`; **founder grazing cover** `G = log₁₀(10⁻³ + Σ_i π(h_i·c)²/L²)`, where the sum runs over founders at their cluster centroid's heterotrophy `h_i` (`producer_centroid` / `consumer_centroid` on the row), `c` is `contact_range_coefficient`, and founder `i`'s cluster is `i mod n`, even clusters producers, as `World::new` assigns them (`n = 1`: every founder at the mean centroid). With `body_reach_coefficient = 0` on the search baseline, feeding reach is `h·c` (`phase::consumption_reach`) |
| models | z-scored ridge, 10-fold CV, best of λ ∈ {0.1, 1, 10, 100} applied identically to every set; the mean over 5 CV shuffles (numpy seeds 0–4). This is the method of both #462 notes. `R²` is computed on the pooled out-of-fold predictions. The single-shuffle sd is 0.003–0.013, so differences below ~0.03 are not real. Per-seed AUC uses the same ridge fitted on seed rows with config-grouped folds. The logistic column is an L2 logistic regression on the onset fraction (IRLS, λ ∈ {0.01, 0.1, 1, 10}), same folds |
| tool | a throwaway Python/NumPy script and the one-off `cargo run --example`; neither is committed (the [474](474-atlas-regen-post-fix.md) precedent). The rows above are complete enough to re-run |

## 2. Reproducing the #462 onset figures

| feature set | median-label (the #462 note) | onset fraction (this note) | per-seed AUC |
|---|---|---|---|
| `log m²` | 0.24 (note: 0.24) | 0.30 | — |
| `log m²` + `log F` | 0.29 (note: 0.29) | 0.36 | 0.806 |
| raw32 | **0.43** (note: 0.44) | **0.59** | 0.877 |

The method matches to within the shuffle spread. The onset fraction carries
more signal than the median label, which rounds each of the 87 mixed configs to
0 or 1. The
gap to raw32 is the same shape on both labels: the amplitude pair holds about
60 % of what the box knows about onset.

## 3. What predicts onset

**Which raw dims.** Univariate Spearman against the onset fraction:
`initial_population_size` **−0.43**, `light_competition_radius` −0.39,
`world_extent` +0.34, `solar_flux_magnitude` +0.25, `contact_range_coefficient`
−0.25, `trait_covariance` −0.21. Greedy forward selection over the unit
coordinates picks, in order, `initial_population_size` (0.21),
`light_competition_radius` (0.34), `world_extent` (0.45),
`contact_range_coefficient` (0.50), `solar_flux_magnitude` (0.55),
`trait_covariance` (0.57), `reproduction_energy_threshold` (0.58) and
`mean_kappa` (0.60). Eight raw dims reach the box. The first five are the
parts of two physical quantities.

**The founder headroom.** The first three greedy dims are the ceiling's `m²`
(`L`, `r`) and the founder count, with opposite signs. The #462 amplitude pair
left out the founder count on purpose, since dividing it out was what made the
bloom *factor* coordinate-free. For onset it is the other half of the ratio: a
world has bloomed once its population exceeds `N₀`, so what matters is how far
the founder cohort sits below what the light can pay for. Written in committed
quantities, that is the light ceiling's tile count `m²` ([viability,
*sustained-count ceiling*](../system-design/viability.md)) times the producer
invasion ratio `ρ = F/B_P` (how many producer maintenances one tile's full flux
pays), per founder:

```
H  =  log₁₀ ( ρ · m² / N₀ )  =  log₁₀ ( F · m² / (B_P · N₀) )
```

| set | onset fraction | median-label | AUC | logistic |
|---|---|---|---|---|
| `log m²` + `log F` | 0.36 | 0.29 | 0.806 | 0.40 |
| `log m²` + `log N₀` + `log F` | 0.46 | — | — | — |
| `log(m²/N₀)` | 0.41 | — | — | — |
| **`H`** (one number) | **0.47** | 0.38 | 0.847 | 0.54 |
| `log m², log N₀, log F, log B_P` free | 0.47 | — | — | — |
| `H + G` | 0.56 | 0.44 | 0.869 | 0.62 |
| **`H + G + E_r`** | **0.59** | **0.46** | **0.886** | **0.67** |
| six-dim set (#462) | 0.44 | 0.35 | 0.822 | 0.47 |
| raw32 | 0.59 | 0.43 | 0.877 | 0.63 |

`E_r` is `reproduction_energy_threshold`. Fitting the four parts of `H` freely
gains nothing (0.47 either way), so the ratio form costs nothing in CV. The
free OLS exponents are not equal (`m²` +0.32, `N₀` −0.56, `F` +0.26, `B_P`
−0.23 per decade), so `H` is the right shape but not a derived exponent.
Crowding the founders counts for more than widening the ceiling.

**The switch.** Binned by `H`:

| `ρ·m²/N₀` | n | onset fraction | configs never blooming | configs always blooming | live | lockup |
|---|---|---|---|---|---|---|
| < 10 | 18 | 0.01 | 0.94 | 0.00 | 0.85 | 0.00 |
| 10–30 | 30 | 0.14 | 0.70 | 0.03 | 0.92 | 0.01 |
| 30–100 | 54 | 0.25 | 0.46 | 0.06 | 0.86 | 0.03 |
| 100–300 | 41 | 0.69 | 0.05 | 0.37 | 0.68 | 0.25 |
| > 300 | 55 | 0.81 | 0.00 | 0.49 | 0.40 | 0.39 |

The step sits between 100 and 300. The logistic fit in `H` alone puts the
midpoint at `H = 2.15`, i.e. `ρ·m²/N₀ ≈ 140`, with slope 2.1 per decade. That
is well above the naive break-even. A founder's light disc covers about `2π`
ceiling tiles (tile side `< r/√2`), so the founders just pay their maintenance
at `ρ·m²/N₀ ≈ 2π ≈ 6`. **A bloom needs about twenty times more light per
founder than survival does.** This is consistent with reproduction needing a
surplus: the `(1 − κ)` share of mobilised surplus has to fill the reproductive
allocation to `E_r` before any birth. But this sweep cannot separate that from
the grazing below, so the factor is measured, not derived.

**The founder grazing cover.** The next greedy dim after the headroom is
`contact_range_coefficient`, with a negative sign. Feeding reach is `h·c`, a
hard contact predicate ([world rules, *Feeding
reach*](../system-design/world-rules.md)), and a drained body dies when its
structure falls below `fragility × peak_structure`. So the founders' combined
feeding-reach area, as a fraction of the world, measures how much of the
cohort sits inside some heterotroph's reach from tick 1. It is `G` above:
median 0.05, 90th percentile 0.53, maximum 5.3 (on `sample:12`). Three checks
support reading `G` as founder grazing:

- `G` predicts the **fraction of founders lost on tick 1** at CV `R²` **0.63**,
  Spearman **+0.88**, better than raw32 (0.59). Terciles of `G` lose 2 %, 10 %
  and 39 % of founders on tick 1.
- The founder endowment in ticks of producer maintenance, `E₀(1−s)/B_P`,
  predicts no tick-1 loss (Spearman −0.08). Its minimum over the sample is 2.3
  ticks, so at the centroid no founder's maintenance can starve it on its
  first tick. The loss
  has to come from the drain pass.
- Across all configs, onset fraction and tick-1 loss correlate at Spearman
  −0.61. Within the 87 mixed configs, the seeds that lose more founders on tick
  1 are the ones that fail to bloom (within-config AUC 0.64). The seed-level
  realisation of the same mechanism shows through.

`H` and `G` interact the way the mechanism says they should. At high headroom
(`H ≥ 2`), low-`G` configs bloom on 0.85 of seeds and high-`G` configs on 0.61.
At low headroom, 0.33 and 0.07.

**First reproduction.** `reproduction_energy_threshold` is the third
coordinate (+0.03). It is the flat energy gate a founder's reproductive
allocation has to clear. A first-reproduction time built from it,
`t_r = E_r / ((1−κ)·(F − B_P))`, reads no better than `E_r` raw (0.58 against
0.59 with `H + G`; 0.04 alone). `F − B_P` is already in `H`, and the
centroid's `κ` is a poor proxy for the founders' own: founder `κ` is drawn with
sd `trait_covariance` (the sixth greedy dim) and clamped to `[0, 1]`. The endowment, `E₀(1−s)/B_P`, adds nothing to `H` (0.46).

## 4. How non-blooming worlds end

All 853 finished seeds that never exceed their founder count (55 % of 1538),
by ending:

| ending | seeds | share |
|---|---|---|
| live at `T`, remnant (< 25 % of founders alive) | 616 | 0.722 |
| live at `T`, partial (25–75 %) | 116 | 0.136 |
| live at `T`, founder cohort held (≥ 75 %) | 3 | 0.004 |
| extinction after tick 1 | 94 | 0.110 |
| **extinction on tick 1 (the regime exit)** | **10** | **0.012** |
| energy death | 8 | 0.009 |
| nutrient lockup | 5 | 0.006 |
| generalist dominance | 1 | 0.001 |

**The typical non-blooming world is a thinning remnant, not a death and not a
stable cohort.** The founder cohort dies back. Median tick-1 loss is 17 % for
non-blooming seeds and 0 % for blooming ones, and 17 % of non-blooming seeds
lose half their founders on tick 1 against 3 % of blooming ones. The world then
settles to a handful of agents that pay their way: a median of three agents at
`T`, and 44 % of remnant worlds still hold a consumer (heterotrophy >
autotrophy). This is the sparse settled
community the [energy bound](../system-design/viability.md) sees on the income
cap: a producer with no competitor within `r` takes the full flux. The sweep
records no births, so whether a remnant world ever reproduced below `N₀` is not
readable from this file.

**Extinction** is 12 %. It comes late when it comes (median tick 438,
interquartile 183–934); only 14 of the 94 post-tick-1 extinctions happen
within 50 ticks.

**The tick-1 regime exit accounts for 1.2 %.** It is 10 seeds on three configs:
`sample:12` (7 of 8; the eighth keeps one founder past tick 1, which dies at
tick 573), `sample:171` (2 of 8) and `sample:58` (1 of 8). All three sit at the
95th–99th percentile of `G`, with `contact_range_coefficient` ≥ 3.97 on a range
of 0.5–5 and substantial founder heterotrophy. Their endowments hold 12–234
ticks of producer maintenance. The [viability](../system-design/viability.md)
note already attributes the exit to "the peak-relative death threshold,
endowment and embodiment per-body floor". This sweep narrows that: **in the
sample, the tick-1 exit is the tail of founder grazing, not of endowment.** It
is the extreme end of the same `G` gradient that thins the cohort everywhere
else. `sample:12` also has low headroom (`ρ·m²/N₀ = 9`), so it would be unlikely
to bloom even without the grazing.

**For #462's targets:** 735 of the 1092 live verdicts in the sample (67 %) are
seeds that never bloomed. A config's live fraction is therefore largely the
no-bloom fraction (live and onset fraction correlate at −0.49). This is why
`log m²` "predicts live" with a negative sign: it is an onset coordinate first.

## 5. Q3: the reduced set with the onset coordinates

Same population, same method. Columns are per-config targets.

| coordinate set | dims | onset | live | lockup | bloom |
|---|---|---|---|---|---|
| six-dim (#462): `log m², log F, r, mob, κ, c` | 6 | 0.44 | 0.43 | 0.35 | 0.46 |
| six + `H` | 7 | 0.54 | 0.46 | 0.36 | 0.47 |
| six + `H, G` | 8 | 0.56 | 0.45 | 0.37 | 0.47 |
| six + `H, G, E_r` | 9 | 0.59 | 0.45 | 0.37 | 0.47 |
| **`H, G, r, mob, κ, c`** | **6** | **0.57** | **0.45** | **0.37** | **0.46** |
| `H, G, E_r, r, mob, κ, c` | 7 | 0.60 | 0.45 | 0.36 | 0.46 |
| raw32 | 32 | 0.59 | 0.40 | 0.37 | 0.38 |
| raw32 + `H, G` | 34 | 0.60 | 0.42 | 0.36 | 0.41 |

(`r` is `light_competition_radius`, `mob` `mean_mobility`, `κ` `mean_kappa`,
`c` `contact_range_coefficient`.) The six-dim row reproduces the amplitude
note's 0.42 / 0.34 / 0.45 to within the shuffle spread.

Three readings.

**Adding the onset coordinates barely moves outcome.** Live gains +0.02 and
lockup +0.02, both inside the spread. The six-dim set was already level with
raw32 on lockup and ahead on live, so there was no outcome gap to close. The
gap the issue named was the onset column, where the six-dim set reads 0.44
against 0.59, and `H, G` close it.

**`H, G` can replace `log m², log F` without growing the set.** `H` carries
`m²` and `F` (with `N₀` and `B_P`), and swapping in `G` keeps the count at six.
That set matches or beats raw32 on all four targets (0.57 / 0.45 / 0.37 / 0.46
against 0.59 / 0.40 / 0.37 / 0.38). `E_r` makes it seven and buys onset only.

**What the box still knows.** On a stricter target, *bloomed and live*
(the live verdicts that did bloom), raw32 reads 0.26 against the six-dim set's
0.09 and six + `H, G`'s 0.18. Once the 735 no-bloom live verdicts are removed,
what is left of "live" is not yet held by any reduced set. That is the natural
next target for #462, and it is a small one: 357 seeds.

## 6. What this does not settle

- **No independent draw.** Every figure is within the seed-421 sample. `H` and
  `G` were found by inspecting this same data (greedy selection, then
  composites built for the dims it picked), and the six-dim set they join was
  selected on it too. Every row of §3 and §5 is optimistic by an unmeasured
  amount. The held-out check is to score the seed-9421 draw (`sample@9421:i`,
  #553 / #556) at `T = 2000` and refit these exact sets there, without
  reselecting. That is a sweep, and it was deliberately not run here.
- **`H` and `G` are composites.** Six numbers in the swapped set hide about
  twelve raw fields (`H`: `F`, `B`, the photo/hetero/mobility maintenance
  terms, the exponent, `L`, `r`, `N₀`; `G`: `N₀`, `n`, the trophic means,
  `c`, `L`). For a lower-dimensional `decode` the question is whether the
  search can move along `H` and `G` directly. That is a re-parameterisation
  choice, not something this note measures.
- **The threshold factor is not derived.** The onset midpoint at
  `ρ·m²/N₀ ≈ 140` sits about 20× above founder break-even. The note offers a
  reason (reproduction needs surplus, and grazing removes founders), but no
  derivation. A closed form would need the time to first reproduction under
  light sharing, which the reduction does not carry.
- **`G` uses centroid traits.** It ignores `trait_covariance` noise (which gives
  producer founders positive heterotrophy after clamping) and the realised
  founder positions. The per-seed tick-1 loss beats it within configs (AUC
  0.64), which is where seed placement enters.
- **Births are not recorded.** "Never bloomed" means the census never exceeded
  `N₀`, not "never reproduced". A remnant world may have replaced its dead below
  the founder count. The row shape carries `peak_tick` since #556 but no birth
  count.
- **Linear models, again, for the Q3 comparison.** The logistic link reads the
  onset switch better (0.67 against 0.59 for the three-number set; raw32 0.63),
  but the Q3 table stays linear so it is comparable with the #462 notes.
