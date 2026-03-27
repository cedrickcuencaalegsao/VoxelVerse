use crate::block::BlockType;
use crate::camera::{MainCamera, Player};
use crate::chunk::{CHUNK_HEIGHT, CHUNK_SIZE};
use crate::world::World as GameWorld;
// WE IMPORT THE TREE BREAKING STATE HERE:
use crate::tree_breaking::TreeBreakingState; 
use bevy::prelude::*;

const BREAK_TIME: f32 = 1.0;
const REACH: f32 = 5.0;

#[derive(Resource, Default)]
pub struct BreakingState {
    pub target: Option<IVec3>,
    pub progress: f32,
    pub crack_entity: Option<Entity>,
}

#[derive(Component)]
pub struct BreakParticle {
    pub velocity: Vec3,
    pub lifetime: f32,
    pub age: f32,
}

#[derive(Component)]
pub struct BlockDrop {
    pub _block_type: BlockType,
    pub origin_y: f32,
    pub age: f32,
}

#[derive(Component)]
pub struct CrackOverlay {
    pub block_pos: IVec3,
}

pub struct BlockBreakingPlugin;

impl Plugin for BlockBreakingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BreakingState>().add_systems(
            Update,
            (
                raycast_target,
                handle_breaking,
                update_crack_overlay,
                update_break_particles,
                update_block_drops,
            )
                .chain(),
        );
    }
}

fn raycast_block(
    world: &GameWorld,
    chunks: &Query<&crate::chunk::Chunk>,
    origin: Vec3,
    direction: Vec3,
) -> Option<IVec3> {
    let steps = (REACH * 10.0) as usize;
    let step_size = REACH / steps as f32;

    for i in 1..=steps {
        let pos = origin + direction * (i as f32 * step_size);
        let bx = pos.x.floor() as i32;
        let by = pos.y.floor() as i32;
        let bz = pos.z.floor() as i32;

        if by < 0 || by >= CHUNK_HEIGHT as i32 {
            continue;
        }

        let chunk_x = bx.div_euclid(CHUNK_SIZE as i32);
        let chunk_z = bz.div_euclid(CHUNK_SIZE as i32);
        let chunk_pos = IVec3::new(chunk_x, 0, chunk_z);

        let Some(&entity) = world.chunks.get(&chunk_pos) else {
            continue;
        };
        let Ok(chunk) = chunks.get(entity) else {
            continue;
        };

        let lx = bx.rem_euclid(CHUNK_SIZE as i32) as usize;
        let lz = bz.rem_euclid(CHUNK_SIZE as i32) as usize;
        let block = chunk.get_block(lx, by as usize, lz as usize);

        if block.is_solid() && !matches!(block, BlockType::Water) {
            return Some(IVec3::new(bx, by, bz));
        }
    }
    None
}

fn raycast_target(
    world: Res<GameWorld>,
    chunks: Query<&crate::chunk::Chunk>,
    mut state: ResMut<BreakingState>,
    camera_query: Query<&Transform, With<MainCamera>>,
    mut commands: Commands,
) {
    let Ok(cam_transform) = camera_query.get_single() else {
        return;
    };

    let origin = cam_transform.translation;
    let direction = cam_transform.forward().into();
    let hit = raycast_block(&world, &chunks, origin, direction);

    if hit != state.target {
        state.progress = 0.0;
        if let Some(e) = state.crack_entity.take() {
            commands.entity(e).despawn_recursive();
        }
    }

    state.target = hit;
}

