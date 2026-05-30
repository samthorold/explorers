use std::fs;

use eframe::egui;
use egui::{Color32, Pos2, Rect, Stroke, Vec2};
use explorers_sim::{
    InitialDistribution, TraitVector, World, WorldParameters, WorldRecipe,
};

/// Map an agent's trophic traits and reserve to a render colour.
///
/// Framework-agnostic: green for autotrophy, red for heterotrophy, dimmed by
/// reserve level. Returns an egui [`Color32`].
fn trophic_color(traits: &TraitVector, reserve: f32) -> Color32 {
    let brightness = (reserve.max(0.0) / 100.0).clamp(0.5, 1.0);
    let total = traits.photosynthetic_absorption + traits.heterotrophy;
    if total <= 0.0 {
        let v = to_u8(0.8 * brightness);
        return Color32::from_rgb(v, v, v);
    }
    // Base colour: green for autotrophy, red for heterotrophy.
    let r = (traits.heterotrophy / total) * brightness;
    let g = (traits.photosynthetic_absorption / total) * brightness;
    let b = 0.15; // baseline blue for visibility
    Color32::from_rgb(to_u8(r.max(0.15)), to_u8(g.max(0.15)), to_u8(b))
}

fn to_u8(channel: f32) -> u8 {
    (channel.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Map a carcass's energy content to a gray render colour. Brighter carcasses
/// hold more structure energy. Framework-agnostic.
fn carcass_color(energy: f32) -> Color32 {
    let brightness = (energy.max(0.0) / 100.0).clamp(0.2, 1.0);
    let gray = to_u8(0.5 * brightness);
    Color32::from_rgb(gray, gray, gray)
}

/// Accumulates elapsed wall-clock time and emits whole simulation steps at a
/// fixed interval. Framework-agnostic so the stepping cadence can be unit
/// tested without a window.
struct TickAccumulator {
    interval: f32,
    accumulated: f32,
}

impl TickAccumulator {
    fn new(interval: f32) -> Self {
        Self { interval, accumulated: 0.0 }
    }

    /// Add `dt` seconds of elapsed time and return how many sim steps are now
    /// due. The remainder is carried over to the next call.
    fn advance(&mut self, dt: f32) -> u32 {
        self.accumulated += dt;
        let mut steps = 0;
        while self.accumulated >= self.interval {
            self.accumulated -= self.interval;
            steps += 1;
        }
        steps
    }
}

/// Parsed command-line configuration.
#[derive(Debug, PartialEq)]
struct CliConfig {
    recipe_path: Option<String>,
    fast_forward: u64,
}

/// The outcome of parsing argv (excluding the program name).
#[derive(Debug, PartialEq)]
enum CliOutcome {
    Run(CliConfig),
    Help,
    Error(String),
}

/// Parse CLI args (already stripped of the program name). Framework-agnostic so
/// argument handling can be unit tested without spawning a process.
fn parse_args(args: &[String]) -> CliOutcome {
    let mut recipe_path: Option<String> = None;
    let mut fast_forward: u64 = 0;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--recipe" | "--scenario" => {
                i += 1;
                match args.get(i) {
                    Some(path) => recipe_path = Some(path.clone()),
                    None => return CliOutcome::Error(format!("{} requires a path", args[i - 1])),
                }
            }
            "--fast-forward" => {
                i += 1;
                match args.get(i).map(|s| s.parse::<u64>()) {
                    Some(Ok(n)) => fast_forward = n,
                    Some(Err(_)) => {
                        return CliOutcome::Error(format!(
                            "--fast-forward requires a number, got {}",
                            args[i]
                        ));
                    }
                    None => return CliOutcome::Error("--fast-forward requires a number".into()),
                }
            }
            "--help" | "-h" => return CliOutcome::Help,
            other => return CliOutcome::Error(format!("Unknown argument: {other}")),
        }
        i += 1;
    }

    CliOutcome::Run(CliConfig { recipe_path, fast_forward })
}

/// Fixed render radius for agents, in world units.
const AGENT_RADIUS: f32 = 3.0;
/// Half-side length of a carcass square, in world units.
const CARCASS_HALF_SIDE: f32 = 3.0;
/// Wall-clock seconds between simulation steps.
const STEP_INTERVAL: f32 = 1.0;

