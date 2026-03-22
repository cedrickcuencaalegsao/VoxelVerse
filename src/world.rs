use crate::block::BlockType;
use crate::chunk::{CHUNK_HEIGHT, CHUNK_SIZE, Chunk};
use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use std::collections::HashMap;
use crate::block_registry::BlockRegistry;

#[derive(Resource)]
pub struct World {
    pub chunks: HashMap<IVec3, Entity>,
    pub noise: Perlin,
    pub render_distance: i32,
}

impl Default for World {
    fn default() -> Self {
        Self {
            chunks: HashMap::new(),
            // Using a fixed seed for consistency during testing
            noise: Perlin::new(42),
            render_distance: 6, // Increased slightly for better views
        }
    }
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<World>()
            .add_systems(Update, (generate_chunks, update_chunk_meshes));
    }
}

// --- NOISE CONFIGURATION ---
const SEED: u32 = 42;
const TERRAIN_SCALE: f64 = 0.01; // How "spread out" the terrain is
const DAMPENING: f64 = 0.5; // Overall height multiplier
const OCTAVES: usize = 7; // Detail layers
const PERSISTENCE: f64 = 0.5; // How much each octave contributes (0.5 = half as much as previous)
const LACUNARITY: f64 = 2.0; // How much frequency increases per octave

/// Calculates multi-octave Perlin noise
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

    total / max_value // Normalized to [-1, 1]
}

fn get_height(noise: &Perlin, x: i32, z: i32) -> usize {
    // --- ORIGIN SAFE ZONE ---
    // Force a flat grassy platform around 0,0 for the city.
    // 48 blocks radius = 3 chunks in every direction, fully guaranteed dry land.
    const CITY_RADIUS: f32 = 48.0;
    const CITY_HEIGHT: usize = 35; // comfortably above water level (28) and sand (29)

    let dist_from_origin = ((x as f32).powi(2) + (z as f32).powi(2)).sqrt();

    if dist_from_origin < CITY_RADIUS {
        return CITY_HEIGHT;
    }

    // Smooth blend from city height into natural terrain
    const BLEND_RADIUS: f32 = 80.0;
    let blend_t = if dist_from_origin < BLEND_RADIUS {
        let t = (dist_from_origin - CITY_RADIUS) / (BLEND_RADIUS - CITY_RADIUS);
        t * t * (3.0 - 2.0 * t) // smoothstep
    } else {
        1.0
    };

    // ... rest of your existing noise calculation ...
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
    let height_variation = detail_noise * (15.0 * mountain_blend + mountain_intensity * 40.0);
    let natural_height = (base_height + height_variation).clamp(1.0, CHUNK_HEIGHT as f64 - 1.0) as usize;

    // Blend between forced city height and natural terrain
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
                    // Water level
                    if y <= 28 {
                        BlockType::Water
                    } else {
                        BlockType::Air
                    }
                } else if y == height {
                    if height <= 29 {
                        BlockType::Sand
                    } else if height > 55 {
                        BlockType::Stone // High mountain peaks
                    } else {
                        BlockType::Grass
                    }
                } else if y > height - 3 {
                    if height <= 29 {
                        BlockType::Sand
                    } else {
                        BlockType::Dirt
                    }
                } else {
                    BlockType::Stone
                };

                chunk.set_block(x, y, z, block);
            }

            // Trees only on Grass and not too high/low
            if height > 30 && height < 50 {
                // Secondary noise for tree placement (Poisson-like)
                let tree_val = fbm_noise(
                    noise,
                    world_x as f64 * 0.5,
                    world_z as f64 * 0.5,
                    2,
                    0.5,
                    2.0,
                );
                if tree_val > 0.75 {
                    generate_tree(chunk, x, height + 1, z);
                }
            }
        }
    }
}

// --- Rest of your functions (generate_chunks, generate_tree, etc.) remain largely the same ---
// (Ensure they are included in your file below)

fn generate_chunks(
    mut commands: Commands,
    mut world: ResMut<World>,
    mut meshes: ResMut<Assets<Mesh>>,
    registry: Res<BlockRegistry>,
    camera_query: Query<&Transform, With<Camera>>,
) {
    // Wait until block.glb is fully loaded
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

    for x in -render_distance..=render_distance {
        for z in -render_distance..=render_distance {
            let chunk_pos = IVec3::new(camera_chunk.x + x, 0, camera_chunk.z + z);
            if world.chunks.contains_key(&chunk_pos) {
                continue;
            }

            let mut chunk = Chunk::new(chunk_pos);
            generate_terrain(&mut chunk, &world.noise);

            let mesh = chunk.generate_mesh();
            let mesh_handle = meshes.add(mesh);

            // Use the material straight from block.glb
            let material = registry.material.clone().unwrap();

            let entity = commands
                .spawn(PbrBundle {
                    mesh: mesh_handle,
                    material,
                    transform: Transform::from_translation(Vec3::ZERO),
                    ..default()
                })
                .insert(chunk)
                .id();

            world.chunks.insert(chunk_pos, entity);
        }
    }

    // ... despawn out-of-range chunks unchanged ...
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
                if y + dy >= CHUNK_HEIGHT {
                    continue;
                }
                let leaf_x = x as i32 + dx;
                let leaf_z = z as i32 + dz;
                if leaf_x >= 0
                    && leaf_x < CHUNK_SIZE as i32
                    && leaf_z >= 0
                    && leaf_z < CHUNK_SIZE as i32
                {
                    if dx.abs() + dz.abs() <= 3 {
                        // Don't replace wood with leaves
                        if chunk.get_block(leaf_x as usize, y + dy, leaf_z as usize)
                            == BlockType::Air
                        {
                            chunk.set_block(
                                leaf_x as usize,
                                y + dy,
                                leaf_z as usize,
                                BlockType::Leaves,
                            );
                        }
                    }
                }
            }
        }
    }
}

fn update_chunk_meshes(
    mut chunks: Query<(&Chunk, &Handle<Mesh>), Changed<Chunk>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (chunk, mesh_handle) in chunks.iter_mut() {
        let new_mesh = chunk.generate_mesh();
        if let Some(mesh) = meshes.get_mut(mesh_handle) {
            *mesh = new_mesh;
        }
    }
}

pub fn get_spawn_height(noise: &Perlin) -> f32 {
    // Returns FEET position = top of surface block
    get_height(noise, 0, 0) as f32 + 1.0
}

/// Mirrors the surface block logic from generate_terrain so both stay in sync.
fn get_surface_block(height: usize) -> BlockType {
    if height <= 28 {
        BlockType::Water
    } else if height <= 29 {
        BlockType::Sand
    } else if height > 55 {
        BlockType::Stone
    } else {
        BlockType::Grass
    }
}