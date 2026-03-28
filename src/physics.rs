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
const PLAYER_WIDTH: f32 = 0.3;
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

    let Some(&entity) = world.chunks.get(&chunk_pos) else {
        return false;
    };
    let Ok(chunk) = chunks.get(entity) else {
        return false;
    };

    let lx = bx.rem_euclid(CHUNK_SIZE as i32) as usize;
    let lz = bz.rem_euclid(CHUNK_SIZE as i32) as usize;
    let block = chunk.get_block(lx, by as usize, lz as usize);
    block.is_solid() && !matches!(block, BlockType::Water)
}

// NEW: Pre-check if we are standing on solid ground (used every frame)
fn is_grounded(
    world: &GameWorld,
    chunks: &Query<&crate::chunk::Chunk>,
    x: f32,
    feet_y: f32,
    z: f32,
) -> bool {
    let probe_block_y = (feet_y - 0.001).floor() as i32; // tiny probe just below feet

    let min_x = (x - PLAYER_WIDTH + 0.001).floor() as i32;
    let max_x = (x + PLAYER_WIDTH - 0.001).floor() as i32;
    let min_z = (z - PLAYER_WIDTH + 0.001).floor() as i32;
    let max_z = (z + PLAYER_WIDTH - 0.001).floor() as i32;

    for bx in min_x..=max_x {
        for bz in min_z..=max_z {
            if is_solid_at(world, chunks, bx, probe_block_y, bz) {
                return true;
            }
        }
    }
    false
}

fn aabb_overlaps_solid(
    world: &GameWorld,
    chunks: &Query<&crate::chunk::Chunk>,
    x: f32,
    feet_y: f32,
    z: f32,
) -> bool {
    let min_x = (x - PLAYER_WIDTH + 0.001).floor() as i32;
    let max_x = (x + PLAYER_WIDTH - 0.001).floor() as i32;
    let min_y = feet_y.floor() as i32;
    let max_y = (feet_y + PLAYER_HEIGHT - 0.001).floor() as i32;
    let min_z = (z - PLAYER_WIDTH + 0.001).floor() as i32;
    let max_z = (z + PLAYER_WIDTH - 0.001).floor() as i32;

    for bx in min_x..=max_x {
        for by in min_y..=max_y {
            for bz in min_z..=max_z {
                if is_solid_at(world, chunks, bx, by, bz) {
                    return true;
                }
            }
        }
    }
    false
}

fn apply_physics(
    time: Res<Time>,
    world: Res<GameWorld>,
    chunks: Query<&crate::chunk::Chunk>,
    mut query: Query<(&mut Transform, &mut Velocity, &mut Grounded), With<Player>>,
) {
    let dt = time.delta_seconds();

    for (mut transform, mut velocity, mut grounded) in query.iter_mut() {
        let pos = transform.translation;
        let feet_y = pos.y - PLAYER_HEIGHT;

        // === NEW GROUNDING LOGIC (eliminates idle sinking) ===
        let currently_grounded = is_grounded(&world, &chunks, pos.x, feet_y, pos.z);

        if currently_grounded && velocity.0.y <= 0.0 {
            velocity.0.y = 0.0;           // prevent any downward drift
            grounded.0 = true;
        } else {
            velocity.0.y += GRAVITY * dt; // only apply gravity when in air
            grounded.0 = false;
        }

        // === Y axis resolution (safety net + upward movement) ===
        let desired_feet_y = feet_y + velocity.0.y * dt;
        let new_feet_y = resolve_y(
            &world,
            &chunks,
            pos.x,
            desired_feet_y,
            pos.z,
            &mut velocity.0.y,
            &mut grounded,
        );

        // === X axis ===
        let desired_x = pos.x + velocity.0.x * dt;
        let new_x = resolve_axis(
            &world,
            &chunks,
            desired_x,
            new_feet_y,
            pos.z,
            pos.x,
            new_feet_y,
            pos.z,
            true,
            &mut velocity.0.x,
        );

        // === Z axis ===
        let desired_z = pos.z + velocity.0.z * dt;
        let new_z = resolve_axis(
            &world,
            &chunks,
            new_x,
            new_feet_y,
            desired_z,
            new_x,
            new_feet_y,
            pos.z,
            false,
            &mut velocity.0.z,
        );

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

// (unchanged - kept for upward movement and final safety)
fn resolve_y(
    world: &GameWorld,
    chunks: &Query<&crate::chunk::Chunk>,
    x: f32,
    new_feet_y: f32,
    z: f32,
    vel_y: &mut f32,
    grounded: &mut Grounded,
) -> f32 {
    if *vel_y <= 0.0 {
        let foot_block_y = (new_feet_y - 0.001).floor() as i32;
        let min_x = (x - PLAYER_WIDTH + 0.001).floor() as i32;
        let max_x = (x + PLAYER_WIDTH - 0.001).floor() as i32;
        let min_z = (z - PLAYER_WIDTH + 0.001).floor() as i32;
        let max_z = (z + PLAYER_WIDTH - 0.001).floor() as i32;

        let mut hit = false;
        for bx in min_x..=max_x {
            for bz in min_z..=max_z {
                if is_solid_at(world, chunks, bx, foot_block_y, bz) {
                    hit = true;
                    break;
                }
            }
            if hit { break; }
        }

        if hit {
            grounded.0 = true;
            *vel_y = 0.0;
            return foot_block_y as f32 + 1.0;
        }
        grounded.0 = false;
    } else {
        let head_block_y = (new_feet_y + PLAYER_HEIGHT - 0.001).floor() as i32;
        let min_x = (x - PLAYER_WIDTH + 0.001).floor() as i32;
        let max_x = (x + PLAYER_WIDTH - 0.001).floor() as i32;
        let min_z = (z - PLAYER_WIDTH + 0.001).floor() as i32;
        let max_z = (z + PLAYER_WIDTH - 0.001).floor() as i32;

        let mut hit = false;
        for bx in min_x..=max_x {
            for bz in min_z..=max_z {
                if is_solid_at(world, chunks, bx, head_block_y, bz) {
                    hit = true;
                    break;
                }
            }
            if hit { break; }
        }

        if hit {
            *vel_y = 0.0;
            return head_block_y as f32 - PLAYER_HEIGHT;
        }
        grounded.0 = false;
    }

    new_feet_y
}

fn resolve_axis(
    world: &GameWorld,
    chunks: &Query<&crate::chunk::Chunk>,
    desired_x: f32,
    desired_feet_y: f32,
    desired_z: f32,
    safe_x: f32,
    _safe_feet_y: f32,
    safe_z: f32,
    is_x: bool,
    vel: &mut f32,
) -> f32 {
    if !aabb_overlaps_solid(world, chunks, desired_x, desired_feet_y, desired_z) {
        return if is_x { desired_x } else { desired_z };
    }

    *vel = 0.0;
    if is_x { safe_x } else { safe_z }
}