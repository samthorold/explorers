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

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use explorers_search::lhs;
use explorers_search::search::{decode, default_ranges};
use explorers_sim::World;

/// Mirrors `role_emergence.rs`'s sampled-config draw (keep in sync if that changes).
const SAMPLE_CONFIGS: usize = 200;
const SAMPLE_SEED: u64 = 421;
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

    let ranges = default_ranges();
    let dims = ranges.len();

    let unit: Vec<f64> = match source.as_str() {
        "atlas" => {
            let path = std::env::var("ATLAS_PATH").unwrap_or_else(|_| "atlas.json".to_string());
            let contents =
                std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
            let v: serde_json::Value =
                serde_json::from_str(&contents).unwrap_or_else(|e| panic!("parse {path}: {e}"));
            let cells = v["cells"].as_array().expect("atlas .cells array");
            let cell = cells
                .get(index)
                .unwrap_or_else(|| panic!("atlas index {index} out of range ({})", cells.len()));
            cell["unit"]
                .as_array()
                .expect("cell .unit array")
                .iter()
                .map(|x| x.as_f64().expect("unit element is a number"))
                .collect()
        }
        "sample" => {
            let mut rng = ChaCha8Rng::seed_from_u64(SAMPLE_SEED);
            let sampled = lhs::sample(dims, SAMPLE_CONFIGS, &mut rng);
            sampled
                .get(index)
                .unwrap_or_else(|| panic!("sample index {index} out of range ({SAMPLE_CONFIGS})"))
                .clone()
        }
        other => panic!("source {other:?} must be atlas|sample"),
    };

    let (params, dist) = decode(&unit, &ranges);
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
