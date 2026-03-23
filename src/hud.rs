use bevy::prelude::*;
use crate::camera::Player;
use crate::physics::PLAYER_HEIGHT;

#[derive(Component)]
pub struct CoordText;

#[derive(Component)]
pub struct Crosshair;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_hud, setup_crosshair))
            .add_systems(Update, update_coords);
    }
}

fn setup_crosshair(mut commands: Commands) {
    // Root node — full screen, centered
    commands.spawn(NodeBundle {
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
        // Horizontal bar
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

        // Vertical bar
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

        // Dark outline horizontal — gives contrast on bright backgrounds
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

        // Dark outline vertical
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
    commands.spawn(NodeBundle {
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
    if (v - rounded).abs() < 0.05 {
        rounded.floor()
    } else {
        v.floor()
    }
}