use std::fs;

use bevy::prelude::*;
use bevy::camera::ScalingMode;
use explorers_sim::{InitialDistribution, TraitVector, World, WorldParameters, WorldRecipe};

#[derive(Resource)]
struct SimWorld(World);

#[derive(Component)]
struct AgentMarker(usize);

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
        .add_systems(Startup, (setup_camera, spawn_agent_sprites))
        .add_systems(FixedUpdate, step_simulation)
        .add_systems(Update, sync_agent_transforms)
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

fn spawn_agent_sprites(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    sim: Res<SimWorld>,
) {
    let mesh = meshes.add(Circle::new(12.0));
    for (i, agent) in sim.0.agents().iter().enumerate() {
        let color = Color::hsl(agent.traits.photosynthetic_absorption * 360.0, 0.7, 0.5);
        commands.spawn((
            Mesh2d(mesh.clone()),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(color))),
            Transform::from_xyz(agent.position.0, agent.position.1, 0.0),
            AgentMarker(i),
        ));
    }
}

fn step_simulation(mut sim: ResMut<SimWorld>) {
    sim.0.step();
}

fn sync_agent_transforms(sim: Res<SimWorld>, mut query: Query<(&AgentMarker, &mut Transform)>) {
    for (marker, mut transform) in &mut query {
        if let Some(agent) = sim.0.agents().get(marker.0) {
            transform.translation.x = agent.position.0;
            transform.translation.y = agent.position.1;
        }
    }
}
