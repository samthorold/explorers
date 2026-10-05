# Issue #677: the atlas searched under the deficit rule, `c_AH` in the box

**Status: data regeneration (2026-10-05). Replaces the committed `atlas.json` and `recipe.json`
with #670's seed-42 search: the flow-3 deficit rule (#666) and #669's box, which searches the
cross-trait cost `c_AH` (`cross_trait_cost`) linearly over [0, 0.14]. The new atlas records its
34-dimension box. Every #670 readout on seed 42 ([670](670-cross-trait-verdict.md)) ran on these
exact worlds. Re-pins what reads the committed files. Changes no stepper, evaluator or search
behaviour.**

## TL;DR

1. **The committed atlas is #670's seed 42, byte for byte.** `atlas.json` and `recipe.json` are
   copies of `target/670/atlas-seed42.json` and `recipe-seed42.json` (same `shasum`). It has 99
   cells over the full 34-dimension box. Its fingerprint is `aa2662b26da489a3`.
2. **It reproduces.** No rollout reached the wall-clock budget, so a rerun of the command below on
   `7fc7fb5` writes the same atlas.
3. **The recipe now sets `c_AH` ≈ 0.139**, near the top of the box, and `b` ≈ 0.717. The
   `WorldParameters` default for `c_AH` stays 0, so only worlds that set it pay it.
4. **Seed 43 is not a candidate.** It found 96 cells and a higher QD-score, but 7 of its rollouts
   ran out of wall clock, so another machine can write another atlas.

## Runs

| | |
|---|---|
| Tree | `7fc7fb5` (main), release build of `explorers-search`, pinned in `target/670/bin` |
| Command | `explorers-search --batch 32 --generations 10 --ensemble 5 --max-ticks 2000 --seed 42 --output … --recipe-output … --checkpoint …`, the settings of #494, #656 and #663. Defaults otherwise: bloom stop at tick 300 × 10, refinement of the top 10 at n = 32 |
| Wall clock | 824 s (generations 12 m 30 s, then refinement). Log: `target/670/search-seed42.log` |
| Rollouts skipped / unfinished | 0 / 0. The rollout budget is wall-clock (600 s simulation, 600 s evaluation), and no rollout reached it, so the atlas reproduces unbudgeted |
| Committed atlas | 99 cells (coverage 99 / 8000); QD-score 21.30; best fitness 0.446; median cell fitness 0.191; 34-dimension box with `uptake_structure_exponent` [0, 1] and `cross_trait_cost` [0, 0.14] last; provenance seed 42, `max_ticks` 2000, bloom stop 300 × 10; fingerprint `aa2662b26da489a3` (pinned in `tests/atlas_search_box.rs`) |
| Dead frontier | 64: nutrient lockup 42, extinction 11, monoculture 6, energy death 3, bloom stop 2 |
| Committed recipe | atlas:90, cell [9, 19, 6]: recorded fitness 0.446, refined 0.297 at n = 32 (coexistence 0.60 → 0.62), `b` = 0.717, `c_AH` = 0.139. The best refined fitness is 0.401 ([7, 19, 3]). The recipe is the top recorded cell that clears the plain floor, as before |

`c_AH` and `b` across the cells, decoded over the recorded box:

| | min | p25 | median | p75 | max | mean |
|---|---:|---:|---:|---:|---:|---:|
| `c_AH` | 0.017 | 0.056 | 0.082 | 0.105 | 0.140 | 0.081 |
| `b` | 0.058 | 0.441 | 0.578 | 0.779 | 1.000 | 0.591 |

## Against the previous committed atlas and seed 43

