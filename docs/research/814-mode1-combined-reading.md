# Issue #814: the pre-registered combined reading for mode 1

**Status: stopped at step 1 (2026-10-12).** By the issue's pre-registered rule, `S_p` did not settle within 4 iterations under all four rules, so the sweep was not run. There is no verdict on the 36 points, and none of the rule's follow-ups fires. No default changed. The code change is one option, `reference_mode --established-producer-structure S_p`, which gives the size-scaled base rate's `S_p` explicitly.

## What it finds

- **`S_p` falls to about 7 and swings by about 5 % between iterations.** Under all four rules at k = 0, L = 1 and K = 1, the pooled `S_p` reads 7.577, 7.001, 7.402 and 7.013 over the four iterations. The last two moves are +5.7 % and −5.3 %, both above the 5 % limit. So step 1 stops, as the issue orders.
- **One supplementary fifth iteration, outside the rule, moves 1.1 %.** At `S_p` = 7.013 it reads 6.937. It is reported for the owner's decision and was not used to start the sweep.
- **The 25 rule-governed runs are all at k = 0, the point the prediction says fails.** They fail. The guild's fewest members are 0 in every run. Producers persist in every run. Mixotrophs (autotrophy at least 0.1) take all the arm's carcass drain: agents below 0.1 drained 0.0 in every run. Wherever the guild drained at all, its living share is 0, as `ρ` = 1 requires.
- **The founders starve at once.** Every founder is dead by tick 15 or 16 in every run, and none is alive at tick 20. Their income on ticks 5–20 is 0.002 a living founder-tick (drained 0.006–0.008), against 0.109–0.149 in #807 with additive drains and no resistance.
- **The producer stand is larger and its litter stands higher.** At the control's end, 803–1,154 producers stand, against 336–473 in #807. Their median established lifespan is 352–526 ticks. A cell holds 783–861 of standing litter, and the inflow is 0.27–0.31 a cell-tick, against 435 and 0.55 in #807.

## Why the sweep did not run

The issue says: "repeat until `S_p` moves less than 5 % between iterations, or for at most 4 iterations … If it doesn't settle within 4 iterations, stop and report; don't sweep." Iteration 1 is the first read, at #804's 21.6. Iteration 4 is the fourth read. Its move is 5.3 %, so the rule says stop.

The count is read this way, not as a first read followed by four repeats. That other count would allow a fifth read, and the fifth read settles (1.1 %). The rule was fixed before the runs, so the stricter count stands. The choice between the two counts is the owner's.

## Step 1: the iterations

Every run is `reference_mode --seed S --clear-at 1500 --ticks 1500 --size-scaled-base-rate --living-resistance --litter-half-life 1 --enzyme-half-saturation 1`, with `--established-producer-structure S_p` from iteration 2 on. Body reach is 0 (no `--body-reach-pieces`). `S_p` out is the nearest-rank median of every `established_structures` entry over the control arm's second-half samples (ticks 2,250–3,000), pooled over seeds 1–5, as in #804 and #807. Each run's own readout median matches the script's per-seed median. `b = base_metabolic_rate / S_p + structure_maintenance_coefficient = 0.32262 / S_p + 0.01`.

| iteration | `S_p` in | `b` | `S_p` out, pooled (producer-samples) | seed medians | leave-one-seed-out range | move |
|---:|---:|---:|---:|---|---|---:|
| 1 | 21.6 | 0.02494 | 7.577 (147,569) | 7.02–8.27 | −2.2 % to +1.8 % | −64.9 % |
| 2 | 7.577 | 0.05258 | 7.001 (123,498) | 6.44–7.55 | −1.8 % to +2.3 % | −7.6 % |
| 3 | 7.001 | 0.05608 | 7.402 (111,196) | 7.24–7.58 | −0.5 % to +0.6 % | +5.7 % |
| 4 | 7.402 | 0.05359 | 7.013 (116,313) | 6.82–7.21 | −0.8 % to +0.7 % | **−5.3 %: not settled; stop** |
| 5 (supplementary) | 7.013 | 0.05600 | 6.937 (112,000) | 6.43–7.57 | −1.9 % to +1.7 % | −1.1 % |

