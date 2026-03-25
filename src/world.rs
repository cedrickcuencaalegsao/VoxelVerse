use crate::block::BlockType;
use crate::block_registry::BlockRegistry;
use crate::chunk::{CHUNK_HEIGHT, CHUNK_SIZE, Chunk};
// We switch this to match your actual main.rs root bound crate module registration
use crate::tree_breaking::{TreePart, TreeRoot};
use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use std::collections::{HashMap, HashSet};
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
            render_distance: 4,
            seed,
        }
    }
}

#[derive(Component)]
pub struct BlockVisual {
    pub world_pos: IVec3,
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<World>()
            .add_systems(Update, (generate_chunks, sync_block_visuals));
    }
}

const TERRAIN_SCALE: f64 = 0.01;
const DAMPENING: f64 = 0.6;
const OCTAVES: usize = 7;
const PERSISTENCE: f64 = 0.5;
const LACUNARITY: f64 = 2.0;

const TREE_THRESHOLD: f64 = 0.30;
const TREE_SPACING: i32 = 8;

#[derive(Clone, Copy)]
enum TreeSize {
    Small,
    Medium,
    Large,
}

struct TreeBlock {
    wx: i32,
    wy: i32,
    wz: i32,
    is_leaves: bool,
}

struct TreeRng(u32);

impl TreeRng {
    fn new(seed: u32) -> Self {
        Self(if seed == 0 { 0x1337 } else { seed })
    }
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn f32(&mut self) -> f32 {
        (self.next() & 0xFFFFFF) as f32 / 16777216.0
    }
    fn range(&mut self, min: i32, max: i32) -> i32 {
        if max <= min {
            min
        } else {
            min + (self.next() % (max - min) as u32) as i32
        }
    }
}

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
    let mountain_intensity = (continent_noise + 1.0) * 0.5 * mountain_blend;
    let height_variation =
        detail_noise * (15.0 * mountain_blend + mountain_intensity * 40.0) * DAMPENING;

    let natural_height = (30.0 + height_variation).clamp(1.0, CHUNK_HEIGHT as f64 - 1.0) as usize;
    let blended = CITY_HEIGHT as f32 * (1.0 - blend_t) + natural_height as f32 * blend_t;
    blended.round() as usize
}

fn add_leaf_clump(
    leaves: &mut HashSet<IVec3>,
    wood: &HashSet<IVec3>,
    rng: &mut TreeRng,
    cx: i32,
    cy: i32,
    cz: i32,
    radius: i32,
) {
    let r_sq = radius as f32 * radius as f32;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let dist_sq = (dx * dx + dy * dy + dz * dz) as f32;
                let jitter = rng.f32() * (radius as f32) * 0.8;
                let drop_mod = if dy < 0 && (dist_sq > r_sq * 0.6) && rng.f32() > 0.4 {
                    1.5
                } else {
                    0.0
                };

                if dist_sq + jitter + drop_mod <= r_sq {
                    let pos = IVec3::new(cx + dx, cy + dy, cz + dz);
                    if !wood.contains(&pos) {
                        leaves.insert(pos);
                    }
                }
            }
        }
    }
}

