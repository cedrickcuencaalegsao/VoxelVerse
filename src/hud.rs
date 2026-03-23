use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use crate::camera::{Player, PlayerCamera};
use crate::physics::PLAYER_HEIGHT;
use crate::world::World as GameWorld;
use crate::chunk::{CHUNK_SIZE, CHUNK_HEIGHT};
use crate::block::BlockType;

#[derive(Component)]
pub struct CoordText;

#[derive(Component)]
pub struct Crosshair;

#[derive(Component)]
pub struct StatsText;

#[derive(Component)]
#[allow(dead_code)]
pub struct MinimapDot {
    pub _kind: MinimapDotKind,
}

#[derive(PartialEq)]
#[allow(dead_code)]
pub enum MinimapDotKind {
    Terrain,
    Player,
}

#[derive(Component)]
pub struct MinimapContainer;

#[derive(Resource)]
pub struct MinimapState {
    pub expanded: bool,
    pub dirty: bool,
    pub last_player_pos: Vec2,
    pub frame_counter: u32,
}

impl Default for MinimapState {
    fn default() -> Self {
        Self {
            expanded: false,
            dirty: true,
            last_player_pos: Vec2::ZERO,
            frame_counter: 0,
        }
    }
}

const REDRAW_THRESHOLD: f32 = 2.0;
const REDRAW_EVERY_FRAMES: u32 = 6;
const MAP_SIZE_SMALL: f32 = 160.0;
const MAP_SIZE_LARGE: f32 = 380.0;
const MAP_RADIUS_SMALL: i32 = 24;
const MAP_RADIUS_LARGE: i32 = 56;
const DOT_SIZE_SMALL: f32 = 3.2;
const DOT_SIZE_LARGE: f32 = 3.5;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin)
            .init_resource::<MinimapState>()
            .add_systems(Startup, (setup_hud, setup_crosshair, setup_minimap, setup_stats))
            .add_systems(Update, (
                update_coords,
                update_stats,
                toggle_minimap,
                update_minimap_terrain,
                update_minimap_overlay,
            ));
    }
}

fn setup_crosshair(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        width: Val::Px(20.0),
                        height: Val::Px(2.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.9)),
                    ..default()
                },
                Crosshair,
            ));
            parent.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        width: Val::Px(2.0),
                        height: Val::Px(20.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.9)),
                    ..default()
                },
                Crosshair,
            ));
            parent.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Px(22.0),
                    height: Val::Px(4.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.4)),
                z_index: ZIndex::Local(-1),
                ..default()
            });
            parent.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Px(4.0),
                    height: Val::Px(22.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.4)),
                z_index: ZIndex::Local(-1),
                ..default()
            });
        });
}

fn setup_hud(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                TextBundle::from_section(
                    "X: 0  Y: 0  Z: 0",
                    TextStyle {
                        font_size: 16.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ),
                CoordText,
            ));
        });
}

fn update_coords(
    mut text_query: Query<&mut Text, With<CoordText>>,
    player_query: Query<&Transform, With<Player>>,
) {
    let Ok(transform) = player_query.get_single() else { return };
    let Ok(mut text) = text_query.get_single_mut() else { return };

    let eye = transform.translation;
    let feet_y = eye.y - PLAYER_HEIGHT;

    let bx = snap_coord(eye.x) as i32;
    let by = snap_coord(feet_y) as i32;
    let bz = snap_coord(eye.z) as i32;

    text.sections[0].value = format!(
        "X: {}  Y: {}  Z: {}\n({:.1}, {:.1}, {:.1})",
        bx, by, bz,
        eye.x, feet_y, eye.z
    );
}

fn snap_coord(v: f32) -> f32 {
    let rounded = v.round();
    if (v - rounded).abs() < 0.05 { rounded.floor() } else { v.floor() }
}
fn setup_stats(mut commands: Commands) {
    commands.spawn((
        TextBundle {
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                right: Val::Px(10.0),
                ..default()
            },
            text: Text::from_section(
                "FPS: --",
                TextStyle {
                    font_size: 13.0,
                    color: Color::srgb(0.3, 1.0, 0.3),
                    ..default()
                },
            ),
            ..default()
        },
        StatsText,
    ));
}

