use crate::physics::{Grounded, Velocity, PLAYER_HEIGHT};
use crate::world::World as GameWorld;
use bevy::prelude::*;
use bevy::window::CursorGrabMode;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PovMode {
    FirstPerson,
    ThirdPerson,
    TopDown,
    FrontView,
}

#[derive(Component)]
pub struct PlayerCamera {
    pub sensitivity: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub pov: PovMode,
    pub tp_distance: f32,
    pub tp_height: f32,
}

impl Default for PlayerCamera {
    fn default() -> Self {
        Self {
            sensitivity: 0.002,
            yaw: 0.0,
            pitch: 0.0,
            pov: PovMode::FirstPerson,
            tp_distance: 3.0,
            tp_height: 2.0,
        }
    }
}

// The player body — holds physics, collision, and the capsule mesh
#[derive(Component)]
pub struct Player {
    pub speed: f32,
    pub sprint_multiplier: f32,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            speed: 5.0,
            sprint_multiplier: 2.0,
        }
    }
}

// Marker for the capsule mesh entity
#[derive(Component)]
pub struct PlayerMesh;

// Marker for the camera entity
#[derive(Component)]
pub struct MainCamera;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_player_and_camera)
            .add_systems(Update, (mouse_look, grab_cursor, switch_pov));
    }
}

fn setup_player_and_camera(
    mut commands: Commands,
    world: Res<GameWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let feet_y = crate::world::get_spawn_height(&world.noise);
    let eye_y = feet_y + PLAYER_HEIGHT;

    // Capsule mesh material
    let capsule_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.6, 1.0),
        ..default()
    });
    let capsule_mesh = meshes.add(Capsule3d::new(0.35, PLAYER_HEIGHT - 0.7));

    // Player body entity — owns physics, capsule mesh, velocity, grounded
    commands.spawn((
        SpatialBundle {
            transform: Transform::from_xyz(0.0, eye_y, 0.0),
            ..default()
        },
        Player::default(),
        Velocity(Vec3::ZERO),
        Grounded(false),
        PlayerCamera::default(),
    )).with_children(|parent| {
        // Capsule mesh centered on the player body
        // Offset down by half height so feet align with transform Y
        parent.spawn((
            PbrBundle {
                mesh: capsule_mesh,
                material: capsule_mat,
                transform: Transform::from_xyz(0.0, -PLAYER_HEIGHT / 2.0, 0.0),
                visibility: Visibility::Hidden, // hidden in FP by default
                ..default()
            },
            PlayerMesh,
        ));
    });

    // Camera entity — completely separate, no physics
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, eye_y, 0.0)
                .looking_at(Vec3::new(0.0, eye_y, 10.0), Vec3::Y),
            ..default()
        },
        MainCamera,
    ));
}

fn switch_pov(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<&mut PlayerCamera, With<Player>>,
    mut mesh_query: Query<&mut Visibility, With<PlayerMesh>>,
) {
    let new_pov =
        if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
            if keyboard.just_pressed(KeyCode::Digit1) {
                Some(PovMode::FirstPerson)
            } else if keyboard.just_pressed(KeyCode::Digit2) {
                Some(PovMode::ThirdPerson)
            } else if keyboard.just_pressed(KeyCode::Digit3) {
                Some(PovMode::TopDown)
            } else if keyboard.just_pressed(KeyCode::Digit4) {
                Some(PovMode::FrontView)
            } else {
                None
            }
        } else {
            None
        };

    let Some(pov) = new_pov else { return };

    for mut cam in player_query.iter_mut() {
        cam.pov = pov;
        if pov == PovMode::TopDown {
            cam.pitch = -std::f32::consts::FRAC_PI_2;
        }
    }

    for mut vis in mesh_query.iter_mut() {
        *vis = match pov {
            PovMode::FirstPerson => Visibility::Hidden,
            PovMode::ThirdPerson | PovMode::TopDown | PovMode::FrontView => Visibility::Visible,
        };
    }
}

fn mouse_look(
    mut mouse_motion: EventReader<bevy::input::mouse::MouseMotion>,
    // Player has the camera settings and the authoritative position
    mut player_query: Query<(&mut PlayerCamera, &Transform), With<Player>>,
    // Camera is a separate entity we reposition each frame
    mut camera_transform_query: Query<&mut Transform, (With<MainCamera>, Without<Player>)>,
) {
    let mut delta = Vec2::ZERO;
    for event in mouse_motion.read() {
        delta += event.delta;
    }

    let Ok((mut cam, player_transform)) = player_query.get_single_mut() else { return };
    let Ok(mut cam_transform) = camera_transform_query.get_single_mut() else { return };

    // The player body transform IS the eye position (set by physics)
    let eye_pos = player_transform.translation;

    if delta != Vec2::ZERO {
        cam.yaw -= delta.x * cam.sensitivity;
        match cam.pov {
            PovMode::FirstPerson => {
                cam.pitch -= delta.y * cam.sensitivity;
                cam.pitch = cam.pitch.clamp(-1.54, 1.54);
            }
            PovMode::ThirdPerson | PovMode::FrontView => {
                cam.pitch -= delta.y * cam.sensitivity;
                cam.pitch = cam.pitch.clamp(-0.8, 0.6);
            }
            PovMode::TopDown => {
                // No pitch change in top-down
            }
        }
    }

    // Reposition camera based on POV mode every frame
    match cam.pov {
        PovMode::FirstPerson => {
            cam_transform.translation = eye_pos;
            cam_transform.rotation = Quat::from_rotation_y(cam.yaw)
                * Quat::from_rotation_x(cam.pitch);
        }

        PovMode::ThirdPerson => {
            let offset = Vec3::new(
                -cam.yaw.sin() * cam.tp_distance,
                cam.tp_height + cam.pitch * cam.tp_distance,
                -cam.yaw.cos() * cam.tp_distance,
            );
            cam_transform.translation = eye_pos + offset;
            cam_transform.look_at(eye_pos + Vec3::Y * 0.5, Vec3::Y);
        }

        PovMode::TopDown => {
            cam_transform.translation = eye_pos + Vec3::new(0.0, cam.tp_distance, 0.0);
            cam_transform.look_at(
                eye_pos,
                Vec3::new(cam.yaw.sin(), 0.0, cam.yaw.cos()),
            );
        }

        PovMode::FrontView => {
            let offset = Vec3::new(
                cam.yaw.sin() * cam.tp_distance,
                cam.tp_height + cam.pitch * cam.tp_distance,
                cam.yaw.cos() * cam.tp_distance,
            );
            cam_transform.translation = eye_pos + offset;
            cam_transform.look_at(eye_pos + Vec3::Y * 0.5, Vec3::Y);
        }
    }
}

fn grab_cursor(
    mut windows: Query<&mut Window>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    key_input: Res<ButtonInput<KeyCode>>,
) {
    let mut window = windows.single_mut();

    if mouse_button.just_pressed(MouseButton::Left) {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
    }

    if key_input.just_pressed(KeyCode::Escape) {
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
    }
}