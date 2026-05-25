use std::fmt::Write as _;
use std::fs;
use std::time::Duration;

use bevy::prelude::*;
use bevy::camera::ScalingMode;
use explorers_sim::{InitialDistribution, TraitVector, World, WorldParameters, WorldRecipe};

#[derive(Resource)]
struct SimWorld(World);

#[derive(Resource)]
struct TickCount(u64);

#[derive(Component)]
struct AgentMarker(u64);

#[derive(Component)]
struct CarcassMarker(u64);

#[derive(Component)]
struct HudText;

#[derive(Resource)]
struct AgentMesh(Handle<Mesh>);

#[derive(Resource)]
struct CarcassMesh(Handle<Mesh>);

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut recipe_path: Option<String> = None;
    let mut fast_forward: u64 = 0;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--recipe" | "--scenario" => {
                i += 1;
                recipe_path = Some(args[i].clone());
            }
            "--fast-forward" => {
                i += 1;
                fast_forward = args[i].parse().unwrap();
            }
            "--help" | "-h" => {
                eprintln!("Usage: explorers-app [OPTIONS]");
                eprintln!();
                eprintln!("Options:");
                eprintln!("  --recipe PATH        Load world recipe from JSON file");
                eprintln!("  --scenario PATH      Load scenario from JSON file (same format as recipe,");
                eprintln!("                       but may include explicit agents list)");
                eprintln!("  --fast-forward N     Advance simulation N ticks before rendering");
                eprintln!("  --help, -h           Show this help");
                return;
            }
            other => {
                eprintln!("Unknown argument: {other}");
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let recipe = match recipe_path {
        Some(path) => {
            let contents = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("Failed to read recipe file {path}: {e}"));
            let recipe: WorldRecipe = serde_json::from_str(&contents)
                .unwrap_or_else(|e| panic!("Failed to parse recipe file {path}: {e}"));
            eprintln!("Loaded recipe from {path}");
            recipe
        }
        None => WorldRecipe {
            parameters: WorldParameters {
                solar_flux_magnitude: 1.0,
                consumption_efficiency: 0.5,
                decomposition_efficiency: 0.5,
                reproduction_efficiency: 0.7,
                base_metabolic_rate: 0.1,
                movement_cost_coefficient: 0.05,
                sensing_cost_coefficient: 0.02,
                reproduction_energy_threshold: 50.0,
                mutation_rate: 0.1,
                mutation_magnitude: 0.05,
                contact_radius: 5.0,
                world_extent: 200.0,
                initial_population_size: 3,
                light_competition_radius: 20.0,
                photo_maintenance_cost: 0.01,
                consumption_maintenance_cost: 0.01,
                scavenging_maintenance_cost: 0.01,
                spatial_decay_rate: 0.5,
            },
            initial_distribution: Some(InitialDistribution {
                mean_traits: TraitVector {
                    photosynthetic_absorption: 0.5,
                    consumption_rate: 0.3,
                    scavenging_rate: 0.2,
                    mobility: 0.4,
                    chemotaxis_sensitivity: 0.3,
                    mate_selectivity: 0.5,
                    sensing_range: 0.4,
                    reproductive_investment: 0.3,
                },
                trait_covariance: 0.1,
                initial_cluster_count: 1,
                initial_energy_per_agent: 100.0,
            }),
            agents: None,
            max_ticks: 100,
        },
    };

    let ticks = if fast_forward > 0 { fast_forward } else { recipe.max_ticks };
    let seed: u64 = rand::random();
    let mut world = World::from_recipe(&recipe, seed);

    eprintln!("Fast-forwarding {ticks} ticks...");
    for _ in 0..ticks {
        world.step();
    }
    eprintln!("Fast-forward complete. {} agents alive.", world.agents().len());

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Explorers".into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(SimWorld(world))
        .insert_resource(TickCount(ticks))
        .add_systems(Startup, (setup_camera, setup_meshes, setup_grid, setup_hud, configure_timestep))
        .add_systems(FixedUpdate, (step_simulation, reconcile_entities).chain())
        .add_systems(Update, (tick_rate_control, update_hud))
        .run();
}

