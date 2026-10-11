# Issue #807: the size-scaled base rate on the mode 1 mesocosm

**Status: reading (2026-10-11).** The size-scaled base rate (world rules, flow 8, *The base rate scales with structure*) is implemented, off by default, and `reference_mode --size-scaled-base-rate` turns it on. This note is the short reading the issue asks for: the committed mesocosm run paired with the rule on, seeds 1–5, against the same runs with it off. It re-reads the producers' persistence under the new form, as flow 8 requires. It is not a sweep, carries no verdict on parent #764, and changes no parameter default.

## What it finds

- **The producer control persists in every seed**, as it does with the rule off. Its producers are more numerous and smaller, and they live longer. At the end of the control arm, 336–473 producers stand, against 113–149 with the rule off. The established producers' pooled median structure falls from 21.6 to 11.9. Their median established lifespan rises from 207–238 ticks to 299–370.
- **The decomposer guild does not hold in any seed**, as with the rule off. Its fewest members in the second half are 0 in every arm. Its share of the carcass drain is 0–0.074 in the controls, against 0–0.031 off. Decomposers by role persist in every arm, 18–45 in the controls at the end, against 12–18 off. The mixotrophs (autotrophy at least 0.1) still drain 99 % of carcass structure.
- **The founders still die out, a little later.** At tick 20, 10–20 of 81 founders are alive, against 1–2 off. None is alive at tick 50 or 100 in either rule. The median founder dies at tick 15–16, against 13 off, and the last at tick 30–38, against 21–24. Their mean income on ticks 5–20 per living founder-tick is 0.109–0.149, against 0.119–0.159 off. But the founders' line now survives in three seeds of five: 10–16 of their descendants are alive at the end of the control (seeds 1, 2 and 4), against none in any seed off.
- **The standing litter falls by about a third, and the inflow rises by about 30 %.** Over the control's second half a cell holds 435 of carcass structure (seed means 360–510), against 629. The inflow is 0.55 a cell-tick (0.51–0.57), against 0.42. More producers die, and each is smaller.
- **`b` is 0.02494** per unit structure per tick. The decomposer founder's upkeep at founding is **0.407 a tick under the per-agent rule and 0.121 size-scaled**, which reproduces #804's figures. The median producer founder, at the same founding structure, pays 0.431 and 0.145.

## Why the founders still die

At the founder's founding structure, upkeep falls to 0.30 of what it was, and income per living founder-tick is roughly the new upkeep. Yet the median founder lives only two or three ticks longer. A throwaway trace of one seed-1 founder (not committed) shows why. The founder's starting reserve of 25.8 is not spent on upkeep. Within about five ticks, growth and reproduction take it, under either rule. By then the founder's structure has risen from 2.5 to about 4, and it holds about 1 of reserve.

The grow phase mobilises reserve above a retention buffer of 3.0 × the metabolic cost (`growth_retention_multiplier`). So a cheaper upkeep also thins the buffer, from about 1.2 to about 0.3, and the surplus that fed upkeep goes to growth and offspring instead. The founder also grows, and its own upkeep with it: at structure 4 it pays about 0.15 a tick, plus movement. The founders then starve on an income near their upkeep, just as they did under the per-agent rule. The difference is that some reproduce first, and in three seeds their line persists. This is an observation for the upkeep work under parent #764. It is not a diagnosis, and nothing here changes the retention rule.

## Method

- **Runs.** `reference_mode --seed S --clear-at 1500 --ticks 1500 --size-scaled-base-rate` for seeds 1–5, release build, on the committed `mode1.json`: decay 0.60, #766's wear, #780's movement, 1× litter, body reach 0, saturating decomposition off. The baseline is the same command without `--size-scaled-base-rate`. Each run settles 1,500 ticks, forks, and runs the control and the perturbed arm to tick 3,000. The guild and the litter are read over the control arm's second half (ticks 2,250–3,000). The founders are read from tick 0, so both arms give the same figures.
- **The baseline reproduces #804.** With the rule off, the litter figures match #804's to the digit: standing 579–667 a cell, inflow 0.38–0.48, and a pooled `S_p` of 21.56 over 18,274 producer-samples.
- **`b`.** `b = base_metabolic_rate / S_p + structure_maintenance_coefficient = 0.32262 / 21.6 + 0.01`. `S_p` is `ESTABLISHED_PRODUCER_STRUCTURE`, #804's pooled median, fixed before the run. Each run's summary records `b`, `S_p` and both founders' upkeep under both rules, and its JSON holds them under `size_scaled_base_rate`.
- **Upkeep** is computed as #804 reckoned it, at founding with no wear: `metabolic_cost` (the base and structure term, trait maintenance and the cross-trait cost), plus movement at a full stride, `eff_mobility · u_M · movement_cost_coefficient · S`. For the decomposer founder it is the mean over the 81 founders. For the producer it is the founder of median per-agent upkeep.

## Results: the control arm, per seed

