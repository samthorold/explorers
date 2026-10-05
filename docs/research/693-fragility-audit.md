# Issue #693: the fragility audit of the committed atlas

**Status: measurement, docs only (2026-10-05). Part (b) of the stepping-back research paused on
#662. For each of the committed atlas's 99 worlds, it measures how often the evaluator's verdict
flips across seeds and under small parameter jitter. No code, design or atlas change.**

## TL;DR

**The atlas holds two populations of worlds. Seed noise and parameter fragility are the same
property. The worlds the search rates highest lean fragile, and so do worlds with more of their
nutrient locked in carcasses.**

| | value |
|---|---|
| worlds whose 10 unperturbed seeds all agree on the verdict | 57 of 99 |
| their mean flip rate under jitter, r = 0.01 / 0.03 / 0.1 | 3.3 / 4.3 / 9.5 % |
| worlds where at least one seed already disagrees | 42 of 99 |
| their mean flip rate under jitter, r = 0.01 / 0.03 / 0.1 | 21.4 / 25.9 / 33.8 % |
| Spearman ρ, seed-only flip rate against flip rate at r = 0.01 (over worlds) | +0.80 |
| basin width 0 (flips ≥ 20 % already at r = 0.01) / 0.1 (< 20 % at every radius) | 22 / 53 worlds |
| seed-to-seed fitness sd of an unperturbed world, median | 0.149 |
| the atlas's median cell fitness | 0.191 |
| ρ of flip rate (r = 0.1) with cell fitness / with the carcass axis | +0.27 / +0.37 |
| strongest single parameter in the flip attribution | ρ = 0.08 (`trait_covariance`) |

1. **Fragility is a property of the world, not of where its parameters sit.** A world whose seeds
   all agree barely moves when its 34 parameters are jittered: 3 % of verdicts flip at r = 0.01
   and under 10 % at r = 0.1, a tenth of the box. A world where seeds already disagree flips a
   fifth to a third of the time at every radius. Jitter adds little to what seed noise already
   shows (ρ = +0.80). These worlds sit on a stochastic knife-edge: the same parameters live or die
   by the draw.
2. **The search cannot see this with 5 seeds, and its fitness ranking is mostly noise.** One
   world's fitness varies across seeds with a median sd of 0.149, against a median cell fitness of
   0.191. Five seeds give a standard error of about 0.07 on the cell's mean, a third of the value
   being ranked.
3. **The search's elites lean fragile.** Flip rate rises with cell fitness (ρ +0.22 to +0.27 over
   the three radii). Fitness rewards oscillation, turnover and coexistence, and the worlds that
   score high on those sit nearer the cliffs. With 5 noisy seeds, a fragile world that happens to
   draw well is kept.
4. **Fragility tracks carcass lockup.** Flip rate rises with the atlas's carcass axis (ρ +0.36 to
   +0.37), more strongly than with any other axis or with fitness. Worlds that hold more of their
   nutrient in carcasses are nearer the edge. This agrees with the literature review's first
   ranked mechanism: donor control is lost where the pile outruns its drainers (part (a) of
   the same research, the literature review of stability and resilience; Moore et al. 2004).
5. **No single parameter carries the fragility.** The strongest attribution is ρ = 0.08. The top
   few are founding conditions (`trait_covariance`, `mean_photosynthetic_absorption`) and contact
   and light geometry (`contact_range_coefficient`, `light_competition_radius`). Fragility is a
   property of a world's dynamics. It is not a knob to clamp.

## 1. What ran

