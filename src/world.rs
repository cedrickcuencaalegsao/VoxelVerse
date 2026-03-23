use crate::block::BlockType;
use crate::block_registry::BlockRegistry;
use crate::chunk::{Chunk, CHUNK_HEIGHT, CHUNK_SIZE};
use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Resource)]
pub struct World {
    pub chunks: HashMap<IVec3, Entity>,
    pub noise: Perlin,
    pub render_distance: i32,
    pub seed: u32,
}

impl Default for World {
    fn default() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| (d.as_millis() & 0xFFFFFFFF) as u32)
            .unwrap_or(42);

        info!("World seed: {}", seed);

        Self {
            chunks: HashMap::new(),
            noise: Perlin::new(seed),
            render_distance: 4, // keep low — we're spawning real models now
            seed,
        }
    }
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<World>()
            .add_systems(Update, generate_chunks);
    }
}

const TERRAIN_SCALE: f64 = 0.01;
const DAMPENING: f64 = 0.6;
const OCTAVES: usize = 7;
const PERSISTENCE: f64 = 0.5;
const LACUNARITY: f64 = 2.0;

fn fbm_noise(
    noise: &Perlin,
    x: f64,
    z: f64,
    octaves: usize,
    persistence: f64,
    lacunarity: f64,
) -> f64 {
    let mut total = 0.0;
    let mut frequency = 1.0;
    let mut amplitude = 1.0;
    let mut max_value = 0.0;

    for _ in 0..octaves {
        total += noise.get([x * frequency, z * frequency]) * amplitude;
        max_value += amplitude;
        amplitude *= persistence;
        frequency *= lacunarity;
    }

    total / max_value
}

fn get_height(noise: &Perlin, x: i32, z: i32) -> usize {
    const CITY_RADIUS: f32 = 48.0;
    const CITY_HEIGHT: usize = 35;

    let dist_from_origin = ((x as f32).powi(2) + (z as f32).powi(2)).sqrt();

    if dist_from_origin < CITY_RADIUS {
        return CITY_HEIGHT;
    }

    const BLEND_RADIUS: f32 = 80.0;
    let blend_t = if dist_from_origin < BLEND_RADIUS {
        let t = (dist_from_origin - CITY_RADIUS) / (BLEND_RADIUS - CITY_RADIUS);
        t * t * (3.0 - 2.0 * t)
    } else {
        1.0
    };

    let x_f = x as f64 * TERRAIN_SCALE;
    let z_f = z as f64 * TERRAIN_SCALE;

    let dist = ((x as f64).powi(2) + (z as f64).powi(2)).sqrt();
    const FLAT_RADIUS: f64 = 64.0;
    const MOUNTAIN_RADIUS: f64 = 256.0;
    let mt = ((dist - FLAT_RADIUS) / (MOUNTAIN_RADIUS - FLAT_RADIUS)).clamp(0.0, 1.0);
    let mountain_blend = mt * mt * (3.0 - 2.0 * mt);

    let continent_noise = fbm_noise(noise, x_f * 0.5, z_f * 0.5, 3, 0.4, 2.0);
    let detail_noise = fbm_noise(noise, x_f, z_f, OCTAVES, PERSISTENCE, LACUNARITY);

    let base_height = 30.0;
    let mountain_intensity = (continent_noise + 1.0) * 0.5 * mountain_blend;
    let height_variation = detail_noise
        * (15.0 * mountain_blend + mountain_intensity * 40.0)
        * DAMPENING;

    let natural_height = (base_height + height_variation)
        .clamp(1.0, CHUNK_HEIGHT as f64 - 1.0) as usize;

    let blended = CITY_HEIGHT as f32 * (1.0 - blend_t) + natural_height as f32 * blend_t;
    blended.round() as usize
}

fn generate_terrain(chunk: &mut Chunk, noise: &Perlin) {
    for x in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            let world_x = chunk.position.x * CHUNK_SIZE as i32 + x as i32;
            let world_z = chunk.position.z * CHUNK_SIZE as i32 + z as i32;
            let height = get_height(noise, world_x, world_z);

            for y in 0..CHUNK_HEIGHT {
                let block = if y > height {
                    if y <= 28 { BlockType::Water } else { BlockType::Air }
                } else if y == height {
                    if height <= 29 { BlockType::Sand }
                    else if height > 55 { BlockType::Stone }
                    else { BlockType::Grass }
                } else if y > height.saturating_sub(3) {
                    if height <= 29 { BlockType::Sand } else { BlockType::Dirt }
                } else {
                    BlockType::Stone
                };

                chunk.set_block(x, y, z, block);
            }

            let dist_from_origin =
                ((world_x as f32).powi(2) + (world_z as f32).powi(2)).sqrt();
            if height > 30 && height < 50 && dist_from_origin > 50.0 {
                let tree_val = fbm_noise(
                    noise,
                    world_x as f64 * 0.5,
                    world_z as f64 * 0.5,
                    2, 0.5, 2.0,
                );
                if tree_val > 0.75 {
                    generate_tree(chunk, x, height + 1, z);
                }
            }
        }
    }
}