fn setup_camera(mut commands: Commands, sim: Res<SimWorld>) {
    let extent = sim.0.params().world_extent;
    let center = extent / 2.0;
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: extent,
                min_height: extent,
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(center, center, 0.0),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::input::ButtonInput;

    #[test]
    fn pure_producer_maps_to_green() {
        let traits = TraitVector {
            photosynthetic_absorption: 1.0,
            consumption_rate: 0.0,
            scavenging_rate: 0.0,
            mobility: 0.0,
            chemotaxis_sensitivity: 0.0,
            mate_selectivity: 0.0,
            sensing_range: 0.0,
            reproductive_investment: 0.0,
        };
        let color = trophic_color(&traits, 100.0);
        let rgba = color.to_srgba();
        assert!(rgba.green > 0.9, "green channel should be high, got {}", rgba.green);
        assert!(rgba.red < 0.1, "red channel should be low, got {}", rgba.red);
        assert!(rgba.blue < 0.1, "blue channel should be low, got {}", rgba.blue);
    }

    #[test]
    fn pure_consumer_maps_to_red() {
        let traits = TraitVector {
            photosynthetic_absorption: 0.0,
            consumption_rate: 1.0,
            scavenging_rate: 0.0,
            mobility: 0.0,
            chemotaxis_sensitivity: 0.0,
            mate_selectivity: 0.0,
            sensing_range: 0.0,
            reproductive_investment: 0.0,
        };
        let color = trophic_color(&traits, 100.0);
        let rgba = color.to_srgba();
        assert!(rgba.red > 0.9, "red channel should be high, got {}", rgba.red);
        assert!(rgba.green < 0.1, "green channel should be low, got {}", rgba.green);
        assert!(rgba.blue < 0.1, "blue channel should be low, got {}", rgba.blue);
    }

    #[test]
    fn pure_decomposer_maps_to_blue() {
        let traits = TraitVector {
            photosynthetic_absorption: 0.0,
            consumption_rate: 0.0,
            scavenging_rate: 1.0,
            mobility: 0.0,
            chemotaxis_sensitivity: 0.0,
            mate_selectivity: 0.0,
            sensing_range: 0.0,
            reproductive_investment: 0.0,
        };
        let color = trophic_color(&traits, 100.0);
        let rgba = color.to_srgba();
        assert!(rgba.blue > 0.9, "blue channel should be high, got {}", rgba.blue);
        assert!(rgba.red < 0.1, "red channel should be low, got {}", rgba.red);
        assert!(rgba.green < 0.1, "green channel should be low, got {}", rgba.green);
    }

    #[test]
    fn low_energy_dims_color() {
        let traits = TraitVector {
            photosynthetic_absorption: 1.0,
            consumption_rate: 0.0,
            scavenging_rate: 0.0,
            mobility: 0.0,
            chemotaxis_sensitivity: 0.0,
            mate_selectivity: 0.0,
            sensing_range: 0.0,
            reproductive_investment: 0.0,
        };
        let bright = trophic_color(&traits, 100.0).to_srgba();
        let dim = trophic_color(&traits, 10.0).to_srgba();
        assert!(dim.green < bright.green, "low energy should dim: {:.2} vs {:.2}", dim.green, bright.green);
        assert!(dim.green > 0.0, "should still be visible at low energy");
    }

    #[test]
    fn dominant_role_classification() {
        let producer = TraitVector {
            photosynthetic_absorption: 0.8, consumption_rate: 0.1, scavenging_rate: 0.1,
            mobility: 0.0, chemotaxis_sensitivity: 0.0, mate_selectivity: 0.0,
            sensing_range: 0.0, reproductive_investment: 0.0,
        };
        let consumer = TraitVector {
            photosynthetic_absorption: 0.1, consumption_rate: 0.8, scavenging_rate: 0.1,
            mobility: 0.0, chemotaxis_sensitivity: 0.0, mate_selectivity: 0.0,
            sensing_range: 0.0, reproductive_investment: 0.0,
        };
        let decomposer = TraitVector {
            photosynthetic_absorption: 0.1, consumption_rate: 0.1, scavenging_rate: 0.8,
            mobility: 0.0, chemotaxis_sensitivity: 0.0, mate_selectivity: 0.0,
            sensing_range: 0.0, reproductive_investment: 0.0,
        };
        assert_eq!(dominant_role(&producer), "producers");
        assert_eq!(dominant_role(&consumer), "consumers");
        assert_eq!(dominant_role(&decomposer), "decomposers");
    }

    fn press_key(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
    }

    fn release_key(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(key);
    }

    fn timestep_secs(app: &App) -> f64 {
        app.world().resource::<Time<Fixed>>().timestep().as_secs_f64()
    }

    fn make_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<Time<Virtual>>();
        app.add_systems(Startup, configure_timestep);
        app.add_systems(Update, tick_rate_control);
        app.update();
        app
    }

    #[test]
    fn default_timestep_is_100ms() {
        let app = make_app();
        let timestep = app.world().resource::<Time<Fixed>>().timestep();
        assert_eq!(timestep, Duration::from_millis(100));
    }

    #[test]
    fn up_arrow_halves_timestep() {
        let mut app = make_app();
        press_key(&mut app, KeyCode::ArrowUp);
        app.update();
        assert_eq!(timestep_secs(&app), 0.05);
    }

    #[test]
    fn down_arrow_doubles_timestep() {
        let mut app = make_app();
        press_key(&mut app, KeyCode::ArrowDown);
        app.update();
        assert_eq!(timestep_secs(&app), 0.2);
    }

    #[test]
    fn speed_up_has_floor_at_10ms() {
        let mut app = make_app();
        // Press up enough times to hit the floor: 100 -> 50 -> 25 -> 12.5 -> 10 (clamped)
        for _ in 0..10 {
            press_key(&mut app, KeyCode::ArrowUp);
            app.update();
            release_key(&mut app, KeyCode::ArrowUp);
            app.update();
        }
        assert_eq!(app.world().resource::<Time<Fixed>>().timestep(), Duration::from_millis(10));
    }

    #[test]
    fn slow_down_has_ceiling_at_2s() {
        let mut app = make_app();
        // Press down enough times to hit the ceiling: 100 -> 200 -> 400 -> 800 -> 1600 -> 2000 (clamped)
        for _ in 0..10 {
            press_key(&mut app, KeyCode::ArrowDown);
            app.update();
            release_key(&mut app, KeyCode::ArrowDown);
            app.update();
        }
        assert_eq!(app.world().resource::<Time<Fixed>>().timestep(), Duration::from_millis(2000));
    }

    fn clear_inputs(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
    }

    #[test]
    fn spacebar_toggles_pause() {
        let mut app = make_app();

        press_key(&mut app, KeyCode::Space);
        app.update();
        assert!(app.world().resource::<Time<Virtual>>().is_paused());

        release_key(&mut app, KeyCode::Space);
        clear_inputs(&mut app);
        press_key(&mut app, KeyCode::Space);
        app.update();
        assert!(!app.world().resource::<Time<Virtual>>().is_paused());
    }

    fn test_world(world_extent: f32) -> World {
        World::new(
            WorldParameters {
                solar_flux_magnitude: 1.0,
                consumption_efficiency: 0.5,
                decomposition_efficiency: 0.5,
                reproduction_efficiency: 0.7,
                base_metabolic_rate: 0.1,
                movement_cost_coefficient: 0.05,
                sensing_cost_coefficient: 0.02,
                reproduction_energy_threshold: 50.0,
                mutation_rate: 0.1,
                mutation_magnitude: 0.05,
                contact_radius: 5.0,
                world_extent,
                initial_population_size: 0,
                light_competition_radius: 1000.0,
                photo_maintenance_cost: 0.0,
                consumption_maintenance_cost: 0.0,
                scavenging_maintenance_cost: 0.0,
spatial_decay_rate: 0.5,

            },
            InitialDistribution {
                mean_traits: TraitVector {
                    photosynthetic_absorption: 0.5,
                    consumption_rate: 0.3,
                    scavenging_rate: 0.2,
                    mobility: 0.4,
                    chemotaxis_sensitivity: 0.3,
                    mate_selectivity: 0.5,
                    sensing_range: 0.4,
                    reproductive_investment: 0.3,
                },
                trait_covariance: 0.1,
                initial_cluster_count: 1,
                initial_energy_per_agent: 100.0,
            },
            42,
        )
    }

    #[test]
    fn camera_is_centered_on_world() {
        let mut app = App::new();
        app.insert_resource(SimWorld(test_world(200.0)));
        let _ = app.world_mut().run_system_once(setup_camera);

        let transform = app
            .world_mut()
            .query::<&Transform>()
            .single(app.world())
            .unwrap();

        assert_eq!(transform.translation.x, 100.0);
        assert_eq!(transform.translation.y, 100.0);
    }

    #[test]
    fn camera_projection_fits_world_extent() {
        let mut app = App::new();
        app.insert_resource(SimWorld(test_world(300.0)));
        let _ = app.world_mut().run_system_once(setup_camera);

        let projection = app
            .world_mut()
            .query::<&Projection>()
            .single(app.world())
            .unwrap();

        match projection {
            Projection::Orthographic(ortho) => {
                assert!(
                    matches!(
                        ortho.scaling_mode,
                        ScalingMode::AutoMin {
                            min_width,
                            min_height,
                        } if min_width == 300.0 && min_height == 300.0
                    ),
                    "expected AutoMin with world_extent 300, got {:?}",
                    ortho.scaling_mode,
                );
            }
            other => panic!("expected orthographic projection, got {:?}", other),
        }
    }
}