The leave-one-seed-out range is the pooled median recomputed with each seed dropped. It moves by at most 2.3 %, so the 5–8 % swings of iterations 2–4 are larger than resampling the seeds. They are not shown to be noise. Each `b` also changes every trajectory, which the leave-one-out range does not measure.

At iteration 4's `b` (0.0536), a decomposer founder's upkeep at founding is 0.191 a tick, against 0.121 at 21.6 and 0.407 under the per-agent rule. Its base-plus-structure cost is 0.132.

## The k = 0 runs, per iteration and seed

The control arm. "Guild share" is the guild's share of all carcass drain over the arm's second half. "Mixotroph share" is the share of the arm's carcass drain by drainers of autotrophy at least 0.1. "Living share" is the guild's living drain over its total ("-" where the guild drained nothing). Income is energy gained per living founder-tick over ticks 5–20, drained in brackets. Litter and inflow are per cell, means over the second half. `k_d` is 0.003027 a tick (L = 1 at `T_p` = 229 ticks) and `K` is 1.0734 absolute (1 founder capability) in every run. The body-reach coefficient is 0 (k = 0). Runtime is wall-clock, CPU in brackets.

| iteration | `S_p` in | `b` | seed | guild holds (fewest) | guild share | mixotroph share | living share | producers persist | pass | founders alive 20 / 50 / 100 | income 5–20 | last founder death | standing litter | inflow | `S_p` out | runtime s |
|---|---:|---:|---:|---|---:|---:|---:|---|---|---|---:|---:|---:|---:|---:|---:|
| 1 | 21.600 | 0.02494 | 1 | no (0) | 0.000 | 1.000 | - | yes | no | 0 / 0 / 0 | 0.002 (0.007) | 16 | 851 | 0.266 | 7.72 | 54 (53) |
| 1 | 21.600 | 0.02494 | 2 | no (0) | 0.000 | 1.000 | - | yes | no | 0 / 0 / 0 | 0.002 (0.007) | 16 | 861 | 0.265 | 7.02 | 58 (57) |
| 1 | 21.600 | 0.02494 | 3 | no (0) | 0.000 | 1.000 | 0.000 | yes | no | 0 / 0 / 0 | 0.002 (0.008) | 16 | 846 | 0.269 | 7.41 | 61 (60) |
| 1 | 21.600 | 0.02494 | 4 | no (0) | 0.000 | 1.000 | - | yes | no | 0 / 0 / 0 | 0.003 (0.008) | 16 | 827 | 0.271 | 7.53 | 59 (58) |
| 1 | 21.600 | 0.02494 | 5 | no (0) | 0.000 | 1.000 | - | yes | no | 0 / 0 / 0 | 0.002 (0.007) | 16 | 851 | 0.279 | 8.27 | 58 (57) |
| 2 | 7.577 | 0.05258 | 1 | no (0) | 0.000 | 1.000 | - | yes | no | 0 / 0 / 0 | 0.002 (0.007) | 15 | 798 | 0.294 | 7.12 | 51 (51) |
| 2 | 7.577 | 0.05258 | 2 | no (0) | 0.002 | 1.000 | 0.000 | yes | no | 0 / 0 / 0 | 0.002 (0.007) | 15 | 803 | 0.314 | 7.12 | 54 (53) |
| 2 | 7.577 | 0.05258 | 3 | no (0) | 0.000 | 1.000 | - | yes | no | 0 / 0 / 0 | 0.002 (0.008) | 15 | 825 | 0.287 | 6.44 | 53 (52) |
| 2 | 7.577 | 0.05258 | 4 | no (0) | 0.000 | 1.000 | - | yes | no | 0 / 0 / 0 | 0.003 (0.008) | 15 | 795 | 0.297 | 7.55 | 48 (48) |
| 2 | 7.577 | 0.05258 | 5 | no (0) | 0.000 | 1.000 | - | yes | no | 0 / 0 / 0 | 0.002 (0.006) | 15 | 810 | 0.279 | 6.86 | 47 (46) |
| 3 | 7.001 | 0.05608 | 1 | no (0) | 0.013 | 1.000 | 0.000 | yes | no | 0 / 0 / 0 | 0.002 (0.007) | 15 | 790 | 0.290 | 7.36 | 54 (54) |
| 3 | 7.001 | 0.05608 | 2 | no (0) | 0.006 | 1.000 | 0.000 | yes | no | 0 / 0 / 0 | 0.002 (0.007) | 15 | 787 | 0.295 | 7.50 | 53 (52) |
| 3 | 7.001 | 0.05608 | 3 | no (0) | 0.000 | 1.000 | - | yes | no | 0 / 0 / 0 | 0.002 (0.008) | 15 | 825 | 0.287 | 7.32 | 47 (46) |
| 3 | 7.001 | 0.05608 | 4 | no (0) | 0.000 | 1.000 | - | yes | no | 0 / 0 / 0 | 0.003 (0.008) | 15 | 798 | 0.302 | 7.58 | 52 (52) |
| 3 | 7.001 | 0.05608 | 5 | no (0) | 0.000 | 1.000 | - | yes | no | 0 / 0 / 0 | 0.002 (0.006) | 15 | 793 | 0.268 | 7.24 | 46 (45) |
| 4 | 7.402 | 0.05359 | 1 | no (0) | 0.000 | 1.000 | - | yes | no | 0 / 0 / 0 | 0.002 (0.007) | 15 | 808 | 0.307 | 7.09 | 50 (50) |
| 4 | 7.402 | 0.05359 | 2 | no (0) | 0.001 | 1.000 | 0.000 | yes | no | 0 / 0 / 0 | 0.002 (0.007) | 15 | 783 | 0.309 | 7.20 | 54 (53) |
| 4 | 7.402 | 0.05359 | 3 | no (0) | 0.014 | 1.000 | 0.000 | yes | no | 0 / 0 / 0 | 0.002 (0.008) | 15 | 792 | 0.265 | 7.07 | 49 (49) |
| 4 | 7.402 | 0.05359 | 4 | no (0) | 0.000 | 1.000 | - | yes | no | 0 / 0 / 0 | 0.003 (0.008) | 15 | 813 | 0.278 | 6.82 | 51 (51) |
| 4 | 7.402 | 0.05359 | 5 | no (0) | 0.000 | 1.000 | - | yes | no | 0 / 0 / 0 | 0.002 (0.006) | 15 | 804 | 0.298 | 6.84 | 56 (56) |
| 5 (supp.) | 7.013 | 0.05600 | 1 | no (0) | 0.000 | 1.000 | - | yes | no | 0 / 0 / 0 | 0.002 (0.007) | 15 | 827 | 0.278 | 6.43 | 934 (44) |
| 5 (supp.) | 7.013 | 0.05600 | 2 | no (0) | 0.000 | 1.000 | - | yes | no | 0 / 0 / 0 | 0.002 (0.007) | 15 | 829 | 0.293 | 7.03 | 937 (48) |
| 5 (supp.) | 7.013 | 0.05600 | 3 | no (0) | 0.000 | 1.000 | - | yes | no | 0 / 0 / 0 | 0.002 (0.008) | 15 | 851 | 0.269 | 6.62 | 934 (45) |
| 5 (supp.) | 7.013 | 0.05600 | 4 | no (0) | 0.004 | 1.000 | 0.000 | yes | no | 0 / 0 / 0 | 0.003 (0.008) | 15 | 823 | 0.275 | 7.05 | 936 (46) |
| 5 (supp.) | 7.013 | 0.05600 | 5 | no (0) | 0.009 | 1.000 | 0.000 | yes | no | 0 / 0 / 0 | 0.002 (0.006) | 15 | 802 | 0.306 | 7.56 | 939 (49) |

