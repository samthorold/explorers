# Scenario verdicts — is each scenario a *sensible* ecology?

**Generated artifact, not a hand-maintained note.** This is the *judged read* layer of
the validation triad (#293): the [observed evidence](observed.json) interpreted against
each scenario's declared `probes` / `prediction` (in its `metadata`), grounded strictly in
[`expected-properties.md`](../docs/system-design/expected-properties.md). It is a
*reading*, not a pass/fail test — precise numbers are evidence for the read, not the
gate.

**The verdict is the majority/supermajority read of a *distribution*, and that read lives
here, not in the binary (#314).** `eval_scenarios` runs each scenario over a
deterministic seed ensemble (`base_seed=1 .. base_seed+32` since #475, mirroring genesis's
`run_ensemble`) and emits the *distribution* — a failure-mode count + modal mode, the
median/min/max of every score, plus a `per_seed` breakdown. It stays prediction-agnostic
and verdict-free; the supermajority read against the declared prediction is made here. This
matters because regime-sensitive scenarios can flip on a single draw, so a
verdict hung on one seed is fragile. The columns below cite the **modal failure mode** and
the **fraction of seeds** matching the prediction; the per-row prose quotes the median and
its spread.

**What `n = 32` actually bounds (#434,
[`docs/research/434-ensemble-confidence.md`](../docs/research/434-ensemble-confidence.md)).** A
unanimous `32/32` read is a 95 % Clopper–Pearson lower bound of **`p ≥ 0.89`** (two-sided;
`≥ 0.91` one-sided) on the true per-seed rate of that mode — the earlier `8/8` bounded only
`p ≥ 0.63`. A non-unanimous mode is now readable rather than noise: at `n = 32` the observed mode
is wrong or tied 1.4 % of the time for a true 0.7/0.3 split (19 % at `n = 8`) and 17 % at 0.6/0.4,
and the two-sided interval at `16/32` is `[0.32, 0.68]`. So read every `n/32` column below as
"≥ 0.89" when it says `32/32`, as a *dominant* mode at `≥ 24/32`, and as a genuine **split
regime** — not an undecided read — at `20/32` (interval `[0.44, 0.79]`) or `12/32`
(`[0.21, 0.57]`). Eleven of the twelve scenarios are unanimous on their modal mode; the one
that is not (`example13`, `20/32`) is read below as a split and the split is explained.

**Regeneration log.** Regenerate by re-running `eval_scenarios` and re-judging (a human or a
fresh-perspective agent).

- **2026-09-12 (#475) — post-fix physics, `n = 32`.** Regenerated on the stepper carrying the six
  fixes #444 (founder trait floor), #445 (metabolic overdraft cap), #446 (nutrient bind), #451
  (chemotaxis sensing order), #452 (drain stoichiometry order / spent carcasses stay targets) and
  #453 (carcass nutrient release), at the `--seeds 32` default #434 recommended. Re-judged by an
  agent as a fresh-perspective reader. Where a scenario's distribution moved, the move was
  **bisected across the six fix commits** in a detached worktree (`example4`, `example10`,
  `example12`, `example13` at 8 seeds per commit) so each changed verdict names its fix. The
  legacy set (`example1/2/3/5/7/8`) and `example9` are byte-identical on their first 8 seeds and
  unanimous over 32 — none of the six fixes reaches a fixed-roster scenario with no surviving
  heterotroph. **Two verdicts changed**, both attributable to **#445** (with #451 as a secondary
  contributor to `example13`): `example12` still confirms its prediction but on a different
  economy (mobile specialist consumers now *persist*, compatible mixotrophs reach a 53 % median
  share), and `example13` moves from *sensible* to **partially-sensible / split** — its brown loop
  carries *more* flux than before (carcass-locked nutrient ≈ 800 vs ≈ 2300 pre-fix) but the
  decomposer guild has outgrown the producer level, and the modal `generalist_dominance` read
  (`20/32`) is localised to the **evaluator**, not the ecology (a DBSCAN cluster-mean artifact
  gated by the 20-agent population floor, see the row). The atlas cited where verdicts compare to
  atlas cells is the **post-fix atlas regenerated in #474 (PR #483)** — 82 live cells, recipe cell
  `[5, 19, 7]`, refined `coexistence_fraction` 0.91 at `n = 32`
  ([`474-atlas-regen-post-fix.md`](../docs/research/474-atlas-regen-post-fix.md)); that note's
  §4 attribution claim that #445 "changes the ledger, not the state" is **contradicted by this
  suite's bisect** (see `example12` / `example13`) and should be revisited.
- 2026-06-02 — re-judged three times: after #328 **retired `example6_decomposer_viability`**
  (trace inspection showed its producers mass-died in a single tick and its decomposer never
  established a lineage — pinned at count 1 for all 2000 ticks — so it demonstrated neither the
  viability nor the sustained carcass supply it claimed; emergent decomposers from the genesis
  search are now the real evidence) and **re-cast `example9_detrital_pathway` as a
  wiring/regression test rather than emergence evidence**; then after **`example9`'s decomposer
  was made obligate** (`photosynthetic_absorption` 0.4 → 0) and its deposit resized (480 → 12000
  energy): the old file's detrital pathway had not actually been load-bearing — a stray `0.4`
  autotrophy trait was funding the agent through photosynthesis, masking a deposit that lasted
  only ~134 ticks on detritus alone.
- Earlier rebuilds: after #302 replaced the energy-death detector with a
  free-energy-stock-trend test, after #313 made the structural death threshold *peak-relative* —
  a fraction of each agent's own peak structure — so newborns and seeds are born viable by
  construction rather than dead-on-arrival below an absolute floor, and after #309 gave feeding
  reach a sessile body-extent solution — `consumption_reach = effective_heterotrophy ×
  (contact_range_coefficient + body_reach_coefficient × √structure)` — so a growing sessile
  decomposer extends its reach to carcass-fall it could not previously touch (every remaining
  file keeps `body_reach_coefficient=0.0`).

| scenario | verdict | modal failure (n/32) | agrees with prediction? (seeds) | primary fault |
|---|---|---|---|---|
| example1 | inconclusive | none (32/32) | n/a | `max_ticks=0`, never steps |
| example2 | not-sensible | extinction (32/32) | agree (predicted live → dead), 32/32 | explicit `growth_efficiency=0` (the drifted default, frozen into the file by the #304 migration) |
| example3 | inconclusive | extinction (32/32) | n/a (undecided) | explicit `growth_efficiency=0` and `asexual_propensity=0` |
| example4 | partially-sensible | none (32/32) | partial, 32/32 survive | incomplete roster (no decomposer), low turnover; unchanged by the fixes |
| example5 | not-sensible | none (32/32) | disagree | roster/probe mismatch; 0 births fails turnover |
| example7 | not-sensible | none (32/32) | n/a (undecided) | roster mismatch (no decomposer); 0 births fails turnover |
| example8 | not-sensible | none (32/32) | disagree | roster mismatch (no decomposer); 0 births fails turnover |
| example9_detrital_pathway | wiring test (not emergence evidence) | nutrient_lockup (32/32) | n/a — by construction | wiring healthy locally; field-level lockup **unchanged by #452/#453** — those fixes mineralise a carcass only when a consumer *drains* it, and the ring's rain is still out of reach (locked fraction ≈ 46 % at tick 2000) |
| example10_predator_prey_hopf | not-sensible as a Hopf reference (producer monoculture from tick 2) | none (32/32) | disagree on the *coupling* (predicted live limit cycle → consumers extinct by tick 2 on every seed); producers persist 32/32 | spatial decoupling — the low-mobility consumer cannot reach the sparse standing crop ([`F-hopf-validation.md`](../docs/research/F-hopf-validation.md)); unchanged by the fixes |
| example11_branching_coexistence | not-sensible (no branching observed) | none (32/32) | disagree (predicted branching → 2–4 survivors, `clustering_strength` 0 on 31/32) | consumer level collapses to a remnant; the mean-field branching prediction is not realised ([`F-branching-validation.md`](../docs/research/F-branching-validation.md)); unchanged by the fixes |
| example12_generalist_dominance | sensible — confirms prediction on a **changed economy** (#445) | none (32/32) | agree (predicted live → broad generalists confined), 32/32 | none for the prediction — broad generalists 0 % energy on 32/32; **changed by #445**: specialist mobile consumers now persist (3–6 at tick 2000, were 0/8) and compatible mixotrophs hold a 53 % median share (44–57 %); `trophic_balance_score` reads 0 on 12/32 — an evaluator tie artifact (single merged DBSCAN cluster, producer count = consumer count), not an inverted pyramid |
| example13_closed_web | **partially-sensible / split regime** (was sensible) — closure *stronger*, pyramid *inverted*, modal read is an evaluator artifact (#445, #451) | generalist_dominance (20/32); none (12/32) | mixed — predicted live → no seed locks up (0/32 `nutrient_lockup`, the probed mode), decomposer lineage persists 32/32, but `trophic_balance_score` 0 on 32/32 | **#445** lifted the phantom starvation tax on the facultative guild, which grows 8 → 9–10 while producers thin to 6–8 — the "too-numerous guild inverts the pyramid" band the file's own metadata warned of; the `generalist_dominance` flag is localised to the **evaluator** (DBSCAN eps 1.0 merges all archetypes into one cluster whose *mean* trophic coordinates read as a generalist; it fires exactly on the seeds whose final population reaches the gate's 20-agent floor) — `probe_generalist` reads trophic-generalist energy 9–15 %, broad 0 % |

## Per-scenario fault localisation

- **example1** — Inconclusive. `max_ticks=0`: seeded, never stepped, so the numbers are the
  initial condition verbatim. `failure=none` only because the evaluator's grace-period guard
  is never crossed. A probe-nothing seed; tests no expected property.
- **example2** — Not-sensible; observation *agrees* with the "live" prediction being wrong (20→0
  is a real `Extinction`, on 32/32 seeds, all dead by tick 4). But it's right for the wrong reason:
  the file now spells out `growth_efficiency = 0` (the #304 migration froze the drifted code default
  into an explicit value, so the file is `status: current` but the fault is unchanged), so zero
  structure is built and collapse is mechanically guaranteed — the outcome reflects a zeroed
  parameter, not the light-competition/self-thinning ecology it claims to probe. None of the six
  stepper fixes touches it: the distribution is byte-identical on the first 8 seeds.
- **example3** — Inconclusive (prediction `undecided`). Extinction is foregone (32/32, tick 4):
  `growth_efficiency=0` *and* `asexual_propensity=0` (the latter precludes the lone-founder
  reproduction an isolation scenario needs), both explicit in the file. The drift/isolation question
  is untested. Unchanged by the fixes.
- **example4** — Partially-sensible, and the most diagnostic row. The fully-specified file
  (20 producers + 2 consumers, heterogeneous traits), and the only one with real turnover.
  Across the 32-seed ensemble it reads `failure=none` on **every** seed (`32/32 ⇒ p ≥ 0.89`;
  median **43 births, 58 deaths**, median final pop 8 spanning 5–10, median fitness **0.483**
  spanning 0.41–0.52). **Unchanged in kind by the six fixes**: the bisect moves its 8-seed median
  fitness only 0.479 → 0.474 (#445) → 0.487 (#451), its first 8 seeds keep the same modal mode, and
  the wider spread is the larger ensemble making the tail legible (one seed at fitness 0.41 with
  `oscillation_strength` 0), not new physics. Its `oscillation_strength` median is **0.36**
  (0–0.55; 25/32 seeds read > 0). Its
  free-energy stock is sustained by an actively reproducing living system, so the previous
  `energy_death` label is confirmed to have been a detector artifact (the old Consumed-only series
  read all-zero once predation tapered in the tail). #313's peak-relative death threshold raised its
  survivors — newborns that the absolute floor used to kill on arrival now live — which is the
  expected ecological direction of the root fix. **Re-checked after #380** (this scenario was flagged
  degenerate under the mobile-feeding bug): with consumption now a binary-reach drain the mobile
  consumers actually feed, and the predator–prey pair persists nearly the full run on most seeds
  rather than the prey escaping once the starving consumers died off; turnover lifts with it
  (births 34→43, deaths 48→58). **Metric note (#394):** `coexistence_duration` now reads **0** here. The
  earlier high reading (≈0.99) was the lineage-clade false positive #394 removed — two persisting
  lineages (producers + consumers) counted as "coexistence." Under the trait-space cluster metric the
  living population does not resolve into ≥2 *persistent* clusters across the post-grace window, even
  though `clustering_strength` is 1.0 (the final snapshot is multimodal): existence ≠ persistence, and
  a predator–prey trophic split is not a trait-space niche split. The ensemble makes its modest
  seed-to-seed wobble legible (pop 5–10) without changing the read — it is unanimously `none`, never
  collapsing. It is still not a *complete* sensible world — no decomposer role, and no registered
  trait-cluster coexistence — but it remains the closest the legacy set has to a live ecology, and the
  detector now treats it as such. (This file is the former
  `example4_consumer_tuning`, promoted to the canonical `example4` slot; the legacy degenerate
  example4 — triple-zeroed mate-limited producers, 0 births — has been retired.)
- **example5** — Not-sensible; disagrees with "live". Roster/probe mismatch ranks first: it declares
  `probes=population_explosion` yet seeds only 2 consumers on stale params and stalls at a final pop
  of 3 (median, identical across all 32 seeds) — the opposite regime — so it can't exercise the
  negative feedbacks it claims. 0 births on every seed fails turnover (fitness 0 throughout). The
  modal failure mode is `none` (survives 1500 ticks without tripping a detector), but a birthless,
  non-reproducing stall is not a sensible ecology. No longer flagged `energy_death`.
- **example9_detrital_pathway** — A **pathway wiring / regression test, not emergence evidence.**
  Its headline property — `detrital_share > 0.5` (in practice ≈0.9–1.0) on every seed — is **true by
  construction**, not a finding about dynamics. The decomposer is **obligate** (`photosynthetic_absorption
  = 0`, so it has no solar income) and seeded on a standing carcass deposit (a `carcasses` recipe capability, #311) with
  **no living agent inside its consumption reach**: with `body_reach_coefficient=0.0` the reach is
  structure-independent and exact — `heterotrophy × contact_range_coefficient = 0.5 × 1.0 = 0.5` world
  units — and the nearest living agent (a producer ring at radius ~30 on a 100-extent torus) is far
  outside it. Being obligate, the deposit is the decomposer's *only* energy source, so the diet is
  detrital not just by geometry but by physiology. That makes this scenario a clean **regression on the
  producer→carcass→decomposer code path**: it verifies energy and nutrient route end to end through
  detritivory (the brown pathway closes, drains carcasses, and returns nutrient), which is genuinely
  useful as a wiring guard. It is **not** evidence that a detritivore niche *arises from dynamics* — the
  diet is detrital because no producer is reachable and (being obligate) it cannot photosynthesise, not
  because detritivory won out in the ecology. (`detrital_share` is ≈0.9–1.0 rather than exactly 1.0 for an
  endogenous reason — the decomposer's own asexual offspring are born co-located, inside the 0.5 reach, so
  a little parent/offspring cannibalism dilutes the purity; no producer is ever predated.) The **local
  wiring is healthy**: the out-of-reach producer ring reproduces and self-thins (≈2200 births / ≈2100
  deaths, final pop ≈90) and the brown loop closes at the deposit (`detrital_share` ≈0.9–1.0). A
  12000-energy seeded deposit — sized to genuinely outlast `max_ticks` (the obligate decomposer draws it
  down at ~3.6 energy/tick and reaches tick 2000 with ~40% unspent) — backstops the decomposer as a single
  individual; an earlier 480-energy deposit had *appeared* to suffice only because a
  `photosynthetic_absorption=0.4` trait was quietly funding the agent through the green pathway (it
  starved at tick ~134 once photosynthesis was removed), so the detrital pathway was never actually
  load-bearing until that fix. But the **field-level verdict is `nutrient_lockup` (32/32, fitness 0.0)**:
  the ring's carcasses fall near radius ~30, out of reach, so they **accumulate unconsumed** and nutrient
  silts irreversibly into the dead pool (~46% of system nutrient by tick 2000 on seed 1: 23 800 of
  51 300 units, 1 923 carcasses standing) — the brown loop closes only
  *locally* at the deposit; the field-wide rain is not a self-sustaining detrital web. The evaluator now
  **registers that lockup** (#342); before, it scored the scenario healthy 8/8, blind to the very pathology
  the scenario targets. **Not moved by #452/#453 — and that is the informative null.** #475 expected the
  carcass-nutrient fixes to shift this read; they did not, because both mineralise a carcass's nutrient
  only when a *drain* exhausts it (#452 keeps a spent carcass a target while it holds stock; #453
  releases the rounding remainder on exhaustion). A carcass nobody can reach is never drained, so the
  ring's rain locks exactly as before: the lockup here is a *reach* pathology, not a bookkeeping one,
  which is the localisation the scenario was built to make. (The exact birth/death/population medians
  vary seed-to-seed *and* run-to-run —
  example9's high-population producer-ring + carcass path is non-deterministic, tracked in #343; the
  `nutrient_lockup` verdict itself is stable across runs.)
  Emergence evidence — that decomposers and a detrital niche
  arise without being hand-built — now comes from the genesis search (71/120 viable random worlds
  produced decomposers, guilds up to 235, including from full-random founders), not from this file; a
  dedicated genesis-emergence regression is a deferred follow-up.
- **example12_generalist_dominance** — **Sensible; confirms the design prediction (#325) — now on a
  changed economy (#445), and still un-confounded (#380).** The probe pits four archetypes in one viable
  world (survives 2000 ticks on every seed, `failure=none` 32/32, `clustering_strength` 1.0 throughout):
  specialist producers, specialist mobile consumers, **broad** generalists (autotrophy + heterotrophy +
  mobility — the rooted-producer + roving-hunter) and **compatible** generalists (autotrophy +
  heterotrophy, sessile). Broad and compatible generalists are seeded identically (reserve 80, autotrophy
  0.5, heterotrophy 0.5) and differ *only* in mobility (0.5 vs 0.0), so their fates isolate the
  sessile/mobile incompatibility axis as an in-run control. The breadth/dominance measure is read
  directly off survivor traits by `probe_generalist` (a *trophic generalist* invests in both autotrophy
  and heterotrophy, photo > 0.25 ∧ het > 0.25; a *broad generalist* additionally invests mobility > 0.25;
  share is energy-weighted):
  - **Broad generalists are eliminated — 0 survivors and 0.0 % energy share on every seed** (32/32 by the
    evaluator's cluster read, 8/8 by `probe_generalist`). The energy-advantaged, light-and-prey-co-located
    rooted-rover cannot establish. This holds with mobile consumers demonstrably feeding *and now
    persisting* (below), so the confinement is attributable to incompatibility + fragility, not to
    broken feeding.
  - **Compatible (sessile) mixotrophs rise to parity and slightly past it** — energy share 44–57 % across
    the probed seeds, **median 53 %** (was 44 % on the pre-fix physics, 20 % before #380). They are a
    *legal*, non-incompatible combination, so the design never predicted them confined; fragility alone
    holds them off runaway, and it still does — they co-exist with specialists rather than displacing
    them (6–7 of 15–19 survivors). The previous verdict said a decisive crossing into dominance would put
    fragility-alone under pressure; a 53 % median with a 44–57 % spread is *parity*, not a decisive
    crossing, but the direction of travel across three regenerations (20 → 44 → 53 %) is the number to
    keep watching.
  - **Specialist mobile consumers now persist** — 3–6 alive at tick 2000 on every probed seed, where the
    pre-fix read had 0 survivors on 8/8. This is the **#445 metabolic-overdraft cap** at work, and the
    bisect pins it there (8-seed final pop 12.5 → 17.0 and births 384 → 544 at `be21bf0`; #451–#453 add
    ≤ 0.5 to either). Mechanism: the tick runs metabolise → drains → death check, and the old stepper
    charged the full metabolic cost even when reserve could not cover it, driving reserve negative; an
    agent that had run its reserve dry then needed *same-tick* feeding income exceeding the overdraft
    to survive the death check, where it now needs any positive income. That phantom debt was a
    starvation tax on exactly the feast-famine heterotroph — a mobile consumer between prey — and
    lifting it is why the consumer level survives here. (This contradicts
    [`474-atlas-regen-post-fix.md`](../docs/research/474-atlas-regen-post-fix.md) §4, which reasons that
    #445 "changes the ledger, not the state"; the scenario lens shows it changes who lives.)
  - Specialist producers hold 11–13 of the survivors; final population is 15–19 (median 17.5, up from
    12.5), turnover 0.26–0.33 (median 0.29, up from 0.21) as the persisting consumer level keeps the
    ring cycling.
  **Metric caveat — `trophic_balance_score` reads 0 on 12/32 seeds, and it is a tie artifact, not an
  inverted pyramid.** The evaluator labels clusters with DBSCAN at `eps = 1.0` over the trait vector;
  the four archetypes sit within ~0.6 of one another, so the whole population is labelled **one**
  cluster, and `trophic_balance_score` then compares that single cluster's *mean* normalised trophic
  coordinates (`avg_photo > avg_hetero` → producer, else consumer, with the tie going to consumer).
  With producers at coordinate (1, 0), mixotrophs at (0.5, 0.5) and consumers at (0, 1), the mean is
  ≥ 0.5 exactly when producers outnumber consumers: the 12 seeds that read 0 are those whose final
  count has producers = consumers (6 = 6 on the probed seeds 1, 4, 6); the 20 that read 1 have producers
  > consumers. It is a count-parity knife edge on one merged cluster, not an energy-pyramid read — which
  is why fitness is bimodal (0.31–0.35 vs 0.51–0.59) with no ecological bimodality behind it. The gate's
  20-agent floor is why `generalist_dominance` does not fire here as it does in `example13`: the same
  merged cluster's mean (≈ 0.5, 0.5) sits above the 0.3 generalist threshold on both axes, so any seed
  reaching 20 survivors would be flagged (max here is 19). **Fault localised to the evaluator's
  cluster-level averaging at small n**, surfaced by the example lens; no change to scoring is made
  here.
  This happens with `mobility_maintenance_cost = 0`, `wear_rate = 0`, and **no cross-trait interaction
  maintenance term** — so the confinement is delivered by the committed structural fragility (#9,
  higher trait-entropy → higher peak-relative death threshold; survivor mean fragility ≈ 0.87) and the
  emergent sessile/mobile functional incompatibility (#2), plus the breadth-neutral movement cost,
  *without* the reserve lever. **Un-confounding (post-#380):** the earlier confirmation predated the #379
  fix, in which a contact-duration consumption ramp reset on every move and left *any* mobile consumer
  unable to feed at all — confounding this in-run control (broad generalists + specialist mobile
  consumers are mobile; the surviving compatible mixotrophs are sessile). With consumption a binary-reach
  drain (#380), mobile consumers feed — proven by the `mobile_consumer_feeds` integration test — and
  with #445 they persist; the broad generalist is therefore eliminated *despite being able to feed and
  despite the mobile-consumer niche being viable in this very world*. **Verdict: generalists stay
  confined; the design prediction holds — on a cleaner control than before — and the cross-trait
  interaction term stays in reserve** (see [`viability.md`](../docs/system-design/viability.md),
  "Resolved finding — generalist dominance has no static gate"). For comparison with the search lens:
  the post-fix atlas (#474, PR #483) records generalist dominance on only 1 of 51 dead configs (down
  from 2), consistent with the scenario read. Regenerate the breadth read with
  `cargo run -p explorers-genesis-eval --bin probe_generalist -- scenarios/example12_generalist_dominance.json`.
- **example10_predator_prey_hopf** — **Not-sensible as a Hopf reference; a producer monoculture from
  tick 2.** (Previously judged only inside the `example12` entry, #400; given its own row here.) The
  scenario commits `base_trophic_efficiency = 0.8` above the mean-field Neimark–Sacker crossing so
  Brief F predicts a limit cycle, but the three consumers go **extinct by tick 2** on every seed traced
  (11 of 32, all identical: 17 producers / 0 consumers at tick 2, 14–16 / 0 at tick 2000) — the
  low-mobility consumer cannot sustain intake from the sparse, trait-distant standing crop (spatial
  decoupling, [`F-hopf-validation.md`](../docs/research/F-hopf-validation.md)). Its 32/32 `none` is the
  *producers* persisting (final pop 14–16, 28–48 births), and with producer energy share pinned ≈ 1.0
  the #393 descriptor correctly reads `oscillation_strength` 0 on 32/32 (the pre-fix 8-seed max of 0.28
  was one seed's finite-N pulsing; gone at n = 32 on the same seed). The fitness distribution is
  bimodal — 0.20 on 23 seeds, 0.40 on 9 — on `clustering_strength` alone (0 vs 1: whether the final
  producer snapshot happens to be multimodal), not on any consumer signal. **Unchanged by the fixes**
  (bisect: modal mode and population identical at every commit; the 8-seed median fitness moves 0.205
  → 0.305 at #445 only because one seed crossed the bimodal midpoint). Prediction `live` disagrees with
  the observation on the coupling it was written to test. Not an oscillation reference.
- **example11_branching_coexistence** — **Not-sensible; the predicted branching is not observed.**
  Brief F's moment-closure prototype predicts evolutionary branching (a producer peak + a consumer peak)
  at the committed `base_trophic_efficiency = 0.8`, just above its crossing `base* ≈ 0.63`. Observed:
  `none` 32/32 (nothing trips a detector), but the world collapses to a **2–4-agent remnant** (median
  2, 7–18 births / 17–28 deaths over 2000 ticks), `clustering_strength` 0 on 31/32, `coexistence_duration`
  0, `trophic_balance_score` 0 on 32/32, fitness median 0.002 (one seed at 0.20 on a lucky final
  snapshot). A 2-agent survivor set cannot exhibit the ≥ 2 persistent trait clusters coexistence
  requires, so the observation **disagrees** with the `live` (branching) prediction; the fault is
  localised by [`F-branching-validation.md`](../docs/research/F-branching-validation.md) to the
  well-mixed mean-field assumption, not to the stepper. **Unchanged by the fixes** (first 8 seeds keep
  the same modal mode; the pre-fix 8-seed fitness max 0.067 and the new max 0.20 are single-seed
  final-snapshot draws).
- **example13_closed_web** — **Partially-sensible, a split regime — was sensible (#136). The brown loop now
  carries *more* flux than on the old physics, but the decomposer guild has outgrown the producer level, and
  the modal `generalist_dominance` read is an evaluator artifact, not an ecological one.** The file takes
  `example12`'s four-archetype world verbatim and adds a lean guild of 8 sessile **facultative**
  detritivores (photo 0.25 + het 0.6, heterotroph-dominant so the sim classifies them as decomposers) on the
  producer ring, *within* the 2.5-unit reach of the ring's carcass rain — the deliberate inversion of
  `example9`. Over 32 seeds it reads **`generalist_dominance` 20/32, `none` 12/32** (interval on the modal
  rate `[0.44, 0.79]` — a genuine split, not an undecided read), `trophic_balance_score` **0 on 32/32**,
  fitness median 0 (0 on the flagged seeds, 0.34–0.37 on the rest), final population 17–22, turnover
  0.32–0.38 on the unflagged seeds (median births 669 / deaths 710, up from 554 / 596). Three findings,
  established by trace inspection (seeds 1–6) and the fix-by-fix bisect:
  - **The closure is stronger, not weaker.** Carcass-locked nutrient at tick 2000 is **≈ 500–1 280 units
    (median ≈ 800)** against the no-decomposer `example12` baseline's ≈ 1 750–2 670 on the same physics
    (median ≈ 2 100) — a **~60 % cut**, up from the ~40 % cut read on the pre-fix physics (≈ 2 300 vs
    ≈ 3 900). No seed trips `nutrient_lockup` — the mode this scenario declares it `probes` — and the
    decomposer lineage persists as a reproducing guild on 32/32 (9–10 alive at tick 2000, from 8 seeded).
    On the property the scenario was built for, it reads *better* than before. (Both baselines fell under
    #452/#453 — spent carcasses now mineralise rather than stranding their remainder — which is why
    `example12`'s dead pool halved with no decomposer at all; the *relative* cut is the decomposition
    signal.)
  - **The pyramid has inverted, and #445 is why.** The bisect flips this scenario at `be21bf0` (#445, the
    metabolic-overdraft cap): 8-seed `none` 8 → 6, `trophic_balance_score` 1.0 → 0.0 on every seed, fitness
    0.565 → 0.36; #451 (chemotaxis sensing order) then takes it to `none` 4 / `generalist_dominance` 4 and
    fitness 0.18; #452/#453 add nothing further. The file's own `intent` records that its guild sits in "a
    narrow honest band": an obligate saprotroph starves by tick ~40, and "a too-autotrophic / too-numerous
    guild inverts the pyramid". That band was tuned on the old stepper, where an agent that overdrew its
    reserve needed same-tick income exceeding the overdraft to survive the death check (metabolise → drains
    → death; see `example12`). The facultative decomposer — "just enough autotrophy to clear baseline
    metabolism between carcass meals" — is precisely the agent that lived at that edge, and the phantom
    debt was culling it. Without the tax the guild grows 8 → 9–10 while the producer ring thins to 6–8
    (sim role counts at tick 2000, seeds 1–6; the ring was 9–11 before), and the heterotroph level
    (decomposers + 1–3 surviving mobile consumers) now *outnumbers* the producer level — the inversion
    the author's sweep had steered clear of. That is a real ecological change and it is the honest reason
    the verdict drops from *sensible*: a producer-led pyramid is one of the expected properties, and this
    world no longer shows one.
  - **The `generalist_dominance` flag is the evaluator's, not the ecology's.** `probe_generalist` reads
    trophic-generalist (compatible mixotroph) energy at **9–15 %** and broad-generalist energy at **0 %**
    on every probed seed — nothing is dominating from above. The flag comes from the same DBSCAN merge as
    `example12`'s tie artifact: at `eps = 1.0` the producers (coordinate 1, 0), mixotrophs (0.5, 0.5),
    consumers (0, 1) and the 9–10 decomposers (≈ 0.3, 0.7) are labelled **one** cluster, and with the
    heterotroph-side agents now in the majority that cluster's *mean* coordinates land near (0.4, 0.6) —
    above the 0.3 generalist threshold on both axes — so `is_generalist_dominant` assigns 100 % of energy
    to a "generalist" cluster. The gate additionally abstains below 20 agents, which is exactly the split:
    the 20 flagged seeds end at population 20–22, the 12 `none` seeds at 17–19. The same mean also drives
    `trophic_balance_score` to 0 on all 32 (the cluster is consumer-side), so the metric is reporting the
    inversion correctly in *direction* but by a count-average on a merged cluster, not by energy through
    labelled levels. **Fault localised to the evaluator's cluster-level averaging** (a disagreement between
    the example lens's per-agent read and the search lens's cluster read, of the kind the triad exists to
    surface); the scoring is deliberately left unchanged by #475.
  **Honest caveats, three, revised.** (1) Closure remains **partial** — the dead pool is ≈ 800 units, not
  0, and 9–10 sessile decomposers with a structure-independent 2.5-unit reach (`body_reach_coefficient=0`)
  still cannot blanket the ~190-unit ring; full field-scale closure stays the *decomposer reach* knob
  [`viability.md`](../docs/system-design/viability.md) names. (2) The "narrow honest band" the file's
  metadata describes is a property of the *pre-#445* physics; on the fixed stepper the lean facultative guild
  is no longer lean enough to keep a producer-led pyramid. Re-tuning the guild (fewer, or less autotrophic)
  is a scenario edit that #475 deliberately does not make — the file stays as authored and the verdict
  reads the world it now produces. (3) The scenario is a web-closure probe, not a generalist probe;
  `example12` remains the canonical generalist test. **Verdict: the brown loop closes structurally and
  functionally inside a live web — better than ever on its own probed mode (0/32 lockup, ~60 % less locked
  nutrient) — but the pyramid it sits in is now inverted (a real #445 effect), and the modal
  `generalist_dominance` read is an evaluator artifact to be fixed in the evaluator, not in the file.**
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
roster/intent drift — for a long time **no scenario in the suite contained a working decomposer**, so
carcasses accumulated unconsumed in every run. `example9_detrital_pathway` (#311) now drives the
producer→carcass→decomposer detrital loop end to end — but only as a **wiring test**: it forces a
detrital diet by physiology and geometry (an *obligate* decomposer, `photosynthetic_absorption = 0`, on
a hand-placed deposit sized to outlast the run, with the producer ring out of reach), so it proves the
code path closes, not that a detritivore niche emerges. Building this pathway turned up
*why* carcasses had always accumulated — a drain-phase index/id bug (now fixed, guarded by
`decomposer_drains_carcass_after_a_death_reindexes_agents`) meant no decomposer could consume a
carcass once any agent had died — so the "carcasses accumulate unconsumed" symptom was partly a code
defect, not only a roster gap. (`example6_decomposer_viability` was retired in #328: it claimed to
prove decomposer viability but its producers mass-died in a single tick and its decomposer never
established a lineage. Whether decomposers *emerge* is now answered by the genesis search — 71/120
viable random worlds produced decomposers — not by a hand-built scenario.)

The previously near-universal `energy_death` verdict was **mostly artifact, not signal**, and #302
has now removed it. The old detector summed only `Consumed` (predation) energy per tick — both
branches of the `||` were literally `EventKind::Consumed` — and fired whenever the final 50 ticks
lacked predation, which was true of every producer-dominated or consumer-collapsed world here. The
detector now measures what `expected-properties.md` actually defines as energy death: the **free
(non-carcass-locked) energy stock** — agent reserve + structure summed across the living population,
sampled each tick — *trending irreversibly toward zero*. It flags energy death only when that stock
collapses to a small fraction of its earlier peak and does not recover. None of the scenarios
trips it now, on any of the 32 seeds: example4 sustains its living stock through active
reproduction, and the others decline slowly without the irreversible carcass-locked collapse the
property describes. The
false `energy_death` is gone, most importantly on example4 — the closest thing to a
live world in the suite.

With the detector corrected, cross-lens *agreement* improves; for the suite to *certify* sensible
worlds it still needs the README's repairs — migrate every file to fully-specified params (the
example4 template, #295) and seed a real decomposer roster (#136) so the detrital,
trophic-structure, and coexistence criteria can be exercised at all.

**Update (#136, partially closed).** `example13_closed_web` is the first scenario to seed a working
decomposer roster *into a live differentiated green web* (it extends `example12` verbatim with a lean
facultative detritivore guild), and the brown loop demonstrably carries flux there: the guild
persists as a reproducing lineage on all 32 seeds and cuts carcass-locked nutrient ~60 % vs the
no-decomposer baseline on the same physics. So the "no scenario contains a working decomposer in a
live web" gap is closed for the *structural and functional* case. What remains open is **full
field-scale** closure: 9–10 sessile decomposers cannot reach the whole ring's carcass rain
(reach-limited by `body_reach_coefficient=0`), so the dead pool is reduced, not eliminated.
Completing it is a *design* question — commit a decomposer reach term (or admit a mobile
detritivore) — exactly the under-committed knob
[`viability.md`](../docs/system-design/viability.md) flags, not a further scenario tweak.

**Update (#475, post-fix regeneration at `n = 32`).** Three things the fixed stepper taught the suite:

1. **#445 changes who lives, not just the ledger.** The metabolic-overdraft cap is the one fix that moves
   any scenario here (bisected: `example12` and `example13` flip at `be21bf0`; #451 adds a secondary
   shift to `example13`; #444/#446/#452/#453 leave every scenario's modal mode where it was). Because the
   tick runs metabolise → drains → death check, the old phantom overdraft was a same-tick starvation tax
   on any heterotroph that ran its reserve dry between meals — the mobile consumer and the facultative
   decomposer. Lifting it lets `example12`'s consumer level persist (a cleaner generalist control) and
   lets `example13`'s guild outgrow its producers (an inverted pyramid). Both are the physics doing what
   the fix intended; only the second costs a verdict.
2. **The evaluator's cluster-level trophic reads are fragile at small n.** DBSCAN at `eps = 1.0` merges
   every archetype in `example12`/`example13` into one cluster, after which `trophic_balance_score` and
   `is_generalist_dominant` reason about that cluster's *mean* trophic coordinates: a count-parity knife
   edge in `example12` (0 on 12/32, bimodal fitness with no ecological bimodality), and a spurious modal
   `generalist_dominance` in `example13` (20/32, gated by the 20-agent population floor) while the
   per-agent `probe_generalist` read shows 0 % broad and 9–15 % trophic-generalist energy. This is the
   suite's second evaluator fault surfaced by cross-lens disagreement (after the #302 energy-death
   detector). It is localised, not fixed, here.
3. **`example9` is the informative null.** #452/#453 release carcass nutrient only through a *drain*;
   unreachable carcasses lock exactly as before (46 % of system nutrient at tick 2000). The lockup the
   scenario registers is a reach pathology, which is what it was built to show.

Where verdicts compare to the search lens they cite the post-fix atlas from #474 (PR #483): 82 live
cells, recipe cell `[5, 19, 7]`, refined `coexistence_fraction` 0.91 at `n = 32`. Its §4 attributes no
state change to #445 by construction; this suite's bisect says otherwise, and the two readings should
be reconciled in a follow-up rather than left to disagree silently.
