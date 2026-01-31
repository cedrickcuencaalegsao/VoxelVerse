use bevy::prelude::*;
use crate::camera::Player;
use crate::world::World;
use crate::chunk::CHUNK_SIZE;
use crate::block::BlockType;

#[derive(Component)]
pub struct Velocity(pub Vec3);

#[derive(Component)]
pub struct Grounded(pub bool);

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (apply_gravity, check_collision));
    }
}

fn apply_gravity(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut Velocity, &Grounded), With<Player>>,
) {
    for (mut transform, mut velocity, grounded) in query.iter_mut() {
        // Simple flying mode for now - no gravity
        // In creative mode, space/shift for up/down movement
        if !keyboard.pressed(KeyCode::Space) && !keyboard.pressed(KeyCode::ShiftLeft) {
            if grounded.0 {
                velocity.0.y = 0.0;
            }
        }

        transform.translation += velocity.0 * time.delta_seconds();
    }
}

fn check_collision(
    world: Res<World>,
    mut query: Query<(&Transform, &mut Grounded), With<Player>>,
) {
    for (transform, mut grounded) in query.iter_mut() {
        let player_pos = transform.translation;
        
        // Check if block below player
        let below_pos = IVec3::new(
            player_pos.x.floor() as i32,
            (player_pos.y - 2.0).floor() as i32,
            player_pos.z.floor() as i32,
        );

        let chunk_pos = IVec3::new(
            below_pos.x.div_euclid(CHUNK_SIZE as i32),
            0,
            below_pos.z.div_euclid(CHUNK_SIZE as i32),
        );

        if let Some(&_chunk_entity) = world.chunks.get(&chunk_pos) {
            grounded.0 = true;
        } else {
            grounded.0 = false;
        }
    }
}

pub fn world_to_block_pos(world_pos: Vec3) -> IVec3 {
    IVec3::new(
        world_pos.x.floor() as i32,
        world_pos.y.floor() as i32,
        world_pos.z.floor() as i32,
    )
}