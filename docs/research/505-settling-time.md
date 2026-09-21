# Research #505 — The settling-time sweep: grace measured, horizon not

**Status: measurement. Commits one value (`EvalConfig::grace_ticks = 260`) and declines to commit
the other.** This note records the sweep the [genesis-search](../system-design/genesis-search.md)
design asks for under *The gates' reference excludes the founder transient* and *The horizon is the
settling time*: the instrument from #504 (`crates/explorers-search/src/bin/settling_time.rs`) run
over the atlas's live cells and a low-discrepancy sample of the search box at a 3000-tick horizon,
and the two ecological constants read off it by the rules #505 fixed in advance. Parent: #504
(instrument), #501 (the settled-community atlas). Stepper at `9f34f84`.

## TL;DR

1. **Grace = 260 ticks.** The provisioning-transient tick — the first tick at which the living
   free-energy stock re-exceeds its tick-0 value — has 50 / 90 / 95 / max = **2 / 253 / 514 / 2586**
   over the 1357 live runs that ever re-exceed it (atlas and sample pooled, nearest-rank). The rule
   (90th percentile, rounded up to 10) gives 260; `EvalConfig::grace_ticks` moves from the 100-tick
   placeholder to it. Half the live runs recover their founder stock within two ticks; the tail is
   long (5 % take more than 500) and **123 live runs (8 %) never do**, which is the case the design
   cites for a fixed count rather than a per-run reference.
