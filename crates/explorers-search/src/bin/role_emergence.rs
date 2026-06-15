//! Trophic-role emergence timing across regimes, past the search horizon (issue #421).
//!
//! ## What it measures
//!
//! Trophic roles (producer / consumer / decomposer) are *emergent positions in
//! trait space*, not assigned types — `topology::trophic_roles` reads each agent's
//! realised role per tick from the green (predation) vs brown (decomposition) food
//! web. Our only prior evidence about *when* those roles emerge was a single regime
//! at one horizon (`decomposer_emergence.rs`: at 500 ticks a decomposer appears in
//! ~1/3 of surviving midpoint seeds). That leaves a real ambiguity: when a regime
//! shows no decomposer at the 500-tick search horizon, is it **monostable** (the
//! trait cloud never branches) or merely **slow** (it would branch at, say, tick
//! 800, but the search horizon truncates it)?
//!
//! This instrument runs many configs to an **extended 2000-tick horizon** (4× the
//! `SearchConfig::max_ticks` search horizon), records the tick at which each
//! trophic-role milestone is first reached per run, and emits a data artifact plus
//! a stdout summary. It answers: how long does it take to reach a
//! producer+consumer+decomposer system, and how many "monoculture" regimes are
//! actually just *late* rather than *never*?
//!
//! ## What it does NOT claim
//!
//! This is a **measurement tool, not a CI gate** and not a stepper/evaluator
//! change. It does not touch the stepper, the evaluator objective, `SearchConfig`,
//! or the search. The decomposer guild stays a *reported observable*, never an
//! objective or a binning axis. It makes no analytic time-to-branch prediction —
//! it only times what the committed dynamics actually do. It reuses
//! `topology::trophic_roles` verbatim for classification (no reimplementation) and
//! drives `decode` + the genesis step loop directly, exactly as
//! `decomposer_emergence.rs::run_seed` does (it does NOT shell out to the search
//! binary).
//!
//! ## The non-eater→Consumer caveat
//!
//! `trophic_roles` defaults a *non-eating* heterotroph to `Consumer` (it has no
//! detrital reliance to read). So `t_first_consumer` over-counts: it fires for a
//! heterotroph that has eaten nothing. To distinguish a genuine predator we also
//! record `t_first_consumer_realised` — the first tick a `Consumer`-classified
//! agent has actually drained *living* biomass (≥1 predation event, read from the
//! event log's `Consumed` events with `target_was_carcass == false`).
//! `t_first_all_three` uses the *realised* consumer, so the three-role milestone
//! means a real producer, a real predator, and a real decomposer coexist.
//!
//! ## Classification sampling
//!
//! `trophic_roles` is O(agents × accumulated edges) and the edge set grows over a
//! run, so classifying every tick of a high-population 2000-tick survivor is
//! prohibitive. Roles are therefore read every `CLASSIFY_INTERVAL` ticks (plus the
//! terminal tick) — mirroring the evaluator's own `coexistence_sample_interval = 10`,
//! which coarsens its (also expensive) DBSCAN classification for the same reason.
//! Cheap per-tick facts (carcass presence, population, predation events) are still
//! read every tick. The cost: each `t_*` role milestone is resolved to the nearest
//! sampled tick (±`CLASSIFY_INTERVAL`).
//!
//! ## Determinism
//!
//! Fully deterministic: the sampled configs come from a fixed-seed LHS draw
//! (`lhs::sample` over a `ChaCha8Rng`), seeds are a fixed contiguous block, and the
//! per-(config, seed) runs are independent, so the rayon parallel collect is
//! order-stable (same output every run, mirroring `run_ensemble`'s #350 contract).
//!
//! ## Output
//!
//! Writes per-run milestone records + the summary to `target/role-emergence.json`
//! (and a flat `target/role-emergence.csv`). `target/` is gitignored — the artifact
//! is not committed. The summary is also printed to stdout.
//!
//! Run with:
//!   cargo run --release -p explorers-search --bin role_emergence
//! (optional first arg: path to the atlas JSON; default `atlas.json`)
//!
//! This is a heavy, long-running diagnostic — tens of minutes, and an observed
//! worst case of ~80 min. The cost is not the classification but the sim *step*:
//! a handful of configs sustain a near-`max_population` world all the way to the
//! 2000-tick horizon, and stepping those (spatial neighbour queries scale with
//! population) dominates the tail. The bulk of runs finish in the first few
//! minutes; per-run progress is logged to stderr so the straggler tail is visible
//! rather than looking like a hang. Run it `--release`.

