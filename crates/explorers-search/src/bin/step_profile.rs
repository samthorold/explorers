//! Single-config step-loop profiling driver (issue #423, Tier 1).
//!
//! Runs **one** known-expensive `(config, seed)` through the genesis step loop to a
//! fixed horizon — *just* `World::step`, with none of `role_emergence`'s per-tick
//! trophic-role classification — so a sampling profiler attaches to a process that
//! is essentially pure stepper, dense with the `SpatialGrid::query_radius` hot path.
//!
//! It reconstructs the exact config `role_emergence` ran: atlas live cells from
//! `atlas.json`, and sampled configs from the same deterministic LHS draw
//! (`lhs::sample(dims, 200, ChaCha8Rng::seed(421))`). So `step_profile sample 147`
//! is bit-for-bit the config that `role_emergence` records as `sample:147`.
//!
//! ## Run
//!
//!   # build with frame symbols (no committed profile change), then profile:
//!   CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --release -p explorers-search --bin step_profile
//!   samply record ./target/release/step_profile sample 147 1000 2000
//!
//! Args: `<source: atlas|sample> <index> [seed=1000] [ticks=2000]`.
//! `samply` needs no sudo on macOS. `cargo flamegraph` / `cargo instruments -t
//! 'Time Profiler'` are fallbacks. The flamegraph is a transient local artifact —
//! do not commit it; this driver and the numeric summary are the deliverables.
//!
//! The default target `sample 147` is the #423 worst config: a large sensing radius
//! against a tiny grid cell in a small (wrapping) world — the H1 poster child.

use std::time::Instant;

use explorers_search::config_source::{SAMPLE_CONFIGS, sample_box, sample_fixed, sampled_units};
use explorers_search::search::decode;
use explorers_search::sweep::read_atlas_units;
use explorers_sim::World;

/// Mirrors `role_emergence.rs`'s sampled-config draw (keep in sync if that changes).
const SEED_BASE: u64 = 1000;
const DEFAULT_TICKS: u64 = 2000;

fn main() {
    let mut args = std::env::args().skip(1);
    let source = args.next().unwrap_or_else(|| "sample".to_string());
    let index: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(147);
    let seed: u64 = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(SEED_BASE);
    let ticks: u64 = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_TICKS);

    // An atlas cell decodes over the box its atlas records (a legacy atlas:
    // the size-blind box); a sample config over the instruments' box (#653).
    let (params, dist) = match source.as_str() {
        "atlas" => {
            let path = std::env::var("ATLAS_PATH").unwrap_or_else(|_| "atlas.json".to_string());
            let atlas = read_atlas_units(std::path::Path::new(&path));
            assert!(
                index < atlas.len(),
                "atlas index {index} out of range ({})",
                atlas.len()
            );
            atlas.decode(index)
        }
        "sample" => {
            let sampled = sampled_units();
            let unit = sampled
                .get(index)
                .unwrap_or_else(|| panic!("sample index {index} out of range ({SAMPLE_CONFIGS})"));
            decode(unit, &sample_box(), &sample_fixed())
        }
        other => panic!("source {other:?} must be atlas|sample"),
    };
    let grid_cell_size = params.light_competition_radius.max(1.0);
    eprintln!(
        "step_profile: {source}:{index} seed={seed} ticks={ticks}\n  \
         sensing_range_coefficient={:.2} light_competition_radius={:.2} (grid cell_size={:.2})\n  \
         contact_range_coefficient={:.2} world_extent={:.2} nutrient_grid_cell_size={:.2}\n  \
         worst-case radius/cell_size ratio (sensing) = {:.1}",
        params.sensing_range_coefficient,
        params.light_competition_radius,
        grid_cell_size,
        params.contact_range_coefficient,
        params.world_extent,
        params.nutrient_grid_cell_size,
        params.sensing_range_coefficient / grid_cell_size,
    );

    let mut world = World::new(params, dist, seed);
    let mut peak = 0usize;
    let start = Instant::now();
    let mut ran = 0u64;
    for _ in 0..ticks {
        world.step();
        ran += 1;
        peak = peak.max(world.agents().len());
        if world.agents().is_empty() {
            break;
        }
    }
    let elapsed = start.elapsed().as_secs_f64();
    eprintln!(
        "step_profile: ran {ran} ticks in {elapsed:.1}s ({:.2} ms/tick), peak_pop={peak} final_pop={}",
        1000.0 * elapsed / ran.max(1) as f64,
        world.agents().len(),
    );
}
