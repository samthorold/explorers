# Issue #837: the pre-registered crossing reading for mode 1

**Status: done (2026-10-15). With crossing gone, the line stays at the vertex and no point passes, so follow-up 2 fires.** Under marker isolation at `s` = 100, no line birth has a parent outside the line in any of the 10 runs. The founders' line stays at the heterotroph vertex by the rule in 4 of 5 seeds at both `n` = 10 and 5. No point passes: `n` = 10 holds the guild in 0 of 5 seeds and `n` = 5 in 2 of 5, where #829 passed it in 3. Producers persist in all 10 runs. Specialists take most of the drain in no seed. But where the line survives the clearing at the vertex (`n` = 5, seeds 2 and 4), the arm's pure-vertex drain is above 0 for the first time in mode 1, at 0.0003 and 0.006. By the rule, **follow-up 2 fires: crossing is fixed but not sufficient, and the reproduction grill continues with ageing, then upkeep with growth, then timing.** Follow-ups 1, 3 and 4 do not fire. The flag does not fire on the producers (mean `c` 0.993–0.995), and it cannot be read on the line, which is extinct at the end in every run.

Two parts of the prediction fail. Founder births do not halve: every founder birth is now asexual, and asexual births rise to take the crossings' place (18 against #833's 23 at `n` = 10, 22 against 21 at 5). And the line dies out in all 10 runs, by tick 80–1,979, where #833's line, carried by its hybrids, was alive at the end in 5 of these 10. Without crossing the line stays a decomposer line, and it does not recruit. No default changed.

## The question

#833 found that the founder line leaves the heterotroph vertex by crossing with producers: 22 of 24 departures had a parent outside the line, because a founder and a recipe producer are 1.52 apart in trait space against a compatibility distance of 3.81. The domain says decomposers and plants do not cross ([trophic roles](../ecology/trophic-roles.md), *A decomposer lineage stays a decomposer*). So the reproduction grill took crossing first, and replaced the isolation rule's form: world rules' *marker isolation* (flow 4), where reproductive isolation follows ancestry through a neutral marker. This reading reads it alone, before ageing, upkeep with growth and timing are grilled. The pre-registration is mode 1's **Crossing** bullet in `docs/system-design/reference-modes.md`.

## The slice

Commit 13ca418 implements *Reproductive isolation follows ancestry, through a neutral marker* in `explorers-sim`, behind `marker_isolation` (off by default, so every default world is unchanged state for state).
- Each agent carries an eight-dimensional marker outside its traits. A birth inherits it (a sexual birth takes each dimension from one parent) and adds a unit Gaussian step a dimension.
- Each candidate pair takes one draw against `c = exp(−d² / (2 s²))` before mating, and trait distance plays no part.
- With the switch on, world creation places each founding cluster's marker origin exactly `10 s` from any other's, so `c` between clusters is `e⁻⁵⁰`.

`reference_mode --marker-isolation --compatibility-scale S` turns it on: producers found cluster 0 and decomposer founders cluster 1. Each arm's report gains a `marker` readout: per sample, the founder line's living members and their share below autotrophy 0.1, and at the end the mean and lowest pairwise `c` within the line and within the producers. Default reports are byte-identical (checked against #833's frozen binary).

## The runs

Release build of `reference_mode` at 13ca418, frozen as `target/837-sweep/reference_mode`. #829's command at its tiling reach, with marker isolation on:

```sh
reference_mode --seed $s --clear-at 1500 --ticks 1500 \
  --size-scaled-base-rate --established-producer-structure 6.937 --living-resistance \
  --litter-half-life 1 --enzyme-half-saturation 1 \
  --decomposer-founders $n --body-reach-pieces $k \
  --marker-isolation --compatibility-scale 100 \
  --out runs/n${n}_s${s}/report.json
```

`n` = 10 (`k` = 137.7) and 5 (`k` = 275.4), seeds 1–5, paired: 10 runs. The baseline is the same command without marker isolation: #829's runs, re-read with the recruitment readout by #833 (`target/833-sweep/runs/`), paired by point and seed.

**The readout.** As in #829 and #833, every figure is the control arm's unless it says otherwise.
- Guild verdict, fewest members, share of the second half's carcass drain, producers persist and the pass come from `report.control.guild` (a point passes by #814's combined rule: the guild holds and producers persist, at 3 of 5 seeds).
- The specialist share is the pure-vertex share of `arm_carcass_drain` (ticks 1,500–3,000).
- Founder deaths, causes and births, and the line's births by parent, come from `report.control.recruitment`.
- **The line at the vertex.** The rule reads the line's share below autotrophy 0.1 at every sample while it lives. The `marker` readout samples only the arm, from tick 1,500, and the line is extinct before then in 7 of 10 runs. So the share was also reconstructed from the recruitment readout's births and deaths at every 25-tick sample from tick 0 (traits are fixed at birth). Both readings give the same verdict (*The verdict*, below).
- Marker spread is `report.control.marker.line` and `.producers`, at the end.

