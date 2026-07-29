use bevy::prelude::*;

use crate::components::{
    Asteroid, Moon, PilotCamera, Planet, PlanetAreaLight, Ship, SkyboxSphere, SpaceDust, Starfield, Sun,
};
use crate::resources::FlightState;

type SunRenderQueryFilter = (
    Without<Ship>,
    Without<Planet>,
    Without<Moon>,
    Without<Asteroid>,
    Without<SpaceDust>,
    Without<Starfield>,
    Without<SkyboxSphere>,
    Without<PilotCamera>,
    Without<PlanetAreaLight>,
);
type PlanetRenderQueryFilter = (
    Without<Ship>,
    Without<Sun>,
    Without<Moon>,
    Without<Asteroid>,
    Without<SpaceDust>,
    Without<Starfield>,
    Without<SkyboxSphere>,
    Without<PilotCamera>,
    Without<PlanetAreaLight>,
);
type MoonRenderQueryFilter = (
    Without<Ship>,
    Without<Sun>,
    Without<Planet>,
    Without<Asteroid>,
    Without<SpaceDust>,
    Without<Starfield>,
    Without<SkyboxSphere>,
    Without<PilotCamera>,
    Without<PlanetAreaLight>,
);
type AsteroidRenderQueryFilter = (
    Without<Ship>,
    Without<Sun>,
    Without<Planet>,
    Without<Moon>,
    Without<SpaceDust>,
    Without<Starfield>,
    Without<SkyboxSphere>,
    Without<PilotCamera>,
    Without<PlanetAreaLight>,
);
type DustRenderQueryFilter = (
    Without<Ship>,
    Without<Sun>,
    Without<Planet>,
    Without<Moon>,
    Without<Asteroid>,
    Without<Starfield>,
    Without<SkyboxSphere>,
    Without<PilotCamera>,
    Without<PlanetAreaLight>,
);
type StarRenderQueryFilter = (
    Without<Ship>,
    Without<Sun>,
    Without<Planet>,
    Without<Moon>,
    Without<Asteroid>,
    Without<SpaceDust>,
    Without<SkyboxSphere>,
    Without<PilotCamera>,
    Without<PlanetAreaLight>,
);
type SkyboxRenderQueryFilter = (
    Without<Ship>,
    Without<Sun>,
    Without<Planet>,
    Without<Moon>,
    Without<Asteroid>,
    Without<SpaceDust>,
    Without<Starfield>,
    Without<PilotCamera>,
    Without<PlanetAreaLight>,
);

