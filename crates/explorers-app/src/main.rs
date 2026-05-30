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

/// Default simulation rate when the app starts, in ticks per second.
const DEFAULT_TICKS_PER_SECOND: f32 = 1.0;
/// Slowest the clock can be driven, in ticks per second (one tick every 2s).
/// Mirrors the old Bevy 2000ms timestep ceiling.
const MIN_TICKS_PER_SECOND: f32 = 0.5;
/// Fastest the clock can be driven, in ticks per second (one tick every 10ms).
/// Mirrors the old Bevy 10ms timestep floor.
const MAX_TICKS_PER_SECOND: f32 = 100.0;

/// Drives simulation time off the eframe update loop. Owns the run/pause state,
/// the target ticks-per-second, and the carried-over time accumulator. Lives in
/// the app (never in `explorers-sim`) per ADR 0001: the sim has no opinion about
/// timing. Framework-agnostic so the clock logic is unit-testable without a
/// window.
struct RunClock {
    ticks_per_second: f32,
    accumulated: f32,
    paused: bool,
}

impl RunClock {
    fn new() -> Self {
        Self {
            ticks_per_second: DEFAULT_TICKS_PER_SECOND,
            accumulated: 0.0,
            paused: false,
        }
    }

    fn ticks_per_second(&self) -> f32 {
        self.ticks_per_second
    }

    fn is_paused(&self) -> bool {
        self.paused
    }

    /// Toggle between running and paused.
    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// Double the tick rate, clamped to the ceiling.
    fn speed_up(&mut self) {
        self.ticks_per_second = (self.ticks_per_second * 2.0).min(MAX_TICKS_PER_SECOND);
    }

    /// Halve the tick rate, clamped to the floor.
    fn slow_down(&mut self) {
        self.ticks_per_second = (self.ticks_per_second / 2.0).max(MIN_TICKS_PER_SECOND);
    }

    /// Request a single manual tick. Only meaningful while paused: returns 1 to
    /// signal one step is due, or 0 when running (where time already drives the
    /// clock). Does not touch the carried-over accumulator.
    fn step_once(&mut self) -> u32 {
        if self.paused {
            1
        } else {
            0
        }
    }