fn setup_meshes(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(AgentMesh(meshes.add(Circle::new(1.0))));
    commands.insert_resource(CarcassMesh(meshes.add(RegularPolygon::new(1.0, 4))));
}

fn reconcile_entities(
    mut commands: Commands,
    sim: Res<SimWorld>,
    agent_mesh: Res<AgentMesh>,
    carcass_mesh: Res<CarcassMesh>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut agents_query: Query<(Entity, &AgentMarker, &mut Transform, &MeshMaterial2d<ColorMaterial>)>,
    mut carcasses_query: Query<(Entity, &CarcassMarker, &mut Transform, &MeshMaterial2d<ColorMaterial>), Without<AgentMarker>>,
) {
    let living: std::collections::HashSet<u64> =
        sim.0.agents().iter().map(|a| a.id).collect();

    for (entity, marker, _, _) in &agents_query {
        if !living.contains(&marker.0) {
            commands.entity(entity).despawn();
        }
    }

    let existing_agents: std::collections::HashSet<u64> =
        agents_query.iter().map(|(_, m, _, _)| m.0).collect();

    for agent in sim.0.agents() {
        if !existing_agents.contains(&agent.id) {
            let color = trophic_color(&agent.traits, agent.energy);
            commands.spawn((
                Mesh2d(agent_mesh.0.clone()),
                MeshMaterial2d(materials.add(ColorMaterial::from_color(color))),
                Transform::from_xyz(agent.position.0, agent.position.1, 1.0)
                    .with_scale(Vec3::splat(AGENT_RADIUS)),
                AgentMarker(agent.id),
            ));
        }
    }

    for (_, marker, mut transform, material_handle) in &mut agents_query {
        if let Some(agent) = sim.0.agents().iter().find(|a| a.id == marker.0) {
            transform.translation.x = agent.position.0;
            transform.translation.y = agent.position.1;
            if let Some(mat) = materials.get_mut(&material_handle.0) {
                mat.color = trophic_color(&agent.traits, agent.energy);
            }
        }
    }

    let sim_carcass_ids: std::collections::HashSet<u64> =
        sim.0.carcasses().iter().map(|c| c.id).collect();

    for (entity, marker, _, _) in &carcasses_query {
        if !sim_carcass_ids.contains(&marker.0) {
            commands.entity(entity).despawn();
        }
    }

    let existing_carcasses: std::collections::HashSet<u64> =
        carcasses_query.iter().map(|(_, m, _, _)| m.0).collect();

    for carcass in sim.0.carcasses() {
        if !existing_carcasses.contains(&carcass.id) {
            let brightness = (carcass.energy.max(0.0) / 100.0).clamp(0.2, 1.0);
            let gray = 0.5 * brightness;
            commands.spawn((
                Mesh2d(carcass_mesh.0.clone()),
                MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::srgb(gray, gray, gray)))),
                Transform::from_xyz(carcass.position.0, carcass.position.1, 0.5)
                    .with_scale(Vec3::splat(AGENT_RADIUS)),
                CarcassMarker(carcass.id),
            ));
        }
    }

    for (_, marker, _, material_handle) in &mut carcasses_query {
        if let Some(carcass) = sim.0.carcasses().iter().find(|c| c.id == marker.0) {
            if let Some(mat) = materials.get_mut(&material_handle.0) {
                let brightness = (carcass.energy.max(0.0) / 100.0).clamp(0.2, 1.0);
                let gray = 0.5 * brightness;
                mat.color = Color::srgb(gray, gray, gray);
            }
        }
    }
}

