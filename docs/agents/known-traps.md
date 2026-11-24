# Known Traps

Diagnoses specific to explorers that have cost time before. Check these first. Traps any project would hit are in the Commons Guides (`~/.claude/commons/MAP.md`), each in its Guide's *Known traps*.

## Translation and symmetry proptest failures are usually hard-threshold straddles

`SpatialGrid::query_radius` once returned the same id twice when its cell window wrapped a small toroidal grid (#412). That is fixed: it now scans the whole grid exactly once when the window covers it, and `move_agents` also dedupes its neighbours. Don't blame it.

The usual real cause is a pair sitting exactly on a hard distance threshold (a sensing, feeding or light radius), where f32 wrap rounding flips the `<=` under translation — the accepted hard-predicate discontinuity (#551; `docs/system-design/world-rules.md`, "Hard distance predicates are an accepted discontinuity in position"). #564 was this. The fix is a test guard within `TRANSLATION_BOUNDARY_TOLERANCE`. For a translation or symmetry failure, first print the pair's distances against every radius threshold.

## proptest blocks in the sim's test binary take `support::proptest_config`

The Rust Guide's *Testing* pins proptest's failure persistence through one shared helper. In `crates/explorers-sim/tests/sim/` that helper is `support::proptest_config(cases)`.

## macOS scans every freshly linked test binary

On macOS, `syspolicyd` (Gatekeeper/XProtect) scans each newly linked executable on its first launch. Every rebuild relinks the test binaries, so every test run after an edit pays the scan: once measured at ~85 s across the workspace, and 400 s for `-p explorers-sim` alone, against ~15 s of actual testing. The tell is a binary reporting `finished in 0.07s` after tens of seconds of wall-clock, and `syspolicyd` high in `ps -Ao pcpu,comm -r`.

The fix is to add the terminal app to System Settings → Privacy & Security → Developer Tools. It applies only to processes started afterwards, so restart the terminal and any Claude Code session running in it.

## A carcass `Consumed` event's `energy_delta` is drain, not income

On a carcass drain, `Consumed.energy_delta` is the structure drained from the carcass, before trophic efficiency. The consumer's income is drain × efficiency. Reading the event as income overstated founder income about 3× during the #764 diagnosis (`docs/research/764-founder-death-diagnosis.md`).

## fish: a quoted variable passes as one argument

In fish, `env "$v" cmd` with `v="A=1 B=2"` passes a single variable named `A` with value `1 B=2`, because fish does not word-split. To pass several environment variables from one string, write a `sh` script that uses an unquoted `$v`.
