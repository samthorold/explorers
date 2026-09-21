# Research #508 — Energy death read against the sustainable stock: the zero-false-positive check

**Status: measurement and promotion. The history-free read passed the check and now runs in the
evaluator's energy-death gate; the history-peak read it replaced is kept only as the check's
comparison arm.** This note records the sweep [expected properties](../system-design/expected-properties.md)
(*Energy death — How it is read*) asks for before the read is promoted: the instrument
(`crates/explorers-search/src/bin/energy_death_check.rs`) run over the atlas's live cells and the
200-point LHS sample of the search box at the settled horizon `T = 2000`, 8 seeds each, comparing
the new read against the stand-in on every run that reaches `T`. Parent: #505 (grace = 260), #506
(incremental gates, carry-to-`T`), #507 (`T = 2000`). Stepper at `f4f4cb3`.

## TL;DR

1. **Zero false positives — the read is promoted.** Over 1593 runs that reached `T` (442 atlas,
   1151 sample), no run the new read calls dead is one the stand-in calls alive *and* that is
   visibly alive at `T` (population `≥ ROSTER_FLOOR = 20`, at least one birth in `(T/2, T]`).
   `is_free_energy_dead_sustainable` now backs `dead_pool_gate` — the horizon verdict and the
   incremental stop alike — and `COLLAPSE_FRACTION` is unwired.
2. **The two reads disagree on 22 runs, in both directions, and every disagreement is a roster of
   one to five bodies.** 18 runs the new read calls dead and the stand-in alive: worlds down to 1–5
   agents at `T` holding 0.4–9.5 % of their sustainable stock, most with no settled-window births —
   the case the design cites against the history reference (a world that never recovers its
   founder stock never acquires a reference and passes as live). 4 runs the stand-in calls dead and
   the new read alive: single survivors holding 12–258 % of the sustainable stock after the bloom
   collapsed — the bloom-ceding misfire the design predicted, and on all four the incremental
   stand-in gate had already fired between ticks 1150 and 1900.
3. **The margin is thin at one sample config.** The smallest living/sustainable ratio on a
   visibly-alive, stand-in-alive run is **0.106** (`sample:61` seed 1005: 20 agents, 610 settled
   births), against `SUSTAINABLE_FRACTION = 0.1`; the same config's seeds 1002–1004 sit at
   0.049–0.066 with 2–5 agents and 20–98 settled births, called dead by the new read. The rule is
   met, the number is the one to watch when the check is re-taken.
4. **The living stock is generally far above the sustainable stock.** Over all runs at `T` the ratio
   reads 5 / 50 / 95 % = 0.78 / 10.3 / 67.3, max 222: living energy has no ceiling under the
   committed rules ([viability](../system-design/viability.md), open finding), and the reproductive
   allocation hoards well past one tick of the solar cap. The reference is therefore a floor a
   living world clears by an order of magnitude, not a ceiling it approaches.

## 1. The read

`sustainable_stock(params) = F·m²·τ`, `m = ⌊√2·L/r⌋ + 1`, `τ = 1`. Derivation (in the function's doc
comment): solar flux is capped per tick at `P_max = F·m²`; every survivor pays `≥ B` per tick; so the
mean survivor count is bounded by `N̄_max = F·m²/B`, and a survivor of a tick held at least one tick
of base metabolism `ε = B·τ` at its start. The stock of a population at the ceiling is `N̄_max·ε =
F·m²·τ`, independent of `B` and of every searched coefficient but `F`, `L`, `r`.

`is_free_energy_dead_sustainable(post_grace, window, stock)`: the trailing `energy_death_window`
(50) peak of the post-grace living stock is below `SUSTAINABLE_FRACTION = 0.1` of the stock. The
fraction is the same tenth the stand-in read (`COLLAPSE_FRACTION`), chosen before the sweep and
not tuned to it. The grace (260) keeps the founder provisioning off the series; the read fires at
the first window boundary past `grace + window` on a world that stays at the founder scale.

## 2. What ran

| | |
|---|---|
| configs | 82 atlas live cells (`atlas.json`) + 200 LHS points (`sampled_units`, seed 421) = 282 |
| seeds | 8 per config (`1000..1008`) → 2256 runs |
| horizon | 2000 ticks (`SearchConfig::max_ticks`), grace 260, window 50 |
| early stops | extinction, explosion, nutrient lockup stop the rollout; an energy-death stop is recorded (first tick) and the rollout carries on to `T` |
| run timeout | 300 s wall clock per (config, seed) |
| output | `target/energy-death-check.jsonl`, SHA-1 `307f74388898785766fe1f7256ed7254d6311b1c` (282 rows) |
| wall clock | ≈ 1 h 25 min of sim time in foreground chunks (`--limit 1` on the dense sample configs, up to 40 elsewhere); one `sample:38` seed spent ≈ 9 min inside a single step past its 300 s budget |

The sweep was run with the stand-in still wired as the incremental gate (the bin was built before
promotion); since an energy-death stop is carried on and both reads are taken directly on the
post-grace series at `T`, the rows are the same rows the promoted bin produces, except that
`standin_first_fire_tick` records the stand-in's incremental fire. Re-running the bin on the
promoted evaluator records the new read's fire tick in that field instead.

### Runs by outcome

| source | runs | reached `T` | visibly alive | extinction | lockup | timeout |
|---|---|---|---|---|---|---|
| atlas | 656 | 442 | 101 | 13 | 193 | 8 |
| sample | 1600 | 1151 | 94 | 117 | 298 | 34 |
| **pooled** | **2256** | **1593** | **195** | **130** | **491** | **42** |