    /// Add `dt` seconds of elapsed wall-clock time and return how many sim steps
    /// are now due. Emits nothing while paused; the remainder carries over.
    fn advance(&mut self, dt: f32) -> u32 {
        if self.paused {
            return 0;
        }
        self.accumulated += dt;
        let interval = 1.0 / self.ticks_per_second;
        let mut steps = 0;
        while self.accumulated >= interval {
            self.accumulated -= interval;
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

/// Select the agent nearest a click position, respecting the toroidal wrap of
/// the world. Returns the agent id, or `None` if there are no agents.
///
/// Framework-agnostic so selection resolution is unit-testable without a
/// window. Lives in the app (never in `explorers-sim`) per ADR 0001: the sim
/// has no opinion about selection.
fn find_nearest_agent(
    agents: &[explorers_sim::Agent],
    click_pos: (f32, f32),
    world_extent: f32,
) -> Option<u64> {
    agents
        .iter()
        .map(|a| (a.id, explorers_sim::toroidal_distance(click_pos, a.position, world_extent)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(id, _)| id)
}

/// A snapshot of where the world's energy and nutrient currently sit. Energy is
/// open (solar in, dissipation/transfer-loss out); nutrient is conserved and
/// cycles between pools. Framework-agnostic so the budget aggregation is
/// unit-testable without a window, and computed in the app (never in
/// `explorers-sim`) per ADR 0001.
#[derive(Debug, PartialEq)]
struct EnergyBudget {
    living_reserve: f32,
    living_structure: f32,
    living_energy: f32,
    carcass_energy: f32,
    dissipated_energy: f32,
    nutrient_available: f32,
    nutrient_living: f32,
    nutrient_carcasses: f32,
}

/// Aggregate the world's instantaneous energy and nutrient distribution into an
/// [`EnergyBudget`] for the debug readout. Pure read of public world state.
fn compute_energy_budget(world: &World) -> EnergyBudget {
    EnergyBudget {
        living_reserve: world.agents().iter().map(|a| a.reserve).sum(),
        living_structure: world.agents().iter().map(|a| a.structure).sum(),
        living_energy: world.agents().iter().map(|a| a.energy()).sum(),
        carcass_energy: world.carcasses().iter().map(|c| c.energy).sum(),
        dissipated_energy: world.dissipated_energy(),
        nutrient_available: world.nutrient_pool(),
        nutrient_living: world.agents().iter().map(|a| a.nutrient).sum(),
        nutrient_carcasses: world.carcasses().iter().map(|c| c.nutrient).sum(),
    }
}

/// Contact-time aggregation across the living population for the status line:
/// the mean and the longest sustained contact. Contact time accrues while an
/// agent is in consuming range of a target. Framework-agnostic and unit-testable.
#[derive(Debug, PartialEq, Default)]
struct ContactTimeStats {
    average: f64,
    max: u64,
}

impl ContactTimeStats {
    /// Compute the contact-time average and maximum for a slice of agents. An
    /// empty population yields zeroes.
    fn of(agents: &[explorers_sim::Agent]) -> Self {
        if agents.is_empty() {
            return Self::default();
        }
        let sum: f64 = agents.iter().map(|a| a.contact_time as f64).sum();
        let max = agents.iter().map(|a| a.contact_time).max().unwrap_or(0);
        Self { average: sum / agents.len() as f64, max }
    }
}

/// The living population split by dominant trophic role for the status line.
/// Roles are a reading of each agent's trait vector (see [`dominant_role`]),
/// never an assigned type. Framework-agnostic and unit-testable.
#[derive(Debug, PartialEq, Default)]
struct RoleBreakdown {
    total: usize,
    producers: usize,
    consumers: usize,
    decomposers: usize,
}

impl RoleBreakdown {
    /// Tally a slice of agents into the role breakdown.
    fn count(agents: &[explorers_sim::Agent]) -> Self {
        let mut breakdown = RoleBreakdown { total: agents.len(), ..Default::default() };
        for agent in agents {
            match dominant_role(&agent.traits) {
                "producers" => breakdown.producers += 1,
                "consumers" => breakdown.consumers += 1,
                _ => breakdown.decomposers += 1,
            }
        }
        breakdown
    }
}

/// Classify an agent's trait vector into its dominant trophic role for the
/// status-line breakdown. A reading of the trait vector, never an assigned type
/// (per CONTEXT.md): autotrophy-leaning reads as a producer, heterotrophy-leaning
/// as a consumer. Ties read as producers. Framework-agnostic and unit-testable.
fn dominant_role(traits: &TraitVector) -> &'static str {
    if traits.photosynthetic_absorption >= traits.heterotrophy {
        "producers"
    } else {
        "consumers"
    }
}

/// Fixed render radius for agents, in world units.
const AGENT_RADIUS: f32 = 3.0;
/// Half-side length of a carcass square, in world units.
const CARCASS_HALF_SIDE: f32 = 3.0;

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
    clock: RunClock,
    /// Shared selection state: the id of the agent the user clicked, if any.
    /// Lives on the app (never in `explorers-sim`) per ADR 0001 so future debug
    /// views can read it. `None` when nothing is selected; a stale id whose
    /// agent has died is handled gracefully at lookup time (the painter and any
    /// reader treat a missing agent as "no longer alive").
    selected_agent: Option<u64>,
}

impl ExplorersApp {
    fn new(world: World, tick_count: u64) -> Self {
        Self { world, tick_count, clock: RunClock::new(), selected_agent: None }
    }

    /// Advance the simulation by `steps` whole ticks, keeping the tick readout
    /// in sync.
    fn apply_steps(&mut self, steps: u32) {
        for _ in 0..steps {
            self.world.step();
            self.tick_count += 1;
        }
    }
}

impl eframe::App for ExplorersApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drive the simulation off wall-clock time, independent of frame rate.
        // While paused this yields nothing; speed changes alter the target rate.
        let dt = ctx.input(|i| i.stable_dt);
        let steps = self.clock.advance(dt);
        self.apply_steps(steps);

        // Run-control toolbar, co-located with the spatial view: play/pause,
        // single-step, speed, and a live tick + agent-count readout.
        egui::TopBottomPanel::top("run_control").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let pause_label = if self.clock.is_paused() { "Play" } else { "Pause" };
                if ui.button(pause_label).clicked() {
                    self.clock.toggle_pause();
                }
                if ui
                    .add_enabled(self.clock.is_paused(), egui::Button::new("Step"))
                    .clicked()
                {
                    let steps = self.clock.step_once();
                    self.apply_steps(steps);
                }
                if ui.button("Slower").clicked() {
                    self.clock.slow_down();
                }
                if ui.button("Faster").clicked() {
                    self.clock.speed_up();
                }
                ui.separator();
                ui.label(format!("{:.2} ticks/s", self.clock.ticks_per_second()));
                if self.clock.is_paused() {
                    ui.label("PAUSED");
                }
                ui.separator();
                ui.label(format!("Tick: {}", self.tick_count));
                ui.label(format!("Agents: {}", self.world.agents().len()));
            });
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(Color32::BLACK))
            .show(ctx, |ui| {
                let extent = self.world.params().world_extent;
                let viewport = ui.available_rect_before_wrap();
                let view = WorldView::fit(extent, viewport);

                // Make the whole spatial view clickable so a left-click selects
                // the nearest agent. Mapping cursor -> world is a pure helper;
                // nearest-agent resolution respects the toroidal wrap.
                let response = ui.allocate_rect(viewport, egui::Sense::click());
                if response.clicked()
                    && let Some(screen_pos) = response.interact_pointer_pos()
                {
                    let click_world = view.to_world(screen_pos);
                    self.selected_agent =
                        find_nearest_agent(self.world.agents(), click_world, extent);
                }

                let painter = ui.painter();

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

                // Selection highlight: a ring around the selected agent. Looked
                // up by id; if the agent has died the lookup yields nothing and
                // nothing is drawn (selection handled gracefully).
                if let Some(selected) = self.selected_agent
                    && let Some(agent) =
                        self.world.agents().iter().find(|a| a.id == selected)
                {
                    let center = view.to_screen(agent.position);
                    painter.circle_stroke(
                        center,
                        view.scale * AGENT_RADIUS + 3.0,
                        Stroke::new(2.0, Color32::WHITE),
                    );
                }
            });

        // Second OS window: instrumentation only, no run-control chrome (that
        // lives on the grid window above). Rendered as an immediate viewport so
        // the parameter sliders can mutate the running world in place.
        self.show_debug_window(ctx);

        // Keep repainting so the simulation continues to advance on its timer.
        ctx.request_repaint();
    }
}

