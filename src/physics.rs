use bevy::prelude::*;
use crate::camera::Player;
use crate::world::World;
use crate::chunk::{Chunk, CHUNK_SIZE, CHUNK_HEIGHT};

#[derive(Component)]
pub struct Velocity(pub Vec3);

#[derive(Component)]
pub struct Grounded(pub bool);

pub const PLAYER_HEIGHT: f32 = 3.0;
pub const EYE_HEIGHT: f32 = 2.6;
const PLAYER_RADIUS: f32 = 0.35;
const GRAVITY: f32 = -24.0;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (apply_gravity, check_collision));
    }
}

fn apply_gravity(
    time: Res<Time>,
    world: Res<World>,
    chunks: Query<&Chunk>,
    mut query: Query<(&mut Transform, &mut Velocity, &mut Grounded), With<Player>>,
) {
    for (mut transform, mut velocity, mut grounded) in query.iter_mut() {
        let dt = time.delta_seconds();
        let mut base_pos = transform.translation - Vec3::Y * EYE_HEIGHT;

        if !grounded.0 {
            velocity.0.y += GRAVITY * dt;
        } else if velocity.0.y < 0.0 {
            velocity.0.y = 0.0;
        }

        let movement = velocity.0 * dt;

        let candidate_x = base_pos + Vec3::new(movement.x, 0.0, 0.0);
        if !collides_at(&world, &chunks, candidate_x) {
            base_pos.x = candidate_x.x;
        } else {
            velocity.0.x = 0.0;
        }

        let candidate_z = base_pos + Vec3::new(0.0, 0.0, movement.z);
        if !collides_at(&world, &chunks, candidate_z) {
            base_pos.z = candidate_z.z;
        } else {
            velocity.0.z = 0.0;
        }

        let candidate_y = base_pos + Vec3::new(0.0, movement.y, 0.0);
        if movement.y <= 0.0 {
            if let Some(ground_height) = find_ground_height(&world, &chunks, candidate_y) {
                if candidate_y.y < ground_height {
                    base_pos.y = ground_height;
                    velocity.0.y = 0.0;
                    grounded.0 = true;
                } else {
                    base_pos.y = candidate_y.y;
                    grounded.0 = false;
                }
            } else {
                base_pos.y = candidate_y.y;
                grounded.0 = false;
            }
        } else {
            if collides_at(&world, &chunks, candidate_y) {
                velocity.0.y = 0.0;
            } else {
                base_pos.y = candidate_y.y;
            }
            grounded.0 = false;
        }

        transform.translation = base_pos + Vec3::Y * EYE_HEIGHT;
    }
}

fn check_collision(
    world: Res<World>,
    chunks: Query<&Chunk>,
    mut query: Query<(&Transform, &mut Grounded), With<Player>>,
) {
    for (transform, mut grounded) in query.iter_mut() {
        let base_pos = transform.translation - Vec3::Y * EYE_HEIGHT;
        grounded.0 = find_ground_height(&world, &chunks, base_pos).is_some();
    }
}

pub fn world_to_block_pos(world_pos: Vec3) -> IVec3 {
    IVec3::new(
        world_pos.x.floor() as i32,
        world_pos.y.floor() as i32,
        world_pos.z.floor() as i32,
    )
}

fn is_solid_block(world: &World, chunks: &Query<&Chunk>, block_pos: IVec3) -> bool {
    if block_pos.y < 0 || block_pos.y >= CHUNK_HEIGHT as i32 {
        return false;
    }

    let chunk_pos = IVec3::new(
        block_pos.x.div_euclid(CHUNK_SIZE as i32),
        0,
        block_pos.z.div_euclid(CHUNK_SIZE as i32),
    );

    let Some(&chunk_entity) = world.chunks.get(&chunk_pos) else {
        return false;
    };
    let Ok(chunk) = chunks.get(chunk_entity) else {
        return false;
    };

    let local_x = block_pos.x.rem_euclid(CHUNK_SIZE as i32) as usize;
    let local_z = block_pos.z.rem_euclid(CHUNK_SIZE as i32) as usize;
    let local_y = block_pos.y as usize;

    chunk.get_block(local_x, local_y, local_z).is_solid()
}

fn collides_at(world: &World, chunks: &Query<&Chunk>, base_pos: Vec3) -> bool {
    let offsets = [
        Vec2::new(PLAYER_RADIUS, PLAYER_RADIUS),
        Vec2::new(-PLAYER_RADIUS, PLAYER_RADIUS),
        Vec2::new(PLAYER_RADIUS, -PLAYER_RADIUS),
        Vec2::new(-PLAYER_RADIUS, -PLAYER_RADIUS),
    ];

    for offset in offsets {
        let foot = Vec3::new(
            base_pos.x + offset.x,
            base_pos.y + 0.1,
            base_pos.z + offset.y,
        );
        let head = Vec3::new(
            base_pos.x + offset.x,
            base_pos.y + PLAYER_HEIGHT - 0.1,
            base_pos.z + offset.y,
        );

        if is_solid_block(world, chunks, world_to_block_pos(foot))
            || is_solid_block(world, chunks, world_to_block_pos(head))
        {
            return true;
        }
    }

    false
}

fn find_ground_height(world: &World, chunks: &Query<&Chunk>, base_pos: Vec3) -> Option<f32> {
    let offsets = [
        Vec2::new(PLAYER_RADIUS, PLAYER_RADIUS),
        Vec2::new(-PLAYER_RADIUS, PLAYER_RADIUS),
        Vec2::new(PLAYER_RADIUS, -PLAYER_RADIUS),
        Vec2::new(-PLAYER_RADIUS, -PLAYER_RADIUS),
    ];

    let mut highest: Option<f32> = None;
    for offset in offsets {
        let probe = Vec3::new(
            base_pos.x + offset.x,
            base_pos.y - 0.05,
            base_pos.z + offset.y,
        );
        let block_pos = world_to_block_pos(probe);
        if is_solid_block(world, chunks, block_pos) {
            let top = block_pos.y as f32 + 1.0;
            highest = Some(highest.map_or(top, |current| current.max(top)));
        }
    }

    highest
}