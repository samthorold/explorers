# Issue #573: does the bloom stop pay for itself in the search?

**Status: measurement. Adds an opt-in search flag (`--bloom-stop T:F`, #575)
and changes no default.** [554-peak-timing-early-stop.md](554-peak-timing-early-stop.md)
§7 found that stopping a rollout at tick 300 when its running peak reaches
5× founders saves few ticks over the incremental gate, but about a quarter of
the agent-ticks. It also reaches the seeds that run into the wall-clock budget
before the gate does. That was on an LHS draw. This note runs the rule inside the
QD search and asks:

1. How much search wall clock does it save?
2. What does it cost the atlas: coverage, QD-score, disagreements?
3. Is any lost cell a live world the rule stopped?

Search at `6b58a91`.

## TL;DR

1. **It roughly halves the search's wall clock.** Ten generations took 19.8
   min against 38.3 (seed 42) and 11.7 against 21.6 (seed 43). CPU time fell
   by 63 % and 26 %.
2. **At ten generations, the atlas comes out slightly smaller:** 90 against 95
   cells and QD-score 34.0 against 36.4 (seed 42); 89 against 93 and 34.7
   against 36.6 (seed 43). At matched wall clock the stop is far ahead. When
   each "on" search finished, its "off" twin was still in generation 2, at QD
   13.4 (seed 42) and 13.9 (seed 43).
3. **The lost cells are not the rule's doing.** None of the "off" atlases'
   live cells has a majority of seeds past 5× at tick 300. Of the 29 cells the
   "on" searches lost, one has a single seed the rule would stop. The runs are
   identical until the first stop changes the archive (generation 6 and 7).
   After that the emitters take a different path, and the cell difference is
   that divergence.
4. **No cross-check disagreement.** 0 of 27 carried early stops (gate and bloom
   together) read alive at the horizon. The carry fraction is 0.05, so few of
   the 30 bloom-stopped configs were among them.

## 1. What ran

| | |
|---|---|
| searches | `explorers-search --seed S [--bloom-stop 300:5] --refine-top-k 1`, S ∈ {42, 43}, defaults otherwise (full box, batch 32, 10 generations, ensemble 5, `T = 2000`, 600 s / 600 s rollout budgets). Run one after another in the order 42 off, 42 on, 43 off, 43 on, in the background on battery under `caffeinate -i`. Outputs are in `target/trial573/` |
| wall clock | the search's own `elapsed` at generation 10, which excludes refinement |
| lost-cell check | every live cell of each "off" atlas re-run to tick 300 with `permanence_crosscheck --horizon 300` over seeds 1000–1007. A seed counts as stopped if its running peak at tick 300 is ≥ 5× founders. These are not the search's ensemble seeds, so this estimates how exposed each cell is, not a replay |

## 2. Results

| seed | arm | elapsed (10 gens) | CPU (user) | cells | QD-score | best | dead (bloom_stop) | unfinished |
|---|---|---|---|---|---|---|---|---|
| 42 | off | 38m18s | 5819 s | 95 | 36.40 | 0.572 | 68 (–) | 2 |
| 42 | on | 19m45s | 2135 s | 90 | 33.97 | 0.613 | 76 (16) | 0 |
| 43 | off | 21m35s | 3051 s | 93 | 36.57 | 0.607 | 58 (–) | 0 |
| 43 | on | 11m42s | 2247 s | 89 | 34.66 | 0.662 | 56 (14) | 0 |

Where the time goes. In the "off" searches, a few generations hold a dense
config and dominate the wall clock:

- Seed 42: generation 2 took 20m31s and hit the budget on 2 rollouts, and
  generation 3 took 9m04s.
- Seed 43: generation 0 took 5m43s and generation 2 took 8m17s.

With the stop on, the same generations took 6m19s, 2m22s, 2m03s and 2m22s.
Those configs are the tall blooms, cut at tick 300.

The dead frontiers differ by more than the new `bloom_stop` column. Lockup
drops from 52 to 44 (seed 42) and from 48 to 35 (seed 43): some of the
configs the gate used to catch late are now caught by the stop. Extinction
moves both ways, which fits the searches diverging.

**Lost-cell check.** Of the "off" atlases' live cells:

- Seed 42: of 95 cells, 5 have any stopped seed and 0 have ≥ 5 of 8. The
  "on" search lacks 19 of the 95; the rule would stop one seed of one of them.
- Seed 43: of 93 cells, 3 have any stopped seed and 0 have ≥ 5 of 8. The
  "on" search lacks 10 of the 93, and the rule would stop none of them.

In both, the "on" search also found cells the "off" one did not: 14 and 6.

## 3. What this does not settle

- **Two seeds.** The QD-score is 5–7 % lower at equal generations in both. The
  check above shows the rule is not stopping the lost cells directly. But it
  cannot separate an indirect effect (a bloom-stopped config is not there to
  steer the emitter) from trajectory noise. A matched-wall-clock run would test
  this directly, for example "on" at 20 generations against "off" at 10.
- **Power state.** The runs were on battery. Running each seed's two arms back
  to back limits the bias, but the absolute times are not comparable with
  earlier notes.
- **Live false stops in the search's own mix.** On the LHS draw it was 0.3 %.
  Here, zero carried bloom stops disagreed, but the carry sample is small.

## 4. Answer

1. About half the search's wall clock, most of it from the few dense
   generations that dominate an unstopped search.
2. At equal generations, 4–5 fewer cells and 5–7 % less QD-score. At equal
   wall clock, much more of both. There were no disagreements.
3. No. The rule would stop no majority of any live cell. The lost cells come
   from the search diverging after its first stop.

Whether to make it the default is a promotion decision for #573, alongside the
matched-wall-clock run above if a stricter test is wanted.

## 5. Addendum: equal wall clock, and the factor raised to 10

**At 5×, the equal-time test passes, but the rule kills a class of live
worlds.** All runs here are on AC, with the "off" searches re-run on AC.
Their atlases are byte-identical to the battery runs', and the first 10
generations of a 20-generation "on" search reproduce the 10-generation run
exactly.

- Seed 42: at 5× and 20 generations, 116 cells / QD 45.2 in 17.6 min,
  against 95 / 36.4 in 23.6 min with the stop off at 10.
- Seed 43: at the 13.4 min mark where the "off" search finished, the stop
  run had 96 / 37.7, against 93 / 36.6.

The cross-check carried two bloom stops in generations 11–20 of seed 43 that
were alive at `T`. Re-run on seeds 1000–1007 to `T = 2000`:

- One config is **live on 8 of 8 seeds, and every seed is past 5× at tick
  300** (5.1–7.7×). It has 42 founders and settles at 250–300 agents.
- The other is mixed: 5 live, 2 lockup, 1 monoculture. The rule would stop
  2 of its live seeds.
- The fall from the running peak does not separate them. At tick 300 every
  seed, live or lockup, is within about 10 % of its peak.

The search proposes worlds an LHS draw rarely produced (§3; the #554 note's
§5), and 5× cuts into them.

**At 10×.** Per seed: a timing run (`--bloom-stop 300:10`, 20 generations,
default carry 0.05), and a measurement run with every early stop carried to
`T` (`--early-stop-crosscheck-fraction 1.0`, #577).

| seed | run | 10 gens | cells / QD at 10 | 20 gens | cells / QD at 20 |
|---|---|---|---|---|---|
| 42 | off | 23m38s | 95 / 36.404 | — | — |
| 42 | 300:10 | 12m14s | 95 / 36.404 | 28m56s | 119 / 46.77 |
| 43 | off | 13m26s | 93 / 36.574 | — | — |
| 43 | 300:10 | 7m25s | 93 / 36.574 | 31m06s | 129 / 52.67 |

- **At 10×, the first 10 generations are identical to the stop-off search,
  in about half the time.** No config's verdict changes, so the search does
  not diverge. The rule only cuts the expensive rollouts short. Later
  generations still hold dense configs the rule does not reach: generation 16
  of seed 42 took 10.6 min, and generation 20 of seed 43 took 7.6 min.
- **Wrongful stops, carrying everything:**

  | | seed 42 | seed 43 |
  |---|---|---|
  | stops carried to `T` | 669 | 592 |
  | alive at `T` | 5 | 13 |
  | of which bloom stops | 1 | 4 |
  | configs behind the bloom stops | 1 | 1 |

  The rest are the existing gates' own reversible collapses: 3 lockup and
  1 energy death on seed 42, 4 lockup and 5 energy death on seed 43, some
  of them at high horizon fitness.
- **Re-run to `T = 2000` on seeds 1000–1007:**
  - Seed 42's config is 8/8 non-live (7 lockup, 1 monoculture). Its one live
    seed in the search was a fluke, and the stop was right.
  - Seed 43's config is live. Every seed that finished persists at 1200–1800
    agents from 37 founders, and the other 4 of 8 hit the 300 s simulation
    budget holding 2000–4700 agents. Its bloom at tick 300 ranges from 3× to
    120×. It is the dense-bloom world the rollout budgets already struggle
    with, and the search placed it nowhere.
- **The carry is not verdict-neutral when budgets bite.** A carried rollout
  that exhausts its budget is unfinished, which drops the seed and can move
  its config's verdict. The measurement runs had 5 and 12 unfinished seeds,
  against 1 and 0 in the timing runs. Their dead frontiers differ by one or
  two configs, and their cells not at all.

**Answer.** At 300:10 the stop halves the time to an identical 10-generation
atlas. Across two 20-generation searches, its one real loss is a very dense
live world, and the existing gates wrongly stop live worlds more often. The
cost of promoting it is a stated blind spot: the atlas will not hold worlds
that bloom past 10× by tick 300 and then persist at thousands of agents.
Whether that is acceptable is a design call on #573.
