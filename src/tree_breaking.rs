use bevy::prelude::*;
use crate::camera::MainCamera;
use crate::world::{World, get_height};   // get_height is now public
use crate::chunk::{Chunk, CHUNK_SIZE, CHUNK_HEIGHT};
use crate::block::BlockType;
use crate::inventory::{Inventory, ItemKind, Pickup};
use crate::physics::PLAYER_HEIGHT;

const TREE_REACH: f32 = 6.0;
const SECONDS_PER_BLOCK_BREAK: f32 = 0.06;

#[derive(Component)]
pub struct TreePart;

#[derive(Component)]
pub struct TreeRoot {
    pub wood_count: u32,
    pub leaves_count: u32,
    pub blocks: Vec<IVec3>,
}

#[derive(Component)]
pub struct TreeCrackOverlay {
    pub _tree_part_entity: Entity,
    pub base_translation: Vec3,
}

#[derive(Component)]
pub struct WoodParticle {
    pub velocity: Vec3,
    pub age: f32,
    pub lifetime: f32,
}

#[derive(Component)]
pub struct TreeDrop {
    pub origin_y: f32,
    pub age: f32,
}

#[derive(Resource, Default)]
pub struct TreeBreakingState {
    pub target_part: Option<Entity>,
    pub root_entity: Option<Entity>,
    pub progress_time: f32,
    pub total_break_duration: f32,
    pub crack_entities: Vec<Entity>,
    pub hit_point_origin: Vec3,
}

pub struct TreeBreakingPlugin;

impl Plugin for TreeBreakingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TreeBreakingState>()
            .add_systems(Update, (
                raycast_tree_target,
                handle_tree_breaking,
                update_tree_crack,
                update_wood_particles,
                update_tree_drops,
            ).chain());
    }
}

fn raycast_tree_target(
    mut state: ResMut<TreeBreakingState>,
    camera_query: Query<&Transform, With<MainCamera>>,
    tree_part_query: Query<(Entity, &GlobalTransform), With<TreePart>>,
    tree_parent_query: Query<&Parent, With<TreePart>>,
    root_query: Query<&TreeRoot>,
    mut commands: Commands,
) {
    let Ok(cam) = camera_query.get_single() else { return };

    let origin = cam.translation;
    let forward: Vec3 = cam.forward().into();

    let mut best_entity: Option<Entity> = None;
    let mut best_dist = f32::MAX;

    for (entity, global) in tree_part_query.iter() {
        let tree_pos = global.translation();
        let to_tree = tree_pos - origin;
        let dist = to_tree.length();

        if dist > TREE_REACH { continue; }
        if to_tree.normalize_or_zero().dot(forward) < 0.98 { continue; }

        if dist < best_dist {
            best_dist = dist;
            best_entity = Some(entity);
        }
    }

    let mut best_root = None;
    let mut calc_duration = 0.0;

    if let Some(target) = best_entity {
        if let Ok(parent) = tree_parent_query.get(target) {
            best_root = Some(parent.get());
            if let Ok(root) = root_query.get(parent.get()) {
                let total_blocks = root.wood_count + root.leaves_count;
                calc_duration = (total_blocks as f32 * SECONDS_PER_BLOCK_BREAK).clamp(0.5, 12.0);
            }
        }
    }

    if state.root_entity != best_root {
        state.progress_time = 0.0;
        state.total_break_duration = calc_duration;
        for e in state.crack_entities.drain(..) { commands.entity(e).despawn_recursive(); }
    }

    state.target_part = best_entity;
    state.root_entity = best_root;

    if let Some(entity) = best_entity {
        if let Ok((_, global)) = tree_part_query.get(entity) {
            state.hit_point_origin = global.translation();
        }
    }
}