fn trophic_color(traits: &TraitVector, energy: f32) -> Color {
    let brightness = (energy.max(0.0) / 100.0).clamp(0.2, 1.0);
    Color::srgb(
        traits.consumption_rate.clamp(0.0, 1.0) * brightness,
        traits.photosynthetic_absorption.clamp(0.0, 1.0) * brightness,
        traits.scavenging_rate.clamp(0.0, 1.0) * brightness,
    )
}

const AGENT_RADIUS: f32 = 1.5;

fn dominant_role(traits: &TraitVector) -> &'static str {
    let p = traits.photosynthetic_absorption;
    let c = traits.consumption_rate;
    let s = traits.scavenging_rate;
    if p >= c && p >= s {
        "producers"
    } else if c >= s {
        "consumers"
    } else {
        "decomposers"
    }
}

fn configure_timestep(mut time: ResMut<Time<Fixed>>) {
    time.set_timestep(Duration::from_millis(100));
}

fn tick_rate_control(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut time: ResMut<Time<Fixed>>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        let new = time.timestep().div_f64(2.0).max(Duration::from_millis(10));
        time.set_timestep(new);
        eprintln!("Tick rate: {:.1} tps ({:.0}ms)", 1.0 / new.as_secs_f64(), new.as_secs_f64() * 1000.0);
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        let new = time.timestep().mul_f64(2.0).min(Duration::from_millis(2000));
        time.set_timestep(new);
        eprintln!("Tick rate: {:.1} tps ({:.0}ms)", 1.0 / new.as_secs_f64(), new.as_secs_f64() * 1000.0);
    }
    if keyboard.just_pressed(KeyCode::Space) {
        if virtual_time.is_paused() {
            virtual_time.unpause();
            eprintln!("Simulation resumed");
        } else {
            virtual_time.pause();
            eprintln!("Simulation paused");
        }
    }
}

