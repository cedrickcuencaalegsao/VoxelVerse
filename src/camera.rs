use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use crate::physics::{Velocity, Grounded, EYE_HEIGHT};
use crate::world::{World, get_height, WATER_LEVEL};

#[derive(Component)]
pub struct PlayerCamera {
    pub sensitivity: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for PlayerCamera {
    fn default() -> Self {
        Self {
            sensitivity: 0.002,
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

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

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(Update, (mouse_look, grab_cursor));
    }
}

fn setup_camera(mut commands: Commands, world: Res<World>) {
    let spawn_x = 0;
    let spawn_z = 0;
    let ground_height = get_height(&world.noise, spawn_x, spawn_z) as f32;
    let safe_ground = ground_height.max(WATER_LEVEL as f32);
    let spawn_y = safe_ground + EYE_HEIGHT + 0.2;

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(spawn_x as f32, spawn_y, spawn_z as f32)
                .looking_at(Vec3::new(10.0, spawn_y, 10.0), Vec3::Y),
            ..default()
        },
        PlayerCamera::default(),
        Player::default(),
        Velocity(Vec3::ZERO),
        Grounded(false),
    ));
}

fn mouse_look(
    mut mouse_motion: EventReader<bevy::input::mouse::MouseMotion>,
    mut camera_query: Query<(&mut PlayerCamera, &mut Transform)>,
) {
    let mut delta = Vec2::ZERO;
    for event in mouse_motion.read() {
        delta += event.delta;
    }

    if delta == Vec2::ZERO {
        return;
    }

    for (mut camera, mut transform) in camera_query.iter_mut() {
        camera.yaw -= delta.x * camera.sensitivity;
        camera.pitch -= delta.y * camera.sensitivity;
        camera.pitch = camera.pitch.clamp(-1.54, 1.54); // Limit pitch to prevent flipping

        // Apply rotation
        let yaw_quat = Quat::from_rotation_y(camera.yaw);
        let pitch_quat = Quat::from_rotation_x(camera.pitch);
        transform.rotation = yaw_quat * pitch_quat;
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