fn print_help() {
    eprintln!("Usage: explorers-app [OPTIONS]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --recipe PATH        Load world recipe from JSON file");
    eprintln!("  --scenario PATH      Load scenario from JSON file (same format as recipe,");
    eprintln!("                       but may include explicit agents list)");
    eprintln!("  --fast-forward N     Advance simulation N ticks before rendering");
    eprintln!("  --help, -h           Show this help");
}

/// The built-in recipe used when no `--recipe`/`--scenario` is supplied.
fn default_recipe() -> WorldRecipe {
    WorldRecipe {
        parameters: WorldParameters {
            solar_flux_magnitude: 1.0,
            base_trophic_efficiency: 0.5,
            trophic_distance_decay: 1.0,
            reproduction_efficiency: 0.7,
            base_metabolic_rate: 0.1,
            movement_cost_coefficient: 0.05,
            sensing_range_coefficient: 10.0,
            reproduction_energy_threshold: 50.0,
            mutation_rate: 0.1,
            mutation_magnitude: 0.05,
            contact_range_coefficient: 5.0,
            world_extent: 200.0,
            initial_population_size: 3,
            light_competition_radius: 20.0,
            photo_maintenance_cost: 0.01,
            heterotrophy_maintenance_cost: 0.01,
            initial_nutrient_pool: 0.0,
            growth_efficiency: 0.0,
            wear_rate: 0.1,
            wear_degradation_steepness: 1.0,
            somatic_maintenance_cost_coefficient: 0.1,
            use_wear_rate: 0.01,
            structure_maintenance_coefficient: 0.01,
            repair_decay: 1.0,
            base_nutrient_ratio: 0.1,
            specification_nutrient_coefficient: 0.2,
            reproductive_compatibility_distance: 2.0,
            mobility_maintenance_cost: 0.0,
            maintenance_cost_exponent: 1.0,
            consumption_contact_half_saturation: 0.001,
            nutrient_grid_cell_size: 10.0,
            growth_retention_multiplier: 2.0,
            offspring_structure_fraction: 0.2,
            asexual_propensity_maintenance_cost: 0.01,
        },
        initial_distribution: Some(InitialDistribution {
            mean_traits: TraitVector {
                photosynthetic_absorption: 0.5,
                heterotrophy: 0.3,
                mobility: 0.4,
                kappa: 0.5,
                fecundity: 0.0,
                asexual_propensity: 0.0,
                dispersal: 0.0,
            },
            trait_covariance: 0.1,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        }),
        agents: None,
        max_ticks: 100,
    }
}

fn load_recipe(config: &CliConfig) -> WorldRecipe {
    match &config.recipe_path {
        Some(path) => {
            let contents = fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("Failed to read recipe file {path}: {e}"));
            let recipe: WorldRecipe = serde_json::from_str(&contents)
                .unwrap_or_else(|e| panic!("Failed to parse recipe file {path}: {e}"));
            eprintln!("Loaded recipe from {path}");
            recipe
        }
        None => default_recipe(),
    }
}

fn main() -> eframe::Result {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let config = match parse_args(&argv) {
        CliOutcome::Run(config) => config,
        CliOutcome::Help => {
            print_help();
            return Ok(());
        }
        CliOutcome::Error(msg) => {
            eprintln!("{msg}");
            std::process::exit(1);
        }
    };

    let recipe = load_recipe(&config);

    let seed: u64 = rand::random();
    let mut world = World::from_recipe(&recipe, seed);

    if config.fast_forward > 0 {
        eprintln!("Fast-forwarding {} ticks...", config.fast_forward);
        for _ in 0..config.fast_forward {
            world.step();
        }
        eprintln!("Fast-forward complete. {} agents alive.", world.agents().len());
    }

    let app = ExplorersApp::new(world, config.fast_forward);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("Explorers"),
        ..Default::default()
    };
    eframe::run_native("Explorers", options, Box::new(|_cc| Ok(Box::new(app))))
}

