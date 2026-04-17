use bevy::prelude::*;
use crate::camera::{MainCamera, Player};
use crate::inventory::{Inventory, ItemKind, Pickup};
use crate::physics::PLAYER_HEIGHT;

const WEED_REACH: f32 = 5.0;
const BREAK_TIME: f32 = 0.25;

#[derive(Component)]
pub struct Weed {
    pub _ground_pos: IVec3,
}

#[derive(Resource, Default)]
pub struct WeedBreakingState {
    pub target: Option<Entity>,
    pub progress: f32,
}

#[derive(Component)]
pub struct WeedParticle {
    pub velocity: Vec3,
    pub age: f32,
}

#[derive(Component)]
pub struct WeedDrop {
    pub origin_y: f32,
    pub age: f32,
}

pub struct WeedBreakingPlugin;

impl Plugin for WeedBreakingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeedBreakingState>()
            .add_systems(Update, (
                raycast_weed,
                break_weed,
                update_particles,
                update_weed_drops,
            ).chain());
    }
}

fn raycast_weed(
    mut state: ResMut<WeedBreakingState>,
    cam_q: Query<&Transform, With<MainCamera>>,
    weeds: Query<(Entity, &GlobalTransform), With<Weed>>,
) {
    let Ok(cam) = cam_q.get_single() else { return };

    let origin = cam.translation;
    let forward: Vec3 = cam.forward().into();

    let mut best = None;
    let mut best_dist = f32::MAX;

    for (entity, transform) in weeds.iter() {
        let pos = transform.translation();
        let to = pos - origin;
        let dist = to.length();

        if dist > WEED_REACH { continue; }
        if to.normalize_or_zero().dot(forward) < 0.97 { continue; }

        if dist < best_dist {
            best = Some(entity);
            best_dist = dist;
        }
    }

    // reset progress if target changed
    if state.target != best {
        state.progress = 0.0;
    }

    state.target = best;
}

fn break_weed(
    mut commands: Commands,
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut state: ResMut<WeedBreakingState>,
    weeds: Query<&Transform, With<Weed>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(target) = state.target else { return };

    if !mouse.pressed(MouseButton::Left) {
        state.progress = 0.0;
        return;
    }

    state.progress += time.delta_seconds();

    if state.progress >= BREAK_TIME {
        if let Ok(transform) = weeds.get(target) {
            let pos = transform.translation;

            // 💥 particles
            spawn_particles(&mut commands, &mut meshes, &mut materials, pos);

            // 🌿 drop a collectible weed
            spawn_weed_drop(&mut commands, &asset_server, pos);

            // 🌱 70% chance to respawn
            if rand::random::<f32>() < 0.7 {
                commands.spawn(SceneBundle {
                    scene: asset_server.load("weed_1.glb#Scene0"),
                    transform: Transform::from_translation(pos),
                    ..default()
                })
                .insert(Weed {
                    _ground_pos: IVec3::new(
                        pos.x.floor() as i32,
                        pos.y.floor() as i32 - 1,
                        pos.z.floor() as i32,
                    ),
                });
            }
        }

        commands.entity(target).despawn_recursive();

        state.target = None;
        state.progress = 0.0;
    }
}

fn spawn_weed_drop(commands: &mut Commands, asset_server: &AssetServer, pos: Vec3) {
    let drop_y = pos.y + 0.25;
    commands.spawn((
        SceneBundle {
            scene: asset_server.load("weed_1.glb#Scene0"),
            transform: Transform::from_translation(Vec3::new(pos.x, drop_y, pos.z))
                .with_scale(Vec3::splat(0.5)),
            ..default()
        },
        WeedDrop { origin_y: drop_y, age: 0.0 },
        Pickup { kind: ItemKind::Weed },
    ));
}

fn update_weed_drops(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut WeedDrop, Option<&Pickup>)>,
    player_query: Query<&Transform, (With<Player>, Without<WeedDrop>)>,
    mut inventory: ResMut<Inventory>,
    time: Res<Time>,
) {
    let player_feet = player_query
        .get_single()
        .map(|t| t.translation - Vec3::Y * PLAYER_HEIGHT)
        .unwrap_or(Vec3::ZERO);

    let dt = time.delta_seconds();

    for (entity, mut transform, mut drop, pickup) in query.iter_mut() {
        drop.age += dt;

        transform.translation.y = drop.origin_y + (drop.age * 2.5).sin() * 0.12;
        transform.rotate_y(dt * 1.8);

        let dist = (transform.translation - player_feet).length();
        if dist < 1.8 {
            if let Some(p) = pickup {
                inventory.add(p.kind, 1);
            }
            commands.entity(entity).despawn_recursive();
            continue;
        }

        if drop.age > 30.0 {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn spawn_particles(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
) {
    let mesh = meshes.add(Cuboid::new(0.1, 0.1, 0.1));
    let mat = materials.add(Color::srgb(0.2, 0.8, 0.2));

    let velocities = [
        Vec3::new(1.0, 2.5, 0.0),
        Vec3::new(-1.0, 2.0, 0.5),
        Vec3::new(0.5, 3.0, -1.0),
        Vec3::new(-0.5, 2.2, -0.5),
    ];

    for v in velocities {
        commands.spawn((
            PbrBundle {
                mesh: mesh.clone(),
                material: mat.clone(),
                transform: Transform::from_translation(center),
                ..default()
            },
            WeedParticle {
                velocity: v,
                age: 0.0,
            },
        ));
    }
}

fn update_particles(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut WeedParticle)>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();

    for (e, mut t, mut p) in query.iter_mut() {
        p.age += dt;

        if p.age > 0.6 {
            commands.entity(e).despawn();
            continue;
        }

        t.translation += p.velocity * dt;
        p.velocity.y -= 9.8 * dt;
    }
}