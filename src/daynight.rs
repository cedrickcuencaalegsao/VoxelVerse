use bevy::prelude::*;

// Full day/night cycle duration in seconds (10 minutes real time)
const DAY_DURATION: f32 = 600.0;

#[derive(Resource)]
pub struct DayNightCycle {
    pub time: f32,       // 0.0 to 1.0 — 0.0 = midnight, 0.25 = sunrise, 0.5 = noon, 0.75 = sunset
}

impl Default for DayNightCycle {
    fn default() -> Self {
        Self { time: 0.25 } // start at sunrise
    }
}

pub struct DayNightPlugin;

impl Plugin for DayNightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DayNightCycle>()
            .add_systems(Update, (update_day_night, update_sky_color));
    }
}

fn update_day_night(
    time: Res<Time>,
    mut cycle: ResMut<DayNightCycle>,
    mut sun_query: Query<(&mut Transform, &mut DirectionalLight)>,
) {
    // Advance time
    cycle.time = (cycle.time + time.delta_seconds() / DAY_DURATION) % 1.0;
    let t = cycle.time;

    // Sun angle — full 360 degrees over the day
    // 0.25 = sunrise (east), 0.5 = noon (overhead), 0.75 = sunset (west)
    let angle = t * std::f32::consts::TAU; // 0 to 2π

    // Sun position orbits on the X/Y plane
    let sun_x = angle.cos();
    let sun_y = angle.sin();

    for (mut transform, mut light) in sun_query.iter_mut() {
        // Rotate the sun around the world
        transform.translation = Vec3::new(sun_x * 200.0, sun_y * 200.0, 50.0);
        transform.look_at(Vec3::ZERO, Vec3::Y);

        // Illuminance based on sun height
        // sun_y > 0 = daytime, sun_y < 0 = night
        let sun_height = sun_y; // -1.0 to 1.0

        light.illuminance = if sun_height > 0.1 {
            // Full day — up to 12000 lux at noon
            sun_height.powf(0.4) * 12000.0
        } else if sun_height > -0.1 {
            // Twilight — fade from 800 to 0
            let twilight_t = (sun_height + 0.1) / 0.2; // 0 to 1
            twilight_t * 800.0
        } else {
            // Night — very dim moonlight
            50.0
        };

        // Sun color — warm orange at sunrise/sunset, white at noon, dark at night
        light.color = sun_color(sun_height);
    }
}

fn sun_color(sun_height: f32) -> Color {
    if sun_height > 0.3 {
        // Midday — pure white
        Color::srgb(1.0, 1.0, 1.0)
    } else if sun_height > 0.0 {
        // Morning/evening — orange tint
        let t = sun_height / 0.3; // 0 to 1
        Color::srgb(1.0, 0.6 + t * 0.4, 0.3 + t * 0.7)
    } else if sun_height > -0.1 {
        // Twilight — deep orange/red
        let t = (sun_height + 0.1) / 0.1; // 0 to 1
        Color::srgb(1.0, 0.3 + t * 0.3, 0.1 + t * 0.2)
    } else {
        // Night — cool blue moonlight
        Color::srgb(0.3, 0.35, 0.5)
    }
}

fn update_sky_color(
    cycle: Res<DayNightCycle>,
    mut clear_color: ResMut<ClearColor>,
) {
    let t = cycle.time;

    // Sun height at current time
    let angle   = t * std::f32::consts::TAU;
    let sun_y   = angle.sin();

    let sky = if sun_y > 0.3 {
        // Day — bright blue sky
        Color::srgb(0.53, 0.81, 0.92)
    } else if sun_y > 0.0 {
        // Sunrise/sunset — blend from orange to blue
        let fade = sun_y / 0.3;
        Color::srgb(
            0.53 + (1.0 - fade) * 0.47,   // more red at horizon
            0.81 * fade + 0.4 * (1.0 - fade),
            0.92 * fade + 0.1 * (1.0 - fade),
        )
    } else if sun_y > -0.15 {
        // Dusk/dawn — dark orange fading to dark blue
        let fade = (sun_y + 0.15) / 0.15;
        Color::srgb(
            fade * 1.0 + (1.0 - fade) * 0.02,
            fade * 0.4 + (1.0 - fade) * 0.02,
            fade * 0.1 + (1.0 - fade) * 0.08,
        )
    } else {
        // Night — deep dark blue
        Color::srgb(0.02, 0.02, 0.08)
    };

    clear_color.0 = sky;
}