fn handle_breaking(
    mut commands: Commands,
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut state: ResMut<BreakingState>,
    world: Res<GameWorld>,
    mut chunks: Query<&mut crate::chunk::Chunk>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    tree_state: Res<TreeBreakingState>, // <- WE ADD TREE BREAKING STATE HERE
) {
    let Some(target) = state.target else {
        state.progress = 0.0;
        return;
    };

    // --- CHECK BLOCK TYPE FIRST ---
    // We figure out the target's exact position on the map
    let chunk_x = target.x.div_euclid(CHUNK_SIZE as i32);
    let chunk_z = target.z.div_euclid(CHUNK_SIZE as i32);
    let chunk_pos = IVec3::new(chunk_x, 0, chunk_z);

    // Identify what we are aiming at right now!
    let mut block_type = BlockType::Air;
    if let Some(&entity) = world.chunks.get(&chunk_pos) {
        if let Ok(chunk) = chunks.get(entity) {
            let lx = target.x.rem_euclid(CHUNK_SIZE as i32) as usize;
            let lz = target.z.rem_euclid(CHUNK_SIZE as i32) as usize;
            block_type = chunk.get_block(lx, target.y as usize, lz as usize);
        }
    }

    // THE MAGIC RULE:
    // If we're looking at Wood or Leaves AND the tree_breaking.rs script acknowledges that 
    // it belongs to an INTACT tree, STOP standard block-breaking dead in its tracks!
    if (block_type == BlockType::Wood || block_type == BlockType::Leaves) && tree_state.target_part.is_some() {
        state.progress = 0.0; // Clear any standard cracks trying to start 
        if let Some(e) = state.crack_entity.take() {
            commands.entity(e).despawn_recursive();
        }
        return; 
    }
    // ---------------------------------

    // Normal non-tree clicking happens correctly down below:
    if !mouse.pressed(MouseButton::Left) {
        state.progress = 0.0;
        if let Some(e) = state.crack_entity.take() {
            commands.entity(e).despawn_recursive();
        }
        return;
    }

    state.progress += time.delta_seconds() / BREAK_TIME;

    if state.crack_entity.is_none() {
        let crack = spawn_crack_overlay(&mut commands, &mut meshes, &mut materials, target);
        state.crack_entity = Some(crack);
    }

    // FINALLY BREAKING NORMAL BLOCKS! (Or player-placed independent logs):
    if state.progress >= 1.0 {
        state.progress = 0.0;

        if let Some(e) = state.crack_entity.take() {
            commands.entity(e).despawn_recursive();
        }

        // Set block to Air — sync_block_visuals will despawn the GLB
        if let Some(&entity) = world.chunks.get(&chunk_pos) {
            if let Ok(mut chunk) = chunks.get_mut(entity) {
                let lx = target.x.rem_euclid(CHUNK_SIZE as i32) as usize;
                let lz = target.z.rem_euclid(CHUNK_SIZE as i32) as usize;
                chunk.set_block(lx, target.y as usize, lz as usize, BlockType::Air);
            }
        }

        let center = Vec3::new(
            target.x as f32 + 0.5,
            target.y as f32 + 0.5,
            target.z as f32 + 0.5,
        );

        spawn_break_particles(
            &mut commands,
            &mut meshes,
            &mut materials,
            center,
            block_type,
        );
        spawn_block_drop(&mut commands, &asset_server, center, block_type);
    }
}

fn spawn_crack_overlay(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    block_pos: IVec3,
) -> Entity {
    let center = Vec3::new(
        block_pos.x as f32 + 0.5,
        block_pos.y as f32 + 0.5,
        block_pos.z as f32 + 0.5,
    );

    let crack_mesh = meshes.add(Cuboid::new(1.02, 1.02, 1.02));
    let crack_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 0.0, 0.0, 0.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    commands
        .spawn((
            PbrBundle {
                mesh: crack_mesh,
                material: crack_mat,
                transform: Transform::from_translation(center),
                ..default()
            },
            CrackOverlay { block_pos },
        ))
        .id()
}

fn update_crack_overlay(
    state: Res<BreakingState>,
    mut query: Query<(&mut Transform, &mut Handle<StandardMaterial>, &CrackOverlay)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    for (mut transform, mat_handle, overlay) in query.iter_mut() {
        let p = state.progress.clamp(0.0, 1.0);
        let shake = (p * p) * 0.04;
        let t = time.elapsed_seconds();

        transform.translation = Vec3::new(
            overlay.block_pos.x as f32 + 0.5 + (t * 40.0).sin() * shake,
            overlay.block_pos.y as f32 + 0.5 + (t * 37.0).cos() * shake,
            overlay.block_pos.z as f32 + 0.5 + (t * 43.0).sin() * shake,
        );

        if let Some(mat) = materials.get_mut(mat_handle.id()) {
            let alpha = p * 0.65;
            mat.base_color = Color::srgba(0.0, 0.0, 0.0, alpha);
        }
    }
}