use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;

use explorers_genesis::EvalConfig;
use explorers_search::lhs;
use explorers_search::search::{decode, default_ranges};
use explorers_sim::World;
use explorers_sim::event::EventKind;
use explorers_sim::topology::{TopologyProjection, TrophicRole};

/// The extended horizon — 4× the `SearchConfig::max_ticks = 500` search horizon, so
/// a late emerger (tick 800) is distinguished from a monostable regime that never
/// branches at all.
const EXTENDED_HORIZON: u64 = 2000;

/// The current search horizon. A config whose three-role system first appears in
/// `(SEARCH_HORIZON, EXTENDED_HORIZON]` would be *truncated* — mislabelled as
/// monoculture — by the live search. Counting those is the headline finding.
const SEARCH_HORIZON: u64 = 500;

/// Fixed contiguous seed block per config (the `decomposer_emergence.rs`
/// determinism convention). 8 seeds is the issue's floor — enough to see whether a
/// regime branches in *any* draw and how sporadic it is, without a 2000-tick run
/// per seed becoming prohibitive.
const N_SEEDS: u64 = 8;
const SEED_BASE: u64 = 1000;

/// Number of low-discrepancy unit-cube configs sampled in addition to the atlas
/// live cells. These span the never-branching (monoculture / extinction) regimes
/// the atlas's count-only dead frontier cannot replay.
const SAMPLE_CONFIGS: usize = 200;

/// Fixed seed for the deterministic LHS draw of the sampled configs.
const SAMPLE_SEED: u64 = 421;

/// A decomposer guild is "persistent" from a tick when ≥1 decomposer is present for
/// at least this fraction of the *remaining* run. Reuses the 0.25 convention from
/// `decomposer_emergence.rs` (the empty gap between the transient and sustained
/// modes of the observed presence fraction).
const PERSISTENCE_FRACTION: f64 = 0.25;

/// Tick interval at which roles are classified via `topology::trophic_roles`. The
/// classification is O(agents × accumulated edges) and the edge set grows over the
/// run, so per-tick classification on a high-population 2000-tick survivor is
/// prohibitive. We sample it instead — directly mirroring the evaluator's
/// `coexistence_sample_interval = 10` (genesis-eval), which exists for exactly this
/// reason. The cheap per-tick facts (carcass presence, population, predation) are
/// still read every tick; only the role readout is coarsened. Consequence: every
/// `t_*` role milestone is resolved to the nearest sampled tick (±`CLASSIFY_INTERVAL`),
/// and `t_persistent_guild`'s presence fraction is computed over the sampled series.
const CLASSIFY_INTERVAL: u64 = 10;

/// Where the configs were drawn from — the atlas live cells vs the sampled cube.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum ConfigSource {
    Atlas,
    Sample,
}

