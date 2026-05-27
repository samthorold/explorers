use std::collections::HashMap;

use crate::toroidal_distance;

#[derive(Clone)]
pub struct SpatialGrid {
    world_extent: f32,
    cell_size: f32,
    cols: usize,
    cells: Vec<Vec<u64>>,
    positions: HashMap<u64, (f32, f32)>,
}

impl SpatialGrid {
    pub fn new(world_extent: f32, cell_size: f32) -> Self {
        let cols = (world_extent / cell_size).ceil() as usize;
        Self {
            world_extent,
            cell_size,
            cols,
            cells: vec![Vec::new(); cols * cols],
            positions: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: u64, position: (f32, f32)) {
        let cell_idx = self.cell_index(position);
        self.cells[cell_idx].push(id);
        self.positions.insert(id, position);
    }

    pub fn update_position(&mut self, id: u64, new_position: (f32, f32)) {
        self.remove(id);
        self.insert(id, new_position);
    }

    pub fn remove(&mut self, id: u64) {
        if let Some(pos) = self.positions.remove(&id) {
            let cell_idx = self.cell_index(pos);
            self.cells[cell_idx].retain(|&stored| stored != id);
        }
    }

    pub fn query_radius(&self, center: (f32, f32), radius: f32) -> Vec<u64> {
        let mut results = Vec::new();
        let cells_to_check = (radius / self.cell_size).ceil() as isize + 1;
        let (center_col, center_row) = self.cell_coords(center);

        for dr in -cells_to_check..=cells_to_check {
            for dc in -cells_to_check..=cells_to_check {
                let row = (center_row as isize + dr).rem_euclid(self.cols as isize) as usize;
                let col = (center_col as isize + dc).rem_euclid(self.cols as isize) as usize;
                let cell_idx = row * self.cols + col;
                for &id in &self.cells[cell_idx] {
                    let pos = self.positions[&id];
                    if toroidal_distance(center, pos, self.world_extent) <= radius {
                        results.push(id);
                    }
                }
            }
        }
        results
    }

    fn cell_coords(&self, position: (f32, f32)) -> (usize, usize) {
        let half = self.world_extent / 2.0;
        let x = position.0 + half;
        let y = position.1 + half;
        let col = ((x / self.cell_size) as usize).min(self.cols - 1);
        let row = ((y / self.cell_size) as usize).min(self.cols - 1);
        (col, row)
    }

    fn cell_index(&self, position: (f32, f32)) -> usize {
        let (col, row) = self.cell_coords(position);
        row * self.cols + col
    }
}

/// Spatially heterogeneous nutrient pool. Wraps a `Vec<f32>` grid where each
/// cell holds the available nutrient at that location. Co-located agents share
/// their cell's pool proportionally.
#[derive(Clone)]
pub struct NutrientGrid {
    extent: f32,
    cell_size: f32,
    cols: usize,
    cells: Vec<f32>,
}

impl NutrientGrid {
    /// Create a new nutrient grid with `initial_total` distributed uniformly.
    pub fn new(extent: f32, cell_size: f32, initial_total: f32) -> Self {
        let cols = (extent / cell_size).ceil() as usize;
        let n = cols * cols;
        let per_cell = if n > 0 { initial_total / n as f32 } else { 0.0 };
        Self {
            extent,
            cell_size,
            cols,
            cells: vec![per_cell; n],
        }
    }

    /// Mutable reference to the nutrient value at a world position.
    pub fn at_position(&mut self, pos: (f32, f32)) -> &mut f32 {
        let idx = self.cell_index(pos);
        &mut self.cells[idx]
    }

    /// Total nutrient across all cells.
    pub fn total(&self) -> f32 {
        self.cells.iter().sum()
    }

    /// Returns the cell index for a given position (public for phase functions).
    pub fn cell_index_for(&self, pos: (f32, f32)) -> usize {
        self.cell_index(pos)
    }

    /// Mutable reference to a cell by index (public for phase functions).
    pub fn cell_mut(&mut self, idx: usize) -> &mut f32 {
        &mut self.cells[idx]
    }

