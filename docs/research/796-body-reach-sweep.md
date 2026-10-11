# Issue #796: the pre-registered body-reach sweep for mode 1

**Status: measurement (2026-10-11). The sweep pre-registered in `reference-modes.md`, mode 1, *Mesocosm → Parameters → Feeding reach*, was run and read by its fixed rule on the committed mesocosm (decay 0.60, #770 wear, #780 movement) at 1× litter. No parameter was changed: by the rule, the "many" points fail.**

## Verdict, by the fixed rule

A point passes when, in at least 3 of 5 seeds, the control arm's heterotroph-dominant decomposer guild holds *and* its centre-patch producers persist.

| k (pieces spanned) | b | Prediction | Seeds passing (readout window) | Seeds passing (run's second half) | Result |
|---:|---:|---|---:|---:|---|
| 0 | 0 | Baseline; no prediction | 0 of 5 | 0 of 5 | No guild in any seed. Producers persist in 5 of 5 |
| 1 | 0 (clamped) | Fails | 0 of 5 | 0 of 5 | **Fails, as predicted.** k = 1 clamps to b = 0, so it is the k = 0 world: every report is identical to k = 0's |
| 3 | 0.589 | Boundary; no prediction | 0 of 5 | 0 of 5 | No guild in any seed. Producers persist in 5 of 5 |
| 10 | 1.752 | Passes | 0 of 5 | 0 of 5 | **Prediction fails.** No guild in any seed. Producers collapse in 3 of 5 |
| 30 | 3.634 | Passes | 0 of 5 | 0 of 5 | **Prediction fails.** No guild in any seed. Producers collapse in 5 of 5, and the whole world dies out in 2 |

**The row that applies: 10 and 30 both fail.** Reach is not, on its own, the binding cause. No parameter changes, and the body-reach coefficient stays at 0. As the design orders, **per-carcass pay is grilled next, then lean-time reserve.** Those grills are for the owner.

**The missing-defence gap is flagged** (below, *Supplementary*), although neither of its literal triggers fires. No point has a guild, so no guild holds while its producers collapse, and no guild drain exists to be mostly living. But at 10 and 30 pieces the decomposer founders kill most of the founding producer stand in the first 20 ticks. Of the 162 founding producers and their first offspring, 100–112 die after a heterotroph-dominant agent drained them at k = 10, and 156–161 at k = 30, against 24–36 at k = 0. That is the case the design names: reach turning decomposers into grazers (*Why producers must persist too*: "It is grilled if the sweep shows reach turning decomposers into grazers").

The verdict does not depend on how "the second half" is read (below, *The guild's window*). Both readings give 0 passes at every point, and the guild falls to 0 members over either window at every point and seed.

## Method

Pre-registered in `reference-modes.md` (merged in #793) and restated in #796:
- **Runs.** Each seed and point ran paired, as #781's did: `reference_mode --seed S --clear-at 1500 --ticks 1500 --body-reach-pieces k`. It settles 1,500 ticks, forks a control and a perturbed arm, and runs each to tick 3,000. The points are k = 0, 1, 3, 10 and 30, seeds 1–5, on the committed `mode1.json` at 1× litter, release build. k = 0 passes the option as 0, a no-op, so it is today's world: the same-code baseline #781 lacked.
- **The coefficient.** `--body-reach-pieces k` (#795) sets flow 3's body-reach coefficient so that a decomposer founder's reach at founding spans k litter pieces: `b = max(0, (√(k / (π ρ)) / h − c) / √S)` with h 1.073, S 2.459, c 1.284 and ρ 0.170. The founder's reach is 1.378 (1.01 pieces) at b = 0, then 2.37, 4.33 and 7.50 at k = 3, 10 and 30. k = 1 clamps to 0. The coefficient is a world parameter, so it applies to every agent's reach, through its own heterotrophy and structure, not to the founders alone.
- **The reading.** The control arm, read to tick 3,000, from each run's `## Heterotroph-dominant decomposer guild` section (`guild` in the JSON, #794). The guild is the committed *heterotroph guild* predicate over agents with the decomposer income role whose heterotrophy exceeds their autotrophy: at least 5 members on every sample of the window, and a birth in it with a member as a parent. The producers persist when the centre patch holds producers on every sample of the window's half. The pass is both together.
- **Reported beside it,** from the same runs: founders alive at ticks 20, 50 and 100 (by id; these ticks are in the settle, so both arms share them); income per living founder-tick over ticks 5–20, as energy gained (drained structure × the trophic efficiency applied), with the drain in brackets (#781's definition); the guild's share of the carcass drain in the window; the guild's drain split between living targets and carcasses; the coefficient and whether it clamped; runtime.

### The guild's window

The design says the arm with decomposers "is read to tick 3,000", and the guild must hold on every sampled tick "of the second half". The readout (#794) reads the **arm's** second half: for the control arm of a paired run with `--ticks 1500`, guild samples at world ticks 2,275–3,000 (births from 2,251), and producers on samples 2,250–3,000. Read as the second half of the **run to tick 3,000**, the window is ticks 1,501–3,000, which is the whole of the control arm. The two differ.

Neither is picked silently. The table gives both. The second reading was computed from an unforked run of each point and seed, `reference_mode --seed S --ticks 3000 --body-reach-pieces k`, whose readout window is ticks 1,501–3,000 (producers from the sample at 1,500). The unforked run is the control arm: the control is a clone of the settled world, RNG included, and all 61 of the control's centre-patch samples (ticks 1,500–3,000, every field) equal the unforked run's at the same ticks, in all 25 pairs. Both windows give the same verdict at every point and seed, and in every seed the guild's fewest members is 0 in both.

## Results

The control arm, per point and seed. "Guild" is holds (fewest members) over the readout's window, the arm's second half. "Guild share" is the guild's share of all carcass drain in that window ("-" where nothing drained carcasses). "Guild drain" is living / carcass structure drained by guild members in that window, living share in brackets. "b" is the body-reach coefficient (clamped?). The three columns under "run's second half" are the same reading over ticks 1,501–3,000. Runtime is paired run / unforked run.

| k | seed | guild | producers persist | **pass** | founders alive at 20 / 50 / 100 | income 5–20: gained (drained) | guild share | guild drain: living / carcass (living share) | b (clamped) | guild, run's 2nd half | producers, run's 2nd half | pass, run's 2nd half | runtime |
|---:|---:|---|---|---|---|---:|---:|---:|---:|---|---|---|---:|
| 0 | 1 | no (0) | yes | **no** | 1 / 0 / 0 | 0.119 (0.371) | 0.031 | 20.2 / 747.7 (0.026) | 0 (n/a) | no (0) | yes | no | 22 s / 14 s |
| 0 | 2 | no (0) | yes | **no** | 1 / 0 / 0 | 0.139 (0.421) | 0.021 | 117.4 / 582.0 (0.168) | 0 (n/a) | no (0) | yes | no | 22 s / 14 s |
| 0 | 3 | no (0) | yes | **no** | 2 / 0 / 0 | 0.159 (0.488) | 0.005 | 41.3 / 121.0 (0.255) | 0 (n/a) | no (0) | yes | no | 21 s / 14 s |
| 0 | 4 | no (0) | yes | **no** | 1 / 0 / 0 | 0.129 (0.402) | 0.000 | 0.0 / 0.0 (-) | 0 (n/a) | no (0) | yes | no | 20 s / 13 s |
| 0 | 5 | no (0) | yes | **no** | 2 / 0 / 0 | 0.150 (0.445) | 0.000 | 4.6 / 4.6 (0.501) | 0 (n/a) | no (0) | yes | no | 20 s / 14 s |
| 1 | 1 | no (0) | yes | **no** | 1 / 0 / 0 | 0.119 (0.371) | 0.031 | 20.2 / 747.7 (0.026) | 0 (yes) | no (0) | yes | no | 22 s / 15 s |
| 1 | 2 | no (0) | yes | **no** | 1 / 0 / 0 | 0.139 (0.421) | 0.021 | 117.4 / 582.0 (0.168) | 0 (yes) | no (0) | yes | no | 23 s / 15 s |
| 1 | 3 | no (0) | yes | **no** | 2 / 0 / 0 | 0.159 (0.488) | 0.005 | 41.3 / 121.0 (0.255) | 0 (yes) | no (0) | yes | no | 22 s / 15 s |
| 1 | 4 | no (0) | yes | **no** | 1 / 0 / 0 | 0.129 (0.402) | 0.000 | 0.0 / 0.0 (-) | 0 (yes) | no (0) | yes | no | 21 s / 15 s |
| 1 | 5 | no (0) | yes | **no** | 2 / 0 / 0 | 0.150 (0.445) | 0.000 | 4.6 / 4.6 (0.501) | 0 (yes) | no (0) | yes | no | 20 s / 15 s |
| 3 | 1 | no (0) | yes | **no** | 0 / 0 / 0 | 0.092 (0.257) | 0.000 | 0.0 / 0.0 (-) | 0.589 (no) | no (0) | yes | no | 22 s / 16 s |
| 3 | 2 | no (0) | yes | **no** | 1 / 0 / 0 | 0.074 (0.190) | 0.000 | 0.0 / 0.0 (-) | 0.589 (no) | no (0) | yes | no | 20 s / 14 s |
| 3 | 3 | no (0) | yes | **no** | 0 / 0 / 0 | 0.084 (0.234) | 0.000 | 0.0 / 0.0 (-) | 0.589 (no) | no (0) | yes | no | 18 s / 13 s |
| 3 | 4 | no (0) | yes | **no** | 1 / 0 / 0 | 0.085 (0.239) | 0.000 | 0.0 / 0.0 (-) | 0.589 (no) | no (0) | yes | no | 18 s / 13 s |
| 3 | 5 | no (0) | yes | **no** | 0 / 0 / 0 | 0.108 (0.303) | 0.000 | 0.0 / 0.0 (-) | 0.589 (no) | no (0) | yes | no | 21 s / 15 s |
| 10 | 1 | no (0) | **no** | **no** | 7 / 0 / 0 | 0.244 (0.650) | 0.000 | 0.0 / 0.0 (-) | 1.752 (no) | no (0) | no | no | 11 s / 8 s |
| 10 | 2 | no (0) | **no** | **no** | 8 / 0 / 0 | 0.224 (0.554) | 0.000 | 0.0 / 0.0 (-) | 1.752 (no) | no (0) | no | no | 14 s / 10 s |
| 10 | 3 | no (0) | yes | **no** | 5 / 0 / 0 | 0.248 (0.650) | 0.000 | 0.0 / 0.0 (-) | 1.752 (no) | no (0) | yes | no | 18 s / 13 s |
| 10 | 4 | no (0) | yes | **no** | 10 / 0 / 0 | 0.241 (0.573) | 0.000 | 0.0 / 0.0 (-) | 1.752 (no) | no (0) | yes | no | 12 s / 9 s |
| 10 | 5 | no (0) | **no** | **no** | 11 / 0 / 0 | 0.254 (0.648) | 0.000 | 0.0 / 0.0 (-) | 1.752 (no) | no (0) | no | no | 15 s / 10 s |
| 30 | 1 | no (0) | **no** | **no** | 13 / 0 / 0 | 0.275 (0.500) | 0.000 | 0.0 / 0.0 (-) | 3.634 (no) | no (0) | no | no | 3 s / 3 s |
| 30 | 2 | no (0) | **no** | **no** | 9 / 0 / 0 | 0.254 (0.498) | 0.000 | 0.0 / 0.0 (-) | 3.634 (no) | no (0) | no | no | 3 s / 2 s |
| 30 | 3 | no (0) | **no** | **no** | 9 / 0 / 0 | 0.285 (0.523) | 0.000 | 0.0 / 0.0 (-) | 3.634 (no) | no (0) | no | no | 2 s / 2 s |
| 30 | 4 | no (0) | **no** | **no** | 12 / 0 / 0 | 0.251 (0.482) | - | 0.0 / 0.0 (-) | 3.634 (no) | no (0) | no | no | 2 s / 2 s |
| 30 | 5 | no (0) | **no** | **no** | 4 / 0 / 0 | 0.137 (0.237) | - | 0.0 / 0.0 (-) | 3.634 (no) | no (0) | no | no | 2 s / 2 s |

**Row assignment.**
1. **k = 1.** The coefficient clamps to 0, since the contact-only reach already spans 1.01 pieces, and every report equals k = 0's. 0 of 5 pass, so the prediction "fails" holds.
2. **k = 10.** 0 of 5 pass: the guild falls to 0 members in every seed, and producers collapse in seeds 1, 2 and 5. The prediction "passes" fails.
3. **k = 30.** 0 of 5 pass: no guild, and producers collapse in every seed. The prediction fails.
4. So **10 and 30 both fail**, and the issue's second row applies: reach is not, on its own, the binding cause, no parameter changes, and per-carcass pay is grilled next, then reserve.

The k = 0 baseline reproduces #781's 1× founder figures exactly (alive at 20, income 0.119–0.159), as #797's bit-identical speed-up says it should.

## Supplementary (outside the pre-registration)

These come from a throwaway probe, an example under `explorers-reference-modes` that is not committed (as #764's `founder_probe` was not). It built the same worlds (`mode1_mesocosm` with the committed spec, then `set_body_reach_pieces(k)`), stepped them, and read every `Consumed` and `Died` event. Drains are booked by the drainer's nominal traits: heterotroph-dominant (heterotrophy > autotrophy) or autotroph-dominant. Targets are a living agent of either kind or a carcass. As in the readout, a drain is structure drained, before efficiency.

- **At many pieces, the decomposer founders kill the founding producer stand.** Producer deaths in ticks 1–20, and how many of the dead had been drained by a heterotroph-dominant agent first:

  | k | producer deaths, ticks 1–20 (162 producers at founding) | of them, drained first | producers alive at tick 10 |
  |---:|---:|---:|---:|
  | 0 | 28–36 | 24–36 | 168–176 |
  | 3 | 43–54 | 38–53 | 136–160 |
  | 10 | 104–114 | 100–112 | 72–78 |
  | 30 | 156–161 | 156–161 | 2–10 |

  At k = 30, the world holds at most 19 producers at ticks 10, 20 and 100 in every seed. The world never recovers: at tick 3,000 the controls hold 0–6 producers, and seeds 4 and 5 are extinct.
- **By drain share, the founders are still saprotrophs.** The living share of what heterotroph-dominant agents drained in ticks 1–20 is 0.04–0.07 at every point, 0.061–0.066 at k = 30. A share of the drain does not read the harm. The ~280–300 structure the k = 30 founders took from living producers is about 2 a producer, and it killed nearly all of them. So the guild-drain share the design names would not have flagged this collapse, even had a guild held.
- **This is the missing-defence gap.** Feeding reach is one predicate for living and dead targets, and a living producer is spared only by recognition. A pure decomposer and a pure producer are far apart in trait space, so recognition spares nothing. A reach spanning ten or thirty pieces of litter also spans the producers among them, as the design's *Why producers must persist too* predicted. The design says the defence gap "is grilled if the sweep shows reach turning decomposers into grazers". The sweep shows it at 10 and 30 pieces. The owner decides how this grill orders against pay and reserve.
- **More reach exhausts the founding litter sooner; it does not feed a line.** Heterotroph-dominant agents drained 636 of carcass structure in ticks 1–20 at k = 0 (seed 1), 1,035 at k = 3, 3,088 at k = 10 and 4,593 at k = 30. The founding stand holds 81 × 54 = 4,374 energy, plus whatever the grazed producers left as carcasses. At k = 30 it is gone within 20 ticks. Founders alive at tick 20 rise with reach (1–2 at k = 0, 4–13 at k = 30), but no founder lives past tick 33 at any point, and the founders' line at 3,000 is 0 everywhere but k = 10 seed 1 (5). As at 10× litter in #781, the founders die as the stock runs out. Without a renewed flux of producer deaths, reach front-loads consumption of a fixed stock.
- **At k = 3, founder income per living founder-tick over ticks 5–20 falls** to 0.074–0.108, below k = 0's 0.119–0.159, and founders die slightly sooner (0–1 alive at tick 20). The likely cause is that overlapping discs empty the stand faster, so the ticks 5–20 window catches the founders after their early feast. This was not tested. At k = 10 and 30 income rises to 0.22–0.29, except k = 30 seed 5 (0.137).
- **A few heterotroph-dominant agents hang on at large reach.** At k = 10, 4–10 stand from tick 100 to tick 1,000 in seeds 1 and 3, near the guild's floor of 5. At k = 30 seed 4, 5 stand at tick 100, falling to 1 by tick 1,500. None is a guild member on any sample after tick 1,500. Elsewhere at most 1 stands at tick 100 and none later.
- **Producer-line mixotrophs still carry the carcass drain at small reach.** Agents with autotrophy ≥ 0.1 take 0.98–0.99 of the control's whole-horizon carcass drain at k = 0 and 3, as in #772 and #781, and 0.90–0.97 at k = 10. At k = 30 they take 0–0.63: the founders drained the founding stand before the producers died out, and little litter followed.

## Runtime

Release build, five at a time on 8 cores. A paired run (`--clear-at 1500 --ticks 1500`, 4,500 ticks) took 18–23 s at k = 0, 1 and 3, 11–18 s at k = 10, and 2–3 s at k = 30. The unforked 3,000-tick runs took 13–16 s, 8–13 s and 2–3 s. Larger reach did not cost more: it collapses the producer stand, so the world holds less to step. The 25 paired runs took about 80 s of wall-clock, and the 25 unforked runs about 60 s.

## Reproducing

```sh
cargo build --release -p explorers-reference-modes --bin reference_mode
# per point and seed (k = 0, 1, 3, 10, 30; s = 1..5), five at a time:
target/release/reference_mode --seed $s --clear-at 1500 --ticks 1500 --body-reach-pieces $k \
  --out target/796/runs/k$k-s$s.json
# the run's-second-half reading (ticks 1,501-3,000), from the unforked run:
target/release/reference_mode --seed $s --ticks 3000 --body-reach-pieces $k \
  --out target/796/straight/k$k-s$s.json
```

Each `.md` summary beside its JSON carries, per arm, the *Feeding reach* section (k, b, the founders' reach and the clamp), the guild section and the *Founders* table. Income over ticks 5–20 is computed from the JSON's `founders.gained` and `founders.drained` (the step ending at tick t is entry t − 1), over the founders alive as each step began.
