# Issue #462: the held-out check. Do the reduced coordinate sets survive an independent draw?

**Status: measurement on existing data. Changes nothing in the stepper, evaluator
or search, and adds no instrument.** Every cross-validated figure in the #462 line
([π-space check](462-pi-space-explanatory-check.md),
[amplitude read](462-amplitude-read.md), [bloom onset](555-bloom-onset.md)) was
fitted and scored inside the seed-421 LHS sample, and the reduced coordinate sets
were *selected* on that same sample. Those figures are optimistic by an amount
nobody has measured, and #462's last acceptance criterion ("CV explanatory power
against raw 32 on an independent draw") is open for that reason. This note scores
a fresh draw, seed 9421, with the same targets, labels, preprocessing and model
forms, and asks:

1. Fitted on seed 421, how well does each set predict seed 9421 (**transfer**)?
2. How much does each set lose between its within-421 CV figure and its transfer
   figure (**optimism**)? Raw32 has more free parameters and should overfit more.
   The reduced sets were selected on 421 and should carry selection optimism.
   Which effect is larger?
3. Do #555's structural claims hold on 9421?
4. For #462's "What" item 3: search a reduced `decode`, or keep raw32 and shrink
   the noise dims?

Stepper and search at `875cb31`.

## TL;DR

1. **The pipeline reproduces the earlier notes.** On seed 421 the six-dim, swap,
   `H + G + E_r` and raw32 rows match #555 §3 and §5 to the second decimal, and
   the `H` switch table matches #555 §3 exactly (§2).
2. **On the outcome targets the reduced sets survive the held-out draw, and raw32
   does not.** Fitted on 421 and scored on 9421, live fraction reads
   **0.46** (six-dim) / **0.44** (swap) against raw32's **0.28**. Lockup reads
   0.39 / 0.37 / 0.35 and bloom 0.47 / 0.47 / 0.35. Here the reduced sets lose
   nothing between within-421 CV and transfer, while raw32 loses 0.13 on live.
   **Raw32's overfitting is the larger effect on outcome.** On lockup and bloom
   the difference is inside the bootstrap spread.
3. **On onset it is the other way round, and the #555 composites were
   optimistic.** Raw32 transfers without loss (0.59 → 0.59). The swap set drops
   0.57 → **0.51**, `H + G + E_r` drops 0.59 → **0.49** and the six-dim set drops
   0.44 → 0.36. #555's "three numbers match the box on onset" does not hold out:
   the paired gap to raw32 is −0.08 (90 % interval −0.16 to −0.01) for the swap
   set and −0.10 (−0.17 to −0.04) for `H + G + E_r`. Under the logistic link it is
   0.54 / 0.50 against raw32's 0.63. **Selection optimism dominates on onset.**
4. **The #555 mechanism holds on 9421. Only its CV figures shrink.** Onset
   switches in `H`, and the binned table is the same shape: 0.07 below
   `ρ·m²/N₀ = 10`, 0.76 above ~300. The logistic midpoint moves from 142 to
   **184** (bootstrap 90 %: 114–180 on 421, 140–250 on 9421; they overlap). `G`
   predicts tick-1 founder loss at CV `R²` **0.63** again (Spearman +0.90).
   **69 %** of live verdicts never bloomed (67 % on 421). The tick-1 exit again
   sits at the top of `G`.
5. **The held-out effective dimension is about eight raw dims**:
   `light_competition_radius`, `world_extent`, `solar_flux_magnitude`,
   `initial_population_size`, `contact_range_coefficient`, `mean_kappa`,
   `mean_mobility`, `reproduction_energy_threshold`. Outcome needs about six of
   them and onset all eight or a couple more. Every one of them is picked by
   greedy selection on *both* draws, and the dims past them change between draws.
   In raw coordinates, those eight (fixed from the 421 notes before 9421 was read)
   transfer at onset 0.53 / live 0.41 / lockup 0.37 / bloom 0.41, against raw32's
   0.59 / 0.28 / 0.35 / 0.35. Adding `trait_covariance` and `mean_heterotrophy`
   (ten dims) brings onset to 0.57, within the spread of raw32.
