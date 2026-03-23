use bevy::prelude::*;
use crate::camera::{Player, PlayerCamera};
use crate::physics::Velocity;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, player_movement);
    }
}

fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&Player, &PlayerCamera, &mut Velocity), With<Player>>,
) {
    for (player, camera, mut velocity) in query.iter_mut() {
        let forward = Vec3::new(camera.yaw.sin(), 0.0, camera.yaw.cos());
        let right = Vec3::new(camera.yaw.cos(), 0.0, -camera.yaw.sin());

        let mut horizontal = Vec3::ZERO;
        if keyboard.pressed(KeyCode::KeyW) { horizontal -= forward; }
        if keyboard.pressed(KeyCode::KeyS) { horizontal += forward; }
        if keyboard.pressed(KeyCode::KeyA) { horizontal -= right; }
        if keyboard.pressed(KeyCode::KeyD) { horizontal += right; }

        let shift_held = keyboard.pressed(KeyCode::ShiftLeft)
            || keyboard.pressed(KeyCode::ShiftRight);
        let digit_held = keyboard.pressed(KeyCode::Digit1)
            || keyboard.pressed(KeyCode::Digit2)
            || keyboard.pressed(KeyCode::Digit3)
            || keyboard.pressed(KeyCode::Digit4);

        let sprinting = shift_held && !digit_held && keyboard.pressed(KeyCode::KeyW);

        let speed = if sprinting {
            player.speed * player.sprint_multiplier
        } else {
            player.speed
        };

        if horizontal.length_squared() > 0.0 {
            horizontal = horizontal.normalize() * speed;
        }

        velocity.0.x = horizontal.x;
        velocity.0.z = horizontal.z;
    }
}