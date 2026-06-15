# Issue #423 — genesis step-loop hot path: profile & attribution

**Status: research finding. Changes no committed semantics.** This is a *diagnostic*
report. It does not touch the stepper, `SpatialGrid::query_radius`, the evaluator, or
the search. It profiles the genesis step loop, attributes the cost with evidence, and
returns an explicit confirm/refute verdict on the three hypotheses in #423, so that
optimisation follow-ups can be opened against confirmed bottlenecks rather than guesses.

The deliverables are committed harness + numbers:

- **Tier 0** — `role_emergence` (`crates/explorers-search/src/bin/role_emergence.rs`)
  now records per-run **wall-time**, **peak/final population**, and the decoded
  **radius / extent params** (`sensing_range_coefficient`, `light_competition_radius`,
  `contact_range_coefficient`, `world_extent`, `nutrient_grid_cell_size`, and the
  `grid_cell_size` the stepper actually builds the grid with). A `ROLE_EMERGENCE_CONFIGS`
  / `ROLE_EMERGENCE_SEEDS` subset selector makes re-running just the worst configs cheap.
- **Tier 1** — `step_profile` (`crates/explorers-search/src/bin/step_profile.rs`): a
  single-config driver that runs one `(config, seed)` through *just* `World::step` (no
  classification), for attaching a sampling profiler.
- **Tier 2** — `bench_query_radius` (`crates/explorers-sim/src/bin/bench_query_radius.rs`):
  a deterministic microbenchmark sweeping population × (radius/cell_size) and comparing
  the real grid against inline-position / scratch-buffer variants.

The flamegraph is a transient local artifact and is **not** committed.

## TL;DR

1. **For the bulk of runs, wall-time tracks population** (Spearman ρ = 0.90 over 170
   runs); the `sensing / cell_size` ratio alone is uncorrelated (ρ ≈ −0.06). Most
   configs have a `cell_size` (= `light_competition_radius`) of several units, so the
   per-query cell scan stays bounded and population dominates.
2. **But the catastrophic tail is an *interaction*: high population × large query radius
   ÷ small grid cell ÷ small (wrapping) world.** The two worst runs are `sample:147`
   (519 s) and `sample:154` (377 s); the next-worst is 40 s — a >9× cliff. `sample:147`
   is *slower than* `sample:154` despite a **lower** peak population (948 vs 1216),
   because its sensing radius (27.6) against a 1.26 grid cell in a 27-unit world makes
   the neighbourhood scan **wrap and revisit cells**.
3. **H1 (structural: large radius ÷ small `cell_size`, plus toroidal cell revisits) —
   CONFIRMED.** On `sample:147` the sensing/movement phase (`phase.rs:1003`) is **91.9 %**
   of the step; `query_radius` is **47 %** inclusive. The microbench shows wall-time
   tracking the `(radius/cell_size)²` cell-scan term almost linearly (cells scanned grows
   ~159×, ns/call ~140× from ratio 1→30, while neighbours *returned* stay tiny), and in
   the small-world regime **~90 % of returned ids are toroidal duplicates** at ratio 30.
4. **H2 (per-candidate `HashMap` probe in `query_radius`) — CONFIRMED, and it is the
   single largest cost.** ~59 % of step self-time on `sample:147` is `HashMap`/SipHash
   work. The microbench shows that storing positions *inline* in the cell buckets
   (dropping the side `HashMap<id,pos>`) removes **27–77 %** of per-call time, rising with
   population and with the wrapping regime.
