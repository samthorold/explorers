use std::fs;
use std::time::Duration;

use bevy::prelude::*;
use bevy::camera::{ScalingMode, Viewport};
use bevy_egui::{EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass};
use explorers_sim::{InitialDistribution, TraitVector, World, WorldParameters, WorldRecipe};

#[derive(Resource)]
struct SimWorld(World);

#[derive(Resource)]
struct TickCount(u64);

#[derive(Component)]
struct AgentMarker(u64);

#[derive(Component)]
struct CarcassMarker(u64);

#[derive(Resource)]
struct AgentMesh(Handle<Mesh>);

#[derive(Resource)]
struct CarcassMesh(Handle<Mesh>);

#[derive(Resource, Default)]
struct SelectedAgent(Option<u64>);

#[derive(Resource)]
struct DebugPanelOpen(bool);

impl Default for DebugPanelOpen {
    fn default() -> Self {
        Self(true)
    }
}

const DEBUG_PANEL_WIDTH: f32 = 300.0;

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
            nutrient_absorption_maintenance_cost: 0.0,
            initial_nutrient_pool: 0.0,
            growth_efficiency: 0.0,
            },
            initial_distribution: Some(InitialDistribution {
                mean_traits: TraitVector {
                    photosynthetic_absorption: 0.5,
                    consumption_rate: 0.3,
                    scavenging_rate: 0.2,
                nutrient_absorption: 0.0,
                    mobility: 0.4,
                    chemotaxis_sensitivity: 0.3,
                    mate_selectivity: 0.5,
                    sensing_range: 0.4,
                    reproductive_investment: 0.3, fecundity: 0.0,
                },
                trait_covariance: 0.1,
                initial_cluster_count: 1,
                initial_energy_per_agent: 100.0,
            }),
            agents: None,
            max_ticks: 100,
        },
    };

    let seed: u64 = rand::random();
    let mut world = World::from_recipe(&recipe, seed);

    if fast_forward > 0 {
        eprintln!("Fast-forwarding {fast_forward} ticks...");
        for _ in 0..fast_forward {
            world.step();
        }
        eprintln!("Fast-forward complete. {} agents alive.", world.agents().len());
    }

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Explorers".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .insert_resource(EguiGlobalSettings {
            enable_absorb_bevy_input_system: false,
            ..default()
        })
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(SimWorld(world))
        .insert_resource(TickCount(fast_forward))
        .init_resource::<SelectedAgent>()
        .init_resource::<DebugPanelOpen>()
        .add_systems(Startup, (setup_camera, setup_meshes, setup_grid, configure_timestep))
        .add_systems(FixedUpdate, (step_simulation, reconcile_entities).chain())
        .add_systems(EguiPrimaryContextPass, debug_panel_ui)
        .add_systems(Update, (
            tick_rate_control,
            click_to_inspect.run_if(not(bevy_egui::input::egui_wants_any_pointer_input)),
        ).chain())
        .run();
}

