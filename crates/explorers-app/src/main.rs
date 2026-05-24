use bevy::prelude::*;
use explorers_sim::{Agent, TraitVector, World};

#[derive(Resource)]
struct SimWorld(World);

#[derive(Component)]
struct AgentMarker(usize);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(SimWorld(World::new(vec![
            Agent {
                position: (0.0, 0.0),
                energy: 100.0,
                traits: TraitVector { values: [0.3, 0.8] },
            },
            Agent {
                position: (100.0, 50.0),
                energy: 80.0,
                traits: TraitVector { values: [0.7, 0.2] },
            },
            Agent {
                position: (-80.0, -60.0),
                energy: 90.0,
                traits: TraitVector { values: [0.5, 0.5] },
            },
        ])))
        .add_systems(Startup, (setup_camera, spawn_agent_sprites))
        .add_systems(FixedUpdate, step_simulation)
        .add_systems(Update, sync_agent_transforms)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn spawn_agent_sprites(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    sim: Res<SimWorld>,
) {
    let mesh = meshes.add(Circle::new(12.0));
    for (i, agent) in sim.0.agents().iter().enumerate() {
        let color = Color::hsl(agent.traits.values[0] * 360.0, 0.7, 0.5);
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
