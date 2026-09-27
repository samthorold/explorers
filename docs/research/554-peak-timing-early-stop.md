# Issue #554: is peak timing an early-stop signal?

**Status: measurement. Adds fields to `permanence_crosscheck` (#568, #571), and
changes nothing in the stepper, evaluator or search.** The
[amplitude read](462-amplitude-read.md) §3 found that the *observed* bloom
factor (`peak / founders`) predicts a config's live and lockup fractions better
than all 32 search coordinates do. If the peak comes early, that scalar is known
long before `T = 2000`, and the search could use it to stop or down-weight a
rollout early. This note asks:

1. Where does `peak_tick` fall, as a fraction of `T`?
2. Taking the bloom observed by tick `t` (the running peak) instead of the
   full-horizon peak, how much predictive power is kept at each cut-off?
3. Is there a cut-off early enough to save meaningful rollout time? What form
   would a rule take?

Stepper at `3da7c21`. There were no stepper changes between the full-horizon
sweep and the cut-off re-run.

## TL;DR

1. **Live worlds peak early and lockup worlds peak late.** Of bloomed seeds that
   end live, 69 % have peaked by `0.1·T`. Of those that end in lockup, 10 % have
   (21 % by `0.15·T`, 41 % by `0.25·T`) (§2).
2. **Most of the full-horizon power is visible by `0.15–0.25·T`.** At the config
   level the running bloom at tick 500 (`0.25·T`) gives CV R² 0.42 on live and
   0.46 on lockup, against 0.49 / 0.52 at `T`. At tick 300 it gives 0.36 / 0.40.
   Adding how far the population has fallen from its running peak closes the
   lockup gap: 0.45 at tick 300, 0.51 at tick 500 (§3).
3. **Per seed, a threshold rule is nearly free of false stops on live worlds.**
   At tick 300, stopping every seed whose running bloom is ≥ 5× founders stops
   113 of 1541 seeds. 110 of them are failures the search rejects anyway (84
   lockup, 26 monoculture or generalist dominance), and 3 of the 1088 live
   seeds (0.3 %) are stopped wrongly. That catches 35 % of lockup seeds. At
   tick 500 it catches 48 %, with the same 3 live seeds (§4).
4. **Over the gate the search already has, it saves few ticks but about a
   quarter of the agent-ticks (#569, §7).** The incremental gate fires on
   every lockup seed, at a median of tick 750. At tick 300 and θ = 5 the
   bloom rule fires before it on all 113 of its stops, a median 150 ticks
   earlier on the lockup seeds. That is 23 ticks per finished seed (1.4 % of
   what the gate runs), but about 24 % of the agent-ticks, because the stops
   are the tall blooms. It also stops 50 of the 59 timed-out seeds, and the
   gate has stopped none of them by tick 300.

## 1. What ran

| | |
|---|---|
| outcomes | `target/permanence-crosscheck-9421.jsonl` (SHA-1 `da1d44f1…`, as in [462-held-out-check.md](462-held-out-check.md)): full horizon `T = 2000`, seeds 1000–1007. Per seed: mode, `founders`, `peak_population`, `peak_tick`. `timeout` / `eval_timeout` seeds are dropped. 197 configs, 1541 finished seeds |
| cut-offs | `target/permanence-crosscheck-9421-t1000.jsonl` (SHA-1 `32c18951…`): the same 200 configs and seeds re-run to tick 1000 with the new `cutoffs` field (#568). Each cut-off records the population and running peak at ticks 25, 50, 100, 200, 300, 500, 750, 1000. Run with `--horizon 1000 --run-timeout-secs 240 --eval-timeout-secs 5`. The tick-1000 verdicts are not used, and labels come only from the full-horizon file |
| join | per (config, seed). **Replay check:** at every cut-off at or past a seed's recorded `peak_tick`, the running peak equals the full run's `peak_population`. Before `peak_tick`, it never exceeds it. 9250 cut-offs checked, 0 mismatches. A seed that ended before a cut-off (extinction, explosion) keeps its final peak |
| config level (§3) | as the amplitude read: per config over finished seeds, **bloom_t** = median running peak at `t` / median founders. Targets: live and lockup fractions. z-scored ridge on `log₁₀ bloom_t`, 10-fold CV, best of λ ∈ {0.1, 1, 10, 100}, mean of 5 shuffles. The second feature set adds `log₁₀((population_t + 1) / running_peak_t)`, the fall from the running peak |
| seed level (§3–4) | AUC of `log₁₀(running_peak_t / founders)` for the seed's eventual mode |
| tool | a throwaway Python/NumPy script, not committed (the [474](474-atlas-regen-post-fix.md) precedent) |

## 2. Where the peak falls (Q1)

659 of 1541 finished seeds bloom (peak above founders). Every other seed has
`peak_tick = 0`. Over bloomed seeds, `peak_tick / T` has quantiles 0.002 (p10),
0.020 (p25), **0.13** (median), 0.34 (p75), 0.75 (p90). This reproduces the
held-out note's §8.

Fraction of bloomed seeds whose peak has passed by each cut-off, by eventual
mode:

| mode | n | 0.0125·T | 0.025 | 0.05 | 0.1 | 0.15 | 0.25 | 0.375 | 0.5 |
|---|---|---|---|---|---|---|---|---|---|
| live (`none`) | 342 | 0.39 | 0.47 | 0.58 | 0.69 | 0.77 | 0.84 | 0.87 | 0.90 |
| nutrient-lockup | 239 | 0.00 | 0.00 | 0.01 | 0.10 | 0.21 | 0.41 | 0.62 | 0.76 |
| monoculture | 44 | 0.00 | 0.00 | 0.00 | 0.11 | 0.34 | 0.52 | 0.61 | 0.66 |

So the peak's *value* is rarely final early for a lockup world. For most lockup
worlds it is still rising at `0.25·T`. That turns out not to matter much (§3):
a bloom still climbing at tick 300 is already tall.

## 3. How much power the running bloom keeps (Q2)

Config level, 197 configs (live sd 0.32, lockup sd 0.27):

| cut-off | `t/T` | live R² | lockup R² | live R², + fall | lockup R², + fall |
|---|---|---|---|---|---|
| 25 | 0.0125 | 0.09 | 0.06 | 0.08 | 0.14 |
| 50 | 0.025 | 0.12 | 0.09 | 0.12 | 0.19 |
| 100 | 0.05 | 0.18 | 0.19 | 0.19 | 0.30 |
| 200 | 0.1 | 0.29 | 0.30 | 0.30 | 0.40 |
| 300 | 0.15 | 0.36 | 0.40 | 0.37 | **0.45** |
| 500 | 0.25 | 0.42 | 0.46 | 0.42 | **0.51** |
| 750 | 0.375 | 0.46 | 0.48 | 0.46 | 0.51 |
| 1000 | 0.5 | 0.49 | 0.51 | 0.48 | 0.53 |
| `T` | 1 | **0.49** | **0.52** | — | — |

(The shuffle spread is about ±0.02, as in the earlier notes. On this draw the
full-horizon figures, 0.49 / 0.52, match the amplitude read's seed-421 figures,
0.49 / 0.41, on live and exceed them on lockup.)

By `0.25·T` the running bloom keeps 86 % of the live power and 89 % of the
lockup power. With the fall from the running peak added, the lockup model at
`0.25·T` equals the full-horizon bloom. A world that has bloomed tall and
already crashed is one the full peak would also have called. The live model
gains nothing from the fall.

Seed level (1541 seeds; 242 lockup, 1088 live):

| cut-off | 25 | 50 | 100 | 200 | 300 | 500 | 750 | 1000 | `T` |
|---|---|---|---|---|---|---|---|---|---|
| lockup AUC | 0.64 | 0.69 | 0.76 | 0.85 | 0.88 | 0.91 | 0.93 | 0.94 | 0.95 |
| not-live AUC | 0.58 | 0.61 | 0.66 | 0.72 | 0.74 | 0.76 | 0.77 | 0.78 | 0.78 |

Lockup is the outcome the bloom reads well. "Not live" includes extinction and
energy death, which a bloom does not predict. Those already stop early on their
own gates.

## 4. A threshold rule (Q3)

The rule: at cut-off `t`, stop a seed that is still running if its running
bloom is ≥ θ× founders. The counts are over the 1541 finished seeds. "Ticks
saved" is `T − t` per stopped seed, averaged over all seeds, before any saving
the existing gates already make (§5):

| t | θ | stopped | lockup | other failure | live | lockup caught | live stopped | ticks saved / seed |
|---|---|---|---|---|---|---|---|---|
| 200 | 3 | 119 | 84 | 25 | 10 | 0.35 | 0.9 % | 139 |
| 200 | 5 | 81 | 57 | 21 | 3 | 0.24 | 0.3 % | 95 |
| 300 | 3 | 165 | 126 | 28 | 11 | 0.52 | 1.0 % | 182 |
| **300** | **5** | **113** | **84** | **26** | **3** | **0.35** | **0.3 %** | **125** |
| 300 | 10 | 75 | 50 | 22 | 3 | 0.21 | 0.3 % | 83 |
| 500 | 3 | 204 | 154 | 34 | 16 | 0.64 | 1.5 % | 199 |
| **500** | **5** | **151** | **117** | **31** | **3** | **0.48** | **0.3 %** | **147** |
| 500 | 10 | 103 | 71 | 29 | 3 | 0.29 | 0.3 % | 100 |

"Other failure" is monoculture or generalist dominance, never extinction or
energy death. At θ = 5 about 97 % of stopped seeds are failures the search
rejects anyway, and the same three live seeds are stopped at every cut-off. Those three are tall-bloom survivors.
θ = 3 roughly doubles the live false stops for 15–17 more points of lockup
caught.

Ticks are the wrong unit for the saving. Stopped seeds are the tall blooms, and
a tick of a world at 5–40× founders costs many ticks of a sparse one; the
held-out draw's 59 timed-out seeds are all in this region. So the wall-clock
saving is larger than the tick saving by an amount this data cannot price.

## 5. What this does not settle

- **The comparison that decides it.** The search already stops a rollout early
  when the incremental lockup gate fires: the dead-pool gate read on the
  series-so-far, stopping at the first window boundary past `t + window`
  ([genesis-search.md](../system-design/genesis-search.md), "The frontier costs
  a bloom"). A bloom rule is worth adding only for lockup seeds on which it
  fires well before that gate. The crosscheck carries every rollout to `T` and
  does not record when the gate would have fired, so the saving over the
  existing stop is unmeasured. Follow-up #569: record the gate's firing tick
  per seed, then read the bloom rule's *marginal* saving. Only then choose `t`
  and θ. *Measured in §7.*
- **A predictive stop is a different kind of stop.** The existing gates stop a
  world that *has* collapsed. A bloom rule stops a world that *will*, and the
  0.3 % live false stops are live worlds it would send to the frontier. The
  early-stop cross-check (`early_stop_crosscheck_fraction`) would carry a sample
  to `T` and surface such disagreements, as it does for the gates. The
  alternative is down-weighting: fitness uses the bloom as a penalty and nothing
  stops.
- **One draw.** The rule was chosen on the same 1541 seeds it is scored on. The
  table is a description, not a held-out rate. Seed 421 has no `peak_tick` or
  cut-offs, so there is no second draw here.
- **Search proposals are not LHS draws.** An emitter concentrates near elites,
  so the search's mix of tall blooms differs from a uniform draw's.

## 6. Answer

1. Median `0.13·T` over bloomed seeds. Live peaks come early; lockup peaks late,
   with a median around `0.3·T`.
2. At `0.25·T` the running bloom keeps about 85–90 % of the full-horizon config
   power, and with the fall from the peak added it matches it. At `0.15·T` it
   keeps about 75 %. Per seed, lockup AUC is 0.88 at `0.15·T` against 0.95 at `T`.
3. **Yes, on the signal:** a rule at `t = 300–500` with θ = 5 catches 35–48 % of
   lockup seeds, 97 % of its stops are failures, and it wrongly stops 0.3 % of
   live seeds. **On the saving, over the gate the search already has (§7):**
   it saves few ticks but about a quarter of the agent-ticks, and it reaches
   the timed-out seeds before the gate does.

## 7. Addendum (#569): the saving over the gate

`permanence_crosscheck` now records `early_stop_tick` and `early_stop_mode`
per seed: the first tick at which the search's incremental stop
(`explorers_genesis_eval::early_stop`) would have fired, read on the
rollout's own observations without stopping it (#571).

| | |
|---|---|
| run | `target/permanence-crosscheck-9421-t2000-gate.jsonl` (SHA-1 `5adb8468…`): the same 200 configs × seeds 1000–1007 at `T = 2000`, `--eval-timeout-secs 600`, at `fe18ead`. Its cut-offs cover every tick in the list up to 1500 |
| replay check | all 1541 finished verdicts match the full-horizon file, and the 59 timeouts are the same seeds. All 12 229 cut-offs shared with the t1000 file match. So the join is exact, and this file stands alone |
| gate tick | `g` = `early_stop_tick`, or the termination tick if the gate never fired. Without a bloom rule, a seed runs to `g` |
| marginal stop | at cut-off `t`, the bloom rule (running peak ≥ θ × founders) fires *before the gate* on a finished seed if it is running at `t` and `g > t`. It saves `g − t` ticks |
| agent-ticks | a cost proxy: the population integrated over ticks, trapezoid on the cut-off samples (tick 0 is the founders, and the last point is the terminal population). The sampling misses peaks between cut-offs, and a step costs more than linearly in agents, so this understates the tall blooms' share. It is reported as the share of the gate policy's total over finished seeds |

**The gate on its own.** Split by the horizon verdict:

| verdict | seeds | gate fires | tick q10 / q50 / q90 | gate mode |
|---|---|---|---|---|
| nutrient lockup | 242 | 242 | 350 / 750 / 1400 | lockup 238, energy death 4 |
| energy death | 12 | 12 | 350 / 600 / 1050 | energy death 12 |
| extinction | 141 | 141 | 8 / 416 / 1441 | extinction 137, energy death 4 |
| live (`none`) | 1088 | 4 | 350 / 350 / 650 | energy death 3, lockup 1 |
| monoculture | 44 | 27 | 350 / 400 / 750 | lockup 23, energy death 4 |
| generalist dominance | 14 | 2 | 600 | lockup 2 |
| timeout | 59 | 36 | 350 / 350 / 750 | lockup 36 |

The gate reaches every lockup seed, but late: half of them run past tick 750.
The grace (260) and the window (50) mean a dead-pool gate cannot fire before
tick 350, so at `t ≤ 300` every bloom stop comes before the gate. Under the
gate, a finished seed runs 1649 ticks on average, against 1873 carried to `T`.
Lockup seeds hold 46 % of the gate policy's agent-ticks.

**The bloom rule over the gate.** The counts are over the 1541 finished
seeds, with the §4 columns, and "ticks saved" is now `g − t`:

| t | θ | fires before gate | lockup | other failure | live | lockup caught before gate | lockup the gate already stopped | ticks saved / seed | median ticks saved per lockup caught | median population at stop | agent-ticks saved | timed-out seeds stopped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 200 | 3 | 119 | 84 | 25 | 10 | 0.35 | 0 | 40 | 250 | 182 | 0.28 | 56 |
| 200 | 5 | 81 | 57 | 21 | 3 | 0.24 | 0 | 20 | 200 | 241 | 0.20 | 54 |
| 300 | 3 | 165 | 126 | 28 | 11 | 0.52 | 0 | 47 | 250 | 175 | 0.31 | 50 |
| **300** | **5** | **113** | **84** | **26** | **3** | **0.35** | **0** | **23** | **150** | **285** | **0.24** | **50** |
| 300 | 10 | 75 | 50 | 22 | 3 | 0.21 | 0 | 14 | 50 | 353 | 0.18 | 49 |
| 500 | 3 | 135 | 99 | 20 | 16 | 0.41 | 55 | 45 | 250 | 132 | 0.27 | 36 (18 already gated) |
| **500** | **5** | **85** | **65** | **17** | **3** | **0.27** | **52** | **22** | **200** | **236** | **0.23** | **35 (18 already gated)** |
| 500 | 10 | 47 | 29 | 15 | 3 | 0.12 | 42 | 14 | 150 | 416 | 0.18 | 34 (18 already gated) |

What this shows:

- **In ticks, the saving is small.** Against `T`, θ = 5 at tick 300 saved 125
  ticks per seed (§4). Against the gate it saves 23, which is 1.4 % of the
  ticks the gate policy runs. Tick 500 is no better than tick 300: by then
  the gate has already stopped 52 of the lockup seeds the rule would catch.
- **In agent-ticks, it is about a quarter.** The seeds it stops are at a
  median of 285 agents at tick 300, against founders in the tens. Cutting
  their last 150 or so ticks removes about 24 % of the finished seeds'
  agent-ticks. By the table's own caveat, this is a floor.
- **It reaches the timed-out seeds.** 50 of the 59 seeds that exhaust the
  300 s budget have a running bloom ≥ 5× at tick 300, and none has been
  gated by then. The gate stops 36 of the 59 before their budget runs out,
  most at tick 350, and 23 not at all. These seeds have no verdict, so it is
  not known whether a stop there would be right. But they are where the wall
  clock goes.
- **The false stops do not change.** The same 3 live seeds, plus the 26
  monoculture and generalist-dominance seeds, which are failures anyway.

**Verdict.** The saving over the gate is not in ticks. It is in the tall
blooms' agent-ticks and in the rollouts that would otherwise run into their
wall-clock budget. That is worth a search change only as a cost measure.
Whether it is worth the live false stops (0.3 %) is the follow-up's
decision, including which form it takes: a predictive stop under the
early-stop cross-check, or a fitness down-weight that saves nothing.
The one-draw caveat of §5 still applies: `t` and θ were chosen on these
seeds.