5. **H3 (per-call `Vec` allocation) — REFUTED as a meaningful win.** A reused scratch
   buffer saves only **0–5 %** on the expensive queries (it is 16–21 % only on the
   cheapest tiny-radius calls, where there's nothing else to pay for). On the flamegraph,
   allocation is ~0 %. Real, but not where the time is.

## Tier 0 — worst configs (time vs population vs radius)

Re-run of 85 configs (all 56 atlas + every 7th sampled), 2 seeds each, to the 2000-tick
horizon (`ROLE_EMERGENCE_CONFIGS=… ROLE_EMERGENCE_SEEDS=2`). `ratio = sensing_range_coefficient
/ grid_cell_size` is the worst-case `radius/cell_size` for the dominant (sensing) query.
Note `wall_time_s` here is the **whole** `role_emergence` run *including* the per-10-tick
trophic-role classification; Tier 1 isolates the pure stepper.

| config | seed | wall_time_s | peak_pop | final_pop | sensing | grid_cell | world_extent | ratio |
|--------|-----:|------------:|---------:|----------:|--------:|----------:|-------------:|------:|
| sample:147 | 1000 | **519.6** | 948 | 292 | 27.6 | 1.26 | 27 | **21.9** |
| sample:154 | 1000 | **377.0** | 1216 | 1009 | 25.2 | 2.72 | 75 | 9.3 |
| sample:147 | 1001 | 331.1 | 707 | 302 | 27.6 | 1.26 | 27 | 21.9 |
| sample:14  | 1001 | 40.0 | 418 | 115 | 7.4 | 4.39 | 53 | 1.7 |
| atlas:41   | 1001 | 14.9 | 270 | 39 | 2.9 | 10.08 | 100 | 0.3 |
| atlas:51   | 1001 | 14.8 | 200 | 177 | 1.0 | 7.12 | 77 | 0.1 |
| atlas:15   | 1000 | 12.8 | 297 | 123 | 21.9 | 9.57 | 100 | 2.3 |

Rank correlations over all 170 runs: `ρ(time, peak_pop) = 0.90`, `ρ(time, ratio) = −0.06`,
`ρ(time, peak_pop × ratio) = 0.65`. **Reading:** population is the population-wide driver;
the extreme tail needs *both* high population *and* a large radius / small cell. A high
ratio with a large cell (atlas:15, ratio 2.3 but cell 9.6) is cheap; a small cell with low
population dies before it matters. The blow-up is the product, and `sample:147` is the
corner where all three (radius, small cell, small wrapping world) line up on a
near-`max_population` survivor.

## Tier 1 — CPU profile of one expensive config

Driver: `step_profile sample 147` runs the `sample:147` config (sensing 27.6, grid cell
1.26, world_extent 27.4 → `cols ≈ 21 < cells_to_check ≈ 23`, i.e. the scan wraps the whole
grid). ~286 ms/tick at peak pop 948. Captured with `samply` (150 ticks, 42 784 samples),
symbolicated against the `CARGO_PROFILE_RELEASE_DEBUG=1` binary:

Reproduce:

```
CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --release -p explorers-search --bin step_profile
samply record ./target/release/step_profile sample 147 1000 150
```

**Inclusive time (share of the step):**

| frame | inclusive |
|-------|----------:|
| `World::step` | 100 % |
| `phase::move_agents` (sensing + locomotion, `phase.rs:1003`) | **91.9 %** |
| `SpatialGrid::query_radius` | **47.2 %** |
| `hashbrown::HashMap` (probe + insert) | 36.0 % |
| `phase::resolve_reproduction` | 1.9 % |
| `phase::resolve_drains` | 0.7 % |
| `phase::photosynthesise` | 0.3 % |

**Self time (where cycles are actually spent):**

| frame | self |
|-------|-----:|
| SipHash / `DefaultHasher::write` / hashing (`<deduplicated_symbol>` + `Hasher::write`) | **~44 %** |
| `SpatialGrid::query_radius` | 23.5 % |
| `hashbrown::HashMap::insert` | 14.1 % |
| `phase::move_agents` | 11.5 % |
| `World::step` | 5.0 % |

The dominant phase is unambiguously **sensing/movement** (`move_agents`), which issues a
`query_radius` at the sensing radius `eff_mobility * sensing_range_coefficient` (up to ~27).
Roughly **59 % of step self-time is `HashMap`/SipHash** — split between the
`self.positions[&id]` probe inside `query_radius` (H2) and the `seen` `HashSet` the phase
uses to dedupe the query results. Crucially, that dedup cost is itself **inflated by H1**:
in this wrapping world `query_radius` returns ~90 % duplicate ids, and every duplicate is
SipHash-inserted into `seen`. H1 and H2 compound.

## Tier 2 — query_radius microbenchmark scaling

`bench_query_radius` (deterministic, `cargo run --release -p explorers-sim --bin
bench_query_radius`). `cell_size = 1` throughout, so `radius == ratio`. `base` = the real
`SpatialGrid`; `inline` = positions in cell buckets (no side `HashMap`, isolates H2);
`scratch` = inline + reused buffer (isolates H3). `scan` = cells visited =
`(2·cells_to_check+1)²`; `dup/call` = double-returned ids.

**Wide world** (`cols = 200 ≫ cells_to_check`, no wrap) — the clean `(radius/cell_size)²` curve:

| N | ratio | scan | ret/call | dup/call | base_ns | inline_ns | scratch_ns | H2_save | H3_save |
|--:|------:|-----:|---------:|---------:|--------:|----------:|-----------:|--------:|--------:|
| 4096 | 1  | 25   | 1.3   | 0 | 179   | 131   | 111   | 27 % | 11 % |
| 4096 | 4  | 121  | 6.1   | 0 | 837   | 589   | 518   | 30 % | 9 % |
| 4096 | 8  | 361  | 21.4  | 0 | 2 556 | 1 662 | 1 494 | 35 % | 7 % |
| 4096 | 16 | 1225 | 83.2  | 0 | 8 186 | 5 051 | 4 784 | 38 % | 3 % |
| 4096 | 30 | 3969 | 290.6 | 0 | 24 950 | 15 332 | 14 709 | 39 % | 2 % |

From ratio 1→30: `scan` ×159, `base_ns` ×139 — wall-time tracks **cells scanned**, not
neighbours returned (`ret/call` stays ≤ 291 while `scan` hits 3969). The scan visits
mostly-empty cells.

**Small world** (`cols = 20`; at ratio ≥ 16 the scan wraps and revisits) — the toroidal
double-return:

| N | ratio | scan | ret/call | dup/call | base_ns | inline_ns | scratch_ns | H2_save | H3_save |
|--:|------:|-----:|---------:|---------:|--------:|----------:|-----------:|--------:|--------:|
| 1024 | 8  | 361  | 515.8    | 0       | 16 548  | 6 236  | 5 485  | 62 % | 5 % |
| 1024 | 16 | 1225 | 3 132.7  | 2 108.7 | 48 459  | 11 766 | 10 147 | 76 % | 3 % |
| 1024 | 30 | 3969 | 10 158.4 | 9 134.4 | 151 948 | 34 293 | 30 033 | 77 % | 3 % |
| 4096 | 30 | 3969 | 40 638.9 | 36 542.9| 529 444 | 120 859| 113 682| 77 % | 1 % |

At ratio 30 in a 20-cell world, **~90 % of returned ids are duplicates** (36 543 / 40 639),
each re-probed through the `HashMap` and (in the real stepper) re-inserted into the phase's
`seen` set. This is the `sample:147` regime, and it is why dropping the side `HashMap`
(H2) removes **77 %** of per-call time there.

## Verdicts

### H1 — large radius ÷ small `cell_size` → near-full-grid scan, + toroidal cell revisits — **CONFIRMED**

Evidence: Tier 1 — sensing/movement is 91.9 % of the step on the large-radius config, and
`query_radius` 47 % inclusive. Tier 2 — `base_ns` scales ~linearly with the cell-scan term
(×139 over ratio 1→30, wide world) while neighbours returned stay tiny; in the small-world
regime ~90 % of returns are toroidal duplicates. The grid is built with `cell_size =
light_competition_radius.max(1.0)` (`lib.rs:1090`), which is *unrelated* to the sensing /
reach radius (`sensing_range_coefficient` up to 30); when sensing ≫ cell_size and
`world_extent` is small, `cells_to_check` exceeds `cols` and the scan wraps over the whole
grid, revisiting cells.

**Recommended fix (follow-up):** size the grid `cell_size` to the **largest** query radius
the config will issue (not just `light_competition_radius`); and **cap `cells_to_check` at
`cols` and visit each cell at most once** (dedup the wrapped cell coordinates), which both
bounds the scan and eliminates the double-returns at the source — removing the downstream
`seen`-dedup cost the phases currently pay. This is on the real `explorers-search` hot path,
so it needs its own issue + a regression guard against `bench_query_radius`.

### H2 — per-candidate `self.positions[&id]` `HashMap` probe — **CONFIRMED (largest single cost)**

Evidence: Tier 1 — ~59 % of step self-time is `HashMap`/SipHash. Tier 2 — inlining
positions into the cell buckets removes 27–77 % of per-call time (`H2_save`), rising with
population and with the wrapping regime. The keys are dense `0..N` slice indices, so SipHash
is gratuitous overhead.

**Recommended fix (follow-up):** store positions **inline** in the cell buckets
(`Vec<(u64, (f32,f32))>`) and drop the side `HashMap` entirely (this is the `inline` variant
benchmarked here). If a side map must stay, at minimum swap `RandomState` for a fast
integer hasher (e.g. `FxHashMap`) — the same applies to the phases' `seen` `HashSet`s, which
hash the same dense indices. Regression-guard with `bench_query_radius`.

### H3 — per-call `Vec<u64>` allocation — **REFUTED (real, but negligible where it matters)**

Evidence: Tier 2 — a reused scratch buffer saves only 0–5 % on the expensive queries
(`H3_save`); the 16–21 % figures appear only on the cheapest ratio-1 calls, where the
allocation is a large share of otherwise-trivial work. On the Tier 1 flamegraph, allocation
is ~0 %. A scratch buffer / visitor-callback variant is a reasonable tidy-up but is **not** a
hot-path win and should not be prioritised over H1/H2.

## Non-goals honoured

No change to the stepper, `query_radius` semantics, the evaluator, or the search — all 273
`explorers-sim` lib tests pass unchanged, and the diff is confined to the two new driver/bench
bins plus the additive `role_emergence` instrumentation. `cargo fmt` clean. The committed
artifacts are the harness and the numbers above; the flamegraph is local and transient.