fn update_stats(
    mut text_query: Query<&mut Text, With<StatsText>>,
    diagnostics: Res<DiagnosticsStore>,
    world: Res<GameWorld>,
    all_entities: Query<Entity>,
) {
    let Ok(mut text) = text_query.get_single_mut() else { return };

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    let frame_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0)
        * 1000.0;

    let entity_count  = all_entities.iter().count();
    let chunk_count   = world.chunks.len();

    let fps_color = if fps >= 55.0 {
        Color::srgb(0.3, 1.0, 0.3)
    } else if fps >= 30.0 {
        Color::srgb(1.0, 0.85, 0.0)
    } else {
        Color::srgb(1.0, 0.3, 0.3)
    };

    text.sections[0].style.color = fps_color;
    text.sections[0].value = format!(
        "FPS {:.0} ({:.1}ms)  |  {} entities  |  {} chunks",
        fps, frame_ms, entity_count, chunk_count,
    );
}

fn setup_minimap(mut commands: Commands) {
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                right: Val::Px(10.0),
                width: Val::Px(MAP_SIZE_SMALL),
                height: Val::Px(MAP_SIZE_SMALL),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            border_color: BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.4)),
            z_index: ZIndex::Global(9),
            ..default()
        },
        MinimapContainer,
    ));
}

fn toggle_minimap(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<MinimapState>,
    mut container_query: Query<&mut Style, With<MinimapContainer>>,
) {
    if keyboard.just_pressed(KeyCode::KeyM) {
        state.expanded = !state.expanded;
        state.dirty = true;
        if let Ok(mut style) = container_query.get_single_mut() {
            let size = if state.expanded { MAP_SIZE_LARGE } else { MAP_SIZE_SMALL };
            style.width = Val::Px(size);
            style.height = Val::Px(size);
        }
    }
}

#[derive(Component)]
pub struct MinimapTerrainDot;

fn update_minimap_terrain(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    world: Res<GameWorld>,
    chunks: Query<&crate::chunk::Chunk>,
    dot_query: Query<Entity, With<MinimapTerrainDot>>,
    windows: Query<&Window>,
    mut state: ResMut<MinimapState>,
) {
    let Ok(player_transform) = player_query.get_single() else { return };
    let Ok(window) = windows.get_single() else { return };

    let eye = player_transform.translation;
    let current_pos = Vec2::new(eye.x, eye.z);

    state.frame_counter += 1;
    let moved_enough = current_pos.distance(state.last_player_pos) > REDRAW_THRESHOLD;
    let frame_ok = state.frame_counter >= REDRAW_EVERY_FRAMES;

    if !state.dirty && !moved_enough && !frame_ok {
        return;
    }

    state.frame_counter = 0;
    if moved_enough || state.dirty {
        state.last_player_pos = current_pos;
        state.dirty = false;
    }

    for entity in dot_query.iter() {
        commands.entity(entity).despawn();
    }

    let map_size   = if state.expanded { MAP_SIZE_LARGE   } else { MAP_SIZE_SMALL   };
    let map_radius = if state.expanded { MAP_RADIUS_LARGE  } else { MAP_RADIUS_SMALL  };
    let dot_size   = if state.expanded { DOT_SIZE_LARGE   } else { DOT_SIZE_SMALL   };

    let win_w    = window.width();
    let map_left = win_w - map_size - 10.0;
    let map_top  = 10.0;
    let scale    = map_size / (map_radius as f32 * 2.0);
    let px = eye.x;
    let pz = eye.z;

    for dx in -map_radius..=map_radius {
        for dz in -map_radius..=map_radius {
            let wx = (px + dx as f32).floor() as i32;
            let wz = (pz + dz as f32).floor() as i32;

            let chunk_x   = wx.div_euclid(CHUNK_SIZE as i32);
            let chunk_z   = wz.div_euclid(CHUNK_SIZE as i32);
            let chunk_pos = IVec3::new(chunk_x, 0, chunk_z);

            let Some(&chunk_entity) = world.chunks.get(&chunk_pos) else { continue };
            let Ok(chunk) = chunks.get(chunk_entity) else { continue };

            let lx = wx.rem_euclid(CHUNK_SIZE as i32) as usize;
            let lz = wz.rem_euclid(CHUNK_SIZE as i32) as usize;

            let mut top_block = BlockType::Air;
            for y in (0..CHUNK_HEIGHT).rev() {
                let b = chunk.get_block(lx, y, lz);
                if b.is_solid() || matches!(b, BlockType::Water) {
                    top_block = b;
                    break;
                }
            }

            if matches!(top_block, BlockType::Air) { continue; }

            let dot_color = minimap_color(top_block);
            let screen_x  = map_left + (dx + map_radius) as f32 * scale;
            let screen_y  = map_top  + (dz + map_radius) as f32 * scale;

            commands.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left:   Val::Px(screen_x),
                        top:    Val::Px(screen_y),
                        width:  Val::Px(dot_size),
                        height: Val::Px(dot_size),
                        ..default()
                    },
                    background_color: BackgroundColor(dot_color),
                    z_index: ZIndex::Global(10),
                    ..default()
                },
                MinimapTerrainDot,
            ));
        }
    }
}

