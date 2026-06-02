# Workspace Architecture

How the crates divide responsibility. Read this before deciding *where* a change belongs.

## Crates

- **`explorers-sim`** — the simulation. A pure, deterministic, history-free stepper: given a world state and parameters (`WorldParameters`, `InitialDistribution`), it advances one tick. It holds no rendering, no observation history, and no search logic. Determinism per seed is the load-bearing invariant — everything above it relies on being able to replay a run exactly.
- **`explorers-genesis-eval`** — evaluation. Scores a finished or in-flight run against the [expected properties](../system-design/expected-properties.md): `FitnessBreakdown` (oscillation, clustering, coexistence, turnover, trophic balance) and the `FailureMode` taxonomy (extinction, population explosion, energy death, monoculture, generalist dominance). This is where "is this world healthy?" is answered.
- **`explorers-genesis`** — orchestration. Thin layer that runs a config through `explorers-sim` over a seed ensemble and reduces it to a fitness via `explorers-genesis-eval` (`RunConfig`/`EnsembleConfig` → `EnsembleResult`). Distribution-as-evidence: a parameterisation is judged on an ensemble of replicate seeds, not a single run.
- **`explorers-search`** — the parameter-space search itself (Bayesian optimisation, Gaussian process, Latin-hypercube and Sobol sampling). Owns the broad, expensive sweep over world parameters and initial trait distributions, choosing which configs genesis should evaluate next.
- **`explorers-app`** — the interactive instrument (eframe/egui, glow backend). Its job is to *understand and debug the simulation*: watch the grid, plot distributions and ledgers, poke hypotheses with live parameter sliders, and explain why a given configuration behaves as it does. Observation history (ring buffers sampled after each step) lives here, never in the sim.

## Division of labour

Thorough parameter search belongs to the genesis/search crates, and they do it well over seed ensembles — so the app deliberately does **not** grow search or optimisation UI, and it is **not** a playtest harness (the emotional-arc playtest experience is a separate, deferred concern). The app's only job right now is *understanding the system*: mechanism debugging (is the physics correct?), explaining genesis verdicts (why is config X degenerate?), and quick hypothesis poking.

When extending the app, bias toward instrumentation (plots, distributions, ledgers, cluster tracks) over game-feel or search features. Keep all measurement and history in the app; never let rendering or observation concerns leak into `explorers-sim` — the sim stays a clean deterministic stepper.
