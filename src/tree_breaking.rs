use bevy::prelude::*;
use crate::camera::MainCamera;

const TREE_REACH: f32 = 6.0;
const TREE_BREAK_TIME: f32 = 2.5; // trees take longer than blocks

#[derive(Component)]
pub struct Tree {
    pub health: f32,    // 0.0 to 1.0
}

#[derive(Component)]
pub struct TreeCrackOverlay {
    pub tree_entity: Entity,
}

#[derive(Component)]
pub struct WoodParticle {
    pub velocity: Vec3,
    pub age: f32,
    pub lifetime: f32,
}

#[derive(Component)]
pub struct WoodDrop {
    pub origin_y: f32,
    pub age: f32,
}

#[derive(Resource, Default)]
pub struct TreeBreakingState {
    pub target: Option<Entity>,
    pub progress: f32,
    pub crack_entity: Option<Entity>,
    pub shake_origin: Vec3,
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
                update_wood_drops,
            ).chain());
    }
}

/// Raycast against tree entities — checks distance to each Tree
/// and picks the closest one within reach that the camera faces.
fn raycast_tree_target(
    mut state: ResMut<TreeBreakingState>,
    camera_query: Query<&Transform, With<MainCamera>>,
    tree_query: Query<(Entity, &Transform, &GlobalTransform), With<Tree>>,
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    let Ok(cam) = camera_query.get_single() else { return };

    let origin    = cam.translation;
    let forward: Vec3 = cam.forward().into();

    let mut best_entity: Option<Entity> = None;
    let mut best_dist = f32::MAX;

    for (entity, _transform, global) in tree_query.iter() {
        let tree_pos = global.translation();

        // Vector from camera to tree center
        let to_tree = tree_pos - origin;
        let dist    = to_tree.length();

        if dist > TREE_REACH { continue; }

        // Check camera is roughly facing the tree
        let dot = to_tree.normalize_or_zero().dot(forward);
        if dot < 0.4 { continue; } // must be within ~66 degrees of view

        if dist < best_dist {
            best_dist   = dist;
            best_entity = Some(entity);
        }
    }

    if best_entity != state.target {
        state.progress = 0.0;
        if let Some(e) = state.crack_entity.take() {
            commands.entity(e).despawn_recursive();
        }
    }

    state.target = best_entity;

    // Store shake origin for the crack overlay
    if let Some(entity) = best_entity {
        if let Ok((_, _, global)) = tree_query.get(entity) {
            state.shake_origin = global.translation();
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
    tree_query: Query<&GlobalTransform, With<Tree>>,
) {
    let Some(target) = state.target else {
        state.progress = 0.0;
        return;
    };

    if !mouse.pressed(MouseButton::Left) {
        state.progress = 0.0;
        if let Some(e) = state.crack_entity.take() {
            commands.entity(e).despawn_recursive();
        }
        return;
    }

    state.progress += time.delta_seconds() / TREE_BREAK_TIME;

    // Spawn crack overlay on first frame
    if state.crack_entity.is_none() {
        let Ok(global) = tree_query.get(target) else { return };
        let pos = global.translation();

        let crack = commands.spawn((
            PbrBundle {
                mesh: meshes.add(Cuboid::new(1.2, 4.0, 1.2)),
                material: materials.add(StandardMaterial {
                    base_color: Color::srgba(0.0, 0.0, 0.0, 0.0),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    double_sided: true,
                    cull_mode: None,
                    ..default()
                }),
                transform: Transform::from_translation(
                    Vec3::new(pos.x, pos.y + 2.0, pos.z)
                ),
                ..default()
            },
            TreeCrackOverlay { tree_entity: target },
        )).id();

        state.crack_entity = Some(crack);
    }

    if state.progress >= 1.0 {
        state.progress = 0.0;

        // Remove crack overlay
        if let Some(e) = state.crack_entity.take() {
            commands.entity(e).despawn_recursive();
        }

        // Get tree position before despawning
        let tree_pos = if let Ok(global) = tree_query.get(target) {
            global.translation()
        } else {
            Vec3::ZERO
        };

        // Despawn the tree GLB
        commands.entity(target).despawn_recursive();
        state.target = None;

        // Spawn wood break particles
        spawn_wood_particles(
            &mut commands,
            &mut meshes,
            &mut materials,
            tree_pos,
        );

        // Spawn multiple wood log drops
        spawn_wood_drops(&mut commands, &asset_server, tree_pos);
    }
}

fn update_tree_crack(
    state: Res<TreeBreakingState>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Handle<StandardMaterial>, &TreeCrackOverlay)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (mut transform, mat_handle, overlay) in query.iter_mut() {
        let p = state.progress.clamp(0.0, 1.0);

        // Shake increases as tree gets closer to breaking
        let shake_amount = p * p * 0.08;
        let t = time.elapsed_seconds();

        let base = state.shake_origin;
        transform.translation = Vec3::new(
            base.x + (t * 35.0).sin() * shake_amount,
            base.y + 2.0 + (t * 28.0).cos() * shake_amount * 0.5,
            base.z + (t * 41.0).sin() * shake_amount,
        );

        // Darken overlay — simulates cracks spreading across bark
        if let Some(mat) = materials.get_mut(mat_handle.id()) {
            // Start transparent, go dark brown like cracking bark
            let alpha = p * 0.7;
            let brown = p * 0.3; // slight brown tint
            mat.base_color = Color::srgba(brown, brown * 0.5, 0.0, alpha);
        }

        // Tilt slightly as it's about to fall
        let tilt = p * p * 0.15;
        transform.rotation = Quat::from_rotation_z((t * 3.0).sin() * tilt);
    }
}

fn spawn_wood_particles(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
) {
    let particle_mesh = meshes.add(Cuboid::new(0.2, 0.2, 0.2));

    // Wood bark color
    let bark_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.20, 0.08),
        ..default()
    });

    // Wood inside color (lighter)
    let wood_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.65, 0.42, 0.18),
        ..default()
    });

    // Spray particles outward and upward from the tree base
    let offsets: &[(f32, f32, f32, f32, f32, f32)] = &[
        // (ox, oy, oz, vx, vy, vz)
        ( 1.0,  0.5,  0.0,  3.5, 4.0,  0.5),
        (-1.0,  0.5,  0.0, -3.5, 4.0,  0.5),
        ( 0.0,  0.5,  1.0,  0.5, 4.0,  3.5),
        ( 0.0,  0.5, -1.0,  0.5, 4.0, -3.5),
        ( 0.7,  1.0,  0.7,  2.5, 5.0,  2.5),
        (-0.7,  1.0, -0.7, -2.5, 5.0, -2.5),
        ( 0.7,  1.5, -0.7,  2.5, 6.0, -2.5),
        (-0.7,  1.5,  0.7, -2.5, 6.0,  2.5),
        // Some chips fly straight up
        ( 0.1,  0.5,  0.1,  0.3, 7.0,  0.3),
        (-0.1,  0.5, -0.1, -0.3, 7.0, -0.3),
        ( 0.0,  2.0,  0.0,  0.0, 8.0,  0.0),
        ( 0.3,  1.0,  0.0,  1.0, 5.5,  0.5),
    ];

    for (i, &(ox, oy, oz, vx, vy, vz)) in offsets.iter().enumerate() {
        let mat = if i % 3 == 0 { wood_mat.clone() } else { bark_mat.clone() };
        commands.spawn((
            PbrBundle {
                mesh: particle_mesh.clone(),
                material: mat,
                transform: Transform::from_translation(
                    center + Vec3::new(ox, oy, oz)
                ).with_scale(Vec3::splat(0.2)),
                ..default()
            },
            WoodParticle {
                velocity: Vec3::new(vx, vy, vz),
                age: 0.0,
                lifetime: 1.2,
            },
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

        // Move
        transform.translation += particle.velocity * dt;
        // Gravity
        particle.velocity.y -= 14.0 * dt;
        // Shrink as they age
        let scale = (1.0 - t * 0.8).max(0.02);
        transform.scale = Vec3::splat(scale * 0.2);
        // Tumble
        transform.rotate_x(dt * 6.0);
        transform.rotate_z(dt * 4.0);
        transform.rotate_y(dt * 3.0);
    }
}

fn spawn_wood_drops(
    commands: &mut Commands,
    asset_server: &AssetServer,
    tree_pos: Vec3,
) {
    // Drop 3 wood log items scattered around the base
    let drop_offsets = [
        Vec3::new( 0.8, 0.5,  0.3),
        Vec3::new(-0.6, 0.5,  0.7),
        Vec3::new( 0.2, 0.5, -0.8),
    ];

    for offset in drop_offsets {
        let drop_pos = tree_pos + offset;
        commands.spawn((
            SceneBundle {
                scene: asset_server.load("block.glb#Scene0"),
                transform: Transform::from_translation(drop_pos)
                    .with_scale(Vec3::splat(0.35)),
                ..default()
            },
            WoodDrop {
                origin_y: drop_pos.y,
                age: 0.0,
            },
        ));
    }
}

fn update_wood_drops(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut WoodDrop)>,
    camera_query: Query<&Transform, (With<crate::camera::Player>, Without<WoodDrop>)>,
    time: Res<Time>,
) {
    let player_pos = camera_query.get_single()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    let dt = time.delta_seconds();

    for (entity, mut transform, mut drop) in query.iter_mut() {
        drop.age += dt;

        // Bob up and down
        transform.translation.y = drop.origin_y + (drop.age * 2.0).sin() * 0.12;

        // Slowly rotate
        transform.rotate_y(dt * 1.5);

        // Collect on player proximity
        let dist = (transform.translation - player_pos).length();
        if dist < 1.8 {
            commands.entity(entity).despawn_recursive();
        }

        // Despawn after 30 seconds
        if drop.age > 30.0 {
            commands.entity(entity).despawn_recursive();
        }
    }
}