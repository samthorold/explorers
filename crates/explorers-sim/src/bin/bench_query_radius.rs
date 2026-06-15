//! Deterministic microbenchmark for `SpatialGrid::query_radius`, the genesis
//! step-loop hot path (issue #423, Tier 2).
//!
//! Run with optimisations (the only meaningful setting):
//!   cargo run --release -p explorers-sim --bin bench_query_radius
//!
//! ## What it isolates
//!
//! `query_radius` cost is roughly `O((radius / cell_size)^2  +  candidates_scanned)`,
//! with a `HashMap` probe per candidate and a heap allocation per call. This bench
//! sweeps **population N × (radius / cell_size) ratio** and reports ns/call so the
//! scaling curve can be read off directly, and it pins which term dominates by
//! comparing three implementations on the *same* deterministic agent layout:
//!
//!   - `baseline`  — the real `explorers_sim::spatial::SpatialGrid::query_radius`
//!                   (side `HashMap<id,pos>` probe per candidate + a fresh `Vec`
//!                   allocation per call). This is exactly what the stepper runs.
//!   - `inline`    — positions stored *inline* in the cell buckets
//!                   (`Vec<(u64,(f32,f32))>`), no side `HashMap`. Isolates the
//!                   per-candidate `HashMap` probe (issue #423 **H2**).
//!   - `inline+scratch` — inline positions *and* a reused scratch buffer instead of
//!                   a fresh `Vec` per call. On top of `inline`, isolates the
//!                   per-call allocation (issue #423 **H3**).
//!
//! The `inline` variants are faithful replicas of the real cell/toroidal-wrap logic
//! (copied from `spatial.rs`), differing *only* in the dimension under test — so the
//! baseline-vs-inline and inline-vs-scratch deltas attribute cost to H2 and H3
//! respectively. The bench changes nothing in the sim crate; the variants live here.
//!
//! ## Two world regimes (issue #423 H1)
//!
//! Each (N, ratio) cell is run at two `world_extent`s with the same `cell_size = 1`:
//!   - **wide** world (`cols ≫ cells_to_check`): a pure cell-scan, no toroidal
//!     wrap — the clean `(radius/cell_size)^2` curve.
//!   - **small** world (`cols < cells_to_check` at large ratio): the neighbourhood
//!     scan wraps and **revisits cells**, the known toroidal double-return. The
//!     `dup/call` column quantifies the wasted re-scan.

use std::hint::black_box;
use std::time::Instant;

use rand::SeedableRng;
use rand::distr::{Distribution, Uniform};
use rand_chacha::ChaCha8Rng;

use explorers_sim::spatial::SpatialGrid;
use explorers_sim::toroidal_distance;

/// Deterministic agent positions, uniform over the toroidal world. Seeded per
/// (N, extent) so a row is reproducible and the three variants see identical data.
fn positions(n: usize, extent: f32, seed: u64) -> Vec<(f32, f32)> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let dist = Uniform::new(-extent / 2.0, extent / 2.0).unwrap();
    (0..n)
        .map(|_| (dist.sample(&mut rng), dist.sample(&mut rng)))
        .collect()
}

/// Inline-position grid: positions live in the cell buckets, no side `HashMap`.
/// Cell/wrap logic copied verbatim from `spatial.rs` so timing is apples-to-apples.
struct InlineGrid {
    world_extent: f32,
    cell_size: f32,
    cols: usize,
    cells: Vec<Vec<(u64, (f32, f32))>>,
}

impl InlineGrid {
    fn new(world_extent: f32, cell_size: f32) -> Self {
        let cols = (world_extent / cell_size).ceil() as usize;
        Self {
            world_extent,
            cell_size,
            cols,
            cells: vec![Vec::new(); cols * cols],
        }
    }

    fn insert(&mut self, id: u64, position: (f32, f32)) {
        let idx = self.cell_index(position);
        self.cells[idx].push((id, position));
    }

    fn cell_coords(&self, position: (f32, f32)) -> (usize, usize) {
        let half = self.world_extent / 2.0;
        let col = (((position.0 + half) / self.cell_size) as usize).min(self.cols - 1);
        let row = (((position.1 + half) / self.cell_size) as usize).min(self.cols - 1);
        (col, row)
    }