/// One (config × seed) run's milestone record. Every `t_*` is the tick of first
/// occurrence, or `null` if it never happened within the horizon.
#[derive(Clone, Debug, serde::Serialize)]
struct RunRecord {
    source: ConfigSource,
    /// Index within that source's config list (stable, reproducible).
    config_index: usize,
    seed: u64,
    /// Ticks actually run (== `EXTENDED_HORIZON` unless terminated early).
    ran_ticks: u64,
    /// Survived to the horizon without extinction or population explosion.
    survived: bool,
    /// Minimal, documented terminal classification (NOT the full evaluator):
    /// `extinction` (population empty), `explosion` (> `max_population`), or
    /// `survived` (reached the horizon).
    terminal_mode: &'static str,
    /// Peak count of agents reading as `Decomposer` over the run.
    peak_decomposers: usize,
    /// Final-tick role composition (0 each if extinct).
    final_producers: usize,
    final_consumers: usize,
    final_decomposers: usize,
    /// First tick a carcass / dead-pool entry exists.
    t_first_carcass: Option<u64>,
    /// First tick ≥1 agent classifies as `Decomposer`.
    t_first_decomposer: Option<u64>,
    /// First tick ≥1 agent classifies as `Consumer`. CAVEAT: a non-eating
    /// heterotroph defaults to `Consumer`, so this over-counts; see
    /// `t_first_consumer_realised`.
    t_first_consumer: Option<u64>,
    /// First tick a `Consumer`-classified agent has actually drained living biomass
    /// (≥1 predation event) — a genuine predator, not a default non-eater.
    t_first_consumer_realised: Option<u64>,
    /// First tick `Producer`, a *realised* `Consumer`, and `Decomposer` are all
    /// present simultaneously.
    t_first_all_three: Option<u64>,
    /// First tick from which ≥1 decomposer is present for ≥`PERSISTENCE_FRACTION` of
    /// the remaining run; `null` if the guild never reaches that persistence.
    t_persistent_guild: Option<u64>,
}

