# Delivery Workflow

How work moves from a design question to merged code in this repo. The general rules are the Commons Guides' and are not restated here:

- branches, PRs, merging and agents in parallel: the Agentic development Guide (`~/.claude/commons/guides/agentic-development.md`, *The PR workflow* and *Agents in parallel*);
- test-first, one behaviour per cycle: its Stance *agents implement test-first*;
- checkpoints and calls made without asking: its Stance *decide, record, carry on*;
- durable knowledge in committed docs, never only in agent memory: the Project knowledge Guide (`~/.claude/commons/guides/project-knowledge.md`), Stance *durable knowledge goes in source control, not private memory*;
- domain claims grounded in sources, with an evidence-quality note: its Stance *domain claims come from sources, not recall*;
- what lands from research and what of its working folders is deleted: its section *Research lands as an opinion and a signpost*. Here, domain findings go to `docs/ecology/`, measurements and world-specific analysis to `docs/research/<issue>-<slug>.md`.

## The pipeline

For an epic: `grill-with-docs` (settle the design against `CONTEXT.md`, `docs/ecology/` and `docs/system-design/`, updating the docs inline) → `to-issues` (tracer-bullet vertical slices, labelled `ready`) → `tdd` per slice, one branch and one PR each.

- The repo's gitignored `.claude/settings.local.json` allows the git/gh commands, so the flow runs without prompts.
- **The pre-commit hooks are the checks** (fmt, clippy with `-D warnings`, the workspace tests), and CI runs exactly them; see `CLAUDE.md`, *Checks and CI*.

## New physics lands behind a default-off parameter

New physics lands behind a world parameter whose default reproduces the old behaviour, so its PR leaves every world and golden fixture byte-identical, and the rule is switched on where it is read (usually a mesocosm option). When a rule becomes the world's default is decided **case by case, by its readiness**, not by a blanket policy: in a deliberate PR that flips the default, updates the fixtures with the change explained, and removes the old path, so the code and `world-rules.md` describe one physics again.

## The test loop

The Rust Guide's *Testing* sets the shape (one integration-test binary per crate, the `slow_` prefix, the workspace run before committing). Here:

The full `cargo test --workspace` runs 721 tests (716 with `--skip slow_`), and running them takes ~20 s; the rest is compiling. After a `touch` of `crates/explorers-sim/src/lib.rs`, `cargo test --workspace --no-run` takes ~80–100 s wall-clock and ~310–340 CPU-s on 8 cores, of which `explorers-sim` is ~70–100 CPU-s; `cargo test -p explorers-sim --no-run` alone takes ~25–60 s and ~40–90 CPU-s (measured for #784, warm dependencies; the `--timings` figures vary with load). `explorers-sim` builds three units: its lib, its lib's unit tests and one integration-test binary, `tests/sim/main.rs`, each at `opt-level = 3`. The sim's wall-clock is the lib followed by the test binary that links it. New sim integration tests are modules of `tests/sim/`.

- **Inner loop:** `cargo test -p <crate> -- --skip slow_` — e.g. `-p explorers-sim` is ~40 s to rebuild after a sim edit and ~15 s to run. `--lib`, or `--test sim <module>::` for one integration-test module, narrows further.
- **Before committing:** `cargo test --workspace`, `slow_` sweeps included.
- **The heaviest `slow_` tests** today are the `slow_pathway_*` tests in `explorers-sim`'s `tests/sim/headless_decomposer.rs` (~4 s for the suite). `--skip slow_` is a substring filter, so it also skips the app's `slow_down_*` tests; that is harmless.
- **The first `-p` run of a crate rebuilds its dependencies** (features resolve per package, not per workspace), so it is slow once, then incremental.
- If test binaries take minutes to *start* rather than to run, see "macOS scans every freshly linked test binary" in `known-traps.md`.