/// The eframe application: owns the simulation and drives it on a timer.
struct ExplorersApp {
    world: World,
    tick_count: u64,
    accumulator: TickAccumulator,
}

impl ExplorersApp {
    fn new(world: World, tick_count: u64) -> Self {
        Self {
            world,
            tick_count,
            accumulator: TickAccumulator::new(STEP_INTERVAL),
        }
    }
}

impl eframe::App for ExplorersApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drive the simulation off wall-clock time, independent of frame rate.
        let dt = ctx.input(|i| i.stable_dt);
        let steps = self.accumulator.advance(dt);
        for _ in 0..steps {
            self.world.step();
            self.tick_count += 1;
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(Color32::BLACK))
            .show(ctx, |ui| {
                let extent = self.world.params().world_extent;
                let painter = ui.painter();
                let viewport = ui.available_rect_before_wrap();
                let view = WorldView::fit(extent, viewport);

                // Toroidal world bounds.
                painter.rect_stroke(
                    view.world_bounds(),
                    0.0,
                    Stroke::new(1.0, Color32::from_gray(80)),
                    egui::StrokeKind::Inside,
                );

                // Carcasses: gray squares (brightness from energy).
                for carcass in self.world.carcasses() {
                    let center = view.to_screen(carcass.position);
                    let half = view.scale * CARCASS_HALF_SIDE;
                    let rect = Rect::from_center_size(center, Vec2::splat(half * 2.0));
                    painter.rect_filled(rect, 0.0, carcass_color(carcass.energy));
                }

                // Agents: trophic-coloured circles (fixed radius).
                for agent in self.world.agents() {
                    let center = view.to_screen(agent.position);
                    painter.circle_filled(
                        center,
                        view.scale * AGENT_RADIUS,
                        trophic_color(&agent.traits, agent.reserve),
                    );
                }
            });

        // Keep repainting so the simulation continues to advance on its timer.
        ctx.request_repaint();
    }
}

/// Maps world coordinates (origin-centred, side `extent`) onto a screen
/// viewport rectangle, preserving aspect ratio and centring the world.
struct WorldView {
    extent: f32,
    scale: f32,
    center: Pos2,
}

impl WorldView {
    fn fit(extent: f32, viewport: Rect) -> Self {
        let side = viewport.width().min(viewport.height());
        let scale = if extent > 0.0 { side / extent } else { 1.0 };
        Self { extent, scale, center: viewport.center() }
    }

    fn to_screen(&self, pos: (f32, f32)) -> Pos2 {
        // World y grows upward; screen y grows downward.
        Pos2::new(
            self.center.x + pos.0 * self.scale,
            self.center.y - pos.1 * self.scale,
        )
    }

