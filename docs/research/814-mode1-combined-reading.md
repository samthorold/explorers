# Issue #814: the pre-registered combined reading for mode 1

**Status: done (2026-10-12). No point passes: 0 of 36.** Step 1 settled `S_p` at 6.937 (`b` 0.0565), counting its first read as iteration 0, by the owner's decision of 2026-10-12. The 180-run sweep ran at that `b`. The guild holds in none of the 180 runs. Producers persist in all of them. Two of the rule's follow-ups fire: **lean-time reserve is grilled next** ("many" points fail with producers intact), and **the specialist's advantage on litter is grilled next** (mixotrophs still take all of the carcass drain). No default changed, and none is recommended to change. The only code change is step 1's option, `reference_mode --established-producer-structure S_p`.

## What it finds

- **Every point fails, including the k = 10 and k = 30 points the prediction says pass.** The guild's fewest members are 0 in 179 runs and 1 in one (k = 30, L = 1, K = 1, seed 5), against the 5 it needs. The k = 0 points fail, as predicted. k = 3 carries no prediction and fails too.
- **Producer-line mixotrophs take all the carcass drain.** In every run, drainers of autotrophy below 0.1 drained 0.0 over the control arm. The guild (heterotrophy above autotrophy) takes at most 10.2 % of the carcass drain, through members whose autotrophy is at least 0.1. The competition prediction applies only where a point passes, so it is untested. The data runs against it.
- **The check holds.** The guild's living drain is 0.0 in all 180 runs, so its living share is 0 wherever it drained, as `ρ` = 1 requires.
- **Producers persist in all 180 runs,** with 688–1,033 producers in the control at its end. No mechanism's off state needs naming.
- **The founders still starve at once.** None is alive at tick 20 in any run. The last founder dies at tick 14–16. Income on ticks 5–20 rises with reach and falls with half-life and K. Its best is 0.057–0.059 a living founder-tick (drained 0.181–0.188) at k = 30, L = 0.5 and K = 1, against a founder's size-scaled upkeep of 0.199 at founding. At k = 0 it is 0.000–0.005.
- **At k = 30 with short-lived litter, the founders' line outlives them.** At k = 30, L = 0.5 and K = 1 the readout's founder line is alive at the end in every seed (18–84 agents), and at L = 1, K = 1 in two seeds (39 and 45). No guild holds in those runs, and no agent below autotrophy 0.1 drained a carcass in the arm's second half. So the line survives as something other than a heterotroph-dominant decomposer guild. Where the line went is not read here.
- **Reach and short half-lives draw the standing litter down.** A cell's standing litter is 732–889 at k = 0, and 144–214 at k = 30, L = 0.5, K = 1. Inflow rises from 0.22–0.33 a cell-tick at k = 0 to 0.38–0.55 at k = 30, L = 0.5.

## Step 1's iteration count: the owner's decision

PR #818 ran step 1 and stopped by the rule. It counted the first read, at #804's 21.6, as iteration 1. Its fourth read moved 5.3 %, above the 5 % limit. A supplementary fifth read, outside that count, moved 1.1 %.

The issue's rule reads: "repeat until `S_p` moves less than 5 % between iterations, or for at most 4 iterations". On 2026-10-12, after step 1, the owner clarified the count on #814: **the first read (21.6 in, 7.577 out) is iteration 0**. The four repeats are iterations 1–4. The fifth read is then iteration 4, within the rule, and it settles: 7.013 → 6.937, −1.1 %. The settled value is **`S_p` = 6.937**, `b` = 0.32262 / 6.937 + 0.01 = **0.05651**. The sweep ran at it with `--established-producer-structure 6.937`. The prediction, the fixed rule and the follow-ups are unchanged. The count was clarified after step 1's runs, which this note records.

Step 1's runs were not redone. The tables below keep #818's numbering, in which the settling read is "5 (supplementary)".

## Step 1: the iterations

Every run is `reference_mode --seed S --clear-at 1500 --ticks 1500 --size-scaled-base-rate --living-resistance --litter-half-life 1 --enzyme-half-saturation 1`, with `--established-producer-structure S_p` from iteration 2 on. Body reach is 0 (no `--body-reach-pieces`). `S_p` out is the nearest-rank median of every `established_structures` entry over the control arm's second-half samples (ticks 2,250–3,000), pooled over seeds 1–5, as in #804 and #807. Each run's own readout median matches the script's per-seed median. `b = base_metabolic_rate / S_p + structure_maintenance_coefficient = 0.32262 / S_p + 0.01`.

| iteration | `S_p` in | `b` | `S_p` out, pooled (producer-samples) | seed medians | leave-one-seed-out range | move |
|---:|---:|---:|---:|---|---|---:|
| 1 | 21.6 | 0.02494 | 7.577 (147,569) | 7.02–8.27 | −2.2 % to +1.8 % | −64.9 % |
| 2 | 7.577 | 0.05258 | 7.001 (123,498) | 6.44–7.55 | −1.8 % to +2.3 % | −7.6 % |
| 3 | 7.001 | 0.05608 | 7.402 (111,196) | 7.24–7.58 | −0.5 % to +0.6 % | +5.7 % |
| 4 | 7.402 | 0.05359 | 7.013 (116,313) | 6.82–7.21 | −0.8 % to +0.7 % | −5.3 %: #818 stopped here |
| 5 (supplementary) | 7.013 | 0.05600 | 6.937 (112,000) | 6.43–7.57 | −1.9 % to +1.7 % | **−1.1 %: settled, by the owner's count** |

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

## Step 1 observations, not verdicts

