# Issue #663: the atlas searched under ratio retention, `b` in the box

**Status: data regeneration. Replaces the committed `atlas.json` and `recipe.json` with a search
under the current physics: #652's retention rule (a consumer keeps its nutrient ratio × the
energy a bite gains it) and #653's box, which searches `b` (`uptake_structure_exponent`) over
[0, 1]. The new atlas records its 33-dimension box. It is the atlas #656 searched and read
([656](656-fresh-atlas-verdict.md)), reproduced on current `main`. Re-pins what reads the committed
files. Changes no stepper, evaluator or search behaviour.**

## TL;DR

1. **Seed 42 reproduces #656's atlas exactly.** On `50cd0af` the search writes the same cells,
   box and recipe as #656's run on `68e515b`. Its fingerprint is `9c79856550a0151e`.
2. **A second seed (43) agrees on everything but where in the box the cells sit.** It finds 80
   cells against 79, a QD-score of 18.97 against 18.54, and the same dead frontier, led by
   nutrient lockup. 50 behaviour cells are common to both.
3. **`b` is not where #656 put it.** Seed 43's median `b` is 0.47 against seed 42's 0.63, and it
   has cells below 0.1 (5) and at the box edges (0.00 and 1.00). The shift is significant
   (Mann–Whitney p = 0.0003), but 5 other dimensions of the 33 shift as strongly between the
   seeds (p < 0.01), so it is lineage clustering, not selection. #656 §2 read the missing low tail
   as "probably weak selection away from `b ≈ 0`". Seed 43 does not support that.
4. **Seed 42 is committed.** The second seed does not show it to be unrepresentative of what
   genesis finds: cell count, coverage, QD-score, fitness and failure modes agree. Every #656
   readout ran on these exact worlds, so committing them keeps those readouts on the baseline.

## Runs

| | |
|---|---|
| Tree | `50cd0af` (main), release build of `explorers-search` |
| Command | `explorers-search --batch 32 --generations 10 --ensemble 5 --max-ticks 2000 --seed <S> --output … --recipe-output … --checkpoint …`, the settings of #494 and #656. Defaults otherwise: bloom stop at tick 300 × 10, refinement of the top 10 at n = 32 |
| Seeds | 42 (committed), 43 (comparison) |
| Wall clock | 1119 s (seed 42), 1142 s (seed 43), one at a time on an 8-core laptop. #656's seed-42 run took 766 s for the same output |
| Rollouts skipped | 0 on both. The rollout budget is wall-clock (600 s simulation, 600 s evaluation); no rollout reached it, so the atlas reproduces unbudgeted |
| Committed atlas | 79 cells; 33-dimension box with `uptake_structure_exponent` [0, 1] last; provenance seed 42, `max_ticks` 2000, bloom stop 300 × 10; fingerprint `9c79856550a0151e` (pinned in `tests/atlas_search_box.rs`) |
| Committed recipe | atlas:39, cell [5, 19, 7]: recorded fitness 0.571, refined 0.428 at n = 32 (coexistence 1.00 → 0.53), `b` = 0.666 |

`jq -S .cells`, `.search_box` and `recipe.json` are byte-identical to #656's
`target/656/{atlas,recipe}.json`.

## Seed 42 against seed 43

| | seed 42 | seed 43 |
|---|---:|---:|
| live cells (coverage of 8000) | 79 | 80 |
| QD-score | 18.54 | 18.97 |
| best fitness | 0.571 | 0.516 |
| median cell fitness | 0.215 | 0.275 |
| mean coexistence fraction | 0.55 | 0.53 |
| top-10 cells clearing the plain refinement floor | 9 | 8 |
| best refined fitness | 0.436 ([7, 19, 0]) | 0.410 ([6, 15, 4]) |
| recipe cell (`b`) | [5, 19, 7] (0.666) | [7, 19, 3] (0.976) |
| dead: nutrient lockup | 35 | 41 |
| dead: extinction | 5 | 10 |
| dead: bloom stop | 6 | 5 |
| dead: monoculture | 6 | 1 |
| dead: energy death | 2 | 5 |
| dead: total | 54 | 62 |
| behaviour cells in common | 50 | 50 |

`b` across the cells, decoded over each atlas's recorded box (#656's python step):