fn generate_chunks(
    mut commands: Commands,
    mut world: ResMut<World>,
    asset_server: Res<AssetServer>,
    camera_query: Query<&Transform, With<Camera>>,
    registry: Res<BlockRegistry>,
) {
    if !registry.loaded {
        return;
    }

    let camera_pos = if let Ok(t) = camera_query.get_single() {
        t.translation
    } else {
        return;
    };

    let camera_chunk = IVec3::new(
        (camera_pos.x / CHUNK_SIZE as f32).floor() as i32,
        0,
        (camera_pos.z / CHUNK_SIZE as f32).floor() as i32,
    );

    let render_distance = world.render_distance;

    for cx in -render_distance..=render_distance {
        for cz in -render_distance..=render_distance {
            let chunk_pos = IVec3::new(camera_chunk.x + cx, 0, camera_chunk.z + cz);
            if world.chunks.contains_key(&chunk_pos) {
                continue;
            }

            let mut chunk = Chunk::new(chunk_pos);
            generate_terrain(&mut chunk, &world.noise);

            let surface_blocks = chunk.get_surface_blocks();

            // Spawn a parent entity to hold all block instances for this chunk
            let chunk_entity = commands
                .spawn((
                    SpatialBundle::default(),
                    chunk,
                ))
                .with_children(|parent| {
                    for (lx, ly, lz, block_type) in surface_blocks {
                        let world_x = chunk_pos.x * CHUNK_SIZE as i32 + lx as i32;
                        let world_z = chunk_pos.z * CHUNK_SIZE as i32 + lz as i32;

                        // Pick the right GLB scene based on block type
                        let scene_path = block_scene_path(block_type);

                        parent.spawn(SceneBundle {
                            scene: asset_server.load(scene_path),
                            transform: Transform::from_xyz(
                                world_x as f32,
                                ly as f32,
                                world_z as f32,
                            ),
                            ..default()
                        });
                    }
                })
                .id();

            world.chunks.insert(chunk_pos, chunk_entity);
        }
    }

    // Despawn out-of-range chunks
    let chunks_to_remove: Vec<IVec3> = world
        .chunks
        .keys()
        .filter(|&&pos| {
            (pos.x - camera_chunk.x).abs() > render_distance
                || (pos.z - camera_chunk.z).abs() > render_distance
        })
        .copied()
        .collect();

    for chunk_pos in chunks_to_remove {
        if let Some(entity) = world.chunks.remove(&chunk_pos) {
            commands.entity(entity).despawn_recursive();
        }
    }
}

/// Map block type to GLB scene path.
/// Right now all blocks use block.glb — you can add more GLBs later.
fn block_scene_path(block: BlockType) -> &'static str {
    match block {
        BlockType::Grass | BlockType::Dirt => "block.glb#Scene0",
        BlockType::Stone => "block.glb#Scene0",
        BlockType::Sand  => "block.glb#Scene0",
        BlockType::Wood  => "block.glb#Scene0",
        BlockType::Leaves => "block.glb#Scene0",
        BlockType::Water => "block.glb#Scene0",
        BlockType::Air   => "block.glb#Scene0",
    }
}

fn generate_tree(chunk: &mut Chunk, x: usize, y: usize, z: usize) {
    let trunk_height = 5;
    for dy in 0..trunk_height {
        if y + dy < CHUNK_HEIGHT {
            chunk.set_block(x, y + dy, z, BlockType::Wood);
        }
    }
    for dx in -2..=2_i32 {
        for dz in -2..=2_i32 {
            for dy in trunk_height - 1..trunk_height + 2 {
                if y + dy >= CHUNK_HEIGHT { continue; }
                let leaf_x = x as i32 + dx;
                let leaf_z = z as i32 + dz;
                if leaf_x >= 0 && leaf_x < CHUNK_SIZE as i32
                    && leaf_z >= 0 && leaf_z < CHUNK_SIZE as i32
                {
                    if dx.abs() + dz.abs() <= 3 {
                        if chunk.get_block(leaf_x as usize, y + dy, leaf_z as usize)
                            == BlockType::Air
                        {
                            chunk.set_block(
                                leaf_x as usize, y + dy, leaf_z as usize,
                                BlockType::Leaves,
                            );
                        }
                    }
                }
            }
        }
    }
}

pub fn get_spawn_height(noise: &Perlin) -> f32 {
    get_height(noise, 0, 0) as f32 + 1.0
}