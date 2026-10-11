# Issue #804: what the mode 1 litter can feed

**Status: measurement (2026-10-11).** The readout #804 adds (standing litter, litter inflow and the established producers' structure) was read on the committed mesocosm (decay 0.60, #770 wear, #780 movement, 1× litter, body reach 0, saturating decomposition off). It is input for the upkeep and defence grills. It carries no verdict and changes no parameter.

## What it finds

- **The standing litter is about twelve times what the design assumed.** Over the control's second half, a cell holds **629 energy** of carcass structure (seed means 579–667), not the founding stock of 54 that *Drain kinetics, and what the litter can feed* reasons from. It has levelled off: today's additive drains by producer-line mixotrophs take 0.37–0.41 a cell-tick over the arm, against an inflow of 0.42. About half the ~9,400 carcasses standing are spent (structure 0, never removed while saturating decomposition is off), so ~4,400 hold structure, about 54 a cell.
- **The inflow is 0.42 a cell-tick** (seed means 0.38–0.48), 97 % of it from producer deaths. That is what litter could yield for ever. Under saturating kinetics a stock above `inflow / k_d` (70, 140 and 560 a cell at half-lives of 0.5, 1 and 4 lifespans) is drawn down, so today's 629 is a transient ceiling, not a supply.
- **`S_p` is 21.6** (pooled median over 18,274 producer-samples; seed medians 18.1–24.2). The spread is wide and right-skewed: p10 5.3, p25 9.8, p75 49.1, p90 101.
- **A decomposer founder's upkeep is 0.407 a tick today, and 0.121 under the size-scaled base rate** (0.117–0.128 across seeds' `S_p`). The design's 0.46 is high: movement at the founder's stride costs 0.017, not about 0.04.
- **What the world's litter could carry, sustained (the inflow, assimilated at 0.31):** about **26 founders today and 88 under the size-scaled rule**, on 81 cells. Today one cell's inflow pays 0.32 of a founder, so a founder must collect from about three cells. Under the size-scaled rule one cell's inflow pays 1.08 founders. While today's standing stock lasts, the ceiling is far higher: 117 founders today and 395 size-scaled at a half-life of one lifespan.
- **The arithmetic is for decomposers taking all of the litter.** Today the producer-line mixotrophs take it: they drained 45,000–50,000 of carcass structure over each control arm, and agents below autotrophy 0.1 drained at most 0.97.

## Method