| | |
|---|---|
| Tree | main `0ac88dc` (#694's `fragility_audit`), ungated with recognition restraint 1 (#684). Binary pinned in `target/fragility/bin` |
| Driver | `scripts/fragility-audit.sh`: committed `atlas.json`, all 99 cells, seeds 1000–1009, T = 2000, 8 draws at each radius 0.01, 0.03 and 0.1 in the atlas's unit space |
| Rollouts | the search's settings: `QdConfig::default()` evaluator, bloom stop 300:10, the search's 600 s + 600 s rollout budget |
| Size | 99 × 25 rows × 10 seeds = 24,750 rollouts. 0 unfinished, 0 gated by the prefilter |
| Wall clock | 1,842 s |
| Outputs | `target/fragility/{rows.jsonl,summary.md,summary.json}` |

**Definitions.** A *flip* is a (draw, seed) evaluation whose verdict (live, or which failure mode)
differs from the unperturbed world's verdict on the same seed. The *seed-only flip rate* is the
share of the unperturbed world's 10 seeds that differ from its modal verdict: the noise floor.
*Basin width* is the largest radius at which that radius and every smaller one flip under 20 %.

**Differences from the search** (stated in #694): seeds are 1000–1009, not the search's own, which
the atlas does not record. The verdict per world is the modal per-seed verdict, not the search's
reduction of its ensemble. The search's 5 % horizon cross-check of early stops is off, since it
never changes a verdict.

**A physics caveat.** The atlas was searched under the surplus gate at `satiation_sensitivity` 33
and recognition restraint ½. This audit reads it under the ungated physics on main (#684). Three
worlds (cells 22, 26, 43) are modally dead under the new physics. Some of the knife-edge may be
the physics change landing on worlds tuned for the old one. #686's re-search would remove that.

## 2. The two populations

Per world, seed-only flip rate against flip rate at each radius
(`target/fragility/summary.md` has every row):

| worlds | n | mean flip, r = 0.01 | r = 0.03 | r = 0.1 |
|---|---:|---:|---:|---:|
| seeds all agree | 57 | 3.3 % | 4.3 % | 9.5 % |
| at least one seed disagrees | 42 | 21.4 % | 25.9 % | 33.8 % |

Basin width over all 99: 53 worlds hold under 20 % at every radius, 15 to r = 0.03, 9 to
r = 0.01, and 22 already exceed 20 % at r = 0.01. Two worlds (18, 88) have no seed disagreement
but flip about a fifth of the time at r = 0.01. They are the only clear cases of a world whose
parameters sit near a boundary while its dynamics, at those parameters, do not.

## 3. Fitness noise

Across the 10 seeds of an unperturbed world, fitness has a median sd of 0.149 (interquartile
0.118–0.166). Jitter barely widens it: 0.149, 0.146 and 0.156 at the three radii. The atlas's
cell fitness has a median of 0.191 and a maximum of 0.446. A cell's archived fitness is a mean
over 5 seeds, so its standard error is about 0.149 / √5 ≈ 0.07. Neighbouring cells' fitness
differences, and the elites' margins over them, are mostly within that.

## 4. Where fragility sits

Spearman ρ over the 99 worlds:

| flip rate | oscillation axis | clustering axis | carcass axis | fitness |
|---|---:|---:|---:|---:|
| seed-only | −0.10 | +0.14 | +0.21 | +0.15 |
| r = 0.01 | −0.06 | +0.17 | +0.37 | +0.22 |
| r = 0.03 | −0.09 | +0.18 | +0.36 | +0.22 |
| r = 0.1 | −0.15 | +0.25 | +0.37 | +0.27 |

The carcass axis is the strongest correlate at every radius. Fitness comes next.

## 5. What this does not show

- **One atlas, read under physics it was not searched under** (§1).
- **Ten seeds.** A world with seed-only flip rate 0 on 10 seeds can still fail on the 11th. The
  95 % upper bound on its true failure rate is about 31 % (exact binomial).
- **Verdicts, not trajectories.** A flip between two failure modes counts the same as a flip
  between live and dead.
- **Correlation over 99 clustered worlds.** Cells descend from a few emitter lineages (#663), so the
  ρ values overstate the evidence. The carcass-axis relation is the same sign and similar size at
  every radius, which is some reassurance.
- **Attribution is per parameter and marginal.** An interaction between parameters would not show.

## 6. What it suggests

These are for the grill that follows #662's pause. None is decided here.

- **Score robustness, not a lucky draw.** A world's persisted fraction over more seeds, or its
  seed-only flip rate, is cheap and separates the two populations cleanly. The literature review
  recommends 20–50 seeds for basin stability.
- **Lockup is where the knife-edge is.** The audit and the literature point to the same place: the
  release of carcass nutrient.
- **Clamping parameters will not help.** No dimension carries the fragility. It is the worlds'
  dynamics.

## Reproduction

From the repo root, on `0ac88dc` or later: `scripts/fragility-audit.sh`. It is resumable and
prints `DONE`. §2's split and the seed-against-jitter ρ are computed from the per-cell table in
`target/fragility/summary.md`. The atlas's fitness distribution is `jq '.cells | map(.fitness)'
atlas.json`.
