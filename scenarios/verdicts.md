# Scenario verdicts — is each scenario a *sensible* ecology?

**Generated artifact, not a hand-maintained note.** This is the *judged read* layer of
the validation triad (#293): the [observed evidence](observed.json) (computed by
`eval_scenarios`, seed 1) interpreted against each scenario's declared `probes` /
`prediction` (in its `metadata`), grounded strictly in
[`expected-properties.md`](../docs/system-design/expected-properties.md). It is a
*reading*, not a pass/fail test — precise numbers are evidence for the read, not the
gate. Regenerate by re-running `eval_scenarios` and re-judging (a human or a
fresh-perspective agent); the verdict below was rendered by an agent on 2026-05-31.

| scenario | verdict | agrees with prediction? | primary fault |
|---|---|---|---|
| example1 | inconclusive | n/a | stale params — `max_ticks=0`, never steps |
| example2 | not-sensible | agree (predicted live → dead) | stale params (`growth_efficiency` unset → 0.0) |
| example3 | inconclusive | n/a (undecided) | stale params (`growth_efficiency`=0, `asexual_propensity`=0) |
| example4 | not-sensible | disagree | stale params, then energy-death **detector artifact** |
| example4_consumer_tuning | not-sensible | disagree | **detector artifact** over genuine failure |
| example5 | not-sensible | disagree | roster/probe mismatch, then stale params + artifact |
| example7 | not-sensible | n/a (undecided) | roster mismatch (no decomposer), then stale params + artifact |
| example8 | not-sensible | disagree | roster mismatch (no decomposer), then stale params + artifact |

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
- **example4** — Not-sensible; disagrees with "live". 0 births = no demographic turnover, so it
  fails that criterion regardless of the detector. Two faults stack: partial params + retired
  vocabulary (the recorded params aren't the world that runs), then an `energy_death` label that
  is itself a detector artifact (no predation in the tail).
- **example4_consumer_tuning** — Not-sensible, but the most diagnostic row. The one fully-specified
  file, and the only one with real turnover (**36 births, 51 deaths**, final pop 7). Its
  `energy_death` verdict is most likely a **detector artifact**: predation tapered in the final
  50-tick window so the Consumed-only energy series read all-zero, even though births prove the
  living system was reproducing. (It is still not a *complete* sensible world — final pop 7, no
  decomposer — it just isn't dying the way the label claims.)
- **example5** — Not-sensible; disagrees with "live". Roster/probe mismatch ranks first: it declares
  `probes=population_explosion` yet seeds only 2 consumers on stale params and collapses toward
  extinction — the opposite regime — so it can't exercise the negative feedbacks it claims. 0 births
  fails turnover.
- **example7** — Not-sensible (prediction `undecided`). Roster mismatch is primary: intent is
  "three trophic roles incl. a decomposer", but the roster is 3 *undifferentiated* mobile consumers —
  no decomposer exists, so the detrital pathway it means to probe is absent and carcasses accumulate.
- **example8** — Not-sensible; disagrees with "live". Same shape as example7 at larger scale (4
  undifferentiated heterotrophs, 2000 ticks): a "full cascade with oscillations" is unrealisable
  without a decomposer role. Fails turnover and trophic-structure criteria.

## Synthesis

This is a **stale, trophically-incomplete validation set, not a fleet of broken ecologies** — the
files say so themselves (`status: stale-params` on 7 of 8). Two structural defects swamp everything:
partial recipes drifting under code defaults (most damningly `growth_efficiency`→0.0 in
example1/2/3, guaranteeing collapse before any ecology runs), and roster/intent drift (**no scenario
in the suite contains a decomposer**, so carcasses accumulate unconsumed in every run).

The near-universal `energy_death` verdict is **mostly artifact, not signal**. The detector
(`crates/explorers-genesis-eval/src/lib.rs:100-104`) sums only `Consumed` (predation) energy per
tick — both branches of the `||` are literally `EventKind::Consumed` — and fires whenever the final
50 ticks lack predation, which is true of every producer-dominated or consumer-collapsed world here.
But `expected-properties.md` defines energy death as *free energy trending to zero / energy locking
in carcasses because decomposers aren't viable* — a free-energy/carcass-stock property, not a
predation-flow one. **The single most leveraged fix is to make the energy-death detector measure
actual free-energy throughput** (photosynthetic input vs. dissipation and carcass-locked energy)
rather than Consumed-only flow. That alone would clear the false `energy_death` on
example4_consumer_tuning — the closest thing to a live world in the suite.

The detector fix improves cross-lens *agreement*; for the suite to *certify* sensible worlds it
still needs the README's repairs — migrate every file to fully-specified params (the
example4_consumer_tuning template, #295) and seed a real decomposer roster (#136) so the detrital,
trophic-structure, and coexistence criteria can be exercised at all.
