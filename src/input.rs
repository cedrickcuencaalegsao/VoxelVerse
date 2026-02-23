use bevy::prelude::*;
use crate::camera::{Player, PlayerCamera};
use crate::physics::{Velocity, Grounded};

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, player_movement);
    }
}

fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&Player, &PlayerCamera, &mut Velocity, &Grounded)>,
) {
    for (player, camera, mut velocity, grounded) in query.iter_mut() {
        let mut input_dir = Vec3::ZERO;
        let forward = Vec3::new(camera.yaw.sin(), 0.0, camera.yaw.cos());
        let right = Vec3::new(camera.yaw.cos(), 0.0, -camera.yaw.sin());

        // Forward/Backward
        if keyboard.pressed(KeyCode::KeyS) {
            input_dir += forward;
        }
        if keyboard.pressed(KeyCode::KeyW) {
            input_dir -= forward;
        }

        // Left/Right
        if keyboard.pressed(KeyCode::KeyA) {
            input_dir -= right;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            input_dir += right;
        }

        let speed = if keyboard.pressed(KeyCode::ControlLeft) {
            player.speed * player.sprint_multiplier
        } else {
            player.speed
        };

        if input_dir.length() > 0.0 {
            input_dir = input_dir.normalize();
            velocity.0.x = input_dir.x * speed;
            velocity.0.z = input_dir.z * speed;
        } else {
            velocity.0.x = 0.0;
            velocity.0.z = 0.0;
        }

        if keyboard.just_pressed(KeyCode::Space) && grounded.0 {
            velocity.0.y = 8.0;
        }
    }
}