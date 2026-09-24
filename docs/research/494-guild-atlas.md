# Issue #494 — the settled-horizon atlas, read for heterotroph guilds

**Status: data regeneration plus a measurement. Commits the regenerated `atlas.json` and
`recipe.json` (the new baseline at `T = 2000`, superseding [474](474-atlas-regen-post-fix.md)) and
adds the `guild_census` instrument (#544). Re-pins the two goldens that read the committed
files — `evaluator_pin` (cells re-chosen from a scan of the new atlas for the same spread of
verdicts) and `guild_anchor` (the new recipe). Changes no stepper, evaluator or search behaviour.
The decision this issue asks for — leave the guild reported, make it a binning axis, or fold it
into the coexistence floor — is set out at the end for a human; it is not taken here.**
Stepper, evaluator and search at `3465c6d` (after #486's per-agent trophic scoring, #527's
ungated guild read, #540's starvation fix).

## TL;DR

1. **The atlas holds no guild cells.** 0 / 95 live cells reach `decomposer_fraction ≥ 0.5` or
   `consumer_fraction ≥ 0.5`; none reach 1.0. Four cells carry a decomposer guild on one seed of
   five; one carries a consumer guild on one seed. Re-measured on an independent seed block the
   count is the same: 0 / 95.
2. **The search is not selecting against guilds among living worlds.** Of seeds that live to
   `T`, 4 / 427 (0.9 %) of atlas seeds and 6 / 695 (0.9 %) of unselected LHS seeds hold a
   decomposer guild. A settled, persisting community almost never holds one, wherever it comes
   from — the [519](519-guild-read-at-settled-horizon.md) finding, now on the whole sample.
3. **Where guilds do form, a gate usually zeroes the world.** Of the 21 LHS seeds with a
   decomposer guild, **12 are `monoculture` and 3 `generalist_dominance`**; only 6 live. Of the
   39 LHS seeds gated at `T` by those two gates, **15 (38 %) carry a decomposer guild**. The
   monoculture gate reads trait-space collapse, and a decomposer is a behavioural role, not a
   trait (trait-space.md) — a trait monoculture can hold a real decomposer guild, and the gate
   removes it. This is the largest single lever on guild visibility the numbers show.
4. **Options 2 and 3 would each act on a near-empty signal today.** One unselected config
   (`sample:127`: live, `coexistence_fraction` 1.0, decomposer 0.6, fitness 0.47) shows a guild
   cell is reachable in the box, but it is 1 of 141 live LHS configs. Under the guild-aware
   coexistence floors (#538) every top-10 refined cell reads 0.00–0.06, so option 3 would clear
   none of them.

## Runs

| run | command | wall clock |
|---|---|---|
| search | `explorers-search --batch 32 --generations 10 --ensemble 5 --max-ticks 2000 --seed 42 --checkpoint …` (defaults `--refine-top-k 10 --refine-ensemble 32`) | 31 min search + ~1 min refinement; two generations carried a dense config (16 m 34 s, 7 m 17 s), the other nine took 16–90 s |
| atlas census | `guild_census --atlas atlas.json --configs atlas:0..94` | ~1 min |
| LHS census | `guild_census` over `sample:0..199`, split across four processes by index | ~60 min; sequential across configs, so each dense config blocks its process |

The census rolls each config out with `run_ensemble` at the search's ensemble size (5), horizon
and `EvalConfig`, and reduces with `qd::config_eval_from_ensemble` — the reduction the archive
stores on a cell — on the seed block 1000..1004. The atlas does not record which seed block
produced each elite, so the atlas census re-measures cells on fresh seeds; it reproduces the
atlas's own counts (0 cells ≥ 0.5 under both).

## The regenerated atlas

| | 474 (T = 500) | this (T = 2000) |
|---|---|---|
| live cells | 82 | 95 |
| best fitness | 0.799 | 0.572 |
| QD-score | — | 36.40 |
| dead frontier | — | nutrient_lockup 52, extinction 9, monoculture 3, energy_death 2, generalist_dominance 2 |

Fitness is not comparable across the two: the horizon, the settled-window reads (#503) and the
per-agent trophic balance (#486) all changed what it sums. The projected recipe is the best
refined-robust live cell under the plain floor.

Refinement (top-10 live cells, n = 32, independent seeds), guild reads per cell:

| cell | fitness | coexist (n=5 → n=32) | decomposer / consumer (n=32) | refined fitness |
|---|---|---|---|---|
| [8, 19, 1] | 0.572 | 1.00 → 0.47 | 0.00 / 0.00 | 0.250 |
| [0, 19, 6] | 0.563 | 0.80 → 0.25 | 0.00 / 0.00 | 0.000 |
| [7, 19, 6] | 0.555 | 0.60 → 0.97 | 0.06 / 0.00 | 0.538 ✓ |
| [1, 19, 5] | 0.513 | 0.80 → 0.62 | 0.00 / 0.00 | 0.521 ✓ |
| [7, 19, 4] | 0.511 | 1.00 → 1.00 | 0.00 / 0.00 | 0.491 ✓ |
| [8, 19, 2] | 0.507 | 1.00 → 0.94 | 0.00 / 0.00 | 0.464 ✓ |
| [11, 19, 1] | 0.502 | 0.80 → 0.81 | 0.00 / 0.00 | 0.468 ✓ |
| [10, 19, 2] | 0.500 | 0.80 → 0.69 | 0.03 / 0.00 | 0.406 ✓ |
| [9, 19, 1] | 0.499 | 0.80 → 0.53 | 0.03 / 0.00 | 0.384 ✓ |
| [9, 19, 2] | 0.498 | 1.00 → 0.97 | 0.00 / 0.00 | 0.471 ✓ |

✓ = clears the plain coexistence floor (0.50). Under the `decomposer`, `consumer` and `either`
floors no cell clears.

## Counts: atlas vs unselected LHS

Per config (fraction of the 5-seed ensemble), cross-tabbed against `coexistence_fraction ≥ 0.5`:

| predicate | atlas (95; 72 coexisting) | LHS all (200; 69 coexisting) | LHS live (141; 69 coexisting) |
|---|---|---|---|
| decomposer ≥ 0.5 | 0 | 3 (1 coexisting) | 1 (1 coexisting) |
| consumer ≥ 0.5 | 0 | 0 | 0 |
| either ≥ 0.5 | 0 | 3 (1 coexisting) | 1 (1 coexisting) |
| both ≥ 0.5 | 0 | 0 | 0 |
| either = 1.0 | 0 | 0 | 0 |

Per seed, by the seed's own verdict:

| | atlas (475 seeds) | LHS (1000 seeds) |
|---|---|---|
| live at `T` | 427 — decomposer 4, consumer 1 | 695 — decomposer 6, consumer 0 |
| monoculture (gated at `T`) | 0 | 34 — decomposer 12, consumer 1 |
| generalist_dominance (gated at `T`) | 0 | 5 — decomposer 3, consumer 2 |
| nutrient_lockup / extinction / energy_death (early stop) | 48 | 266 — none carry a guild |

Early-stopped seeds never reach the settled window, so no guild is read on them; that is the
read's definition, not an absence observed. The LHS configs with a decomposer fraction ≥ 0.5 are
`sample:20` (0.8, representative `monoculture`), `sample:36` (0.6, `monoculture`) and
`sample:127` (0.6, live, coexisting 1.0).

**`sample:55`**, the config #492 read a decomposer guild on (7 / 8 seeds over (1000, 2000]), now
early-stops on `nutrient_lockup` on all five census seeds between ticks 350 and 750. The census
does not say which change since #492 moved it; it is recorded here so that config is not reused
as a guild exemplar without re-checking it.

## What the numbers say about the three options

The authority boundary these options revisit, from
[genesis-search.md](../system-design/genesis-search.md) *Authority boundary: the heterotroph
guilds are reported, never optimised*:

> The atlas therefore records it as a **per-cell distribution** — the fraction of a cell's seed
> ensemble that holds a **heterotroph guild** of that role, with the sample count — and **never**
> as a behaviour axis or a fitness term. […] Both values are starting points, not settled
> thresholds: regenerating the atlas with the read reported, and counting the cells that pass,
> is what tests them and decides whether the observable ever becomes a binning axis or folds
> into `coexistence_fraction`.

1. **Leave it reported.** The condition in the issue ("if the fixed-stepper search already lands
   guild cells at a usable rate") is not met: 0 / 95. But the search is also not the cause —
   living LHS worlds hold guilds at the same 0.9 % per seed. Leaving it reported is honest about
   an observable that is almost always zero on living worlds.
2. **Binning axis.** Revisits the boundary. Among live configs the axis would put 140 / 141 LHS
   configs at the floor bin; the emitter would see almost no gradient along it. `sample:127` shows
   the target region is not empty, so the axis could still pull the archive towards it, but on a
   near-flat signal.
3. **Fold into the coexistence floor.** Revisits the boundary at the projection only. Today it
   rejects every top-10 refined cell (guild-aware floors 0.00–0.06), so the projection would fall
   back to argmax-fitness with its warning.

**The input the issue did not anticipate** is finding 3: the `monoculture` gate zeroes 12 of the
21 LHS seeds that do form a decomposer guild, and `generalist_dominance` another 3. Any of the
three options is decided on a guild signal that the gates have already mostly removed. Whether a
trait-space monoculture holding a behavioural guild should count as a failed world is a separate
question from this issue's three options, and upstream of them.

*Resolved in #546* ([546-guild-in-monoculture.md](546-guild-in-monoculture.md)): those gated
guilds are mostly a straddle of the producer / heterotroph line inside one mixotroph trait
continuum, not a separate diet-specialised population. The gate is kept as is, and the gated
count above overstates the real guilds it removes (genesis-search.md, *Authority boundary*).

## Reproduce

```
cargo build --release -p explorers-search
./target/release/explorers-search --batch 32 --generations 10 --ensemble 5 --max-ticks 2000 \
  --seed 42 --output atlas.json --recipe-output recipe.json --checkpoint target/494/search.ckpt
./target/release/guild_census --atlas atlas.json --configs atlas:0,…,atlas:94 \
  --output target/494/atlas-census.jsonl --limit 20      # repeat until it runs 0 configs
./target/release/guild_census --output target/494/lhs-census.jsonl --limit 20   # repeat; 200 configs
./target/release/guild_census --summary --output target/494/lhs-census.jsonl
```

Interrupted searches resume with `--resume target/494/search.ckpt` (#530). Census files are
JSON-lines and resume by skipping configs already present; disjoint `--configs` sets can run in
parallel into separate files and be concatenated.