/// Width hint for the debug window when it first opens.
const DEBUG_WINDOW_WIDTH: f32 = 360.0;
/// Height hint for the debug window when it first opens.
const DEBUG_WINDOW_HEIGHT: f32 = 720.0;

impl ExplorersApp {
    /// Render the dedicated debug window in its own OS window via egui
    /// multi-viewport. Hosts the instantaneous readouts ported from the old
    /// Bevy app: status line, energy budget, world-parameter sliders, and the
    /// selected-agent inspector. All readout state lives here in the app, never
    /// in `explorers-sim`, per ADR 0001.
    fn show_debug_window(&mut self, ctx: &egui::Context) {
        let viewport_id = egui::ViewportId::from_hash_of("debug_window");
        let builder = egui::ViewportBuilder::default()
            .with_title("Explorers — Debug")
            .with_inner_size([DEBUG_WINDOW_WIDTH, DEBUG_WINDOW_HEIGHT]);

        ctx.show_viewport_immediate(viewport_id, builder, |ctx, _class| {
            egui::CentralPanel::default().show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.debug_status_line(ui);
                    ui.separator();
                    self.debug_energy_budget(ui);
                    ui.separator();
                    self.debug_world_parameters(ui);
                    ui.separator();
                    self.debug_selected_agent(ui);
                });
            });

            // Closing the debug window should not tear down the whole app; it
            // simply leaves the grid window running.
            if ctx.input(|i| i.viewport().close_requested()) {
                // Nothing to clean up; the viewport disappears on its own.
            }
        });
    }

    /// Status line: tick, role breakdown, carcass count, contact-time avg/max,
    /// and ticks-per-second.
    fn debug_status_line(&self, ui: &mut egui::Ui) {
        let agents = self.world.agents();
        let breakdown = RoleBreakdown::count(agents);
        let contact = ContactTimeStats::of(agents);

        ui.label(format!("Tick: {}", self.tick_count));
        ui.label(format!(
            "Agents: {} ({} P / {} C / {} D)",
            breakdown.total, breakdown.producers, breakdown.consumers, breakdown.decomposers
        ));
        ui.label(format!("Carcasses: {}", self.world.carcasses().len()));
        ui.label(format!(
            "Contact time: avg {:.0} / max {}",
            contact.average, contact.max
        ));
        let paused = if self.clock.is_paused() { " | PAUSED" } else { "" };
        ui.label(format!("TPS: {:.2}{}", self.clock.ticks_per_second(), paused));
    }

    /// Energy budget: living reserve/structure/total, carcass structure,
    /// dissipated, grand total, and the three nutrient pools.
    fn debug_energy_budget(&self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Energy Budget")
            .default_open(true)
            .show(ui, |ui| {
                let budget = compute_energy_budget(&self.world);
                ui.label(format!("Living reserve: {:.1}", budget.living_reserve));
                ui.label(format!("Living structure: {:.1}", budget.living_structure));
                ui.label(format!("Living total: {:.1}", budget.living_energy));
                ui.label(format!("Carcass structure: {:.1}", budget.carcass_energy));
                ui.label(format!("Dissipated: {:.1}", budget.dissipated_energy));
                let grand_total =
                    budget.living_energy + budget.carcass_energy + budget.dissipated_energy;
                ui.label(format!("Grand total: {:.1}", grand_total));
                ui.separator();
                ui.label("Nutrients:");
                ui.label(format!("  Available pool: {:.1}", budget.nutrient_available));
                ui.label(format!("  Living agents: {:.1}", budget.nutrient_living));
                ui.label(format!("  Carcasses: {:.1}", budget.nutrient_carcasses));
            });
    }

    /// World-parameter sliders: live mutation of the running world's parameters
    /// mid-run via `params_mut`. The hypothesis-poking surface.
    fn debug_world_parameters(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("World Parameters")
            .default_open(true)
            .show(ui, |ui| {
                let params = self.world.params_mut();
                ui.add(egui::Slider::new(&mut params.solar_flux_magnitude, 0.0..=200.0).text("Solar flux"));
                ui.add(egui::Slider::new(&mut params.base_metabolic_rate, 0.0..=2.0).text("Base metabolic rate"));
                ui.add(egui::Slider::new(&mut params.photo_maintenance_cost, 0.0..=0.5).text("Photo maintenance"));
                ui.add(egui::Slider::new(&mut params.heterotrophy_maintenance_cost, 0.0..=0.5).text("Heterotrophy maintenance"));
                ui.add(egui::Slider::new(&mut params.somatic_maintenance_cost_coefficient, 0.0..=1.0).text("Somatic maintenance"));
                ui.add(egui::Slider::new(&mut params.structure_maintenance_coefficient, 0.0..=0.1).text("Structure maintenance"));
                ui.add(egui::Slider::new(&mut params.mobility_maintenance_cost, 0.0..=0.5).text("Mobility maintenance"));
                ui.add(egui::Slider::new(&mut params.asexual_propensity_maintenance_cost, 0.0..=0.5).text("Asexual-propensity maintenance"));
                ui.add(egui::Slider::new(&mut params.maintenance_cost_exponent, 1.0..=3.0).text("Maintenance cost exponent"));
                ui.add(egui::Slider::new(&mut params.mutation_rate, 0.0..=1.0).text("Mutation rate"));
                ui.add(egui::Slider::new(&mut params.mutation_magnitude, 0.0..=0.5).text("Mutation magnitude"));
                ui.add(egui::Slider::new(&mut params.contact_range_coefficient, 1.0..=50.0).text("Contact range coefficient"));
                ui.add(egui::Slider::new(&mut params.light_competition_radius, 1.0..=100.0).text("Light competition radius"));
                ui.add(egui::Slider::new(&mut params.growth_efficiency, 0.0..=1.0).text("Growth efficiency"));
                ui.add(egui::Slider::new(&mut params.wear_rate, 0.0..=1.0).text("Wear rate"));
                ui.add(egui::Slider::new(&mut params.wear_degradation_steepness, 0.0..=5.0).text("Wear degradation steepness"));
                ui.add(egui::Slider::new(&mut params.use_wear_rate, 0.0..=0.5).text("Use wear rate"));
                ui.add(egui::Slider::new(&mut params.repair_decay, 0.0..=5.0).text("Repair decay"));
                ui.add(egui::Slider::new(&mut params.base_trophic_efficiency, 0.0..=1.0).text("Base trophic efficiency"));
                ui.add(egui::Slider::new(&mut params.reproduction_efficiency, 0.0..=1.0).text("Reproduction efficiency"));
                ui.add(egui::Slider::new(&mut params.reproduction_energy_threshold, 0.0..=200.0).text("Reproduction energy threshold"));
            });
    }

    /// Selected-agent inspector: reads the shared selection and shows the
    /// agent's full state and trait vector. Handles the no-selection and
    /// dead-agent cases gracefully.
    fn debug_selected_agent(&self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Selected Agent")
            .default_open(true)
            .show(ui, |ui| match self.selected_agent {
                None => {
                    ui.label("Click an agent in the grid window to inspect");
                }
                Some(agent_id) => {
                    match self.world.agents().iter().find(|a| a.id == agent_id) {
                        Some(agent) => {
                            ui.label(format!("ID: {}", agent.id));
                            ui.label(format!(
                                "Position: ({:.1}, {:.1})",
                                agent.position.0, agent.position.1
                            ));
                            ui.label(format!("Reserve: {:.1}  (death at 0)", agent.reserve));
                            ui.label(format!("Structure: {:.1}", agent.structure));
                            ui.label(format!("Nutrient: {:.1}", agent.nutrient));
                            ui.label(format!("Repro reserve: {:.1}", agent.repro_reserve));
                            ui.label(format!("Contact time: {}", agent.contact_time));
                            ui.label(format!("Dominant role: {}", dominant_role(&agent.traits)));
                            let threshold =
                                self.world.params().reproduction_energy_threshold;
                            let demand = explorers_sim::stoichiometric_demand(
                                &agent.traits,
                                agent.structure,
                                self.world.params(),
                            );
                            ui.label(format!(
                                "Repro gates: energy >= {:.0}, nutrient >= {:.1}",
                                threshold, demand
                            ));
                            ui.separator();
                            ui.label("Trait vector:");
                            let t = &agent.traits;
                            ui.label(format!("  autotrophy: {:.3}", t.photosynthetic_absorption));
                            ui.label(format!("  heterotrophy: {:.3}", t.heterotrophy));
                            ui.label(format!("  mobility: {:.3}", t.mobility));
                            ui.label(format!("  kappa: {:.3}", t.kappa));
                            ui.label(format!("  fecundity: {:.3}", t.fecundity));
                            ui.label(format!("  asexual_propensity: {:.3}", t.asexual_propensity));
                            ui.label(format!("  dispersal: {:.3}", t.dispersal));
                        }
                        None => {
                            ui.label(format!("Agent {} is no longer alive", agent_id));
                        }
                    }
                }
            });
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

    /// Inverse of [`to_screen`]: map a screen position back to world
    /// coordinates. Used to resolve a click position into the world so the
    /// nearest agent can be selected.
    fn to_world(&self, screen: Pos2) -> (f32, f32) {
        (
            (screen.x - self.center.x) / self.scale,
            (self.center.y - screen.y) / self.scale,
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
    fn contact_time_stats_average_and_max() {
        let mut a = agent_at(1, (0.0, 0.0));
        a.contact_time = 2;
        let mut b = agent_at(2, (0.0, 0.0));
        b.contact_time = 4;
        let mut c = agent_at(3, (0.0, 0.0));
        c.contact_time = 6;
        let stats = ContactTimeStats::of(&[a, b, c]);
        assert!((stats.average - 4.0).abs() < 1e-6, "avg of 2,4,6 is 4");
        assert_eq!(stats.max, 6);
    }

    #[test]
    fn contact_time_stats_of_empty_population_is_zero() {
        let stats = ContactTimeStats::of(&[]);
        assert_eq!(stats.average, 0.0);
        assert_eq!(stats.max, 0);
    }

    #[test]
    fn role_breakdown_counts_by_dominant_role() {
        let agents = vec![
            agent_with_traits(1, traits(0.9, 0.1)), // producer
            agent_with_traits(2, traits(0.8, 0.2)), // producer
            agent_with_traits(3, traits(0.1, 0.9)), // consumer
        ];
        let breakdown = RoleBreakdown::count(&agents);
        assert_eq!(breakdown.total, 3);
        assert_eq!(breakdown.producers, 2);
        assert_eq!(breakdown.consumers, 1);
        assert_eq!(breakdown.decomposers, 0);
    }

    #[test]
    fn role_breakdown_of_empty_population_is_all_zero() {
        let breakdown = RoleBreakdown::count(&[]);
        assert_eq!(breakdown.total, 0);
        assert_eq!(breakdown.producers, 0);
        assert_eq!(breakdown.consumers, 0);
        assert_eq!(breakdown.decomposers, 0);
    }

    #[test]
    fn energy_budget_aggregates_living_carcass_and_dissipated() {
        let world = World::from_recipe(&default_recipe(), 42);
        let budget = compute_energy_budget(&world);

        // Living energy is the sum of every agent's reserve + structure.
        let expected_reserve: f32 = world.agents().iter().map(|a| a.reserve).sum();
        let expected_structure: f32 = world.agents().iter().map(|a| a.structure).sum();
        assert!((budget.living_reserve - expected_reserve).abs() < 1e-3);
        assert!((budget.living_structure - expected_structure).abs() < 1e-3);
        assert!(
            (budget.living_energy - (expected_reserve + expected_structure)).abs() < 1e-3,
            "living energy should be reserve + structure"
        );

        // Carcass energy and dissipated come straight off the world.
        let expected_carcass: f32 = world.carcasses().iter().map(|c| c.energy).sum();
        assert!((budget.carcass_energy - expected_carcass).abs() < 1e-3);
        assert!((budget.dissipated_energy - world.dissipated_energy()).abs() < 1e-3);

        // Nutrient pools are reported.
        assert!((budget.nutrient_available - world.nutrient_pool()).abs() < 1e-3);
        let expected_living_nutrient: f32 = world.agents().iter().map(|a| a.nutrient).sum();
        assert!((budget.nutrient_living - expected_living_nutrient).abs() < 1e-3);
    }

    #[test]
    fn energy_budget_grand_total_is_conserved_quantity() {
        // The grand total — living + carcass + dissipated — should track the
        // total solar input the world has admitted (open system: solar is the
        // only tap). At tick zero with no solar yet, grand total is the seeded
        // living energy and total_solar_input is zero plus that seed.
        let world = World::from_recipe(&default_recipe(), 7);
        let budget = compute_energy_budget(&world);
        let grand_total =
            budget.living_energy + budget.carcass_energy + budget.dissipated_energy;
        assert!(grand_total > 0.0, "seeded world holds energy");
    }

    #[test]
    fn dominant_role_classification() {
        assert_eq!(dominant_role(&traits(0.8, 0.1)), "producers");
        assert_eq!(dominant_role(&traits(0.1, 0.8)), "consumers");
    }

    #[test]
    fn dominant_role_ties_to_producer() {
        // A tie (equal autotrophy and heterotrophy) reads as a producer.
        assert_eq!(dominant_role(&traits(0.5, 0.5)), "producers");
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
    fn new_clock_runs_and_emits_steps_as_time_accumulates() {
        let mut clock = RunClock::new();
        // At the default rate, one interval's worth of dt yields one step.
        let interval = 1.0 / clock.ticks_per_second();
        assert!(!clock.is_paused());
        assert_eq!(clock.advance(interval), 1);
    }

    #[test]
    fn paused_clock_emits_no_steps() {
        let mut clock = RunClock::new();
        clock.toggle_pause();
        assert!(clock.is_paused());
        // Even a large dt must not advance the simulation while paused.
        assert_eq!(clock.advance(100.0), 0);
    }

    #[test]
    fn toggle_pause_resumes_advancement() {
        let mut clock = RunClock::new();
        let interval = 1.0 / clock.ticks_per_second();
        clock.toggle_pause();
        assert!(clock.is_paused());
        clock.toggle_pause();
        assert!(!clock.is_paused());
        // After resuming, time advances the simulation again.
        assert_eq!(clock.advance(interval), 1);
    }

    #[test]
    fn speed_up_doubles_the_tick_rate() {
        let mut clock = RunClock::new();
        let before = clock.ticks_per_second();
        clock.speed_up();
        assert_eq!(clock.ticks_per_second(), before * 2.0);
    }

    #[test]
    fn slow_down_halves_the_tick_rate() {
        let mut clock = RunClock::new();
        let before = clock.ticks_per_second();
        clock.slow_down();
        assert_eq!(clock.ticks_per_second(), before / 2.0);
    }

    #[test]
    fn speed_up_is_clamped_to_the_ceiling() {
        let mut clock = RunClock::new();
        for _ in 0..50 {
            clock.speed_up();
        }
        assert_eq!(clock.ticks_per_second(), MAX_TICKS_PER_SECOND);
    }

    #[test]
    fn slow_down_is_clamped_to_the_floor() {
        let mut clock = RunClock::new();
        for _ in 0..50 {
            clock.slow_down();
        }
        assert_eq!(clock.ticks_per_second(), MIN_TICKS_PER_SECOND);
    }

    #[test]
    fn speeding_up_yields_more_steps_for_the_same_dt() {
        let mut clock = RunClock::new();
        // Default 1 TPS: one second of dt -> one step.
        clock.speed_up(); // now 2 TPS.
        assert_eq!(clock.advance(1.0), 2);
    }

    #[test]
    fn step_once_advances_exactly_one_tick_while_paused() {
        let mut clock = RunClock::new();
        clock.toggle_pause();
        assert_eq!(clock.step_once(), 1);
        // A second request is a separate, single step.
        assert_eq!(clock.step_once(), 1);
    }

    #[test]
    fn step_once_does_nothing_while_running() {
        let mut clock = RunClock::new();
        assert!(!clock.is_paused());
        assert_eq!(clock.step_once(), 0);
    }

    #[test]
    fn step_once_does_not_disturb_the_accumulator() {
        let mut clock = RunClock::new();
        let interval = 1.0 / clock.ticks_per_second();
        // Build up almost-but-not-quite a full interval of carried time.
        clock.toggle_pause();
        clock.toggle_pause();
        assert_eq!(clock.advance(interval * 0.9), 0);
        clock.toggle_pause();
        // Stepping by hand emits its single tick without consuming carried time.
        assert_eq!(clock.step_once(), 1);
        clock.toggle_pause();
        // The carried 0.9 interval is still there: a further 0.1 completes a tick.
        assert_eq!(clock.advance(interval * 0.1), 1);
    }

    #[test]
    fn clock_emits_no_step_before_interval_elapses() {
        // Default 1 TPS -> 1s interval.
        let mut clock = RunClock::new();
        assert_eq!(clock.advance(0.4), 0);
        assert_eq!(clock.advance(0.4), 0);
    }

    #[test]
    fn clock_emits_one_step_when_interval_reached() {
        let mut clock = RunClock::new();
        assert_eq!(clock.advance(0.6), 0);
        assert_eq!(clock.advance(0.6), 1);
    }

    #[test]
    fn clock_emits_multiple_steps_for_large_dt() {
        // 2 TPS -> 0.5s interval; 1.6s of dt clears three intervals.
        let mut clock = RunClock::new();
        clock.speed_up();
        assert_eq!(clock.advance(1.6), 3);
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

    fn agent_at(id: u64, position: (f32, f32)) -> explorers_sim::Agent {
        explorers_sim::Agent {
            id,
            position,
            reserve: 50.0,
            structure: 0.0,
            nutrient: 0.0,
            traits: TraitVector {
                photosynthetic_absorption: 1.0,
                heterotrophy: 0.0,
                mobility: 0.0,
                kappa: 0.0,
                fecundity: 0.0,
                asexual_propensity: 0.0,
                dispersal: 0.0,
            },
            contact_time: 0,
            wear: [0.0; explorers_sim::FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 0.0,
        }
    }

    fn agent_with_traits(id: u64, traits: TraitVector) -> explorers_sim::Agent {
        let mut a = agent_at(id, (0.0, 0.0));
        a.traits = traits;
        a
    }

    #[test]
    fn to_world_is_the_inverse_of_to_screen() {
        let viewport = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(400.0, 400.0));
        let view = WorldView::fit(200.0, viewport);
        for world_pos in [(0.0, 0.0), (50.0, -30.0), (-80.0, 90.0)] {
            let screen = view.to_screen(world_pos);
            let back = view.to_world(screen);
            assert!(
                (back.0 - world_pos.0).abs() < 1e-3 && (back.1 - world_pos.1).abs() < 1e-3,
                "round-trip {world_pos:?} -> {screen:?} -> {back:?}"
            );
        }
    }

    #[test]
    fn find_nearest_agent_returns_closest() {
        let agents = vec![agent_at(1, (10.0, 10.0)), agent_at(2, (20.0, 20.0))];
        let result = find_nearest_agent(&agents, (12.0, 12.0), 100.0);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn find_nearest_agent_returns_none_when_empty() {
        let agents: Vec<explorers_sim::Agent> = vec![];
        let result = find_nearest_agent(&agents, (0.0, 0.0), 100.0);
        assert_eq!(result, None);
    }

    #[test]
    fn find_nearest_agent_handles_toroidal_wrapping() {
        let agents = vec![agent_at(1, (5.0, 5.0)), agent_at(2, (95.0, 95.0))];
        // Click at (97,97) — toroidally closest to agent 2 at (95,95).
        let result = find_nearest_agent(&agents, (97.0, 97.0), 100.0);
        assert_eq!(result, Some(2));

        // Click at (99,99) — still closest to agent 2 at (95,95) (dist ~5.7)
        // versus agent 1 at (5,5) reached by wrapping (dist ~8.5).
        let result2 = find_nearest_agent(&agents, (99.0, 99.0), 100.0);
        assert_eq!(result2, Some(2));
    }

    #[test]
    fn clock_carries_remainder_across_calls() {
        // Default 1 TPS -> 1s interval.
        let mut clock = RunClock::new();
        assert_eq!(clock.advance(0.9), 0);
        // 0.9 + 0.2 = 1.1 -> one step, 0.1 carried.
        assert_eq!(clock.advance(0.2), 1);
        assert_eq!(clock.advance(0.85), 0);
        // 0.1 + 0.85 + 0.06 = 1.01 -> one step.
        assert_eq!(clock.advance(0.06), 1);
    }
}