/// Drive one (config, seed) through the genesis step loop to the extended horizon,
/// classifying trophic roles each tick via `topology::trophic_roles`. Mirrors
/// `decomposer_emergence.rs::run_seed` / `explorers_genesis::run_single`'s loop
/// (step, early-stop on empty / explosion), threading a `TopologyProjection` so
/// realised diets can be read out, plus a predator set (agents that have drained
/// living biomass) read straight from the event log.
fn run(source: ConfigSource, config_index: usize, unit: &[f64], seed: u64) -> RunRecord {
    let ranges = default_ranges();
    let (params, dist) = decode(unit, &ranges);
    let max_pop = EvalConfig::default().max_population;
    let mut world = World::new(params, dist, seed);
    let mut topo = TopologyProjection::new();

    // Agents that have ever drained *living* biomass (a predation `Consumed` event,
    // `target_was_carcass == false`). Read incrementally from the event log; this
    // is the "has eaten living biomass" signal the realised-consumer milestone needs.
    let mut predators: HashSet<u64> = HashSet::new();
    let mut event_cursor = 0usize;

    let mut ran_ticks = 0u64;
    let mut survived = true;
    let mut terminal_mode = "survived";
    let mut peak_decomposers = 0usize;
    // Sampled decomposer-presence series and the ticks the samples were taken at
    // (roles are classified only every `CLASSIFY_INTERVAL` ticks; see module docs).
    let mut decomposer_present: Vec<bool> = Vec::new();
    let mut sample_ticks: Vec<u64> = Vec::new();

    let mut t_first_carcass = None;
    let mut t_first_decomposer = None;
    let mut t_first_consumer = None;
    let mut t_first_consumer_realised = None;
    let mut t_first_all_three = None;

    let mut final_producers = 0usize;
    let mut final_consumers = 0usize;
    let mut final_decomposers = 0usize;

    for _ in 0..EXTENDED_HORIZON {
        world.step();
        ran_ticks += 1;
        let tick = ran_ticks;
        topo.update(world.event_log());

        // Absorb new predation facts from the log tail (cheap, per-tick).
        let log = world.event_log();
        for ev in log.since(event_cursor) {
            if ev.kind == EventKind::Consumed && !ev.target_was_carcass {
                predators.insert(ev.source);
            }
        }
        event_cursor = log.len();

        // Cheap per-tick fact: carcass / dead-pool presence.
        if t_first_carcass.is_none() && !world.carcasses().is_empty() {
            t_first_carcass = Some(tick);
        }

        // Early-stop conditions (cheap, read every tick — mirrors run_single).
        let empty = world.agents().is_empty();
        let explosion = world.agents().len() > max_pop;
        let terminal = empty || explosion;

        // Classify roles on the sampling cadence, and always on the terminal tick so
        // the final composition reflects the run's end.
        if tick % CLASSIFY_INTERVAL == 0 || terminal {
            // Authoritative role classification — reused verbatim.
            let roles = topo.trophic_roles(world.agents());
            let mut producers = 0usize;
            let mut consumers = 0usize;
            let mut decomposers = 0usize;
            let mut realised_consumer_present = false;
            for (id, role) in &roles {
                match role {
                    TrophicRole::Producer => producers += 1,
                    TrophicRole::Consumer => {
                        consumers += 1;
                        if predators.contains(id) {
                            realised_consumer_present = true;
                        }
                    }
                    TrophicRole::Decomposer => decomposers += 1,
                }
            }

            if t_first_decomposer.is_none() && decomposers >= 1 {
                t_first_decomposer = Some(tick);
            }
            if t_first_consumer.is_none() && consumers >= 1 {
                t_first_consumer = Some(tick);
            }
            if t_first_consumer_realised.is_none() && realised_consumer_present {
                t_first_consumer_realised = Some(tick);
            }
            if t_first_all_three.is_none()
                && producers >= 1
                && realised_consumer_present
                && decomposers >= 1
            {
                t_first_all_three = Some(tick);
            }

            peak_decomposers = peak_decomposers.max(decomposers);
            decomposer_present.push(decomposers >= 1);
            sample_ticks.push(tick);
            final_producers = producers;
            final_consumers = consumers;
            final_decomposers = decomposers;
        }

        if empty {
            survived = false;
            terminal_mode = "extinction";
            break;
        }
        if explosion {
            survived = false;
            terminal_mode = "explosion";
            break;
        }
    }

    // Map the persistence onset (an index into the sampled series) back to its tick.
    let t_persistent_guild =
        first_persistent_index(&decomposer_present, PERSISTENCE_FRACTION).map(|i| sample_ticks[i]);

    RunRecord {
        source,
        config_index,
        seed,
        ran_ticks,
        survived,
        terminal_mode,
        peak_decomposers,
        final_producers,
        final_consumers,
        final_decomposers,
        t_first_carcass,
        t_first_decomposer,
        t_first_consumer,
        t_first_consumer_realised,
        t_first_all_three,
        t_persistent_guild,
    }
}

/// Index of the first sample at which the guild is present *and* from which ≥1
/// decomposer is present for at least `frac` of the *remaining* (sampled) run — the
/// onset of a sustained guild. O(n) via a suffix count of present samples; `None` if
/// no sample qualifies. The caller maps the index back to its tick.
fn first_persistent_index(present: &[bool], frac: f64) -> Option<usize> {
    let n = present.len();
    if n == 0 {
        return None;
    }
    // suffix[i] = number of present samples in present[i..].
    let mut suffix = vec![0usize; n + 1];
    for i in (0..n).rev() {
        suffix[i] = suffix[i + 1] + usize::from(present[i]);
    }
    for i in 0..n {
        // The onset sample must itself be present — otherwise a full back half would
        // spuriously qualify index 0 (a "guild" that does not yet exist there).
        let remaining = n - i;
        if present[i] && suffix[i] as f64 / remaining as f64 >= frac {
            return Some(i);
        }
    }
    None
}

