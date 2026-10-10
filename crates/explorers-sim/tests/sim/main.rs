//! `explorers-sim`'s integration tests, built as one test binary (#784).
//!
//! Each module was once its own `tests/<name>.rs` binary; merging them means
//! the sim (at `opt-level = 3`) is linked into one test binary instead of
//! fourteen. Test names are unchanged, so `--skip slow_` and name filters still
//! work; narrow to one former binary with
//! `cargo test -p explorers-sim --test sim <module>::`.
//!
//! A module's `.proptest-regressions` file sits beside its source file. That is
//! not proptest's default once a `main.rs` is present, so every `proptest!`
//! block here must use `support::proptest_config(cases)`, or its stored failure
//! seeds silently stop being replayed (see `docs/agents/known-traps.md`).

mod founder_nutrient_binding;
mod founder_placement;
mod headless_decomposer;
mod headless_example4;
mod hyphal_uptake;
mod kin_blind_recognition_limit;
mod mobile_consumer_feeds;
mod prop_conservation;
mod prop_energy_bound;
mod prop_monotonicity;
mod prop_symmetry;
mod seeded_consumer_liquidity;
mod soa_differential;
mod support;
mod trait_unit_anchors;