#[derive(Component)]
pub struct MinimapOverlayDot;

fn update_minimap_overlay(
    mut commands: Commands,
    player_query: Query<(&Transform, &PlayerCamera), With<Player>>,
    overlay_query: Query<Entity, With<MinimapOverlayDot>>,
    windows: Query<&Window>,
    state: Res<MinimapState>,
) {
    let Ok((_player_transform, camera)) = player_query.get_single() else { return };
    let Ok(window) = windows.get_single() else { return };

    for entity in overlay_query.iter() {
        commands.entity(entity).despawn();
    }

    let map_size = if state.expanded { MAP_SIZE_LARGE } else { MAP_SIZE_SMALL };
    let win_w    = window.width();
    let map_left = win_w - map_size - 10.0;
    let map_top  = 10.0;
    let cx = map_left + map_size / 2.0;
    let cy = map_top  + map_size / 2.0;

    // Player dot
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left:   Val::Px(cx - 4.0),
                top:    Val::Px(cy - 4.0),
                width:  Val::Px(8.0),
                height: Val::Px(8.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgb(1.0, 1.0, 0.0)),
            z_index: ZIndex::Global(12),
            ..default()
        },
        MinimapOverlayDot,
    ));

    // Direction arrow
    let yaw   = camera.yaw;
    let sin_y = -yaw.sin();
    let cos_y = yaw.cos();
    let arrow_dist = 12.0;

    let tip_x   = cx + sin_y * arrow_dist;
    let tip_z   = cy - cos_y * arrow_dist;
    let left_x  = cx + (sin_y * 5.0 - cos_y * 5.0);
    let left_z  = cy - (cos_y * 5.0 + sin_y * 5.0);
    let right_x = cx + (sin_y * 5.0 + cos_y * 5.0);
    let right_z = cy - (cos_y * 5.0 - sin_y * 5.0);

    for (ax, az) in [(tip_x, tip_z), (left_x, left_z), (right_x, right_z)] {
        commands.spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left:   Val::Px(ax - 3.0),
                    top:    Val::Px(az - 3.0),
                    width:  Val::Px(6.0),
                    height: Val::Px(6.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgb(1.0, 0.2, 0.2)),
                z_index: ZIndex::Global(13),
                ..default()
            },
            MinimapOverlayDot,
        ));
    }

    // Cardinal labels
    let label_size = 14.0;
    let cardinals = [
        ("N", cx,                         map_top + 4.0),
        ("S", cx,                         map_top + map_size - 16.0),
        ("W", map_left + 4.0,             map_top + map_size / 2.0 - 7.0),
        ("E", map_left + map_size - 12.0, map_top + map_size / 2.0 - 7.0),
    ];

    for (label, lx, ly) in cardinals {
        commands.spawn((
            TextBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(lx - label_size / 2.0),
                    top:  Val::Px(ly),
                    ..default()
                },
                text: Text::from_section(
                    label,
                    TextStyle {
                        font_size: label_size,
                        color: Color::srgb(1.0, 0.9, 0.2),
                        ..default()
                    },
                ),
                z_index: ZIndex::Global(14),
                ..default()
            },
            MinimapOverlayDot,
        ));
    }

    // M hint
    let hint_text = if state.expanded { "[M] minimize" } else { "[M] expand" };
    commands.spawn((
        TextBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(map_left + 4.0),
                top:  Val::Px(map_top + map_size - 16.0),
                ..default()
            },
            text: Text::from_section(
                hint_text,
                TextStyle {
                    font_size: 11.0,
                    color: Color::srgba(1.0, 1.0, 1.0, 0.5),
                    ..default()
                },
            ),
            z_index: ZIndex::Global(14),
            ..default()
        },
        MinimapOverlayDot,
    ));
}

fn minimap_color(block: BlockType) -> Color {
    match block {
        BlockType::Grass  => Color::srgb(0.25, 0.65, 0.15),
        BlockType::Dirt   => Color::srgb(0.45, 0.28, 0.12),
        BlockType::Stone  => Color::srgb(0.50, 0.50, 0.50),
        BlockType::Sand   => Color::srgb(0.85, 0.78, 0.50),
        BlockType::Wood   => Color::srgb(0.40, 0.25, 0.10),
        BlockType::Leaves => Color::srgb(0.15, 0.50, 0.10),
        BlockType::Water  => Color::srgb(0.10, 0.40, 0.85),
        BlockType::Air    => Color::srgba(0.0, 0.0, 0.0, 0.0),
    }
}