fn build_tree_blocks(wx: i32, base_y: i32, wz: i32, size: TreeSize) -> Vec<TreeBlock> {
    let seed_mix = (wx as u32)
        .wrapping_mul(73856093)
        .wrapping_add((wz as u32).wrapping_mul(19349663));
    let mut rng = TreeRng::new(seed_mix);

    let (height, branch_count, branch_radius) = match size {
        TreeSize::Small => (rng.range(6, 9), rng.range(2, 4), 2),
        TreeSize::Medium => (rng.range(11, 15), rng.range(4, 7), 3),
        TreeSize::Large => (rng.range(16, 20), rng.range(7, 10), 4),
    };

    let mut wood_set: HashSet<IVec3> = HashSet::new();
    let mut leaf_set: HashSet<IVec3> = HashSet::new();

    for dy in 0..height {
        let (max_r, threshold_sq) = match size {
            TreeSize::Large => {
                if dy < 3 {
                    (2, 4)
                } else if dy < height / 2 {
                    (1, 2)
                } else {
                    (1, 1)
                }
            }
            TreeSize::Medium => {
                if dy < 2 {
                    (1, 1)
                } else {
                    (0, 0)
                }
            }
            TreeSize::Small => (0, 0),
        };
        for dx in -max_r..=max_r {
            for dz in -max_r..=max_r {
                if dx * dx + dz * dz <= threshold_sq {
                    wood_set.insert(IVec3::new(wx + dx, base_y + dy, wz + dz));
                }
            }
        }
    }

    let trunk_top = base_y + height;
    let branch_start_y = base_y + (height / 3).max(1);

    for _ in 0..branch_count {
        let mut b_pos =
            Vec3::new(wx as f32, rng.range(branch_start_y, trunk_top) as f32, wz as f32);
        let angle = rng.f32() * std::f32::consts::TAU;
        let elevation = rng.f32() * 0.4 + 0.3;

        let dir = Vec3::new(
            angle.cos() * (1.0 - elevation * elevation).sqrt(),
            elevation,
            angle.sin() * (1.0 - elevation * elevation).sqrt(),
        )
        .normalize();

        let branch_length = rng.range(3, (height / 2).max(4));
        let steps = branch_length * 2;
        let step_vec = dir * 0.5;

        for step_i in 0..=steps {
            b_pos += step_vec;
            let bx = b_pos.x.round() as i32;
            let by = b_pos.y.round() as i32;
            let bz = b_pos.z.round() as i32;
            wood_set.insert(IVec3::new(bx, by, bz));

            if matches!(size, TreeSize::Large) {
                // FIXED ERROR -> Wrapping equations separately blocks incorrect f32 rust generic parsings checks bindings errors mapping!
                let percent = (step_i as f32) / (steps as f32);
                if percent < 0.6 {
                    wood_set.insert(IVec3::new(bx, by - 1, bz));
                    if (bx + by + bz) % 2 == 0 {
                        let padding = if (bx + by) % 3 == 0 { IVec3::X } else { IVec3::Z };
                        wood_set.insert(IVec3::new(bx, by, bz) + padding);
                    }
                }
            }
        }
        
        // FIXED ERROR -> Extract calculation bounds before &mut borrow argument bindings so mutability ownership avoids overlaps precisely cleanly handling checks native execution orders limits checks
        let calculated_clump_radius = rng.range(branch_radius - 1, branch_radius + 1);
        add_leaf_clump(
            &mut leaf_set,
            &wood_set,
            &mut rng,
            b_pos.x.round() as i32,
            b_pos.y.round() as i32,
            b_pos.z.round() as i32,
            calculated_clump_radius,
        );
    }

    // Identical error ownership scoping rule fixes passing values successfully before mappings mappings 
    let crown_branch_radius = rng.range(branch_radius, branch_radius + 2);
    add_leaf_clump(&mut leaf_set, &wood_set, &mut rng, wx, trunk_top, wz, crown_branch_radius);

    let mut blocks = Vec::with_capacity(wood_set.len() + leaf_set.len());
    for w in wood_set {
        blocks.push(TreeBlock { wx: w.x, wy: w.y, wz: w.z, is_leaves: false });
    }
    for l in leaf_set {
        blocks.push(TreeBlock { wx: l.x, wy: l.y, wz: l.z, is_leaves: true });
    }

    blocks
}

fn tree_size_at(wx: i32, wz: i32, noise: &Perlin) -> TreeSize {
    let size_val =
        fbm_noise(noise, wx as f64 * 0.07 + 200.0, wz as f64 * 0.07 + 200.0, 2, 0.5, 2.0);
    if size_val < -0.2 {
        TreeSize::Small
    } else if size_val < 0.25 {
        TreeSize::Medium
    } else {
        TreeSize::Large
    }
}

