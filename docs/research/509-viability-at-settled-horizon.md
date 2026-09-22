# Research #509 — Viability's tightness and permanence claims re-checked at the settled horizon

**Status: measurement. Commits nothing in the stepper, evaluator or search; changes two
instruments' shape and one instrument's read.** [Viability](../system-design/viability.md) stated
its solar-ceiling tightness and its permanence failure-cell cross-check at the 500-tick horizon
([433 B2](433-energy-bound.md), [439](439-permanence-crosscheck.md)). The horizon is now `T = 2000`
(#507) and the atlas reads the settled community; this note re-measures both claims there with
`energy_bound_check` and `permanence_crosscheck`, both rebuilt on the resumable JSON-lines shape
of `settling_time` / `energy_death_check` (`explorers_search::sweep`), and records what moved.
Parent: #505 (grace 260), #506 (incremental gates, carry-to-`T`), #507 (`T = 2000`), #508
(sustainable-stock read). Stepper at `1633490`.

## TL;DR

1. **The solar ceiling holds at `T = 2000`, and the income cap is now reached exactly.** Over 2189
   checked runs no run violates Lemma 1, the Theorem or the summed Lemma 2. The per-tick income cap
   `P(t) ≤ F·min(N_P, m²)` has observed/bound median **1.000** (q1 0.83, max 1.000; 1361 runs at
   `≥ 0.999`) — up from 0.93 at 500 ticks: a settled community is sparse and a producer alone
   within `r` takes the full flux. The count bound's slack is unchanged in order: mean `N_s / N̄_max`
   median **0.0032**, max **0.054** (was 0.0041 / 0.067); realised form median **0.120**, max
   **0.759** (was 0.119 / 0.759). The claim stands; viability.md's numbers are replaced in place.
2. **The permanence positive direction breaks for `I > 1`: two strict false positives on 56
   predicted-permanent cells** (was 0/50). `sample:12` is extinct 8/8 (every founder dead on tick 1
   on seven seeds; the eighth seed's lone survivor dies at tick 573 — at 500 ticks it was still
   alive, which is why the cell was mixed then). `sample:31` (`ρ = 86`, `I = 1.40`) is
   nutrient-lockup 8/8 at `T` — a bloom-then-lockup cell. **`Λ > 1` goes from two strict failures
   to six** on 194: `atlas:2`, `sample:12`, `31`, `154`, `158`, `192`, all lockup at `T`. Recorded
   in viability.md where the claim stands; a `needs-triage` issue names the cells for the designer.
   Nothing is rewritten in the theory.
3. **The `κ_C = 0` re-take is done.** With `β = χ_C·γ·e·ĥ_C` (#482) the 19 atlas cells at
   `mean_kappa = 0` read `Λ` permanent on all 19 (5 persist on all seeds, 14 mixed, none collapses
   8/8) and `I` not-permanent on 17 for the trait-distance reason the rest of the box shows — no
   longer the coefficient. The negative direction of `I` is still wrong on 183 of 188 observed cells.
4. **The living-energy hoard has grown.** The reproductive allocation holds 63 % of living energy at
   the median survivor's peak (was 36 %), more than half on 1247 of 2059 survivors, and 979
   survivors' stocks are still rising at `T` — as an untaxed stock must (viability's open finding).
5. **Timeouts.** 67 energy-bound runs (16 configs) and 70 permanence runs (16 configs) hit the 300 s
   guard and are excluded, not counted as violations; three permanence cells have no finished seed.

## 1. What ran

| | energy_bound_check | permanence_crosscheck |
|---|---|---|
| configs | 82 atlas live cells + 200 LHS points (seed 421) = 282 | same |
| seeds | 8 per config (`1000..1008`) → 2256 runs | same |
| horizon | 2000 ticks (`SearchConfig::max_ticks`) | same |
| stops | extinction, explosion (`run_single`'s stops), else carried to `T` | extinction, explosion; else carried to `T` and verdicted by `evaluate_from_log` on the full series |
| run timeout | 300 s wall clock per (config, seed) → mode `timeout` | same |
| output | `target/energy-bound-check.jsonl`, SHA-1 `1b559e07cd43f99085a5bd074d784a50f5a272ee` (282 rows) | `target/permanence-crosscheck.jsonl`, SHA-1 `e92eb3709796251ea6e3c0dd6d506b777b16b731` (282 rows) |
| check file | `target/energy-bound-check-check.jsonl` (5 rows re-run after a memory change to the bin: `atlas:0, 81`, `sample:3, 59, 199`, every row byte-identical to the main file's; `atlas:59` re-run too — 4 timeouts both times, at wall-clock-dependent ticks) | — |
| wall clock | ≈ 2 h in foreground chunks (`--limit 1` on the dense sample configs, 20–50 elsewhere) | ≈ 6 h; the dense configs cost 15–60 min each (below) |

**Which claim needs `T`.** The income cap is a per-tick inequality and the count bound is checked at
every prefix `T' ≤ T`, so a run stopped early — by extinction, explosion or the guard — contributes
every tick it ran. The instrument nonetheless excludes the 67 timed-out runs from the distributions
and the violation list (their ticks are not read), and the summary states their count; none of
their ticks violates a bound either (an informational read off the rows). Permanence is a statement
about the state at `T`: the observed verdict is the evaluator's full-series classification at the
horizon, a dead-pool stop is not taken as the verdict (the carry-to-`T` shape of #506), and a
timed-out seed is neither collapse nor persistence — a cell's ensemble is read over its finished
seeds, and a cell with none is `unobserved` and enters no confusion matrix.

**Both bins are resumable.** One JSON line per config appended as it completes, configs already in
the file skipped, `--limit N`, fixed sweep order and seed block; a unit test in each bin runs a
three-config sweep once and again split `--limit 2` / `--limit 1` and asserts the two files are
byte-identical. The single-shot JSON artifact and the `ENERGY_BOUND_*` / `PERMANENCE_CROSSCHECK_*`
environment selectors are gone; `--configs` and `--seeds` replace them.

### The first energy-bound sweep was set aside

The 500-tick instrument read the tick's solar income as the increment of the world's cumulative
`total_solar_input`, an `f32` counter. At the 2000-tick horizon that counter is large enough that a
tick's income is lost to its ULP: on `sample:131` seed 1000 the differenced read exceeded the cap by
`1.1e-4` on 44 ticks (a "violation" beyond the `1e-4` tolerance) while the per-tick energy ledger —
rebuilt every tick from that tick's `Photosynthesized` events — reads exactly 1.000 on the same
ticks. The bin now reads `P(t)` off `world.energy_ledger().total_solar_input()`. The first sweep,
complete to `sample:131` with the differenced read, is kept as
`target/energy-bound-check-cumulative-read.jsonl` (SHA-1 `72fa7f41a500a8aa24176ba0069b0c30c387b260`,
150 rows) and is not used: every number below is from a fresh sweep with the ledger read. The
first 36 rows of the two sweeps agree on every run's terminal mode, tick count and Lemma 1 ratio to
within the read's `2e-4` noise.

## 2. The solar ceiling at `T = 2000`

Terminal modes: 2059 survived, 130 extinct, 0 exploded, 67 timed out (excluded); no NaN; 0 of
66 800 founders culled at tick 1. **No violation of Lemma 1, the Theorem, or the summed Lemma 2 on
the 2189 checked runs.**

| Check | observed / bound | n | min | q1 | median | q3 | max |
|---|---|---|---|---|---|---|---|
| 1 Lemma 1 | `max_t P(t) / (F·min(N_P, m²))` | 2189 | 0.113 | 0.833 | **1.000** | 1.000 | 1.000 |
| 2 Theorem | `max_T B·ΣN_s / (E_tot(0) + T·P_max)` | 2189 | 0.000 | 0.0030 | 0.0074 | 0.0157 | 0.116 |
| 2 Lemma 2 summed | `max_T B·ΣN_s / (E_tot(0) − E_tot(T) + ΣP)` | 2189 | 0.000 | 0.060 | **0.120** | 0.202 | **0.759** |
| 2 asymptote | `mean_t N_s / N̄_max` | 2189 | 0.000 | 0.0013 | **0.0032** | 0.0064 | **0.054** |
| 3 envelope | `max_t E_living / (E_tot(0) + t·P_max)` | 2189 | 0.000 | 0.0049 | 0.0116 | 0.0225 | 0.946 |
| 3 allocation share at peak | `Σ repro_reserve / E_living` | 2189 | 0.000 | 0.170 | **0.635** | 0.876 | 0.9999 |
| 3 allocation share, survivors | same | 2059 | 0.000 | 0.201 | 0.668 | 0.884 | 0.9999 |
| 3 growth | `max_t E_living / E_tot(0)` | 2189 | 0.000 | 3.87 | 22.3 | 89.8 | 21 764 |

**Lemma 1** is tight on the `N_P` branch and reached exactly: median 1.000 (atlas 0.875, sample
1.000), 1361 of 2189 runs at `≥ 0.999`, and the maximum is 1.000 to the ledger's f32 (no run above
`1 + 1e-4`). The rise from 0.93 is the settled community: at 2000 ticks the run has spent most of
its ticks past the bloom, where producers are sparse and a lone producer within `r` takes the whole
flux at least once. The `m²` branch is exceeded by the peak population in 593 runs (was 558) and
never binds as the tighter cap. **The Theorem** holds with two orders of magnitude to spare and the
realised form is essentially where it was: the worst cases are the same cells (`sample:155` seed
1007 at 0.759; `sample:192` seeds 1007 / 1006 at 0.756 / 0.746). The asymptote's maximum moved from
`sample:70` (0.067, `N̄_max = 51`) to `atlas:10` seed 1000 (0.054, `N̄_max = 4957`, peak 434). **The
obstruction** grew: the allocation's share of peak living energy has median 0.635 (was 0.357),
1247 of 2059 survivors hold more than half of their living energy in it (was 815 of 2180), and 979
survivors peak at `T` itself; the extreme is `sample:96` seed 1001 at 21 764× its endowment
(951 253 E, 67 % allocation), with `sample:68` seed 1007 at 2 198 400 E (99.8 % allocation).

### Timeouts (excluded)

| config | timed-out seeds | tick reached at 300 s |
|---|---|---|
| `atlas:59` | 4 | 1170–1965 |
| `sample:20` | 7 | 438–1434 |
| `sample:24` | 4 | 402–1350 |
| `sample:38` | 2 | 977–1567 |
| `sample:45` | 2 | 1398–1525 |
| `sample:55` | 3 | 1337–1775 |
| `sample:100` | 2 | 1267–1505 |
| `sample:129` | 8 | 609–1307 |
| `sample:131` | 2 | 547–1211 |
| `sample:146` | 2 | 1676–2000 |
| `sample:147` | 8 | 724–1751 |
| `sample:151` | 1 | 1561 |
| `sample:154` | 8 | 522–1953 |
| `sample:165` | 7 | 597–1384 |
| `sample:169` | 3 | 1193–1827 |
| `sample:197` | 4 | 832–1926 |

67 runs on 16 configs — the dense worlds of [505](505-settling-time.md) §4 (whose 3000-tick list
this matches but for `sample:68`, `130`, `164` finishing and `atlas:59`, `sample:20` joining). `atlas:59` is the
one live cell among them; its four finished seeds are in the distributions.

## 3. Permanence at `T = 2000`

Modes over 2256 runs: 1514 `none`, 452 nutrient-lockup, 130 extinction, 59 monoculture, 18
energy-death, 13 generalist-dominance, 70 `timeout`. Collapses (extinction | energy-death |
lockup): 600 runs (was 197 at 500 ticks) — lockup at the settled horizon is the dominant collapse,
as [505](505-settling-time.md) saw at 3000. Observed per cell: 105 persist on every finished seed,
165 mixed, 9 collapse on every finished seed, 3 unobserved (`atlas:59`, `sample:20`, `sample:129`:
every seed timed out). Atlas: 21 persist, 59 mixed, 1 collapse (`atlas:2`), 1 unobserved.

### False-positive rate — the dangerous direction

| | predicted permanent (observed) | strict FP (all finished seeds collapse) | any-seed FP |
|---|---|---|---|
| A1 `ρ ∧ I` | 56 | **2** (3.6 %) | 38 (68 %) |
| A2 `Λ` (coupled) | 194 | **6** (3.1 %) | 114 (59 %) |

At 500 ticks: A1 0/50, 13 any-seed; A2 2/176, 50 any-seed. The predicted-permanent counts moved
because `β` now carries `χ_C` (#482), not because of the horizon.

**A1 strict false positives:**

- **`sample:12`** — `ρ = 11.2`, `I = 2.19`, `Λ = 7960`, `κ_C = 0.33`; 31 founders. Extinct 8/8:
  on seven seeds every founder dies on tick 1 (the peak-relative death threshold, endowment and
  embodiment floor the biomass map has no coordinate for — the *reduction regime only* clause); on
  seed 1000 one founder survives tick 1 and dies at tick 573. At 500 ticks that seed was alive and
  the cell was mixed; the settled horizon makes it strict. Fault: reduction regime exit at tick 0.
- **`sample:31`** — `ρ = 86`, `I = 1.40`, `Λ = 2674`, `κ_C = 0.01`; 31 founders, peak 80–127.
  Nutrient-lockup 8/8 at `T`, every seed reaching the horizon. Fault: lockup is outside the
  two-compartment map's coordinates; `I` is within 40 % of the boundary.

**A2 strict false positives** (all nutrient-lockup at `T`, all at `ι = 1`):

| cell | `Λ` | `I` | `κ_C` | founders | peak | seeds | note |
|---|---|---|---|---|---|---|---|
| `atlas:2` | 1166 | 0.90 | 0.19 | 37 | 53 | 8/8 lockup | a live cell (500-tick atlas); A1 says not-permanent |
| `sample:12` | 7960 | 2.19 | 0.33 | 31 | 31 | 8/8 extinction | above |
| `sample:31` | 2674 | 1.40 | 0.01 | 31 | 111 | 8/8 lockup | above |
| `sample:154` | 128 | 0.05 | 0.27 | 21 | 1142 | 1/1 lockup, 7 timeouts | one finished seed; strict on `n = 1` |
| `sample:158` | 20.0 | 0.005 | 0.08 | 44 | 466 | 6/8 lockup, 1 extinction, 1 energy-death | new |
| `sample:192` | 99.2 | 0.03 | 0.05 | 10 | 411 | 8/8 lockup | as at 500 ticks |

`sample:147` (`Λ = 25.7`), the first-named cell of this class, has two finished seeds, both
monoculture — not a collapse on this read. The fault is the one A2 names: the reference lumping
sets every carcass within every consumer's reach, and the settled horizon exposes it on more cells
because a bloom that silts its nutrient does so after tick 500 (36 of the 108 mixed cells under a
permanent `Λ` are modal-lockup).

### The negative direction

`I ≤ 1` on 188 observed cells: 183 persist on at least one finished seed, 77 on all, 161 with
consumers at `T`. `I` has median 0.35 over the box, `Λ` median 763, `ρ ≥ 1.9` everywhere. The
evaluator's decomposer-guild observable is seen on the persisting seeds of only 7 of those 161
cells (the earlier cross-check counted 172 of 180 on its own guild read); the roster shows the
pile route persisting, the guild signature does not, and which of the two moved — the read (#490)
or the settled community — this sweep does not settle.

The 19 `κ_C = 0` atlas cells: `I` not-permanent on 17, permanent on 2; `Λ` permanent on all 19;
observed 5 persist, 14 mixed, 0 collapse. The earlier reading (`I = Λ = 0` from the `κ_C` lumping)
is retired.

### Fault classes of the mixed cells under a permanent prediction

A1 (36 cells): demographic-stochastic 21, reduction 14 (lockup or energy-death on a minority of
seeds), spatial (consumers lost by tick 25) 1. A2 (108): demographic-stochastic 60, reduction 39
(36 of them modal-lockup), spatial 9.

### Timeouts (excluded)

| config | timed-out seeds | tick reached at 300 s |
|---|---|---|
| `atlas:59` | 8 | 988–1951 |
| `sample:20` | 8 | 407–1533 |
| `sample:24` | 4 | 376–1235 |
| `sample:38` | 2 | 951–1467 |
| `sample:45` | 2 | 1345–1469 |
| `sample:55` | 3 | 1129–1537 |
| `sample:100` | 2 | 1180–1304 |
| `sample:129` | 8 | 631–1347 |
| `sample:131` | 2 | 618–1379 |
| `sample:146` | 1 | 1872 |
| `sample:147` | 6 | 937–1729 |
| `sample:151` | 1 | 1615 |
| `sample:154` | 7 | 600–1998 |
| `sample:165` | 7 | 654–1506 |
| `sample:169` | 3 | 911–1246 |
| `sample:197` | 6 | 675–1919 |

70 runs on 16 configs. The permanence bin is slower per tick than the energy-bound bin on the same
worlds because it carries the evaluator's observations (`RolloutObservations::observe`, whose
trophic-role snapshot every 10 ticks is linear in agents × retained edges), so a dense seed that
the energy-bound bin carried to `T` inside 300 s can time out here (`atlas:59`: 4 vs 8 timeouts),
and a step-plus-observe past the budget can itself take many minutes — the dense configs cost
15–60 min of wall clock each. That cost is the evaluator's, not this instrument's, and is noted
rather than changed.

## 4. Consequences

- viability.md: the *Tightness* bullet, the open finding's hoard numbers, the *failure cells*
  section and the two matching *Findings and follow-ups* bullets are updated in place, present
  tense, horizon stated. The two `I > 1` strict failures and the four new `Λ` failures are recorded
  where the claims stand; the theory is not rewritten.
- #517 (`needs-triage`) names the cells and numbers for the designer.
- `energy_bound_check` reads `P(t)` off the per-tick ledger (above) and keeps no event log;
  `permanence_crosscheck` retains only the evaluator's event kinds and compacts the log as
  `run_single` does. Both are resumable with `--limit`, `--out`, `--configs`, `--seeds`,
  `--run-timeout-secs`, `--summary`.

## 5. Reproducing

```sh
cargo build --release -p explorers-search --bin energy_bound_check --bin permanence_crosscheck
# resume-safe; repeat until "running 0 now"; --limit 1 on the dense sample configs
./target/release/energy_bound_check --limit 5
./target/release/energy_bound_check --summary
./target/release/permanence_crosscheck --limit 5
./target/release/permanence_crosscheck --summary
```

Rows are appended per completed config in a fixed order, so a loop of short calls produces a file
byte-identical to one uninterrupted run; a timed-out row's `termination_tick` / `ran_ticks` is
wall-clock-dependent and is not expected to reproduce. The per-cell reads above were taken off the
JSON-lines files with a short script; each summary prints its distributions and confusion matrices.
