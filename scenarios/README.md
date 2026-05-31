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

Run one headless and watch it:

```
cargo run -p explorers-app -- --scenario scenarios/example4_consumer_tuning.json
```

## Metadata header

Every file carries a top-level `metadata` object documenting what it is *for*
(per issue #293). The loader ignores it — `WorldRecipe` does not declare these fields
and serde drops unknown keys — so it is descriptive only, not consumed by the sim.
Promoting it into the struct so the harness can read predictions and assert outcomes
is tracked in **#293**.

| field | meaning |
|---|---|
| `intent` | what mechanism/pattern the scenario is meant to exercise |
| `source_issue` | the GitHub issue that defined it (intent used to live only here) |
| `roster` | the agents it seeds, by trophic role |
| `probes` | the [failure mode](../docs/system-design/expected-properties.md) it stresses (extinction / energy death / monoculture / population explosion / frozen dynamics / generalist dominance) |
| `viability_prediction` | a priori call — `live` / `dead` / `undecided`, and why |
| `expected_outcome` | what a *passing* run looks like |
| `observed` | what the headless harness actually produced (dated, with seed) |
| `status` | health of the file itself — `current` or `stale-params` |

## Current status (2026-05-31, seed 1)

> **The suite is trophically stale.** Only `example4_consumer_tuning.json` reproduces;
> every other scenario ends with **zero consumers and zero births**, and no scenario
> exercises a decomposer. See the two root causes below.

| file | intent | roster | observed | status |
|---|---|---|---|---|
| `example1.json` | one moss: photosynthesis, dispersal, patch formation | 1 P | seed-only (`max_ticks` 0) | stale-params |
| `example2.json` | twenty moss: light competition, self-thinning | 20 P | 20→0, 0 births, 0 structure | stale-params |
| `example3.json` | three moss far apart: isolation, drift | 3 P | 3→0, 0 births | stale-params |
| `example4.json` | moss + 1 consumer: grazing, top-down control | 20 P + 1 C | 21→4, 0 births | stale-params |
| `example4_consumer_tuning.json` | fully-specified diagnostic variant of ex4 | 20 P + 2 C | 22→7, **36 births** | **current** |
| `example5.json` | moss + 2 consumers: predator-prey oscillation | 20 P + 2 C | 22→3, 0 births | stale-params |
| `example7.json` | three trophic roles, dual recycling | 20 P + 3 C | 23→4, 0 births | stale-params |
| `example8.json` | full cascade + population oscillation | 20 P + 4 C | 24→3, 0 births | stale-params |

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

1. **Specify every `WorldParameter`** (copy `example4_consumer_tuning.json` as the
   template) so the file fully determines its world.
2. **Fill in the `metadata` header** — at minimum `intent`, `probes`,
   `viability_prediction`, and `expected_outcome`.
3. **Run it headless, record `observed`** (date + seed) honestly, even when failing.
4. Use current vocabulary in new work; `photosynthetic_absorption` / `contact_radius`
   remain only for backward-compatible loading.

Related: **#279** (headless scenario runner emitting per-tick telemetry) would turn the
ad-hoc sweep behind the `observed` rows into a checked-in regression harness.
