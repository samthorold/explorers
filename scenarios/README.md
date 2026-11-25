# Scenarios

Hand-authored `WorldRecipe` files: small, legible worlds (a named roster of agents at fixed positions) with every parameter written down. A scenario is here for one of two reasons, and its outcome is never recorded beside it:

- **A test's fixture.** A test loads it and asserts on what it does, exactly where it can be.
- **A hypothesis's setup.** The [end state](../docs/system-design/end-state.md) names it as the setup a hypothesis is read on. The reading rule is declared there, a test implements it, and the status is in the [register](../docs/system-design/register.md).

A file that is neither is deleted. An outcome kept by hand beside a scenario (a judged verdict, a snapshot of observed scores) goes stale with every change to the physics, and nothing says when: a test fails when the world stops doing what it asserts, and a register row is Due when what it was read on changes.

## The files

| File | What it is | What reads it |
|---|---|---|
| `example4.json` | A small mixed world: about twenty producers and two mobile consumers, reproducing sexually and asexually. The template for a fully specified parameter block. | Byte-identity tests (golden trajectories, the SoA differential, creation and kin-blind fingerprints) and tests that borrow its parameter block, in `crates/explorers-sim`. |
| `example9_detrital_pathway.json` | A sessile, obligate decomposer seeded on a carcass deposit, with a producer ring outside its feeding reach (`body_reach_coefficient = 0`, so the geometry is exact). The decomposer founds its own founding cluster, unrelated to the producers, so the producers' broadcast gametes cannot reach it as mates. A wiring test of part 1, the physics: the path from producer to carcass to decomposer, not a phenomenon. | `example9_wires_producer_to_carcass_to_decomposer_exactly` (fast lane, exact) and the determinism guards in `headless_decomposer.rs`. |
| `example12_generalist_dominance.json` | Specialist producers, specialist mobile consumers, sessile mixotrophs and mobile mixotrophs on one producer ring, with light and prey in reach of all. | The setup of hypothesis Sp, under trophic structure in the end state; its test is `crates/explorers-genesis-eval/tests/specialists_hold_living_energy.rs`. |

## File format

A scenario is a `WorldRecipe` (`crates/explorers-sim/src/lib.rs`): a `parameters` block (`WorldParameters`), an optional `agents` list (fixed roster; omit for a random seeded population), optional `carcasses`, and `max_ticks`. A random seeded population is described by an `initial_distribution` (`InitialDistribution`). Its `founder_aggregation` (`0` well-mixed to `1` tight, separate patches, one per founding cluster) defaults to `0.8` when omitted. Loaded via `--scenario PATH` / `--recipe PATH` in `explorers-app`, or `serde_json::from_str` in tests.

Each file also carries a top-level `metadata` object: the author's notes on what the file is for, its roster and its geometry. The loader ignores it (`WorldRecipe` doesn't declare it and serde drops unknown keys), so it never changes how the scenario runs.

Open one in the GUI:

```
cargo run -p explorers-app -- --scenario scenarios/example4.json
```

Or run it **headless** (no window) and emit per-tick telemetry as JSON-lines to stdout: population, births and deaths, the producer/consumer/decomposer split, the energy budget, the nutrient pools, and a `reproduction` block summarising both reproductive earmarks against their gates (overall and per role, so a birthless tick reads as energy-gated, nutrient-gated, mate-limited, or dying). Pass `--seed N` for a reproducible run; the chosen seed is logged to stderr either way:

```
cargo run -p explorers-app -- --scenario scenarios/example4.json --trace --seed 1
```

## Adding a scenario

1. **Add it with what reads it**: the test that loads it, or the hypothesis in the end state that names it as its setup, with that hypothesis's rule, test and register row.
2. **Name each founder's founding cluster** (`founding_cluster`, default 0) when the roster stands for unrelated clades, a plant cohort and its decomposers: founders of one cluster are interfertile ([world rules](../docs/system-design/world-rules.md), flow 4).
3. **Specify every `WorldParameter`** (copy `example4.json`), so the file fully determines its world. A parameter left out takes its code default, and defaults move under old files.
4. **Use current vocabulary.** `photosynthetic_absorption` (now [`autotrophy`](../CONTEXT.md)) and `contact_radius` (now `contact_range_coefficient`) load only through serde aliases.