    fn world_bounds(&self) -> Rect {
        let half = self.extent * self.scale / 2.0;
        Rect::from_center_size(self.center, Vec2::splat(half * 2.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn traits(photosynthetic_absorption: f32, heterotrophy: f32) -> TraitVector {
        TraitVector {
            photosynthetic_absorption,
            heterotrophy,
            mobility: 0.0,
            kappa: 0.0,
            fecundity: 0.0,
            asexual_propensity: 0.0,
            dispersal: 0.0,
        }
    }

    #[test]
    fn pure_producer_maps_to_green() {
        let color = trophic_color(&traits(1.0, 0.0), 100.0);
        assert!(color.g() > 230, "green channel should be high, got {}", color.g());
        assert!(color.r() < 50, "red channel should be low, got {}", color.r());
        assert!(color.b() < 50, "blue channel should be low, got {}", color.b());
    }

    #[test]
    fn pure_consumer_maps_to_red() {
        let color = trophic_color(&traits(0.0, 1.0), 100.0);
        assert!(color.r() > 230, "red channel should be high, got {}", color.r());
        assert!(color.g() < 50, "green channel should be low, got {}", color.g());
        assert!(color.b() < 50, "blue channel should be low, got {}", color.b());
    }

    #[test]
    fn low_reserve_dims_color() {
        let bright = trophic_color(&traits(1.0, 0.0), 100.0);
        let dim = trophic_color(&traits(1.0, 0.0), 10.0);
        assert!(dim.g() < bright.g(), "low energy should dim: {} vs {}", dim.g(), bright.g());
        assert!(dim.g() > 0, "should still be visible at low energy");
    }

    #[test]
    fn brightness_maps_to_reserve_not_total_energy() {
        let high_reserve = trophic_color(&traits(1.0, 0.0), 80.0);
        let low_reserve = trophic_color(&traits(1.0, 0.0), 20.0);
        assert!(
            high_reserve.g() > low_reserve.g(),
            "higher reserve should be brighter: {} vs {}",
            high_reserve.g(),
            low_reserve.g()
        );
    }

    #[test]
    fn carcass_brightness_tracks_energy() {
        let rich = carcass_color(100.0);
        let poor = carcass_color(10.0);
        assert!(rich.r() > poor.r(), "richer carcass should be brighter");
        assert_eq!(rich.r(), rich.g());
        assert_eq!(rich.g(), rich.b(), "carcass colour should be gray");
    }

    #[test]
    fn carcass_brightness_has_a_visible_floor() {
        let empty = carcass_color(0.0);
        assert!(empty.r() > 0, "even an empty carcass should be faintly visible");
    }

    #[test]
    fn accumulator_emits_no_step_before_interval_elapses() {
        let mut acc = TickAccumulator::new(1.0);
        assert_eq!(acc.advance(0.4), 0);
        assert_eq!(acc.advance(0.4), 0);
    }

    #[test]
    fn accumulator_emits_one_step_when_interval_reached() {
        let mut acc = TickAccumulator::new(1.0);
        assert_eq!(acc.advance(0.6), 0);
        assert_eq!(acc.advance(0.6), 1);
    }

    #[test]
    fn accumulator_emits_multiple_steps_for_large_dt() {
        let mut acc = TickAccumulator::new(0.5);
        assert_eq!(acc.advance(1.6), 3);
    }

    fn args(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_recipe_path() {
        let out = parse_args(&args(&["--recipe", "world.json"]));
        assert_eq!(
            out,
            CliOutcome::Run(CliConfig {
                recipe_path: Some("world.json".into()),
                fast_forward: 0,
            })
        );
    }

    #[test]
    fn scenario_is_an_alias_for_recipe() {
        let out = parse_args(&args(&["--scenario", "scn.json"]));
        assert_eq!(
            out,
            CliOutcome::Run(CliConfig {
                recipe_path: Some("scn.json".into()),
                fast_forward: 0,
            })
        );
    }

    #[test]
    fn parses_fast_forward() {
        let out = parse_args(&args(&["--recipe", "w.json", "--fast-forward", "50"]));
        assert_eq!(
            out,
            CliOutcome::Run(CliConfig {
                recipe_path: Some("w.json".into()),
                fast_forward: 50,
            })
        );
    }

    #[test]
    fn help_flag_returns_help() {
        assert_eq!(parse_args(&args(&["--help"])), CliOutcome::Help);
        assert_eq!(parse_args(&args(&["-h"])), CliOutcome::Help);
    }

    #[test]
    fn no_args_runs_with_defaults() {
        assert_eq!(
            parse_args(&[]),
            CliOutcome::Run(CliConfig { recipe_path: None, fast_forward: 0 })
        );
    }

    #[test]
    fn unknown_argument_is_an_error() {
        assert!(matches!(parse_args(&args(&["--nope"])), CliOutcome::Error(_)));
    }

    #[test]
    fn non_numeric_fast_forward_is_an_error() {
        assert!(matches!(
            parse_args(&args(&["--fast-forward", "lots"])),
            CliOutcome::Error(_)
        ));
    }

    #[test]
    fn accumulator_carries_remainder_across_calls() {
        let mut acc = TickAccumulator::new(1.0);
        assert_eq!(acc.advance(0.9), 0);
        // 0.9 + 0.2 = 1.1 -> one step, 0.1 carried.
        assert_eq!(acc.advance(0.2), 1);
        assert_eq!(acc.advance(0.85), 0);
        // 0.1 + 0.85 + 0.06 = 1.01 -> one step.
        assert_eq!(acc.advance(0.06), 1);
    }
}
