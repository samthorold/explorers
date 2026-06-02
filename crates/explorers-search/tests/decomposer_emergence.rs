//! Genesis emergence regression: a persistent decomposer guild reliably forms
//! under the fixed #326 search baseline (issue #337).
//!
//! This is the empirical evidence #330's design grill defers to — does a detrital
//! guild reliably *form* under genesis, on correctly-specified (not hand-built)
//! worlds? The by-construction example9 wiring test (`headless_decomposer.rs`)
//! proves the producer->carcass->decomposer code path *closes*; it is not evidence
//! of emergence. This test supplies the emergence evidence.
//!
//! ## What it does
//!
//! It drives the genesis ensemble path the real `explorers-search` binary uses —
//! the same `decode()` over `default_ranges()` that yields the #326 known-viable
//! baseline, then the same per-tick step loop as `explorers_genesis::run_single`
//! (step → early-stop on extinction/explosion). It additionally accumulates a
//! `TopologyProjection` so it can classify each surviving agent's realised diet
//! via the authoritative `topology::trophic_roles`, counting `Decomposer`s and
//! how long >= 1 decomposer persists. It does NOT shell out to the binary or run a
//! Sobol sweep (too slow — see #326); it drives `decode` + the sim directly.
//!
//! The world under test is the decoder **midpoint** (every searched dimension at
//! 0.5), which inherits `viable_baseline()` for every non-searched field. This is
//! a correctly-specified world produced by the real search decoder, NOT a scenario
//! hand-constructed to force detritivory: nothing seeds a decomposer, a carcass,
//! or a detrital deposit. Any decomposer that appears is emergent.
//!
//! ## Empirical finding (the headline for #330)
//!
//! Over a fixed ensemble of 50 seeds on the midpoint baseline (deterministic):
//!   * 48/50 seeds survive to the 500-tick horizon (the #326 viability fix holds —
//!     no extinction regime).
//!   * A decomposer **emerges from dynamics** (>= 1 agent reads as `Decomposer` at
//!     some point) in ~1/3 of surviving seeds (16/48 observed).
//!   * A **persistent** decomposer guild (>= 1 decomposer present for >= 25% of the
//!     run) forms in ~1/5 of surviving seeds (10/48 observed).
//!
//! So the guild *does* form, unseeded, on correctly-specified worlds — but at the
//! decoder midpoint it is **sporadic, not a strong majority**: detritivory is a
//! regime-sensitive emergent role, present and self-sustaining in a substantial
//! minority of draws rather than in (almost) every draw. (A wider LHS sweep over
//! the search space confirms some regimes produce a persistent guild in the
//! majority of their seeds; the midpoint is simply not the strongest such regime.)
//! This is the distributional emergence claim asserted below, deliberately framed
//! over the ensemble rather than on any single seed (the #314 ensemble precedent).
//!
//! The assertions encode that finding with margin: the test fails if the baseline
//! regresses to an extinction regime (survival collapses) OR to a decomposer-free
//! regime (no persistent guild forms). It does NOT assert a strong-majority
//! per-seed rate, because the measured distribution does not support that claim.
//!
//! ## The `slow_` marker convention
//!
//! Like the multi-seed sweeps in `explorers-sim/tests/headless_decomposer.rs`,
//! this runs a spread of full sim runs, so it carries the `slow_` name prefix to
//! join the selectable slow category (`cargo test slow_` / `--skip slow_`). It is
//! a first-class default-run test (NOT `#[ignore]`d) and is fast in practice
//! (~1.5s for all 50 seeds), but the prefix keeps the convention consistent.
//!
//! Run with:
//!   cargo test -p explorers-search --test decomposer_emergence -- --nocapture

use explorers_genesis::EvalConfig;
use explorers_search::search::{decode, default_ranges};
use explorers_sim::World;
use explorers_sim::topology::{TopologyProjection, TrophicRole};

/// Horizon for each run. 500 ticks matches the `explorers-search` default
/// (`SearchConfig::max_ticks`) and the example9 test horizon; the emergence
/// distribution is stable well before it.
const MAX_TICKS: u64 = 500;

/// Fixed seed ensemble. 50 is enough to estimate the (sporadic) emergence rate
/// stably — the survived / appeared / persistent counts move by at most a seed or
/// two between 24 and 50 seeds — while keeping the whole test ~1.5s. Seeds are a
/// fixed contiguous block so the test is fully deterministic.
const N_SEEDS: u64 = 50;
const SEED_BASE: u64 = 1000;

/// A decomposer guild is "persistent" in a run when >= 1 agent reads as a
/// `Decomposer` for at least this fraction of the ticks the run actually ran.
/// 0.25 distinguishes a sustained detrital guild from a one-tick transient (the
/// observed `frac` values cluster either near 0 / a few percent, or above 0.5 —
/// 0.25 sits cleanly in the empty gap between those modes).
const PERSISTENCE_FRACTION: f64 = 0.25;

/// One run's emergence summary.
struct RunOutcome {
    /// Survived to the horizon without extinction or population explosion.
    survived: bool,
    /// Peak count of agents reading as `Decomposer` over the run (0 => the
    /// detrital role never emerged in this run at all).
    peak_decomposers: usize,
    /// Ticks with >= 1 decomposer present.
    decomposer_ticks: u64,
    /// Ticks the run actually ran (== MAX_TICKS unless it terminated early).
    ran_ticks: u64,
}

