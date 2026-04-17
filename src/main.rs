use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::window::PresentMode;

mod block;
mod block_breaking;
mod block_registry;
mod camera;
mod chunk;
mod daynight;
mod fire;
mod hud;
mod input;
mod inventory;
mod physics;
mod tree_breaking;
mod world;
mod weed_breaking;

use block_breaking::BlockBreakingPlugin;
use block_registry::{BlockRegistry, BlockRegistryPlugin};
use camera::CameraPlugin;
use chunk::ChunkPlugin;
use daynight::DayNightPlugin;
use fire::FirePlugin;
use hud::HudPlugin;
use input::InputPlugin;
use inventory::InventoryPlugin;
use physics::PhysicsPlugin;
use tree_breaking::TreeBreakingPlugin;
use world::WorldPlugin;
use weed_breaking::WeedBreakingPlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Voxel Verse".to_string(),
                        resolution: (1280.0, 720.0).into(),
                        present_mode: PresentMode::AutoNoVsync,
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        backends: Some(Backends::METAL),
                        ..default()
                    }),
                    ..default()
                }),
        )
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
            DayNightPlugin,
            TreeBreakingPlugin,
            WeedBreakingPlugin,
            InventoryPlugin,
        ))
        // Start with sunrise sky — day/night will take over immediately
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
        .add_systems(Startup, (setup, set_window_title))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let gltf_handle = asset_server.load("block.glb");
    commands.insert_resource(BlockRegistry::new(gltf_handle));

    // Directional light — day/night plugin controls its angle, color and illuminance
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 8000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(1.0, 2.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn set_window_title(world: Res<crate::world::World>, mut windows: Query<&mut Window>) {
    if let Ok(mut window) = windows.get_single_mut() {
        window.title = format!("Voxel Verse — seed: {}", world.seed);
    }
}