## The verdict, by the fixed rule

| `n` | point passes | guild holds | producers persist | line stays at the vertex (seeds) | specialists take > half (seeds) | specialist share, by seed | `S_p` median (range) |
|---:|---|---|---|---|---|---|---|
| 10 | no (0/5) | 0/5 | 5/5 | **yes (4/5)**; not seed 4 | no (0/5) | 0 in all | 8.27 (7.32–8.38) |
| 5 | no (2/5) | 2/5 (seeds 2, 4) | 5/5 | **yes (4/5)**; not seed 1 | no (0/5) | 0, 0.0003, 0, 0.006, 0 | 7.89 (7.52–8.18) |

- **The guild.** At `n` = 10 it never holds (fewest 0, 3, 0, 1, 0). At 5 it holds in seeds 2 (fewest 5, share 0.149) and 4 (9, 0.428), against #829's seeds 3, 4 and 5. In both, the founder line is extinct by tick 1,694, so the guild over ticks 2,250–3,000 is not the line's. Its members are heterotroph-dominant mixotrophs of the producers' cluster, as in #829.
- **The line at the vertex.** At every sample while it lives, it is wholly below autotrophy 0.1 in 8 of 10 runs. The two exceptions are departures by mutation, not crossing:
  - `n` = 10 seed 4: a founder's asexual offspring at tick 5 has autotrophy 0.208. Its asexual descendants keep it, the pure-vertex members are gone by tick 300, and the line is wholly off the vertex from then until it dies at 1,979. In the arm's 20 samples with the line alive (1–4 members) its share below 0.1 is 0.
  - `n` = 5 seed 1: one offspring at tick 519 has autotrophy 0.1005, just over the line. In a line of 2–4 members the share falls to 0.5 at tick 600, and the mutant is the last member, dead at 1,024.
  - Read on the arm's samples alone, the verdict is the same: only `n` = 10 seed 4 has a sample below 90 %; the seeds with no arm sample do not fail it.
- **Producers.** They persist in all 10 runs, with 664–784 at the control's end.

## The prediction and its result

*With crossing gone, no line birth has a parent outside the line, and the line stays at the heterotroph vertex. Founder births fall by about half. Founders still die as #833 found, on reserve at `n` = 10 and by age at 5, so no point passes. Where the line survives past the clearing, specialists take a share of carcass drain above 0.*

1. **No line birth has a parent outside the line: holds, in all 10 runs.** The lines have 95 births, 92 asexual and 3 sexual within the line. #833 had 220 crossings among 1,367 line births in these ten worlds.
2. **The line stays at the vertex: holds by the rule,** in 4 of 5 seeds at each point. The two departures are by mutation (above).
3. **Founder births halve: fails.**

   | `n` | founder births, #837 (seeds 1–5) | #833, same seeds | of which crossings in #833 | founder attempts, #837 / #833 |
   |---:|---|---|---|---|
   | 10 | 18 (1, 2, 3, 10, 2) | 23 (5, 4, 1, 12, 1) | 9 | 48 / 50 |
   | 5 | 22 (5, 5, 1, 10, 1) | 21 (8, 2, 2, 8, 1) | 7 | 73 / 79 |

   Every founder birth under marker isolation is asexual: no founder mates with another, since founders are scattered far apart. Asexual births rise from 14 to 18 at `n` = 10 and from 14 to 22 at 5, taking most of the crossings' place. A founder that fails its asexual roll and finds no compatible mate stays gated and rolls again; it does not spend the earmark on a cross. The pre-registration took "about half" from #833's 37 of 79 across all three points. At 10 and 5 alone, crossings were 9 of 23 and 7 of 21, so even a full loss would have been about 40 % and 33 %.
