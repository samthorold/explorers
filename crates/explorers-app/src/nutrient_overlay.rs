//! The spatial nutrient overlay (#583): where the world's conserved nutrient
//! sits — available in the nutrient grid, stranded in carcasses, bound in
//! living bodies — binned onto the nutrient grid's own cells so bloom-then-
//! lockup can be watched where it happens. Pure reads of public world state;
//! the sim is untouched and nothing here paints.

use egui::Color32;
use explorers_sim::World;

/// Which nutrient the overlay shades, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NutrientLayer {
    #[default]
    Off,
    /// The nutrient grid: the pool agents draw from.
    Available,
    /// Nutrient stranded in carcasses.
    Carcasses,
    /// Nutrient held by living agents.
    Living,
    /// All three at once, one colour channel each.
    Combined,
}

impl NutrientLayer {
    /// The toggle's choices, in the order it offers them.
    pub const ALL: [NutrientLayer; 5] = [
        NutrientLayer::Off,
        NutrientLayer::Available,
        NutrientLayer::Carcasses,
        NutrientLayer::Living,
        NutrientLayer::Combined,
    ];

    /// The toggle's label for this choice.
    pub fn label(self) -> &'static str {
        match self {
            NutrientLayer::Off => "Off",
            NutrientLayer::Available => "Available",
            NutrientLayer::Carcasses => "In carcasses",
            NutrientLayer::Living => "In bodies",
            NutrientLayer::Combined => "Combined",
        }
    }
}

/// One cell's nutrient, by where it sits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellNutrient {
    pub available: f32,
    pub carcasses: f32,
    pub living: f32,
}

/// The shade to lay over one cell under `layer`, `None` for nothing to draw.
///
/// A single layer tints the cell in its pool's hue (available blue, carcasses
/// amber, living green), its opacity the pool's [`shade`]. The combined view
/// gives each pool one channel — carcasses red, living green, available blue —
/// so a cell's hue says where its nutrient sits and its brightness how much.
pub fn cell_color(layer: NutrientLayer, cell: CellNutrient, reference: f32) -> Option<Color32> {
    let single = |value: f32, hue: [f32; 3]| {
        let s = shade(value, reference);
        (s > 0.0).then(|| tint(hue, 1.0, to_alpha(s)))
    };
    match layer {
        NutrientLayer::Off => None,
        NutrientLayer::Available => single(cell.available, AVAILABLE_HUE),
        NutrientLayer::Carcasses => single(cell.carcasses, CARCASS_HUE),
        NutrientLayer::Living => single(cell.living, LIVING_HUE),
        NutrientLayer::Combined => {
            let r = shade(cell.carcasses, reference);
            let g = shade(cell.living, reference);
            let b = shade(cell.available, reference);
            let strongest = r.max(g).max(b);
            (strongest > 0.0).then(|| {
                let c = |x: f32| (x / strongest * 255.0).round() as u8;
                Color32::from_rgba_unmultiplied(c(r), c(g), c(b), to_alpha(strongest))
            })
        }
    }
}

/// Most opaque an overlay cell is drawn, so agents and carcasses on top
/// still read against it.
const MAX_CELL_ALPHA: f32 = 200.0;
/// Available nutrient's hue: blue.
const AVAILABLE_HUE: [f32; 3] = [0.25, 0.55, 1.0];
/// Living-body nutrient's hue: green.
const LIVING_HUE: [f32; 3] = [0.3, 1.0, 0.35];

fn to_alpha(shade: f32) -> u8 {
    (shade * MAX_CELL_ALPHA).round() as u8
}

/// Colour of one carcass square under the carcass layer: amber, brightening
/// with the nutrient the carcass holds against the field's `reference`
/// (never dimmer than a visible floor, so an empty carcass still shows).
pub fn carcass_nutrient_color(nutrient: f32, reference: f32) -> Color32 {
    let brightness = CARCASS_FLOOR + (1.0 - CARCASS_FLOOR) * shade(nutrient, reference);
    tint(CARCASS_HUE, brightness, 255)
}

/// Dimmest a carcass square is drawn under the carcass layer.
const CARCASS_FLOOR: f32 = 0.25;
/// Carcass nutrient's hue: amber.
const CARCASS_HUE: [f32; 3] = [1.0, 0.55, 0.1];

