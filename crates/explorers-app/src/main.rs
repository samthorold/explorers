use std::fs;

use std::time::Duration;

use bevy::prelude::*;
use bevy::camera::ScalingMode;
use explorers_sim::{InitialDistribution, TraitVector, World, WorldParameters, WorldRecipe};

#[derive(Resource)]
struct SimWorld(World);

#[derive(Component)]
struct AgentMarker(u64);

#[derive(Component)]
struct CarcassMarker(u64);

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
            "--recipe" => {
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
            },
            initial_distribution: InitialDistribution {
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
            max_ticks: 100,
        },
    };

    let ticks = if fast_forward > 0 { fast_forward } else { recipe.max_ticks };
    let seed: u64 = rand::random();
    let mut world = World::new(recipe.parameters, recipe.initial_distribution, seed);

    eprintln!("Fast-forwarding {ticks} ticks...");
    for _ in 0..ticks {
        world.step();
    }
    eprintln!("Fast-forward complete. {} agents alive.", world.agents().len());

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(SimWorld(world))
        .add_systems(Startup, (setup_camera, setup_meshes, configure_timestep))
        .add_systems(FixedUpdate, (step_simulation, reconcile_entities).chain())
        .add_systems(Update, tick_rate_control)
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
        let color = trophic_color(&traits);
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
        let color = trophic_color(&traits);
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
        let color = trophic_color(&traits);
        let rgba = color.to_srgba();
        assert!(rgba.blue > 0.9, "blue channel should be high, got {}", rgba.blue);
        assert!(rgba.red < 0.1, "red channel should be low, got {}", rgba.red);
        assert!(rgba.green < 0.1, "green channel should be low, got {}", rgba.green);
    }

    #[test]
    fn hybrid_traits_produce_blended_color() {
        let traits = TraitVector {
            photosynthetic_absorption: 0.6,
            consumption_rate: 0.4,
            scavenging_rate: 0.2,
            mobility: 0.0,
            chemotaxis_sensitivity: 0.0,
            mate_selectivity: 0.0,
            sensing_range: 0.0,
            reproductive_investment: 0.0,
        };
        let color = trophic_color(&traits);
        let rgba = color.to_srgba();
        assert!((rgba.red - 0.4).abs() < 0.01, "red should be ~0.4, got {}", rgba.red);
        assert!((rgba.green - 0.6).abs() < 0.01, "green should be ~0.6, got {}", rgba.green);
        assert!((rgba.blue - 0.2).abs() < 0.01, "blue should be ~0.2, got {}", rgba.blue);
    }

    #[test]
    fn zero_energy_has_minimum_visible_scale() {
        let scale = energy_to_scale(0.0);
        assert!(scale > 0.0, "scale should be positive at zero energy, got {scale}");
    }

    #[test]
    fn energy_to_scale_increases_monotonically() {
        let low = energy_to_scale(10.0);
        let mid = energy_to_scale(50.0);
        let high = energy_to_scale(100.0);
        assert!(mid > low, "mid ({mid}) should exceed low ({low})");
        assert!(high > mid, "high ({high}) should exceed mid ({mid})");
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

const CARCASS_COLOR: Color = Color::srgb(0.5, 0.5, 0.5);

fn reconcile_entities(
    mut commands: Commands,
    sim: Res<SimWorld>,
    agent_mesh: Res<AgentMesh>,
    carcass_mesh: Res<CarcassMesh>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut agents_query: Query<(Entity, &AgentMarker, &mut Transform, &MeshMaterial2d<ColorMaterial>)>,
    mut carcasses_query: Query<(Entity, &CarcassMarker, &mut Transform), Without<AgentMarker>>,
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
            let color = trophic_color(&agent.traits);
            let scale = energy_to_scale(agent.energy);
            commands.spawn((
                Mesh2d(agent_mesh.0.clone()),
                MeshMaterial2d(materials.add(ColorMaterial::from_color(color))),
                Transform::from_xyz(agent.position.0, agent.position.1, 0.0)
                    .with_scale(Vec3::splat(scale)),
                AgentMarker(agent.id),
            ));
        }
    }

    for (_, marker, mut transform, material_handle) in &mut agents_query {
        if let Some(agent) = sim.0.agents().iter().find(|a| a.id == marker.0) {
            transform.translation.x = agent.position.0;
            transform.translation.y = agent.position.1;
            let scale = energy_to_scale(agent.energy);
            transform.scale = Vec3::splat(scale);
            if let Some(mat) = materials.get_mut(&material_handle.0) {
                mat.color = trophic_color(&agent.traits);
            }
        }
    }

    let sim_carcass_ids: std::collections::HashSet<u64> =
        sim.0.carcasses().iter().map(|c| c.id).collect();

    for (entity, marker, _) in &carcasses_query {
        if !sim_carcass_ids.contains(&marker.0) {
            commands.entity(entity).despawn();
        }
    }

    let existing_carcasses: std::collections::HashSet<u64> =
        carcasses_query.iter().map(|(_, m, _)| m.0).collect();

    for carcass in sim.0.carcasses() {
        if !existing_carcasses.contains(&carcass.id) {
            let scale = energy_to_scale(carcass.energy);
            commands.spawn((
                Mesh2d(carcass_mesh.0.clone()),
                MeshMaterial2d(materials.add(ColorMaterial::from_color(CARCASS_COLOR))),
                Transform::from_xyz(carcass.position.0, carcass.position.1, 0.0)
                    .with_scale(Vec3::splat(scale)),
                CarcassMarker(carcass.id),
            ));
        }
    }

    for (_, marker, mut transform) in &mut carcasses_query {
        if let Some(carcass) = sim.0.carcasses().iter().find(|c| c.id == marker.0) {
            let scale = energy_to_scale(carcass.energy);
            transform.scale = Vec3::splat(scale);
        }
    }
}

fn trophic_color(traits: &TraitVector) -> Color {
    Color::srgb(
        traits.consumption_rate.clamp(0.0, 1.0),
        traits.photosynthetic_absorption.clamp(0.0, 1.0),
        traits.scavenging_rate.clamp(0.0, 1.0),
    )
}

const BASE_RADIUS: f32 = 4.0;
const ENERGY_SCALE_FACTOR: f32 = 0.1;

fn energy_to_scale(energy: f32) -> f32 {
    BASE_RADIUS + energy.max(0.0) * ENERGY_SCALE_FACTOR
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

fn step_simulation(mut sim: ResMut<SimWorld>) {
    sim.0.step();
}