4. **Founders die as #833 found: holds.** At `n` = 10, 49 of 50 die on reserve and 1 senesces (#833: the same), at ticks 15–92. At 5, 22 of 25 die by age and 3 on reserve (#833: 23 and 2), at ticks 53–734.
5. **No point passes: holds.**
6. **Specialists' drain share above 0 where the line survives the clearing: holds where the line survives at the vertex.** The line is alive at tick 1,500 in three runs. At `n` = 5 seeds 2 and 4 it is at the vertex, alive to 1,510 and 1,694, and the arm's pure-vertex drain is 19 and 374, a share of 0.0003 and 0.006, all of it the line's. At `n` = 10 seed 4 it is off the vertex at autotrophy 0.208, and the share is 0. Every run of #814, #822, #826, #829 and #833 had an arm pure-vertex drain of exactly 0.

## What follows, by the rule

1. **`n` = 5 passes, the line stays at the vertex and specialists take most of the drain: does not fire.** `n` = 5 passes in 2 of 5 seeds and specialists take most of the drain in none. The marker-isolation switch is not recommended on this reading as the binding fix, and it stays off by default.
2. **The line stays at the vertex, but no point passes: fires.** Crossing is fixed but not sufficient. The reproduction grill continues with #833's other discrepancies, in its order: **ageing, then upkeep with growth, then timing.**
3. **The line does not stay at the vertex: does not fire.** It stays in 4 of 5 seeds at each point. The two mutational departures are recorded under *Observations*.
4. **Producers collapse at any point: does not fire.** They persist in all 10 runs, in both arms.

These are recorded, not acted on. No grill has run, no issue has been filed, and no default changed.

## The flag: marker spread

| `n` | line at the end (members) | producers: mean `c` | producers: lowest `c` |
|---:|---|---|---|
| 10 | 0 in every run, both arms | 0.9930–0.9946 | 0.9658–0.9745 |
| 5 | 0 in every run, both arms | 0.9935–0.9945 | 0.9573–0.9728 |

The flag is a mean within-cluster `c` below 0.8. **It does not fire on the producers,** whose mean `c` is 0.993–0.995 in both arms, against the pre-registration's ≥ 0.99. **It cannot be read on the line,** which is extinct at the end of every arm, so `marker.line` has no pairs. The line's 95 births, about 10 a run, are far below the 250 generations the pre-registration's bound allowed, so its drift would have been far smaller than that bound's `d` of about 63. `s` = 100 was not too small.

## Observations beyond the rule, not verdicts