| | previous (#663) | **committed (#677)** | #670 seed 43 |
|---|---:|---:|---:|
| tree, physics | `50cd0af`, ratio retention, no `c_AH` | `7fc7fb5`, deficit rule, `c_AH` searched | same as #677 |
| box | 33 dims (untaxed) | 34 dims (full) | 34 dims (full) |
| live cells | 79 | 99 | 96 |
| QD-score | 18.54 | 21.30 | 25.20 |
| best fitness | 0.571 | 0.446 | 0.551 |
| median cell fitness | 0.215 | 0.191 | 0.316 |
| mean coexistence fraction | 0.55 | 0.51 | 0.58 |
| cells in clustering bins 0 and 19 | 69 | 90 | 78 |
| dead: lockup / extinction / monoculture / energy death / bloom stop | 35 / 5 / 6 / 2 / 6 | 42 / 11 / 6 / 3 / 2 | 35 / 5 / 10 / 0 / 8 |
| median `b` | 0.627 | 0.578 | 0.621 |
| median `c_AH` | 0 (not searched) | 0.082 | 0.075 |
| rollouts unfinished | 0 | 0 | **7** |
| fingerprint | `9c79856550a0151e` | `aa2662b26da489a3` | `e03608050c196850` |
| behaviour cells in common with #677 | 53 | 99 | 55 |

- **Against the previous atlas.** The searches ran under different physics, so the worlds are not
  comparable one for one. The new atlas fills 20 more cells, mostly in the same two clustering bins.
  Its dead frontier is still led by lockup, and it has more extinctions.
- **Against seed 43.** Seed 43 scores higher on QD-score and fitness. But wall clock decided which of
  its rollouts entered the archive, so its atlas is one machine's draw. The two seeds agree on what a
  baseline needs: cell count, a dead frontier led by lockup, persistence, and the readouts' verdicts
  ([670](670-cross-trait-verdict.md) §1, §10).

The readouts on these worlds (conditionality, carcass shares, persistence, the `(k, τ)` region) are
in [670-cross-trait-verdict.md](670-cross-trait-verdict.md), seed 42 rows.

## Pins moved

- **`evaluator_pin`** (`crates/explorers-search/tests/evaluator_pin.rs`). The cell indices and
  every golden are new, because the atlas was replaced. The cells were re-chosen from a scan of
  all 99 cells × seeds 1000–1001 at the 500-tick horizon. `CELLS` = 15 (a monoculture on seed 1000
  at the box's top `c_AH`, 0.140, live on 1001), 21 (decomposer shares on seed 1000 only), 22 (the
  lowest-fitness live seed, 1000), 27 (generalist dominance on 1001, live on 1000), 49 (a monoculture
  on both seeds that still carries decomposer shares on 1001), 66 (decomposer shares on one seed,
  consumer shares on the other), 69 (both heterotroph shares live on both seeds), 80 (the top-fitness
  seed, 1001). At 500 ticks no cell of this atlas locks up, goes extinct or holds a guild on either
  seed, so none is pinned. The reused index 27 is a new cell.
- **`guild_anchor`** (`crates/explorers-genesis/tests/guild_anchor.rs`). The values are new
  because `recipe.json` was replaced: a different world, not a changed read. Seeds 1 and 3 reach
  the horizon. Seed 2 locks up at tick 1650 and reads zero.
- **`atlas_search_box`** (`crates/explorers-search/tests/atlas_search_box.rs`). The test that
  pinned the committed atlas's 33-dimension untaxed box now pins the full 34-dimension box
  (`default_ranges()`), fingerprint `aa2662b26da489a3`, and a nonzero `c_AH` on every cell. The
  untaxed case moves to a fixture: a new test cuts the committed atlas's box and units to 33
  dimensions in scratch and checks that each cell decodes over the untaxed box with `c_AH = 0`.
  The two legacy tests (no box, no heterotroph shares) still build their fixture from the committed
  atlas and cut its units to 32 dimensions.
- **Unchanged and still passing on the new files:** `prefilter_atlas`, the `atlas:0` rollouts in
  `energy_death_check` and `settling_time`, `radius_sweep`'s `atlas.json` / `recipe.json` defaults,
  and the app's `recipe.json` tests.
- **`kin_killer_diet`'s #642 reference** stays keyed to #642's atlas (`e12caad9b8f2a5a2`). Rows on
  the new atlas print "no reference (different atlas)", as rows on #663's did.

## What this does not show

- **One reproducible seed.** #670's two seeds show how far one draw can move the medians and the
  region. This atlas is one draw.
- **Nothing new is measured here.** The readouts are #670's.
