use bevy::prelude::*;
use bevy::transform::TransformSystems;

use crate::components::{
    Asteroid, Moon, PilotCamera, Planet, PlanetAreaLight, Ship, SkyboxSphere, SpaceDust, Starfield, Sun,
    SunAnimation, SunDirectionalLight,
};
use crate::resources::{AppState, FlightState};

type NonRenderObjectFilter = (Without<Ship>, Without<PilotCamera>, Without<PlanetAreaLight>);
type AreaLightFilter = (Without<Planet>, Without<PlanetAreaLight>);
type SunLightFilter = (With<SunDirectionalLight>, Without<Ship>, Without<PilotCamera>);

type SunRenderQueryFilter = (With<Sun>, Without<Planet>, Without<Moon>, Without<Asteroid>, Without<SpaceDust>, Without<Starfield>, Without<SkyboxSphere>, NonRenderObjectFilter);
type PlanetRenderQueryFilter = (With<Planet>, Without<Sun>, Without<Moon>, Without<Asteroid>, Without<SpaceDust>, Without<Starfield>, Without<SkyboxSphere>, NonRenderObjectFilter);
type MoonRenderQueryFilter = (With<Moon>, Without<Sun>, Without<Planet>, Without<Asteroid>, Without<SpaceDust>, Without<Starfield>, Without<SkyboxSphere>, NonRenderObjectFilter);
type AsteroidRenderQueryFilter = (With<Asteroid>, Without<Sun>, Without<Planet>, Without<Moon>, Without<SpaceDust>, Without<Starfield>, Without<SkyboxSphere>, NonRenderObjectFilter);
type DustRenderQueryFilter = (With<SpaceDust>, Without<Sun>, Without<Planet>, Without<Moon>, Without<Asteroid>, Without<Starfield>, Without<SkyboxSphere>, NonRenderObjectFilter);
type StarRenderQueryFilter = (With<Starfield>, Without<Sun>, Without<Planet>, Without<Moon>, Without<Asteroid>, Without<SpaceDust>, Without<SkyboxSphere>, NonRenderObjectFilter);
type SkyboxRenderQueryFilter = (With<SkyboxSphere>, Without<Sun>, Without<Planet>, Without<Moon>, Without<Asteroid>, Without<SpaceDust>, Without<Starfield>, NonRenderObjectFilter);

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            animate_sun_surface_system.run_if(in_state(AppState::InGame)),
        )
        .add_systems(
            PostUpdate,
            (
                logarithmic_distance_render_system.before(TransformSystems::Propagate),
                update_directional_sunlight_system.before(TransformSystems::Propagate),
                update_planet_area_lights_system.before(TransformSystems::Propagate),
            )
                .run_if(in_state(AppState::InGame)),
        );
    }
}

fn compute_logarithmic_transform(
    world_pos: Vec3,
    cam_pos: Vec3,
    radius: f32,
    min_scale_factor: f32,
) -> (Vec3, Vec3, Visibility) {
    let vec_to = world_pos - cam_pos;
    let d_real = vec_to.length();

    if d_real < 0.1 {
        return (Vec3::ZERO, Vec3::ONE, Visibility::Inherited);
    }

    let dir = vec_to / d_real;
    let k = 0.0000003;
    let scale_const = 30000.0;
    let transition_dist = 100000.0;

    let blend = (d_real / transition_dist).clamp(0.0, 1.0);
    let blend_smooth = blend * blend * (3.0 - 2.0 * blend);

    let d_log = scale_const * (1.0 + k * d_real).ln();
    let d_vis = d_real * (1.0 - blend_smooth) + d_log * blend_smooth;

    let scale_log = (d_vis / d_real).clamp(0.001, 1.0);
    let min_scale = (min_scale_factor * d_vis) / radius.max(0.1);
    let final_scale_log = scale_log.max(min_scale);

    let final_scale = 1.0 * (1.0 - blend_smooth) + final_scale_log * blend_smooth;

    (dir * d_vis, Vec3::splat(final_scale), Visibility::Inherited)
}