- **Runs.** `reference_mode --seed S --clear-at 1500 --ticks 1500` for seeds 1–5, release build, on the committed `mode1.json`, without `--body-reach-pieces` (body reach 0). Each settles 1,500 ticks, forks, and runs the control and the perturbed arm to tick 3,000. Everything below is the **control arm's second half**, the readout's window: samples at ticks 2,250–3,000, and inflow over the sample intervals ending at ticks 2,275–3,000.
- **The readout** (#804, `LitterReadout` in `crates/explorers-reference-modes/src/mesocosm.rs`), per arm, at the 25-tick sample cadence:
  - **standing litter:** the summed structure of every carcass in the world over its 81 cells. A carcass's energy is its structure. Spent carcasses stand at 0, so they add nothing to it, but they are in the carcass count; the readout also counts those holding structure;
  - **litter inflow:** the structure at death of each carcass a death creates in the interval, split by whether the dead agent was a producer (the income read, as for the producer lifespans) or anything else, per cell per tick. A `Died` event carries no energy, so the readout reads the new carcass's `energy_at_death` by id;
  - **established producers:** the structure of every living producer older than 50 ticks (`ESTABLISHMENT_AGE`) at each sample, pooled over the window's samples for `S_p`; and the arm's median established lifespan (age at death over 50 ticks).
- **Producer lifespan.** The supply arithmetic uses the 229-tick median established producer lifespan of #766's calibration (`766-mode1-wear-recalibration.md`). The runs' own established medians are 207–238 (below). The median over every producer death is 16–20 ticks, set by seedlings, and is not a lifespan for this purpose.
- **Before and after.** A short paired run (`--seed 1 --clear-at 400 --ticks 400`) was captured before the change. After it, the `.md` differs only by the two added sections, and the JSON is identical with the new `litter` fields removed.

## Results: the control arm, per seed

"Standing" is mean (min–max) over the window's 31 samples. "Inflow" is the window's total over its 750 ticks, with the least and greatest 25-tick interval in brackets. All are per nutrient cell.

| seed | standing litter per cell | inflow per cell per tick | from producer deaths | from other deaths | carcasses: all / holding structure (mean) | established producers alive (mean per sample) | `S_p`: median | p10 / p25 / p75 / p90 | established lifespan median | runtime |
|---:|---:|---:|---:|---:|---:|---:|---:|---|---:|---:|
| 1 | 667.5 (658.1–679.7) | 0.388 (0.091–0.749) | 0.374 | 0.014 | 9,477 / 4,713 | 125 | 20.5 | 5.3 / 9.7 / 47.0 / 89.9 | 234 | 22 s |
| 2 | 596.1 (579.9–609.0) | 0.444 (0.284–0.775) | 0.429 | 0.016 | 9,853 / 4,449 | 133 | 18.1 | 4.7 / 8.5 / 45.3 / 92.9 | 238 | 22 s |
| 3 | 664.4 (654.8–679.4) | 0.430 (0.226–0.676) | 0.417 | 0.013 | 9,357 / 4,547 | 125 | 22.6 | 6.0 / 10.8 / 50.9 / 109.9 | 231 | 22 s |
| 4 | 579.2 (555.2–593.3) | 0.376 (0.087–0.578) | 0.362 | 0.014 | 9,438 / 3,818 | 89 | 23.2 | 4.8 / 10.3 / 48.6 / 96.5 | 207 | 21 s |
| 5 | 638.4 (620.1–658.8) | 0.481 (0.088–1.021) | 0.465 | 0.016 | 9,072 / 4,273 | 118 | 24.2 | 5.7 / 10.5 / 56.0 / 114.6 | 226 | 20 s |
| **mean / pooled** | **629.1** | **0.424** | 0.410 | 0.014 | 9,439 / 4,360 | 118 | **21.6** (pooled, n 18,274) | 5.3 / 9.8 / 49.1 / 101.3 (pooled) | 207–238 | |

The standing litter is flat over the second half in every seed (seed 1: 670 at tick 2,250, 666 at 3,000). It rose from 54 at founding to 519–606 by the fork at tick 1,500.

## Supply at the band's litter half-lives

`k_d = ln 2 / (h · 229)` for a half-life of `h` producer lifespans, under full saturation. Means over the five seeds. "Assimilated" is drained × 0.31, flow 7's efficiency for a pure-vertex decomposer on producer litter (0.314 at decay 0.60).

| half-life | ticks | `k_d` | `k_d` × standing: drained / assimilated | `k_d` × standing + inflow: drained / assimilated | inflow alone (sustained): drained / assimilated | standing at steady state, `inflow / k_d` |
|---:|---:|---:|---:|---:|---:|---:|
| 0.5 | 115 | 0.00605 | 3.81 / 1.18 | 4.23 / 1.31 | 0.424 / 0.131 | 70 |
| 1 | 229 | 0.00303 | 1.90 / 0.59 | 2.33 / 0.72 | 0.424 / 0.131 | 140 |
| 4 | 916 | 0.00076 | 0.48 / 0.15 | 0.90 / 0.28 | 0.424 / 0.131 | 560 |

At the design's 54 per cell the same formula gives 0.33, 0.16 and 0.04 drained, the design's figures, so the difference is the stock and not the arithmetic.

**Which column is the supply.** `k_d × standing + inflow` is what one tick could yield now, the most a cell gives. It is not sustainable. Under saturating kinetics the stock relaxes to `inflow / k_d`, and from then on the yield is the inflow. So the inflow, 0.42 drained and 0.13 assimilated a cell-tick, is the supply at every half-life. The half-life sets how long the stock above the steady state lasts and how much litter stands, not what it yields for ever. At a half-life of 4 lifespans today's stock is near its steady state (629 against 560). At 0.5 lifespans it is nine times it.

Using each seed's own established lifespan instead of 229 moves `k_d × standing` by at most 11 % (seed 4, 207 ticks).

## Supply against a decomposer founder's upkeep

**The founder.** Heterotrophy 1.0734, mobility 0.1432, asexual propensity 0.1504, autotrophy 0, co-limited founding structure `S = 2.459` (`mode1.json`), at founding with no wear.

**Today** (`metabolic_cost` and the movement phase, `crates/explorers-sim/src/phase.rs`), with `p` = `maintenance_cost_exponent` = 2.205:

```
upkeep = base_metabolic_rate
       + heterotrophy_maintenance_cost · H^p + mobility_maintenance_cost · M^p
       + asexual_propensity_maintenance_cost · A^p
       + structure_maintenance_coefficient · S
       + d · movement_cost_coefficient · S,     d = eff_mobility · u_M
       = 0.3226 + (0.04304 + 0 + 0.00015) + 0.01 · 2.459 + 0.1432 · 1.0 · 0.04693 · 2.459
       = 0.3226 + 0.0432 + 0.0246 + 0.0165
       = 0.407 a tick
```

The cross-trait cost is 0 in the mesocosm. Movement assumes a full stride every tick, which the founder takes unless its pull and random walk cancel. The design's "about 0.46" came from #764's diagnosis (metabolism 0.40–0.42, movement about 0.04). At the founder's traits and founding structure the movement term is 0.017, so 0.41 is the figure here.

**Under the size-scaled base rate** (world rules, flow 8, *How large `b` is*): the base rate becomes `b · S` with `b = base_metabolic_rate / S_p + structure_maintenance_coefficient`. That absorbs today's structure term, so it is not charged again:

```
upkeep = b · S + trait maintenance + movement
b      = 0.3226 / 21.56 + 0.01 = 0.02496
upkeep = 0.02496 · 2.459 + 0.0432 + 0.0165 = 0.0614 + 0.0432 + 0.0165 = 0.121 a tick
```

Per seed's `S_p` (18.1–24.2), `b` is 0.0233–0.0278 and the upkeep 0.117–0.128. The founder's base-plus-structure cost falls from 0.347 to 0.061, a factor of 5.7, so trait maintenance (0.043) and movement (0.017) become half its upkeep.

**How many founders the world's litter could carry** (81 cells; assimilated supply over upkeep):

| supply | per cell, assimilated | founders on the world, today (0.407) | founders on the world, size-scaled (0.121) |
|---|---:|---:|---:|
| **inflow, sustained (any half-life)** | 0.131 | **26** | **88** |
| `k_d` × standing, half-life 0.5 | 1.18 | 235 | 790 |
| `k_d` × standing, half-life 1 | 0.59 | 117 | 395 |
| `k_d` × standing, half-life 4 | 0.15 | 29 | 99 |
| `k_d` × standing + inflow, half-life 0.5 / 1 / 4 | 1.31 / 0.72 / 0.28 | 261 / 144 / 56 | 878 / 483 / 187 |

Per cell, the sustained inflow pays 0.32 of a founder today and 1.08 under the size-scaled rule. While today's standing stock lasts, one cell pays 2.9, 1.45 and 0.36 founders today at half-lives of 0.5, 1 and 4 lifespans, against the design's "cannot pay for one founder, by a factor of 5–35" from 54 a cell.

## Caveats

- **The stock and the inflow are today's world.** The standing litter is what additive drains by producer-line mixotrophs leave. The inflow is today's producer turnover. A world with saturating kinetics, a guild and a size-scaled base rate would change both: the stock relaxes to `inflow / k_d`, and nutrient returned faster may change producer turnover. The size-scaled rule also re-prices producers, so the inflow would be re-read (flow 8 says the producer control's persistence is re-read, not assumed).
- **Decomposers would share the supply.** The founder counts assume decomposers take all of it. Today the mixotrophs take nearly all of it, and under saturation every drainer on a carcass takes a share of its yield.
- **Upkeep is the founder at founding.** A grown decomposer pays more structure and movement; one that does not move pays less.
- **A cell's litter is not one carcass.** The inflow arrives in about 54 standing pieces a cell, so the founder still needs the reach to gather it (mode 1, *Feeding reach*).

## Runtime

Release build, five at a time on 8 cores: a paired run (`--clear-at 1500 --ticks 1500`) took 20–22 s, as before the readout (#796: 18–23 s). The five took 22 s of wall-clock. The JSON is 0.3–0.4 MB a run, most of it the established producers' structures in every sample.

## Reproducing

```sh
cargo build --release -p explorers-reference-modes --bin reference_mode
# s = 1..5, five at a time:
target/release/reference_mode --seed $s --clear-at 1500 --ticks 1500 --out target/804/runs/s$s.json
```

Each `.md` summary carries, per arm, the *Litter supply, second half* and *Established producers, second half* sections. The JSON's `report.control.litter` holds the series (`samples`, each with `standing_per_cell`, `carcasses`, `nonempty_carcasses`, `inflow`, `inflow_ticks` and `established_structures`) and the window (`second_half`). The pooled `S_p` is the nearest-rank median of every `established_structures` entry over the second half's samples of the five controls. The founder's upkeep terms were read from the built world with `metabolic_cost` and the movement formula, through a throwaway example that is not committed.