fn setup_camera(mut commands: Commands, sim: Res<SimWorld>) {
    let extent = sim.0.params().world_extent;
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: extent,
                min_height: extent,
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(0.0, 0.0, 0.0),
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
                nutrient_absorption: 0.0,
            mobility: 0.0,
            chemotaxis_sensitivity: 0.0,
            mate_selectivity: 0.0,
            sensing_range: 0.0,
            reproductive_investment: 0.0, fecundity: 0.0,
        };
        let color = trophic_color(&traits, 100.0);
        let rgba = color.to_srgba();
        assert!(rgba.green > 0.9, "green channel should be high, got {}", rgba.green);
        assert!(rgba.red < 0.2, "red channel should be low, got {}", rgba.red);
        assert!(rgba.blue < 0.2, "blue channel should be low, got {}", rgba.blue);
    }

    #[test]
    fn pure_consumer_maps_to_red() {
        let traits = TraitVector {
            photosynthetic_absorption: 0.0,
            consumption_rate: 1.0,
            scavenging_rate: 0.0,
                nutrient_absorption: 0.0,
            mobility: 0.0,
            chemotaxis_sensitivity: 0.0,
            mate_selectivity: 0.0,
            sensing_range: 0.0,
            reproductive_investment: 0.0, fecundity: 0.0,
        };
        let color = trophic_color(&traits, 100.0);
        let rgba = color.to_srgba();
        assert!(rgba.red > 0.9, "red channel should be high, got {}", rgba.red);
        assert!(rgba.green < 0.2, "green channel should be low, got {}", rgba.green);
        assert!(rgba.blue < 0.2, "blue channel should be low, got {}", rgba.blue);
    }

    #[test]
    fn pure_decomposer_maps_to_blue() {
        let traits = TraitVector {
            photosynthetic_absorption: 0.0,
            consumption_rate: 0.0,
            scavenging_rate: 1.0,
                nutrient_absorption: 0.0,
            mobility: 0.0,
            chemotaxis_sensitivity: 0.0,
            mate_selectivity: 0.0,
            sensing_range: 0.0,
            reproductive_investment: 0.0, fecundity: 0.0,
        };
        let color = trophic_color(&traits, 100.0);
        let rgba = color.to_srgba();
        assert!(rgba.blue > 0.9, "blue channel should be high, got {}", rgba.blue);
        assert!(rgba.red < 0.2, "red channel should be low, got {}", rgba.red);
        assert!(rgba.green < 0.2, "green channel should be low, got {}", rgba.green);
    }

    #[test]
    fn low_reserve_dims_color() {
        let traits = TraitVector {
            photosynthetic_absorption: 1.0,
            consumption_rate: 0.0,
            scavenging_rate: 0.0,
                nutrient_absorption: 0.0,
            mobility: 0.0,
            chemotaxis_sensitivity: 0.0,
            mate_selectivity: 0.0,
            sensing_range: 0.0,
            reproductive_investment: 0.0, fecundity: 0.0,
        };
        let bright = trophic_color(&traits, 100.0).to_srgba();
        let dim = trophic_color(&traits, 10.0).to_srgba();
        assert!(dim.green < bright.green, "low energy should dim: {:.2} vs {:.2}", dim.green, bright.green);
        assert!(dim.green > 0.0, "should still be visible at low energy");
    }

    #[test]
    fn brightness_maps_to_reserve_not_total_energy() {
        let traits = TraitVector {
            photosynthetic_absorption: 1.0,
            consumption_rate: 0.0,
            scavenging_rate: 0.0,
            nutrient_absorption: 0.0,
            mobility: 0.0,
            chemotaxis_sensitivity: 0.0,
            mate_selectivity: 0.0,
            sensing_range: 0.0,
            reproductive_investment: 0.0,
            fecundity: 0.0,
        };
        // Same total energy (100), but different reserve values
        let high_reserve = trophic_color(&traits, 80.0).to_srgba(); // reserve=80
        let low_reserve = trophic_color(&traits, 20.0).to_srgba();  // reserve=20
        assert!(
            high_reserve.green > low_reserve.green,
            "higher reserve should be brighter: {:.3} vs {:.3}",
            high_reserve.green,
            low_reserve.green
        );
    }

    #[test]
    fn dominant_role_classification() {
        let producer = TraitVector {
            photosynthetic_absorption: 0.8, consumption_rate: 0.1, scavenging_rate: 0.1,
                nutrient_absorption: 0.0,
            mobility: 0.0, chemotaxis_sensitivity: 0.0, mate_selectivity: 0.0,
            sensing_range: 0.0, reproductive_investment: 0.0, fecundity: 0.0,
        };
        let consumer = TraitVector {
            photosynthetic_absorption: 0.1, consumption_rate: 0.8, scavenging_rate: 0.1,
                nutrient_absorption: 0.0,
            mobility: 0.0, chemotaxis_sensitivity: 0.0, mate_selectivity: 0.0,
            sensing_range: 0.0, reproductive_investment: 0.0, fecundity: 0.0,
        };
        let decomposer = TraitVector {
            photosynthetic_absorption: 0.1, consumption_rate: 0.1, scavenging_rate: 0.8,
                nutrient_absorption: 0.0,
            mobility: 0.0, chemotaxis_sensitivity: 0.0, mate_selectivity: 0.0,
            sensing_range: 0.0, reproductive_investment: 0.0, fecundity: 0.0,
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
    fn default_timestep_is_1s() {
        let app = make_app();
        let timestep = app.world().resource::<Time<Fixed>>().timestep();
        assert_eq!(timestep, Duration::from_millis(1000));
    }

    #[test]
    fn up_arrow_halves_timestep() {
        let mut app = make_app();
        press_key(&mut app, KeyCode::ArrowUp);
        app.update();
        assert_eq!(timestep_secs(&app), 0.5);
    }

    #[test]
    fn down_arrow_doubles_timestep() {
        let mut app = make_app();
        press_key(&mut app, KeyCode::ArrowDown);
        app.update();
        assert_eq!(timestep_secs(&app), 2.0);
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
                nutrient_absorption_maintenance_cost: 0.0,
                initial_nutrient_pool: 0.0,
            growth_efficiency: 0.0,
            },
            InitialDistribution {
                mean_traits: TraitVector {
                    photosynthetic_absorption: 0.5,
                    consumption_rate: 0.3,
                    scavenging_rate: 0.2,
                nutrient_absorption: 0.0,
                    mobility: 0.4,
                    chemotaxis_sensitivity: 0.3,
                    mate_selectivity: 0.5,
                    sensing_range: 0.4,
                    reproductive_investment: 0.3, fecundity: 0.0,
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

        assert_eq!(transform.translation.x, 0.0);
        assert_eq!(transform.translation.y, 0.0);
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

    #[test]
    fn energy_budget_sums_correctly() {
        let mut world = test_world(100.0);
        // Add some agents with known energy
        world.add_agent(explorers_sim::Agent {
            id: 0, position: (0.0, 0.0), reserve: 30.0,
 structure: 0.0,
            nutrient: 0.0,
            traits: TraitVector {
                photosynthetic_absorption: 1.0, consumption_rate: 0.0,
                scavenging_rate: 0.0, nutrient_absorption: 0.0, mobility: 0.0, chemotaxis_sensitivity: 0.0,
                mate_selectivity: 0.0, sensing_range: 0.0, reproductive_investment: 0.0, fecundity: 0.0,
            },
            contact_time: 0,
        });
        world.add_agent(explorers_sim::Agent {
            id: 0, position: (10.0, 10.0), reserve: 20.0,
 structure: 0.0,
            nutrient: 0.0,
            traits: TraitVector {
                photosynthetic_absorption: 1.0, consumption_rate: 0.0,
                scavenging_rate: 0.0, nutrient_absorption: 0.0, mobility: 0.0, chemotaxis_sensitivity: 0.0,
                mate_selectivity: 0.0, sensing_range: 0.0, reproductive_investment: 0.0, fecundity: 0.0,
            },
            contact_time: 0,
        });
        world.add_carcass(explorers_sim::Carcass {
            id: 99, position: (5.0, 5.0), energy: 15.0,
            nutrient: 0.0,
        });

        let budget = compute_energy_budget(&world);
        assert_eq!(budget.living_energy, 50.0);
        assert_eq!(budget.carcass_energy, 15.0);
        assert_eq!(budget.dissipated_energy, 0.0);
    }

    #[test]
    fn energy_budget_handles_empty_world() {
        let world = test_world(100.0);
        let budget = compute_energy_budget(&world);
        assert_eq!(budget.living_energy, 0.0);
        assert_eq!(budget.carcass_energy, 0.0);
        assert_eq!(budget.dissipated_energy, 0.0);
    }

    #[test]
    fn find_nearest_agent_returns_closest() {
        let agents = vec![
            explorers_sim::Agent {
                id: 1, position: (10.0, 10.0), reserve: 50.0,
 structure: 0.0,
            nutrient: 0.0,
                traits: TraitVector {
                    photosynthetic_absorption: 1.0, consumption_rate: 0.0,
                    scavenging_rate: 0.0, nutrient_absorption: 0.0, mobility: 0.0, chemotaxis_sensitivity: 0.0,
                    mate_selectivity: 0.0, sensing_range: 0.0, reproductive_investment: 0.0, fecundity: 0.0,
                },
                contact_time: 0,
            },
            explorers_sim::Agent {
                id: 2, position: (20.0, 20.0), reserve: 50.0,
 structure: 0.0,
            nutrient: 0.0,
                traits: TraitVector {
                    photosynthetic_absorption: 1.0, consumption_rate: 0.0,
                    scavenging_rate: 0.0, nutrient_absorption: 0.0, mobility: 0.0, chemotaxis_sensitivity: 0.0,
                    mate_selectivity: 0.0, sensing_range: 0.0, reproductive_investment: 0.0, fecundity: 0.0,
                },
                contact_time: 0,
            },
        ];
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
        let agents = vec![
            explorers_sim::Agent {
                id: 1, position: (5.0, 5.0), reserve: 50.0,
 structure: 0.0,
            nutrient: 0.0,
                traits: TraitVector {
                    photosynthetic_absorption: 1.0, consumption_rate: 0.0,
                    scavenging_rate: 0.0, nutrient_absorption: 0.0, mobility: 0.0, chemotaxis_sensitivity: 0.0,
                    mate_selectivity: 0.0, sensing_range: 0.0, reproductive_investment: 0.0, fecundity: 0.0,
                },
                contact_time: 0,
            },
            explorers_sim::Agent {
                id: 2, position: (95.0, 95.0), reserve: 50.0,
 structure: 0.0,
            nutrient: 0.0,
                traits: TraitVector {
                    photosynthetic_absorption: 1.0, consumption_rate: 0.0,
                    scavenging_rate: 0.0, nutrient_absorption: 0.0, mobility: 0.0, chemotaxis_sensitivity: 0.0,
                    mate_selectivity: 0.0, sensing_range: 0.0, reproductive_investment: 0.0, fecundity: 0.0,
                },
                contact_time: 0,
            },
        ];
        // Click at (97,97) — toroidally closer to agent 2 at (95,95) = dist 2.83
        // but also toroidally close to agent 1 at (5,5) via wrapping = dist ~11.3
        let result = find_nearest_agent(&agents, (97.0, 97.0), 100.0);
        assert_eq!(result, Some(2));

        // Click at (99,99) — toroidally closer to agent 1 at (5,5) via wrapping = dist ~8.5
        // than to agent 2 at (95,95) = dist ~5.7
        let result2 = find_nearest_agent(&agents, (99.0, 99.0), 100.0);
        assert_eq!(result2, Some(2));
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
            let color = trophic_color(&agent.traits, agent.reserve);
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
                mat.color = trophic_color(&agent.traits, agent.reserve);
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

fn trophic_color(traits: &TraitVector, reserve: f32) -> Color {
    let brightness = (reserve.max(0.0) / 100.0).clamp(0.5, 1.0);
    let total = traits.photosynthetic_absorption + traits.consumption_rate + traits.scavenging_rate;
    if total <= 0.0 {
        return Color::srgb(0.8 * brightness, 0.8 * brightness, 0.8 * brightness);
    }
    // Base color from dominant trophic trait, with a minimum per-channel floor
    let r = (traits.consumption_rate / total) * brightness;
    let g = (traits.photosynthetic_absorption / total) * brightness;
    let b = (traits.scavenging_rate / total) * brightness;
    // Ensure minimum visibility: add a small baseline so agents are never invisible
    Color::srgb(r.max(0.15), g.max(0.15), b.max(0.15))
}

const AGENT_RADIUS: f32 = 3.0;

#[derive(Debug, PartialEq)]
struct EnergyBudget {
    living_reserve: f32,
    living_structure: f32,
    living_energy: f32,
    carcass_energy: f32,
    dissipated_energy: f32,
    nutrient_available: Option<f32>,
    nutrient_living: Option<f32>,
    nutrient_carcasses: Option<f32>,
}

fn compute_energy_budget(world: &explorers_sim::World) -> EnergyBudget {
    let living_reserve: f32 = world.agents().iter().map(|a| a.reserve).sum();
    let living_structure: f32 = world.agents().iter().map(|a| a.structure).sum();
    let living_energy: f32 = world.agents().iter().map(|a| a.energy()).sum();
    let carcass_energy: f32 = world.carcasses().iter().map(|c| c.energy).sum();
    EnergyBudget {
        living_reserve,
        living_structure,
        living_energy,
        carcass_energy,
        dissipated_energy: world.dissipated_energy(),
        nutrient_available: Some(world.nutrient_pool()),
        nutrient_living: Some(world.agents().iter().map(|a| a.nutrient).sum()),
        nutrient_carcasses: Some(world.carcasses().iter().map(|c| c.nutrient).sum()),
    }
}

fn find_nearest_agent(agents: &[explorers_sim::Agent], click_pos: (f32, f32), world_extent: f32) -> Option<u64> {
    agents.iter()
        .map(|a| (a.id, explorers_sim::toroidal_distance(click_pos, a.position, world_extent)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(id, _)| id)
}

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
    time.set_timestep(Duration::from_millis(1000));
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

    let half = extent / 2.0;
    for i in 0..=divisions {
        let pos = -half + i as f32 * spacing;
        commands.spawn((
            Mesh2d(vertical_mesh.clone()),
            MeshMaterial2d(line_material.clone()),
            Transform::from_xyz(pos, 0.0, 0.0),
        ));
        commands.spawn((
            Mesh2d(horizontal_mesh.clone()),
            MeshMaterial2d(line_material.clone()),
            Transform::from_xyz(0.0, pos, 0.0),
        ));
    }
}


fn click_to_inspect(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    sim: Res<SimWorld>,
    mut selected: ResMut<SelectedAgent>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Ok((camera, cam_transform)) = cameras.single() else { return };
    let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) else { return };

    let click = (world_pos.x, world_pos.y);
    let extent = sim.0.params().world_extent;
    selected.0 = find_nearest_agent(sim.0.agents(), click, extent);
}

fn debug_panel_ui(
    mut contexts: EguiContexts,
    mut sim: ResMut<SimWorld>,
    tick_count: Res<TickCount>,
    time: Res<Time<Fixed>>,
    virtual_time: Res<Time<Virtual>>,
    selected: Res<SelectedAgent>,
    mut panel_open: ResMut<DebugPanelOpen>,
    mut warmup_frames: Local<u32>,
    mut cameras: Query<&mut Camera, With<Camera2d>>,
    windows: Query<&Window>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    // Skip early frames — egui pass may not have started yet
    if *warmup_frames < 3 {
        *warmup_frames += 1;
        return;
    }

    // Toggle panel with F12
    if ctx.input(|i| i.key_pressed(bevy_egui::egui::Key::F12)) {
        panel_open.0 = !panel_open.0;
    }

    if !panel_open.0 {
        if let Ok(mut camera) = cameras.single_mut() {
            camera.viewport = None;
        }
        return;
    }

    let panel_width = bevy_egui::egui::SidePanel::right("debug_panel")
        .default_width(DEBUG_PANEL_WIDTH)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Debug Panel");
            ui.separator();

            // -- Status bar (preserves old HUD info) --
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
            let avg_contact_time = if total > 0 {
                agents.iter().map(|a| a.contact_time as f64).sum::<f64>() / total as f64
            } else {
                0.0
            };
            let max_contact_time = agents.iter().map(|a| a.contact_time).max().unwrap_or(0);

            ui.label(format!("Tick: {}", tick_count.0));
            ui.label(format!("Agents: {} ({} P / {} C / {} D)", total, producers, consumers, decomposers));
            ui.label(format!("Carcasses: {}", sim.0.carcasses().len()));
            ui.label(format!("Contact time: avg {:.0} / max {}", avg_contact_time, max_contact_time));
            ui.label(format!("TPS: {:.0}{}", tps, if paused { " | PAUSED" } else { "" }));

            ui.separator();

            // -- Energy Budget --
            bevy_egui::egui::CollapsingHeader::new("Energy Budget")
                .default_open(true)
                .show(ui, |ui| {
                    let budget = compute_energy_budget(&sim.0);
                    ui.label(format!("Living reserve: {:.1}", budget.living_reserve));
                    ui.label(format!("Living structure: {:.1}", budget.living_structure));
                    ui.label(format!("Living total: {:.1}", budget.living_energy));
                    ui.label(format!("Carcass structure: {:.1}", budget.carcass_energy));
                    ui.label(format!("Dissipated: {:.1}", budget.dissipated_energy));
                    ui.label(format!("Total: {:.1}", budget.living_energy + budget.carcass_energy + budget.dissipated_energy));
                    ui.separator();
                    ui.label("Nutrients:");
                    ui.label(format!("  Available pool: {}", budget.nutrient_available.map_or("N/A".to_string(), |v| format!("{:.1}", v))));
                    ui.label(format!("  Living agents: {}", budget.nutrient_living.map_or("N/A".to_string(), |v| format!("{:.1}", v))));
                    ui.label(format!("  Carcasses: {}", budget.nutrient_carcasses.map_or("N/A".to_string(), |v| format!("{:.1}", v))));
                });

            ui.separator();

            // -- World Parameter Sliders --
            bevy_egui::egui::CollapsingHeader::new("World Parameters")
                .default_open(true)
                .show(ui, |ui| {
                    let params = sim.0.params_mut();
                    ui.add(bevy_egui::egui::Slider::new(&mut params.solar_flux_magnitude, 0.0..=10.0).text("Solar flux"));
                    ui.add(bevy_egui::egui::Slider::new(&mut params.base_metabolic_rate, 0.0..=2.0).text("Base metabolic rate"));
                    ui.add(bevy_egui::egui::Slider::new(&mut params.photo_maintenance_cost, 0.0..=0.5).text("Photo maintenance"));
                    ui.add(bevy_egui::egui::Slider::new(&mut params.consumption_maintenance_cost, 0.0..=0.5).text("Consumption maintenance"));
                    ui.add(bevy_egui::egui::Slider::new(&mut params.scavenging_maintenance_cost, 0.0..=0.5).text("Scavenging maintenance"));
                    ui.add(bevy_egui::egui::Slider::new(&mut params.mutation_rate, 0.0..=1.0).text("Mutation rate"));
                    ui.add(bevy_egui::egui::Slider::new(&mut params.mutation_magnitude, 0.0..=0.5).text("Mutation magnitude"));
                    ui.add(bevy_egui::egui::Slider::new(&mut params.contact_radius, 1.0..=50.0).text("Contact radius"));
                    ui.add(bevy_egui::egui::Slider::new(&mut params.light_competition_radius, 1.0..=100.0).text("Light competition radius"));
                    ui.add(bevy_egui::egui::Slider::new(&mut params.growth_efficiency, 0.0..=1.0).text("Growth efficiency"));
                });

            ui.separator();

            // -- Click-to-inspect --
            bevy_egui::egui::CollapsingHeader::new("Selected Agent")
                .default_open(true)
                .show(ui, |ui| {
                    match selected.0 {
                        None => {
                            ui.label("Click an agent to inspect");
                        }
                        Some(agent_id) => {
                            if let Some(agent) = sim.0.agents().iter().find(|a| a.id == agent_id) {
                                ui.label(format!("ID: {}", agent.id));
                                ui.label(format!("Position: ({:.1}, {:.1})", agent.position.0, agent.position.1));
                                ui.label(format!("Reserve: {:.1}  (death at 0)", agent.reserve));
                                ui.label(format!("Structure: {:.1}", agent.structure));
                                ui.label(format!("Nutrient: {:.1}", agent.nutrient));
                                let repro_threshold = sim.0.params().reproduction_energy_threshold;
                                let demand = explorers_sim::stoichiometric_demand(&agent.traits);
                                ui.label(format!("Repro: energy ≥ {:.0}, nutrient ≥ {:.1}",
                                    repro_threshold, demand));
                                ui.label(format!("Contact time: {}", agent.contact_time));
                                ui.label(format!("Dominant role: {}", dominant_role(&agent.traits)));
                                ui.separator();
                                ui.label("Trait vector:");
                                ui.label(format!("  photosynthetic_absorption: {:.3}", agent.traits.photosynthetic_absorption));
                                ui.label(format!("  consumption_rate: {:.3}", agent.traits.consumption_rate));
                                ui.label(format!("  scavenging_rate: {:.3}", agent.traits.scavenging_rate));
                                ui.label(format!("  nutrient_absorption: {:.3}", agent.traits.nutrient_absorption));
                                ui.label(format!("  mobility: {:.3}", agent.traits.mobility));
                                ui.label(format!("  chemotaxis_sensitivity: {:.3}", agent.traits.chemotaxis_sensitivity));
                                ui.label(format!("  mate_selectivity: {:.3}", agent.traits.mate_selectivity));
                                ui.label(format!("  sensing_range: {:.3}", agent.traits.sensing_range));
                                ui.label(format!("  reproductive_investment: {:.3}", agent.traits.reproductive_investment));
                                ui.label(format!("  fecundity: {:.3}", agent.traits.fecundity));
                            } else {
                                ui.label(format!("Agent {} no longer alive", agent_id));
                            }
                        }
                    }
                });
        })
        .response
        .rect
        .width();

    // Set camera viewport to exclude the panel area
    if let Ok(mut camera) = cameras.single_mut() {
        if let Ok(window) = windows.single() {
            let right_px = (panel_width * window.scale_factor()) as u32;
            let vp_width = window.physical_width().saturating_sub(right_px);
            if vp_width > 0 {
                camera.viewport = Some(Viewport {
                    physical_position: UVec2::ZERO,
                    physical_size: UVec2::new(vp_width, window.physical_height()),
                    ..default()
                });
            }
        }
    }
}

