# Delivery Workflow

How work moves from a design question to merged code in this repo.

## The pipeline

For an epic: `grill-with-docs` (settle the design against `CONTEXT.md`, `docs/ecology/` and `docs/system-design/`, updating the docs inline) → `to-issues` (tracer-bullet vertical slices, labelled `ready-for-agent`) → `tdd` per slice.

- **One branch and one PR per slice.** Squash-merge with `--delete-branch`, and close the issue referencing the PR. The repo's gitignored `.claude/settings.local.json` allows the git/gh commands, so the flow runs without prompts.
- **TDD strictly red → green,** one behaviour per cycle. Surface design ambiguity the moment it appears rather than guessing.
- **Checkpoint at natural boundaries** (for example halfway through an epic) with a concise status and a default to continue, not a stop-and-ask. The owner delegates heavily and wants momentum with visibility, not permission-seeking at every step.
- **Keep new code `cargo fmt` clean** (fmt is not enforced in CI). Clippy is not enforced either; don't churn working code over style nits.

## The test loop

The full `cargo test --workspace` runs 708 tests (703 with `--skip slow_`), and running them takes ~20 s; the rest is compiling. After a `touch` of `crates/explorers-sim/src/lib.rs`, `cargo test --workspace --no-run` takes ~80–100 s wall-clock and ~310–340 CPU-s on 8 cores, of which `explorers-sim` is ~70–100 CPU-s; `cargo test -p explorers-sim --no-run` alone takes ~25–60 s and ~40–90 CPU-s (measured for #784, warm dependencies; the `--timings` figures vary with load). `explorers-sim` builds only three units now: its lib, its lib's unit tests and one integration-test binary, `tests/sim/main.rs`, each at `opt-level = 3`. The sim's wall-clock is the lib followed by the test binary that links it. Before #784 deleted the sim's five spike bins and merged its fourteen integration-test binaries into one, the same builds took ~120–140 s and ~790–820 CPU-s for the workspace and ~90–95 s and ~500–550 CPU-s for the sim (27 units). Before #783 archived `explorers-search` (tag `atlas-search-archive`), the workspace build took ~340–430 s and ~2,100–2,500 CPU-s. Keep new sim integration tests as modules of `tests/sim/`, not new files directly under `tests/`: each top-level file is another binary that re-optimises the sim. So iterate on the crate you are changing and run the workspace once before committing:

- **Inner loop:** `cargo test -p <crate> -- --skip slow_` — e.g. `-p explorers-sim` is ~40 s to rebuild after a sim edit and ~15 s to run. `--lib`, or `--test sim <module>::` for one integration-test module, narrows further.
- **Before committing:** `cargo test --workspace`, `slow_` sweeps included.
- **`slow_` is the slow-test category.** Multi-seed sweeps stay in the default run (not `#[ignore]`d) but carry a `slow_` name prefix so the inner loop can skip them; the heaviest today are the `slow_pathway_*` tests in `explorers-sim`'s `tests/sim/headless_decomposer.rs` (~4 s for the suite). `--skip slow_` is a substring filter, so it also skips the app's `slow_down_*` tests; that is harmless. Give any new slow test the prefix.
- **The first `-p` run of a crate rebuilds its dependencies** (features resolve per package, not per workspace), so it is slow once, then incremental.
- **The ecology is led by the literature, not by the agent's own reading of it.** In a grill, every domain claim is grounded in sources first, written to `docs/ecology/` with citations and an *evidence quality* note, and only then carried into the design. Mark what was read from abstracts, what was recalled rather than read, and what the sources leave open, such as a form with no magnitude. Don't offer "my read" of how nature works as a reason for a design choice. Where the literature is silent, say so, and settle the gap by a pre-registered reading across the range the domain allows.
- **New physics lands behind a world parameter whose default reproduces the old behaviour,** so its PR leaves every world and golden fixture byte-identical, and the rule is switched on where it is read (usually a mesocosm option). When a rule becomes the world's default is decided **case by case, by its readiness**, not by a blanket policy: in a deliberate PR that flips the default, updates the fixtures with the change explained, and removes the old path, so the code and `world-rules.md` describe one physics again. Each rule's *Current state* note says whether it is on by default.
- If test binaries take minutes to *start* rather than to run, see "macOS scans every freshly linked test binary" in `known-traps.md`.

## Durable knowledge lives in the repo

Anything worth remembering across sessions — workflow, runtimes, known traps, project vision — is written into committed docs (`CLAUDE.md`, `docs/agents/`, `CONTEXT.md`, `docs/system-design/`), never only in an agent's private memory. Private memory is not under source control and drifts out of date unseen.

Research working folders (for example `research_notes/` and `reports/` from deep-research runs, and scratch outputs) are temporary. Once the research is finished, fold what is durable into the right docs layer: domain findings into `docs/ecology/`, measurements and world-specific analysis into `docs/research/<issue>-<slug>.md`, and design into `docs/system-design/` via a grill. Then delete the working folders. They are never committed.
