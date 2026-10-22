## Agent skills

### Issues

GitHub Issues for this repo, through `gh`, with the states `ready` and `needs-design` (no label = backlog), as in the Global Guide's *Issues* section.

### Domain docs

Single-context layout — three layers: CONTEXT.md (ubiquitous language) + docs/ecology/ (domain) + docs/system-design/ (the design, which is self-justifying — no separate decision-record layer). See `docs/agents/domain.md`.

### Workspace architecture

How the crates divide responsibility (sim = pure deterministic stepper; genesis/search own parameter search; app = debugging instrument). Read before deciding where a change belongs. See `docs/agents/architecture.md`.

### Delivery workflow

grill-with-docs → to-issues → one TDD slice per PR, squash-merged. Iterate with `cargo test -p <crate> -- --skip slow_`; run the workspace before committing. Durable knowledge goes in committed docs, never only in agent memory. See `docs/agents/delivery-workflow.md`.

### Project vision

The game the simulation serves (confusion → wonder → symbiosis). See `docs/agents/project-vision.md`.

### Instrument runtimes

Wall-clock for the reference-mode runs (the archived search bins' figures live at the tag `atlas-search-archive`), and how to drive long sweeps in-harness. See `docs/agents/instrument-runtimes.md`.

### Known traps

Diagnoses that have cost time before (e.g. translation proptest failures). See `docs/agents/known-traps.md`.

## Code style

The committed code is rustfmt-clean (`cargo fmt --check` passes on HEAD). Run `cargo fmt` freely and keep it clean; on a clean tree it touches only what you changed, so a focused diff stays focused. fmt is not enforced in CI, so it's on you to run it before committing.
