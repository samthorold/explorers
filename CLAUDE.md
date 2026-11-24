## Commons Guides

@~/.claude/commons/guides/rust.md

@~/.claude/commons/guides/emergent-systems.md

## End state and register

- **Before any change, review the Due rows of `docs/system-design/register.md`** that the change could affect: read the hypothesis now, defer it with a reason in its Due cell, or revise the design. A change to the stepper (`crates/explorers-sim/src`) makes every hypothesis Due. The hypotheses, their reading rules, the refutation ladder and the one-physics rule are in `docs/system-design/end-state.md`.
- **Deviation (Project knowledge Guide, *Register*): the register is kept by hand,** not derived from tests and readings. Most hypotheses have no test written against their rule yet, and the readings that exist are long runs whose outputs are reports, not stored results a derivation could read. *Revisit when* the fast-lane hypotheses have tests named against their rules; their rows can then be derived from the suite.

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

## Checks and CI

- The toolchain is pinned in `rust-toolchain.toml`; rustup installs it on first use. Run cargo from the repo root, where rustup finds the pin.
- The checks are the pre-commit hooks in `.pre-commit-config.yaml`: file hygiene, actionlint, shellcheck, `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo test --workspace`. Install once per clone: `uv tool install pre-commit==4.7.0`, then `pre-commit install`. The pre-commit version is pinned here and in `ci.yml`; change them together.
- CI (`.github/workflows/ci.yml`) is one job, **`pre-commit`**, which runs every hook on every file. It runs on a macOS arm64 runner, not Linux: the golden tests pin exact `f32` bits, which depend on the platform's libm (Apple's and glibc's `exp`/`powf` differ in the last bits). Re-pin golden values on Apple silicon. **The required status check is `pre-commit`.** Renaming the job breaks merging under the `main` ruleset.
