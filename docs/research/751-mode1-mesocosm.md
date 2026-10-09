# Issue #751: the mode-1 mesocosm's first runs

**Status: measurement, docs only (2026-10-09). The instrument is `reference_mode`
(`crates/explorers-search/src/bin/reference_mode.rs`, construction in
`explorers_search::mesocosm`). Every run here used the committed `recipe.json`, 1,500 or 3,000
ticks, seeds 1–3. These are the first readings of the mesocosm as built. They are not a
calibration: the wear rate is calibrated in #753.**

## TL;DR

The mesocosm builds and runs in seconds. As built, it does not yet hold the case mode 1 needs:

- **The decomposer founders die out at once.** At tick 50 the world-wide income read finds no
  decomposers (seed 1, wear 0.2), and at most one is alive at tick 3,000 in any run below. The
  likely cause, not yet read off their income, is that no carcasses stand at tick 0, so they
  find no food. Producers' carcasses then pile up undrained. In the centre cell, carcass
  nutrient climbs to 600–1,000 while the available pool sits near 0.
- **The centre cell does not persist as a control.** World-wide the mesocosm holds 9–23
  producers at tick 3,000 for wear rates up to 0.2, under one per nutrient cell, each with a
  structure of hundreds. The centre cell holds 0–6 producers and is empty on some samples in
  the second half of every run.
- **Wear has an edge, not a gradient.** Under the committed repair law (flow 9), repair is
  `kappa × exp(−repair_decay × wear)` a tick, paid from the soma budget. So while
  `wear_rate × autotrophy < kappa`, an agent with surplus repairs all of each tick's wear, and
  wear only costs it energy. Above that, repair falls as wear grows, wear runs away, and every
  producer dies at one fixed age. For the founder producers (kappa 0.325, autotrophy 1.073) the
  edge is at about 0.30:

| wear rate | producer lifespans (median, max) | outcome |
|---|---|---|
| 0.01 – 0.2 | 9–10, 2,200–2,900 | indistinguishable: founders live past 1,300 ticks, offspring mostly die within 20 |
| 0.28 – 0.29 | 9–11, 1,700–2,900 | fewer births; 1–7 producers world-wide at tick 3,000 |
| 0.35 | 289 (all 47 founder deaths within 288–294) | every founder dies before reproducing, extinct |
| 0.5 | 204 (203–209) | as 0.35 |
| 0.8 | 131 (129–136) | as 0.35 |

The design's target, producer lifespans of a few hundred ticks with a cohort that reproduces
(reference-modes.md, *Mesocosm*, *Wear rate*), sits on the far side of the edge, where no
producer reproduces. Below the edge a producer's lifespan does not follow from the wear rate at
all. The median lifespan below the edge, about 10 ticks, is seedling mortality: most offspring die
within 20 ticks of birth, whatever the wear rate.

The default wear rate is provisionally 0.2, below the edge. Choosing the real one, and whether
the mesocosm needs a different founder design to keep its decomposers, is for #753 and the
owner. Both are posted as questions on #751.