#[allow(clippy::too_many_arguments)]
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
    mut skybox_query: Query<&mut Transform, SkyboxRenderQueryFilter>,
) {
    let Ok(ship_transform) = ship_query.single() else { return; };
    let Ok(cam_transform) = camera_query.single() else { return; };

    let cam_pos = flight_state.world_pos + ship_transform.rotation * cam_transform.translation;

    // Render Sun
    for (sun, mut transform, mut vis) in &mut sun_query {
        let (pos, scale, v) = compute_logarithmic_transform(Vec3::ZERO, cam_pos, sun.radius, 0.035);
        transform.translation = pos;
        transform.scale = scale;
        *vis = v;
    }

    // Render Planets
    for (planet, mut transform, mut vis) in &mut planet_query {
        let (pos, scale, v) = compute_logarithmic_transform(planet.world_pos, cam_pos, planet.radius, 0.015);
        transform.translation = pos;
        transform.scale = scale;
        *vis = v;
    }

    // Render Moons
    for (moon, mut transform, mut vis) in &mut moon_query {
        let (pos, scale, v) = compute_logarithmic_transform(moon.world_pos, cam_pos, moon.radius, 0.012);
        transform.translation = pos;
        transform.scale = scale;
        *vis = v;
    }

    // Render Asteroids
    for (asteroid, mut transform, mut vis) in &mut asteroid_query {
        let d_real = asteroid.world_pos.distance(cam_pos);
        if !(1.2..=150_000.0).contains(&d_real) {
            *vis = Visibility::Hidden;
            continue;
        }
        let (pos, scale, v) = compute_logarithmic_transform(asteroid.world_pos, cam_pos, asteroid.radius.max(0.5), 0.008);
        transform.translation = pos;
        transform.scale = scale;
        *vis = v;
    }

    // Render Space Dust Clouds
    for (dust, mut transform, mut vis) in &mut dust_query {
        let d_real = dust.world_pos.distance(cam_pos);
        if !(1.2..=300_000.0).contains(&d_real) {
            *vis = Visibility::Hidden;
            continue;
        }
        let (pos, _, v) = compute_logarithmic_transform(dust.world_pos, cam_pos, 1.0, 0.001);
        transform.translation = pos;
        transform.scale = Vec3::splat(dust.size_scale);
        *vis = v;
    }

    // Render SkyboxSphere (Fixed 360-degree Space Spheremap around camera)
    for mut transform in &mut skybox_query {
        transform.translation = Vec3::ZERO;
    }

    // Render Starfield (Fixed 2D Billboard Point Skybox around camera)
    for (star, mut transform, mut vis) in &mut star_query {
        *vis = Visibility::Inherited;
        let d_vis = 800_000.0;

        let star_render_pos = star.direction * d_vis;
        transform.translation = star_render_pos;
        transform.scale = Vec3::splat(star.size_scale);
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}

pub fn update_planet_area_lights_system(
    mut area_light_query: Query<(&PlanetAreaLight, &ChildOf, &mut PointLight, &mut Transform)>,
    planet_query: Query<(&Planet, &Transform), Without<PlanetAreaLight>>,
    moon_query: Query<(&Moon, &Transform), AreaLightFilter>,
) {
    for (area_light, child_of, mut light, mut transform) in &mut area_light_query {
        let parent_entity = child_of.parent();
        let (world_pos, planet_radius, parent_rot) = if let Ok((planet, p_trans)) = planet_query.get(parent_entity) {
            (planet.world_pos, planet.radius, p_trans.rotation)
        } else if let Ok((moon, m_trans)) = moon_query.get(parent_entity) {
            (moon.world_pos, moon.radius, m_trans.rotation)
        } else {
            (area_light.destination_world_pos, area_light.planet_radius, Quat::IDENTITY)
        };

        let sun_dir_world = -world_pos.normalize_or_zero();
        if sun_dir_world == Vec3::ZERO {
            continue;
        }

        let local_sun_dir = parent_rot.inverse() * sun_dir_world;

        transform.translation = local_sun_dir * (planet_radius * 3.5);

        light.radius = 0.0;
        light.range = 0.0;
        light.intensity = 0.0;
    }
}

pub fn update_directional_sunlight_system(
    flight_state: Res<FlightState>,
    ship_query: Query<&Transform, With<Ship>>,
    mut sun_light_query: Query<&mut Transform, SunLightFilter>,
) {
    let Ok(ship_transform) = ship_query.single() else { return; };
    let cam_pos = flight_state.world_pos + ship_transform.rotation * Vec3::new(0.0, 1.2, 4.0);

    let dir_to_sun = (-cam_pos).normalize_or_zero();
    if dir_to_sun != Vec3::ZERO {
        for mut light_transform in &mut sun_light_query {
            *light_transform = Transform::IDENTITY.looking_at(dir_to_sun, Vec3::Y);
        }
    }
}

pub fn animate_sun_surface_system(
    time: Res<Time>,
    mut sun_query: Query<(&Sun, &mut SunAnimation, &mut Transform, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let delta = time.delta();
    let delta_secs = time.delta_secs();

    for (_sun, mut anim, mut transform, mat_handle) in &mut sun_query {
        // Slow natural spherical rotation of the Sun along Y-axis
        transform.rotate_y(0.005 * delta_secs);

        anim.frame_timer.tick(delta);
        anim.pulse_timer += delta_secs;

        if let Some(mut mat) = materials.get_mut(mat_handle) {
            if anim.frame_timer.just_finished() && !anim.frame_handles.is_empty() {
                anim.current_frame = (anim.current_frame + 1) % anim.frame_handles.len();
                let next_tex = anim.frame_handles[anim.current_frame].clone();
                mat.base_color_texture = Some(next_tex.clone());
                mat.emissive_texture = Some(next_tex);
            }

            // Gentle solar flare emissive pulsation
            let pulse = (anim.pulse_timer * 0.5).sin() * 1.5;
            mat.emissive = LinearRgba::new(35.0 + pulse, 25.0 + pulse * 0.7, 6.0, 1.0);
        }
    }
}

