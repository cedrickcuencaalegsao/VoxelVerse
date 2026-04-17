// item_preview.rs
//
// Renders each inventory item's .glb model into a small off-screen texture so
// the hotbar can display a live 3-D preview instead of a flat colour swatch.
//
// Fixes vs previous version
// ──────────────────────────
// 1. Texture format is Rgba8UnormSrgb (Metal does not accept Bgra as a
//    render-attachment in all configurations).
// 2. Each preview model is placed at a unique world position far below the
//    main scene (y = -2000) so they never overlap or appear in-world.
// 3. A propagation system walks newly-spawned scene children and stamps them
//    with the correct RenderLayers — without this the main camera (layer 0)
//    picks up the mesh entities that Bevy's scene spawner creates as children,
//    which is what caused the blank world.

use bevy::{
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        view::RenderLayers,
    },
};

use crate::inventory::ItemKind;

// ── resource ─────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct ItemPreviews {
    pub images: bevy::utils::HashMap<ItemKind, Handle<Image>>,
}

// ── plugin ────────────────────────────────────────────────────────────────────

pub struct ItemPreviewPlugin;

impl Plugin for ItemPreviewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ItemPreviews>()
            .add_systems(Startup, spawn_preview_scenes)
            .add_systems(Update, propagate_preview_render_layers);
    }
}

// ── constants ─────────────────────────────────────────────────────────────────

const PREVIEW_SIZE: u32 = 64;

/// Layers 2 … 2+N are reserved for item previews.
/// Layer 0 = main world.  Layer 1 = spare.  Layer 2+ = previews.
const PREVIEW_BASE_LAYER: usize = 2;

/// Park preview scenes far underground so even without render-layer isolation
/// they would never be visible in-game.
const PREVIEW_ORIGIN: Vec3 = Vec3::new(0.0, -2000.0, 0.0);
const PREVIEW_SPACING: f32 = 20.0;

// ── startup ───────────────────────────────────────────────────────────────────

