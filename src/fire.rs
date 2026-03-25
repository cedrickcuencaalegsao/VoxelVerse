use bevy::prelude::*;
use crate::daynight::DayNightCycle;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParticleKind {
    Fire,
    Ember,
    Smoke,
}

#[derive(Component)]
pub struct FireEffect;

#[derive(Component)]
pub struct FireParticle {
    pub velocity: Vec3,
    pub lifetime: f32,
    pub age: f32,
    pub kind: ParticleKind,
}

#[derive(Component)]
pub struct CampfireLight;

#[derive(Component)]
pub struct CampfireGlow;

pub struct FirePlugin;

impl Plugin for FirePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_fire)
            .add_systems(Update, (
                spawn_fire_cubes,
                update_fire_cubes,
                flicker_campfire_light,
            ));
    }
}

#[derive(Resource)]
pub struct FireCubeAssets {
    pub fire_mesh: Handle<Mesh>,
    pub fire_mat: Handle<StandardMaterial>,
    pub ember_mat: Handle<StandardMaterial>,
    pub smoke_mat: Handle<StandardMaterial>,
    pub timer: Timer,
    pub rng_state: u32,
}

impl FireCubeAssets {
    fn rand(&mut self) -> f32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 17;
        self.rng_state ^= self.rng_state << 5;
        (self.rng_state as f32) / (u32::MAX as f32)
    }

    fn rand_range(&mut self, min: f32, max: f32) -> f32 {
        min + self.rand() * (max - min)
    }
}

fn spawn_fire(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        SceneBundle {
            scene: asset_server.load("fire.glb#Scene0"),
            transform: Transform::from_xyz(3.0, 36.0, 0.0),
            ..default()
        },
        FireEffect,
    ));

    // Main campfire point light — always on, scales with day/night
    commands.spawn((
        PointLightBundle {
            point_light: PointLight {
                intensity: 200_000.0,
                range: 25.0,
                color: Color::srgb(1.0, 0.6, 0.2),
                shadows_enabled: false,
                ..default()
            },
            transform: Transform::from_xyz(3.0, 37.5, 0.0),
            ..default()
        },
        CampfireLight,
    ));

    // Secondary ground glow — wider, softer
    commands.spawn((
        PointLightBundle {
            point_light: PointLight {
                intensity: 60_000.0,
                range: 12.0,
                color: Color::srgb(1.0, 0.5, 0.15),
                shadows_enabled: false,
                ..default()
            },
            transform: Transform::from_xyz(3.0, 36.3, 0.0),
            ..default()
        },
        CampfireGlow,
    ));

    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    let fire_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.4, 0.0),
        emissive: LinearRgba::new(2.0, 0.8, 0.0, 1.0),
        unlit: true,
        ..default()
    });

    let ember_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.9, 0.2),
        emissive: LinearRgba::new(3.0, 1.5, 0.0, 1.0),
        unlit: true,
        ..default()
    });

    let smoke_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.3, 0.3, 0.3, 0.5),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    commands.insert_resource(FireCubeAssets {
        fire_mesh: cube_mesh,
        fire_mat,
        ember_mat,
        smoke_mat,
        timer: Timer::from_seconds(0.04, TimerMode::Repeating),
        rng_state: 12345678,
    });
}

fn flicker_campfire_light(
    time: Res<Time>,
    cycle: Res<DayNightCycle>,
    mut main_query: Query<&mut PointLight, (With<CampfireLight>, Without<CampfireGlow>)>,
    mut glow_query: Query<&mut PointLight, (With<CampfireGlow>, Without<CampfireLight>)>,
) {
    let t = time.elapsed_seconds();

    // Sun height — sin of the day angle
    let angle  = cycle.time * std::f32::consts::TAU;
    let sun_y  = angle.sin(); // -1.0 night .. 1.0 noon

    // During day the sun washes out the fire — minimum 10% visible
    // During night the fire is the main light source — 100%
    // Smoothly interpolate: night = 1.0, midday = 0.10
    let night_boost = if sun_y > 0.0 {
        // Daytime: fire looks dim next to the sun
        1.0 - sun_y * 0.90  // 1.0 at sunrise, 0.10 at noon
    } else {
        // Nighttime: full brightness
        1.0
    };

    // Minimum base so fire is always at least ~10 watts visible
    // 10_000 lux ≈ roughly "10 watts of noticeable light" in Bevy's units
    let min_intensity: f32 = 10_000.0;

    // Flicker factor — combines multiple sine waves for organic feel
    let flicker = 1.0
        + (t * 7.3).sin()  * 0.15
        + (t * 13.7).sin() * 0.08
        + (t * 3.1).cos()  * 0.07;
    let flicker = flicker.clamp(0.75, 1.3);

    // Main light: 200_000 max at night, always >= min_intensity
    let main_base = 200_000.0_f32 * night_boost;
    let main_intensity = (main_base * flicker).max(min_intensity);

    // Color shifts more orange/red at night, slightly cooler at day
    let heat = ((t * 5.0).sin() * 0.5 + 0.5) as f32;
    let r = 1.0_f32;
    let g = if sun_y > 0.0 { 0.55 + heat * 0.2 } else { 0.45 + heat * 0.25 };
    let b = if sun_y > 0.0 { 0.15 + heat * 0.1 } else { 0.05 + heat * 0.1 };

    for mut light in main_query.iter_mut() {
        light.intensity = main_intensity;
        light.color     = Color::srgb(r, g, b);
        // Expand range slightly at night so the glow reaches further
        light.range = if sun_y < 0.0 { 30.0 } else { 22.0 };
    }

    // Secondary glow: 60_000 max at night, always >= min_intensity / 3
    let glow_base = 60_000.0_f32 * night_boost;
    let glow_intensity = (glow_base * flicker).max(min_intensity / 3.0);

    for mut light in glow_query.iter_mut() {
        light.intensity = glow_intensity;
        light.color     = Color::srgb(1.0, 0.45 + heat * 0.2, 0.1);
        light.range     = if sun_y < 0.0 { 14.0 } else { 10.0 };
    }
}

