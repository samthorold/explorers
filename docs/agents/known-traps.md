# Known Traps

Diagnoses that have cost time before. Check these first.

## Translation and symmetry proptest failures are usually hard-threshold straddles

`SpatialGrid::query_radius` once returned the same id twice when its cell window wrapped a small toroidal grid (#412). That is fixed: it now scans the whole grid exactly once when the window covers it, and `move_agents` also dedupes its neighbours. Don't blame it.

The usual real cause is a pair sitting exactly on a hard distance threshold (a sensing, feeding or light radius), where f32 wrap rounding flips the `<=` under translation — the accepted hard-predicate discontinuity (#551; `docs/system-design/world-rules.md`, "Hard distance predicates are an accepted discontinuity in position"). #564 was this. The fix is a test guard within `TRANSLATION_BOUNDARY_TOLERANCE`. For a translation or symmetry failure, first print the pair's distances against every radius threshold.
