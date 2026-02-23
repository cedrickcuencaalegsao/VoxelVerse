use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use crate::block::{BlockType, Face};
use crate::chunk::{Chunk, CHUNK_SIZE, CHUNK_HEIGHT};
use std::collections::HashMap;

#[derive(Resource)]
pub struct World {
    pub chunks: HashMap<IVec3, Entity>,
    pub noise: Perlin,
    pub render_distance: i32,
}

#[derive(Resource)]
pub struct ChunkMaterial {
    pub handle: Handle<StandardMaterial>,
}

impl Default for World {
    fn default() -> Self {
        Self {
            chunks: HashMap::new(),
            // Using a fixed seed for consistency during testing
            noise: Perlin::new(SEED),
            render_distance: 6, // Increased slightly for better views
        }
    }
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<World>()
            .init_resource::<BlockSelection>()
            .add_systems(Startup, setup_chunk_material)
            .add_systems(
                Update,
                (generate_chunks, update_chunk_meshes, update_block_selection),
            );
    }
}

// --- NOISE CONFIGURATION ---
const SEED: u32 = 42;
const TERRAIN_SCALE: f64 = 0.01;   // How "spread out" the terrain is
const OCTAVES: usize = 7;          // Detail layers
const PERSISTENCE: f64 = 0.5;      // How much each octave contributes (0.5 = half as much as previous)
const LACUNARITY: f64 = 2.0;       // How much frequency increases per octave
pub const WATER_LEVEL: usize = 28;
const BEACH_DEPTH: usize = 3;
const TEMP_SCALE: f64 = 0.002;     // Large-scale temperature variation
const MOISTURE_SCALE: f64 = 0.002; // Large-scale moisture variation
const CHUNKS_PER_FRAME: usize = 2;

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

fn ridge_noise(value: f64) -> f64 {
    let ridged = 1.0 - value.abs(); // [0, 1]
    ridged * ridged
}

pub fn get_height(noise: &Perlin, x: i32, z: i32) -> usize {
    let x_f = x as f64 * TERRAIN_SCALE;
    let z_f = z as f64 * TERRAIN_SCALE;

    // 1. Continentalness: Large scale variation (plains vs mountains)
    let continent_noise = fbm_noise(noise, x_f * 0.5, z_f * 0.5, 3, 0.4, 2.0);

    // 2. Detail: fine-scale noise
    let detail_noise = fbm_noise(noise, x_f, z_f, OCTAVES, PERSISTENCE, LACUNARITY);

    // 3. Ridges: sharper peaks to break up smooth hills
    let ridge_source = fbm_noise(noise, x_f * 1.8, z_f * 1.8, 4, 0.5, 2.2);
    let ridge = ridge_noise(ridge_source);

    // Weight variations by continentalness
    let mountain_intensity = (continent_noise + 1.0) * 0.5; // 0 to 1
    let base_height = 26.0 + mountain_intensity * 8.0;
    let height_variation = detail_noise * (12.0 + mountain_intensity * 28.0)
        + ridge * (8.0 + mountain_intensity * 16.0);

    let final_height = base_height + height_variation;
    final_height.clamp(1.0, CHUNK_HEIGHT as f64 - 1.0) as usize
}

/// Returns the block at the given world position. Returns Air for positions outside loaded chunks or bounds.
pub fn get_block_at(world: &World, chunks: &Query<&Chunk>, block_pos: IVec3) -> BlockType {
    if block_pos.y < 0 || block_pos.y >= CHUNK_HEIGHT as i32 {
        return BlockType::Air;
    }

    let chunk_pos = IVec3::new(
        block_pos.x.div_euclid(CHUNK_SIZE as i32),
        0,
        block_pos.z.div_euclid(CHUNK_SIZE as i32),
    );

    let Some(&chunk_entity) = world.chunks.get(&chunk_pos) else {
        return BlockType::Air;
    };

    let Ok(chunk) = chunks.get(chunk_entity) else {
        return BlockType::Air;
    };

    let local_x = block_pos.x.rem_euclid(CHUNK_SIZE as i32) as usize;
    let local_z = block_pos.z.rem_euclid(CHUNK_SIZE as i32) as usize;
    let local_y = block_pos.y as usize;

    chunk.get_block(local_x, local_y, local_z)
}

/// Result of a voxel raycast hit.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct RaycastHit {
    pub block_pos: IVec3,
    pub face: Face,
    pub distance: f32,
}