impl RunOutcome {
    /// >= 1 decomposer was present for a meaningful fraction of the run.
    fn has_persistent_decomposer(&self) -> bool {
        self.decomposer_ticks as f64 / self.ran_ticks.max(1) as f64 >= PERSISTENCE_FRACTION
    }
}

/// Run one seed of the #326 baseline through the genesis ensemble step loop,
/// classifying trophic roles each tick via `topology::trophic_roles`. Mirrors
/// `explorers_genesis::run_single`'s loop (step, early-stop on empty / explosion)
/// but also threads a `TopologyProjection` so realised diets can be read out.
fn run_seed(unit: &[f64], seed: u64) -> RunOutcome {
    let ranges = default_ranges();
    let (params, dist) = decode(unit, &ranges);
    let eval = EvalConfig::default();
    let mut world = World::new(params, dist, seed);
    let mut topo = TopologyProjection::new();

    let mut peak_decomposers = 0usize;
    let mut decomposer_ticks = 0u64;
    let mut ran_ticks = 0u64;
    let mut survived = true;

    for _ in 0..MAX_TICKS {
        world.step();
        ran_ticks += 1;
        topo.update(world.event_log());

        let roles = topo.trophic_roles(world.agents());
        let decomposers = roles
            .values()
            .filter(|&&r| r == TrophicRole::Decomposer)
            .count();
        peak_decomposers = peak_decomposers.max(decomposers);
        if decomposers >= 1 {
            decomposer_ticks += 1;
        }

        if world.agents().is_empty() || world.agents().len() > eval.max_population {
            survived = false;
            break;
        }
    }

    RunOutcome {
        survived,
        peak_decomposers,
        decomposer_ticks,
        ran_ticks,
    }
}

/// Aggregate emergence over the fixed seed ensemble on the midpoint baseline.
#[derive(Clone, Copy)]
struct EnsembleEmergence {
    surviving: usize,
    appeared: usize,
    persistent: usize,
}

/// Memoised ensemble run: the 50-seed sweep is identical for all three property
/// tests (which run in parallel by default), so compute it once and share it
/// rather than re-running 150 sims. Mirrors `pathway_seed_result`'s memoisation in
/// `headless_decomposer.rs`.
fn midpoint_ensemble() -> EnsembleEmergence {
    use std::sync::OnceLock;
    static CACHE: OnceLock<EnsembleEmergence> = OnceLock::new();
    *CACHE.get_or_init(measure_midpoint_ensemble)
}

fn measure_midpoint_ensemble() -> EnsembleEmergence {
    let dims = default_ranges().len();
    // The #326 known-viable baseline as the search exercises it: decoder midpoint.
    let unit = vec![0.5f64; dims];

    let mut surviving = 0usize;
    let mut appeared = 0usize;
    let mut persistent = 0usize;

    for s in 0..N_SEEDS {
        let outcome = run_seed(&unit, SEED_BASE + s);
        if !outcome.survived {
            continue;
        }
        surviving += 1;
        if outcome.peak_decomposers >= 1 {
            appeared += 1;
        }
        if outcome.has_persistent_decomposer() {
            persistent += 1;
        }
    }

    EnsembleEmergence {
        surviving,
        appeared,
        persistent,
    }
}

/// The #326 baseline stays out of the extinction regime: a strong majority of the
/// fixed seed ensemble survives to the horizon. Observed: 48/50. Threshold 40/50
/// (80%) sits well below that with margin; falling under it means the baseline has
/// regressed toward the extinction regime that made the pre-#326 search yield only
/// dead worlds.
#[test]
fn slow_baseline_survives_extinction_regime() {
    let e = midpoint_ensemble();
    assert!(
        e.surviving >= 40,
        "the #326 baseline should keep a strong majority of the {N_SEEDS}-seed \
         ensemble alive to the horizon (extinction-regime guard); only {} survived",
        e.surviving
    );
}

/// A decomposer EMERGES from dynamics — unseeded — in a meaningful fraction of the
/// surviving ensemble. Observed: 16/48 surviving seeds spawn >= 1 agent that reads
/// as `Decomposer` via `trophic_roles`. Threshold: >= 6 surviving seeds. This is
/// the core emergence claim — the detrital role appears on correctly-specified
/// worlds with nothing hand-seeded — asserted distributionally over the ensemble,
/// never on a single seed.
#[test]
fn slow_decomposer_role_emerges_across_ensemble() {
    let e = midpoint_ensemble();
    assert!(
        e.appeared >= 6,
        "a decomposer should emerge (read as `Decomposer`) in a meaningful fraction \
         of the {} surviving seeds; only {} produced one (decomposer-free regression?)",
        e.surviving,
        e.appeared
    );
}

/// A PERSISTENT decomposer guild forms — >= 1 decomposer sustained for >= 25% of a
/// run — in several seeds of the ensemble, again unseeded. Observed: 10/48
/// surviving seeds. Threshold: >= 4 surviving seeds. This is the property #330's
/// grill turns on: the guild does not merely flicker, it self-sustains on the
/// emergent carcass supply across multiple independent draws. Asserted over the
/// ensemble (#314 precedent), NOT on any single regime-sensitive seed.
#[test]
fn slow_persistent_decomposer_guild_forms_across_ensemble() {
    let e = midpoint_ensemble();
    assert!(
        e.persistent >= 4,
        "a persistent decomposer guild (>= 1 decomposer for >= {PERSISTENCE_FRACTION} of \
         the run) should form in several of the {} surviving seeds; only {} did. The \
         guild has regressed to flicker-only or absent.",
        e.surviving,
        e.persistent
    );
}