2. **`T` is not measured by this sweep.** The producer-settling tick reads 50 / 90 / 95 / max =
   **1677 / 2981 / 3000 / 3000** over 1407 live runs; the rule (smallest multiple of 500 with `T/2`
   above the 90th percentile) returns 6000. The statistic is saturated: 29 % of live runs leave the
   ±20 % band *inside* the final-500-tick window the band is the mean of, the median settled
   producer count is 3 (so ±20 % is narrower than one agent and any birth or death breaks it), and
   the upper quantiles sit at the horizon on every sub-population. The reading would move with any
   horizon it was run at, so it is not a settling time. **`T = 2000` stands as the working value and
   `SearchConfig::max_ticks` is not moved** (#507 sets it to 2000 separately).
3. **The sweep needed a wall-clock guard.** Seventeen sample configs cost seconds per tick at their
   3000-tick population; a chunk that reached `sample:20`–`sample:24` ran 6.5 h before it was killed. The bin now
   has `--run-timeout-secs` (default 300): 85 of the 2256 runs are recorded as mode `timeout` (all
   in the sample, none in the atlas) and are excluded from the live set like any other non-live run.
4. **Byte-identical resume holds.** An independent partial sweep to a second file reproduced 10 of
   10 overlapping rows byte for byte, including atlas rows first produced by the pre-guard binary.

## 1. What ran

| | |
|---|---|
| configs | 82 atlas live cells (`atlas.json`) + 200 LHS points (`sampled_units`, seed 421) = 282 |
| seeds | 8 per config (`1000..1008`) → 2256 runs |
| horizon | 3000 ticks, nothing injected; the genesis step loop with the evaluator's early stops |
| band | ±20 % of the mean producer count over the final 500 ticks |
| run timeout | 300 s wall clock per (config, seed) |
| output | `target/settling-time.jsonl`, SHA-1 `5e4e62b1d73af3e9ebbb1dac40669c2e16a06da8` (282 rows) |
| check file | `target/settling-time-check.jsonl`, SHA-1 `95726f1d71416b733168bb8601535867f1292668` (10 rows; `atlas:0–4, 60, 81`, `sample:3, 30, 199`; every row byte-identical to the main file's) |
| wall clock | ≈ 2.5 h of productive compute in foreground chunks, plus a 6.5 h stall on `sample:24` before the guard existed — ≈ 9 h end to end |

The two ticks are as the instrument defines them (its summary restates the procedure beside the
quantiles): *provisioning-transient tick* = first tick at which `World::free_energy` exceeds its
tick-0 value, `null` if never; *producer-settling tick* = last tick at which the producer count
(`photosynthetic_absorption >= heterotrophy`) lies outside `mean × (1 ± 0.2)`, `mean` over the final
500 ticks, `0` if never outside, `null` if the run did not reach the horizon or the tail holds no
producers. Quantiles are nearest-rank over **live** runs (mode `none`, reached the horizon).

### Live and non-live runs

| source | runs | live | extinction | energy-death | monoculture | generalist-dom. | nutrient-lockup | timeout |
|---|---|---|---|---|---|---|---|---|
| atlas | 656 | 412 | 17 | 2 | 11 | 1 | 213 | 0 |
| sample | 1600 | 1068 | 129 | 4 | 36 | 7 | 271 | 85 |
| pooled | 2256 | 1480 | 146 | 6 | 47 | 8 | 484 | 85 |

Lockup is the dominant non-live mode on the atlas at 3000 ticks (213 / 656 runs of cells that were
*live* at the 500-tick horizon the atlas was built at) — the bloom-then-lockup trajectory the design's
*one rollout, one settled window* paragraph anticipates.

## 2. Grace: the provisioning transient

| population | n | p50 | p90 | p95 | max | never re-exceeds |
|---|---|---|---|---|---|---|
| atlas | 401 | 1 | 198 | 419 | 1885 | 11 |
| sample | 956 | 3 | 296 | 551 | 2586 | 112 |
| **pooled** | **1357** | **2** | **253** | **514** | **2586** | **123** |

Histogram of the pooled transient in 50-tick bins: `[0, 50)` 981, `[50, 100)` 110, `[100, 150)` 60,
`[150, 200)` 43, `[200, 250)` 25, `[250, 300)` 18, `[300, 350)` 14, `[350, 400)` 11, `[400, 450)` 12,
`[450, 500)` 13, `≥ 500` 70. Two regimes: most worlds are net-positive from the first step
(photosynthetic income exceeds the founder budget at once), and a long tail of worlds whose founder
stock is large relative to income takes hundreds of ticks to be overtaken. The rule reads the tail:
**grace = ⌈253 / 10⌉ · 10 = 260**.

Consequence for the evaluator: the energy-death and lockup references now start at tick 260, and the
monoculture / generalist-dominance roster gates hold off the founder cohort for 260 ticks. On the
committed goldens (`evaluator_pin` at the 500-tick horizon, `guild_anchor` on the projected recipe) no
pinned bit moved — no gate verdict on those cells depended on the reference starting between 100 and
260 — so neither golden was re-captured.

## 3. Horizon: the settling statistic saturates

| population | n | p50 | p90 | p95 | max | no producers in tail |
|---|---|---|---|---|---|---|
| atlas | 396 | 2021 | 2989 | 3000 | 3000 | 16 |
| sample | 1011 | 1527 | 2973 | 3000 | 3000 | 57 |
| **pooled** | **1407** | **1677** | **2981** | **3000** | **3000** | **73** |

Applied mechanically the rule gives `T = 6000` (`6000 / 2 = 3000 > 2981`). The reading is not
believed, for three reasons visible in the same file:

- **The band breaks inside its own reference window.** 403 of the 1407 runs (29 %) have their last
  out-of-band tick after 2500 — inside the final 500 ticks whose mean defines the band. Those runs are
  not "still settling at 2981"; their producer count fluctuates by more than 20 % of its own tail
  mean, tick to tick, for as long as the run lasts.
- **The band is narrower than one agent.** The settled producer count has 50 / 90 / 95 / max =
  3 / 14 / 26 / 1945: at the median, ±20 % is ±0.6 agents, so any birth or death is an excursion.
  875 of 1407 runs have a tail mean below 5. Restricting to runs with tail mean ≥ 5 (n = 532) reads
  1960 / 2987 / 3000 / 3000; ≥ 20 (n = 102) reads 2025 / 3000 / 3000 / 3000 — the upper quantiles
  are at the horizon on every sub-population, dense ones included.
- **It would move with the horizon.** A last-excursion statistic on a series whose noise exceeds the
  band has expected value `T − O(noise interval)`; at a 6000-tick horizon the same rule would return
  12000. A quantity that tracks the instrument's own setting is not an ecological constant, which is
  the property the design requires of `T`.

Histogram of the settling tick in 500-tick bins: `[0, 500)` 224, `[500, 1000)` 210, `[1000, 1500)`
206, `[1500, 2000)` 171, `[2000, 2500)` 193, `[2500, 3000]` 403 — flat across the run, then the
pile-up at the end. The flat part is consistent with a mixture of genuinely settling worlds and
noise-driven excursions; the statistic as defined cannot separate them.

**What is committed:** `T = 2000` remains the working value on the [443 §6.2–6.3](443-invasion-growth.md)
evidence (decomposers still rising 2–18× at 1000 ticks; the control arm only drifting at 2000;
guild doubling time ≈ 700 ticks). **What is not:** `SearchConfig::max_ticks` is not moved on a
saturated read. **What is owed:** a settling statistic that is robust to demographic noise at small
counts — a band in absolute agents (`max(0.2 · mean, k)`), or a smoothed series (a trailing mean
over a window longer than the demographic interval), or a change-point read — re-run off the same
rows' procedure before `T` is read again. The raw file carries every run's founder, peak, tail-mean
and terminal producer counts, so the band can be re-designed against it without re-running the
sweep for the sizing question; the settling tick itself must be re-measured.

## 4. The timeouts

| config | timed-out seeds | tick reached at 300 s |
|---|---|---|
| `sample:24` | 4 | 314–1059 |
| `sample:38` | 3 | 712–2110 |
| `sample:45` | 4 | 1059–2055 |
| `sample:55` | 7 | 833–2445 |
| `sample:68` | 2 | 2261–2635 |
| `sample:100` | 7 | 800–2285 |
| `sample:129` | 8 | 526–1113 |
| `sample:130` | 2 | 1765–2845 |
| `sample:131` | 3 | 402–1782 |
| `sample:146` | 7 | 1218–2871 |
| `sample:147` | 8 | 604–1414 |
| `sample:151` | 3 | 1214–2746 |
| `sample:154` | 8 | 444–1487 |
| `sample:164` | 2 | 1942–2159 |
| `sample:165` | 7 | 516–1194 |
| `sample:169` | 4 | 871–2577 |
| `sample:197` | 6 | 670–1868 |

85 runs on 17 sample configs; no atlas cell timed out. These are the dense worlds — `sample:55`,
`sample:129` are 443's positive-control guild configs; `sample:154` is a #439 name — with
populations in the hundreds to low thousands where `move_agents` / `query_radius` costs seconds per
tick. Before the guard, `sample:20` took ≈ 2.5 h for its 8 seeds and `sample:24` was still on its
last seeds after 3 h. The guard reads a clock between steps, so a run's row is byte-identical
whether or not a budget is set unless the budget fires; a timed-out row's `termination_tick` is
wall-clock-dependent and is not expected to reproduce. A step that itself takes minutes is only
interrupted at its end, so a 300 s budget can still cost a chunk ~8 min on the densest seeds.

Those 85 runs are excluded from the live set; their configs are exactly the dense ones where the
provisioning transient is likely long and the settling read least noisy, so the sweep under-samples
the tail of both distributions on the sample side. On the atlas side, which is what the gates and
the search actually run over, nothing was lost.

## 5. Reproducing

```sh
cargo build --release -p explorers-search --bin settling_time
# resume-safe; repeat until it reports "running 0 now"
./target/release/settling_time --limit 1
./target/release/settling_time --summary
```

Rows are appended per completed config in a fixed order, so a loop of short calls produces a file
byte-identical to one uninterrupted run (checked above). Pooled quantiles are not printed by the
summary (it splits by source); they were taken with a ten-line script over the JSON-lines file
(nearest-rank, `ceil(p · n)`).