fn spawn_fire_cubes(
    mut commands: Commands,
    mut assets: ResMut<FireCubeAssets>,
    time: Res<Time>,
) {
    assets.timer.tick(time.delta());
    if !assets.timer.just_finished() {
        return;
    }

    let base = Vec3::new(3.0, 36.2, 0.0);

    // --- FIRE cubes ---
    for _ in 0..4 {
        let radius  = assets.rand_range(0.0, 0.4);
        let angle   = assets.rand_range(0.0, std::f32::consts::TAU);
        let offset  = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
        let speed_up = assets.rand_range(1.2, 2.2);
        let drift_x  = assets.rand_range(-0.2, 0.2);
        let drift_z  = assets.rand_range(-0.2, 0.2);
        let size     = assets.rand_range(0.06, 0.18);
        let lifetime = assets.rand_range(0.5, 1.0);

        commands.spawn((
            PbrBundle {
                mesh: assets.fire_mesh.clone(),
                material: assets.fire_mat.clone(),
                transform: Transform::from_translation(base + offset)
                    .with_scale(Vec3::new(size, size * assets.rand_range(0.8, 1.3), size)),
                ..default()
            },
            FireParticle {
                velocity: Vec3::new(drift_x, speed_up, drift_z),
                lifetime,
                age: 0.0,
                kind: ParticleKind::Fire,
            },
        ));
    }

    // --- EMBERS ---
    if assets.rand() > 0.6 {
        let angle   = assets.rand_range(0.0, std::f32::consts::TAU);
        let radius  = assets.rand_range(0.0, 0.3);
        let offset  = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
        let size     = assets.rand_range(0.04, 0.10);
        let lifetime = assets.rand_range(1.2, 2.2);

        commands.spawn((
            PbrBundle {
                mesh: assets.fire_mesh.clone(),
                material: assets.ember_mat.clone(),
                transform: Transform::from_translation(base + offset + Vec3::Y * 0.3)
                    .with_scale(Vec3::splat(size)),
                ..default()
            },
            FireParticle {
                velocity: Vec3::new(
                    assets.rand_range(-0.5, 0.5),
                    assets.rand_range(2.5, 4.5),
                    assets.rand_range(-0.5, 0.5),
                ),
                lifetime,
                age: 0.0,
                kind: ParticleKind::Ember,
            },
        ));
    }

    // --- SMOKE ---
    if assets.rand() > 0.7 {
        let angle   = assets.rand_range(0.0, std::f32::consts::TAU);
        let radius  = assets.rand_range(0.0, 0.2);
        let offset  = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
        let size     = assets.rand_range(0.15, 0.30);
        let lifetime = assets.rand_range(2.5, 4.0);

        commands.spawn((
            PbrBundle {
                mesh: assets.fire_mesh.clone(),
                material: assets.smoke_mat.clone(),
                transform: Transform::from_translation(base + offset + Vec3::Y * 1.8)
                    .with_scale(Vec3::splat(size)),
                ..default()
            },
            FireParticle {
                velocity: Vec3::new(
                    assets.rand_range(-0.08, 0.08),
                    assets.rand_range(0.4, 0.9),
                    assets.rand_range(-0.08, 0.08),
                ),
                lifetime,
                age: 0.0,
                kind: ParticleKind::Smoke,
            },
        ));
    }
}

fn update_fire_cubes(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut FireParticle)>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();

    for (entity, mut transform, mut particle) in query.iter_mut() {
        particle.age += dt;

        if particle.age >= particle.lifetime {
            commands.entity(entity).despawn();
            continue;
        }

        transform.translation += particle.velocity * dt;

        match particle.kind {
            ParticleKind::Fire => {
                particle.velocity.y -= 1.5 * dt;
                let new_scale = transform.scale.x * (1.0 - dt * 1.2);
                transform.scale = Vec3::splat(new_scale.max(0.01));
                transform.rotate_y(dt * 1.5);
                transform.rotate_x(dt * 1.0);
            }
            ParticleKind::Ember => {
                particle.velocity.y -= 2.0 * dt;
                let new_scale = transform.scale.x * (1.0 - dt * 0.8);
                transform.scale = Vec3::splat(new_scale.max(0.01));
                transform.rotate_y(dt * 3.0);
                transform.rotate_z(dt * 2.0);
            }
            ParticleKind::Smoke => {
                let new_scale = transform.scale.x + dt * 0.08;
                transform.scale = Vec3::splat(new_scale.min(0.6));
            }
        }
    }
}