fn generate_terrain(chunk: &mut Chunk, noise: &Perlin) -> Vec<(i32, i32, i32, TreeSize)> {
    let mut tree_positions = Vec::new();
    for x in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            let world_x = chunk.position.x * CHUNK_SIZE as i32 + x as i32;
            let world_z = chunk.position.z * CHUNK_SIZE as i32 + z as i32;
            let height = get_height(noise, world_x, world_z);

            for y in 0..CHUNK_HEIGHT {
                let block = if y > height {
                    if y <= 28 {
                        BlockType::Water
                    } else {
                        BlockType::Air
                    }
                } else if y == height {
                    if height <= 29 {
                        BlockType::Sand
                    } else if height > 55 {
                        BlockType::Stone
                    } else {
                        BlockType::Grass
                    }
                } else if y > height.saturating_sub(3) {
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

            let dist_from_origin = ((world_x as f32).powi(2) + (world_z as f32).powi(2)).sqrt();
            if height > 32 && height < 52 && dist_from_origin > 40.0 {
                let grid_x = world_x.div_euclid(TREE_SPACING) * TREE_SPACING;
                let grid_z = world_z.div_euclid(TREE_SPACING) * TREE_SPACING;

                if world_x == grid_x && world_z == grid_z {
                    let tree_val = fbm_noise(
                        noise,
                        world_x as f64 * 0.12 + 100.0,
                        world_z as f64 * 0.12 + 100.0,
                        3,
                        0.5,
                        2.0,
                    );
                    if tree_val > TREE_THRESHOLD {
                        tree_positions.push((
                            world_x,
                            (height + 1) as i32,
                            world_z,
                            tree_size_at(world_x, world_z, noise),
                        ));
                    }
                }
            }
        }
    }
    tree_positions
}

