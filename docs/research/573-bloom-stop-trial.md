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