/// How strongly `value` shows against `reference`, in `[0, 1]`: a square
/// root over four times the reference, so a lockup-scale pile saturates
/// only well above the mean and faint spreads still register. Monotone.
pub fn shade(value: f32, reference: f32) -> f32 {
    if reference <= 0.0 {
        return if value > 0.0 { 1.0 } else { 0.0 };
    }
    (value.max(0.0) / (4.0 * reference)).sqrt().clamp(0.0, 1.0)
}

fn tint(hue: [f32; 3], brightness: f32, alpha: u8) -> Color32 {
    let c = |x: f32| ((x * brightness).clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgba_unmultiplied(c(hue[0]), c(hue[1]), c(hue[2]), alpha)
}

/// The world's nutrient, per nutrient-grid cell, by where it sits. Cells are
/// row-major over the grid the sim draws from (`nutrient_grid_cell_size`
/// squares tiling the origin-centred world).
pub struct NutrientField {
    pub cols: usize,
    pub cell_size: f32,
    pub extent: f32,
    /// The nutrient grid's own per-cell values: the pool agents draw from.
    pub available: Vec<f32>,
    /// Carcass nutrient, summed over the carcasses lying in each cell.
    pub carcasses: Vec<f32>,
    /// Nutrient held by living agents (free, earmarked and bound in
    /// structure), summed over the agents in each cell.
    pub living: Vec<f32>,
}

impl NutrientField {
    /// Read the world's nutrient onto its grid cells. The grid's geometry is
    /// the sim's own (`world_extent` tiled by `nutrient_grid_cell_size`, as
    /// `NutrientGrid::new` lays it out); neither is live-tunable in the app.
    pub fn of(world: &World) -> Self {
        let params = world.params();
        let extent = params.world_extent;
        let cell_size = params.nutrient_grid_cell_size;
        let cols = (extent / cell_size).ceil() as usize;
        // The sim exposes per-cell values only through `cell_mut`, so read a
        // copy: the overlay never touches the running world's grid.
        let mut grid = world.nutrient_grid().clone();
        let available = (0..cols * cols).map(|i| *grid.cell_mut(i)).collect();
        let mut field = Self {
            cols,
            cell_size,
            extent,
            available,
            carcasses: vec![0.0; cols * cols],
            living: vec![0.0; cols * cols],
        };
        for carcass in world.carcasses() {
            let cell = field.cell_of(carcass.position);
            field.carcasses[cell] += carcass.nutrient;
        }
        for agent in world.agents() {
            let cell = field.cell_of(agent.position);
            field.living[cell] += agent.nutrient_total(params);
        }
        field
    }

    /// The shade reference: the world's total nutrient per cell.
    pub fn reference(&self) -> f32 {
        let cells = self.available.len();
        if cells == 0 {
            return 0.0;
        }
        let total: f32 = [&self.available, &self.carcasses, &self.living]
            .iter()
            .flat_map(|pool| pool.iter())
            .sum();
        total / cells as f32
    }

    /// Cell `i`'s world-space square, as (min corner, max corner).
    /// The last row and column are cut at the world's edge when the extent
    /// is not a whole number of cells.
    pub fn cell_bounds(&self, i: usize) -> ((f32, f32), (f32, f32)) {
        let half = self.extent / 2.0;
        let (row, col) = (i / self.cols, i % self.cols);
        let min = (
            col as f32 * self.cell_size - half,
            row as f32 * self.cell_size - half,
        );
        let max = (
            (min.0 + self.cell_size).min(half),
            (min.1 + self.cell_size).min(half),
        );
        (min, max)
    }

    /// Cell `i`'s nutrient, by pool.
    pub fn cell(&self, i: usize) -> CellNutrient {
        CellNutrient {
            available: self.available[i],
            carcasses: self.carcasses[i],
            living: self.living[i],
        }
    }

    /// Row-major index of the cell holding world position `pos`, mapped the
    /// way the sim's `NutrientGrid` maps it (edges clamp into the grid).
    pub fn cell_of(&self, pos: (f32, f32)) -> usize {
        let half = self.extent / 2.0;
        let last = self.cols.saturating_sub(1);
        let col = (((pos.0 + half) / self.cell_size) as usize).min(last);
        let row = (((pos.1 + half) / self.cell_size) as usize).min(last);
        row * self.cols + col
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use explorers_sim::WorldRecipe;

    /// The repo's `recipe.json` (an atlas cell's world), stepped until uptake
    /// and release have made the grid uneven.
    fn stepped_world(ticks: u32) -> World {
        let recipe: WorldRecipe =
            serde_json::from_str(include_str!("../../../recipe.json")).unwrap();
        let mut world = World::from_recipe(&recipe, 11);
        for _ in 0..ticks {
            world.step();
        }
        world
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-3 * a.abs().max(b.abs()).max(1.0)
    }

    #[test]
    fn the_available_layer_sums_to_the_nutrient_pool_the_budget_reports() {
        let world = stepped_world(60);
        let field = NutrientField::of(&world);
        assert_eq!(field.available.len(), field.cols * field.cols);
        let summed: f32 = field.available.iter().sum();
        let reported = crate::compute_energy_budget(&world).nutrient_available;
        assert!(
            close(summed, reported),
            "overlay cells sum to {summed}, the budget panel reports {reported}"
        );
        assert!(
            world.nutrient_pool() > 0.0,
            "the check needs a non-empty pool"
        );
    }

    #[test]
    fn the_carcass_layer_bins_carcass_nutrient_into_the_cell_under_each_carcass() {
        let world = stepped_world(150);
        assert!(!world.carcasses().is_empty(), "the check needs carcasses");
        let field = NutrientField::of(&world);
        assert_eq!(field.carcasses.len(), field.cols * field.cols);
        let summed: f32 = field.carcasses.iter().sum();
        let reported = crate::compute_energy_budget(&world).nutrient_carcasses;
        assert!(
            close(summed, reported),
            "carcass cells sum to {summed}, the budget panel reports {reported}"
        );
        let richest = world
            .carcasses()
            .iter()
            .max_by(|a, b| a.nutrient.total_cmp(&b.nutrient))
            .unwrap();
        assert!(richest.nutrient > 0.0);
        let under = field.carcasses[field.cell_of(richest.position)];
        assert!(
            under >= richest.nutrient * 0.999,
            "the cell under the richest carcass holds {under}, less than its {}",
            richest.nutrient
        );
    }

    #[test]
    fn the_living_layer_bins_the_nutrient_bound_in_living_bodies() {
        let world = stepped_world(60);
        assert!(!world.agents().is_empty(), "the check needs living agents");
        let field = NutrientField::of(&world);
        assert_eq!(field.living.len(), field.cols * field.cols);
        let summed: f32 = field.living.iter().sum();
        let reported = crate::compute_energy_budget(&world).nutrient_living;
        assert!(
            close(summed, reported),
            "living cells sum to {summed}, the budget panel reports {reported}"
        );
    }

    #[test]
    fn carcass_colour_brightens_monotonically_with_carcass_nutrient() {
        let reference = 10.0;
        let levels = [0.0, 1.0, 5.0, 10.0, 40.0, 200.0];
        let reds: Vec<u8> = levels
            .iter()
            .map(|&n| carcass_nutrient_color(n, reference).r())
            .collect();
        assert!(
            reds.windows(2).all(|w| w[0] <= w[1]),
            "brightness never falls as nutrient rises: {reds:?}"
        );
        assert!(
            carcass_nutrient_color(20.0, reference).r()
                > carcass_nutrient_color(2.0, reference).r(),
            "a carcass holding more nutrient is brighter"
        );
    }

    #[test]
    fn carcass_colour_has_a_visible_floor() {
        let empty = carcass_nutrient_color(0.0, 10.0);
        assert!(
            empty.r() > 0,
            "a carcass holding no nutrient is still faintly visible"
        );
    }

    /// A cell holding `n` in exactly one pool.
    fn only(pool: NutrientLayer, n: f32) -> CellNutrient {
        CellNutrient {
            available: if pool == NutrientLayer::Available {
                n
            } else {
                0.0
            },
            carcasses: if pool == NutrientLayer::Carcasses {
                n
            } else {
                0.0
            },
            living: if pool == NutrientLayer::Living {
                n
            } else {
                0.0
            },
        }
    }

    #[test]
    fn with_the_overlay_off_no_cell_is_shaded() {
        let full = CellNutrient {
            available: 50.0,
            carcasses: 50.0,
            living: 50.0,
        };
        assert_eq!(cell_color(NutrientLayer::Off, full, 10.0), None);
    }

    #[test]
    fn a_single_layer_shades_only_its_own_pool_and_brightens_with_it() {
        for layer in [
            NutrientLayer::Available,
            NutrientLayer::Carcasses,
            NutrientLayer::Living,
        ] {
            let others = [
                NutrientLayer::Available,
                NutrientLayer::Carcasses,
                NutrientLayer::Living,
            ]
            .into_iter()
            .filter(|&o| o != layer);
            for other in others {
                assert_eq!(
                    cell_color(layer, only(other, 50.0), 10.0),
                    None,
                    "{layer:?} ignores nutrient in {other:?}"
                );
            }
            let faint = cell_color(layer, only(layer, 2.0), 10.0).unwrap();
            let rich = cell_color(layer, only(layer, 30.0), 10.0).unwrap();
            assert!(
                rich.a() > faint.a(),
                "{layer:?}: more nutrient, stronger shade"
            );
        }
    }

    #[test]
    fn the_combined_view_tells_the_three_pools_apart() {
        let rgb = |pool| {
            let c = cell_color(NutrientLayer::Combined, only(pool, 30.0), 10.0).unwrap();
            [c.r(), c.g(), c.b()]
        };
        let dominant = |c: [u8; 3]| (0..3).max_by_key(|&i| c[i]).unwrap();
        let (a, c, l) = (
            rgb(NutrientLayer::Available),
            rgb(NutrientLayer::Carcasses),
            rgb(NutrientLayer::Living),
        );
        let channels = [dominant(a), dominant(c), dominant(l)];
        assert!(
            channels[0] != channels[1] && channels[1] != channels[2] && channels[0] != channels[2],
            "each pool leads with its own channel: available {a:?}, carcasses {c:?}, living {l:?}"
        );
    }

    #[test]
    fn the_shading_reference_is_the_worlds_conserved_nutrient_per_cell() {
        // One reference for every layer and every tick: the whole world's
        // nutrient spread evenly over the cells. Nutrient is conserved, so a
        // transfer from the grid into carcasses dims one layer as it
        // brightens the other rather than rescaling both.
        let world = stepped_world(150);
        let field = NutrientField::of(&world);
        let budget = crate::compute_energy_budget(&world);
        let total = budget.nutrient_available + budget.nutrient_carcasses + budget.nutrient_living;
        let expected = total / (field.cols * field.cols) as f32;
        assert!(
            close(field.reference(), expected),
            "reference {} vs {expected}",
            field.reference()
        );
    }

    #[test]
    fn each_cell_covers_the_world_square_its_positions_bin_into() {
        let world = stepped_world(0);
        let field = NutrientField::of(&world);
        assert!(field.cols > 1);
        for i in 0..field.cols * field.cols {
            let (min, max) = field.cell_bounds(i);
            let centre = ((min.0 + max.0) / 2.0, (min.1 + max.1) / 2.0);
            assert_eq!(field.cell_of(centre), i, "cell {i} spans {min:?}..{max:?}");
            let side = max.0 - min.0;
            assert!(side > 0.0 && side <= field.cell_size * 1.0001);
        }
    }

    #[test]
    fn the_toggle_offers_no_overlay_each_layer_and_the_combined_view() {
        assert_eq!(
            NutrientLayer::ALL,
            [
                NutrientLayer::Off,
                NutrientLayer::Available,
                NutrientLayer::Carcasses,
                NutrientLayer::Living,
                NutrientLayer::Combined,
            ]
        );
        let labels: std::collections::HashSet<_> =
            NutrientLayer::ALL.iter().map(|l| l.label()).collect();
        assert_eq!(
            labels.len(),
            NutrientLayer::ALL.len(),
            "each choice is labelled apart"
        );
    }
}