6. **Recommendation for item 3: keep the raw coordinates and shrink the ranges
   of the other ~22 dims. Do not build a reduced `decode` over composites.** The
   composites `H` and `G` are the right *explanation* of onset but not better
   *coordinates*: held out, they do worse than the raw dims they are built from,
   and each hides many raw fields the search would still have to set. See §7.

## 1. What ran

| | |
|---|---|
| training draw | `target/permanence-crosscheck.jsonl` (#509), SHA-1 `e92eb3709796251ea6e3c0dd6d506b777b16b731`. The 198 `source = "sample"` configs with ≥ 1 finished seed (`sample:20`, `sample:129` have none); 1538 finished seeds |
| held-out draw | `target/permanence-crosscheck-9421.jsonl`, SHA-1 `da1d44f10b71356c414758bf40b7f6d4800fffef`. 200 configs, `source = "sample@9421"` (the `ConfigSource::Sample(9421)` label, #556), seeds 1000–1007, `T = 2000`, `--eval-timeout-secs 600`. 59 simulation `timeout`, 0 `eval_timeout`. 197 configs with ≥ 1 finished seed (`sample@9421:54`, `:145`, `:161` have none); 1541 finished seeds |
| dropped | seeds with mode `timeout` or `eval_timeout`, per seed, never counted as an outcome (as in all three earlier notes) |
| coordinates | `config_source::sample_draw(421, 32)` and `sample_draw(9421, 32)`, emitted with the `default_ranges()` names and bounds by a one-off `cargo run --release --example`. The 421 vectors are identical to the ones the earlier notes used. Decoded as `min + u·(max − min)`; `initial_population_size` and `initial_cluster_count` rounded as `decode` rounds them, and checked against the row's own `initial_population_size` / `initial_cluster_count` on every config of both files |
| targets | per config, over finished seeds: **onset** fraction (`peak_population > founders`), **live** fraction (mode `none`), **lockup** fraction (`nutrient-lockup`), **bloom** = `log₁₀`(median peak / median founders) |
| sets | **raw32**: the unit vector. **six (#462)**: `log m², log F, r, mean_mobility, mean_kappa, contact_range_coefficient`. **swap (#555)**: `H, G, r, mean_mobility, mean_kappa, contact_range_coefficient`. **`H + G + E_r`**. **π4**: `log₁₀ ρ`, `log₁₀ I`, `log₁₀ Λ`, `κ_C` (`a1.rho`, `a1.invasion_ratio`, `a2.lockup_escape_ratio`, `a1.kappa_c`). `m`, `H`, `G` exactly as #555 §1 defines them, `ρ` and the centroids read off each row |
| within-draw CV | z-scored ridge, 10-fold, best of λ ∈ {0.1, 1, 10, 100}, mean over 5 shuffles (numpy seeds 0–4); `R²` on pooled out-of-fold predictions. Logistic: L2 IRLS on the onset fraction, λ ∈ {0.01, 0.1, 1, 10}, same folds. This is the method of all three earlier notes |
| transfer | λ chosen by within-421 CV; z-scoring and fit on all of seed 421; `R²` on seed 9421 against 9421's own mean. No refitting or reselecting on 9421 |
| spread | the transfer fit is held fixed and the 197 held-out configs are resampled (2000 bootstraps, paired across sets). The table gives 90 % intervals. The single-shuffle sd of within-9421 CV is 0.007–0.026. Differences under ~0.03 are not read |
| tool | throwaway Python/NumPy scripts and the one-off example; neither is committed (the [474](474-atlas-regen-post-fix.md) precedent). The rows above are complete enough to re-run |

The two draws agree on the base rates: onset 0.46 / 0.43, live 0.70 / 0.70,
lockup 0.17 / 0.16, mean bloom 0.24 / 0.24 (421 / 9421), with target sds within
0.02 of each other. The held-out draw is a fair test and not a shifted population.

## 2. Reproducing the seed-421 figures

| set | onset | live | lockup | bloom | earlier note |
|---|---|---|---|---|---|
| raw32 | 0.59 | 0.40 | 0.37 | 0.38 | #555 §5: 0.59 / 0.40 / 0.37 / 0.38 |
| six (#462) | 0.44 | 0.43 | 0.35 | 0.46 | #555 §5: 0.44 / 0.43 / 0.35 / 0.46 |
| swap (#555) | 0.57 | 0.45 | 0.37 | 0.46 | #555 §5: 0.57 / 0.45 / 0.37 / 0.46 |
| `H + G + E_r` | 0.59 | — | — | — | #555 §3: 0.59; logistic 0.67 (here 0.67) |
| π4 | — | 0.05 | 0.05 | — | π-space §2: 0.07 / 0.05 |

The one mismatch is π4 on live: 0.05 here against the π-space note's 0.07. That
note fixed a single fold assignment. Single shuffles here range 0.04–0.07, so its
figure is one draw from this spread, and nothing reads differently. `G`'s tick-1
loss CV (0.63), the `H` bins, and the 67 % never-bloomed share also reproduce
exactly (§5).

## 3. Transfer: fit on 421, score on 9421

Ridge, linear. Each cell is **within-421 CV / transfer to 9421 / within-9421 CV**.
The bracket is the transfer's 90 % bootstrap interval.

| set | dims | onset | live | lockup | bloom |
|---|---|---|---|---|---|
| raw32 | 32 | 0.59 / **0.59** [0.52, 0.65] / 0.55 | 0.40 / **0.28** [0.16, 0.38] / 0.33 | 0.37 / **0.35** [0.22, 0.44] / 0.34 | 0.38 / **0.35** [0.22, 0.44] / 0.32 |
| six (#462) | 6 | 0.44 / **0.36** [0.21, 0.49] / 0.39 | 0.43 / **0.46** [0.37, 0.55] / 0.45 | 0.35 / **0.39** [0.26, 0.48] / 0.37 | 0.46 / **0.47** [0.34, 0.56] / 0.44 |
| swap (#555) | 6 | 0.57 / **0.51** [0.40, 0.61] / 0.51 | 0.45 / **0.44** [0.33, 0.53] / 0.45 | 0.37 / **0.37** [0.25, 0.47] / 0.36 | 0.46 / **0.47** [0.36, 0.54] / 0.44 |
| `H + G + E_r` | 3 | 0.59 / **0.49** [0.38, 0.58] / 0.48 | 0.31 / 0.27 / 0.24 | 0.20 / 0.24 / 0.23 | 0.37 / 0.39 / 0.37 |
| π4 | 4 | 0.02 / 0.06 / 0.09 | 0.05 / 0.03 / 0.02 | 0.05 / 0.01 / 0.02 | 0.05 / 0.02 / 0.00 |

Logistic link on onset (the #555 form), same layout: raw32 0.63 / **0.63** / 0.59;
swap 0.63 / **0.54** / 0.54; `H + G + E_r` 0.67 / **0.50** / 0.51; six-dim
0.47 / 0.43 / 0.44. The link does not change the ranking: raw32 transfers without
loss and the composites do not.

**Paired gap to raw32 on transfer** (set minus raw32, 90 % interval):

| set | onset | live | lockup | bloom |
|---|---|---|---|---|
| six (#462) | −0.23 [−0.34, −0.13] | **+0.19** [+0.12, +0.27] | +0.04 [−0.03, +0.11] | **+0.12** [+0.05, +0.21] |
| swap (#555) | **−0.08** [−0.16, −0.01] | **+0.16** [+0.10, +0.23] | +0.03 [−0.04, +0.09] | **+0.12** [+0.06, +0.19] |
| `H + G + E_r` | **−0.10** [−0.17, −0.04] | −0.01 [−0.10, +0.09] | −0.10 [−0.19, −0.01] | +0.04 [−0.05, +0.14] |
| π4 | −0.54 | −0.24 | −0.34 | −0.33 |

Two readings.

**On outcome, the reduced sets beat raw32 held out.** On live, both six-sets are
ahead of raw32 by 0.16–0.19, and the interval excludes zero. On bloom they are
ahead by 0.12. On lockup they are level. The within-9421 column tells the same
story without any transfer: the six-sets read 0.45 on live against raw32's 0.33.
That column is also a held-out test of the *set choice*, since the sets were
fixed on 421 and only their coefficients are fitted on 9421.

**On onset, raw32 wins held out.** Its 0.59 transfers unchanged. The best reduced
set, the swap set, is 0.08 behind, and the interval just excludes zero. The
three-number `H + G + E_r` is 0.10 behind. The six-dim set, which has no founder
count in it, is 0.23 behind.

Raw32's live transfer (0.28) sits at the low end of its range. Its within-9421 CV
is 0.33, and the reverse transfer (fit on 9421, score on 421) is 0.36. A fair
held-out value for raw32 on live is about 0.30–0.35. The reduced sets are still
ahead by 0.10 or more on that reading.

## 4. Optimism: which effect dominates

Optimism is within-421 CV minus transfer. A positive value means the 421 figure
was too high.

| set | onset | live | lockup | bloom |
|---|---|---|---|---|
| raw32 | −0.01 | **+0.13** | +0.03 | +0.03 |
| six (#462) | +0.08 | −0.03 | −0.03 | −0.01 |
| swap (#555) | +0.06 | +0.02 | −0.00 | −0.01 |
| `H + G + E_r` | **+0.10** | +0.04 | −0.04 | −0.02 |

Set optimism minus raw32 optimism, with the 90 % interval from the paired
bootstrap (within-421 CV held fixed): onset: six +0.09 [−0.01, +0.20], swap
+0.07 [−0.00, +0.15], `H + G + E_r` **+0.11 [+0.05, +0.18]**. Live: six
**−0.16 [−0.23, −0.09]**, swap **−0.11 [−0.18, −0.04]**. Lockup and bloom: −0.03
to −0.06, every interval straddling zero.

**The verdict depends on the target.**

- **Live: raw32's overfitting dominates**, clearly. Raw32 lost 0.13 and the
  reduced sets lost nothing. The six-sets were selected with live as a target,
  yet they show no selection optimism on it.
- **Onset: selection optimism dominates.** Raw32 lost nothing. The composites
  built for onset in #555 (`H`, `G`, and the choice of `E_r` as third number)
  lost 0.06–0.10. `H + G + E_r` is the one case where the interval excludes
  zero. This is the expected cost of constructing features by inspecting the
  target on the same data, and it is the size #555 §6 warned about.
- **Lockup and bloom: neither effect is resolvable.** All sets move by
  ≤ 0.04, inside the spread.

The reduced sets do *not* lose more than raw32 in general. They lose more only
on the target whose features were engineered on 421.

## 5. Do the #555 structural claims hold on 9421?

**The onset switch in `H`.** Binned as in #555 §3 (bin edges at `H` = 1, 1.5, 2,
2.5):

| `ρ·m²/N₀` | 421: n | onset | never | always | 9421: n | onset | never | always |
|---|---|---|---|---|---|---|---|---|
| < 10 | 18 | 0.01 | 0.94 | 0.00 | 15 | 0.07 | 0.87 | 0.00 |
| 10–32 | 30 | 0.14 | 0.70 | 0.03 | 34 | 0.10 | 0.68 | 0.03 |
| 32–100 | 54 | 0.25 | 0.46 | 0.06 | 47 | 0.24 | 0.40 | 0.04 |
| 100–316 | 41 | 0.69 | 0.05 | 0.37 | 46 | 0.60 | 0.11 | 0.22 |
| > 316 | 55 | 0.81 | 0.00 | 0.49 | 55 | 0.76 | 0.04 | 0.38 |

The switch is there, and in the same place: "below 30 almost never, above 300
mostly" holds. The logistic midpoint in `H` alone (the IRLS fit #555 used) is
`ρ·m²/N₀ ≈` **184** on 9421 against 142 on 421, with slope 1.55 against 2.07 per
decade. The bootstrap intervals overlap (114–180 and 140–250). So the switch is
slightly softer and later on 9421, but the midpoint is consistent. The 421 fit of
`H` alone transfers at 0.41 (logistic) and 0.36 (ridge), against 0.47 within 421.
The `H × G` interaction reproduces: at `H ≥ 2`, low-`G` configs bloom on 0.81 of
seeds and high-`G` configs on 0.51 (421: 0.85 / 0.61). At `H < 2` the figures
are 0.34 / 0.05 (421: 0.33 / 0.07).

**`G` as founder grazing.** `G` predicts the fraction of founders lost on tick 1
at CV `R²` **0.63** (421: 0.63), Spearman **+0.90** (421: +0.88), better than
raw32 (0.55). By tercile of `G`, 2 %, 8 % and 42 % of founders are lost on tick 1
(421: 2 / 10 / 39 %). The founder endowment `E₀(1−s)/B_P` again predicts nothing
(Spearman +0.00). Onset and tick-1 loss correlate at −0.57 (421: −0.61). The
tick-1 regime exit is 11 seeds on four configs (`sample@9421:57` 6/8, `:166`
3/8, `:52` and `:160` 1/8 each), at the 87th–99th percentile of `G`. It is still
the tail of founder grazing.

**Live verdicts that never bloomed.** **746 of 1088 (69 %)** on 9421, against
735 of 1092 (67 %) on 421. Non-blooming seeds are 57 % of the finished seeds
(421: 55 %), and 85 % of them end live (421: 86 %). 74 % are thinning remnants
(< 25 % of founders at `T`; 421: 72 %). Live fraction and onset fraction
correlate at −0.47 (421: −0.49). The live target is still mostly a no-bloom
target.

**The bloomed-and-live residue.** #555 §5 named this as the target no reduced set
held. Held out, raw32 transfers at 0.29, the swap set at 0.23 and the six-dim set
at 0.06. The gap is smaller than on 421 (0.26 / 0.18 / 0.09), but it is still
there.

Every structural claim of #555 holds on the fresh draw. What shrank is the
claim that the composites *match the box* on onset. They carry the mechanism,
but not all of the signal.

## 6. The effective dimension, held out

Greedy forward selection over the 32 unit coordinates, run separately on each
draw (ridge CV, as above). The first picks, in order:

| target | seed 421 | seed 9421 |
|---|---|---|
| onset | N₀, r, L, c, F, trait_cov, E_r, κ | L, N₀, r, F, κ, c, mutation_rate, E_r |
| live | r, mob, F, κ, sensing, N₀ | r, κ, c, mob, F, mutation_rate |
| lockup | r, c, mob, mutation_mag, κ, L | r, c, κ, L, mutation_rate, base_metabolic |
| bloom | r, L, F, κ, c, N₀ | r, c, κ, L, reproduction_eff, N₀ |

(`r` light_competition_radius, `L` world_extent, `F` solar_flux_magnitude, `N₀`
initial_population_size, `c` contact_range_coefficient, `κ` mean_kappa, `mob`
mean_mobility, `E_r` reproduction_energy_threshold.)

Within the picks listed, the dims that both draws pick for the same target are
exactly eight: **`r, L, F, N₀, c, κ, mob, E_r`**. Everything outside that set
changes between draws
(`sensing_range_coefficient`, `mutation_magnitude` and `trait_covariance` on 421;
`mutation_rate`, `base_metabolic_rate` and `reproduction_efficiency` on 9421).
That is what noise dims look like at n ≈ 200. The greedy curves flatten
at about six dims for live, lockup and bloom, and at eight to ten for onset.

The same eight dims as a raw set (**raw8**), fixed from the 421 notes (the
π-space six plus #555's `N₀` and `E_r`), and a ten-dim version that adds
`trait_covariance` and `mean_heterotrophy` (the other ingredients of #555's
greedy list and of `G`), transfer as follows:

| set | onset | live | lockup | bloom | onset (logistic) |
|---|---|---|---|---|---|
| raw32 | 0.59 | 0.28 | 0.35 | 0.35 | 0.63 |
| raw10 | 0.57 [Δ −0.02: −0.07, +0.02] | 0.41 [Δ +0.13] | 0.38 | 0.41 | 0.61 |
| raw8 | 0.53 [Δ −0.06: −0.11, −0.00] | 0.41 [Δ +0.13] | 0.37 | 0.41 | 0.56 |
| raw6 (π-space §3) | 0.41 | 0.43 | 0.38 | 0.39 | — |

Δ is the paired transfer gap to raw32. Raw10 is level with raw32 on onset
(both links) and ahead or level on every other target. Its optimism is
+0.03 / +0.02 / −0.02 / +0.01. It also transfers at 0.30 on bloomed-and-live,
level with raw32's 0.29.

**Effective dimension: about eight raw dims hold the signal, held out.** About six
of them suffice for the outcome targets. Onset needs the founder count and the
reproduction gate on top of those, and gains a little from two more. The other
22 raw dims carry nothing that a 200-config draw can detect, and nothing that
transfers from one draw to the other.

## 7. Recommendation for #462 "What" item 3

**Keep raw32 as the `decode` coordinate system and shrink the ranges of the noise
dims. Do not search in a reduced `decode` over composites.**

The held-out evidence gives three reasons.

1. **Composites do not beat their own ingredients held out.** Six numbers
   (swap set) reach onset 0.51. The raw dims that `H` and `G` are built from
   reach 0.53–0.57, and on the outcome targets they are level with the swap set
   (live 0.41–0.43 against 0.44, lockup 0.37–0.38 against 0.37). The
   one place a composite form helps is bloom, where `log m²` beats raw `r, L`
   (0.47 against 0.39–0.41). That is a model-form effect, the power law of the
   amplitude note, and a search does not need it: it moves `r` and `L` and the
   bloom follows. A search over `H` and `G` would also need an inverse map: `H`
   folds together `F`, `B_P`'s maintenance terms, `L`, `r` and `N₀`, and `G`
   folds together `N₀`, the cluster count, the trophic means, `c` and `L`. That
   is re-parameterisation work, and the held-out numbers give no reason to do it.
2. **The noise dims are real noise, and costly.** Raw32 loses 0.13 on live
   between draws, and raw8 or raw10 do not. The 22 dims outside raw10 are not
   chosen consistently by either draw. Carrying them at full width into a
   covariance-adapting search spends evaluations on directions with no
   detectable effect.
3. **Freezing them would overclaim.** "Undetectable at n ≈ 200 with 8 seeds" is
   not "zero". Onset gains 0.02–0.04 from the dims past eight, and the
   bloomed-and-live residue is still only partly held. **Shrinking** the other
   ~22 ranges (towards the baseline, keeping some width) keeps the search able
   to find a weak effect that this data cannot see. Freezing them removes that
   option.

Concretely: keep **`light_competition_radius`, `world_extent`,
`solar_flux_magnitude`, `initial_population_size`, `contact_range_coefficient`,
`mean_kappa`, `mean_mobility`, `reproduction_energy_threshold`** at their full
searched ranges. Keep `trait_covariance` and `mean_heterotrophy` wide as well,
if the onset margin matters. Shrink the rest. The composites `H`, `G` and `m²`
stay what they have been in this line of work: the *explanation* of what those
dims do (the ceiling, the founder headroom, founder grazing), documented in
[viability](../system-design/viability.md) terms, and the natural axes for
reading search results. They are not the search's coordinates.

This closes #462's last acceptance criterion. On an independent draw, a reduced
set of about eight raw dims matches raw32 on onset (within the spread) and beats
it on live (+0.13) and bloom (+0.06), while the 421-selected six-dim composite
sets beat raw32 on live and bloom and fall short on onset.

## 8. Side note: `peak_tick` descriptives (9421 only)

The 9421 rows carry `peak_tick` per seed (#556). This section is description
only. Whether peak timing is an early-stop signal is #554's question.

- Every non-blooming seed has `peak_tick = 0` (the peak is the founder count),
  so the field says something only for the 659 bloomed seeds.
- Over bloomed seeds, `peak_tick / T` has quantiles 0.002 (p10), 0.02 (p25),
  **0.13** (median), 0.34 (p75), 0.75 (p90). 44 % peak by `0.1·T` and 66 % by
  `0.25·T`; 17 % peak after `T/2`.

By eventual mode (bloomed seeds only):

| mode | n | p10 | p25 | median | p75 | p90 | median bloom factor |
|---|---|---|---|---|---|---|---|
| live (`none`) | 342 | 0.002 | 0.003 | 0.033 | 0.13 | 0.52 | 1.25 |
| nutrient-lockup | 239 | 0.10 | 0.17 | **0.30** | 0.49 | 0.76 | 7.8 |
| monoculture | 44 | 0.09 | 0.13 | 0.24 | 0.75 | 0.92 | 36 |
| generalist-dominance | 13 | 0.07 | 0.08 | 0.18 | 0.57 | 0.79 | — |
| extinction | 20 | 0.001 | 0.002 | 0.007 | 0.04 | 0.11 | — |

(Columns are `peak_tick / T` with `T = 2000`. Lockup, monoculture and
generalist-dominance verdicts are all read at `T`. Extinction ends at a median
tick of 758, and its peaks fall at a median 6 % of the time to extinction.)

Live worlds that bloomed mostly bloomed small (median factor 1.25) and peaked
within the first few percent of the run. Lockup and monoculture worlds peaked
later and higher: a median of 0.30 · T for lockup, and 10 % of lockup peaks come
before 0.10 · T. What that separation is worth as a stopping rule is left to
#554.

## 9. Caveats

- **One held-out draw.** Transfer is a single 197-config test. The bootstrap
  covers test-set sampling, not the training draw's luck. The reverse transfer
  (9421 → 421) is an independent reading only for raw32, since the reduced sets
  were selected on 421. It puts raw32's live figure at 0.36, which does not
  change any conclusion. A joint train-and-test bootstrap was tried and set
  aside: resampling the training configs with duplicates penalises the 32-dim
  fit more than the 6-dim fits, so it biases the comparison.
- **raw10's last two dims were chosen by me**, from the 421 notes (#555's greedy
  list and `G`'s ingredients) before 9421's greedy selection was run. They were
  chosen after the composite transfer figures were seen, though. raw8 is the
  clean pre-specified set. The recommendation rests on it and treats raw10 as
  the margin.
- **Linear and logistic forms only**, as in the earlier notes. Interactions are
  still unmeasured (#462 item 2). The `H × G` table in §5 is the only
  interaction read, and it reproduces.
- **Timeouts.** 59 seeds on 9421 and 62 on 421 were dropped per seed. If
  timeouts are not random with respect to outcome (for example, slow tall
  blooms), both draws are biased the same way. Nothing here can check that.
- **`G` still uses centroid traits and assigned cluster roles** (#555 §6). It
  reproduced anyway.
- **"Shrink the ranges" is a direction, not a width.** How far to shrink the 22
  dims, and around which centre, is a search-design choice this data does not
  settle. The baseline values that `decode` inherits are the obvious centre.
