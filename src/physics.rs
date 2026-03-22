use bevy::prelude::*;
use crate::camera::Player;
use crate::world::World as GameWorld;
use crate::chunk::{CHUNK_SIZE, CHUNK_HEIGHT};
use crate::block::BlockType;

#[derive(Component)]
pub struct Velocity(pub Vec3);

#[derive(Component)]
pub struct Grounded(pub bool);

const GRAVITY: f32 = -28.0;
const JUMP_VELOCITY: f32 = 9.0;
const PLAYER_WIDTH: f32 = 0.35;
pub const PLAYER_HEIGHT: f32 = 2.5;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (apply_physics, handle_jump).chain());
    }
}

fn is_solid_at(
    world: &GameWorld,
    chunks: &Query<&crate::chunk::Chunk>,
    bx: i32, by: i32, bz: i32,
) -> bool {
    if by < 0 || by >= CHUNK_HEIGHT as i32 {
        return false;
    }
    let chunk_x = bx.div_euclid(CHUNK_SIZE as i32);
    let chunk_z = bz.div_euclid(CHUNK_SIZE as i32);
    let chunk_pos = IVec3::new(chunk_x, 0, chunk_z);

    let Some(&entity) = world.chunks.get(&chunk_pos) else { return false; };
    let Ok(chunk) = chunks.get(entity) else { return false; };

    let lx = bx.rem_euclid(CHUNK_SIZE as i32) as usize;
    let lz = bz.rem_euclid(CHUNK_SIZE as i32) as usize;
    let block = chunk.get_block(lx, by as usize, lz as usize);
    block.is_solid() && !matches!(block, BlockType::Water)
}

fn apply_physics(
    time: Res<Time>,
    world: Res<GameWorld>,
    chunks: Query<&crate::chunk::Chunk>,
    mut query: Query<(&mut Transform, &mut Velocity, &mut Grounded), With<Player>>,
) {
    let dt = time.delta_seconds();

    for (mut transform, mut velocity, mut grounded) in query.iter_mut() {
        velocity.0.y += GRAVITY * dt;

        // transform.translation.y is the EYE (camera) position.
        // Feet = eye - PLAYER_HEIGHT.
        let eye = transform.translation;
        let feet_y = eye.y - PLAYER_HEIGHT;

        // Resolve Y in feet space
        let new_feet_y = resolve_y(
            &world, &chunks,
            eye.x, feet_y + velocity.0.y * dt, eye.z,
            &mut velocity.0.y, &mut grounded,
        );

        // Resolve X and Z in feet space
        let new_x = resolve_xz(&world, &chunks, eye.x + velocity.0.x * dt, new_feet_y, eye.z, true);
        let new_z = resolve_xz(&world, &chunks, new_x, new_feet_y, eye.z + velocity.0.z * dt, false);

        // Store back as eye position
        transform.translation = Vec3::new(new_x, new_feet_y + PLAYER_HEIGHT, new_z);
    }
}

fn handle_jump(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Velocity, &Grounded), With<Player>>,
) {
    for (mut velocity, grounded) in query.iter_mut() {
        if keyboard.just_pressed(KeyCode::Space) && grounded.0 {
            velocity.0.y = JUMP_VELOCITY;
        }
    }
}

fn resolve_y(
    world: &GameWorld,
    chunks: &Query<&crate::chunk::Chunk>,
    x: f32, new_feet_y: f32, z: f32,
    vel_y: &mut f32,
    grounded: &mut Grounded,
) -> f32 {
    let corners = player_corners(x, z);

    if *vel_y <= 0.0 {
        // Check the block directly under feet
        let foot_block_y = (new_feet_y - 0.001).floor() as i32;
        for (cx, cz) in &corners {
            if is_solid_at(world, chunks, *cx, foot_block_y, *cz) {
                grounded.0 = true;
                *vel_y = 0.0;
                // Feet land on TOP of the block: block_y + 1.0
                return foot_block_y as f32 + 1.0;
            }
        }
        grounded.0 = false;
    } else {
        // Check the block at head level (feet + PLAYER_HEIGHT)
        let head_block_y = (new_feet_y + PLAYER_HEIGHT).floor() as i32;
        for (cx, cz) in &corners {
            if is_solid_at(world, chunks, *cx, head_block_y, *cz) {
                *vel_y = 0.0;
                return head_block_y as f32 - PLAYER_HEIGHT - 0.001;
            }
        }
        grounded.0 = false;
    }

    new_feet_y
}

fn resolve_xz(
    world: &GameWorld,
    chunks: &Query<&crate::chunk::Chunk>,
    new_x: f32, feet_y: f32, new_z: f32,
    is_x: bool,
) -> f32 {
    let foot_block_y = feet_y.floor() as i32;
    let head_block_y = (feet_y + PLAYER_HEIGHT - 0.001).floor() as i32;

    if is_x {
        let bx = if new_x >= 0.0 {
            (new_x + PLAYER_WIDTH).floor() as i32
        } else {
            (new_x - PLAYER_WIDTH).floor() as i32
        };
        for by in foot_block_y..=head_block_y {
            let bz = new_z.floor() as i32;
            if is_solid_at(world, chunks, bx, by, bz) {
                return if new_x >= 0.0 {
                    bx as f32 - PLAYER_WIDTH - 0.001
                } else {
                    bx as f32 + 1.0 + PLAYER_WIDTH + 0.001
                };
            }
        }
        new_x
    } else {
        let bz = if new_z >= 0.0 {
            (new_z + PLAYER_WIDTH).floor() as i32
        } else {
            (new_z - PLAYER_WIDTH).floor() as i32
        };
        for by in foot_block_y..=head_block_y {
            let bx = new_x.floor() as i32;
            if is_solid_at(world, chunks, bx, by, bz) {
                return if new_z >= 0.0 {
                    bz as f32 - PLAYER_WIDTH - 0.001
                } else {
                    bz as f32 + 1.0 + PLAYER_WIDTH + 0.001
                };
            }
        }
        new_z
    }
}

fn player_corners(x: f32, z: f32) -> [(i32, i32); 4] {
    [
        ((x - PLAYER_WIDTH).floor() as i32, (z - PLAYER_WIDTH).floor() as i32),
        ((x + PLAYER_WIDTH).floor() as i32, (z - PLAYER_WIDTH).floor() as i32),
        ((x - PLAYER_WIDTH).floor() as i32, (z + PLAYER_WIDTH).floor() as i32),
        ((x + PLAYER_WIDTH).floor() as i32, (z + PLAYER_WIDTH).floor() as i32),
    ]
}