fn generate_chunks(
    mut commands: Commands,
    mut world: ResMut<World>,
    asset_server: Res<AssetServer>,
    registry: Res<BlockRegistry>,
    camera_query: Query<&Transform, With<Camera>>,
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

    let mut pending: Vec<IVec3> = (-render_distance..=render_distance)
        .flat_map(|cx| {
            (-render_distance..=render_distance)
                .map(move |cz| IVec3::new(camera_chunk.x + cx, 0, camera_chunk.z + cz))
        })
        .filter(|pos| !world.chunks.contains_key(pos))
        .collect();

    pending.sort_by_key(|pos| {
        let dx = pos.x - camera_chunk.x;
        let dz = pos.z - camera_chunk.z;
        dx * dx + dz * dz
    });

    for chunk_pos in pending.iter().take(2) {
        let mut chunk = Chunk::new(*chunk_pos);
        let tree_positions = generate_terrain(&mut chunk, &world.noise);
        let surface_blocks = chunk.get_surface_blocks();

        let mut rendered_trees = Vec::new();
        for (wx, wy, wz, size) in tree_positions {
            let tree_blocks = build_tree_blocks(wx, wy, wz, size);

            for tb in &tree_blocks {
                let lx = tb.wx - (chunk_pos.x * CHUNK_SIZE as i32);
                let lz = tb.wz - (chunk_pos.z * CHUNK_SIZE as i32);
                if lx >= 0
                    && lx < CHUNK_SIZE as i32
                    && lz >= 0
                    && lz < CHUNK_SIZE as i32
                    && tb.wy >= 0
                    && tb.wy < CHUNK_HEIGHT as i32
                {
                    chunk.set_block(
                        lx as usize,
                        tb.wy as usize,
                        lz as usize,
                        if tb.is_leaves {
                            BlockType::Leaves
                        } else {
                            BlockType::Wood
                        },
                    );
                }
            }
            rendered_trees.push(tree_blocks);
        }

        let chunk_entity = commands
            .spawn((SpatialBundle::default(), chunk))
            .with_children(|parent| {
                for (lx, ly, lz, block_type) in surface_blocks {
                    if matches!(block_type, BlockType::Wood | BlockType::Leaves) {
                        continue;
                    }
                    let wx = chunk_pos.x * CHUNK_SIZE as i32 + lx as i32;
                    let wz = chunk_pos.z * CHUNK_SIZE as i32 + lz as i32;
                    let (scene_path, y_offset) = block_visual(block_type);

                    parent.spawn((
                        SceneBundle {
                            scene: asset_server.load(scene_path),
                            transform: Transform::from_translation(Vec3::new(
                                wx as f32 + 0.5,
                                ly as f32 + 0.5 + y_offset,
                                wz as f32 + 0.5,
                            )),
                            ..default()
                        },
                        BlockVisual {
                            world_pos: IVec3::new(wx, ly as i32, wz),
                        },
                    ));
                }

                // Generates whole coherent tracked grouping roots assigning physical coordinates for erasing cleanly post target strikes!
                for tree_blocks in rendered_trees {
                    let mut wood_count = 0;
                    let mut leaves_count = 0;
                    let mut pos_storage = Vec::with_capacity(tree_blocks.len());

                    for tb in &tree_blocks {
                        pos_storage.push(IVec3::new(tb.wx, tb.wy, tb.wz));
                        if tb.is_leaves {
                            leaves_count += 1;
                        } else {
                            wood_count += 1;
                        }
                    }

                    // Root element encapsulating blocks logically so Bevy can destroy hierarchies cohesively without orphans!
                    parent
                        .spawn((
                            SpatialBundle::default(),
                            TreeRoot {
                                wood_count,
                                leaves_count,
                                blocks: pos_storage,
                            },
                        ))
                        .with_children(|tree_builder| {
                            for tb in tree_blocks {
                                let scene_path = if tb.is_leaves {
                                    "leaves.glb#Scene0"
                                } else {
                                    "wood.glb#Scene0"
                                };
                                let center = Vec3::new(
                                    tb.wx as f32 + 0.5,
                                    tb.wy as f32 + 0.5,
                                    tb.wz as f32 + 0.5,
                                );

                                tree_builder.spawn((
                                    SceneBundle {
                                        scene: asset_server.load(scene_path),
                                        transform: Transform::from_translation(center),
                                        ..default()
                                    },
                                    TreePart, // The actual individual component that strikes bounds register against inside breaking.
                                ));
                            }
                        });
                }
            })
            .id();

        world.chunks.insert(*chunk_pos, chunk_entity);
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
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn sync_block_visuals(
    mut commands: Commands,
    world: Res<World>,
    chunks: Query<&Chunk>,
    visuals: Query<(Entity, &BlockVisual)>,
) {
    for (entity, visual) in visuals.iter() {
        let chunk_pos = IVec3::new(
            visual.world_pos.x.div_euclid(CHUNK_SIZE as i32),
            0,
            visual.world_pos.z.div_euclid(CHUNK_SIZE as i32),
        );

        let Some(&chunk_entity) = world.chunks.get(&chunk_pos) else {
            continue;
        };
        let Ok(chunk) = chunks.get(chunk_entity) else {
            continue;
        };

        let lx = visual.world_pos.x.rem_euclid(CHUNK_SIZE as i32) as usize;
        let lz = visual.world_pos.z.rem_euclid(CHUNK_SIZE as i32) as usize;

        if chunk.get_block(lx, visual.world_pos.y as usize, lz) == BlockType::Air {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn block_visual(block: BlockType) -> (&'static str, f32) {
    match block {
        BlockType::Grass => ("grass.glb#Scene0", 0.0),
        BlockType::Dirt | BlockType::Stone => ("soil.glb#Scene0", 0.0),
        BlockType::Sand => ("sand.glb#Scene0", 0.0),
        BlockType::Water => ("water.glb#Scene0", 0.0),
        _ => ("grass.glb#Scene0", 0.0), // Procedurals wood/leaves bypassed cleanly directly internally under chunks arrays logic.
    }
}

pub fn get_spawn_height(noise: &Perlin) -> f32 {
    get_height(noise, 0, 0) as f32 + 2.0
}