    fn cell_index(&self, position: (f32, f32)) -> usize {
        let (col, row) = self.cell_coords(position);
        row * self.cols + col
    }

    /// Fresh-allocation variant (isolates H2 vs baseline).
    fn query_radius(&self, center: (f32, f32), radius: f32) -> Vec<u64> {
        let mut out = Vec::new();
        self.query_into(center, radius, &mut out);
        out
    }

    /// Scratch-buffer variant (isolates H3 vs `query_radius`): caller owns `out`.
    fn query_into(&self, center: (f32, f32), radius: f32, out: &mut Vec<u64>) {
        out.clear();
        let cells_to_check = (radius / self.cell_size).ceil() as isize + 1;
        let (center_col, center_row) = self.cell_coords(center);
        for dr in -cells_to_check..=cells_to_check {
            for dc in -cells_to_check..=cells_to_check {
                let row = (center_row as isize + dr).rem_euclid(self.cols as isize) as usize;
                let col = (center_col as isize + dc).rem_euclid(self.cols as isize) as usize;
                for &(id, pos) in &self.cells[row * self.cols + col] {
                    if toroidal_distance(center, pos, self.world_extent) <= radius {
                        out.push(id);
                    }
                }
            }
        }
    }
}

struct Row {
    n: usize,
    ratio: u32,
    cols: usize,
    cells_to_check: i64,
    /// Mean candidates returned and mean duplicate (double-returned) ids per call.
    mean_returned: f64,
    mean_dups: f64,
    base_ns: f64,
    inline_ns: f64,
    scratch_ns: f64,
}

/// Number of timed queries to issue per row: one per agent (mirroring the stepper,
/// which queries from every agent's position), repeated to a stable sample size.
fn reps_for(n: usize) -> usize {
    (200_000 / n.max(1)).max(4)
}

fn bench_row(n: usize, ratio: u32, extent: f32) -> Row {
    let cell_size = 1.0_f32;
    let radius = ratio as f32 * cell_size;
    let pos = positions(n, extent, 0xC0FFEE ^ (n as u64) ^ ((extent as u64) << 20));

    // Build all three grids on the identical layout.
    let mut base = SpatialGrid::new(extent, cell_size);
    let mut inl = InlineGrid::new(extent, cell_size);
    for (i, &p) in pos.iter().enumerate() {
        base.insert(i as u64, p);
        inl.insert(i as u64, p);
    }
    let cols = (extent / cell_size).ceil() as usize;
    let cells_to_check = (radius / cell_size).ceil() as i64 + 1;

    // --- Untimed stats pass: returned count and double-returns per call. ---
    let mut tot_returned = 0u64;
    let mut tot_dups = 0u64;
    for &c in &pos {
        let r = base.query_radius(c, radius);
        let returned = r.len() as u64;
        let unique = {
            let mut v = r.clone();
            v.sort_unstable();
            v.dedup();
            v.len() as u64
        };
        tot_returned += returned;
        tot_dups += returned - unique;
    }
    let mean_returned = tot_returned as f64 / n as f64;
    let mean_dups = tot_dups as f64 / n as f64;

    let reps = reps_for(n);

    // --- baseline: real SpatialGrid (HashMap probe + fresh alloc). ---
    let t = Instant::now();
    for _ in 0..reps {
        for &c in &pos {
            black_box(base.query_radius(black_box(c), black_box(radius)));
        }
    }
    let base_ns = t.elapsed().as_secs_f64() * 1e9 / (reps * n) as f64;

    // --- inline: positions in buckets, fresh alloc (isolates H2). ---
    let t = Instant::now();
    for _ in 0..reps {
        for &c in &pos {
            black_box(inl.query_radius(black_box(c), black_box(radius)));
        }
    }
    let inline_ns = t.elapsed().as_secs_f64() * 1e9 / (reps * n) as f64;

    // --- inline + scratch: reused buffer (isolates H3 on top of H2). ---
    let mut scratch = Vec::new();
    let t = Instant::now();
    for _ in 0..reps {
        for &c in &pos {
            inl.query_into(black_box(c), black_box(radius), &mut scratch);
            black_box(scratch.len());
        }
    }
    let scratch_ns = t.elapsed().as_secs_f64() * 1e9 / (reps * n) as f64;

    Row {
        n,
        ratio,
        cols,
        cells_to_check,
        mean_returned,
        mean_dups,
        base_ns,
        inline_ns,
        scratch_ns,
    }
}