- **The line no longer recruits.** It has 1–24 births a run (95 in all), against #833's 1–394 (1,367) in the same worlds, and every line is extinct by tick 1,979 in both arms. In #833 most line births were off-vertex hybrids breeding among themselves. Without them, what is left is the pure-vertex line, and #833 already found that it does not establish. Its 95 non-founder members die on reserve (52) or by age (43).
- **The guild at `n` = 5 changed seeds.** It holds in seeds 2 and 4 here, against 3, 4 and 5 in #829. In #829 seeds 3 and 5 the line was extinct by tick 791 and 328, so the guild there was not the line's either. A world diverges from its #829 twin at the first mating the switch changes, so the guild's seeds are not paired in any finer sense than the point. With five seeds this is an observation.
- **Pure-vertex drain before the clearing rose.** It is 0.009–0.103 of the drain over ticks 0–1,500 at `n` = 10 and 0.054–0.364 at 5, against #829's 0.006–0.009 and 0.052–0.278. Single long-lived pure-vertex members carry it at `n` = 10 seeds 3 and 5 (alive to 718 and 1,151).
- **Founders at `n` = 10 live a little longer.** 0–4 are alive at tick 50 by seed (#829: 1–2). The last founder death at `n` = 5 seed 4 is at 734, against 558.
- **The perturbed arm agrees.** It passes only at `n` = 5 seed 4 (fewest 11); producers persist everywhere; the line is extinct at the end everywhere.
- **Mutation can carry a pure-vertex line across autotrophy 0.1 in one birth** (0.208 at tick 5 at `n` = 10 seed 4). #833 found the same twice in 15 runs. With a line this small, one such member can decide the vertex share.

## Runtime

10 paired runs, release build, on 8 cores, through `xargs -P 5`, `n` = 5 first. Cost is CPU time (`user` + `sys` from `/usr/bin/time -p`):
- `n` = 10: 1,319–1,636 s a run, 7,298 s for the five. Wall-clock was 1,462–1,786 s, 1.09–1.11 times CPU; they ran awake, five in parallel, about 25–30 min each.
- `n` = 5: 1,413–1,799 s a run, 8,053 s for the five. **Wall-clock is not meaningful** (7,942 s recorded for each): the laptop slept with the runs under `caffeinate -i`, which does not stop system or lid sleep, and they were paused with SIGSTOP and resumed with SIGCONT under `caffeinate -s` (see `docs/agents/instrument-runtimes.md`).
- In all: 15,351 s of CPU (4.3 CPU-hours), against the pre-registered estimate of about 4.5. In `user` time alone, as #829 and #833 reported it, 14,988 s: 7,139 s at `n` = 10 and 7,848 s at 5, 9 % and 13 % below #833's 7,807 and 8,998 s. The line is smaller, and the marker draw costs nothing measurable.

## Reproducing

The frozen binary, scripts and per-run reports are under `target/837-sweep/` (not committed). To rebuild:

```sh
cargo build --release -p explorers-reference-modes --bin reference_mode
mkdir -p target/837-sweep && cp target/release/reference_mode target/837-sweep/
# one paired run per line "n seed"; k = 137.7 and 275.4 for n = 10 and 5
caffeinate -s xargs -P 5 -L 1 sh target/837-sweep/scripts/run.sh < target/837-sweep/jobs.txt
python3 -I target/837-sweep/scripts/analyze.py
```

`run.sh N SEED` maps `n` to `k`, skips a run whose `report.md` exists, and runs the command above under `/usr/bin/time -p` into `runs/n${N}_s${SEED}/`. `analyze.py` reads both arms of every run and #833's reports for the same point and seed, and prints the per-seed rows below; with the argument `rec` it also prints the reconstructed line and its vertex share every 100 ticks.

## Appendix: per point and seed

The control arm. Line at vertex is the lowest share of living line members below autotrophy 0.1 over every 25-tick sample while the line lives (reconstructed from tick 0). Founder births are against #833's for the same seed. CPU is `user` + `sys`.

| `n` | seed | guild holds (fewest), share | producers (at end) | pass | specialist share | founder deaths: senesced / reserve | first–last death | founder births (#833) | line births: asexual / sexual in line / crossing | line at vertex, lowest share | line's last member dies | arm samples with the line | producers' mean (lowest) `c` | CPU s |
|---:|---:|---|---:|---|---:|---|---|---|---|---:|---:|---:|---|---:|
| 10 | 1 | no (0), 0.000 | 784 | no | 0.000 | 1 / 9 | 18–80 | 1 (5) | 1 / 0 / 0 | 1.00 | 80 | 0 | 0.9930 (0.9674) | 1,518 |
| 10 | 2 | no (3), 0.215 | 776 | no | 0.000 | 0 / 10 | 15–69 | 2 (4) | 2 / 0 / 0 | 1.00 | 69 | 0 | 0.9944 (0.9684) | 1,636 |
| 10 | 3 | no (0), 0.000 | 762 | no | 0.000 | 0 / 10 | 17–92 | 3 (1) | 3 / 0 / 0 | 1.00 | 718 | 0 | 0.9944 (0.9735) | 1,423 |
| 10 | 4 | no (1), 0.079 | 688 | no | 0.000 | 0 / 10 | 16–45 | 10 (12) | 23 / 1 / 0 | **0.00** | 1,979 | 20 | 0.9942 (0.9676) | 1,319 |
| 10 | 5 | no (0), 0.000 | 729 | no | 0.000 | 0 / 10 | 16–85 | 2 (1) | 6 / 0 / 0 | 1.00 | 1,151 | 0 | 0.9941 (0.9659) | 1,401 |
| 5 | 1 | no (0), 0.040 | 758 | no | 0.000 | 3 / 2 | 53–550 | 5 (8) | 7 / 0 / 0 | **0.00** | 1,024 | 0 | 0.9940 (0.9685) | 1,799 |
| 5 | 2 | **yes (5), 0.149** | 701 | **yes** | 0.0003 | 5 / 0 | 71–487 | 5 (2) | 19 / 1 / 0 | 1.00 | 1,510 | 1 | 0.9944 (0.9722) | 1,631 |
| 5 | 3 | no (0), 0.010 | 702 | no | 0.000 | 4 / 1 | 75–327 | 1 (2) | 7 / 0 / 0 | 1.00 | 1,176 | 0 | 0.9938 (0.9728) | 1,574 |
| 5 | 4 | **yes (9), 0.428** | 664 | **yes** | 0.006 | 5 / 0 | 133–734 | 10 (8) | 23 / 1 / 0 | 1.00 | 1,694 | 8 | 0.9937 (0.9625) | 1,413 |
| 5 | 5 | no (0), 0.035 | 772 | no | 0.000 | 5 / 0 | 116–328 | 1 (1) | 1 / 0 / 0 | 1.00 | 328 | 0 | 0.9938 (0.9711) | 1,636 |
