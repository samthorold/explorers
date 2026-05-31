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
cargo run -p explorers-app -- --scenario scenarios/example4_consumer_tuning.json
```

Or run it **headless** (no window) and emit per-tick telemetry as JSON-lines to
stdout — population, births/deaths, the producer/consumer/decomposer split, the
energy budget, the nutrient pools, and a `reproduction` block summarising both
reproductive earmarks against their gates (overall and per role, so a birthless
tick reads as energy-gated, nutrient-gated, mate-limited, or dying — #280) — for
grepping or plotting (#279). Pass `--seed N` for a reproducible run; the chosen
seed is logged to stderr either way:

```
cargo run -p explorers-app -- --scenario scenarios/example4_consumer_tuning.json --trace --seed 1
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
   search lens. Regenerate (deterministic, seed 1):
   ```
   cargo run -p explorers-genesis-eval --bin eval_scenarios -- scenarios/example*.json > scenarios/observed.json
   ```
   Each row carries the `failure_mode` (against the six), the five sensible-world scores,
   `ticks_survived`, final population, and true birth/death counts.
3. **Judged verdict** — [`verdicts.md`](verdicts.md): the *read* of the evidence against the
   declarations, grounded in `expected-properties.md`. A reading (by a human or a
   fresh-perspective agent), **not** a pass/fail test — precise numbers are evidence for the
   judgment, not the gate. Drift shows up as a diff in `observed.json`, prompting a re-judge.

Agreement across the three raises confidence; **disagreement localises the fault** — and
already has: the verdict synthesis traces the suite's near-universal `energy_death` verdict to
a detector that measures predation flow rather than free-energy throughput (a genesis-evaluator
issue, surfaced by the example lens).

## Current status (seed 1)

> **The suite is stale and trophically incomplete** — `status: stale-params` on 7 of 8 files.
> Only `example4_consumer_tuning.json` reproduces (36 births); no scenario seeds a decomposer.
> Per-scenario verdicts in [`verdicts.md`](verdicts.md); raw numbers in
> [`observed.json`](observed.json). Two root causes:

### Root cause 1 — partial recipes drift under code defaults

The 27-May files specify only a subset of `WorldParameters`; everything else inherits
a `#[serde(default)]` value. As the model grew its nutrient / embodiment / wear /
kappa-split machinery, those defaults moved *under* the old files, so **the recorded
`parameters` block is no longer the world that runs.** The sharpest case:
`growth_efficiency` defaults to **0.0** when unset — agents build *zero* structure, so
carcasses carry no biomass energy and the decomposer pathway has nothing to eat.
`example1/2/3` all omit it. The fix pattern is `example4_consumer_tuning.json`:
**specify every parameter**. Unifying one canonical parameter/form model across genesis, scenarios, and
viability is tracked in **#295**; promoting committed functional forms into system
design is **#294**.

### Root cause 2 — roster/intent drift

`example7`/`example8` seed undifferentiated mobile consumers rather than the distinct
trophic roles their intent calls for. **No scenario in the suite exercises a
decomposer**, and carcasses accumulate unconsumed in every run. The decomposer-viability
scenario #136 specifies — *20 moss + 1 decomposer* — remains unbuilt.

The files also still use retired vocabulary — `photosynthetic_absorption` (now
[`autotrophy`](../CONTEXT.md)) and `contact_radius` (now `contact_range_coefficient`) —
which survive only on serde aliases.

## Adding or repairing a scenario

1. **Specify every `WorldParameter`** (copy `example4_consumer_tuning.json`) so the file
   fully determines its world.
2. **Fill in the `metadata` declarations** — `intent`, `probes`, `prediction`, `rationale`.
3. **Regenerate `observed.json`** (`eval_scenarios`, above) — never hand-type the outcome.
4. **Re-judge** into `verdicts.md` if the outcome changed.
5. Use current vocabulary; `photosynthetic_absorption` / `contact_radius` remain only for
   backward-compatible loading.
