# Delivery Workflow

How work moves from a design question to merged code in this repo.

## The pipeline

For an epic: `grill-with-docs` (settle the design against `CONTEXT.md`, `docs/ecology/` and `docs/system-design/`, updating the docs inline) → `to-issues` (tracer-bullet vertical slices, labelled `ready-for-agent`) → `tdd` per slice.

- **One branch and one PR per slice.** Squash-merge with `--delete-branch`, and close the issue referencing the PR. The repo's gitignored `.claude/settings.local.json` allows the git/gh commands, so the flow runs without prompts.
- **TDD strictly red → green,** one behaviour per cycle. Surface design ambiguity the moment it appears rather than guessing.
- **Checkpoint at natural boundaries** (for example halfway through an epic) with a concise status and a default to continue, not a stop-and-ask. The owner delegates heavily and wants momentum with visibility, not permission-seeking at every step.
- **Keep new code `cargo fmt` clean** (fmt is not enforced in CI). Clippy is not enforced either; don't churn working code over style nits.

## The test loop

The full `cargo test --workspace` runs 827 tests, but they take only ~25 s. The rest is compiling: an edit to `explorers-sim` rebuilds the 26 research bins in `explorers-search` (each ~11–13 s, and those carrying unit tests are built twice), ~100 s on 8 cores. So iterate on the crate you are changing and run the workspace once before committing:

- **Inner loop:** `cargo test -p <crate> -- --skip slow_` — e.g. `-p explorers-sim` is ~40 s to rebuild after a sim edit and ~15 s to run. `--lib` or `--test <name>` narrows further.
- **Before committing:** `cargo test --workspace`, `slow_` sweeps included.
- **`slow_` is the slow-test category.** Multi-seed sweeps stay in the default run (not `#[ignore]`d) but carry a `slow_` name prefix so the inner loop can skip them; the heaviest is `qd::tests::slow_carcass_direction_populates_the_lockup_layer_across_a_seed_sweep` (~15 s). Give any new slow test the prefix.
- **The first `-p` run of a crate rebuilds its dependencies** (features resolve per package, not per workspace), so it is slow once, then incremental.
- If test binaries take minutes to *start* rather than to run, see "macOS scans every freshly linked test binary" in `known-traps.md`.
- **Don't auto-merge when a headline acceptance is empirically unverified.** Leave the PR open and name the gap. Test-covered logic is fine to merge.

## Durable knowledge lives in the repo

Anything worth remembering across sessions — workflow, runtimes, known traps, project vision — is written into committed docs (`CLAUDE.md`, `docs/agents/`, `CONTEXT.md`, `docs/system-design/`), never only in an agent's private memory. Private memory is not under source control and drifts out of date unseen.