| seed | min | p25 | median | p75 | max | mean |
|---|---:|---:|---:|---:|---:|---:|
| 42 | 0.101 | 0.502 | 0.627 | 0.759 | 0.962 | 0.622 |
| 43 | 0.000 | 0.346 | 0.473 | 0.689 | 1.000 | 0.500 |

| `b` in | [0, 0.1) | [0.1, 0.25) | [0.25, 0.5) | [0.5, 0.75) | [0.75, 0.9) | [0.9, 1] |
|---|---:|---:|---:|---:|---:|---:|
| seed 42 | 0 | 2 | 15 | 41 | 19 | 2 |
| seed 43 | 5 | 10 | 27 | 25 | 7 | 6 |

- **The behaviour map agrees.** Both searches fill the same two clustering bins (0 and 19 hold
  69 of 79 cells and 69 of 80), find about 80 cells, and die mainly of lockup. Seed 43's frontier has more
  extinctions and fewer monocultures. The counts are small.
- **Parameter medians are a property of the search draw.** Cells within one search descend from
  a few emitter lineages, so they cluster. Between the two seeds the medians of 6 of 33
  dimensions differ at p < 0.01: `b`, base metabolic rate, movement cost, mutation rate,
  trophic distance decay and reproduction efficiency. `b` is one of these, and not the largest.
- **What this means for #656.** Its claims about `b` *within* the atlas (§2's shape, §3's
  per-config ρ against `b`) describe seed 42's draw. Its readouts did not depend on `b`: the
  per-config niche share was unrelated to `b` (§4). A readout on seed 43's worlds would test that.
  It was not run here.

## Pins moved

- **`evaluator_pin`** (`crates/explorers-search/tests/evaluator_pin.rs`). The cell indices and
  every golden are new, because the atlas was replaced. The cells were re-chosen from a scan of
  all 79 cells × seeds 1000–1001 at the 500-tick horizon. `CELLS` = 27 (consumer shares on one
  seed, decomposer shares on the other), 39 (monoculture on both seeds), 50 (both heterotroph
  shares live on seed 1000), 59 (a monoculture on seed 1001 that still carries shares), 71
  (generalist dominance on 1001, live on 1000), 77 (top-fitness seed, full coexistence on 1000),
  78 (lowest-fitness live seed, 1001). At 500 ticks no cell of this atlas locks up, goes extinct
  or holds a guild on either seed, so the old spread's lockup, extinction and guild entries have
  no counterpart.
- **`guild_anchor`** (`crates/explorers-genesis/tests/guild_anchor.rs`). The values are new
  because `recipe.json` was replaced: a different world, not a changed read. Seeds 1 and 3 lock up
  (ticks 1600 and 1800) and read zero on every column. Seed 2 reaches the horizon and carries the
  pin.
- **`atlas_search_box`** (`crates/explorers-search/tests/atlas_search_box.rs`). Its two tests
  that read the committed atlas as legacy (no box, 32-dimension units, no heterotroph shares)
  now read a legacy atlas made from the committed one by stripping those three things. A new
  test pins that the committed atlas records the 33-dimension box, decodes each cell over it,
  and has fingerprint `9c79856550a0151e`.
- **Unchanged and still passing on the new files:** `prefilter_atlas` (every live cell clears
  the energy-death gate), the `atlas:0` rollouts in `energy_death_check` and `settling_time`,
  `radius_sweep`'s `atlas.json` / `recipe.json` defaults, and the app's two `recipe.json` tests.
- **`kin_killer_diet`'s #642 reference** stays keyed to the old atlas (`e12caad9b8f2a5a2`). Rows
  on the new atlas print "no reference (different atlas)". `--summary` still re-reads
  `target/645`, `target/655` and `target/656` rows, with the #642 comparison where it applied
  before. `role_diet_census --summary --fullness-region` still re-reads `target/637`, `655` and
  `656`. #656's persistence (decoded 355 / 395, lockup 17; fa 0 363 / 395, lockup 18) was not
  added as a second reference.

## What this does not show

- **Two seeds, 10 generations.** Two draws are enough to show that `b`'s distribution varies
  between searches. They do not show what a longer or wider search converges to.
- **No readouts on seed 43.** Whether #647's two tests read the same on seed 43's worlds is not
  measured.
