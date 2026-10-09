# Issue #753: calibrating the mode-1 mesocosm's wear rate

**Status: measurement, docs only (2026-10-09). The instrument is `reference_mode`
(`crates/explorers-search/src/bin/reference_mode.rs`), built by #751
([751-mode1-mesocosm.md](751-mode1-mesocosm.md)). Every run used the committed `recipe.json`,
3,000 ticks, a sample every 25 ticks, 26 wear rates and seeds 1–5 (seeds 1–10 at 0.2 and 0.25).
No code changed apart from a doc comment.**

## TL;DR

**No wear rate meets both criteria, and the default stays at the provisional 0.2.** Wear stays on
(world-rules.md, *Wear is always on*).

- **The control never persists reliably at any rate.** Below the wear edge (≤ 0.25), the centre
  cell holds producers in about 25–50 % of second-half samples, and through the whole second half
  in at most 1 run in 5. Near the edge (0.26–0.313) it is almost always empty. Above the edge
  (≥ 0.314) the world goes extinct.
- **Read over all producers, the median lifespan is never a few hundred ticks in a living
  world.** Below the edge it is 9–11 ticks (seedling mortality) at every rate. It is a few hundred
  ticks only above the edge, where every producer dies at one age before reproducing: 322 ticks
  at 0.314, 289 at 0.35, 204 at 0.5.
- **Read over producers past the seedling stage (lifespan > 50 ticks), there is a window.** At
  0.24–0.25 the median is 126–133 ticks, below the 277-tick leaching half-life. At 0.2 and below
  it is 450–520 ticks, above the half-life.
- **The closest candidates are 0.24–0.25.** They meet the lifespan criterion under the
  past-seedling reading, but the control persists no better than at 0.2 (0.25: 2 runs in 10
  through the whole second half; 0.2: 1 in 10). The window is narrow: at 0.26 the centre is
  occupied in only 9 % of second-half samples.
- **The default stays at 0.2.** The candidates are not clearly better by the design's criteria:
  the control, the qualification for a known state, fails at both, and 0.25 is one step from
  where the centre empties. The blocker is persistence, and it looks structural. The centre cell
  holds 0–6 producers, and the decomposer founders die out at once. Both are the open design
  questions on #751. Re-run this sweep once those are settled, starting with 0.24–0.25.

## Definitions

- **Wear edge.** Under the committed repair law (flow 9), repair is
  `kappa × exp(−repair_decay × wear)` a tick. While `wear_rate × autotrophy < kappa` an agent with
  surplus repairs all its wear. Above that, wear runs away and every producer dies at one fixed
  age (#751). For the founders the observed edge is between 0.313 and 0.314. That is where every
  founder starts to die before reproducing, in all five seeds.
- **Producer lifespan.** The age at death of every agent the income read called a producer when
  it died, world-wide, over the run: from tick 0 for a founder, from birth for an offspring. The
  report's `producer_lifespans`. Medians are nearest-rank, over the five seeds' deaths pooled.
- **Median lifespan, all.** The median of that list. This is the issue's stated criterion read
  literally. Below the edge it is dominated by offspring that die within 20 ticks of birth.
- **Median lifespan, past seedling.** The median over deaths at ages above 50 ticks. It is
  labelled separately because it is not the issue's literal criterion. It reads the cohort that
  holds a patch, which is what the design's criterion is about (cohort turnover against the
  carcass return), so this note uses it to name the candidates.
- **Leaching half-life.** `ln 2 / 0.0025 ≈ 277` ticks at the recipe's leaching rate.
- **Control persists.** The centre cell is the 13th of the 25 nutrient cells, the patch mode 1
  clears. The control persists in a run when the centre holds at least one producer at every
  25-tick sample in the second half of the run (ticks 1,500–3,000). That window is longer than any
  candidate lifespan, so it covers "at least one cohort lifespan" after settling. Two weaker
  readings are reported beside it. One is the share of second-half samples with producers in the
  centre. The other is whether the centre has an unbroken occupied stretch at least as long as
  the past-seedling median within the second half.

## The sweep

Run as below, 8 at a time, artifacts under `target/reference-mode/753*/`. Medians are pooled over
seeds. "Producers at 3,000" is the world-wide count per seed, in seed order.

```sh
for w in <rates>; do for s in 1 2 3 4 5; do echo "$w $s"; done; done |
  xargs -P 8 -n 2 sh -c 'target/agent/release/reference_mode --wear-rate $0 --seed $1 \
    --out target/reference-mode/753/w$0-s$1.json'
```

