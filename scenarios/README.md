# Scenarios

Hand-authored `WorldRecipe` files — deterministic initial conditions used to probe
specific ecological behaviours by hand, outside the genesis search. Each is a small,
legible world (a named roster of agents at fixed positions) chosen to exercise one
mechanism or stress one [failure mode](../docs/system-design/expected-properties.md).

A scenario is one leg of the **validation triad** described in
[`docs/system-design/viability.md`](../docs/system-design/viability.md): the same
initial condition can be *predicted a priori* (viability lens), *observed in a run*
(these files + a headless harness), and *located in the search* (genesis). Agreement
across the three raises confidence; disagreement localises the fault.

## File format

A scenario is a `WorldRecipe` (`crates/explorers-sim/src/lib.rs`): a `parameters`
block (`WorldParameters`), an optional `agents` list (fixed roster; omit for a random
seeded population), and `max_ticks`. Loaded via `--scenario PATH` / `--recipe PATH`
in `explorers-app`, or `serde_json::from_str` in tests.

Open one in the GUI:

```
cargo run -p explorers-app -- --scenario scenarios/example4.json
```

Or run it **headless** (no window) and emit per-tick telemetry as JSON-lines to
stdout — population, births/deaths, the producer/consumer/decomposer split, the
energy budget, the nutrient pools, and a `reproduction` block summarising both
reproductive earmarks against their gates (overall and per role, so a birthless
tick reads as energy-gated, nutrient-gated, mate-limited, or dying — #280) — for
grepping or plotting (#279). Pass `--seed N` for a reproducible run; the chosen
seed is logged to stderr either way:

```
cargo run -p explorers-app -- --scenario scenarios/example4.json --trace --seed 1
```

`--trace` is per-tick and fine-grained (for debugging *why* a run behaves as it does).
For the coarse per-scenario *outcome*, see the validation triad below.

## Metadata header — the scenario's declarations

Every file carries a top-level `metadata` object: what the author **declares** about the
scenario. The loader ignores it (`WorldRecipe` doesn't declare these fields and serde drops
unknown keys), so it never changes how the scenario runs — it is the a priori half of the
cross-validation.

| field | meaning |
|---|---|
| `intent` | what mechanism/pattern the scenario is meant to exercise |
| `source_issue` | the GitHub issue that defined it |
| `roster` | the agents it seeds, by trophic role |
| `probes` | the [failure mode](../docs/system-design/expected-properties.md) it is designed to stress — one of `extinction` / `energy_death` / `monoculture` / `population_explosion` / `generalist_dominance`, or `none` |
| `prediction` | a priori call — `live` / `dead` / `undecided` |
| `rationale` | why that prediction |
| `status` | health of the *file* — `current` or `stale-params` |

## The validation triad

A scenario is one leg of the triad in [`viability.md`](../docs/system-design/viability.md):
the same initial condition is **predicted** a priori (the `metadata` above), **observed** by
a run, and **located** by genesis. Three artifacts make that concrete:

1. **Declarations** — `metadata` in each `*.json` (above).
2. **Observed evidence** — [`observed.json`](observed.json), *computed*, never hand-typed.
   Produced by running every scenario through the **same evaluator genesis uses**
   (`evaluate_from_log`), so "sensible" means the same thing to the example lens and the
   search lens. It is an **ensemble distribution, not a single seed (#314)**: each scenario
   is run over a deterministic seed set (`base_seed=1 .. base_seed+8`, mirroring genesis's
   `run_ensemble`), so the evidence is robust to regime-sensitive scenarios that flip between
   regimes on a single draw. Regenerate (same args, same seed set):
   ```
   cargo run -p explorers-genesis-eval --bin eval_scenarios -- scenarios/example*.json > scenarios/observed.json
   ```
   `--seed N` sets the base seed (default 1); `--seeds N` the ensemble size (default 8, the
   same set the headless harness sweeps). Each scenario's entry carries a **failure-mode
   distribution** (count over the six + modal mode), the **median and min/max spread** of the
   five sensible-world scores + `fitness`, the same spread for `ticks_survived` / final
   population / true birth & death counts, the seed set used, and a `per_seed` array with each
   seed's individual row. The binary stays **prediction-agnostic and verdict-free** — it does
   not read `metadata.prediction` or apply a pass/fail threshold.
3. **Judged verdict** — [`verdicts.md`](verdicts.md): the *read* of the evidence against the
   declarations, grounded in `expected-properties.md`. A reading (by a human or a
   fresh-perspective agent), **not** a pass/fail test — precise numbers are evidence for the
   judgment, not the gate. The verdict is a **majority/supermajority read of the distribution**
   (modal failure mode + fraction of seeds matching the declared prediction); that read lives
   here, deliberately *not* in the binary. Drift shows up as a diff in `observed.json`,
   prompting a re-judge.

Agreement across the three raises confidence; **disagreement localises the fault** — and
already has: the verdict synthesis traces the suite's near-universal `energy_death` verdict to
a detector that measures predation flow rather than free-energy throughput (a genesis-evaluator
issue, surfaced by the example lens).

## Current status (8-seed ensemble, base seed 1)

> **The legacy suite is stale and trophically incomplete** — `status: stale-params` on most of
> the older files. Numbers below are ensemble medians; `observed.json` carries the full spread.
> Across the current suite all eight seeds agree on the modal failure mode for every scenario, so
> the ensemble *confirms* the earlier single-seed reads were not lucky draws — but the demographic
> and score spreads (example4 final pop 6–11) are now legible.
> `example4.json` reproduces (median 34 births); `example9_detrital_pathway.json` (#311) is a
> **pathway wiring test**: a sessile, *obligate* decomposer (`photosynthetic_absorption = 0`) is
> seeded on a standing carcass deposit (a `carcasses` recipe capability) with no living agent
> inside its consumption reach, so `detrital_share` stays majority-detrital (≈0.9–1.0) *by geometry* — it verifies the
> producer→carcass→decomposer code path closes end to end, **not** that detritivory emerges. The
> deposit (12000 energy) is sized to genuinely outlast the run: the decomposer persists as a single
> individual to tick 2000 on detritus alone. An out-of-reach producer ring (final pop ≈90, ≈2200
> births / ≈2100 deaths) supplies the living producer→carcass front of the pathway and the deaths
> that exercise the post-reindex drain regression, but its carcasses fall near the ring, out of reach,
> so they accumulate unconsumed rather than feeding the decomposer — the brown loop closes only
> *locally*, at the deposit. (Earlier framing called this "a full living brown food web"; the telemetry
> shows the field-wide rain is unconsumed, so the closed loop is the local deposit, not the field.)
> That unconsumed rain now reads honestly: the field-level verdict is **`nutrient_lockup` (8/8)** since
> the evaluator gained a carcass-locked-nutrient signal (#342) — locally healthy wiring, nutrient
> lockup at the field scale. (example9's demographic medians are non-deterministic run-to-run, #343;
> the `nutrient_lockup` verdict is stable.)
> Whether decomposers *emerge* is answered by the genesis search (71/120 viable random
> worlds produced decomposers), not by a hand-built scenario; `example6_decomposer_viability.json`
> was retired in #328 because it demonstrated neither the viability nor the sustained carcass supply
> it claimed (its producers mass-died in a single tick and its decomposer never established a
> lineage).
> `example13_closed_web.json` (#136) is the **complement to example9**: where example9 holds its
> decomposer *out of reach* to force a pure-detrital diet (and the field locks up), example13 seeds a
> lean *facultative* decomposer guild **inside** example12's live differentiated web, on the producer
> ring within reach of the carcass rain. There the brown loop carries flux: the guild persists as a
> reproducing lineage (8/8 seeds, fitness 0.565, `trophic_balance_score` 1.0 — a producer-led pyramid;
> the figure is below the 0.686 reported pre-#394, which counted lineage clades as coexistence —
> example13's trait-cluster `coexistence_duration` is 0, as it is across the suite)
> and cuts carcass-locked nutrient ~40% vs the no-decomposer example12 baseline. It is the suite's
> first decomposer-in-a-live-web — but closure is **partial** (the dead pool is reduced, not
> eliminated): 8 sessile decomposers can't reach the whole ring's carcass-fall with
> `body_reach_coefficient=0`, so full field-scale closure is reach-limited, a *design* knob (see
> [`viability.md`](../docs/system-design/viability.md)), not a scenario tweak.
> Per-scenario verdicts in [`verdicts.md`](verdicts.md); raw numbers in
> [`observed.json`](observed.json). Two root causes shaped the legacy set:

### Root cause 1 — partial recipes drift under code defaults

The 27-May files specify only a subset of `WorldParameters`; everything else inherits
a `#[serde(default)]` value. As the model grew its nutrient / embodiment / wear /
kappa-split machinery, those defaults moved *under* the old files, so **the recorded
`parameters` block is no longer the world that runs.** The sharpest case:
`growth_efficiency` defaults to **0.0** when unset — agents build *zero* structure, so
carcasses carry no biomass energy and the decomposer pathway has nothing to eat.
`example1/2/3` all omit it. The fix pattern is `example4.json`:
**specify every parameter**. Unifying one canonical parameter/form model across genesis, scenarios, and
viability is tracked in **#295**; promoting committed functional forms into system
design is **#294**.

### Root cause 2 — roster/intent drift

`example7`/`example8` seed undifferentiated mobile consumers rather than the distinct
trophic roles their intent calls for, so they exercise no decomposer and carcasses accumulate
unconsumed. `example9_detrital_pathway.json` (#311) drives the producer→carcass→decomposer loop
end to end as a **wiring test** (the decomposer's detrital diet is forced by geometry, not emergent),
and building that pathway exposed why carcasses had always accumulated: a drain-phase index/id bug
stopped any decomposer from consuming a carcass once a death had reindexed the living population.
That is fixed; the legacy roster gap in `example7`/`example8` remains.

The files also still use retired vocabulary — `photosynthetic_absorption` (now
[`autotrophy`](../CONTEXT.md)) and `contact_radius` (now `contact_range_coefficient`) —
which survive only on serde aliases.

## Adding or repairing a scenario

1. **Specify every `WorldParameter`** (copy `example4.json`) so the file
   fully determines its world.
2. **Fill in the `metadata` declarations** — `intent`, `probes`, `prediction`, `rationale`.
3. **Regenerate `observed.json`** (`eval_scenarios`, above) — never hand-type the outcome.
4. **Re-judge** into `verdicts.md` if the outcome changed.
5. Use current vocabulary; `photosynthetic_absorption` / `contact_radius` remain only for
   backward-compatible loading.
