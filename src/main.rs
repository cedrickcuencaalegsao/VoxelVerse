use bevy::prelude::*;

mod block;
mod block_registry;
mod camera;
mod chunk;
mod input;
mod physics;
mod world;

use block_registry::{BlockRegistry, BlockRegistryPlugin};
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
            BlockRegistryPlugin,
        ))
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // Load block.glb once — all chunks will share this
    let gltf_handle = asset_server.load("block.glb");
    commands.insert_resource(BlockRegistry::new(gltf_handle));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(50.0, 100.0, 50.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}