fn main() {
    println!("# Issue #423 Tier 2 — query_radius microbenchmark\n");
    #[cfg(debug_assertions)]
    println!("WARNING: running in debug. Use `cargo run --release` for meaningful numbers.\n");

    println!("cell_size = 1.0 for every row, so radius == ratio. ns are per query call.");
    println!("base = real SpatialGrid (HashMap probe + fresh Vec alloc)");
    println!("inline = positions in cell buckets, fresh alloc (H2: no HashMap probe)");
    println!("scratch = inline + reused buffer (H3: no per-call alloc)\n");

    let populations = [64usize, 256, 1024, 4096];
    let ratios = [1u32, 2, 4, 8, 16, 30];
    // wide: cols=200 ≫ cells_to_check (no wrap). small: cols=20 < cells_to_check at
    // large ratio (toroidal wrap → double-returns).
    let extents = [("wide", 200.0_f32), ("small", 20.0_f32)];

    println!(
        "{:<6} {:>6} {:>5} {:>5} {:>6} {:>9} {:>8} {:>9} {:>9} {:>9} {:>8} {:>8}",
        "world",
        "N",
        "ratio",
        "cols",
        "scan",
        "ret/call",
        "dup/call",
        "base_ns",
        "inline_ns",
        "scrat_ns",
        "H2_save",
        "H3_save"
    );
    for (label, extent) in extents {
        for &n in &populations {
            for &ratio in &ratios {
                let r = bench_row(n, ratio, extent);
                // scan = cells visited per call = (2*cells_to_check+1)^2.
                let scan = (2 * r.cells_to_check + 1).pow(2);
                let h2_save = 100.0 * (r.base_ns - r.inline_ns) / r.base_ns;
                let h3_save = 100.0 * (r.inline_ns - r.scratch_ns) / r.base_ns;
                println!(
                    "{:<6} {:>6} {:>5} {:>5} {:>6} {:>9.1} {:>8.2} {:>9.1} {:>9.1} {:>9.1} {:>7.0}% {:>7.0}%",
                    label,
                    r.n,
                    r.ratio,
                    r.cols,
                    scan,
                    r.mean_returned,
                    r.mean_dups,
                    r.base_ns,
                    r.inline_ns,
                    r.scratch_ns,
                    h2_save,
                    h3_save,
                );
            }
        }
        println!();
    }
    println!(
        "H2_save = (base-inline)/base: share of per-call time removed by dropping the HashMap probe."
    );
    println!(
        "H3_save = (inline-scratch)/base: additional share removed by reusing the result buffer."
    );
    println!(
        "scan = cells visited per call = (2*cells_to_check+1)^2; dup/call = double-returned ids."
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// The H2/H3 attribution is only valid if the `inline` variants return the same
    /// neighbours as the real grid. Coarse differential check over a few layouts:
    /// the *sets* must match (both may contain toroidal duplicates, hence the set).
    #[test]
    fn inline_grid_matches_spatial_grid() {
        for &(extent, cell, n, radius) in &[
            (100.0_f32, 10.0_f32, 50usize, 12.0_f32),
            (20.0, 1.0, 80, 25.0), // wrapping regime
            (75.0, 2.5, 120, 18.0),
        ] {
            let pos = positions(n, extent, 7);
            let mut base = SpatialGrid::new(extent, cell);
            let mut inl = InlineGrid::new(extent, cell);
            for (i, &p) in pos.iter().enumerate() {
                base.insert(i as u64, p);
                inl.insert(i as u64, p);
            }
            for &c in &pos {
                let b: HashSet<u64> = base.query_radius(c, radius).into_iter().collect();
                let i: HashSet<u64> = inl.query_radius(c, radius).into_iter().collect();
                assert_eq!(b, i, "extent={extent} cell={cell} radius={radius}");
            }
        }
    }
}
