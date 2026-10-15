# Issue #840: the pre-registered ageing reading for mode 1

**Status: done (2026-10-15). Growth dilution does not bite, so follow-up 3 fires.** At `n` = 5, 20 of the 25 founders still die by age, against #837's 22 and the rule's 11 or fewer. Dilution does lower the founders' wear, but too little to move their deaths. 17 of the 25 die at the same tick, by the same cause, as their #837 twins. At death their wear is a median 8 % lower (0.73–1.14 times #837's), and their hazard summed over life is 9 % lower (0.74–0.96 times). The founders do not level off and then senesce, as follow-up 3's wording assumed. Every founder that senesces is still growing on the tick before it dies, and grows on nearly every tick of its life. It grows slowly: about 1.1–1.5 % a tick for its first 50 ticks, then 0.1–0.3 %. So dilution removes about a fifth to under a half of the wear a long-lived founder accrues, and the rest kills it at the same ages.

The rest of the prediction holds. `n` = 10 is unchanged: 49 of 50 founders die on reserve and one by age, as in #837. No point passes: `n` = 10 in 0 of 5 seeds and `n` = 5 in 1 of 5. Producers persist in all 10 runs. Their established median lifespan moves by 13.3 % at most, and by under 20 % in every seed, so they do not *shift*. The line stays at the vertex by #837's rule in 4 of 5 seeds at `n` = 10 and 3 of 5 at 5. By the rule, **follow-up 3 fires: the upkeep grill takes shedding, which must then carry ageing too.** The paths below show that what it must deal with is slow growth, not growth that has stopped. Follow-ups 1, 2 and 4 do not fire. No default changed.

## The question

#837 removed crossing, and the founder line stayed at the heterotroph vertex, but no point passed. At `n` = 5 the founders out-earn their upkeep and die by age (22 of 25). In this world a decomposer has a plant's ageing: its hazard rises with its own drain, so the largest, best-fed founders age fastest. In the domain a saprotroph's genet has no age-tied hazard ([fungi](../ecology/fungi.md), *The saprotroph's life history*). The reproduction grill took ageing next, and specified world rules' *growth dilutes wear* (*Somatic wear*). New structure is built unworn, so a tick of growth from `S` to `S + ΔS` scales each trait's wear by `S / (S + ΔS)`. This reading reads it alone, with marker isolation on as in #837. The pre-registration is mode 1's **Ageing** bullet in `docs/system-design/reference-modes.md`.

## The slice

Commit c491eac implements *Growth dilutes wear* in `explorers-sim`, behind `growth_dilutes_wear` (serde default false, so every default world is unchanged state for state). Dilution sits in `phase::grow` and its SoA mirror, where growth adds the structure: after that tick's repair, and before its wear accumulation. A fall in structure leaves wear unchanged, and at `S = 0` the wear is left as it is. It applies to every agent, producers included. `reference_mode --growth-dilutes-wear` turns it on and records it in the summary and the artifact. There are no new readouts: #833's recruitment readout already records each founder's structure and wear every tick.

## The runs

Release build of `reference_mode` at c491eac, frozen as `target/840-sweep/reference_mode`. #837's command with dilution on:

```sh
reference_mode --seed $s --clear-at 1500 --ticks 1500 \
  --size-scaled-base-rate --established-producer-structure 6.937 --living-resistance \
  --litter-half-life 1 --enzyme-half-saturation 1 \
  --decomposer-founders $n --body-reach-pieces $k \
  --marker-isolation --compatibility-scale 100 \
  --growth-dilutes-wear \
  --out runs/n${n}_s${s}/report.json
```

`n` = 10 (`k` = 137.7) and 5 (`k` = 275.4), seeds 1–5, paired: 10 runs. The baseline is #837's runs (`target/837-sweep/runs/`), the same command without dilution, paired by point and seed.