fn handle_tree_breaking(
    mut commands: Commands,
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut state: ResMut<TreeBreakingState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    tree_transform_query: Query<&GlobalTransform, With<TreePart>>,
    children_query: Query<&Children>,
    root_query: Query<&TreeRoot>,
    mut chunks: Query<&mut Chunk>,
    world: Res<World>,
) {
    let Some(target_root) = state.root_entity else { return; };

    if !mouse.pressed(MouseButton::Left) {
        state.progress_time = 0.0;
        for e in state.crack_entities.drain(..) { commands.entity(e).despawn_recursive(); }
        return;
    }

    state.progress_time += time.delta_seconds();

    if state.crack_entities.is_empty() {
        let crack_mat = materials.add(StandardMaterial {
            base_color: Color::srgba(0.0, 0.0, 0.0, 0.0),
            alpha_mode: AlphaMode::Blend, unlit: true, double_sided: true, cull_mode: None, ..default()
        });
        let mesh = meshes.add(Cuboid::new(1.15, 1.15, 1.15));

        if let Ok(children) = children_query.get(target_root) {
            for &child in children.iter() {
                if let Ok(global) = tree_transform_query.get(child) {
                    let pos = global.translation();
                    let crack = commands.spawn((
                        PbrBundle {
                            mesh: mesh.clone(),
                            material: crack_mat.clone(),
                            transform: Transform::from_translation(pos),
                            ..default()
                        },
                        TreeCrackOverlay { _tree_part_entity: child, base_translation: pos },
                    )).id();
                    state.crack_entities.push(crack);
                }
            }
        }
    }

    if state.progress_time >= state.total_break_duration && state.total_break_duration > 0.0 {
        if let Ok(tree_root) = root_query.get(target_root) {
            // Clear blocks from world
            for pos in &tree_root.blocks {
                let chunk_pos = IVec3::new(pos.x.div_euclid(CHUNK_SIZE as i32), 0, pos.z.div_euclid(CHUNK_SIZE as i32));
                if let Some(&chunk_ent) = world.chunks.get(&chunk_pos) {
                    if let Ok(mut chunk) = chunks.get_mut(chunk_ent) {
                        let lx = pos.x.rem_euclid(CHUNK_SIZE as i32) as usize;
                        let lz = pos.z.rem_euclid(CHUNK_SIZE as i32) as usize;
                        if pos.y >= 0 && pos.y < CHUNK_HEIGHT as i32 {
                            chunk.set_block(lx, pos.y as usize, lz, BlockType::Air);
                        }
                    }
                }
            }

            // FIXED: spawn drops ON THE GROUND
            let ground_y = get_height(&world.noise, state.hit_point_origin.x as i32, state.hit_point_origin.z as i32) as f32 + 0.5;

            spawn_tree_drops(&mut commands, &asset_server, state.hit_point_origin, tree_root.wood_count, tree_root.leaves_count, ground_y);
            spawn_wood_particles(&mut commands, &mut meshes, &mut materials, state.hit_point_origin);

            commands.entity(target_root).despawn_recursive();
        }

        for e in state.crack_entities.drain(..) { commands.entity(e).despawn_recursive(); }

        state.target_part = None;
        state.root_entity = None;
        state.progress_time = 0.0;
    }
}

fn update_tree_crack(
    state: Res<TreeBreakingState>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Handle<StandardMaterial>, &TreeCrackOverlay)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let p = if state.total_break_duration > 0.0 {
        (state.progress_time / state.total_break_duration).clamp(0.0, 1.0)
    } else { 0.0 };

    let t = time.elapsed_seconds();
    let shake_amount = p * p * 0.12;

    for (mut transform, mat_handle, overlay) in query.iter_mut() {
        let base = overlay.base_translation;
        let phase_shift = base.x * 2.1 + base.y * 3.3 + base.z * 1.5;

        transform.translation = Vec3::new(
            base.x + ((t * 35.0) + phase_shift).sin() * shake_amount,
            base.y + ((t * 28.0) + phase_shift).cos() * shake_amount * 0.5,
            base.z + ((t * 41.0) + phase_shift).sin() * shake_amount,
        );

        if let Some(mat) = materials.get_mut(mat_handle.id()) {
            let alpha = p * 0.75;
            let burn_ratio = p * 0.35;
            mat.base_color = Color::srgba(burn_ratio, burn_ratio * 0.45, 0.0, alpha);
        }

        let tilt = p * p * 0.20;
        transform.rotation = Quat::from_rotation_z(((t * 5.0) + phase_shift).sin() * tilt);
    }
}