fn spawn_preview_scenes(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut previews: ResMut<ItemPreviews>,
    asset_server: Res<AssetServer>,
) {
    for (index, &kind) in ItemKind::ALL.iter().enumerate() {
        let layer_id = PREVIEW_BASE_LAYER + index;
        let render_layer = RenderLayers::layer(layer_id);

        // Each preview lives at a unique position far outside the playable world.
        let origin = PREVIEW_ORIGIN + Vec3::X * (index as f32 * PREVIEW_SPACING);

        // ── render-target texture ─────────────────────────────────────────
        let size = Extent3d {
            width: PREVIEW_SIZE,
            height: PREVIEW_SIZE,
            depth_or_array_layers: 1,
        };

        let mut image = Image {
            texture_descriptor: TextureDescriptor {
                label: Some("item_preview"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                // Rgba8UnormSrgb is universally supported as a render attachment.
                // Bgra8UnormSrgb is NOT guaranteed on Metal and caused blank screens.
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            ..default()
        };
        image.resize(size);

        let image_handle = images.add(image);
        previews.images.insert(kind, image_handle.clone());

        // ── dedicated camera ──────────────────────────────────────────────
        let cam_pos = origin + Vec3::new(0.0, 1.4, 2.5);
        let look_at = origin + Vec3::new(0.0, 0.4, 0.0);

        commands.spawn((
            Camera3dBundle {
                camera: Camera {
                    // Negative order renders before the primary camera.
                    // Each preview camera gets a unique order so they don't
                    // conflict with each other.
                    order: -((index as isize) + 1),
                    target: RenderTarget::Image(image_handle),
                    clear_color: ClearColorConfig::Custom(Color::srgba(0.08, 0.08, 0.08, 1.0)),
                    ..default()
                },
                transform: Transform::from_translation(cam_pos).looking_at(look_at, Vec3::Y),
                ..default()
            },
            render_layer.clone(),
            PreviewCamera { kind },
        ));

        // ── per-preview directional light ─────────────────────────────────
        commands.spawn((
            DirectionalLightBundle {
                directional_light: DirectionalLight {
                    illuminance: 10_000.0,
                    shadows_enabled: false,
                    ..default()
                },
                transform: Transform::from_rotation(Quat::from_euler(
                    EulerRot::XYZ,
                    -std::f32::consts::FRAC_PI_4,
                    std::f32::consts::FRAC_PI_4,
                    0.0,
                )),
                ..default()
            },
            render_layer.clone(),
        ));

        // ── GLB scene root ────────────────────────────────────────────────
        // RenderLayers on the root entity alone is not enough — Bevy's scene
        // spawner creates child entities for each mesh node and those children
        // don't inherit the parent's RenderLayers.  The propagation system
        // below handles stamping the children once they appear.
        commands.spawn((
            SceneBundle {
                scene: asset_server.load(format!("{}#Scene0", kind.glb_path())),
                transform: Transform {
                    translation: origin,
                    rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_4),
                    scale: Vec3::splat(kind.preview_scale()),
                },
                ..default()
            },
            render_layer.clone(),
            PreviewScene { kind },
        ));
    }
}

// ── render-layer propagation ──────────────────────────────────────────────────

/// Bevy's SceneSpawner populates children asynchronously.  Without stamping
/// each child with RenderLayers the main camera (layer 0) will render every
/// mesh entity, making the preview models appear in the 3-D world and
/// disrupting the render graph (the blank-world bug).
///
/// Strategy: every frame, walk all `PreviewScene` roots.  DFS every child and
/// insert the layer on anything that doesn't have it yet.  Once the tree has
/// children AND all descendants are stamped, remove `PreviewScene` so we stop
/// visiting it.
fn propagate_preview_render_layers(
    mut commands: Commands,
    scene_roots: Query<(Entity, &PreviewScene, &RenderLayers)>,
    children_query: Query<&Children>,
    has_layers: Query<(), With<RenderLayers>>,
) {
    for (root, _tag, layer) in scene_roots.iter() {
        // Walk the full descendant tree.
        let mut stack = vec![root];
        let mut found_children = false;
        let mut all_done = true;

        while let Some(entity) = stack.pop() {
            if let Ok(children) = children_query.get(entity) {
                for &child in children.iter() {
                    found_children = true;
                    if !has_layers.contains(child) {
                        commands.entity(child).insert(layer.clone());
                        all_done = false;
                    }
                    stack.push(child);
                }
            }
        }

        // Only retire this root once children have actually loaded.
        if found_children && all_done {
            commands.entity(root).remove::<PreviewScene>();
        }
    }
}

// ── marker components ─────────────────────────────────────────────────────────

#[derive(Component)]
pub struct PreviewCamera {
    #[allow(dead_code)]
    pub kind: ItemKind,
}

/// Kept on the scene-root entity until all descendants are stamped with
/// RenderLayers, then removed.
#[derive(Component, Clone)]
pub struct PreviewScene {
    #[allow(dead_code)]
    pub kind: ItemKind,
}

// ── ItemKind asset helpers ────────────────────────────────────────────────────

pub trait ItemKindPreviewExt {
    fn glb_path(self) -> &'static str;
    fn preview_scale(self) -> f32;
}

impl ItemKindPreviewExt for ItemKind {
    fn glb_path(self) -> &'static str {
        match self {
            ItemKind::Grass  => "grass.glb",
            ItemKind::Dirt   => "soil.glb",
            ItemKind::Stone  => "stone.glb",
            ItemKind::Sand   => "sand.glb",
            ItemKind::Wood   => "wood.glb",
            ItemKind::Leaves => "leaves.glb",
            ItemKind::Weed   => "weed_1.glb",
        }
    }

    fn preview_scale(self) -> f32 {
        match self {
            ItemKind::Weed   => 0.55,
            ItemKind::Leaves => 0.85,
            _                => 1.0,
        }
    }
}