| seed | rule | producers at end | established producers: samples / median structure / lifespan median | decomposers at end | guild holds / fewest / share of carcass drain | founders alive at 20 / 50 / 100 | founder income, ticks 5–20, per living founder-tick: gained (drained) | founders' line alive at end | standing litter per cell | inflow per cell-tick |
|---:|---|---:|---|---:|---|---|---|---:|---:|---:|
| 1 | off | 149 | 3,885 / 20.5 / 234 | 16 | no / 0 / 0.031 | 1 / 0 / 0 | 0.119 (0.371) | 0 | 667 | 0.388 |
| 1 | on | 336 | 10,528 / 12.0 / 310 | 18 | no / 0 / 0.074 | 10 / 0 / 0 | 0.109 (0.337) | 10 | 375 | 0.512 |
| 2 | off | 143 | 4,113 / 18.1 / 238 | 14 | no / 0 / 0.021 | 1 / 0 / 0 | 0.139 (0.421) | 0 | 596 | 0.444 |
| 2 | on | 431 | 12,483 / 12.2 / 303 | 34 | no / 0 / 0.000 | 13 / 0 / 0 | 0.121 (0.371) | 16 | 467 | 0.566 |
| 3 | off | 132 | 3,863 / 22.6 / 231 | 18 | no / 0 / 0.005 | 2 / 0 / 0 | 0.159 (0.488) | 0 | 664 | 0.430 |
| 3 | on | 472 | 13,489 / 11.9 / 348 | 45 | no / 0 / 0.000 | 20 / 0 / 0 | 0.149 (0.454) | 0 | 465 | 0.567 |
| 4 | off | 113 | 2,763 / 23.2 / 207 | 12 | no / 0 / 0.000 | 1 / 0 / 0 | 0.129 (0.402) | 0 | 579 | 0.376 |
| 4 | on | 473 | 13,760 / 11.1 / 370 | 38 | no / 0 / 0.000 | 13 / 0 / 0 | 0.125 (0.387) | 14 | 510 | 0.539 |
| 5 | off | 143 | 3,650 / 24.2 / 226 | 12 | no / 0 / 0.000 | 2 / 0 / 0 | 0.150 (0.445) | 0 | 638 | 0.481 |
| 5 | on | 412 | 12,460 / 12.4 / 299 | 35 | no / 0 / 0.000 | 13 / 0 / 0 | 0.136 (0.413) | 0 | 360 | 0.569 |
| **mean** | off | 136 | 3,655 / **21.6** pooled / 207–238 | 14 | none holds | 1.4 / 0 / 0 | | 0 | **629** | **0.424** |
| **mean** | on | 425 | 12,544 / **11.9** pooled / 299–370 | 34 | none holds | 13.8 / 0 / 0 | | 8 | **435** | **0.551** |

"Producers persist" is yes in every arm, perturbed and control, under both rules. Producers are the roster count at the arm's end. Carcasses holding structure fall from 3,818–4,713 to 2,398–3,836. The pooled established structure under the rule is 11.9 (n 62,720; p10 2.9, p90 58.8), against 21.6 (p10 5.3, p90 101).

## Upkeep at founding

| founder | structure | per agent: base and structure / metabolic / movement / upkeep | size-scaled: base and structure / metabolic / movement / upkeep |
|---|---:|---|---|
| decomposer (mean of 81) | 2.459 | 0.3472 / 0.3904 / 0.0165 / **0.4069** | 0.0613 / 0.1045 / 0.0165 / **0.1210** |
| producer (median) | 2.459 | 0.3472 / 0.4147 / 0.0165 / 0.4312 | 0.0613 / 0.1288 / 0.0165 / 0.1453 |

#804 gave 0.407 and 0.121 for the decomposer founder, and 0.347 and 0.061 for its base-plus-structure cost. These match.

## Caveats

- **`S_p` is not self-consistent under the new rule.** `b` is set so that a producer of structure 21.6 pays what it paid. Under the rule the established producers' median falls to 11.9, so the median producer now pays less than before. Flow 8 conserves the budget of the producer read under the old rule. Whether `S_p` should be re-read under the new form is a question for the upkeep work, not for this note.
- **One trace, one seed.** The account of the founders' deaths rests on one founder of seed 1, traced with a throwaway example that is not committed.
- **Body reach 0, additive drains.** The reading uses the committed mesocosm. Under saturating decomposition or a body reach, the litter and the guild would read differently.

## Runtime

Release build, five at a time on 8 cores: a paired run took 30–33 s with the rule on, against 20–22 s off. The larger producer population is the extra cost.

## Reproducing

```sh
cargo build --release -p explorers-reference-modes --bin reference_mode
# s = 1..5, five at a time; drop --size-scaled-base-rate for the baseline:
target/release/reference_mode --seed $s --clear-at 1500 --ticks 1500 --size-scaled-base-rate --out target/807/on/s$s.json
```

Each `.md` summary opens with the *Size-scaled base rate* section, which gives `b`, `S_p` and the upkeep table. Its *Founders*, *Heterotroph-dominant decomposer guild* and *Litter supply* sections give the rest. The pooled structure is the nearest-rank median of every `established_structures` entry over the second half's samples of the five controls, as in #804.
