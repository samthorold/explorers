# Scenario verdicts — is each scenario a *sensible* ecology?

**Generated artifact, not a hand-maintained note.** This is the *judged read* layer of
the validation triad (#293): the [observed evidence](observed.json) interpreted against
each scenario's declared `probes` / `prediction` (in its `metadata`), grounded strictly in
[`expected-properties.md`](../docs/system-design/expected-properties.md). It is a
*reading*, not a pass/fail test — precise numbers are evidence for the read, not the
gate.

**The verdict is the majority/supermajority read of a *distribution*, and that read lives
here, not in the binary (#314).** `eval_scenarios` now runs each scenario over a
deterministic seed ensemble (`base_seed=1 .. base_seed+8`, mirroring genesis's
`run_ensemble`) and emits the *distribution* — a failure-mode count + modal mode, the
median/min/max of every score, plus a `per_seed` breakdown. It stays prediction-agnostic
and verdict-free; the supermajority read against the declared prediction is made here. This
matters because regime-sensitive scenarios (example6) can flip on a single draw, so a
verdict hung on one seed is fragile. The columns below cite the **modal failure mode** and
the **fraction of seeds** matching the prediction; the per-row prose quotes the median and
its spread. (Across the current suite all eight seeds agree on the modal failure mode for
every scenario — the ensemble *confirms* the single-seed reads were not lucky draws, which
is itself evidence — but the demographic and score spreads are now visible, and example6's
births swing seed to seed exactly as expected.)

Regenerate by re-running `eval_scenarios` and re-judging (a human or a
fresh-perspective agent); the verdict below was re-judged by an agent on 2026-06-02
(after #302 replaced the energy-death detector with a free-energy-stock-trend test, after
#303 added `example6_decomposer_viability` and fixed the drain-phase index/id bug that had
stopped any decomposer from ever consuming a carcass, after #313 made the structural
death threshold *peak-relative* — a fraction of each agent's own peak structure — so
newborns and seeds are born viable by construction rather than dead-on-arrival below an
absolute floor, and after #309 gave feeding reach a sessile body-extent solution
— `consumption_reach = effective_heterotrophy × (contact_range_coefficient +
body_reach_coefficient × √structure)` — so a growing sessile decomposer extends its reach
to carcass-fall it could not previously touch, set non-zero only in `example6`).

| scenario | verdict | modal failure (n/8) | agrees with prediction? (seeds) | primary fault |
|---|---|---|---|---|
| example1 | inconclusive | none (8/8) | n/a | stale params — `max_ticks=0`, never steps |
| example2 | not-sensible | extinction (8/8) | agree (predicted live → dead), 8/8 | stale params (`growth_efficiency` unset → 0.0) |
| example3 | inconclusive | extinction (8/8) | n/a (undecided) | stale params (`growth_efficiency`=0, `asexual_propensity`=0) |
| example4 | partially-sensible | none (8/8) | partial, 8/8 survive | incomplete roster (no decomposer), low turnover |
| example5 | not-sensible | none (8/8) | disagree | roster/probe mismatch, then stale params; 0 births fails turnover |
| example6_decomposer_viability | partially-sensible | none (8/8) | agree (predicted live → lives), 8/8 | detrital pathway works, but a single-cohort carcass pulse — no sustained multi-cluster ecology |
| example7 | not-sensible | none (8/8) | n/a (undecided) | roster mismatch (no decomposer); 0 births fails turnover |
| example8 | not-sensible | none (8/8) | disagree | roster mismatch (no decomposer); 0 births fails turnover |
| example9_detrital_pathway | sensible | none (8/8) | agree (predicted live → lives), 8/8 | none material — clean by-construction brown food web; detrital_share > 0.5 holds by geometry on every seed |

## Per-scenario fault localisation

- **example1** — Inconclusive. `max_ticks=0`: seeded, never stepped, so the numbers are the
  initial condition verbatim. `failure=none` only because the evaluator's grace-period guard
  is never crossed. A probe-nothing seed; tests no expected property.
- **example2** — Not-sensible; observation *agrees* with the "live" prediction being wrong (20→0
  is a real `Extinction`). But it's right for the wrong reason: `growth_efficiency` defaults to
  0.0, so zero structure is built and collapse is mechanically guaranteed — the outcome reflects
  a missing parameter, not the light-competition/self-thinning ecology it claims to probe.
- **example3** — Inconclusive (prediction `undecided`). Extinction is foregone: `growth_efficiency=0`
  *and* `asexual_propensity=0` (the latter precludes the lone-founder reproduction an isolation
  scenario needs). The drift/isolation question is untested.
- **example4** — Partially-sensible, and the most diagnostic row. The fully-specified file
  (20 producers + 2 consumers, heterogeneous traits), and the only one with real turnover.
  Across the 8-seed ensemble it reads `failure=none` on **every** seed (median **34 births, 48
  deaths**, median final pop 8 spanning 6–11, median fitness 0.537 spanning 0.49–0.65). Its
  free-energy stock is sustained by an actively reproducing living system, so the previous
  `energy_death` label is confirmed to have been a detector artifact (the old Consumed-only series
  read all-zero once predation tapered in the tail). #313's peak-relative death threshold raised its
  survivors — newborns that the absolute floor used to kill on arrival now live — which is the
  expected ecological direction of the root fix. The coexistence and oscillation medians (0.446,
  0.181) sit below the old over-mortality run's sharp predator-prey cycle: with more survivors the
  system settles toward a steadier trajectory. The ensemble makes its modest seed-to-seed wobble
  legible (pop 6–11) without changing the read — it is unanimously `none`, never collapsing. It is
  still not a *complete* sensible world — no decomposer role, lower coexistence — but it remains the
  closest the legacy set has to a live ecology, and the detector now treats it as such. (This file is the former
  `example4_consumer_tuning`, promoted to the canonical `example4` slot; the legacy degenerate
  example4 — triple-zeroed mate-limited producers, 0 births — has been retired.)
- **example5** — Not-sensible; disagrees with "live". Roster/probe mismatch ranks first: it declares
  `probes=population_explosion` yet seeds only 2 consumers on stale params and stalls at a final pop
  of 3 (median, identical across all 8 seeds) — the opposite regime — so it can't exercise the
  negative feedbacks it claims. 0 births on every seed fails turnover (fitness 0 throughout). The
  modal failure mode is `none` (survives 1500 ticks without tripping a detector), but a birthless,
  non-reproducing stall is not a sensible ecology. No longer flagged `energy_death`.
- **example6_decomposer_viability** — Partially-sensible; agrees with "live", and the first scenario
  in the suite to actually exercise the detrital pathway. A dense producer stand self-thins under
  light competition into a carcass field, and a sessile, low-reach, heterotrophy-dominant decomposer
  embedded in it feeds on those carcasses: it reads behaviourally as a `Decomposer` for 1991 of 2000
  ticks (the headless `--trace` brown/green split), majority of its consumed energy is detrital
  (~62% across the run), and it never starves out — it survives to `max_ticks` on every seed
  (`failure=none` 8/8; median 18 births / 41 deaths under #313's peak-relative threshold *and* #309's
  body-as-feeding-reach). This is exactly the regime-sensitive scenario #314 was motivated by: its
  birth count swings seed to seed (the carcass pulse is finite, so how many offspring catch it before
  it drains is a draw), now visible as the `per_seed` spread rather than hidden behind one lucky
  number — yet the *verdict* is stable because all eight seeds agree it lives. #309 set
  `body_reach_coefficient=2.0` here so the sessile decomposer's reach
  grows with its structure (the mycelium foraging through the substrate) and it touches carcass-fall
  that previously sat just outside its fixed contact radius — the modestly higher turnover and fitness
  (0.0039 → 0.0043) reflect that extra reachable detritus; it is the only scenario with the term
  enabled (all others keep `body_reach_coefficient=0.0`, so their numbers are unchanged).
  This is what #303 set out to prove possible and what #136 only claimed. Building it
  surfaced a latent bug: the drain phase keyed the spatial grid by slice index but looked consumers up
  by agent id, so once any death reindexed the living slice — i.e. exactly when carcasses first exist —
  the carcass pass found zero consumers and carcasses accumulated unconsumed forever. With that fixed,
  the pathway runs. It is still not a *complete* sensible world: the producers self-thin in essentially
  one early cohort, so the decomposer lives off a finite carcass pulse plus its own sparse offspring
  rather than a renewed producer→carcass supply; clustering, coexistence, and oscillation all score 0
  (final pop 1), and fitness is low. It proves decomposer *viability*, not a full multi-trophic ecology.
- **example9_detrital_pathway** — Sensible; agrees with "live", and the clean companion to
  example6's borderline showcase (#311). example6 reveals an embedded sessile decomposer as a
  *mixed* feeder — co-located with the producers it self-thins, "carcasses in reach" and "living
  producers in reach" are inseparable, so its detrital share sits at a parameter-luck ~0.47–0.62.
  example9 removes the luck by *geometry*: the decomposer is seeded on a standing carcass deposit
  (a new `carcasses` recipe capability, #311) with **no living agent inside its consumption reach**.
  With `body_reach_coefficient=0.0` the reach is structure-independent and exact —
  `heterotrophy × contact_range_coefficient = 0.5 × 1.0 = 0.5` world units — and the nearest living
  agent (a producer ring at radius ~30 on a 100-extent torus) is far outside it, so every unit the
  decomposer drains is detrital. `detrital_share > 0.5` therefore holds on every seed by
  construction, not by tuning (now confirmed across the 8-seed ensemble, not one seed). It is also a
  *complete* living ecology, not a sterile pile: the out-of-reach producer ring reproduces and
  self-thins (median 2062 births / 1998 deaths, median final pop 90 spanning 79–96, `failure=none`
  8/8, median fitness 0.80, trophic balance 1.0), raining carcasses across the field, while a
  generously-sized seeded deposit (480 energy) backstops the decomposer past `max_ticks`. The tight
  fitness spread (0.796–0.809) shows this is a robust live regime, not a draw. This is the
  producer→carcass→decomposer brown web demonstrated robustly — the property example6 only borderline
  reaches.
- **example7** — Not-sensible (prediction `undecided`). Roster mismatch is primary: intent is
  "three trophic roles incl. a decomposer", but the roster is 3 *undifferentiated* mobile consumers —
  no decomposer exists, so the detrital pathway it means to probe is absent and carcasses accumulate.
  No longer flagged `energy_death`; the fault is the missing decomposer roster and 0 turnover.
- **example8** — Not-sensible; disagrees with "live". Same shape as example7 at larger scale (4
  undifferentiated heterotrophs, 2000 ticks): a "full cascade with oscillations" is unrealisable
  without a decomposer role. Fails turnover and trophic-structure criteria. No longer flagged
  `energy_death`.

## Synthesis

This is a **stale, trophically-incomplete validation set, not a fleet of broken ecologies** — the
older files say so themselves (`status: stale-params` on most of the legacy set). Two structural
defects swamp the legacy scenarios: partial recipes drifting under code defaults (most damningly
`growth_efficiency`→0.0 in example1/2/3, guaranteeing collapse before any ecology runs), and
roster/intent drift — until #303, **no scenario in the suite contained a working decomposer**, so
carcasses accumulated unconsumed in every run. `example6_decomposer_viability` now closes that gap:
it is the first scenario to seed a decomposer that reads behaviourally as one and to drive the
producer→carcass→decomposer detrital loop end to end. Closing it also turned up *why* carcasses had
always accumulated — a drain-phase index/id bug (now fixed) meant no decomposer could consume a
carcass once any agent had died — so the "carcasses accumulate unconsumed" symptom was partly a code
defect, not only a roster gap.

The previously near-universal `energy_death` verdict was **mostly artifact, not signal**, and #302
has now removed it. The old detector summed only `Consumed` (predation) energy per tick — both
branches of the `||` were literally `EventKind::Consumed` — and fired whenever the final 50 ticks
lacked predation, which was true of every producer-dominated or consumer-collapsed world here. The
detector now measures what `expected-properties.md` actually defines as energy death: the **free
(non-carcass-locked) energy stock** — agent reserve + structure summed across the living population,
sampled each tick — *trending irreversibly toward zero*. It flags energy death only when that stock
collapses to a small fraction of its earlier peak and does not recover. None of the nine scenarios
trips it now, on any of the eight seeds: example4 sustains its living stock through active
reproduction, and the others decline slowly without the irreversible carcass-locked collapse the
property describes. The
false `energy_death` is gone, most importantly on example4 — the closest thing to a
live world in the suite.

With the detector corrected, cross-lens *agreement* improves; for the suite to *certify* sensible
worlds it still needs the README's repairs — migrate every file to fully-specified params (the
example4 template, #295) and seed a real decomposer roster (#136) so the detrital,
trophic-structure, and coexistence criteria can be exercised at all.