- **Founder income at k = 0 is about what saturating kinetics allows at contact.** A carcass under saturation yields about `k_d · S · Σc / (K + Σc)` a tick. At `k_d` 0.003, a carcass of about 8–15 structure and one founder at `c / (K + c)` ≈ 0.5, that is about 0.01–0.02 drained a tick. The founders drained 0.006–0.008. Under #807's additive drains they drained 0.34–0.45. This is arithmetic on the runs, not a diagnosis. k = 0 is the point the prediction says fails, so it tests nothing new.
- **`S_p` depends on the whole rule set.** It is 21.6 under the per-agent rule (#804), 11.9 size-scaled alone (#807), and about 7 with all four rules on. A sweep at another (k, L, K) may move it again. Step 1 settles it at one point only.

## Step 2: the sweep

**Method.** Every run is `reference_mode --seed S --clear-at 1500 --ticks 1500 --size-scaled-base-rate --established-producer-structure 6.937 --living-resistance --litter-half-life L --enzyme-half-saturation K --body-reach-pieces k` at 1× litter, for k = 0, 3, 10, 30, L = 0.5, 1, 4, K = 1, 3, 10 and seeds 1–5: 36 points, 180 paired runs. The release binary was copied to `target/814-sweep/` and each run wrote its own directory, `target/814-sweep/runs/k{k}_L{L}_K{K}_s{S}/`, skipped if its report existed, so the sweep could resume. Every column is read from each run's `.md` summary and JSON over the control arm's second half (ticks 2,250–3,000), as in step 1 and #794.

The fixed settings, per run:
- `b` = 0.05651 in every run (`S_p` 6.937).
- `k_d` = 0.006054, 0.003027 and 0.000757 a tick at L = 0.5, 1 and 4 (`T_p` = 229 ticks).
- `K` = 1.0734, 3.2203 and 10.7343 absolute at 1, 3 and 10 founder capabilities.
- The body-reach coefficient is 0, 0.5895, 1.7522 and 3.6342 at k = 0, 3, 10 and 30 (no clamp).

The columns:
- **guild holds, fewest members, guild share:** the #794 readout. The guild share is the guild's carcass drain over all carcass drain in the arm's second half.
- **mixotroph share:** the share of the arm's carcass drain taken by drainers of autotrophy at least 0.1, as in step 1.
- **living share:** the guild's living drain over its total ("-" where it drained nothing).
- **producers persist:** the centre patch's producers persist (#794), with the control's producers at its end in brackets.
- **founders alive at 20, founder line alive at end, income:** from the readout's founders table. Income is energy gained per living founder-tick on ticks 5–20, with drained in brackets.
- **standing litter and inflow:** per cell, means over the second half.
- **runtime:** wall-clock, with CPU in brackets.

### Summary per (k, L) at each K

Ranges are over the five seeds. The per-seed rows are in the appendix.

**K = 1**

| k | L | seeds passing | verdict | guild holds | fewest members | guild share | mixotroph share | living share | producers persist (at end) | founders alive at 20 | founder line alive at end | income 5–20 | standing litter | inflow | runtime s (CPU s) |
|---:|---:|---:|---|---:|---|---|---|---|---|---:|---|---|---|---|---|
| 0 | 0.5 | 0 / 5 | fail | 0 | 0 | 0.000–0.014 | 1.000 | 0.000 | 5 (815–888) | 0 | 0 | 0.004 (0.013) to 0.005 (0.016) | 732–777 | 0.300–0.333 | 46–50 (45–49) |
| 0 | 1 | 0 / 5 | fail | 0 | 0 | 0.000–0.001 | 1.000 | 0.000 | 5 (792–956) | 0 | 0 | 0.002 (0.006) to 0.003 (0.008) | 777–804 | 0.269–0.295 | 51–56 (50–55) |
| 0 | 4 | 0 / 5 | fail | 0 | 0 | 0.000–0.000 | 1.000 | 0.000 | 5 (812–973) | 0 | 0 | 0.001 (0.002) | 838–881 | 0.250–0.263 | 38–53 (38–52) |
| 3 | 0.5 | 0 / 5 | fail | 0 | 0 | 0.000–0.002 | 1.000 | 0.000 | 5 (714–870) | 0 | 0 | 0.008 (0.026) to 0.009 (0.030) | 656–682 | 0.357–0.411 | 71–85 (70–84) |
| 3 | 1 | 0 / 5 | fail | 0 | 0 | 0.000–0.004 | 1.000 | 0.000 | 5 (756–931) | 0 | 0 | 0.004 (0.013) to 0.005 (0.015) | 741–764 | 0.313–0.338 | 69–76 (68–76) |
| 3 | 4 | 0 / 5 | fail | 0 | 0 | 0.000 | 1.000 | - | 5 (850–1033) | 0 | 0 | 0.001 (0.003) to 0.001 (0.004) | 826–862 | 0.264–0.282 | 44–79 (44–78) |
| 10 | 0.5 | 0 / 5 | fail | 0 | 0 | 0.000–0.004 | 1.000 | 0.000 | 5 (757–806) | 0 | 0 | 0.026 (0.082) to 0.028 (0.088) | 414–513 | 0.433–0.497 | 147–222 (145–220) |
| 10 | 1 | 0 / 5 | fail | 0 | 0 | 0.000–0.000 | 1.000 | 0.000 | 5 (688–954) | 0 | 0 | 0.013 (0.040) to 0.013 (0.043) | 580–655 | 0.380–0.415 | 113–172 (112–171) |
| 10 | 4 | 0 / 5 | fail | 0 | 0 | 0.000–0.000 | 1.000 | 0.000 | 5 (848–999) | 0 | 0 | 0.003 (0.010) to 0.003 (0.011) | 774–824 | 0.290–0.313 | 93–162 (92–160) |
| 30 | 0.5 | 0 / 5 | fail | 0 | 0 | 0.000–0.071 | 1.000 | 0.000 | 5 (719–803) | 0 | 18–84 | 0.057 (0.181) to 0.059 (0.188) | 144–214 | 0.468–0.518 | 551–774 (542–764) |
| 30 | 1 | 0 / 5 | fail | 0 | 0–1 | 0.001–0.102 | 1.000 | 0.000 | 5 (747–842) | 0 | 0–45 | 0.029 (0.091) to 0.030 (0.095) | 349–486 | 0.439–0.480 | 453–787 (446–775) |
| 30 | 4 | 0 / 5 | fail | 0 | 0 | 0.000–0.002 | 1.000 | 0.000 | 5 (763–926) | 0 | 0 | 0.007 (0.022) to 0.007 (0.023) | 674–778 | 0.320–0.366 | 354–524 (343–508) |

**K = 3**

| k | L | seeds passing | verdict | guild holds | fewest members | guild share | mixotroph share | living share | producers persist (at end) | founders alive at 20 | founder line alive at end | income 5–20 | standing litter | inflow | runtime s (CPU s) |
|---:|---:|---:|---|---:|---|---|---|---|---|---:|---|---|---|---|---|
| 0 | 0.5 | 0 / 5 | fail | 0 | 0 | 0.000–0.009 | 1.000 | 0.000 | 5 (841–933) | 0 | 0 | 0.002 (0.007) to 0.003 (0.008) | 799–836 | 0.275–0.307 | 46–50 (45–49) |
| 0 | 1 | 0 / 5 | fail | 0 | 0 | 0.000–0.000 | 1.000 | 0.000 | 5 (854–938) | 0 | 0 | 0.001 (0.003) to 0.001 (0.004) | 822–844 | 0.253–0.270 | 51–57 (50–57) |
| 0 | 4 | 0 / 5 | fail | 0 | 0 | 0.000 | 1.000 | - | 5 (825–956) | 0 | 0 | 0.000 (0.001) | 856–889 | 0.236–0.257 | 49–59 (48–58) |
| 3 | 0.5 | 0 / 5 | fail | 0 | 0 | 0.000–0.000 | 1.000 | 0.000 | 5 (760–921) | 0 | 0 | 0.004 (0.013) to 0.005 (0.016) | 709–769 | 0.316–0.338 | 71–80 (70–79) |
| 3 | 1 | 0 / 5 | fail | 0 | 0 | 0.000–0.013 | 1.000 | 0.000 | 5 (838–948) | 0 | 0 | 0.002 (0.007) to 0.002 (0.008) | 792–841 | 0.282–0.309 | 61–80 (60–80) |
| 3 | 4 | 0 / 5 | fail | 0 | 0 | 0.000 | 1.000 | - | 5 (869–1012) | 0 | 0 | 0.001 (0.002) | 842–870 | 0.241–0.266 | 66–83 (65–82) |
| 10 | 0.5 | 0 / 5 | fail | 0 | 0 | 0.000–0.006 | 1.000 | 0.000 | 5 (731–903) | 0 | 0 | 0.014 (0.043) to 0.015 (0.047) | 595–672 | 0.353–0.455 | 121–179 (120–176) |
| 10 | 1 | 0 / 5 | fail | 0 | 0 | 0.000–0.001 | 1.000 | 0.000 | 5 (787–884) | 0 | 0 | 0.007 (0.021) to 0.007 (0.023) | 722–778 | 0.287–0.354 | 95–162 (94–160) |
| 10 | 4 | 0 / 5 | fail | 0 | 0 | 0.000 | 1.000 | - | 5 (849–937) | 0 | 0 | 0.002 (0.005) to 0.002 (0.006) | 821–857 | 0.257–0.279 | 91–117 (90–115) |
| 30 | 0.5 | 0 / 5 | fail | 0 | 0 | 0.000–0.037 | 1.000 | 0.000 | 5 (694–808) | 0 | 0 | 0.036 (0.113) to 0.037 (0.117) | 255–433 | 0.471–0.546 | 1444–1737 (444–734) |
| 30 | 1 | 0 / 5 | fail | 0 | 0 | 0.000–0.000 | 1.000 | 0.000 | 5 (753–929) | 0 | 0 | 0.017 (0.055) to 0.018 (0.057) | 467–645 | 0.373–0.440 | 370–792 (364–780) |
| 30 | 4 | 0 / 5 | fail | 0 | 0 | 0.000–0.000 | 1.000 | 0.000 | 5 (844–911) | 0 | 0 | 0.004 (0.013) to 0.004 (0.014) | 775–838 | 0.265–0.321 | 291–615 (285–604) |

**K = 10**

| k | L | seeds passing | verdict | guild holds | fewest members | guild share | mixotroph share | living share | producers persist (at end) | founders alive at 20 | founder line alive at end | income 5–20 | standing litter | inflow | runtime s (CPU s) |
|---:|---:|---:|---|---:|---|---|---|---|---|---:|---|---|---|---|---|
| 0 | 0.5 | 0 / 5 | fail | 0 | 0 | 0.000 | 1.000 | - | 5 (842–954) | 0 | 0 | 0.001 (0.002) to 0.001 (0.003) | 832–863 | 0.254–0.284 | 48–54 (47–53) |
| 0 | 1 | 0 / 5 | fail | 0 | 0 | 0.000 | 1.000 | - | 5 (746–961) | 0 | 0 | 0.000 (0.001) | 840–882 | 0.231–0.260 | 45–52 (44–51) |
| 0 | 4 | 0 / 5 | fail | 0 | 0 | 0.000 | 1.000 | - | 5 (805–897) | 0 | 0 | 0.000 (0.000) | 858–870 | 0.217–0.244 | 49–55 (49–55) |
| 3 | 0.5 | 0 / 5 | fail | 0 | 0 | 0.000–0.008 | 1.000 | 0.000 | 5 (787–1002) | 0 | 0 | 0.002 (0.005) to 0.002 (0.006) | 795–848 | 0.279–0.302 | 63–72 (62–72) |
| 3 | 1 | 0 / 5 | fail | 0 | 0 | 0.000–0.000 | 1.000 | 0.000 | 5 (857–930) | 0 | 0 | 0.001 (0.002) to 0.001 (0.003) | 829–860 | 0.257–0.264 | 66–76 (65–75) |
| 3 | 4 | 0 / 5 | fail | 0 | 0 | 0.000 | 1.000 | - | 5 (820–952) | 0 | 0 | 0.000 (0.001) | 851–888 | 0.242–0.261 | 53–68 (52–67) |
| 10 | 0.5 | 0 / 5 | fail | 0 | 0 | 0.000–0.000 | 1.000 | 0.000 | 5 (804–936) | 0 | 0 | 0.005 (0.016) to 0.006 (0.018) | 773–805 | 0.287–0.332 | 110–139 (108–138) |
| 10 | 1 | 0 / 5 | fail | 0 | 0 | 0.000–0.002 | 1.000 | 0.000 | 5 (792–1003) | 0 | 0 | 0.003 (0.008) to 0.003 (0.009) | 805–839 | 0.261–0.293 | 91–168 (90–167) |
| 10 | 4 | 0 / 5 | fail | 0 | 0 | 0.000 | 1.000 | - | 5 (861–988) | 0 | 0 | 0.001 (0.002) | 854–883 | 0.236–0.261 | 83–114 (82–113) |
| 30 | 0.5 | 0 / 5 | fail | 0 | 0 | 0.000–0.016 | 1.000 | 0.000 | 5 (825–857) | 0 | 0 | 0.015 (0.048) to 0.016 (0.050) | 549–646 | 0.382–0.426 | 423–539 (415–531) |
| 30 | 1 | 0 / 5 | fail | 0 | 0 | 0.000–0.008 | 1.000 | 0.000 | 5 (825–899) | 0 | 0 | 0.007 (0.023) to 0.008 (0.024) | 723–814 | 0.296–0.342 | 293–383 (284–371) |
| 30 | 4 | 0 / 5 | fail | 0 | 0 | 0.000–0.000 | 1.000 | 0.000 | 5 (847–918) | 0 | 0 | 0.002 (0.006) | 839–867 | 0.260–0.271 | 222–344 (220–339) |

### The verdict, by the fixed rule

**0 of 36 points pass.** No seed of any point passes, so no point reaches 3 of 5.

| k | points | pass | prediction | as predicted |
|---:|---:|---:|---|---|
| 0 | 9 | 0 | fail | yes |
| 3 | 9 | 0 | none | - |
| 10 | 9 | 0 | pass | **no** |
| 30 | 9 | 0 | pass | **no** |

By K, 0 of 12 points pass at each of K = 1, 3 and 10. By L, 0 of 12 pass at each of L = 0.5, 1 and 4.

- **The guild.** It holds in none of the 180 runs. Its fewest members are 0 in 179 runs and 1 in one.
- **Competition.** No point passes, so the prediction that specialists take more than half the carcass drain where a point passes has no point to test. Across all 180 runs, drainers below autotrophy 0.1 drained 0.0 in the control arm's second half: the mixotroph share is 1.000 in every run. The guild's share is at most 0.102 (k = 30, L = 1, K = 1), and that drain is by guild members with autotrophy of at least 0.1.
- **The check.** The guild's living drain is 0.0 in every run. Its living share is 0.000 in the 61 runs where it drained at all, and undefined in the other 119.
- **Producers.** They persist in all 180 runs, 688–1,033 at the control's end.
- **Founders.** None is alive at tick 20 in any run. The last founder dies at tick 14, 15 or 16 in every run. Income on ticks 5–20 rises with reach: at L = 1 and K = 1 it is 0.002–0.003 at k = 0, 0.004–0.005 at k = 3, 0.013 at k = 10 and 0.029–0.030 at k = 30. It falls with longer half-lives and larger K. The best, 0.057–0.059 at k = 30, L = 0.5, K = 1, is about 30 % of a decomposer founder's size-scaled upkeep at founding, 0.199. The founder line is alive at the end in 7 runs, all at k = 30 and K = 1 (L = 0.5 in all five seeds, 18–84 agents; L = 1 in seeds 3 and 5, 39 and 45).
- **Litter.** Standing litter per cell is 144–889 and inflow 0.217–0.546 a cell-tick across the sweep. Both move with reach and half-life: at k = 30 and L = 0.5, standing litter is 144–646 and inflow 0.382–0.546; at k = 0 and L = 4, 838–889 and 0.217–0.263.

### What follows, by the rule

Each branch the issue named, read against the sweep:
- **"Many" points pass, and specialists take most of the drain: does not fire.** No point passes, so the four mechanisms together do not carry mode 1's detrital loop by this reading, and no off-by-default rule is recommended as a default on this evidence.
- **"Many" points fail with producers intact: fires.** All 18 points at k = 10 and 30 fail, with producers persisting in every run. **Lean-time reserve is grilled next.** #807's founder trace already points at it: the starting reserve goes into growth and reproduction within about 5 ticks, and the retention buffer shrinks with upkeep. The sweep agrees in direction: every founder is dead by tick 16 at every point, on income below a third of upkeep, before any competition with the mixotrophs.
- **Mixotrophs still take most of the carcass drain: fires.** They take all of it in every run. **The specialist's advantage on litter is grilled next:** world rules, flow 7, with the matched-enzyme kernel held in reserve.
- **Producers collapse: does not fire.** They persist in all 180 runs, so no mechanism's off state needs naming.

These are recorded, not acted on. No grill has run, no issue has been filed, and no default changed.

### Sweep observations, not verdicts

- **The founders die on the same schedule at every point.** Reach multiplies their income more than tenfold from k = 0 to k = 30, but the last founder dies at tick 14–16 everywhere. The time to death barely moves while income does, which fits a reserve that is gone before income matters. That is the lean-time reserve question, not a diagnosis of it.
- **`S_p` moves across the sweep.** The control's per-seed median established structure reads 5.80–8.49 across the 180 runs (median 6.90). Step 1 settled `S_p` at k = 0, L = 1 and K = 1 only, as it said it would. `b` was held at 0.05651 throughout.
- **k = 30 costs most.** Its runs take 220–780 s of CPU, against 38–58 s at k = 0. The worst cells are k = 30 at short half-lives, where the litter is drawn down and the founder line survives. That is the opposite of #796, where reach collapsed the stand and made runs cheap.

## The code change

`reference_mode --established-producer-structure S_p` (#814) sets `S_p` for `--size-scaled-base-rate`, which it needs. It must be finite and above 0. The library function is `set_size_scaled_base_rate_at(world, s_p)`. `set_size_scaled_base_rate` calls it at `ESTABLISHED_PRODUCER_STRUCTURE`, 21.6, which keeps #804's provenance. An option fits better than a new constant: the settled value depends on the rules and the point it is read at, and #807's runs stay reproducible. Without the option every run is unchanged. At seed 3 over 200 ticks the plain run, `--body-reach-pieces 10`, `--size-scaled-base-rate`, and `--size-scaled-base-rate --established-producer-structure 21.6` all write reports byte-identical to main's.

## Runtime

Release build, five at a time on 8 cores: a paired run with all four rules on at k = 0, L = 1 and K = 1 took 46–61 s of wall-clock and 45–60 s of CPU. Each iteration's five runs took about 60 s. That is about twice #807's 30–33 s, with two to three times the producers. The supplementary iteration's runs took 934–939 s of wall-clock but 44–49 s of CPU. The laptop idle-slept on battery for 917 s during them (`pmset -g log`), the trap in *Running long sweeps*.

The sweep's 180 paired runs, release build, five at a time on 8 cores, wall-clock (CPU):
- k = 0: 38–59 s (38–58 s).
- k = 3: 44–85 s (44–84 s).
- k = 10: 83–222 s (82–220 s).
- k = 30: 222–1,737 s (220–780 s).

The costliest cells are k = 30 at L = 0.5 and 1 with K = 1 or 3, at 364–780 s of CPU a run. In CPU it took about 9.1 hours (2,245, 3,137, 5,866 and 21,431 s by k). In wall-clock it took about 2 h 25 min, from 10:42 to 13:07, including about 1,000 s of idle sleep on battery in one batch (k = 30, L = 0.5, K = 3: 1,444–1,737 s of wall-clock, 444–734 s of CPU). The pilot timed one seed at k = 0, L = 4, K = 1 (38 s) and at k = 3, L = 4, K = 1 (44 s), and missed the k = 30 cost by more than ten times. Size by the worst cells, as *Running long sweeps* says.

## Reproducing

```sh
cargo build --release -p explorers-reference-modes --bin reference_mode
# s = 1..5, five at a time; S_p = 7.577, 7.001, 7.402 for iterations 2-4 (omit the flag for iteration 1):
target/release/reference_mode --seed $s --clear-at 1500 --ticks 1500 --size-scaled-base-rate \
  --established-producer-structure $S_p --living-resistance \
  --litter-half-life 1 --enzyme-half-saturation 1 --out target/814/step1/itN/s$s.json
```

Each `.md` summary gives `b`, `S_p`, the kinetics, the guild, the founders, the litter and the established producers' structure. The JSON's `report.control.litter.samples[*].established_structures` holds the structures pooled for `S_p`.

The sweep, one run per (k, L, K, seed), five at a time, for k in 0 3 10 30, L in 0.5 1 4, K in 1 3 10 and s in 1..5:

```sh
target/release/reference_mode --seed $s --clear-at 1500 --ticks 1500 --size-scaled-base-rate \
  --established-producer-structure 6.937 --living-resistance --litter-half-life $L \
  --enzyme-half-saturation $K --body-reach-pieces $k --out target/814-sweep/runs/k${k}_L${L}_K${K}_s$s/report.json
```

At k = 0 the option leaves the coefficient at its default of 0, and the artifact records no `body_reach`.

## Appendix: the sweep, per point and seed

The control arm, columns as in *Step 2*. "Line alive at end" is the readout's founder line. "Last founder death" is the tick of the last founder's death. `b`, `k_d`, `K` (absolute) and the body-reach coefficient are as each run recorded them.

| k | L | K | seed | guild holds (fewest) | guild share | mixotroph share | living share | producers persist (at end) | pass | founders alive 20 / 50 / 100 | line alive at end | income 5–20 | last founder death | standing litter | inflow | b | k_d | K abs | reach coef | runtime s (CPU) |
|---:|---:|---:|---:|---|---:|---:|---:|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 | 0.5 | 1 | 1 | no (0) | 0.004 | 1.000 | 0.000 | yes (877) | no | 0 / 0 / 0 | 0 | 0.004 (0.014) | 15 | 765 | 0.300 | 0.0565 | 0.006054 | 1.073 | 0.000 | 47 (46) |
| 0 | 0.5 | 1 | 2 | no (0) | 0.014 | 1.000 | 0.000 | yes (815) | no | 0 / 0 / 0 | 0 | 0.004 (0.013) | 15 | 777 | 0.317 | 0.0565 | 0.006054 | 1.073 | 0.000 | 46 (46) |
| 0 | 0.5 | 1 | 3 | no (0) | 0.000 | 1.000 | - | yes (827) | no | 0 / 0 / 0 | 0 | 0.005 (0.015) | 15 | 763 | 0.331 | 0.0565 | 0.006054 | 1.073 | 0.000 | 46 (45) |
| 0 | 0.5 | 1 | 4 | no (0) | 0.000 | 1.000 | - | yes (855) | no | 0 / 0 / 0 | 0 | 0.005 (0.016) | 15 | 732 | 0.305 | 0.0565 | 0.006054 | 1.073 | 0.000 | 46 (46) |
| 0 | 0.5 | 1 | 5 | no (0) | 0.009 | 1.000 | 0.000 | yes (888) | no | 0 / 0 / 0 | 0 | 0.004 (0.013) | 15 | 777 | 0.333 | 0.0565 | 0.006054 | 1.073 | 0.000 | 50 (49) |
| 0 | 0.5 | 3 | 1 | no (0) | 0.000 | 1.000 | - | yes (933) | no | 0 / 0 / 0 | 0 | 0.002 (0.007) | 15 | 799 | 0.289 | 0.0565 | 0.006054 | 3.220 | 0.000 | 47 (46) |
| 0 | 0.5 | 3 | 2 | no (0) | 0.009 | 1.000 | 0.000 | yes (927) | no | 0 / 0 / 0 | 0 | 0.002 (0.007) | 15 | 825 | 0.291 | 0.0565 | 0.006054 | 3.220 | 0.000 | 49 (48) |
| 0 | 0.5 | 3 | 3 | no (0) | 0.000 | 1.000 | - | yes (909) | no | 0 / 0 / 0 | 0 | 0.002 (0.008) | 15 | 827 | 0.279 | 0.0565 | 0.006054 | 3.220 | 0.000 | 48 (48) |
| 0 | 0.5 | 3 | 4 | no (0) | 0.002 | 1.000 | 0.000 | yes (889) | no | 0 / 0 / 0 | 0 | 0.003 (0.008) | 15 | 800 | 0.307 | 0.0565 | 0.006054 | 3.220 | 0.000 | 50 (49) |
| 0 | 0.5 | 3 | 5 | no (0) | 0.000 | 1.000 | - | yes (841) | no | 0 / 0 / 0 | 0 | 0.002 (0.007) | 15 | 836 | 0.275 | 0.0565 | 0.006054 | 3.220 | 0.000 | 46 (45) |
| 0 | 0.5 | 10 | 1 | no (0) | 0.000 | 1.000 | - | yes (842) | no | 0 / 0 / 0 | 0 | 0.001 (0.003) | 15 | 835 | 0.284 | 0.0565 | 0.006054 | 10.734 | 0.000 | 50 (49) |
| 0 | 0.5 | 10 | 2 | no (0) | 0.000 | 1.000 | - | yes (854) | no | 0 / 0 / 0 | 0 | 0.001 (0.002) | 15 | 857 | 0.254 | 0.0565 | 0.006054 | 10.734 | 0.000 | 48 (47) |
| 0 | 0.5 | 10 | 3 | no (0) | 0.000 | 1.000 | - | yes (954) | no | 0 / 0 / 0 | 0 | 0.001 (0.003) | 15 | 836 | 0.265 | 0.0565 | 0.006054 | 10.734 | 0.000 | 53 (52) |
| 0 | 0.5 | 10 | 4 | no (0) | 0.000 | 1.000 | - | yes (930) | no | 0 / 0 / 0 | 0 | 0.001 (0.003) | 15 | 863 | 0.260 | 0.0565 | 0.006054 | 10.734 | 0.000 | 54 (53) |
| 0 | 0.5 | 10 | 5 | no (0) | 0.000 | 1.000 | - | yes (943) | no | 0 / 0 / 0 | 0 | 0.001 (0.002) | 15 | 832 | 0.275 | 0.0565 | 0.006054 | 10.734 | 0.000 | 53 (52) |
| 0 | 1 | 1 | 1 | no (0) | 0.001 | 1.000 | 0.000 | yes (939) | no | 0 / 0 / 0 | 0 | 0.002 (0.007) | 15 | 777 | 0.285 | 0.0565 | 0.003027 | 1.073 | 0.000 | 56 (55) |
| 0 | 1 | 1 | 2 | no (0) | 0.000 | 1.000 | - | yes (821) | no | 0 / 0 / 0 | 0 | 0.002 (0.007) | 15 | 784 | 0.276 | 0.0565 | 0.003027 | 1.073 | 0.000 | 51 (50) |
| 0 | 1 | 1 | 3 | no (0) | 0.000 | 1.000 | - | yes (792) | no | 0 / 0 / 0 | 0 | 0.002 (0.008) | 15 | 804 | 0.269 | 0.0565 | 0.003027 | 1.073 | 0.000 | 51 (50) |
| 0 | 1 | 1 | 4 | no (0) | 0.000 | 1.000 | - | yes (798) | no | 0 / 0 / 0 | 0 | 0.003 (0.008) | 15 | 795 | 0.287 | 0.0565 | 0.003027 | 1.073 | 0.000 | 51 (50) |
| 0 | 1 | 1 | 5 | no (0) | 0.000 | 1.000 | - | yes (956) | no | 0 / 0 / 0 | 0 | 0.002 (0.006) | 15 | 792 | 0.295 | 0.0565 | 0.003027 | 1.073 | 0.000 | 56 (55) |
| 0 | 1 | 3 | 1 | no (0) | 0.000 | 1.000 | - | yes (895) | no | 0 / 0 / 0 | 0 | 0.001 (0.004) | 15 | 844 | 0.270 | 0.0565 | 0.003027 | 3.220 | 0.000 | 51 (50) |
| 0 | 1 | 3 | 2 | no (0) | 0.000 | 1.000 | - | yes (862) | no | 0 / 0 / 0 | 0 | 0.001 (0.003) | 15 | 822 | 0.260 | 0.0565 | 0.003027 | 3.220 | 0.000 | 52 (52) |
| 0 | 1 | 3 | 3 | no (0) | 0.000 | 1.000 | 0.000 | yes (854) | no | 0 / 0 / 0 | 0 | 0.001 (0.004) | 15 | 829 | 0.253 | 0.0565 | 0.003027 | 3.220 | 0.000 | 57 (57) |
| 0 | 1 | 3 | 4 | no (0) | 0.000 | 1.000 | 0.000 | yes (938) | no | 0 / 0 / 0 | 0 | 0.001 (0.004) | 15 | 837 | 0.261 | 0.0565 | 0.003027 | 3.220 | 0.000 | 55 (54) |
| 0 | 1 | 3 | 5 | no (0) | 0.000 | 1.000 | - | yes (895) | no | 0 / 0 / 0 | 0 | 0.001 (0.003) | 15 | 837 | 0.263 | 0.0565 | 0.003027 | 3.220 | 0.000 | 52 (51) |
| 0 | 1 | 10 | 1 | no (0) | 0.000 | 1.000 | - | yes (816) | no | 0 / 0 / 0 | 0 | 0.000 (0.001) | 15 | 868 | 0.260 | 0.0565 | 0.003027 | 10.734 | 0.000 | 45 (44) |
| 0 | 1 | 10 | 2 | no (0) | 0.000 | 1.000 | - | yes (961) | no | 0 / 0 / 0 | 0 | 0.000 (0.001) | 15 | 882 | 0.244 | 0.0565 | 0.003027 | 10.734 | 0.000 | 52 (51) |
| 0 | 1 | 10 | 3 | no (0) | 0.000 | 1.000 | - | yes (746) | no | 0 / 0 / 0 | 0 | 0.000 (0.001) | 15 | 857 | 0.231 | 0.0565 | 0.003027 | 10.734 | 0.000 | 49 (48) |
| 0 | 1 | 10 | 4 | no (0) | 0.000 | 1.000 | - | yes (885) | no | 0 / 0 / 0 | 0 | 0.000 (0.001) | 15 | 840 | 0.236 | 0.0565 | 0.003027 | 10.734 | 0.000 | 48 (48) |
| 0 | 1 | 10 | 5 | no (0) | 0.000 | 1.000 | - | yes (837) | no | 0 / 0 / 0 | 0 | 0.000 (0.001) | 15 | 849 | 0.256 | 0.0565 | 0.003027 | 10.734 | 0.000 | 48 (48) |
| 0 | 4 | 1 | 1 | no (0) | 0.000 | 1.000 | - | yes (910) | no | 0 / 0 / 0 | 0 | 0.001 (0.002) | 15 | 838 | 0.260 | 0.0565 | 0.000757 | 1.073 | 0.000 | 38 (38) |
| 0 | 4 | 1 | 2 | no (0) | 0.000 | 1.000 | - | yes (812) | no | 0 / 0 / 0 | 0 | 0.001 (0.002) | 15 | 848 | 0.263 | 0.0565 | 0.000757 | 1.073 | 0.000 | 51 (51) |
| 0 | 4 | 1 | 3 | no (0) | 0.000 | 1.000 | - | yes (928) | no | 0 / 0 / 0 | 0 | 0.001 (0.002) | 15 | 876 | 0.256 | 0.0565 | 0.000757 | 1.073 | 0.000 | 50 (49) |
| 0 | 4 | 1 | 4 | no (0) | 0.000 | 1.000 | 0.000 | yes (914) | no | 0 / 0 / 0 | 0 | 0.001 (0.002) | 15 | 881 | 0.260 | 0.0565 | 0.000757 | 1.073 | 0.000 | 53 (52) |
| 0 | 4 | 1 | 5 | no (0) | 0.000 | 1.000 | - | yes (973) | no | 0 / 0 / 0 | 0 | 0.001 (0.002) | 15 | 859 | 0.250 | 0.0565 | 0.000757 | 1.073 | 0.000 | 53 (52) |
| 0 | 4 | 3 | 1 | no (0) | 0.000 | 1.000 | - | yes (893) | no | 0 / 0 / 0 | 0 | 0.000 (0.001) | 15 | 872 | 0.257 | 0.0565 | 0.000757 | 3.220 | 0.000 | 53 (53) |
| 0 | 4 | 3 | 2 | no (0) | 0.000 | 1.000 | - | yes (825) | no | 0 / 0 / 0 | 0 | 0.000 (0.001) | 15 | 881 | 0.236 | 0.0565 | 0.000757 | 3.220 | 0.000 | 49 (48) |
| 0 | 4 | 3 | 3 | no (0) | 0.000 | 1.000 | - | yes (942) | no | 0 / 0 / 0 | 0 | 0.000 (0.001) | 15 | 869 | 0.257 | 0.0565 | 0.000757 | 3.220 | 0.000 | 56 (55) |
| 0 | 4 | 3 | 4 | no (0) | 0.000 | 1.000 | - | yes (903) | no | 0 / 0 / 0 | 0 | 0.000 (0.001) | 15 | 889 | 0.248 | 0.0565 | 0.000757 | 3.220 | 0.000 | 51 (50) |
| 0 | 4 | 3 | 5 | no (0) | 0.000 | 1.000 | - | yes (956) | no | 0 / 0 / 0 | 0 | 0.000 (0.001) | 15 | 856 | 0.256 | 0.0565 | 0.000757 | 3.220 | 0.000 | 59 (58) |
| 0 | 4 | 10 | 1 | no (0) | 0.000 | 1.000 | - | yes (862) | no | 0 / 0 / 0 | 0 | 0.000 (0.000) | 15 | 858 | 0.232 | 0.0565 | 0.000757 | 10.734 | 0.000 | 55 (55) |
| 0 | 4 | 10 | 2 | no (0) | 0.000 | 1.000 | - | yes (897) | no | 0 / 0 / 0 | 0 | 0.000 (0.000) | 15 | 862 | 0.244 | 0.0565 | 0.000757 | 10.734 | 0.000 | 52 (52) |
| 0 | 4 | 10 | 3 | no (0) | 0.000 | 1.000 | - | yes (888) | no | 0 / 0 / 0 | 0 | 0.000 (0.000) | 15 | 866 | 0.244 | 0.0565 | 0.000757 | 10.734 | 0.000 | 50 (49) |
| 0 | 4 | 10 | 4 | no (0) | 0.000 | 1.000 | - | yes (888) | no | 0 / 0 / 0 | 0 | 0.000 (0.000) | 15 | 863 | 0.217 | 0.0565 | 0.000757 | 10.734 | 0.000 | 49 (49) |
| 0 | 4 | 10 | 5 | no (0) | 0.000 | 1.000 | - | yes (805) | no | 0 / 0 / 0 | 0 | 0.000 (0.000) | 15 | 870 | 0.243 | 0.0565 | 0.000757 | 10.734 | 0.000 | 51 (51) |
| 3 | 0.5 | 1 | 1 | no (0) | 0.000 | 1.000 | 0.000 | yes (714) | no | 0 / 0 / 0 | 0 | 0.008 (0.026) | 15 | 682 | 0.357 | 0.0565 | 0.006054 | 1.073 | 0.589 | 72 (71) |
| 3 | 0.5 | 1 | 2 | no (0) | 0.000 | 1.000 | 0.000 | yes (870) | no | 0 / 0 / 0 | 0 | 0.008 (0.026) | 15 | 673 | 0.411 | 0.0565 | 0.006054 | 1.073 | 0.589 | 85 (84) |
| 3 | 0.5 | 1 | 3 | no (0) | 0.000 | 1.000 | - | yes (841) | no | 0 / 0 / 0 | 0 | 0.009 (0.030) | 15 | 656 | 0.371 | 0.0565 | 0.006054 | 1.073 | 0.589 | 82 (81) |
| 3 | 0.5 | 1 | 4 | no (0) | 0.002 | 1.000 | 0.000 | yes (855) | no | 0 / 0 / 0 | 0 | 0.009 (0.029) | 15 | 678 | 0.399 | 0.0565 | 0.006054 | 1.073 | 0.589 | 73 (72) |
| 3 | 0.5 | 1 | 5 | no (0) | 0.000 | 1.000 | - | yes (821) | no | 0 / 0 / 0 | 0 | 0.008 (0.026) | 15 | 667 | 0.389 | 0.0565 | 0.006054 | 1.073 | 0.589 | 71 (70) |
| 3 | 0.5 | 3 | 1 | no (0) | 0.000 | 1.000 | - | yes (760) | no | 0 / 0 / 0 | 0 | 0.004 (0.014) | 15 | 734 | 0.330 | 0.0565 | 0.006054 | 3.220 | 0.589 | 71 (70) |
| 3 | 0.5 | 3 | 2 | no (0) | 0.000 | 1.000 | - | yes (852) | no | 0 / 0 / 0 | 0 | 0.004 (0.013) | 15 | 761 | 0.324 | 0.0565 | 0.006054 | 3.220 | 0.589 | 75 (75) |
| 3 | 0.5 | 3 | 3 | no (0) | 0.000 | 1.000 | 0.000 | yes (921) | no | 0 / 0 / 0 | 0 | 0.005 (0.016) | 15 | 763 | 0.338 | 0.0565 | 0.006054 | 3.220 | 0.589 | 80 (79) |
| 3 | 0.5 | 3 | 4 | no (0) | 0.000 | 1.000 | - | yes (815) | no | 0 / 0 / 0 | 0 | 0.005 (0.015) | 15 | 709 | 0.319 | 0.0565 | 0.006054 | 3.220 | 0.589 | 80 (79) |
| 3 | 0.5 | 3 | 5 | no (0) | 0.000 | 1.000 | - | yes (861) | no | 0 / 0 / 0 | 0 | 0.004 (0.014) | 15 | 769 | 0.316 | 0.0565 | 0.006054 | 3.220 | 0.589 | 72 (72) |
| 3 | 0.5 | 10 | 1 | no (0) | 0.000 | 1.000 | - | yes (983) | no | 0 / 0 / 0 | 0 | 0.002 (0.005) | 15 | 810 | 0.279 | 0.0565 | 0.006054 | 10.734 | 0.589 | 72 (72) |
| 3 | 0.5 | 10 | 2 | no (0) | 0.008 | 1.000 | 0.000 | yes (1002) | no | 0 / 0 / 0 | 0 | 0.002 (0.005) | 15 | 832 | 0.287 | 0.0565 | 0.006054 | 10.734 | 0.589 | 64 (64) |
| 3 | 0.5 | 10 | 3 | no (0) | 0.000 | 1.000 | - | yes (841) | no | 0 / 0 / 0 | 0 | 0.002 (0.006) | 15 | 806 | 0.284 | 0.0565 | 0.006054 | 10.734 | 0.589 | 67 (66) |
| 3 | 0.5 | 10 | 4 | no (0) | 0.000 | 1.000 | - | yes (787) | no | 0 / 0 / 0 | 0 | 0.002 (0.006) | 15 | 848 | 0.290 | 0.0565 | 0.006054 | 10.734 | 0.589 | 63 (62) |
| 3 | 0.5 | 10 | 5 | no (0) | 0.000 | 1.000 | - | yes (938) | no | 0 / 0 / 0 | 0 | 0.002 (0.005) | 15 | 795 | 0.302 | 0.0565 | 0.006054 | 10.734 | 0.589 | 71 (71) |
| 3 | 1 | 1 | 1 | no (0) | 0.004 | 1.000 | 0.000 | yes (831) | no | 0 / 0 / 0 | 0 | 0.004 (0.013) | 15 | 749 | 0.314 | 0.0565 | 0.003027 | 1.073 | 0.589 | 69 (68) |
| 3 | 1 | 1 | 2 | no (0) | 0.000 | 1.000 | - | yes (931) | no | 0 / 0 / 0 | 0 | 0.004 (0.013) | 15 | 764 | 0.338 | 0.0565 | 0.003027 | 1.073 | 0.589 | 76 (75) |
| 3 | 1 | 1 | 3 | no (0) | 0.004 | 1.000 | 0.000 | yes (898) | no | 0 / 0 / 0 | 0 | 0.005 (0.015) | 15 | 741 | 0.315 | 0.0565 | 0.003027 | 1.073 | 0.589 | 76 (76) |
| 3 | 1 | 1 | 4 | no (0) | 0.000 | 1.000 | 0.000 | yes (822) | no | 0 / 0 / 0 | 0 | 0.005 (0.015) | 15 | 751 | 0.313 | 0.0565 | 0.003027 | 1.073 | 0.589 | 73 (72) |
| 3 | 1 | 1 | 5 | no (0) | 0.000 | 1.000 | - | yes (756) | no | 0 / 0 / 0 | 0 | 0.004 (0.013) | 15 | 751 | 0.331 | 0.0565 | 0.003027 | 1.073 | 0.589 | 72 (72) |
| 3 | 1 | 3 | 1 | no (0) | 0.000 | 1.000 | - | yes (848) | no | 0 / 0 / 0 | 0 | 0.002 (0.007) | 15 | 812 | 0.288 | 0.0565 | 0.003027 | 3.220 | 0.589 | 61 (60) |
| 3 | 1 | 3 | 2 | no (0) | 0.000 | 1.000 | 0.000 | yes (948) | no | 0 / 0 / 0 | 0 | 0.002 (0.007) | 15 | 802 | 0.288 | 0.0565 | 0.003027 | 3.220 | 0.589 | 71 (71) |
| 3 | 1 | 3 | 3 | no (0) | 0.000 | 1.000 | - | yes (919) | no | 0 / 0 / 0 | 0 | 0.002 (0.008) | 15 | 801 | 0.282 | 0.0565 | 0.003027 | 3.220 | 0.589 | 80 (80) |
| 3 | 1 | 3 | 4 | no (0) | 0.013 | 1.000 | 0.000 | yes (883) | no | 0 / 0 / 0 | 0 | 0.002 (0.008) | 15 | 792 | 0.303 | 0.0565 | 0.003027 | 3.220 | 0.589 | 77 (76) |
| 3 | 1 | 3 | 5 | no (0) | 0.000 | 1.000 | - | yes (838) | no | 0 / 0 / 0 | 0 | 0.002 (0.007) | 15 | 841 | 0.309 | 0.0565 | 0.003027 | 3.220 | 0.589 | 69 (68) |
| 3 | 1 | 10 | 1 | no (0) | 0.000 | 1.000 | - | yes (930) | no | 0 / 0 / 0 | 0 | 0.001 (0.003) | 15 | 860 | 0.264 | 0.0565 | 0.003027 | 10.734 | 0.589 | 66 (66) |
| 3 | 1 | 10 | 2 | no (0) | 0.000 | 1.000 | 0.000 | yes (898) | no | 0 / 0 / 0 | 0 | 0.001 (0.002) | 15 | 842 | 0.263 | 0.0565 | 0.003027 | 10.734 | 0.589 | 74 (73) |
| 3 | 1 | 10 | 3 | no (0) | 0.000 | 1.000 | - | yes (924) | no | 0 / 0 / 0 | 0 | 0.001 (0.003) | 15 | 845 | 0.258 | 0.0565 | 0.003027 | 10.734 | 0.589 | 76 (75) |
| 3 | 1 | 10 | 4 | no (0) | 0.000 | 1.000 | - | yes (909) | no | 0 / 0 / 0 | 0 | 0.001 (0.003) | 15 | 858 | 0.257 | 0.0565 | 0.003027 | 10.734 | 0.589 | 66 (65) |
| 3 | 1 | 10 | 5 | no (0) | 0.000 | 1.000 | - | yes (857) | no | 0 / 0 / 0 | 0 | 0.001 (0.003) | 15 | 829 | 0.261 | 0.0565 | 0.003027 | 10.734 | 0.589 | 71 (70) |
| 3 | 4 | 1 | 1 | no (0) | 0.000 | 1.000 | - | yes (876) | no | 0 / 0 / 0 | 0 | 0.001 (0.003) | 15 | 835 | 0.274 | 0.0565 | 0.000757 | 1.073 | 0.589 | 44 (44) |
| 3 | 4 | 1 | 2 | no (0) | 0.000 | 1.000 | - | yes (862) | no | 0 / 0 / 0 | 0 | 0.001 (0.003) | 15 | 862 | 0.282 | 0.0565 | 0.000757 | 1.073 | 0.589 | 67 (66) |
| 3 | 4 | 1 | 3 | no (0) | 0.000 | 1.000 | - | yes (883) | no | 0 / 0 / 0 | 0 | 0.001 (0.004) | 15 | 848 | 0.274 | 0.0565 | 0.000757 | 1.073 | 0.589 | 77 (76) |
| 3 | 4 | 1 | 4 | no (0) | 0.000 | 1.000 | - | yes (850) | no | 0 / 0 / 0 | 0 | 0.001 (0.004) | 15 | 854 | 0.264 | 0.0565 | 0.000757 | 1.073 | 0.589 | 66 (65) |
| 3 | 4 | 1 | 5 | no (0) | 0.000 | 1.000 | - | yes (1033) | no | 0 / 0 / 0 | 0 | 0.001 (0.003) | 15 | 826 | 0.272 | 0.0565 | 0.000757 | 1.073 | 0.589 | 79 (78) |
| 3 | 4 | 3 | 1 | no (0) | 0.000 | 1.000 | - | yes (872) | no | 0 / 0 / 0 | 0 | 0.001 (0.002) | 15 | 842 | 0.241 | 0.0565 | 0.000757 | 3.220 | 0.589 | 66 (65) |
| 3 | 4 | 3 | 2 | no (0) | 0.000 | 1.000 | - | yes (1012) | no | 0 / 0 / 0 | 0 | 0.001 (0.002) | 15 | 853 | 0.244 | 0.0565 | 0.000757 | 3.220 | 0.589 | 74 (73) |
| 3 | 4 | 3 | 3 | no (0) | 0.000 | 1.000 | - | yes (883) | no | 0 / 0 / 0 | 0 | 0.001 (0.002) | 15 | 870 | 0.255 | 0.0565 | 0.000757 | 3.220 | 0.589 | 71 (70) |
| 3 | 4 | 3 | 4 | no (0) | 0.000 | 1.000 | - | yes (892) | no | 0 / 0 / 0 | 0 | 0.001 (0.002) | 15 | 853 | 0.266 | 0.0565 | 0.000757 | 3.220 | 0.589 | 72 (71) |
| 3 | 4 | 3 | 5 | no (0) | 0.000 | 1.000 | - | yes (869) | no | 0 / 0 / 0 | 0 | 0.001 (0.002) | 15 | 845 | 0.265 | 0.0565 | 0.000757 | 3.220 | 0.589 | 83 (82) |
| 3 | 4 | 10 | 1 | no (0) | 0.000 | 1.000 | - | yes (952) | no | 0 / 0 / 0 | 0 | 0.000 (0.001) | 15 | 874 | 0.259 | 0.0565 | 0.000757 | 10.734 | 0.589 | 68 (67) |
| 3 | 4 | 10 | 2 | no (0) | 0.000 | 1.000 | - | yes (820) | no | 0 / 0 / 0 | 0 | 0.000 (0.001) | 15 | 871 | 0.261 | 0.0565 | 0.000757 | 10.734 | 0.589 | 63 (63) |
| 3 | 4 | 10 | 3 | no (0) | 0.000 | 1.000 | - | yes (820) | no | 0 / 0 / 0 | 0 | 0.000 (0.001) | 15 | 886 | 0.242 | 0.0565 | 0.000757 | 10.734 | 0.589 | 55 (55) |
| 3 | 4 | 10 | 4 | no (0) | 0.000 | 1.000 | - | yes (887) | no | 0 / 0 / 0 | 0 | 0.000 (0.001) | 15 | 888 | 0.255 | 0.0565 | 0.000757 | 10.734 | 0.589 | 55 (54) |
| 3 | 4 | 10 | 5 | no (0) | 0.000 | 1.000 | - | yes (870) | no | 0 / 0 / 0 | 0 | 0.000 (0.001) | 15 | 851 | 0.244 | 0.0565 | 0.000757 | 10.734 | 0.589 | 53 (52) |
| 10 | 0.5 | 1 | 1 | no (0) | 0.004 | 1.000 | 0.000 | yes (765) | no | 0 / 0 / 0 | 0 | 0.027 (0.085) | 15 | 487 | 0.442 | 0.0565 | 0.006054 | 1.073 | 1.752 | 152 (151) |
| 10 | 0.5 | 1 | 2 | no (0) | 0.000 | 1.000 | - | yes (774) | no | 0 / 0 / 0 | 0 | 0.026 (0.082) | 15 | 417 | 0.497 | 0.0565 | 0.006054 | 1.073 | 1.752 | 147 (145) |
| 10 | 0.5 | 1 | 3 | no (0) | 0.001 | 1.000 | 0.000 | yes (804) | no | 0 / 0 / 0 | 0 | 0.027 (0.086) | 14 | 513 | 0.443 | 0.0565 | 0.006054 | 1.073 | 1.752 | 222 (220) |
| 10 | 0.5 | 1 | 4 | no (0) | 0.002 | 1.000 | 0.000 | yes (757) | no | 0 / 0 / 0 | 0 | 0.028 (0.088) | 15 | 441 | 0.433 | 0.0565 | 0.006054 | 1.073 | 1.752 | 169 (168) |
| 10 | 0.5 | 1 | 5 | no (0) | 0.001 | 1.000 | 0.000 | yes (806) | no | 0 / 0 / 0 | 0 | 0.026 (0.084) | 15 | 414 | 0.496 | 0.0565 | 0.006054 | 1.073 | 1.752 | 211 (208) |
| 10 | 0.5 | 3 | 1 | no (0) | 0.000 | 1.000 | - | yes (846) | no | 0 / 0 / 0 | 0 | 0.014 (0.045) | 15 | 672 | 0.353 | 0.0565 | 0.006054 | 3.220 | 1.752 | 121 (120) |
| 10 | 0.5 | 3 | 2 | no (0) | 0.000 | 1.000 | - | yes (903) | no | 0 / 0 / 0 | 0 | 0.014 (0.043) | 15 | 610 | 0.390 | 0.0565 | 0.006054 | 3.220 | 1.752 | 134 (132) |
| 10 | 0.5 | 3 | 3 | no (0) | 0.000 | 1.000 | - | yes (745) | no | 0 / 0 / 0 | 0 | 0.015 (0.046) | 15 | 607 | 0.455 | 0.0565 | 0.006054 | 3.220 | 1.752 | 150 (147) |
| 10 | 0.5 | 3 | 4 | no (0) | 0.000 | 1.000 | 0.000 | yes (759) | no | 0 / 0 / 0 | 0 | 0.015 (0.047) | 15 | 645 | 0.382 | 0.0565 | 0.006054 | 3.220 | 1.752 | 152 (150) |
| 10 | 0.5 | 3 | 5 | no (0) | 0.006 | 1.000 | 0.000 | yes (731) | no | 0 / 0 / 0 | 0 | 0.014 (0.044) | 15 | 595 | 0.407 | 0.0565 | 0.006054 | 3.220 | 1.752 | 179 (176) |
| 10 | 0.5 | 10 | 1 | no (0) | 0.000 | 1.000 | - | yes (936) | no | 0 / 0 / 0 | 0 | 0.006 (0.018) | 15 | 804 | 0.310 | 0.0565 | 0.006054 | 10.734 | 1.752 | 110 (108) |
| 10 | 0.5 | 10 | 2 | no (0) | 0.000 | 1.000 | - | yes (827) | no | 0 / 0 / 0 | 0 | 0.005 (0.016) | 15 | 805 | 0.319 | 0.0565 | 0.006054 | 10.734 | 1.752 | 127 (126) |
| 10 | 0.5 | 10 | 3 | no (0) | 0.000 | 1.000 | 0.000 | yes (863) | no | 0 / 0 / 0 | 0 | 0.006 (0.018) | 15 | 776 | 0.287 | 0.0565 | 0.006054 | 10.734 | 1.752 | 132 (131) |
| 10 | 0.5 | 10 | 4 | no (0) | 0.000 | 1.000 | - | yes (859) | no | 0 / 0 / 0 | 0 | 0.006 (0.018) | 15 | 790 | 0.291 | 0.0565 | 0.006054 | 10.734 | 1.752 | 114 (113) |
| 10 | 0.5 | 10 | 5 | no (0) | 0.000 | 1.000 | 0.000 | yes (804) | no | 0 / 0 / 0 | 0 | 0.005 (0.017) | 15 | 773 | 0.332 | 0.0565 | 0.006054 | 10.734 | 1.752 | 139 (138) |
| 10 | 1 | 1 | 1 | no (0) | 0.000 | 1.000 | - | yes (795) | no | 0 / 0 / 0 | 0 | 0.013 (0.042) | 15 | 639 | 0.415 | 0.0565 | 0.003027 | 1.073 | 1.752 | 151 (149) |
| 10 | 1 | 1 | 2 | no (0) | 0.000 | 1.000 | 0.000 | yes (954) | no | 0 / 0 / 0 | 0 | 0.013 (0.040) | 15 | 655 | 0.403 | 0.0565 | 0.003027 | 1.073 | 1.752 | 172 (171) |
| 10 | 1 | 1 | 3 | no (0) | 0.000 | 1.000 | - | yes (769) | no | 0 / 0 / 0 | 0 | 0.013 (0.043) | 15 | 644 | 0.386 | 0.0565 | 0.003027 | 1.073 | 1.752 | 113 (112) |
| 10 | 1 | 1 | 4 | no (0) | 0.000 | 1.000 | - | yes (688) | no | 0 / 0 / 0 | 0 | 0.013 (0.043) | 15 | 580 | 0.380 | 0.0565 | 0.003027 | 1.073 | 1.752 | 152 (150) |
| 10 | 1 | 1 | 5 | no (0) | 0.000 | 1.000 | - | yes (902) | no | 0 / 0 / 0 | 0 | 0.013 (0.041) | 15 | 621 | 0.390 | 0.0565 | 0.003027 | 1.073 | 1.752 | 163 (161) |
| 10 | 1 | 3 | 1 | no (0) | 0.001 | 1.000 | 0.000 | yes (884) | no | 0 / 0 / 0 | 0 | 0.007 (0.023) | 15 | 722 | 0.340 | 0.0565 | 0.003027 | 3.220 | 1.752 | 150 (148) |
| 10 | 1 | 3 | 2 | no (0) | 0.000 | 1.000 | - | yes (811) | no | 0 / 0 / 0 | 0 | 0.007 (0.021) | 15 | 778 | 0.317 | 0.0565 | 0.003027 | 3.220 | 1.752 | 121 (119) |
| 10 | 1 | 3 | 3 | no (0) | 0.000 | 1.000 | 0.000 | yes (787) | no | 0 / 0 / 0 | 0 | 0.007 (0.023) | 15 | 756 | 0.354 | 0.0565 | 0.003027 | 3.220 | 1.752 | 162 (160) |
| 10 | 1 | 3 | 4 | no (0) | 0.000 | 1.000 | - | yes (884) | no | 0 / 0 / 0 | 0 | 0.007 (0.023) | 15 | 752 | 0.308 | 0.0565 | 0.003027 | 3.220 | 1.752 | 136 (134) |
| 10 | 1 | 3 | 5 | no (0) | 0.000 | 1.000 | - | yes (792) | no | 0 / 0 / 0 | 0 | 0.007 (0.022) | 15 | 771 | 0.287 | 0.0565 | 0.003027 | 3.220 | 1.752 | 95 (94) |
| 10 | 1 | 10 | 1 | no (0) | 0.000 | 1.000 | - | yes (792) | no | 0 / 0 / 0 | 0 | 0.003 (0.009) | 15 | 805 | 0.286 | 0.0565 | 0.003027 | 10.734 | 1.752 | 117 (116) |
| 10 | 1 | 10 | 2 | no (0) | 0.000 | 1.000 | - | yes (1003) | no | 0 / 0 / 0 | 0 | 0.003 (0.008) | 15 | 834 | 0.293 | 0.0565 | 0.003027 | 10.734 | 1.752 | 168 (167) |
| 10 | 1 | 10 | 3 | no (0) | 0.000 | 1.000 | - | yes (884) | no | 0 / 0 / 0 | 0 | 0.003 (0.009) | 15 | 827 | 0.279 | 0.0565 | 0.003027 | 10.734 | 1.752 | 164 (163) |
| 10 | 1 | 10 | 4 | no (0) | 0.002 | 1.000 | 0.000 | yes (927) | no | 0 / 0 / 0 | 0 | 0.003 (0.009) | 15 | 839 | 0.261 | 0.0565 | 0.003027 | 10.734 | 1.752 | 91 (90) |
| 10 | 1 | 10 | 5 | no (0) | 0.000 | 1.000 | - | yes (833) | no | 0 / 0 / 0 | 0 | 0.003 (0.008) | 15 | 825 | 0.291 | 0.0565 | 0.003027 | 10.734 | 1.752 | 113 (112) |
| 10 | 4 | 1 | 1 | no (0) | 0.000 | 1.000 | - | yes (887) | no | 0 / 0 / 0 | 0 | 0.003 (0.010) | 15 | 807 | 0.295 | 0.0565 | 0.000757 | 1.073 | 1.752 | 93 (92) |
| 10 | 4 | 1 | 2 | no (0) | 0.000 | 1.000 | - | yes (999) | no | 0 / 0 / 0 | 0 | 0.003 (0.010) | 15 | 824 | 0.290 | 0.0565 | 0.000757 | 1.073 | 1.752 | 100 (99) |
| 10 | 4 | 1 | 3 | no (0) | 0.000 | 1.000 | - | yes (941) | no | 0 / 0 / 0 | 0 | 0.003 (0.011) | 15 | 809 | 0.293 | 0.0565 | 0.000757 | 1.073 | 1.752 | 114 (112) |
| 10 | 4 | 1 | 4 | no (0) | 0.000 | 1.000 | 0.000 | yes (941) | no | 0 / 0 / 0 | 0 | 0.003 (0.011) | 15 | 817 | 0.313 | 0.0565 | 0.000757 | 1.073 | 1.752 | 127 (125) |
| 10 | 4 | 1 | 5 | no (0) | 0.000 | 1.000 | - | yes (848) | no | 0 / 0 / 0 | 0 | 0.003 (0.010) | 15 | 774 | 0.303 | 0.0565 | 0.000757 | 1.073 | 1.752 | 162 (160) |
| 10 | 4 | 3 | 1 | no (0) | 0.000 | 1.000 | - | yes (918) | no | 0 / 0 / 0 | 0 | 0.002 (0.006) | 15 | 839 | 0.262 | 0.0565 | 0.000757 | 3.220 | 1.752 | 104 (102) |
| 10 | 4 | 3 | 2 | no (0) | 0.000 | 1.000 | - | yes (849) | no | 0 / 0 / 0 | 0 | 0.002 (0.005) | 15 | 843 | 0.257 | 0.0565 | 0.000757 | 3.220 | 1.752 | 117 (115) |
| 10 | 4 | 3 | 3 | no (0) | 0.000 | 1.000 | - | yes (922) | no | 0 / 0 / 0 | 0 | 0.002 (0.006) | 15 | 857 | 0.274 | 0.0565 | 0.000757 | 3.220 | 1.752 | 104 (102) |
| 10 | 4 | 3 | 4 | no (0) | 0.000 | 1.000 | - | yes (874) | no | 0 / 0 / 0 | 0 | 0.002 (0.006) | 15 | 824 | 0.279 | 0.0565 | 0.000757 | 3.220 | 1.752 | 94 (93) |
| 10 | 4 | 3 | 5 | no (0) | 0.000 | 1.000 | - | yes (937) | no | 0 / 0 / 0 | 0 | 0.002 (0.005) | 15 | 821 | 0.278 | 0.0565 | 0.000757 | 3.220 | 1.752 | 91 (90) |
| 10 | 4 | 10 | 1 | no (0) | 0.000 | 1.000 | - | yes (861) | no | 0 / 0 / 0 | 0 | 0.001 (0.002) | 15 | 870 | 0.261 | 0.0565 | 0.000757 | 10.734 | 1.752 | 93 (93) |
| 10 | 4 | 10 | 2 | no (0) | 0.000 | 1.000 | - | yes (988) | no | 0 / 0 / 0 | 0 | 0.001 (0.002) | 15 | 874 | 0.244 | 0.0565 | 0.000757 | 10.734 | 1.752 | 109 (108) |
| 10 | 4 | 10 | 3 | no (0) | 0.000 | 1.000 | - | yes (888) | no | 0 / 0 / 0 | 0 | 0.001 (0.002) | 15 | 863 | 0.236 | 0.0565 | 0.000757 | 10.734 | 1.752 | 101 (100) |
| 10 | 4 | 10 | 4 | no (0) | 0.000 | 1.000 | - | yes (912) | no | 0 / 0 / 0 | 0 | 0.001 (0.002) | 15 | 883 | 0.246 | 0.0565 | 0.000757 | 10.734 | 1.752 | 83 (82) |
| 10 | 4 | 10 | 5 | no (0) | 0.000 | 1.000 | - | yes (868) | no | 0 / 0 / 0 | 0 | 0.001 (0.002) | 15 | 854 | 0.256 | 0.0565 | 0.000757 | 10.734 | 1.752 | 114 (113) |
| 30 | 0.5 | 1 | 1 | no (0) | 0.071 | 1.000 | 0.000 | yes (723) | no | 0 / 0 / 0 | 74 | 0.057 (0.181) | 15 | 144 | 0.468 | 0.0565 | 0.006054 | 1.073 | 3.634 | 711 (701) |
| 30 | 0.5 | 1 | 2 | no (0) | 0.000 | 1.000 | 0.000 | yes (789) | no | 0 / 0 / 0 | 44 | 0.059 (0.187) | 15 | 175 | 0.496 | 0.0565 | 0.006054 | 1.073 | 3.634 | 571 (561) |
| 30 | 0.5 | 1 | 3 | no (0) | 0.023 | 1.000 | 0.000 | yes (719) | no | 0 / 0 / 0 | 18 | 0.058 (0.185) | 15 | 214 | 0.484 | 0.0565 | 0.006054 | 1.073 | 3.634 | 551 (542) |
| 30 | 0.5 | 1 | 4 | no (0) | 0.023 | 1.000 | 0.000 | yes (803) | no | 0 / 0 / 0 | 84 | 0.058 (0.183) | 15 | 206 | 0.494 | 0.0565 | 0.006054 | 1.073 | 3.634 | 682 (674) |
| 30 | 0.5 | 1 | 5 | no (0) | 0.002 | 1.000 | 0.000 | yes (733) | no | 0 / 0 / 0 | 68 | 0.059 (0.188) | 16 | 173 | 0.518 | 0.0565 | 0.006054 | 1.073 | 3.634 | 774 (764) |
| 30 | 0.5 | 3 | 1 | no (0) | 0.037 | 1.000 | 0.000 | yes (783) | no | 0 / 0 / 0 | 0 | 0.036 (0.113) | 14 | 346 | 0.475 | 0.0565 | 0.006054 | 3.220 | 3.634 | 1737 (734) |
| 30 | 0.5 | 3 | 2 | no (0) | 0.001 | 1.000 | 0.000 | yes (791) | no | 0 / 0 / 0 | 0 | 0.036 (0.114) | 14 | 433 | 0.471 | 0.0565 | 0.006054 | 3.220 | 3.634 | 1444 (444) |
| 30 | 0.5 | 3 | 3 | no (0) | 0.000 | 1.000 | 0.000 | yes (808) | no | 0 / 0 / 0 | 0 | 0.037 (0.116) | 14 | 330 | 0.512 | 0.0565 | 0.006054 | 3.220 | 3.634 | 1538 (535) |
| 30 | 0.5 | 3 | 4 | no (0) | 0.031 | 1.000 | 0.000 | yes (715) | no | 0 / 0 / 0 | 0 | 0.036 (0.114) | 14 | 282 | 0.506 | 0.0565 | 0.006054 | 3.220 | 3.634 | 1674 (670) |
| 30 | 0.5 | 3 | 5 | no (0) | 0.003 | 1.000 | 0.000 | yes (694) | no | 0 / 0 / 0 | 0 | 0.037 (0.117) | 14 | 255 | 0.546 | 0.0565 | 0.006054 | 3.220 | 3.634 | 1668 (664) |
| 30 | 0.5 | 10 | 1 | no (0) | 0.001 | 1.000 | 0.000 | yes (836) | no | 0 / 0 / 0 | 0 | 0.015 (0.048) | 15 | 635 | 0.426 | 0.0565 | 0.006054 | 10.734 | 3.634 | 539 (531) |
| 30 | 0.5 | 10 | 2 | no (0) | 0.016 | 1.000 | 0.000 | yes (849) | no | 0 / 0 / 0 | 0 | 0.015 (0.048) | 15 | 644 | 0.396 | 0.0565 | 0.006054 | 10.734 | 3.634 | 423 (415) |
| 30 | 0.5 | 10 | 3 | no (0) | 0.000 | 1.000 | 0.000 | yes (836) | no | 0 / 0 / 0 | 0 | 0.016 (0.050) | 15 | 646 | 0.401 | 0.0565 | 0.006054 | 10.734 | 3.634 | 441 (435) |
| 30 | 0.5 | 10 | 4 | no (0) | 0.000 | 1.000 | - | yes (825) | no | 0 / 0 / 0 | 0 | 0.015 (0.049) | 15 | 622 | 0.382 | 0.0565 | 0.006054 | 10.734 | 3.634 | 432 (426) |
| 30 | 0.5 | 10 | 5 | no (0) | 0.002 | 1.000 | 0.000 | yes (857) | no | 0 / 0 / 0 | 0 | 0.015 (0.049) | 15 | 549 | 0.419 | 0.0565 | 0.006054 | 10.734 | 3.634 | 453 (447) |
| 30 | 1 | 1 | 1 | no (0) | 0.002 | 1.000 | 0.000 | yes (773) | no | 0 / 0 / 0 | 0 | 0.029 (0.091) | 14 | 380 | 0.451 | 0.0565 | 0.003027 | 1.073 | 3.634 | 477 (469) |
| 30 | 1 | 1 | 2 | no (0) | 0.019 | 1.000 | 0.000 | yes (842) | no | 0 / 0 / 0 | 0 | 0.029 (0.092) | 14 | 420 | 0.439 | 0.0565 | 0.003027 | 1.073 | 3.634 | 485 (477) |
| 30 | 1 | 1 | 3 | no (0) | 0.001 | 1.000 | 0.000 | yes (779) | no | 0 / 0 / 0 | 39 | 0.029 (0.093) | 14 | 361 | 0.472 | 0.0565 | 0.003027 | 1.073 | 3.634 | 787 (775) |
| 30 | 1 | 1 | 4 | no (0) | 0.025 | 1.000 | 0.000 | yes (788) | no | 0 / 0 / 0 | 0 | 0.029 (0.092) | 14 | 486 | 0.480 | 0.0565 | 0.003027 | 1.073 | 3.634 | 453 (446) |
| 30 | 1 | 1 | 5 | no (1) | 0.102 | 1.000 | 0.000 | yes (747) | no | 0 / 0 / 0 | 45 | 0.030 (0.095) | 14 | 349 | 0.477 | 0.0565 | 0.003027 | 1.073 | 3.634 | 567 (558) |
| 30 | 1 | 3 | 1 | no (0) | 0.000 | 1.000 | 0.000 | yes (883) | no | 0 / 0 / 0 | 0 | 0.017 (0.055) | 15 | 467 | 0.410 | 0.0565 | 0.003027 | 3.220 | 3.634 | 792 (780) |
| 30 | 1 | 3 | 2 | no (0) | 0.000 | 1.000 | - | yes (929) | no | 0 / 0 / 0 | 0 | 0.017 (0.055) | 15 | 626 | 0.393 | 0.0565 | 0.003027 | 3.220 | 3.634 | 514 (504) |
| 30 | 1 | 3 | 3 | no (0) | 0.000 | 1.000 | - | yes (766) | no | 0 / 0 / 0 | 0 | 0.018 (0.057) | 15 | 645 | 0.373 | 0.0565 | 0.003027 | 3.220 | 3.634 | 370 (364) |
| 30 | 1 | 3 | 4 | no (0) | 0.000 | 1.000 | 0.000 | yes (794) | no | 0 / 0 / 0 | 0 | 0.018 (0.056) | 15 | 489 | 0.440 | 0.0565 | 0.003027 | 3.220 | 3.634 | 549 (539) |
| 30 | 1 | 3 | 5 | no (0) | 0.000 | 1.000 | - | yes (753) | no | 0 / 0 / 0 | 0 | 0.018 (0.057) | 15 | 591 | 0.395 | 0.0565 | 0.003027 | 3.220 | 3.634 | 436 (430) |
| 30 | 1 | 10 | 1 | no (0) | 0.000 | 1.000 | - | yes (849) | no | 0 / 0 / 0 | 0 | 0.007 (0.023) | 15 | 723 | 0.320 | 0.0565 | 0.003027 | 10.734 | 3.634 | 350 (340) |
| 30 | 1 | 10 | 2 | no (0) | 0.008 | 1.000 | 0.000 | yes (843) | no | 0 / 0 / 0 | 0 | 0.007 (0.024) | 15 | 814 | 0.296 | 0.0565 | 0.003027 | 10.734 | 3.634 | 293 (284) |
| 30 | 1 | 10 | 3 | no (0) | 0.000 | 1.000 | 0.000 | yes (855) | no | 0 / 0 / 0 | 0 | 0.008 (0.024) | 15 | 790 | 0.335 | 0.0565 | 0.003027 | 10.734 | 3.634 | 382 (371) |
| 30 | 1 | 10 | 4 | no (0) | 0.000 | 1.000 | - | yes (899) | no | 0 / 0 / 0 | 0 | 0.007 (0.024) | 15 | 750 | 0.342 | 0.0565 | 0.003027 | 10.734 | 3.634 | 383 (370) |
| 30 | 1 | 10 | 5 | no (0) | 0.000 | 1.000 | - | yes (825) | no | 0 / 0 / 0 | 0 | 0.007 (0.024) | 15 | 768 | 0.305 | 0.0565 | 0.003027 | 10.734 | 3.634 | 349 (339) |
| 30 | 4 | 1 | 1 | no (0) | 0.000 | 1.000 | 0.000 | yes (849) | no | 0 / 0 / 0 | 0 | 0.007 (0.022) | 15 | 694 | 0.366 | 0.0565 | 0.000757 | 1.073 | 3.634 | 524 (508) |
| 30 | 4 | 1 | 2 | no (0) | 0.000 | 1.000 | - | yes (872) | no | 0 / 0 / 0 | 0 | 0.007 (0.022) | 15 | 778 | 0.320 | 0.0565 | 0.000757 | 1.073 | 3.634 | 416 (403) |
| 30 | 4 | 1 | 3 | no (0) | 0.002 | 1.000 | 0.000 | yes (872) | no | 0 / 0 / 0 | 0 | 0.007 (0.023) | 15 | 730 | 0.326 | 0.0565 | 0.000757 | 1.073 | 3.634 | 496 (480) |
| 30 | 4 | 1 | 4 | no (0) | 0.000 | 1.000 | - | yes (763) | no | 0 / 0 / 0 | 0 | 0.007 (0.022) | 15 | 674 | 0.322 | 0.0565 | 0.000757 | 1.073 | 3.634 | 361 (351) |
| 30 | 4 | 1 | 5 | no (0) | 0.000 | 1.000 | - | yes (926) | no | 0 / 0 / 0 | 0 | 0.007 (0.023) | 15 | 740 | 0.340 | 0.0565 | 0.000757 | 1.073 | 3.634 | 354 (343) |
| 30 | 4 | 3 | 1 | no (0) | 0.000 | 1.000 | - | yes (910) | no | 0 / 0 / 0 | 0 | 0.004 (0.013) | 15 | 775 | 0.300 | 0.0565 | 0.000757 | 3.220 | 3.634 | 615 (604) |
| 30 | 4 | 3 | 2 | no (0) | 0.000 | 1.000 | - | yes (885) | no | 0 / 0 / 0 | 0 | 0.004 (0.013) | 15 | 832 | 0.268 | 0.0565 | 0.000757 | 3.220 | 3.634 | 291 (285) |
| 30 | 4 | 3 | 3 | no (0) | 0.000 | 1.000 | - | yes (844) | no | 0 / 0 / 0 | 0 | 0.004 (0.014) | 15 | 838 | 0.281 | 0.0565 | 0.000757 | 3.220 | 3.634 | 422 (414) |
| 30 | 4 | 3 | 4 | no (0) | 0.000 | 1.000 | 0.000 | yes (911) | no | 0 / 0 / 0 | 0 | 0.004 (0.014) | 15 | 801 | 0.265 | 0.0565 | 0.000757 | 3.220 | 3.634 | 386 (380) |
| 30 | 4 | 3 | 5 | no (0) | 0.000 | 1.000 | 0.000 | yes (852) | no | 0 / 0 / 0 | 0 | 0.004 (0.014) | 15 | 788 | 0.321 | 0.0565 | 0.000757 | 3.220 | 3.634 | 590 (580) |
| 30 | 4 | 10 | 1 | no (0) | 0.000 | 1.000 | - | yes (847) | no | 0 / 0 / 0 | 0 | 0.002 (0.006) | 15 | 841 | 0.265 | 0.0565 | 0.000757 | 10.734 | 3.634 | 344 (339) |
| 30 | 4 | 10 | 2 | no (0) | 0.000 | 1.000 | - | yes (866) | no | 0 / 0 / 0 | 0 | 0.002 (0.006) | 15 | 856 | 0.271 | 0.0565 | 0.000757 | 10.734 | 3.634 | 333 (330) |
| 30 | 4 | 10 | 3 | no (0) | 0.000 | 1.000 | - | yes (867) | no | 0 / 0 / 0 | 0 | 0.002 (0.006) | 15 | 859 | 0.260 | 0.0565 | 0.000757 | 10.734 | 3.634 | 307 (303) |
| 30 | 4 | 10 | 4 | no (0) | 0.000 | 1.000 | - | yes (918) | no | 0 / 0 / 0 | 0 | 0.002 (0.006) | 15 | 867 | 0.262 | 0.0565 | 0.000757 | 10.734 | 3.634 | 222 (220) |
| 30 | 4 | 10 | 5 | no (0) | 0.000 | 1.000 | 0.000 | yes (851) | no | 0 / 0 / 0 | 0 | 0.002 (0.006) | 15 | 839 | 0.266 | 0.0565 | 0.000757 | 10.734 | 3.634 | 308 (306) |