**The readout.** As in #837, every figure is the control arm's unless it says otherwise.
- Guild verdict, fewest members, share of carcass drain, producers persist and the pass come from `report.control.guild`. A point passes by #814's combined rule: the guild holds and producers persist, in at least 3 of 5 seeds.
- The specialist share is the pure-vertex share of `arm_carcass_drain` (ticks 1,500–3,000).
- Founder deaths, causes, ages, structure and wear, and the line's births, come from `report.control.recruitment`. The readout's wear is the mean over the three functional traits. The hazard is `η` × their sum, with `η` = 0.022, so a tick's hazard is 0.066 × the readout's wear. *Hazard over life* is that summed over every tick the founder lived.
- The line's share below autotrophy 0.1 is reconstructed at every 25-tick sample from tick 0, from the recruitment readout's births and deaths, as in #837.
- The producers' established median lifespan is `report.control.litter.second_half.established_lifespan_median` (producers older than 50 ticks, dying over ticks 2,250–3,000).

## The verdict, by the fixed rule

| `n` | dilution bites (senescent founders, #837) | point passes | guild holds | producers persist | line stays at the vertex (seeds) | producers shift (seeds) | established median lifespan, change from #837 by seed |
|---:|---|---|---|---|---|---|---|
| 10 | (not read; 1 of 50, #837: 1) | no (0/5) | 0/5 | 5/5 | **yes (4/5)**; not seed 4 | no (0/5) | +0.6, +2.3, +1.5, +1.2, −1.5 % |
| 5 | **no: 20 of 25** (22); the rule needs 11 or fewer | no (1/5) | 1/5 (seed 2) | 5/5 | **yes (3/5)**; not seeds 2, 3 | no (0/5) | −7.4, +3.9, −3.5, +0.9, −13.3 % |

- **Dilution does not bite.** Senescent founder deaths at `n` = 5 are 3, 5, 4, 4 and 4 by seed (#837: 3, 5, 4, 5, 5). The two fewer are seed 4's founder 164, which dies on reserve at 132 instead of by age at 734, and seed 5's founder 166, which dies on reserve at 588 instead of by age at 328.
- **The guild.** At `n` = 10 it never holds (fewest 0, 0, 0, 4, 3). At `n` = 5 it holds only in seed 2 (fewest 7, share 0.218), as in #837 (5, 0.149). Seed 4 lost the guild it held in #837 (fewest 9, share 0.428). In seed 2 the founder line is extinct by tick 2,124, before the guild's window of ticks 2,250–3,000, so the guild is not the line's.
- **The line at the vertex.** At `n` = 10 seed 4 it leaves by the same mutation as in #837: an asexual offspring at tick 5 with autotrophy 0.208. At `n` = 5 the two departures are also by mutation, and late: seed 2, autotrophy 0.230 at tick 1,005, whose descendants are the line's last members; seed 3, autotrophy 0.127 at tick 1,407, half of a two-member line. No line birth has a parent outside the line in any run. At `n` = 5 the rule holds at its edge: three seeds of five.
- **Producers.** They persist in all 10 runs, with 678–875 at the control's end (#837: 664–784). None shifts.

## The prediction and its result

*At `n` = 5 dilution bites: senescent founder deaths fall from #837's 22 of 25 to at most 11, founders live longer, and most that die do so on reserve once they stop growing. At `n` = 10 nothing changes, because founders die on reserve before age matters. No point passes, because the line still barely recruits; longer-lived founders may give more births, not a guild. Producers persist in all 10 runs, and their established median lifespan on the control moves by less than 20 % from #837's.*

1. **Senescent founder deaths at `n` = 5 at most 11: fails.** 20 of 25.
2. **Founders live longer: mostly fails.** Of the 25 founders at `n` = 5, 17 die at the same tick as in #837, 4 live longer and 4 die sooner. The median age at death is 276 ticks against 266. The longer lives are seed 3's founders 164 (550 against 327) and 166 (512 against 210), and seed 5's 164 (276 against 265) and 166 (588 against 328). The shorter are two reserve deaths in seed 1 (133 against 229, 122 against 145), one in seed 3 (72 against 75), and seed 4's founder 164 (132 against 734). The worlds diverge from their twins once dilution changes any agent's state, so those changes are not all dilution's direct effect on that founder.
3. **Most founders that die do so on reserve once they stop growing: fails.** 5 of 25 die on reserve (#837: 3). The 20 that senesce never stop growing (*Founder structure paths*, below).
4. **`n` = 10 unchanged: holds.** 49 of 50 founders die on reserve and 1 senesces, as in #837, at ticks 15–84 (#837: 15–92). 37 of the 50 die at the same tick as in #837, and 12 within 1–8 ticks of it, as the world drifts from its twin. Seed 5's founder 164 dies on reserve at 50 instead of 85.
5. **No point passes: holds.**
6. **The line barely recruits: holds,** though dilution gives it more births. The line has 132 births in all (49 at `n` = 10, 83 at 5), against #837's 95 (36 and 59). Founder births are 18 at `n` = 10, as in #837, and 25 at 5 against 22. It survives the clearing in 6 of 10 runs, against 3 in #837. At `n` = 10 seed 4 it is alive at the end, with 2 members off the vertex at autotrophy 0.208, the first line alive at the end since #833. Specialists take a share of the arm's carcass drain above 0 in 5 runs (0.0006–0.013), against 2 in #837, and above a half in none.
7. **Producers persist in all 10 runs, and their established median lifespan moves by less than 20 %: holds.** The largest change is −13.3 % (`n` = 5 seed 5, 307 against 354 ticks).

## Did dilution lower wear, or lower it too little?

It lowered it too little. The implementer's smoke run found why that is enough to leave the deaths in place. The senescence draw is keyed by agent, tick and phase, so a founder draws the same uniform at the same tick with dilution on and off. Its death moves only if one of those uniforms falls between the two hazards. Late in a founder's life its hazard is 0.001–0.007 a tick, so a 10 % fall in hazard moves a given death about one time in ten.

The 17 founders at `n` = 5 that die at the same tick by age in both runs show the size of the fall:

| | median | range |
|---|---:|---|
| wear at death, on / off | 0.92 | 0.73–1.14 |
| hazard summed over life, on / off | 0.91 | 0.74–0.96 |
| hazard a tick at death, on (off) | 0.0024 (0.0027) | 0.0009–0.0074 (0.0009–0.0074) |

The two ratios above 1 (seed 4's founders 163 and 165) belong to founders whose world had diverged: they grew larger than their twins (24.0 against 17.5 and 19.0 against 15.4), drained more and wore more. Summed hazard fell for all 17.

Dilution's share of the wear a founder accrued, estimated from the readout as `w · (1 − S_prev / S)` summed over the ticks it grew, is 2–12 % for founders that live under 140 ticks and 18–45 % for the long-lived ones. For the founders at `n` = 10, which die by tick 92, it is at most 7 %, and their wear at the last tick both twins lived is 0.94–0.99 times #837's.

## Founder structure paths: when growth stopped

Every founder starts at structure 2.46.

**`n` = 10.** Founders grow fast to 3.4–4.0 by tick 10–14 (one, seed 3's founder 162, to 4.03 at tick 39), then stop growing and die on reserve by tick 15–92. Their growth stops because their reserve runs out, not because they are large. Ageing does not reach them.

**`n` = 5.** Two paths.
- **The 20 that senesce never stop.** All 20 grow on the tick before they die. 18 grow on all but two ticks of their life; seed 5's founders 162 and 164 grow on 250 of 266 and 260 of 276. Over the first 50 ticks the long-lived ones grow 1.1–1.5 % a tick, to about 4.5. After that they grow 0.1–0.3 % a tick. The long-lived reach 10–24 by their death at ticks 487–558. Structure at ticks 0, 100, 200, 300, 400 and 500 for some of them:

  | seed, founder | death (cause) | 0 | 100 | 200 | 300 | 400 | 500 |
  |---|---|---:|---:|---:|---:|---:|---:|
  | 1, 163 | 550 (age) | 2.5 | 5.2 | 6.7 | 9.7 | 13.9 | 17.1 |
  | 2, 164 | 371 (age) | 2.5 | 4.5 | 4.9 | 5.9 | – | – |
  | 3, 166 | 512 (age) | 2.5 | 5.4 | 6.6 | 8.2 | 9.6 | 10.6 |
  | 4, 163 | 558 (age) | 2.5 | 5.1 | 6.6 | 9.5 | 14.0 | 20.7 |
  | 5, 162 | 266 (age) | 2.5 | 4.6 | 5.0 | – | – | – |

- **The 5 that die on reserve stop first.** Four stop growing at tick 35–87, at structure 4.1–4.4, and die on reserve 37–57 ticks later. The fifth, seed 5's founder 166, grows to 24.9 by tick 556 and dies on reserve at 588.

So `n` = 5's founders are not bodies that have stopped growing. They are bodies that grow at a fraction of a percent a tick while their wear accrues with their drain. The hazard rises with that drain, and growth this slow cannot dilute it away. This is what the upkeep grill inherits. Shedding would have to carry the ageing of a body that is still growing, and how fast a founder can grow is a question for upkeep with growth.

## What follows, by the rule

1. **A point passes: does not fire.** `n` = 10 passes in 0 of 5 seeds and `n` = 5 in 1 of 5.
2. **Dilution bites, but no point passes: does not fire.** Dilution does not bite.
3. **Dilution does not bite: fires.** Founders still senesce. The upkeep grill takes shedding, which must then carry ageing too. The founders' structure paths are above. They do not level off: they senesce while growing at 0.1–0.3 % a tick.
4. **Producers collapse at any point, or shift: does not fire.** They persist in all 10 runs, and none shifts at either point.

These are recorded, not acted on. No grill has run, no issue has been filed, and no default changed. Growth dilution stays off by default and is not recommended on this reading.

## Observations beyond the rule, not verdicts

- **Producers senesce less, but their established lifespan barely moves.** Senescent producer deaths fall from 1,280–1,546 a run to 1,182–1,349, and the median senescent producer's age at death rises from 489–515 ticks to 496–543. The established median lifespan, which counts every cause, moves by −13 % to +4 %. Producers grow while their wear is still low, as the pre-registration expected, so dilution reaches them mostly where age was not what killed them.
- **The producers' end counts spread more.** They are 678–875 at the control's end, against 664–784. `n` = 10 seed 4 has 875 against 688. `S_p` (median established structure, second half) is 7.35–8.52, against 7.32–8.38.
- **The perturbed arm agrees.** It passes only at `n` = 5 seed 2 (fewest 6). In #837 it passed only at seed 4. Producers persist everywhere.
- **The line's late mutants.** Both of `n` = 5's departures from the vertex are births after tick 1,000 by members that lived longer under dilution. A line that lives longer gives mutation more births to carry it off the vertex. With so few members, one such birth decides the share.

## Runtime

10 paired runs, release build, on 8 cores, through `xargs -P 5`, `n` = 5 first. Cost is CPU time (`user` + `sys` from `/usr/bin/time -p`):
- `n` = 10: 1,404–1,620 s a run, 7,670 s for the five. Wall-clock was 1,417–1,636 s, 1.01 times CPU, about 25 min each.
- `n` = 5: 1,563–1,925 s a run, 8,625 s for the five. **Wall-clock is not meaningful** (4,185–4,553 s, 2.4–2.7 times CPU). The laptop was on battery, under `caffeinate -i -s`, and `-s` holds the system awake only on AC power (see `docs/agents/instrument-runtimes.md`).
- In all: 16,295 s of CPU (4.5 CPU-hours), 6 % above #837's 15,351 s, against the pre-registered estimate of about 4.3. The line is larger and lives longer.

## Reproducing

The frozen binary, scripts and per-run reports are under `target/840-sweep/` (not committed). To rebuild:

```sh
cargo build --release -p explorers-reference-modes --bin reference_mode
mkdir -p target/840-sweep && cp target/release/reference_mode target/840-sweep/
# one paired run per line "n seed"; k = 137.7 and 275.4 for n = 10 and 5
caffeinate -s xargs -P 5 -L 1 sh target/840-sweep/scripts/run.sh < target/840-sweep/jobs.txt
python3 -I target/840-sweep/scripts/analyze.py
python3 -I target/840-sweep/scripts/wear.py
```

`run.sh N SEED` maps `n` to `k`, skips a run whose `report.md` exists, and runs the command above under `/usr/bin/time -p` into `runs/n${N}_s${SEED}/`. `analyze.py` reads every run and #837's run for the same point and seed, and prints the per-seed rows below with each founder's death, wear at death, summed hazard and structure (with the argument `paths`, the structure and wear paths every 25 ticks). `wear.py` pairs each founder with its #837 twin and prints the wear ratio, dilution's share of the wear accrued and the mean growth rate.

## Appendix: per point and seed

The control arm, with #837's figure in brackets for the same point and seed. Line at vertex is the lowest share of living line members below autotrophy 0.1 over every 25-tick sample while the line lives. CPU is `user` + `sys`.

| `n` | seed | guild holds (fewest), share | pass | producers (at end) | established median lifespan | specialist share | founder deaths: senesced / reserve | first–last death | founder births | line births | line at vertex, lowest share | line's last member dies | CPU s |
|---:|---:|---|---|---:|---:|---:|---|---|---:|---:|---:|---:|---:|
| 10 | 1 | no (0), 0.065 | no | 678 (784) | 336 (334) | 0.000 | 1 / 9 (1 / 9) | 18–83 | 1 (1) | 1 (1) | 1.00 | 83 | 1,571 |
| 10 | 2 | no (0), 0.016 | no | 743 (776) | 351 (343) | 0.000 | 0 / 10 (0 / 10) | 15–66 | 2 (2) | 2 (2) | 1.00 | 66 | 1,620 |
| 10 | 3 | no (0), 0.024 | no | 784 (762) | 347 (342) | 0.0006 | 0 / 10 (0 / 10) | 17–84 | 3 (3) | 7 (3) | 1.00 | 1,510 | 1,524 |
| 10 | 4 | no (4), 0.196 | no | 875 (688) | 345 (341) | 0.000 | 0 / 10 (0 / 10) | 16–46 | 10 (10) | 37 (24) | **0.00** | alive (2) | 1,551 |
| 10 | 5 | no (3), 0.223 | no | 741 (729) | 322 (327) | 0.000 | 0 / 10 (0 / 10) | 16–50 | 2 (2) | 2 (6) | 1.00 | 88 | 1,404 |
| 5 | 1 | no (0), 0.026 | no | 697 (758) | 337 (364) | 0.000 | 3 / 2 (3 / 2) | 53–550 | 4 (5) | 7 (7) | 1.00 | 862 | 1,925 |
| 5 | 2 | **yes (7), 0.218** | **yes** | 746 (701) | 345 (332) | 0.0011 | 5 / 0 (5 / 0) | 71–487 | 4 (5) | 32 (20) | **0.00** | 2,124 | 1,818 |
| 5 | 3 | no (0), 0.005 | no | 704 (702) | 334 (346) | 0.013 | 4 / 1 (4 / 1) | 72–550 | 4 (1) | 15 (7) | **0.50** | 1,847 | 1,628 |
| 5 | 4 | no (0), 0.019 | no | 697 (664) | 344 (341) | 0.0095 | 4 / 1 (5 / 0) | 132–558 | 8 (10) | 16 (24) | 1.00 | 1,791 | 1,563 |
| 5 | 5 | no (0), 0.015 | no | 739 (772) | 307 (354) | 0.0007 | 4 / 1 (5 / 0) | 116–588 | 5 (1) | 13 (1) | 1.00 | 1,643 | 1,691 |
