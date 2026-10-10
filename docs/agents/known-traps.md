# Known Traps

Diagnoses that have cost time before. Check these first.

## Translation and symmetry proptest failures are usually hard-threshold straddles

`SpatialGrid::query_radius` once returned the same id twice when its cell window wrapped a small toroidal grid (#412). That is fixed: it now scans the whole grid exactly once when the window covers it, and `move_agents` also dedupes its neighbours. Don't blame it.

The usual real cause is a pair sitting exactly on a hard distance threshold (a sensing, feeding or light radius), where f32 wrap rounding flips the `<=` under translation — the accepted hard-predicate discontinuity (#551; `docs/system-design/world-rules.md`, "Hard distance predicates are an accepted discontinuity in position"). #564 was this. The fix is a test guard within `TRANSLATION_BOUNDARY_TOLERANCE`. For a translation or symmetry failure, first print the pair's distances against every radius threshold.

## macOS scans every freshly linked test binary

On macOS, `syspolicyd` (Gatekeeper/XProtect) scans each newly linked executable on its first launch. Every rebuild relinks the test binaries, so every test run after an edit pays the scan: once measured at ~85 s across the workspace, and 400 s for `-p explorers-sim` alone, against ~15 s of actual testing. The tell is a binary reporting `finished in 0.07s` after tens of seconds of wall-clock, and `syspolicyd` high in `ps -Ao pcpu,comm -r`.

The fix is to add the terminal app to System Settings → Privacy & Security → Developer Tools. It applies only to processes started afterwards, so restart the terminal and any Claude Code session running in it.

## `reinvasion_barrier`'s accounted artifacts are not byte-identical across runs

In `--accounting` and `--dispersal` modes, the per-member `account` floats in the JSON differ in the last ulp between two runs of the same command. `LineageAccountant` sums over `HashMap`s, whose hasher is seeded randomly per process. Trajectories, counts and every printed table are identical (#619). To check that a change leaves a run unchanged, diff the `.md` summaries, or compare the JSON with the `account` fields stripped. Plain mode is byte-identical.

## proptest seeds stop replaying when a suite moves under a `main.rs`

proptest's default failure persistence (`SourceParallel`) walks up from a test's source file to the nearest directory holding a `lib.rs` or `main.rs`, and stores failures in a sibling `proptest-regressions/<path>.txt`. Only when it finds none does it fall back to `<file>.proptest-regressions` beside the source. The sim's property suites now live in the one test binary `crates/explorers-sim/tests/sim/`, whose `main.rs` sits beside them, so the default would read and write `tests/proptest-regressions/<module>.txt` and silently ignore the committed `<module>.proptest-regressions` seeds (#784). Every suite there therefore takes its config from `support::proptest_config(cases)`, which pins `WithSource("proptest-regressions")`. Use it for any new `proptest!` block in that binary. A stray `tests/proptest-regressions/` directory means a block bypassed it.

## A carcass `Consumed` event's `energy_delta` is drain, not income

On a carcass drain, `Consumed.energy_delta` is the structure drained from the carcass, before trophic efficiency. The consumer's income is drain × efficiency. Reading the event as income overstated founder income about 3× during the #764 diagnosis (`docs/research/764-founder-death-diagnosis.md`).

## GitHub closing keywords anywhere in a PR body close the issue

GitHub closes `#N` on merge when `close`, `closes`, `fix`, `fixes`, `resolve` or `resolves` sits directly before it, even inside a quoted table cell. #771's body quoted a reading table's "Close #764", and the merge closed #764; it had to be reopened. Write "#764 closes" or "the row closing #764" unless you mean it.

## Delegated agents can outlive their hand-back

A sub-agent can report done while work it started is still running. A grinder handed back while its #773 sub-agent was still live; the child then committed, opened PR #776, and wrote its PR body to the same scratchpad path the orchestrator then overwrote. #781's sub-agent handed back with `reference_mode` runs still going in the background. Before touching a branch after a delegated agent hands back, check `gh pr list --head <branch>`, check `ps` for `cargo` and `reference_mode`, wait for every child's completion notification, and confirm the branch head is stable. Give each delegated PR body file a unique name, and don't let grinders nest agents.

## The auto-mode classifier can block commands after `gh pr merge`

After a merge, the classifier has flagged follow-up commands as "Merge Without Review". When it does, stop and ask the owner; don't work around it.

## fish: a quoted variable passes as one argument

In fish, `env "$v" cmd` with `v="A=1 B=2"` passes a single variable named `A` with value `1 B=2`, because fish does not word-split. To pass several environment variables from one string, write a `sh` script that uses an unquoted `$v`.