The guild's share is non-zero in a few runs while the mixotroph share is 1.000. Guild members are agents with heterotrophy above autotrophy, and some of them have autotrophy of at least 0.1. Producers at the control's end: 1,051–1,154 in iteration 1, 803–969 in iterations 2–5.

## Observations, not verdicts

- **Founder income at k = 0 is about what saturating kinetics allows at contact.** A carcass under saturation yields about `k_d · S · Σc / (K + Σc)` a tick. At `k_d` 0.003, a carcass of about 8–15 structure and one founder at `c / (K + c)` ≈ 0.5, that is about 0.01–0.02 drained a tick. The founders drained 0.006–0.008. Under #807's additive drains they drained 0.34–0.45. This is arithmetic on the runs, not a diagnosis. k = 0 is the point the prediction says fails, so it tests nothing new.
- **`S_p` depends on the whole rule set.** It is 21.6 under the per-agent rule (#804), 11.9 size-scaled alone (#807), and about 7 with all four rules on. A sweep at another (k, L, K) may move it again. Step 1 settles it at one point only.

## What the owner decides

None of the issue's follow-ups fires, since they are read off the sweep. These are the options, not acted on here:
- **Count the fifth read.** If the first read is iteration 0, the fourth repeat moves 1.1 % and settles at `S_p` = 6.94, `b` = 0.0565. The sweep can then run at that `b` with `--established-producer-structure 6.937`.
- **Widen the tolerance or average.** The swings are 5–8 %, and the seed medians span about ±8 % within an iteration.
- **Sweep at the last `b` anyway**, with the rule's verdict caveated by the unsettled `S_p`.

The sweep is sized but not piloted at its worst cells. At the k = 0 cost of 46–61 s a run, 180 runs five at a time are about 30–40 minutes of wall-clock.

## The code change

`reference_mode --established-producer-structure S_p` (#814) sets `S_p` for `--size-scaled-base-rate`, which it needs. It must be finite and above 0. The library function is `set_size_scaled_base_rate_at(world, s_p)`. `set_size_scaled_base_rate` calls it at `ESTABLISHED_PRODUCER_STRUCTURE`, 21.6, which keeps #804's provenance. An option fits better than a new constant: the settled value depends on the rules and the point it is read at, and #807's runs stay reproducible. Without the option every run is unchanged. At seed 3 over 200 ticks the plain run, `--body-reach-pieces 10`, `--size-scaled-base-rate`, and `--size-scaled-base-rate --established-producer-structure 21.6` all write reports byte-identical to main's.

## Runtime

Release build, five at a time on 8 cores: a paired run with all four rules on at k = 0, L = 1 and K = 1 took 46–61 s of wall-clock and 45–60 s of CPU. Each iteration's five runs took about 60 s. That is about twice #807's 30–33 s, with two to three times the producers. The supplementary iteration's runs took 934–939 s of wall-clock but 44–49 s of CPU. The laptop idle-slept on battery for 917 s during them (`pmset -g log`), the trap in *Running long sweeps*.

## Reproducing

```sh
cargo build --release -p explorers-reference-modes --bin reference_mode
# s = 1..5, five at a time; S_p = 7.577, 7.001, 7.402 for iterations 2-4 (omit the flag for iteration 1):
target/release/reference_mode --seed $s --clear-at 1500 --ticks 1500 --size-scaled-base-rate \
  --established-producer-structure $S_p --living-resistance \
  --litter-half-life 1 --enzyme-half-saturation 1 --out target/814/step1/itN/s$s.json
```

Each `.md` summary gives `b`, `S_p`, the kinetics, the guild, the founders, the litter and the established producers' structure. The JSON's `report.control.litter.samples[*].established_structures` holds the structures pooled for `S_p`.
