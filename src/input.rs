use bevy::prelude::*;
use crate::camera::{Player, PlayerCamera};

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, player_movement);
    }
}

fn player_movement(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&Player, &PlayerCamera, &mut Transform)>,
) {
    for (player, camera, mut transform) in query.iter_mut() {
        let mut velocity = Vec3::ZERO;
        let forward = Vec3::new(camera.yaw.sin(), 0.0, camera.yaw.cos());
        let right = Vec3::new(camera.yaw.cos(), 0.0, -camera.yaw.sin());

        // Forward/Backward
        if keyboard.pressed(KeyCode::KeyS) {
            velocity += forward;
        }
        if keyboard.pressed(KeyCode::KeyW) {
            velocity -= forward;
        }

        // Left/Right
        if keyboard.pressed(KeyCode::KeyA) {
            velocity -= right;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            velocity += right;
        }

        // Up/Down
        if keyboard.pressed(KeyCode::Space) {
            velocity.y += 1.0;
        }
        if keyboard.pressed(KeyCode::ShiftLeft) {
            velocity.y -= 1.0;
        }

        // Normalize and apply speed
        if velocity.length() > 0.0 {
            velocity = velocity.normalize();
            
            let speed = if keyboard.pressed(KeyCode::ControlLeft) {
                player.speed * player.sprint_multiplier
            } else {
                player.speed
            };

            transform.translation += velocity * speed * time.delta_seconds();
        }
    }
}