Timeouts: `atlas:59` (8 seeds), `sample:20` (8), `sample:24` (4), `sample:129` (6), `sample:165`
(7), `sample:147` (2), `sample:197` (2), and one seed each of `sample:38`, `45`, `100`, `154`,
`169`. These are the dense worlds #505 named; they are excluded from the comparison like any run
that did not reach `T`.

## 3. The comparison

Over runs that reached `T`:

| source | stand-in dead | new read dead | both | new dead / stand-in alive | stand-in dead / new alive | **false positives** |
|---|---|---|---|---|---|---|
| atlas | 0 | 1 | 0 | 1 | 0 | **0** |
| sample | 4 | 17 | 0 | 17 | 4 | **0** |
| pooled | 4 | 18 | 0 | 18 | 4 | **0** |

The reads never agree on a death: the four worlds the stand-in kills are alive by the new read and
the eighteen the new read kills are alive by the stand-in. That is expected of two references that
measure different things — the run's own bloom versus the config's resource base — and the
promotion rule asks only that the new read's kills are not visibly-alive worlds.

**New dead, stand-in alive (18).** `atlas:0` s1003 (1 agent, 0 births, ratio 0.026); `sample:24`
s1000 (2, 0, 0.016); `sample:38` s1007 (1, 9, 0.004); `sample:61` s1001–1004 (2 / 2 / 5 / 3 agents,
2 / 44 / 20 / 98 births, 0.017–0.066); `sample:93` s1003 (1, 0, 0.095); `sample:96` s1003 (2, 0,
0.066); `sample:106` s1000–1005 (1–2 agents, 0 births, 0.012–0.030); `sample:131` s1000, s1004 (1,
0, 0.015 / 0.008); `sample:158` s1005 (2, 4, 0.015). None reaches the roster floor; twelve have no
settled-window birth at all. On every one the incremental stand-in gate never fired: the founder
bloom on these worlds was itself small, so the stock never fell to a tenth of its own history.

**Stand-in dead, new alive (4).** `sample:46` s1002 (1 agent, 0 births, ratio 2.58, stand-in fired
at 1400); `sample:50` s1002 (1, 108, 0.12, fired 1900); `sample:73` s1006 (1, 0, 0.21, fired 1150);
`sample:194` s1002 (1, 0, 0.21, fired 1850). Single survivors holding more than a tenth of the
sustainable stock — on `sample:46` two and a half times it — after a bloom that collapsed. Whether a
lone hoarder is "alive" is exactly what the ceiling cannot say (no drain on the reproductive
allocation); the new read declines to call it dead, and none of the four is visibly alive, so
neither read is contradicted by the roster.

**Reversible collapses.** Ten runs at `T` had the incremental stand-in gate fire between ticks 1150
and 1900; six of them read alive on the full series by *both* reads (`atlas:79`, `sample:19`, `88`,
`131`, `155`, `176`) — the carry-to-`T` disagreements #506 surfaces. The new read's own
incremental behaviour on them is not in this file (the sweep ran with the stand-in as the wired
gate); the promoted bin records it.

### The margin

Stock ratio (`window_peak_stock / sustainable_stock`) over the visibly-alive, stand-in-alive runs —
the population a false positive would have to come from:

| population | n | min | p05 | p50 | p95 |
|---|---|---|---|---|---|
| atlas | 101 | 3.30 | 7.45 | 28.4 | 80.9 |
| sample | 94 | 0.106 | 1.80 | 11.9 | 71.0 |
| pooled | 195 | 0.106 | 3.00 | 21.6 | 80.9 |

The five lowest: `sample:61` s1005 (0.106; 20 agents, 610 births), `sample:61` s1006 (0.150; 26,
774), `sample:131` s1003 (0.267; 131, 532), `sample:61` s1000 (0.504; 78, 2596), `sample:96` s1006
(1.80). Every atlas live cell clears the fraction by at least 33×. `sample:61` is a large, sparse
world (sustainable stock 54 458 E) whose settled community is small; at `SUSTAINABLE_FRACTION =
0.11` its seed 1005 would be the first false positive. The fraction was fixed before the sweep and
is not moved on it.

## 4. Consequences

- `dead_pool_gate` and hence `evaluate_from_log` and `early_stop` read energy death against
  `sustainable_stock(world.params())`; `early_stop` takes the stock as an argument, computed once per
  rollout by `run_single` and the instrument bins.
- `is_free_energy_dead` (`COLLAPSE_FRACTION = 0.1`) is unwired and kept as the comparison arm of
  `energy_death_check`, so the check can be re-taken when the stepper changes.
- Two evaluator tests that encoded the history reference were rewritten to the new property (the
  verdict is history-free whatever the horizon or grace; a founder-scale stock that never climbs is
  dead from the first post-grace window), and the genesis fixture for "a world the energy-death gate
  stops" is now a starved population under a large ceiling rather than a founder budget being spent
  down — the old fixture is precisely a world the new read (correctly) calls alive.
- The committed goldens (`evaluator_pin` at the 500-tick horizon, `guild_anchor` on the projected
  recipe) did not move: no pinned seed's energy-death verdict depended on the reference.
- The doc's stand-in sentence is replaced by the check's result (expected-properties.md); the
  matching clause in genesis-search.md and a pointer on the sustained-population bound in
  viability.md are updated. Nothing else in the design moves.

## 5. Reproducing

```sh
cargo build --release -p explorers-search --bin energy_death_check
# resume-safe; repeat until it reports "running 0 now"; --limit 1 near the dense sample configs
./target/release/energy_death_check --limit 5
./target/release/energy_death_check --summary
```

Rows are appended per completed config in a fixed order, so a loop of short calls produces a file
byte-identical to one uninterrupted run; a timed-out row's `termination_tick` is wall-clock-dependent
and is not expected to reproduce. The per-run details above were read off the JSON-lines file with a
short script; the summary prints the counts and the ratio quantiles.