fn step_simulation(mut sim: ResMut<SimWorld>, mut tick_count: ResMut<TickCount>) {
    sim.0.step();
    tick_count.0 += 1;
}

fn setup_grid(
    mut commands: Commands,
    sim: Res<SimWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let extent = sim.0.params().world_extent;
    let divisions = 5;
    let spacing = extent / divisions as f32;
    let grid_color = Color::srgba(1.0, 1.0, 1.0, 0.05);
    let line_material = materials.add(ColorMaterial::from_color(grid_color));
    let line_thickness = 0.5;

    let vertical_mesh = meshes.add(Rectangle::new(line_thickness, extent));
    let horizontal_mesh = meshes.add(Rectangle::new(extent, line_thickness));

    for i in 0..=divisions {
        let pos = i as f32 * spacing;
        commands.spawn((
            Mesh2d(vertical_mesh.clone()),
            MeshMaterial2d(line_material.clone()),
            Transform::from_xyz(pos, extent / 2.0, 0.0),
        ));
        commands.spawn((
            Mesh2d(horizontal_mesh.clone()),
            MeshMaterial2d(line_material.clone()),
            Transform::from_xyz(extent / 2.0, pos, 0.0),
        ));
    }
}

fn setup_hud(mut commands: Commands) {
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        HudText,
    ));
}

fn update_hud(
    sim: Res<SimWorld>,
    tick_count: Res<TickCount>,
    time: Res<Time<Fixed>>,
    virtual_time: Res<Time<Virtual>>,
    mut query: Query<&mut Text, With<HudText>>,
) {
    let agents = sim.0.agents();
    let total = agents.len();
    let mut producers = 0usize;
    let mut consumers = 0usize;
    let mut decomposers = 0usize;
    for agent in agents {
        match dominant_role(&agent.traits) {
            "producers" => producers += 1,
            "consumers" => consumers += 1,
            _ => decomposers += 1,
        }
    }

    let tps = 1.0 / time.timestep().as_secs_f64();
    let paused = virtual_time.is_paused();

    let mut hud = String::new();
    let _ = write!(hud, "Tick {}", tick_count.0);
    let _ = write!(hud, " | {total} agents ({producers} P / {consumers} C / {decomposers} D)");
    let _ = write!(hud, " | {tps:.0} tps");
    if paused {
        let _ = write!(hud, " | PAUSED");
    }

    for mut text in &mut query {
        **text = hud.clone();
    }
}

