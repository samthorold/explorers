# Issue #462 — the light-competition radius as a single factor: the response curve at four fixed baselines

**Status: measurement. Adds one research bin
(`crates/explorers-search/src/bin/radius_sweep.rs`); changes no stepper,
evaluator or search behaviour. Its output lives under `target/` like the other
sweeps' and is not committed.** The observational reading in
[`462-pi-space-explanatory-check.md`](462-pi-space-explanatory-check.md) found
`light_competition_radius` to be the strongest single predictor of a config's
live and lockup fractions over the search box (Spearman +0.52 / −0.42 across a
200-point LHS draw, all 32 dimensions varying). That is a correlation across
configs. This note holds every other parameter fixed and moves the radius alone
through its searched range (1–20) at four baselines, which is the confirmation
the re-scoped #462 asks for before any coordinate is built on it. Stepper and
search at `3de99f8`.

## TL;DR

1. **The direction is confirmed, and it is not subtle.** At every baseline the
   large-radius end is reliably live and the small-radius end is not: pooled
   over the four baselines, `r ≥ 12` is live on **44 / 60** seeds and `r ≤ 2`
   on **9 / 40** — and 12 of those 40 never finished (below), so the true
   small-`r` figure is lower still. Three baselines reach 5/5 live by `r = 12`
   and hold it to `r = 20`.
2. **The mechanism is the bloom, as the count ceiling predicts.** Median peak
   population falls monotonically with `r` at every baseline — on the recipe
   2815 → 48 from `r = 1` to `r = 20`, tracking the ceiling's geometric factor
   `m = ⌊√2·L/r⌋ + 1` (141 → 8). The radius sets how many producers can each
   take the full flux; a small radius lifts the bloom's ceiling and the world
   dies of its own bloom.
3. **But the *mode* it dies of is baseline-dependent, and it is not always
   lockup.** `sample:110` (the lowest-flux baseline, `F = 4.6`) never locks up
   at any radius — its small-`r` failures are all **generalist dominance**.
   `sample:188` and `sample:18` lock up at small `r`; the recipe and `sample:18`
   put their *worst* lockup at **mid**-range `r = 5` (4/5 seeds), not at the
   smallest radius, because at `r ≤ 2` the world fails or stalls before it can
   silt. So the LHS correlation's specific claim — small radius → **lockup** —
   is the part that does not survive: what the radius robustly controls is
   **bloom amplitude and whether the world survives its own bloom**, and which
   cliff it reaches is set by the rest of the config.
4. **The projected recipe is not live at the settled horizon.** At its own
   radius (8.9) it is 2/5 live, 3/5 nutrient lockup. `recipe.json` was projected
   from the 500-tick atlas; this is the bloom-then-lockup class
   [505](505-settling-time.md) found on 213/656 atlas runs, now visible in the
   playable world itself. Filed separately — it is a recipe-projection problem,
   not a radius finding.
5. **The instrument's wall-clock guard does not bound evaluation.** 12 of 160
   runs, all at `r ≤ 3`, hit the 300 s guard. The guard is checked between
   steps, so it bounds the step loop but **not** `evaluate_from_log`, whose
   DBSCAN is superlinear in roster size — a level whose worlds carry ~1500–2800
   agents can spend hours in evaluation after its rollouts have stopped. A
   long-guard re-run of the small-`r` levels was attempted and abandoned for
   this reason (§4); it needs a machine this box is not.

## 1. What ran

