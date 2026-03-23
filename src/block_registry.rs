use bevy::prelude::*;
use bevy::gltf::Gltf;

#[derive(Resource)]
pub struct BlockRegistry {
    pub gltf_handle: Handle<Gltf>,
    pub material: Option<Handle<StandardMaterial>>,
    pub loaded: bool,
}

impl BlockRegistry {
    pub fn new(gltf_handle: Handle<Gltf>) -> Self {
        Self {
            gltf_handle,
            material: None,
            loaded: false,
        }
    }
}

pub struct BlockRegistryPlugin;

impl Plugin for BlockRegistryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, extract_block_assets);
    }
}

fn extract_block_assets(
    mut registry: ResMut<BlockRegistry>,
    gltf_assets: Res<Assets<Gltf>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if registry.loaded {
        return;
    }

    let Some(_gltf) = gltf_assets.get(&registry.gltf_handle) else {
        return;
    };

    registry.material = Some(materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.85,
        reflectance: 0.3,
        unlit: false,
        ..default()
    }));

    registry.loaded = true;
    info!("BlockRegistry loaded — vertex colors enabled");
}