/// Linear-interpolated percentile of a sorted slice (`q` in [0,1]). Empty → `None`.
fn percentile(sorted: &[u64], q: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    if sorted.len() == 1 {
        return Some(sorted[0] as f64);
    }
    let rank = q * (sorted.len() - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    let frac = rank - lo as f64;
    Some(sorted[lo] as f64 + frac * (sorted[hi] as f64 - sorted[lo] as f64))
}

fn quartiles(values: &[u64]) -> (Option<f64>, Option<f64>, Option<f64>) {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    (
        percentile(&sorted, 0.25),
        percentile(&sorted, 0.50),
        percentile(&sorted, 0.75),
    )
}

/// Minimal config-list deserialisation: we only need each live cell's `unit` vector
/// (the atlas type is `Serialize`-only, and `unit` is all we replay).
#[derive(serde::Deserialize)]
struct AtlasFile {
    cells: Vec<AtlasCellUnit>,
}

#[derive(serde::Deserialize)]
struct AtlasCellUnit {
    unit: Vec<f64>,
}

/// Per-config aggregation, keyed by (source, index). `config_first_all_three` is the
/// earliest tick across the config's seeds at which all three roles appear — the
/// regime's demonstrated capability. `seeds_reaching` exposes the per-seed
/// sporadicity behind that single number.
#[derive(serde::Serialize)]
struct ConfigSummary {
    source: ConfigSource,
    config_index: usize,
    seeds_total: usize,
    seeds_surviving: usize,
    seeds_reaching_all_three: usize,
    config_first_all_three: Option<u64>,
    config_first_persistent_guild: Option<u64>,
}

#[derive(serde::Serialize)]
struct Summary {
    horizon: u64,
    search_horizon: u64,
    n_seeds: u64,
    persistence_fraction: f64,
    atlas_configs: usize,
    sampled_configs: usize,
    total_configs: usize,
    total_runs: usize,
    // Run-level (each run is one config × seed).
    runs_reaching_all_three: usize,
    runs_reaching_all_three_fraction: f64,
    all_three_p25: Option<f64>,
    all_three_median: Option<f64>,
    all_three_p75: Option<f64>,
    persistent_guild_p25: Option<f64>,
    persistent_guild_median: Option<f64>,
    persistent_guild_p75: Option<f64>,
    // Config-level (a config "reaches" if any of its seeds does).
    configs_reaching_all_three: usize,
    configs_reaching_all_three_fraction: f64,
    /// Headline: configs first reaching three roles in (SEARCH_HORIZON, HORIZON] —
    /// late emergers the 500-tick search horizon would mislabel as monoculture.
    configs_truncated_by_search_horizon: usize,
    /// Configs that never reach three roles within the extended horizon in any
    /// seed — candidate monostable (genuinely never-branching) regimes.
    configs_never_reaching_all_three: usize,
}

#[derive(serde::Serialize)]
struct Artifact {
    summary: Summary,
    configs: Vec<ConfigSummary>,
    runs: Vec<RunRecord>,
}

fn main() {
    let atlas_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "atlas.json".to_string());
    let ranges = default_ranges();
    let dims = ranges.len();

    // Source 1: the atlas live-cell unit vectors (the known coexisting regimes).
    let atlas_units: Vec<Vec<f64>> = {
        let contents = std::fs::read_to_string(&atlas_path)
            .unwrap_or_else(|e| panic!("read {atlas_path}: {e}"));
        let atlas: AtlasFile =
            serde_json::from_str(&contents).unwrap_or_else(|e| panic!("parse {atlas_path}: {e}"));
        atlas.cells.into_iter().map(|c| c.unit).collect()
    };

    // Source 2: a deterministic low-discrepancy (LHS) sample of the unit cube,
    // decoded via the same `decode` over `default_ranges`. Naturally includes
    // monoculture / extinction regimes the atlas dead frontier cannot replay.
    let sampled_units: Vec<Vec<f64>> = {
        let mut rng = ChaCha8Rng::seed_from_u64(SAMPLE_SEED);
        lhs::sample(dims, SAMPLE_CONFIGS, &mut rng)
    };

    eprintln!(
        "role_emergence: {} atlas + {} sampled configs × {} seeds, horizon {} ticks",
        atlas_units.len(),
        sampled_units.len(),
        N_SEEDS,
        EXTENDED_HORIZON
    );

    // Build the flat task list: every (config, seed) is an independent run.
    let mut tasks: Vec<(ConfigSource, usize, Vec<f64>, u64)> = Vec::new();
    for (i, unit) in atlas_units.iter().enumerate() {
        for s in 0..N_SEEDS {
            tasks.push((ConfigSource::Atlas, i, unit.clone(), SEED_BASE + s));
        }
    }
    for (i, unit) in sampled_units.iter().enumerate() {
        for s in 0..N_SEEDS {
            tasks.push((ConfigSource::Sample, i, unit.clone(), SEED_BASE + s));
        }
    }

    // Independent runs → order-stable parallel collect (the #350 determinism
    // contract): same output every invocation. A completion counter logs progress
    // so the long tail (a few large-population configs running to the 2000-tick
    // horizon) is visible rather than looking like a hang. The counter races to
    // near-total quickly, then stalls on the stragglers — which is exactly the
    // signal: e.g. "2040/2048" sitting still means 8 expensive runs remain.
    let total_runs = tasks.len();
    let done = AtomicUsize::new(0);
    let start = Instant::now();
    let log_step = (total_runs / 40).max(1);
    let runs: Vec<RunRecord> = tasks
        .par_iter()
        .map(|(source, idx, unit, seed)| {
            let record = run(*source, *idx, unit, *seed);
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            if n % log_step == 0 || n == total_runs {
                eprintln!(
                    "  progress: {n}/{total_runs} runs done ({:.0}s elapsed)",
                    start.elapsed().as_secs_f64()
                );
            }
            record
        })
        .collect();
    eprintln!(
        "role_emergence: all {total_runs} runs complete in {:.0}s",
        start.elapsed().as_secs_f64()
    );

    // --- Per-config aggregation ---
    let total_configs = atlas_units.len() + sampled_units.len();
    let mut configs: Vec<ConfigSummary> = Vec::with_capacity(total_configs);
    for (source, count) in [
        (ConfigSource::Atlas, atlas_units.len()),
        (ConfigSource::Sample, sampled_units.len()),
    ] {
        for idx in 0..count {
            let cfg_runs: Vec<&RunRecord> = runs
                .iter()
                .filter(|r| r.source == source && r.config_index == idx)
                .collect();
            let seeds_surviving = cfg_runs.iter().filter(|r| r.survived).count();
            let seeds_reaching = cfg_runs
                .iter()
                .filter(|r| r.t_first_all_three.is_some())
                .count();
            let config_first_all_three = cfg_runs.iter().filter_map(|r| r.t_first_all_three).min();
            let config_first_persistent_guild =
                cfg_runs.iter().filter_map(|r| r.t_persistent_guild).min();
            configs.push(ConfigSummary {
                source,
                config_index: idx,
                seeds_total: cfg_runs.len(),
                seeds_surviving,
                seeds_reaching_all_three: seeds_reaching,
                config_first_all_three,
                config_first_persistent_guild,
            });
        }
    }

    // --- Run-level distribution ---
    let all_three_ticks: Vec<u64> = runs.iter().filter_map(|r| r.t_first_all_three).collect();
    let persistent_ticks: Vec<u64> = runs.iter().filter_map(|r| r.t_persistent_guild).collect();
    let (a_p25, a_med, a_p75) = quartiles(&all_three_ticks);
    let (g_p25, g_med, g_p75) = quartiles(&persistent_ticks);

    // --- Config-level headline counts ---
    let configs_reaching = configs
        .iter()
        .filter(|c| c.config_first_all_three.is_some())
        .count();
    let configs_truncated = configs
        .iter()
        .filter(|c| {
            c.config_first_all_three
                .is_some_and(|t| t > SEARCH_HORIZON && t <= EXTENDED_HORIZON)
        })
        .count();
    let configs_never = configs
        .iter()
        .filter(|c| c.config_first_all_three.is_none())
        .count();

    let summary = Summary {
        horizon: EXTENDED_HORIZON,
        search_horizon: SEARCH_HORIZON,
        n_seeds: N_SEEDS,
        persistence_fraction: PERSISTENCE_FRACTION,
        atlas_configs: atlas_units.len(),
        sampled_configs: sampled_units.len(),
        total_configs,
        total_runs: runs.len(),
        runs_reaching_all_three: all_three_ticks.len(),
        runs_reaching_all_three_fraction: all_three_ticks.len() as f64 / runs.len().max(1) as f64,
        all_three_p25: a_p25,
        all_three_median: a_med,
        all_three_p75: a_p75,
        persistent_guild_p25: g_p25,
        persistent_guild_median: g_med,
        persistent_guild_p75: g_p75,
        configs_reaching_all_three: configs_reaching,
        configs_reaching_all_three_fraction: configs_reaching as f64 / total_configs.max(1) as f64,
        configs_truncated_by_search_horizon: configs_truncated,
        configs_never_reaching_all_three: configs_never,
    };

    print_summary(&summary);
    write_artifacts(&Artifact {
        summary,
        configs,
        runs,
    });
}