const RAYCAST_EPSILON: f32 = 0.0001;
const RAYCAST_MAX_STEPS: i32 = 100;

/// Performs a 3D DDA voxel raycast. Returns the first solid block hit (excluding Water for selection).
/// Uses Amanatides-Woo style traversal.
pub fn raycast(
    world: &World,
    chunks: &Query<&Chunk>,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<RaycastHit> {
    let dir = direction.normalize_or_zero();
    if dir.length_squared() < 0.0001 {
        return None;
    }

    let mut pos = IVec3::new(
        origin.x.floor() as i32,
        origin.y.floor() as i32,
        origin.z.floor() as i32,
    );

    let step = IVec3::new(
        if dir.x >= 0.0 { 1 } else { -1 },
        if dir.y >= 0.0 { 1 } else { -1 },
        if dir.z >= 0.0 { 1 } else { -1 },
    );

    let delta = Vec3::new(
        if dir.x.abs() < RAYCAST_EPSILON {
            f32::MAX
        } else {
            1.0 / dir.x.abs()
        },
        if dir.y.abs() < RAYCAST_EPSILON {
            f32::MAX
        } else {
            1.0 / dir.y.abs()
        },
        if dir.z.abs() < RAYCAST_EPSILON {
            f32::MAX
        } else {
            1.0 / dir.z.abs()
        },
    );

    let block_center = Vec3::new(
        pos.x as f32 + 0.5,
        pos.y as f32 + 0.5,
        pos.z as f32 + 0.5,
    );
    let offset = origin - block_center;
    let mut t_max = Vec3::new(
        if dir.x >= 0.0 {
            (0.5 - offset.x) / dir.x
        } else {
            (-0.5 - offset.x) / dir.x
        },
        if dir.y >= 0.0 {
            (0.5 - offset.y) / dir.y
        } else {
            (-0.5 - offset.y) / dir.y
        },
        if dir.z >= 0.0 {
            (0.5 - offset.z) / dir.z
        } else {
            (-0.5 - offset.z) / dir.z
        },
    )
    .max(Vec3::ZERO);
    let mut prev_axis = 0u8;
    let mut prev_step = 1i32;
    let mut steps = 0i32;

    for _ in 0..RAYCAST_MAX_STEPS {
        if pos.y < 0 || pos.y >= CHUNK_HEIGHT as i32 {
            return None;
        }

        let block = get_block_at(world, chunks, pos);
        let is_solid_for_selection = block.is_solid() && block != BlockType::Water;

        if is_solid_for_selection {
            if steps > 0 {
                let face = step_axis_to_face(prev_axis, prev_step);
                let distance = if prev_axis == 0 {
                    t_max.x - delta.x
                } else if prev_axis == 1 {
                    t_max.y - delta.y
                } else {
                    t_max.z - delta.z
                };
                if distance <= max_distance && distance >= 0.0 {
                    return Some(RaycastHit {
                        block_pos: pos,
                        face,
                        distance,
                    });
                }
            }
        }

        if t_max.x <= t_max.y && t_max.x <= t_max.z {
            prev_axis = 0;
            prev_step = step.x;
            pos.x += step.x;
            t_max.x += delta.x;
        } else if t_max.y <= t_max.x && t_max.y <= t_max.z {
            prev_axis = 1;
            prev_step = step.y;
            pos.y += step.y;
            t_max.y += delta.y;
        } else {
            prev_axis = 2;
            prev_step = step.z;
            pos.z += step.z;
            t_max.z += delta.z;
        };
        steps += 1;

        if t_max.x.min(t_max.y).min(t_max.z) > max_distance {
            return None;
        }
    }

    None
}

/// Current block the player is looking at (from raycast). None if no block in range.
#[derive(Resource, Default)]
pub struct BlockSelection(pub Option<RaycastHit>);

const RAYCAST_MAX_DISTANCE: f32 = 8.0;

fn update_block_selection(
    world: Res<World>,
    chunks: Query<&Chunk>,
    camera_query: Query<&Transform, With<Camera>>,
    mut selection: ResMut<BlockSelection>,
) {
    let Ok(transform) = camera_query.get_single() else {
        selection.0 = None;
        return;
    };

    let origin = transform.translation;
    let direction = transform.rotation * Vec3::NEG_Z;

    selection.0 = raycast(&world, &chunks, origin, direction, RAYCAST_MAX_DISTANCE);
}

fn step_axis_to_face(axis: u8, step_val: i32) -> Face {
    match (axis, step_val) {
        (0, -1) => Face::West,
        (0, 1) => Face::East,
        (1, -1) => Face::Bottom,
        (1, 1) => Face::Top,
        (2, -1) => Face::South,
        (2, 1) => Face::North,
        _ => Face::North,
    }
}

fn generate_terrain(chunk: &mut Chunk, noise: &Perlin) {
    for x in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            let world_x = chunk.position.x * CHUNK_SIZE as i32 + x as i32;
            let world_z = chunk.position.z * CHUNK_SIZE as i32 + z as i32;

            let height = get_height(noise, world_x, world_z);
            let world_xf = world_x as f64;
            let world_zf = world_z as f64;
            let temperature = (fbm_noise(noise, world_xf * TEMP_SCALE, world_zf * TEMP_SCALE, 3, 0.5, 2.0) + 1.0) * 0.5;
            let moisture = (fbm_noise(noise, world_xf * MOISTURE_SCALE, world_zf * MOISTURE_SCALE, 3, 0.5, 2.0) + 1.0) * 0.5;

            let continent_noise = fbm_noise(noise, world_xf * TERRAIN_SCALE * 0.5, world_zf * TERRAIN_SCALE * 0.5, 3, 0.4, 2.0);
            let mountain_intensity = (continent_noise + 1.0) * 0.5;

            let is_beach = height <= WATER_LEVEL + BEACH_DEPTH;
            let is_desert = temperature > 0.6 && moisture < 0.35;
            let is_mountain = mountain_intensity > 0.75 && height > WATER_LEVEL + 12;

            let (surface_block, subsurface_block) = if is_beach || is_desert {
                (BlockType::Sand, BlockType::Sand)
            } else if is_mountain {
                (BlockType::Stone, BlockType::Stone)
            } else if moisture > 0.65 {
                (BlockType::Grass, BlockType::Dirt)
            } else {
                (BlockType::Grass, BlockType::Dirt)
            };

            for y in 0..CHUNK_HEIGHT {
                let block = if y > height {
                    // Water level
                    if y <= WATER_LEVEL { BlockType::Water } else { BlockType::Air }
                } else if y == height {
                    surface_block
                } else if y >= height.saturating_sub(3) {
                    subsurface_block
                } else {
                    BlockType::Stone
                };

                chunk.set_block(x, y, z, block);
            }

            // Trees only on Grass and not too high/low
            if height > WATER_LEVEL + 2 && height < 50 {
                // Secondary noise for tree placement (Poisson-like)
                let tree_val = fbm_noise(noise, world_x as f64 * 0.5, world_z as f64 * 0.5, 2, 0.5, 2.0);
                if tree_val > 0.75 && chunk.get_block(x, height, z) == BlockType::Grass {
                    generate_tree(chunk, x, height + 1, z);
                }
            }
        }
    }
}