pub fn logarithmic_distance_render_system(
    flight_state: Res<FlightState>,
    ship_query: Query<&Transform, With<Ship>>,
    camera_query: Query<&Transform, (With<PilotCamera>, Without<Ship>)>,
    mut sun_query: Query<(&Sun, &mut Transform, &mut Visibility), SunRenderQueryFilter>,
    mut planet_query: Query<(&Planet, &mut Transform, &mut Visibility), PlanetRenderQueryFilter>,
    mut moon_query: Query<(&Moon, &mut Transform, &mut Visibility), MoonRenderQueryFilter>,
    mut asteroid_query: Query<(&Asteroid, &mut Transform, &mut Visibility), AsteroidRenderQueryFilter>,
    mut dust_query: Query<(&SpaceDust, &mut Transform, &mut Visibility), DustRenderQueryFilter>,
    mut star_query: Query<(&Starfield, &mut Transform, &mut Visibility), StarRenderQueryFilter>,
    mut skybox_query: Query<&mut Transform, (With<SkyboxSphere>, SkyboxRenderQueryFilter)>,
) {
    let Ok(ship_transform) = ship_query.single() else { return; };
    let Ok(cam_transform) = camera_query.single() else { return; };

    let cam_pos = flight_state.world_pos + ship_transform.rotation * cam_transform.translation;

    let k = 0.000035;
    let scale_const = 6500.0;
    let transition_dist = 6000.0;

    // Render Sun
    for (sun, mut transform, mut vis) in &mut sun_query {
        let sun_world_pos = Vec3::ZERO;
        let vec_to = sun_world_pos - cam_pos;
        let d_real = vec_to.length();

        if d_real < 0.1 {
            transform.translation = Vec3::ZERO;
            transform.scale = Vec3::ONE;
            *vis = Visibility::Inherited;
            continue;
        }

        let dir = vec_to / d_real;
        *vis = Visibility::Inherited;

        let blend = (d_real / transition_dist).clamp(0.0, 1.0);
        let blend_smooth = blend * blend * (3.0 - 2.0 * blend);

        let d_log = scale_const * (1.0 + k * d_real).ln();
        let d_vis = d_real * (1.0 - blend_smooth) + d_log * blend_smooth;

        transform.translation = dir * d_vis;

        let scale_log = (d_vis / d_real).clamp(0.001, 1.0);
        let min_scale = (0.025 * d_vis) / sun.radius;
        let final_scale_log = scale_log.max(min_scale);

        let final_scale = 1.0 * (1.0 - blend_smooth) + final_scale_log * blend_smooth;
        transform.scale = Vec3::splat(final_scale);
    }

    // Render Planets
    for (planet, mut transform, mut vis) in &mut planet_query {
        let vec_to = planet.world_pos - cam_pos;
        let d_real = vec_to.length();

        if d_real < 0.1 {
            transform.translation = Vec3::ZERO;
            transform.scale = Vec3::ONE;
            *vis = Visibility::Inherited;
            continue;
        }

        let dir = vec_to / d_real;
        *vis = Visibility::Inherited;

        // Smooth transition between 1:1 physical scale (close range) and logarithmic scale (deep space)
        let blend = (d_real / transition_dist).clamp(0.0, 1.0);
        let blend_smooth = blend * blend * (3.0 - 2.0 * blend);

        let d_log = scale_const * (1.0 + k * d_real).ln();
        let d_vis = d_real * (1.0 - blend_smooth) + d_log * blend_smooth;

        transform.translation = dir * d_vis;

        let scale_log = (d_vis / d_real).clamp(0.001, 1.0);
        let min_scale = (0.015 * d_vis) / planet.radius;
        let final_scale_log = scale_log.max(min_scale);

        let final_scale = 1.0 * (1.0 - blend_smooth) + final_scale_log * blend_smooth;
        transform.scale = Vec3::splat(final_scale);
    }

    // Render Moons
    for (moon, mut transform, mut vis) in &mut moon_query {
        let vec_to = moon.world_pos - cam_pos;
        let d_real = vec_to.length();

        if d_real < 0.1 {
            transform.translation = Vec3::ZERO;
            transform.scale = Vec3::ONE;
            *vis = Visibility::Inherited;
            continue;
        }

        let dir = vec_to / d_real;
        *vis = Visibility::Inherited;

        let blend = (d_real / transition_dist).clamp(0.0, 1.0);
        let blend_smooth = blend * blend * (3.0 - 2.0 * blend);

        let d_log = scale_const * (1.0 + k * d_real).ln();
        let d_vis = d_real * (1.0 - blend_smooth) + d_log * blend_smooth;

        transform.translation = dir * d_vis;

        let scale_log = (d_vis / d_real).clamp(0.001, 1.0);
        let min_scale = (0.012 * d_vis) / moon.radius;
        let final_scale_log = scale_log.max(min_scale);

        let final_scale = 1.0 * (1.0 - blend_smooth) + final_scale_log * blend_smooth;
        transform.scale = Vec3::splat(final_scale);
    }

    // Render Asteroids
    for (asteroid, mut transform, mut vis) in &mut asteroid_query {
        let vec_to = asteroid.world_pos - cam_pos;
        let d_real = vec_to.length();

        if d_real > 150_000.0 || d_real < 1.2 {
            *vis = Visibility::Hidden;
            continue;
        }

        let dir = vec_to / d_real.max(0.1);
        *vis = Visibility::Inherited;

        let blend = (d_real / transition_dist).clamp(0.0, 1.0);
        let blend_smooth = blend * blend * (3.0 - 2.0 * blend);

        let d_log = scale_const * (1.0 + k * d_real).ln();
        let d_vis = d_real * (1.0 - blend_smooth) + d_log * blend_smooth;

        transform.translation = dir * d_vis;

        let scale_log = (d_vis / d_real).clamp(0.001, 1.0);
        let min_scale = (0.008 * d_vis) / asteroid.radius.max(0.5);
        let final_scale_log = scale_log.max(min_scale);

        let final_scale = 1.0 * (1.0 - blend_smooth) + final_scale_log * blend_smooth;
        transform.scale = Vec3::splat(final_scale);
    }

    // Render Space Dust Clouds
    for (dust, mut transform, mut vis) in &mut dust_query {
        let vec_to = dust.world_pos - cam_pos;
        let d_real = vec_to.length();

        if d_real > 300_000.0 || d_real < 1.2 {
            *vis = Visibility::Hidden;
            continue;
        }

        let dir = vec_to / d_real.max(0.1);
        *vis = Visibility::Inherited;

        let blend = (d_real / transition_dist).clamp(0.0, 1.0);
        let blend_smooth = blend * blend * (3.0 - 2.0 * blend);

        let d_log = scale_const * (1.0 + k * d_real).ln();
        let d_vis = d_real * (1.0 - blend_smooth) + d_log * blend_smooth;

        transform.translation = dir * d_vis;
        transform.scale = Vec3::splat(dust.size_scale);
    }

    // Render SkyboxSphere (Fixed 360-degree Space Spheremap around camera)
    for mut transform in &mut skybox_query {
        transform.translation = Vec3::ZERO;
    }

    // Render Starfield (Fixed 2D Billboard Point Skybox around camera)
    for (star, mut transform, mut vis) in &mut star_query {
        *vis = Visibility::Inherited;
        let d_vis = 85_000.0;

        let star_render_pos = star.direction * d_vis;
        transform.translation = star_render_pos;
        transform.scale = Vec3::splat(star.size_scale);
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}

pub fn update_planet_area_lights_system(
    mut area_light_query: Query<(&PlanetAreaLight, &ChildOf, &mut PointLight, &mut Transform)>,
    planet_query: Query<(&Planet, &Transform), Without<PlanetAreaLight>>,
    moon_query: Query<(&Moon, &Transform), (Without<Planet>, Without<PlanetAreaLight>)>,
) {
    for (area_light, child_of, mut light, mut transform) in &mut area_light_query {
        let parent_entity = child_of.parent();
        let (world_pos, planet_radius, parent_rot, parent_scale) = if let Ok((planet, p_trans)) = planet_query.get(parent_entity) {
            (planet.world_pos, planet.radius, p_trans.rotation, p_trans.scale.x)
        } else if let Ok((moon, m_trans)) = moon_query.get(parent_entity) {
            (moon.world_pos, moon.radius, m_trans.rotation, m_trans.scale.x)
        } else {
            (area_light.destination_world_pos, area_light.planet_radius, Quat::IDENTITY, 1.0)
        };

        let sun_dir_world = -world_pos.normalize_or_zero();
        if sun_dir_world == Vec3::ZERO {
            continue;
        }

        let local_sun_dir = parent_rot.inverse() * sun_dir_world;
        let visual_radius = planet_radius * parent_scale;

        transform.translation = local_sun_dir * (planet_radius * 3.5);

        light.radius = visual_radius * 1.8;
        light.range = (visual_radius * 12.0).max(10.0);
        let factor = (visual_radius / 100.0).powf(1.2).clamp(0.2, 5.0);
        light.intensity = (15_000_000.0 * factor).max(200_000.0);
    }
}