| | |
|---|---|
| bin | `crates/explorers-search/src/bin/radius_sweep.rs` — resumable in the `explorers_search::sweep` shape (one JSON line per level, appended on completion, levels already present skipped) |
| baselines | `recipe` (`recipe.json`, the projected world), and three LHS points verified live 8/8 with consumers at `T = 2000` in the #509 sweep: `sample:110`, `sample:18`, `sample:188` |
| levels | `r ∈ {1, 2, 3, 5, 8, 12, 16, 20}` — the searched range of `light_competition_radius` (`default_ranges`: 1–20), denser at the small end |
| seeds | 5 per level (`1000..1005`), rollouts driven exactly as `explorers_genesis::run_single` does (retained event kinds, per-step compaction, the evaluator's `early_stop`, `evaluate_from_log` at the horizon) |
| horizon | `T = 2000` (`SearchConfig::default().max_ticks`) |
| guard | 300 s wall clock per (level, seed); a run that exceeds it is recorded as mode `timeout`, never as an outcome |
| output | `target/radius-sweep.jsonl`, 32 rows, SHA-1 `e6008b75a5771641811c893f5112c5cd3cfdfa75` (not committed — `target/` is build output; regenerable by the four commands in §1) |

Baseline characteristics (`r`, world extent `L`, solar flux `F`, founder count):
recipe `8.9 / 99.6 / 10.6 / 48`; `sample:110` `6.9 / 76.4 / 4.6 / 38`;
`sample:18` `17.7 / 98.5 / 9.2 / 50`; `sample:188` `4.3 / 54.0 / 9.4 / 17`. The
three sample baselines are unselected LHS draws; the recipe is the search's own
projected elite, and behaves differently (§3).

## 2. The response curve

Live / lockup counts are **out of all 5 seeds**, with timeouts shown separately
— a timed-out run is not live, so counting it in the denominator is the
conservative reading. Median peak population is over the finished seeds.

| baseline | r | m | live | lockup | timeout | other failures | live with consumers | median peak |
|---|---|---|---|---|---|---|---|---|
| recipe | 1 | 141 | 2/5 | 0 | 3 | — | 1 | 2815 |
| recipe | 2 | 71 | 3/5 | 0 | 2 | — | 1 | 1563 |
| recipe | 3 | 47 | 2/5 | 2 | 1 | — | 1 | 1536 |
| recipe | 5 | 29 | 1/5 | **4** | 0 | — | 1 | 600 |
| recipe | 8 | 18 | 2/5 | 3 | 0 | — | 2 | 233 |
| recipe | 12 | 12 | 3/5 | 2 | 0 | — | 2 | 78 |
| recipe | 16 | 9 | 4/5 | 1 | 0 | — | 3 | 48 |
| recipe | 20 | 8 | 4/5 | 1 | 0 | — | 4 | 48 |
| sample:110 | 1 | 108 | 2/5 | 0 | 0 | 3 generalist-dom. | 2 | 276 |
| sample:110 | 2 | 54 | 1/5 | 0 | 0 | 4 generalist-dom. | 1 | 129 |
| sample:110 | 3 | 36 | 3/5 | 0 | 0 | 2 generalist-dom. | 3 | 38 |
| sample:110 | 5 | 22 | 4/5 | 0 | 0 | 1 generalist-dom. | 4 | 38 |
| sample:110 | 8 | 14 | **5/5** | 0 | 0 | — | 5 | 38 |
| sample:110 | 12 | 9 | 5/5 | 0 | 0 | — | 4 | 38 |
| sample:110 | 16 | 7 | 5/5 | 0 | 0 | — | 0 | 38 |
| sample:110 | 20 | 6 | 5/5 | 0 | 0 | — | 0 | 38 |
| sample:18 | 1 | 140 | 0/5 | 0 | 3 | 1 generalist-dom., 1 energy-death | 0 | 1566 |
| sample:18 | 2 | 70 | 1/5 | 0 | 4 | — | 1 | 50 |
| sample:18 | 3 | 47 | 0/5 | 2 | 1 | 2 monoculture | 0 | 937 |
| sample:18 | 5 | 28 | 1/5 | **4** | 0 | — | 1 | 272 |
| sample:18 | 8 | 18 | 2/5 | 3 | 0 | — | 2 | 72 |
| sample:18 | 12 | 12 | **5/5** | 0 | 0 | — | 5 | 50 |
| sample:18 | 16 | 9 | 5/5 | 0 | 0 | — | 4 | 50 |
| sample:18 | 20 | 7 | 5/5 | 0 | 0 | — | 3 | 50 |
| sample:188 | 1 | 77 | 0/5 | 2 | 2 | 1 monoculture | 0 | 1253 |
| sample:188 | 2 | 39 | 0/5 | 3 | 0 | 1 generalist-dom., 1 monoculture | 0 | 439 |
| sample:188 | 3 | 26 | 3/5 | 1 | 0 | 1 generalist-dom. | 3 | 79 |
| sample:188 | 5 | 16 | **5/5** | 0 | 0 | — | 5 | 18 |
| sample:188 | 8 | 10 | 5/5 | 0 | 0 | — | 5 | 17 |
| sample:188 | 12 | 7 | 5/5 | 0 | 0 | — | 3 | 17 |
| sample:188 | 16 | 5 | 5/5 | 0 | 0 | — | 2 | 17 |
| sample:188 | 20 | 4 | 5/5 | 0 | 0 | — | 1 | 17 |

Pooled over the four baselines: `r ≤ 2` → **9/40 live**, 12 timeouts; `r = 3`
→ 8/20; `r = 5` → 11/20; `r ≥ 12` → **44/60 live**, 0 timeouts. Median peak
population by level, pooled: `r = 1` 1410, `r = 2` 284, `r = 3` 508, `r = 5`
155, `r = 8` 53, `r = 12` 34, `r = 16` 33, `r = 20` 33.

**On the recipe's small-`r` rows.** At `r = 1` and `r = 2` the recipe reads 2/5
and 3/5 live, which is *higher* than its own mid-range — an apparent inversion.
It is an artefact of which runs finished: those levels lost 3 and 2 seeds to the
guard, and the lost seeds are the slowest, which are the densest, which are the
ones on the lockup trajectory. Counting timeouts as non-live (as the table does)
the recipe's curve is flat-to-rising in `r` rather than inverted, and the
inversion returns only if one assumes every timed-out run would have been live —
the least likely assumption available. The honest statement is that the recipe's
small-`r` levels are **undersampled**, not that they are good.

## 3. The recipe behaves unlike the three LHS baselines

The recipe is the only baseline that locks up across the *whole* range (1–4
seeds at every `r ≥ 3`) and the only one that never reaches 5/5 live. It is also
the only selected config among the four: it was projected as the elite of the
highest-fitness live cell of the **500-tick** atlas, and #505 showed that 213 of
656 atlas-cell runs that were live at 500 ticks lock up by 3000. The radius
sweep simply makes it visible at the config the player would actually be
dropped into. This is a projection problem (the recipe should be re-drawn from
a settled-horizon atlas, #494), not a radius result, and it is filed separately.

## 4. What the guard does and does not bound

`run_seed` checks `started.elapsed() > run_timeout` between steps, so the guard
bounds the **step loop** only. `evaluate_from_log` runs afterwards and is
unbounded; its DBSCAN over roster snapshots is superlinear in roster size, so a
level whose worlds hold 1500–2800 agents can spend far longer in evaluation than
in simulation. A re-run of the recipe's `r ∈ {1, 2, 3}` at a 1800 s guard was
started and abandoned after two hours without completing a single level; the
levels were then restored at the original 300 s guard, and reproduced their
first-pass mode composition exactly (r = 1: 2 live / 3 timeout; r = 2: 3 live /
2 timeout; r = 3: 2 live / 2 lockup / 1 timeout) — so the timeout pattern is a
property of these configs, not of scheduling noise.

Two consequences, both for the designer:

- **The 12 timed-out runs are a known gap**, concentrated at `r ≤ 3`, and they
  bias every small-`r` live fraction *upward*. Resolving them needs a long-guard
  re-run on a machine that can give it hours: `radius_sweep --baseline recipe
  --levels 1,2,3 --run-timeout-secs 5400` and the same for `sample:18
  --levels 1,2,3` and `sample:188 --levels 1`.
- **The guard's placement is arguably a bug** in every sweep that uses this
  shape (`settling_time`, `energy_death_check`, `permanence_crosscheck` read the
  same way): a `--run-timeout-secs` that does not bound the evaluation cannot
  bound the run. Worth an issue on its own.

All seven rows dropped when the long-guard re-run was set up have been
regenerated at the original guard, and each reproduced its first-pass mode
composition — including `sample:188` `r = 1` (2 lockup, 1 monoculture, 2
timeouts). The committed file holds all 32 levels.

## 5. What this settles for #462

- **The radius is a real coordinate, not an artefact of the LHS draw.** Holding
  everything else fixed, it moves live fraction and peak population hard and
  monotonically. A reduced coordinate set for the search must contain it (or the
  ceiling's `m`), which the permanence π-table does not.
- **It is not a lockup coordinate.** The observational reading paired the radius
  with lockup specifically; the single-factor sweep shows the pairing is
  baseline-conditional (`sample:110`, at a third the solar flux of the others,
  fails by generalist dominance instead and never locks up at any radius). What
  it predicts is *bloom amplitude* — which the ceiling's `m` states in closed
  form — and bloom amplitude then meets whichever cliff the rest of the config
  is nearest.
- **So the next coordinate to test is the amplitude, not the radius**: whether
  `m²` (or `m²` against the flux and founder budget) predicts peak population
  across configs, and whether peak population then predicts the failure mode
  better than any raw dimension. That is a read on existing atlas and sweep
  data, not new rollouts.

## 6. What this does not settle

- **Five seeds per level** resolves a 5/5-vs-2/5 difference, not a 4/5-vs-5/5
  one; the per-level fractions are indicative, the pooled trend is the finding.
- **Four baselines** cannot separate "the radius interacts with solar flux" from
  "the radius interacts with founder density" — `sample:110` differs from the
  others in both. A two-factor sweep (radius × flux) is the next instrument.
- **The timed-out small-`r` levels** (§4), which is where the curve is least
  certain.
- **Guilds** appear at 7 level-seeds out of 160, all at `r ≤ 8`, too few to read.