    fn cell_index(&self, pos: (f32, f32)) -> usize {
        let half = self.extent / 2.0;
        let x = pos.0 + half;
        let y = pos.1 + half;
        let col = ((x / self.cell_size) as usize).min(self.cols - 1);
        let row = ((y / self.cell_size) as usize).min(self.cols - 1);
        row * self.cols + col
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserted_agent_is_returned_by_query_at_same_position() {
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(1, (5.0, 5.0));
        let results = grid.query_radius((5.0, 5.0), 1.0);
        assert_eq!(results, vec![1]);
    }

    #[test]
    fn query_matches_brute_force_for_random_configurations() {
        use rand::SeedableRng;
        use rand::distr::{Distribution, Uniform};
        use rand_chacha::ChaCha8Rng;

        let world_extent = 100.0;
        let cell_size = 15.0;
        let n_agents = 200;
        let n_queries = 50;

        for seed in 0..10 {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let pos_dist = Uniform::new(-world_extent / 2.0, world_extent / 2.0).unwrap();

            let mut grid = SpatialGrid::new(world_extent, cell_size);
            let mut agents: Vec<(u64, (f32, f32))> = Vec::new();

            for id in 0..n_agents {
                let pos = (pos_dist.sample(&mut rng), pos_dist.sample(&mut rng));
                grid.insert(id, pos);
                agents.push((id, pos));
            }

            for _ in 0..n_queries {
                let center = (pos_dist.sample(&mut rng), pos_dist.sample(&mut rng));
                let radius = Uniform::new(0.0_f32, 30.0).unwrap().sample(&mut rng);

                let grid_results: std::collections::HashSet<u64> =
                    grid.query_radius(center, radius).into_iter().collect();

                let brute_results: std::collections::HashSet<u64> = agents
                    .iter()
                    .filter(|(_, pos)| {
                        crate::toroidal_distance(center, *pos, world_extent) <= radius
                    })
                    .map(|(id, _)| *id)
                    .collect();

                assert_eq!(
                    grid_results, brute_results,
                    "seed={seed}, center={center:?}, radius={radius}"
                );
            }
        }
    }

    #[test]
    fn empty_grid_returns_no_results() {
        let grid = SpatialGrid::new(100.0, 10.0);
        let results = grid.query_radius((0.0, 0.0), 50.0);
        assert!(results.is_empty());
    }

    #[test]
    fn zero_radius_query_returns_only_agents_at_exact_position() {
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(1, (5.0, 5.0));
        grid.insert(2, (5.0, 5.0));
        grid.insert(3, (5.1, 5.0));
        let results = grid.query_radius((5.0, 5.0), 0.0);
        assert!(results.contains(&1));
        assert!(results.contains(&2));
        assert!(!results.contains(&3));
    }

    #[test]
    fn single_agent_world() {
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(42, (0.0, 0.0));
        let results = grid.query_radius((0.0, 0.0), 10.0);
        assert_eq!(results, vec![42]);
    }

    #[test]
    fn agent_on_cell_boundary_is_found() {
        let mut grid = SpatialGrid::new(100.0, 10.0);
        // Place agent exactly on a cell boundary
        let boundary_x = -50.0 + 10.0; // = -40.0, boundary between cell 0 and cell 1
        grid.insert(1, (boundary_x, 0.0));
        let results = grid.query_radius((boundary_x, 0.0), 1.0);
        assert_eq!(results, vec![1]);
    }

    #[test]
    fn updated_agent_appears_at_new_position() {
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(1, (0.0, 0.0));
        grid.update_position(1, (40.0, 40.0));
        let at_old = grid.query_radius((0.0, 0.0), 5.0);
        let at_new = grid.query_radius((40.0, 40.0), 5.0);
        assert!(!at_old.contains(&1));
        assert!(at_new.contains(&1));
    }

    #[test]
    fn removed_agent_is_not_returned_by_query() {
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(1, (0.0, 0.0));
        grid.insert(2, (1.0, 0.0));
        grid.remove(1);
        let results = grid.query_radius((0.0, 0.0), 5.0);
        assert!(!results.contains(&1));
        assert!(results.contains(&2));
    }

    #[test]
    fn query_returns_deterministic_order_by_cell_then_insertion() {
        let mut grid = SpatialGrid::new(100.0, 10.0);
        // Place agents in different cells, insert in non-cell order
        // Cell for (15,15) has higher index than cell for (-15,-15)
        grid.insert(10, (15.0, 15.0));
        grid.insert(20, (-15.0, -15.0));
        grid.insert(30, (-15.0, -14.0)); // same cell as agent 20
        let results = grid.query_radius((0.0, 0.0), 30.0);
        // Agent 20 and 30 are in a lower-index cell than agent 10
        // Within their cell, 20 was inserted before 30
        let pos_20 = results.iter().position(|&id| id == 20).unwrap();
        let pos_30 = results.iter().position(|&id| id == 30).unwrap();
        let pos_10 = results.iter().position(|&id| id == 10).unwrap();
        assert!(pos_20 < pos_30, "insertion order within cell");
        assert!(pos_20 < pos_10, "lower cell index comes first");
    }

    #[test]
    fn query_wraps_toroidally_across_world_edge() {
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(1, (-48.0, 0.0));
        grid.insert(2, (48.0, 0.0));
        // Toroidal distance between these is 4.0, not 96.0
        let results = grid.query_radius((-48.0, 0.0), 5.0);
        assert!(results.contains(&1));
        assert!(results.contains(&2));
    }

    #[test]
    fn agents_outside_radius_are_excluded() {
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(1, (0.0, 0.0));
        grid.insert(2, (3.0, 0.0));
        grid.insert(3, (20.0, 0.0));
        let results = grid.query_radius((0.0, 0.0), 5.0);
        assert!(results.contains(&1));
        assert!(results.contains(&2));
        assert!(!results.contains(&3));
    }

    // --- NutrientGrid tests ---

    #[test]
    fn nutrient_grid_total_equals_initial() {
        let grid = NutrientGrid::new(100.0, 10.0, 500.0);
        assert!((grid.total() - 500.0).abs() < 1e-3,
            "total should equal initial, got {}", grid.total());
    }

    #[test]
    fn nutrient_grid_different_cells_are_independent() {
        // Two positions in different cells should access independent pools
        let mut grid = NutrientGrid::new(100.0, 10.0, 500.0);
        let far_left = (-40.0, 0.0);  // cell near left edge
        let far_right = (40.0, 0.0);  // cell near right edge

        // Drain from one cell
        *grid.at_position(far_left) -= 3.0;

        // Other cell unaffected
        let initial_per_cell = 500.0 / 100.0; // 100 cells (10x10)
        let right_val = *grid.at_position(far_right);
        assert!((right_val - initial_per_cell).abs() < 1e-6,
            "distant cell should be unaffected, got {}", right_val);

        // Total reflects the drain
        assert!((grid.total() - 497.0).abs() < 1e-3,
            "total should reflect drain, got {}", grid.total());
    }
}
