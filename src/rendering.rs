use bevy::prelude::*;

use crate::components::{Moon, PilotCamera, Planet, Ship, Starfield};

type PlanetRenderQueryFilter = (Without<Ship>, Without<PilotCamera>);
type MoonRenderQueryFilter = (Without<Ship>, Without<Planet>, Without<PilotCamera>);
type StarRenderQueryFilter = (Without<Ship>, Without<Planet>, Without<Moon>, Without<PilotCamera>);

pub fn logarithmic_distance_render_system(
    ship_query: Query<&Transform, With<Ship>>,
    camera_query: Query<&Transform, (With<PilotCamera>, Without<Ship>)>,
    mut planet_query: Query<(&Planet, &mut Transform, &mut Visibility), PlanetRenderQueryFilter>,
    mut moon_query: Query<(&Moon, &mut Transform, &mut Visibility), MoonRenderQueryFilter>,
    mut star_query: Query<(&Starfield, &mut Transform, &mut Visibility), StarRenderQueryFilter>,
) {
    let Ok(ship_transform) = ship_query.single() else { return; };
    let Ok(cam_transform) = camera_query.single() else { return; };

    let cam_pos = ship_transform.translation + ship_transform.rotation * cam_transform.translation;
    let cam_rot = ship_transform.rotation * cam_transform.rotation;

    let forward = cam_rot * Vec3::NEG_Z;
    let right = cam_rot * Vec3::X;
    let up = cam_rot * Vec3::Y;

    let k = 0.000035;
    let scale_const = 6500.0;

    // Render Planets
    for (planet, mut transform, mut vis) in &mut planet_query {
        let vec_to = planet.world_pos - cam_pos;
        let d_real = vec_to.length();

        if d_real < 1.0 {
            transform.translation = planet.world_pos;
            *vis = Visibility::Inherited;
            continue;
        }

        let dir = vec_to / d_real;
        let z_proj = dir.dot(forward);
        let x_proj = dir.dot(right);
        let y_proj = dir.dot(up);

        let half_fov_tan = 0.85;
        if z_proj <= 0.02 || x_proj.abs() / z_proj > half_fov_tan * 1.5 || y_proj.abs() / z_proj > half_fov_tan * 1.5 {
            *vis = Visibility::Hidden;
            continue;
        }

        *vis = Visibility::Inherited;
        let d_vis = scale_const * (1.0 + k * d_real).ln();
        transform.translation = cam_pos + dir * d_vis;

        let scale_factor = (d_vis / d_real).clamp(0.001, 1.0);
        let _vis_radius = planet.radius * scale_factor;
        let min_scale = (0.015 * d_vis) / planet.radius;
        let final_scale = scale_factor.max(min_scale);

        transform.scale = Vec3::splat(final_scale);
    }

    // Render Moons
    for (moon, mut transform, mut vis) in &mut moon_query {
        let vec_to = moon.world_pos - cam_pos;
        let d_real = vec_to.length();

        if d_real < 1.0 {
            transform.translation = moon.world_pos;
            *vis = Visibility::Inherited;
            continue;
        }

        let dir = vec_to / d_real;
        let z_proj = dir.dot(forward);

        if z_proj <= 0.02 {
            *vis = Visibility::Hidden;
            continue;
        }

        *vis = Visibility::Inherited;
        let d_vis = scale_const * (1.0 + k * d_real).ln();
        transform.translation = cam_pos + dir * d_vis;

        let scale_factor = (d_vis / d_real).clamp(0.001, 1.0);
        let min_scale = (0.012 * d_vis) / moon.radius;
        let final_scale = scale_factor.max(min_scale);

        transform.scale = Vec3::splat(final_scale);
    }

    // Render Starfield
    for (star, mut transform, mut vis) in &mut star_query {
        let vec_to = star.world_pos - cam_pos;
        let d_real = vec_to.length();
        let dir = vec_to / d_real;

        let z_proj = dir.dot(forward);
        if z_proj <= 0.01 {
            *vis = Visibility::Hidden;
            continue;
        }

        *vis = Visibility::Inherited;
        let d_vis = 85_000.0;
        transform.translation = cam_pos + dir * d_vis;
    }
}
