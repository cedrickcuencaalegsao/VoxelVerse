use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use crate::block::BlockType;
use crate::chunk::{Chunk, CHUNK_SIZE, CHUNK_HEIGHT};
use std::collections::HashMap;

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
const TERRAIN_SCALE: f64 = 0.01;   // How "spread out" the terrain is
const DAMPENING: f64 = 0.5;       // Overall height multiplier
const OCTAVES: usize = 7;         // Detail layers
const PERSISTENCE: f64 = 0.5;     // How much each octave contributes (0.5 = half as much as previous)
const LACUNARITY: f64 = 2.0;      // How much frequency increases per octave

/// Calculates multi-octave Perlin noise
fn fbm_noise(noise: &Perlin, x: f64, z: f64, octaves: usize, persistence: f64, lacunarity: f64) -> f64 {
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
    let x_f = x as f64 * TERRAIN_SCALE;
    let z_f = z as f64 * TERRAIN_SCALE;

    // 1. Continentalness: Large scale variation (plains vs mountains)
    let continent_noise = fbm_noise(noise, x_f * 0.5, z_f * 0.5, 3, 0.4, 2.0);
    
    // 2. Erosion/Detail: The "mountainous" noise
    let detail_noise = fbm_noise(noise, x_f, z_f, OCTAVES, PERSISTENCE, LACUNARITY);

    // Calculate base height based on continentalness
    // If continent_noise is high, we have mountains. If low, we have plains.
    let base_height = 30.0;
    let mountain_intensity = (continent_noise + 1.0) * 0.5; // 0 to 1
    
    // Use the mountain intensity to weight how much the detail noise affects height
    let height_variation = detail_noise * (15.0 + mountain_intensity * 40.0);
    
    let final_height = base_height + height_variation;

    final_height.clamp(1.0, CHUNK_HEIGHT as f64 - 1.0) as usize
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
                    if y <= 28 { BlockType::Water } else { BlockType::Air }
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
                let tree_val = fbm_noise(noise, world_x as f64 * 0.5, world_z as f64 * 0.5, 2, 0.5, 2.0);
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
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera_query: Query<&Transform, With<Camera>>,
) {
    let camera_pos = if let Ok(camera_transform) = camera_query.get_single() {
        camera_transform.translation
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
            
            let material = materials.add(StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: 0.8,
                metallic: 0.0,
                cull_mode: None,
                ..default()
            });

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
            commands.entity(entity).despawn();
        }
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
                if leaf_x >= 0 && leaf_x < CHUNK_SIZE as i32 && leaf_z >= 0 && leaf_z < CHUNK_SIZE as i32 {
                    if dx.abs() + dz.abs() <= 3 {
                        // Don't replace wood with leaves
                        if chunk.get_block(leaf_x as usize, y + dy, leaf_z as usize) == BlockType::Air {
                            chunk.set_block(leaf_x as usize, y + dy, leaf_z as usize, BlockType::Leaves);
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