fn spawn_break_particles(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    block_type: BlockType,
) {
    let particle_mesh = meshes.add(Cuboid::new(0.18, 0.18, 0.18));

    let color = match block_type {
        BlockType::Grass => Color::srgb(0.3, 0.7, 0.2),
        BlockType::Dirt => Color::srgb(0.5, 0.3, 0.15),
        BlockType::Stone => Color::srgb(0.5, 0.5, 0.5),
        BlockType::Sand => Color::srgb(0.86, 0.78, 0.52),
        BlockType::Wood => Color::srgb(0.45, 0.28, 0.1),
        BlockType::Leaves => Color::srgb(0.2, 0.55, 0.1),
        _ => Color::srgb(0.6, 0.6, 0.6),
    };

    let mat = materials.add(StandardMaterial {
        base_color: color,
        unlit: false,
        ..default()
    });

    let offsets =[
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(-1.0, 1.0, 1.0),
        Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(1.0, 1.0, -1.0),
        Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(-1.0, -1.0, -1.0),
    ];

    for offset in offsets {
        let vel = offset.normalize() * 3.5 + Vec3::new(0.0, 2.0, 0.0);
        commands.spawn((
            PbrBundle {
                mesh: particle_mesh.clone(),
                material: mat.clone(),
                transform: Transform::from_translation(center).with_scale(Vec3::splat(1.0)),
                ..default()
            },
            BreakParticle {
                velocity: vel,
                lifetime: 0.6,
                age: 0.0,
            },
        ));
    }
}

fn update_break_particles(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut BreakParticle)>,
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
        particle.velocity.y -= 12.0 * dt;
        let scale = (1.0 - t).max(0.01);
        transform.scale = Vec3::splat(scale * 0.18);
        transform.rotate_x(dt * 5.0);
        transform.rotate_z(dt * 3.0);
    }
}

fn spawn_block_drop(
    commands: &mut Commands,
    asset_server: &AssetServer,
    center: Vec3,
    block_type: BlockType,
) {
    let scene_path = match block_type {
        BlockType::Grass => "grass.glb#Scene0",
        BlockType::Dirt => "soil.glb#Scene0",
        BlockType::Stone => "stone.glb#Scene0",
        BlockType::Sand => "sand.glb#Scene0",
        BlockType::Wood => "wood.glb#Scene0",
        BlockType::Leaves => "leaves.glb#Scene0",
        BlockType::Water => "water.glb#Scene0",
        BlockType::Air => "block.glb#Scene0",
    };

    commands.spawn((
        SceneBundle {
            scene: asset_server.load(scene_path),
            transform: Transform::from_translation(Vec3::new(center.x, center.y + 0.3, center.z))
                .with_scale(Vec3::splat(0.4)),
            ..default()
        },
        BlockDrop {
            _block_type: block_type,
            origin_y: center.y + 0.3,
            age: 0.0,
        },
    ));
}

fn update_block_drops(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut BlockDrop)>,
    player_query: Query<&Transform, (With<Player>, Without<BlockDrop>)>,
    time: Res<Time>,
) {
    let player_pos = player_query
        .get_single()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    let dt = time.delta_seconds();

    for (entity, mut transform, mut drop) in query.iter_mut() {
        drop.age += dt;

        transform.translation.y = drop.origin_y + (drop.age * 2.5).sin() * 0.15;
        transform.rotate_y(dt * 1.2);

        let dist = (transform.translation - player_pos).length();
        if dist < 1.5 {
            commands.entity(entity).despawn_recursive();
        }

        if drop.age > 30.0 {
            commands.entity(entity).despawn_recursive();
        }
    }
}