fn setup_chunk_material(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.8,
        metallic: 0.0,
        cull_mode: None,
        ..default()
    });
    commands.insert_resource(ChunkMaterial { handle: material });
}

// --- Rest of your functions (generate_chunks, generate_tree, etc.) remain largely the same ---
// (Ensure they are included in your file below)

fn generate_chunks(
    mut commands: Commands,
    mut world: ResMut<World>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_material: Res<ChunkMaterial>,
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

    let mut chunks_to_spawn = Vec::new();
    for x in -render_distance..=render_distance {
        for z in -render_distance..=render_distance {
            let chunk_pos = IVec3::new(camera_chunk.x + x, 0, camera_chunk.z + z);

            if world.chunks.contains_key(&chunk_pos) {
                continue;
            }

            chunks_to_spawn.push(chunk_pos);
        }
    }

    chunks_to_spawn.sort_by_key(|pos| {
        let dx = pos.x - camera_chunk.x;
        let dz = pos.z - camera_chunk.z;
        dx * dx + dz * dz
    });

    for chunk_pos in chunks_to_spawn.into_iter().take(CHUNKS_PER_FRAME) {
        let mut chunk = Chunk::new(chunk_pos);
        generate_terrain(&mut chunk, &world.noise);

        let mesh = chunk.generate_mesh();
        let mesh_handle = meshes.add(mesh);

        let entity = commands
            .spawn(PbrBundle {
                mesh: mesh_handle,
                material: chunk_material.handle.clone(),
                transform: Transform::from_translation(Vec3::ZERO),
                ..default()
            })
            .insert(chunk)
            .id();

        world.chunks.insert(chunk_pos, entity);
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