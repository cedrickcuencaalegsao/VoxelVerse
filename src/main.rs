use bevy::prelude::*;

mod block;
mod block_registry;
mod camera;
mod chunk;
mod fire;
mod hud;
mod input;
mod physics;
mod world;
mod block_breaking;

use block_registry::{BlockRegistry, BlockRegistryPlugin};
use camera::CameraPlugin;
use chunk::ChunkPlugin;
use fire::FirePlugin;
use hud::HudPlugin;
use input::InputPlugin;
use physics::PhysicsPlugin;
use world::WorldPlugin;
use block_breaking::BlockBreakingPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Voxel Verse".to_string(),
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
            FirePlugin,
            HudPlugin,
            BlockBreakingPlugin,
        ))
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
        .add_systems(Startup, (setup, set_window_title))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let gltf_handle = asset_server.load("block.glb");
    commands.insert_resource(BlockRegistry::new(gltf_handle));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 8000.0,
            shadows_enabled: true,
            ..default()
        },
        // Angled sun so each face gets different brightness — gives depth
        transform: Transform::from_xyz(1.0, 2.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn set_window_title(world: Res<crate::world::World>, mut windows: Query<&mut Window>) {
    if let Ok(mut window) = windows.get_single_mut() {
        window.title = format!("Voxel Verse — seed: {}", world.seed);
    }
}