fn fmt_opt(v: Option<f64>) -> String {
    v.map_or_else(|| "—".to_string(), |x| format!("{x:.0}"))
}

fn print_summary(s: &Summary) {
    println!("\n# Trophic-role emergence timing (issue #421)");
    println!(
        "# extended horizon {} ticks ({}× the {}-tick search horizon)",
        s.horizon,
        s.horizon / s.search_horizon,
        s.search_horizon
    );
    println!(
        "# {} configs ({} atlas live cells + {} sampled), {} seeds each = {} runs\n",
        s.total_configs, s.atlas_configs, s.sampled_configs, s.n_seeds, s.total_runs
    );

    println!("## Reaching all three roles (producer + realised consumer + decomposer)");
    println!(
        "  runs reaching all three:    {} / {} ({:.1}%)",
        s.runs_reaching_all_three,
        s.total_runs,
        100.0 * s.runs_reaching_all_three_fraction
    );
    println!(
        "  configs reaching all three: {} / {} ({:.1}%)",
        s.configs_reaching_all_three,
        s.total_configs,
        100.0 * s.configs_reaching_all_three_fraction
    );
    println!();

    println!("## t_first_all_three among reaching runs (ticks)");
    println!(
        "  p25 = {}   median = {}   p75 = {}",
        fmt_opt(s.all_three_p25),
        fmt_opt(s.all_three_median),
        fmt_opt(s.all_three_p75)
    );
    println!("## t_persistent_guild among runs that form one (ticks)");
    println!(
        "  p25 = {}   median = {}   p75 = {}",
        fmt_opt(s.persistent_guild_p25),
        fmt_opt(s.persistent_guild_median),
        fmt_opt(s.persistent_guild_p75)
    );
    println!();

    println!("## Headline — search-horizon truncation");
    println!(
        "  configs first reaching three roles in ({}, {}]: {}  (would be mislabelled monoculture by the {}-tick search)",
        s.search_horizon, s.horizon, s.configs_truncated_by_search_horizon, s.search_horizon
    );
    println!(
        "  configs never reaching three roles within {}:   {}  (candidate monostable regimes)",
        s.horizon, s.configs_never_reaching_all_three
    );
    println!();
}

