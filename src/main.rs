use bevy::prelude::*;

mod block;
mod camera;
mod chunk;
mod input;
mod physics;
mod world;

use camera::CameraPlugin;
use chunk::ChunkPlugin;
use input::InputPlugin;
use physics::PhysicsPlugin;
use world::WorldPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Voxel Game".to_string(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            WorldPlugin,
            ChunkPlugin,
            CameraPlugin,
            InputPlugin,
            PhysicsPlugin,
        ))
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // Add directional light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(50.0, 100.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}