| wear rate | producer deaths | median lifespan, all | median, past seedling (> 50 ticks) | producers world-wide at 3,000 (per seed) | centre occupied, share of 2nd-half samples | occupied stretch ≥ past-seedling median | control persists (whole 2nd half) |
|---:|---:|---:|---:|---|---:|---:|---:|
| 0.01 | 8,927 | 10 | 479 | 18, 14, 9, 17, 6 | 0.27 | 1/5 | 0/5 |
| 0.05 | 8,538 | 10 | 496 | 10, 18, 10, 9, 8 | 0.33 | 1/5 | 0/5 |
| 0.1 | 8,579 | 10 | 493 | 10, 23, 10, 7, 12 | 0.36 | 2/5 | 0/5 |
| 0.15 | 8,763 | 10 | 447 | 15, 10, 15, 8, 11 | 0.38 | 2/5 | 1/5 |
| **0.2** (default) | 17,823 | 10 | 496 | 16, 14, 11, 16, 8, 17, 14, 11, 8, 10 | 0.40 | 4/10 | 1/10 |
| 0.22 | 8,557 | 10 | 485 | 11, 9, 5, 13, 11 | 0.23 | 1/5 | 0/5 |
| **0.24** | 4,756 | 10 | **126** | 8, 11, 10, 13, 7 | 0.46 | 3/5 | 1/5 |
| **0.25** | 9,089 | 9 | **133** | 12, 11, 8, 11, 6, 14, 9, 14, 11, 9 | 0.39 | 5/10 | 2/10 |
| 0.26 | 3,578 | 10 | 186 | 9, 9, 14, 12, 9 | 0.09 | 1/5 | 0/5 |
| 0.27 | 3,039 | 10 | 117 | 12, 8, 7, 2, 9 | 0.04 | 1/5 | 0/5 |
| 0.28 | 1,915 | 10 | 162 | 6, 7, 5, 2, 1 | 0.00 | 0/5 | 0/5 |
| 0.29 | 1,188 | 11 | 245 | 1, 6, 3, 0, 6 | 0.08 | 1/5 | 0/5 |
| 0.295 | 1,042 | 11 | 235 | 3, 5, 3, 0, 1 | 0.00 | 0/5 | 0/5 |
| 0.3 | 1,225 | 10 | 332 | 2, 3, 3, 2, 7 | 0.00 | 0/5 | 0/5 |
| 0.305 | 1,206 | 10 | 291 | 3, 2, 3, 2, 8 | 0.26 | 1/5 | 1/5 |
| 0.31 | 816 | 18 | 328 | 1, 3, 1, 0, 0 | 0.00 | 0/5 | 0/5 |
| 0.311 | 1,050 | 10 | 328 | 2, 3, 3, 3, 4 | 0.00 | 0/5 | 0/5 |
| 0.312 | 722 | 27 | 324 | 1, 4, 2, 2, 0 | 0.00 | 0/5 | 0/5 |
| 0.313 | 1,357 | 10 | 322 | 7, 2, 2, 1, 6 | 0.20 | 1/5 | 1/5 |
| 0.314 | 236 | 322 | 322 | 0, 0, 0, 0, 0 | 0.00 | 0/5 | 0/5 |
| 0.315 | 236 | 321 | 321 | 0, 0, 0, 0, 0 | 0.00 | 0/5 | 0/5 |
| 0.32 | 236 | 316 | 316 | 0, 0, 0, 0, 0 | 0.00 | 0/5 | 0/5 |
| 0.33 | 236 | 306 | 306 | 0, 0, 0, 0, 0 | 0.00 | 0/5 | 0/5 |
| 0.34 | 236 | 298 | 298 | 0, 0, 0, 0, 0 | 0.00 | 0/5 | 0/5 |
| 0.35 | 236 | 289 | 289 | 0, 0, 0, 0, 0 | 0.00 | 0/5 | 0/5 |
| 0.5 | 236 | 204 | 204 | 0, 0, 0, 0, 0 | 0.00 | 0/5 | 0/5 |

What the table shows:

- **Three regimes.** At ≤ 0.22 the world holds 5–23 producers. Founders and a few offspring live
  for hundreds to thousands of ticks, and the past-seedling median sits at 450–520. At 0.24–0.313,
  births fall with the rate. The past-seedling median drops to 117–186 at 0.24–0.28 and climbs back
  towards the edge age (~320) by 0.30. The world thins to 0–8 producers, and the centre empties
  from 0.26 on. At ≥ 0.314, the 236 producer deaths (of 250 producer founders over five seeds) fall within a few ticks of one age, with no
  births, and the world is extinct.
- **The step between 0.22 and 0.24 is not explained here.** The founders' own edge is at
  0.313, so the founders still repair in full at 0.24. A likely reading, not checked, is that
  offspring carry mutated kappa and autotrophy, so each has its own edge, and from about 0.24 a
  large share of them sit above theirs. This needs a per-agent reading of the lifespans.
- **The extinct runs fix the edge lifespan.** Lifespan above the edge falls smoothly with the
  rate (322 at 0.314, 204 at 0.5). Rates from about 0.4 give a lifespan below the half-life, but
  there no producer ever reproduces.
- **Persistence varies more by seed than by rate below the edge.** At 0.2 and at 0.25, 10 seeds
  each, the centre is occupied in about 40 % of second-half samples, and the whole second half is
  occupied in 1–2 runs. That is consistent with the cause #751 named: the centre holds a handful
  of large producers, and when one dies the patch can stand empty.

## The choice

None. Of the closest candidates, 0.24–0.25 meet the lifespan criterion under the past-seedling
reading, and none meets the control criterion. The mesocosm's default stays at
`PROVISIONAL_WEAR_RATE = 0.2`, which is no worse on persistence and is not next to the collapse
at 0.26. Its doc comment now points here.

The sweep cannot fix persistence by tuning wear. #751's open questions decide what can: the
founder design (decomposers that survive the first ticks, and a patch that holds more than a few
producers) and the repair law's edge. Once those change, re-run this sweep first over 0.2–0.28.