fn spawn_wood_particles(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
) {
    let particle_mesh = meshes.add(Cuboid::new(0.2, 0.2, 0.2));
    let bark_mat = materials.add(StandardMaterial { base_color: Color::srgb(0.35, 0.20, 0.08), ..default() });
    let wood_mat = materials.add(StandardMaterial { base_color: Color::srgb(0.65, 0.42, 0.18), ..default() });

    let offsets: &[(f32, f32, f32, f32, f32, f32)] = &[
        (1.0, 0.5, 0.0, 3.5, 4.0, 0.5), (-1.0, 0.5, 0.0, -3.5, 4.0, 0.5),
        (0.0, 0.5, 1.0, 0.5, 4.0, 3.5), (0.0, 0.5, -1.0, 0.5, 4.0, -3.5),
        (0.7, 1.0, 0.7, 2.5, 5.0, 2.5), (-0.7, 1.0, -0.7, -2.5, 5.0, -2.5),
        (0.7, 1.5, -0.7, 2.5, 6.0, -2.5), (-0.7, 1.5, 0.7, -2.5, 6.0, 2.5),
        (0.1, 0.5, 0.1, 0.3, 7.0, 0.3), (-0.1, 0.5, -0.1, -0.3, 7.0, -0.3),
        (0.0, 2.0, 0.0, 0.0, 8.0, 0.0), (0.3, 1.0, 0.0, 1.0, 5.5, 0.5),
    ];

    for (i, &(ox, oy, oz, vx, vy, vz)) in offsets.iter().enumerate() {
        let mat = if i % 3 == 0 { wood_mat.clone() } else { bark_mat.clone() };
        commands.spawn((
            PbrBundle {
                mesh: particle_mesh.clone(),
                material: mat,
                transform: Transform::from_translation(center + Vec3::new(ox, oy, oz)).with_scale(Vec3::splat(0.2)),
                ..default()
            },
            WoodParticle { velocity: Vec3::new(vx, vy, vz), age: 0.0, lifetime: 1.2 },
        ));
    }
}

fn update_wood_particles(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut WoodParticle)>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();
    for (entity, mut transform, mut particle) in query.iter_mut() {
        particle.age += dt;
        if particle.age >= particle.lifetime {
            commands.entity(entity).despawn();
            continue;
        }

        let t = particle.age / particle.lifetime;
        transform.translation += particle.velocity * dt;
        particle.velocity.y -= 14.0 * dt;

        transform.scale = Vec3::splat((1.0 - t * 0.8).max(0.02) * 0.2);
        transform.rotate_x(dt * 6.0);
        transform.rotate_z(dt * 4.0);
        transform.rotate_y(dt * 3.0);
    }
}

fn spawn_tree_drops(
    commands: &mut Commands,
    asset_server: &AssetServer,
    center: Vec3,
    wood_qty: u32,
    leaf_qty: u32,
    ground_y: f32,
) {
    let mut item_spawn_mapper = |qty: u32,
                                 drop_scale: f32,
                                 file: &'static str,
                                 distance: f32,
                                 kind: ItemKind| {
        for i in 0..qty {
            let radial_dist = (i as f32 * 0.3 % 4.0) + distance;
            let elevation = (i as f32 % 5.0) * 0.4;
            let a = i as f32 * 2.4;

            let spawn_pos = Vec3::new(
                center.x + a.cos() * radial_dist,
                ground_y + elevation,
                center.z + a.sin() * radial_dist,
            );

            commands.spawn((
                SceneBundle {
                    scene: asset_server.load(file),
                    transform: Transform::from_translation(spawn_pos).with_scale(Vec3::splat(drop_scale)),
                    ..default()
                },
                TreeDrop { origin_y: spawn_pos.y, age: 0.0 },
                Pickup { kind },
            ));
        }
    };

    item_spawn_mapper(wood_qty, 0.35, "wood.glb#Scene0", 0.5, ItemKind::Wood);
    item_spawn_mapper(leaf_qty, 0.45, "leaves.glb#Scene0", 1.2, ItemKind::Leaves);
}

fn update_tree_drops(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut TreeDrop, Option<&Pickup>)>,
    camera_query: Query<&Transform, (With<crate::camera::Player>, Without<TreeDrop>)>,
    mut inventory: ResMut<Inventory>,
    time: Res<Time>,
) {
    let player_feet = camera_query
        .get_single()
        .map(|t| t.translation - Vec3::Y * PLAYER_HEIGHT)
        .unwrap_or(Vec3::ZERO);
    let dt = time.delta_seconds();

    for (entity, mut transform, mut drop, pickup) in query.iter_mut() {
        drop.age += dt;
        transform.translation.y = drop.origin_y + (drop.age * 2.0).sin() * 0.12;
        transform.rotate_y(dt * 1.5);

        if (transform.translation - player_feet).length() < 1.8 {
            if let Some(p) = pickup {
                inventory.add(p.kind, 1);
            }
            commands.entity(entity).despawn_recursive();
            continue;
        }
        if drop.age > 45.0 {
            commands.entity(entity).despawn_recursive();
        }
    }
}