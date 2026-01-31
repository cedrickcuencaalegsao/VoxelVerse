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
            noise: Perlin::new(42),
            render_distance: 4,
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
                base_color: Color::srgb(0.3, 0.7, 0.3),
                perceptual_roughness: 0.9,
                metallic: 0.0,
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

    // Remove chunks outside render distance
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

fn generate_terrain(chunk: &mut Chunk, noise: &Perlin) {
    for x in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            let world_x = chunk.position.x * CHUNK_SIZE as i32 + x as i32;
            let world_z = chunk.position.z * CHUNK_SIZE as i32 + z as i32;

            let height = get_height(noise, world_x, world_z);

            for y in 0..CHUNK_HEIGHT {
                let block = if y > height {
                    BlockType::Air
                } else if y == height {
                    if height < 30 {
                        BlockType::Sand
                    } else if height < 35 {
                        BlockType::Grass
                    } else {
                        BlockType::Stone
                    }
                } else if y > height - 3 {
                    if height < 30 {
                        BlockType::Sand
                    } else {
                        BlockType::Dirt
                    }
                } else {
                    BlockType::Stone
                };

                chunk.set_block(x, y, z, block);
            }

            // Add trees
            if height > 32 && height < 40 {
                let tree_noise = noise.get([world_x as f64 * 0.1, world_z as f64 * 0.1]);
                if tree_noise > 0.7 {
                    generate_tree(chunk, x, height + 1, z);
                }
            }
        }
    }
}

fn get_height(noise: &Perlin, x: i32, z: i32) -> usize {
    let scale = 0.02;
    let height_scale = 15.0;
    let base_height = 32.0;

    let noise_value = noise.get([x as f64 * scale, z as f64 * scale]);
    let height = base_height + noise_value * height_scale;

    height.clamp(0.0, CHUNK_HEIGHT as f64 - 1.0) as usize
}

fn generate_tree(chunk: &mut Chunk, x: usize, y: usize, z: usize) {
    let trunk_height = 5;

    // Trunk
    for dy in 0..trunk_height {
        if y + dy < CHUNK_HEIGHT {
            chunk.set_block(x, y + dy, z, BlockType::Wood);
        }
    }

    // Leaves
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
                        chunk.set_block(leaf_x as usize, y + dy, leaf_z as usize, BlockType::Leaves);
                    }
                }
            }
        }
    }

    // Top of tree
    if y + trunk_height + 2 < CHUNK_HEIGHT {
        chunk.set_block(x, y + trunk_height + 2, z, BlockType::Leaves);
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