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
the re-scoped #462 asks for before any coordinate is built on it.

**Re-run on current code (#524).** The first pass ran at `3de99f8`. This note
now reads the second pass (v2), the whole sweep re-run at `fe18ead` into a fresh
file. Between the two, the recipe was regenerated (#545), the stepper changed so
starving agents die on the tick their charge is capped (#541), generalist
dominance is scored per agent instead of on cluster means (#543), and each run
got a separate, bounded evaluation budget (#532). A partial backfill would have
spliced two codebases into one curve, so no v1 row is reused. Where v1 and v2
disagree, §2 says so.

## TL;DR

1. **The direction is confirmed, and it has sharpened.** Pooled over the four
   baselines, `r ≥ 12` is live on **60 / 60** seeds (v1: 56 / 60), with zero
   timeouts. `r ≤ 2` is live on **13 / 40**, and 10 of those 13 are
   `sample:110` (item 3). On the other three baselines `r ≤ 2` is live on
   3 / 30, with 6 more that never finished. All four baselines are 5/5 live
   at every `r ≥ 12`.
2. **The mechanism is still the bloom.** Pooled median peak population falls
   monotonically with `r`: 1425 at `r = 1`, 550 at `r = 2`, 278 at `r = 3`,
   then about 42–44 from `r = 5` on. On the recipe it falls from 4936 at
   `r = 1` to 45 at `r ≥ 12`. This tracks the ceiling's geometric factor
   `m = ⌊√2·L/r⌋ + 1`: the radius sets how many producers can each take the
   full flux, so a small radius lifts the bloom's ceiling.
3. **Which cliff a small radius reaches depends on the baseline, and one
   baseline reaches none.** The recipe's small-`r` end is **lockup** (10/10 at
   `r = 2, 3`). `sample:18` is **monoculture** at `r ≤ 2` and **lockup** at
   `r = 3–8`. `sample:188` is **lockup** with some **monoculture** at `r ≤ 2`.
   `sample:110` is now **live 5/5 at every radius**. Its v1 small-`r` failures
   were all generalist dominance, and none recur. Its parameters are unchanged,
   so a code change removed them. #543 rewrote exactly that verdict, which makes
   it the likely cause, but this sweep does not separate #543 from #541. The
   LHS correlation's specific claim, that small radius means **lockup**, still
   does not survive. What the radius robustly controls is bloom amplitude, and
   whether that is fatal depends on the rest of the config.
4. **The regenerated recipe is mostly live at its own radius.** At `r = 8`,
   next to its own 8.8, it is 4/5 live, 1/5 lockup; v1's recipe was 2/5 live
   there. It is 5/5 live from `r = 12` up. But only 1 of its 4 live seeds at
   `r = 8` still has a consumer at `T` (§3).
5. **The budgets now bound the run.** 6 of 160 runs, all at `r ≤ 2`, exhausted
   the **simulation** budget (1800 s). None exhausted the **evaluation**
   budget. v1 lost 16 runs to a guard that did not bound evaluation (§4).

## 1. What ran

| | |
|---|---|
| bin | `crates/explorers-search/src/bin/radius_sweep.rs`, resumable in the `explorers_search::sweep` shape (one JSON line per level, appended when the level completes; levels already present are skipped) |
| baselines | `recipe` (`recipe.json`, the projected world, as regenerated in #545), and three LHS points verified live 8/8 with consumers at `T = 2000` in the #509 sweep: `sample:110`, `sample:18`, `sample:188` |
| levels | `r ∈ {1, 2, 3, 5, 8, 12, 16, 20}`, the searched range of `light_competition_radius` (`default_ranges`: 1–20), denser at the small end |
| seeds | 5 per level (`1000..1005`), each rollout driven exactly as `explorers_genesis::run_single` drives it (retained event kinds, per-step compaction, the evaluator's `early_stop`, `evaluate_from_log` at the horizon) |
| horizon | `T = 2000` (`SearchConfig::default().max_ticks`) |
| budgets | `--run-timeout-secs 1800 --eval-timeout-secs 1800` per (level, seed). A run that spends its simulation budget is recorded as mode `timeout`, and one that spends its evaluation budget at the horizon as `eval_timeout`. Neither is an outcome. |
| output | `target/radius-sweep-v2.jsonl`, 32 rows, SHA-1 `a6c984118e5aa3f5d81e4b63d3b4e82b4b3e9d2a` (not committed; regenerate with the four commands below) |

```
radius_sweep --baseline recipe     --out target/radius-sweep-v2.jsonl --run-timeout-secs 1800 --eval-timeout-secs 1800
radius_sweep --baseline sample:110 --out target/radius-sweep-v2.jsonl --run-timeout-secs 1800 --eval-timeout-secs 1800
radius_sweep --baseline sample:18  --out target/radius-sweep-v2.jsonl --run-timeout-secs 1800 --eval-timeout-secs 1800
radius_sweep --baseline sample:188 --out target/radius-sweep-v2.jsonl --run-timeout-secs 1800 --eval-timeout-secs 1800
```

Baseline characteristics (`r`, world extent `L`, solar flux `F`, founder count):
recipe `8.8 / 61.8 / 8.1 / 45` (v1's pre-#545 recipe was
`8.9 / 99.6 / 10.6 / 48`); `sample:110` `6.9 / 76.4 / 4.6 / 38`; `sample:18`
`17.7 / 98.5 / 9.2 / 50`; `sample:188` `4.3 / 54.0 / 9.4 / 17`. The three
sample baselines are unselected LHS draws, and their parameters are identical
in v1 and v2. The recipe is the search's own projected elite, and it is a
different world in v2.

## 2. The response curve

Live and failure counts are **out of all 5 seeds**, with timeouts shown
separately. A timed-out run is not live, so keeping it in the denominator is
the conservative reading. No run had an `eval_timeout`. "Live with consumers"
is a live seed with at least one consumer at `T`. Median peak population is
over the finished seeds.

| baseline | r | m | live | lockup | timeout | other failures | live with consumers | median peak |
|---|---|---|---|---|---|---|---|---|
| recipe | 1 | 88 | 0/5 | 1 | 3 | 1 generalist-dom. | 0 | 4936 |
| recipe | 2 | 44 | 0/5 | **5** | 0 | — | 0 | 1107 |
| recipe | 3 | 30 | 0/5 | **5** | 0 | — | 0 | 432 |
| recipe | 5 | 18 | 2/5 | 3 | 0 | — | 2 | 124 |
| recipe | 8 | 11 | 4/5 | 1 | 0 | — | 1 | 54 |
| recipe | 12 | 8 | **5/5** | 0 | 0 | — | 4 | 45 |
| recipe | 16 | 6 | 5/5 | 0 | 0 | — | 3 | 45 |
| recipe | 20 | 5 | 5/5 | 0 | 0 | — | 1 | 45 |
| sample:110 | 1 | 108 | **5/5** | 0 | 0 | — | 5 | 425 |
| sample:110 | 2 | 54 | 5/5 | 0 | 0 | — | 5 | 119 |
| sample:110 | 3 | 36 | 5/5 | 0 | 0 | — | 5 | 38 |
| sample:110 | 5 | 22 | 5/5 | 0 | 0 | — | 5 | 38 |
| sample:110 | 8 | 14 | 5/5 | 0 | 0 | — | 5 | 38 |
| sample:110 | 12 | 9 | 5/5 | 0 | 0 | — | 4 | 38 |
| sample:110 | 16 | 7 | 5/5 | 0 | 0 | — | 0 | 38 |
| sample:110 | 20 | 6 | 5/5 | 0 | 0 | — | 0 | 38 |
| sample:18 | 1 | 140 | 0/5 | 0 | 2 | 2 monoculture, 1 energy-death | 0 | 1439 |
| sample:18 | 2 | 70 | 1/5 | 0 | 1 | 3 monoculture | 1 | 1731 |
| sample:18 | 3 | 47 | 1/5 | 3 | 0 | 1 monoculture | 1 | 972 |
| sample:18 | 5 | 28 | 1/5 | **4** | 0 | — | 1 | 278 |
| sample:18 | 8 | 18 | 2/5 | 3 | 0 | — | 2 | 94 |
| sample:18 | 12 | 12 | **5/5** | 0 | 0 | — | 5 | 50 |
| sample:18 | 16 | 9 | 5/5 | 0 | 0 | — | 4 | 50 |
| sample:18 | 20 | 7 | 5/5 | 0 | 0 | — | 3 | 50 |
| sample:188 | 1 | 77 | 1/5 | 2 | 0 | 2 monoculture | 1 | 1526 |
| sample:188 | 2 | 39 | 1/5 | 3 | 0 | 1 monoculture | 1 | 457 |
| sample:188 | 3 | 26 | 4/5 | 1 | 0 | — | 4 | 52 |
| sample:188 | 5 | 16 | **5/5** | 0 | 0 | — | 5 | 18 |
| sample:188 | 8 | 10 | 5/5 | 0 | 0 | — | 5 | 17 |
| sample:188 | 12 | 7 | 5/5 | 0 | 0 | — | 3 | 17 |
| sample:188 | 16 | 5 | 5/5 | 0 | 0 | — | 2 | 17 |
| sample:188 | 20 | 4 | 5/5 | 0 | 0 | — | 1 | 17 |

Pooled over the four baselines: `r ≤ 2` → **13/40 live**, 6 timeouts; `r = 3`
→ 10/20; `r = 5` → 13/20; `r = 8` → 16/20; `r ≥ 12` → **60/60 live**, 0
timeouts. Median peak population by level, pooled over finished seeds: `r = 1`
1425, `r = 2` 550, `r = 3` 278, `r = 5` 44, `r = 8` 43, `r ≥ 12` 42.

**The small-`r` end, per baseline.** This is the question v1 left open.

- **recipe: lockup.** At `r = 2` and `r = 3` all 10 seeds lock up, stopped by
  the gate at ticks 350–550. At `r = 1`, 3 of 5 seeds spent the simulation
  budget, holding 3500–4500 agents at ticks 1305–1928. Of the two that
  finished, one locked up and one ended in generalist dominance, so `r = 1` is
  undersampled rather than a different regime. v1's apparent inversion
  (2/5 and 3/5 live at `r = 1, 2`, above its own mid-range) is gone. On
  current code the recipe's curve rises monotonically in `r`.
- **sample:18: monoculture, then lockup.** At `r ≤ 2` the finished failures are
  5 monoculture and 1 energy death, with 3 timeouts. At `r = 3–8` they are
  10 lockup and 1 monoculture. The mode switches with the bloom's size. It is
  genuinely mixed, but in two bands rather than at random.
- **sample:188: mostly lockup, some monoculture.** At `r ≤ 2` there are 5
  lockup, 3 monoculture and 2 live. By `r = 3` it is 4/5 live.
- **sample:110: no small-`r` failure.** It is live at every level, and every
  seed at `r ≤ 8` has consumers. The bloom still grows at small `r` (median
  peak 425 at `r = 1`), and at `r ≤ 3` a decomposer guild appears on 13 of 15
  seeds. In v1 it failed by generalist dominance on 10 of 20 seeds at
  `r ≤ 5`. Those failures were the note's main evidence that the failure mode
  depends on the baseline.

**Near each baseline's own radius**, the three sample baselines are live:
`sample:110` (6.9) is 5/5 at both `r = 5` and `r = 8`; `sample:18` (17.7) is
5/5 at `r = 16` and `r = 20`; `sample:188` (4.3) is 4/5 at `r = 3` and 5/5 at
`r = 5`. This is consistent with their #509 verdicts.

**What moved from v1.**

- **The pooled headline.** `r ≥ 12` rose from 56/60 to 60/60. The four
  non-live seeds in v1 were all recipe lockups, and the recipe is a different
  world now. v1's text reported this figure as 44/60, but the v1 file on disk
  gives 56/60, so 44 was a miscount. Likewise v1 lost **16** runs to its guard
  at `r ≤ 3`, not the 12 it reported.
- **`sample:18` and `sample:188`** are nearly unchanged. Their lockup bands and
  their timeouts at `r ≤ 2` are where they were, with a few seeds trading
  lockup for monoculture.
- **`sample:110`'s** generalist-dominance failures are gone (item 3 of the
  TL;DR).
- **The recipe's curve** changed shape: it now fails completely at `r ≤ 3` and
  is fully live at `r ≥ 12`. Whether that comes from the new recipe (#545) or
  from the starvation fix (#541) is **unresolved**. The optional check in
  #524, which sweeps the pre-#545 recipe on current code, was **not run**. It
  is the instrument that would separate the two.

## 3. The recipe at its own radius

In v1 the recipe (then projected from the 500-tick atlas) was the only
baseline that locked up across the *whole* range and never reached 5/5 live.
That was filed as #522 and answered by regenerating the recipe from the
settled-horizon atlas (#545). The regenerated recipe lies in a smaller world
(`L = 61.8` against 99.6) with less flux (8.1 against 10.6). It reaches 5/5
live from `r = 12` up, and 4/5 at `r = 8`, next to its own 8.8.

Its radius sits close to where the curve turns: `r = 5` is 2/5 live, 3/5
lockup. So a player's world at the recipe is live, but not far from the lockup
band. Among its live seeds, consumers at `T` are thin at its own radius (1 of 4
at `r = 8`) and more common at `r = 12` (4 of 5). This note does not read
consumer persistence further. It is the guild question, not the radius one.

## 4. What the budgets bound

v1's guard checked `started.elapsed() > run_timeout` between steps, so it
bounded the **step loop** but not `evaluate_from_log`. A level whose worlds held
1500–2800 agents could then spend hours in evaluation. A long-guard re-run of
the small-`r` levels was abandoned for that reason, and the gap was filed as
#524. #532 fixed it (#523): the evaluation now has its own budget, and a run
that spends it is recorded as `eval_timeout`, apart from `timeout`.

At 1800 s / 1800 s on current code:

- **6 simulation timeouts, 0 evaluation timeouts.** They are recipe `r = 1`
  (3 of 5), `sample:18` `r = 1` (2 of 5) and `sample:18` `r = 2` (1 of 5).
  Every one was still stepping, at ticks 1305–1988, holding 2200–4500 agents.
  They are the densest runs in the sweep, so the small-`r` end is still
  undersampled at exactly those three levels. Of their 9 finished siblings,
  8 are non-live, so the unfinished runs cannot flip a level from mostly
  failing to mostly live.
- **No run was lost to evaluation.** With the step loop bounded and the
  evaluation budgeted, every run that reached `T` was verdicted.

The v1 section's claim stands: a `--run-timeout-secs` that does not bound the
evaluation cannot bound the run. It is now fixed in this bin and in the sweeps
that share `explorers_search::sweep`.

## 5. What this settles for #462

- **The radius is a real coordinate, not an artefact of the LHS draw.** Holding
  everything else fixed, it moves peak population hard and monotonically on
  every baseline. On three of four it also moves the live fraction from mostly
  failing at `r ≤ 2` to 5/5 live at `r ≥ 12`. A reduced coordinate set for the
  search must contain it, or the ceiling's `m`. The permanence π-table does
  not.
- **It is not a lockup coordinate.** The observational reading paired the
  radius with lockup specifically. The single-factor sweep shows that pairing
  depends on the baseline. `sample:18` fails by monoculture at the smallest
  radii and by lockup in the middle. `sample:110` blooms tenfold at `r = 1`
  and survives it. What the radius predicts is *bloom amplitude*, which the
  ceiling's `m` states in closed form. Whether a bloom kills a world, and how,
  is set by the rest of the config. In v1, `sample:110` supported this by
  failing differently. In v2 it supports it by not failing at all.
- **So the next coordinate to test is the amplitude, not the radius.** That
  means asking whether `m²` (or `m²` against the flux and founder budget)
  predicts peak population across configs, and whether peak population
  predicts the failure mode better than any raw dimension. It is a read on
  existing data (see [`462-amplitude-read.md`](462-amplitude-read.md)), not
  new rollouts.

## 6. What this does not settle

- **Five seeds per level** resolves a 5/5-vs-2/5 difference, not a 4/5-vs-5/5
  one. The per-level fractions are indicative. The pooled trend is the finding.
- **Four baselines** cannot separate "the radius interacts with solar flux"
  from "the radius interacts with founder density". `sample:110` differs from
  the others in both, and it is now the baseline where the radius does not
  decide survival.
- **Why `sample:110` stopped failing.** #543 (the per-agent generalist-dominance
  verdict) is the likely cause and #541 the other candidate. Neither was
  isolated.
- **Why the recipe's curve moved.** It could be the new recipe (#545) or #541.
  The pre-#545 recipe check in #524 was not run.
- **The three levels with timeouts** (recipe `r = 1`, `sample:18` `r = 1, 2`)
  are undersampled. Their finished seeds are all non-live except one.
- **Guilds** (decomposer / consumer) appear on 25 / 11 of 160 level-seeds.
  They are concentrated at small `r`: `sample:110` `r ≤ 3` accounts for 13 /
  8 of them, and `sample:188` `r ≤ 3` for 7 / 3. That is too few, and too
  concentrated, to read.