fn write_artifacts(artifact: &Artifact) {
    std::fs::create_dir_all("target").ok();

    let json_path = "target/role-emergence.json";
    let json = serde_json::to_string_pretty(artifact).expect("serialise artifact");
    std::fs::write(json_path, json).unwrap_or_else(|e| panic!("write {json_path}: {e}"));

    let csv_path = "target/role-emergence.csv";
    let mut csv = String::new();
    csv.push_str(
        "source,config_index,seed,ran_ticks,survived,terminal_mode,peak_decomposers,\
         final_producers,final_consumers,final_decomposers,\
         t_first_carcass,t_first_decomposer,t_first_consumer,t_first_consumer_realised,\
         t_first_all_three,t_persistent_guild\n",
    );
    let cell = |v: Option<u64>| v.map_or_else(String::new, |x| x.to_string());
    for r in &artifact.runs {
        let source = match r.source {
            ConfigSource::Atlas => "atlas",
            ConfigSource::Sample => "sample",
        };
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            source,
            r.config_index,
            r.seed,
            r.ran_ticks,
            r.survived,
            r.terminal_mode,
            r.peak_decomposers,
            r.final_producers,
            r.final_consumers,
            r.final_decomposers,
            cell(r.t_first_carcass),
            cell(r.t_first_decomposer),
            cell(r.t_first_consumer),
            cell(r.t_first_consumer_realised),
            cell(r.t_first_all_three),
            cell(r.t_persistent_guild),
        ));
    }
    std::fs::write(csv_path, csv).unwrap_or_else(|e| panic!("write {csv_path}: {e}"));

    eprintln!(
        "role_emergence: wrote {json_path} and {csv_path} ({} runs)",
        artifact.runs.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Coarse smoke check: the instrument runs a config and produces a record with
    /// the milestone fields populated (no brittle assertion on emergent *timings*).
    #[test]
    fn run_produces_a_record() {
        let ranges = default_ranges();
        let unit = vec![0.5f64; ranges.len()];
        let rec = run(ConfigSource::Sample, 0, &unit, SEED_BASE);
        assert!(rec.ran_ticks > 0, "the run must advance at least one tick");
        assert!(
            rec.ran_ticks <= EXTENDED_HORIZON,
            "the run must not exceed the horizon"
        );
        // Ordering invariants that must hold regardless of regime: the three-role
        // milestone cannot precede its constituent role milestones.
        if let Some(all3) = rec.t_first_all_three {
            assert!(rec.t_first_decomposer.unwrap() <= all3);
            assert!(rec.t_first_consumer_realised.unwrap() <= all3);
        }
        // A realised consumer is a strictly stronger event than a classified one.
        if let (Some(realised), Some(any)) = (rec.t_first_consumer_realised, rec.t_first_consumer) {
            assert!(any <= realised);
        }
    }

    #[test]
    fn persistent_index_reads_first_qualifying_sample() {
        // Present for the whole back half from index 2: the remaining run is
        // all-present → fraction 1.0 ≥ 0.25, and index 2 is itself present.
        let present = vec![false, false, true, true, true, true];
        assert_eq!(first_persistent_index(&present, 0.25), Some(2));
        // Never present → never persistent.
        assert_eq!(first_persistent_index(&[false, false, false], 0.25), None);
        // A single flicker at the start over a long absence never qualifies: from
        // index 0 the fraction is 1/5 = 0.2 < 0.25, and every later window is empty.
        let flicker = vec![true, false, false, false, false];
        assert_eq!(first_persistent_index(&flicker, 0.25), None);
        // A lone present sample at the very end trivially qualifies there (1/1 ≥ 0.25).
        let late = vec![false, false, false, false, true];
        assert_eq!(first_persistent_index(&late, 0.25), Some(4));
    }

    #[test]
    fn percentile_interpolates() {
        let sorted = [10u64, 20, 30, 40];
        assert_eq!(percentile(&sorted, 0.0), Some(10.0));
        assert_eq!(percentile(&sorted, 0.5), Some(25.0));
        assert_eq!(percentile(&sorted, 1.0), Some(40.0));
        assert_eq!(percentile(&[], 0.5), None);
    }
}
