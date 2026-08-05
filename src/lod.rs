use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use bevy::render::render_resource::TextureFormat;

use crate::components::{Moon, Planet};
use crate::resources::{AppState, AutoPilotState, FlightState};

pub struct PlanetLodPlugin;

impl Plugin for PlanetLodPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (init_planet_lod_system, update_planet_lod_mesh_system)
                .run_if(in_state(AppState::InGame)),
        );
    }
}

#[derive(Component)]
pub struct PlanetLod {
    pub planet_index: usize,
    pub is_moon: bool,
    pub moon_name: &'static str,
    pub height_scale: f32,
    pub texture_handle: Handle<Image>,
    pub is_initialized: bool,
    pub blend: f32,
    pub target_blend: f32,
    pub base_positions: Vec<[f32; 3]>,
    pub base_normals: Vec<[f32; 3]>,
    pub height_displacements: Vec<f32>,
    pub lod_normals: Vec<[f32; 3]>,
    pub sectors: u32,
    pub stacks: u32,
}

impl PlanetLod {
    pub fn new(
        planet_index: usize,
        is_moon: bool,
        moon_name: &'static str,
        height_scale: f32,
        texture_handle: Handle<Image>,
        sectors: u32,
        stacks: u32,
    ) -> Self {
        Self {
            planet_index,
            is_moon,
            moon_name,
            height_scale,
            texture_handle,
            is_initialized: false,
            blend: 0.0,
            target_blend: 0.0,
            base_positions: Vec::new(),
            base_normals: Vec::new(),
            height_displacements: Vec::new(),
            lod_normals: Vec::new(),
            sectors,
            stacks,
        }
    }
}

pub fn sample_image_luminance(image: &Image, u: f32, v: f32) -> f32 {
    let width = image.texture_descriptor.size.width as usize;
    let height = image.texture_descriptor.size.height as usize;
    let Some(data) = &image.data else {
        return 0.5;
    };
    if width == 0 || height == 0 || data.is_empty() {
        return 0.5;
    }

    let u_clamped = u.fract().abs();
    let v_clamped = v.clamp(0.0, 1.0);

    let x = ((u_clamped * width as f32) as usize).min(width - 1);
    let y = ((v_clamped * height as f32) as usize).min(height - 1);

    let bytes_per_pixel = match image.texture_descriptor.format {
        TextureFormat::Rgba8Unorm
        | TextureFormat::Rgba8UnormSrgb
        | TextureFormat::Bgra8Unorm
        | TextureFormat::Bgra8UnormSrgb => 4,
        TextureFormat::R8Unorm => 1,
        _ => 4,
    };

    let idx = (y * width + x) * bytes_per_pixel;
    if idx + bytes_per_pixel > data.len() {
        return 0.5;
    }

    let (r, g, b) = match bytes_per_pixel {
        4 => {
            if matches!(
                image.texture_descriptor.format,
                TextureFormat::Bgra8Unorm | TextureFormat::Bgra8UnormSrgb
            ) {
                (data[idx + 2], data[idx + 1], data[idx])
            } else {
                (data[idx], data[idx + 1], data[idx + 2])
            }
        }
        1 => {
            let val = data[idx];
            (val, val, val)
        }
        _ => (128, 128, 128),
    };

    (0.299 * (r as f32) + 0.587 * (g as f32) + 0.114 * (b as f32)) / 255.0
}

pub fn init_planet_lod_system(
    images: Res<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut lod_query: Query<(&mut PlanetLod, &Mesh3d)>,
) {
    for (mut lod, mesh_handle) in &mut lod_query {
        if lod.is_initialized {
            continue;
        }

        let Some(image) = images.get(&lod.texture_handle) else {
            continue;
        };

        let Some(mesh) = meshes.get_mut(mesh_handle) else {
            continue;
        };

        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).cloned()
        else {
            continue;
        };

        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL).cloned()
        else {
            continue;
        };

        let Some(VertexAttributeValues::Float32x2(uvs)) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_0).cloned()
        else {
            continue;
        };

        let num_verts = positions.len();
        let mut base_positions = Vec::with_capacity(num_verts);
        let mut base_normals = Vec::with_capacity(num_verts);
        let mut height_displacements = Vec::with_capacity(num_verts);

        for i in 0..num_verts {
            let pos = positions[i];
            let norm = normals[i];
            let uv = uvs[i];

            base_positions.push(pos);
            base_normals.push(norm);

            let lum = sample_image_luminance(image, uv[0], uv[1]);
            let p_dir = Vec3::from_array(norm).normalize_or_zero();

            // Multi-octave procedural terrain synthesis over texture heightmap base:
            // 1. Macro geography: smooth continent/basin/crater elevation
            let macro_terrain = (lum - 0.5) * 0.5;

            // 2. Secondary octave: smooth mountain ridges & crater walls
            let ridge_f = (p_dir.x * 24.0).sin() * (p_dir.y * 24.0).cos() * (p_dir.z * 24.0).sin();
            let ridge_height = ridge_f.abs() * 0.08;

            // 3. High-frequency octave: gentle terrain slope variation
            let micro_f = (p_dir.x * 50.0 + p_dir.y * 40.0).sin() * (p_dir.z * 60.0).cos();
            let micro_height = micro_f * 0.03;

            // 4. Ultra-fine octave: subtle surface micro-texture
            let fine_f = (p_dir.x * 120.0 + p_dir.z * 100.0).cos();
            let fine_height = fine_f * 0.01;

            let total_height = macro_terrain + (ridge_height + micro_height + fine_height) * (lum + 0.2);
            let disp = total_height * lod.height_scale;
            height_displacements.push(disp);
        }

        // Compute displaced normals for smooth 3D terrain shading
        let mut lod_normals = base_normals.clone();
        let sectors = lod.sectors as usize;
        let stacks = lod.stacks as usize;

        if sectors > 0 && stacks > 0 && (sectors + 1) * (stacks + 1) == num_verts {
            let stride = sectors + 1;

            let get_displaced_pos = |s: usize, t: usize| -> Vec3 {
                let idx = t * stride + s;
                let base_p = Vec3::from_array(base_positions[idx]);
                let norm_p = Vec3::from_array(base_normals[idx]).normalize_or_zero();
                let disp = height_displacements[idx];
                base_p + norm_p * disp
            };

            for t in 0..=stacks {
                for s in 0..=sectors {
                    let idx = t * stride + s;

                    let s_prev = if s == 0 { sectors - 1 } else { s - 1 };
                    let s_next = if s == sectors { 1 } else { s + 1 };
                    let t_prev = if t == 0 { 0 } else { t - 1 };
                    let t_next = if t == stacks { stacks } else { t + 1 };

                    let p_right = get_displaced_pos(s_next, t);
                    let p_left = get_displaced_pos(s_prev, t);
                    let p_up = get_displaced_pos(s, t_prev);
                    let p_down = get_displaced_pos(s, t_next);

                    let tan_u = (p_right - p_left).normalize_or_zero();
                    let tan_v = (p_down - p_up).normalize_or_zero();

                    let calc_norm = tan_u.cross(tan_v).normalize_or_zero();
                    if calc_norm.length_squared() > 0.01 {
                        lod_normals[idx] = calc_norm.to_array();
                    }
                }
            }
        }

        lod.base_positions = base_positions;
        lod.base_normals = base_normals;
        lod.height_displacements = height_displacements;
        lod.lod_normals = lod_normals;
        lod.is_initialized = true;
    }
}

pub fn update_planet_lod_mesh_system(
    time: Res<Time>,
    autopilot: Res<AutoPilotState>,
    flight_state: Res<FlightState>,
    planet_query: Query<&Planet>,
    moon_query: Query<&Moon>,
    mut lod_query: Query<(&mut PlanetLod, &Mesh3d)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let dt = time.delta_secs();

    for (mut lod, mesh_handle) in &mut lod_query {
        if !lod.is_initialized {
            continue;
        }

        // Quick exit if blend is already 0.0 and target blend was 0.0, and ship is far away
        if (lod.blend - lod.target_blend).abs() <= 0.001 && lod.target_blend == 0.0 {
            let is_destination = if autopilot.active {
                if lod.is_moon {
                    autopilot.destination_name == lod.moon_name
                } else {
                    planet_query.iter().find(|p| p.index == lod.planet_index).map_or(false, |p| p.name == autopilot.destination_name)
                }
            } else {
                false
            };

            if !is_destination {
                let is_near = if lod.is_moon {
                    moon_query.iter().find(|m| m.name == lod.moon_name).map_or(false, |m| flight_state.world_pos.distance(m.world_pos) < m.radius * 6.0)
                } else {
                    planet_query.iter().find(|p| p.index == lod.planet_index).map_or(false, |p| flight_state.world_pos.distance(p.world_pos) < p.radius * 6.0)
                };
                if !is_near {
                    continue;
                }
            }
        }

        let is_destination = if autopilot.active {
            if lod.is_moon {
                autopilot.destination_name == lod.moon_name
            } else {
                planet_query.iter().find(|p| p.index == lod.planet_index).map_or(false, |p| p.name == autopilot.destination_name)
            }
        } else {
            false
        };

        // Check proximity if ship is near a planet or moon surface
        let is_near = if lod.is_moon {
            moon_query.iter().find(|m| m.name == lod.moon_name).map_or(false, |m| flight_state.world_pos.distance(m.world_pos) < m.radius * 6.0)
        } else {
            planet_query.iter().find(|p| p.index == lod.planet_index).map_or(false, |p| flight_state.world_pos.distance(p.world_pos) < p.radius * 6.0)
        };

        let target_blend = if is_destination || is_near { 1.0 } else { 0.0 };
        lod.target_blend = target_blend;

        // Smooth continuous transition with S-curve easing
        let blend_speed = 0.8;
        if (lod.blend - lod.target_blend).abs() > 0.001 {
            if lod.blend < lod.target_blend {
                lod.blend = (lod.blend + blend_speed * dt).min(lod.target_blend);
            } else {
                lod.blend = (lod.blend - blend_speed * dt).max(lod.target_blend);
            }

            // Smoothstep S-curve easing (smooth acceleration and deceleration)
            let raw_blend = lod.blend.clamp(0.0, 1.0);
            let s_blend = raw_blend * raw_blend * (3.0 - 2.0 * raw_blend);

            let Some(mut mesh) = meshes.get_mut(mesh_handle) else {
                continue;
            };

            if let Some(VertexAttributeValues::Float32x3(positions)) =
                mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
            {
                for (i, pos) in positions.iter_mut().enumerate() {
                    let base_p = Vec3::from_array(lod.base_positions[i]);
                    let norm_p = Vec3::from_array(lod.base_normals[i]).normalize_or_zero();
                    let disp = lod.height_displacements[i];

                    let curr_p = base_p + norm_p * (s_blend * disp);
                    *pos = curr_p.to_array();
                }
            }

            if let Some(VertexAttributeValues::Float32x3(normals)) =
                mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL)
            {
                for (i, norm) in normals.iter_mut().enumerate() {
                    let base_n = Vec3::from_array(lod.base_normals[i]);
                    let lod_n = Vec3::from_array(lod.lod_normals[i]);

                    let blended_n = (base_n * (1.0 - s_blend) + lod_n * s_blend).normalize_or_zero();
                    *norm = blended_n.to_array();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planet_lod_blend_transition_math() {
        let base_p = Vec3::new(100.0, 0.0, 0.0);
        let norm_p = Vec3::new(1.0, 0.0, 0.0);
        let disp = 15.0; // Extruded peak height

        // 1. Out of orbit (blend = 0.0): Position remains standard sphere base position
        let blend_0 = 0.0;
        let pos_0 = base_p + norm_p * (blend_0 * disp);
        assert_eq!(pos_0, Vec3::new(100.0, 0.0, 0.0));

        // 2. Full orbit mode (blend = 1.0): Position extrudes peak height fully
        let blend_1 = 1.0;
        let pos_1 = base_p + norm_p * (blend_1 * disp);
        assert_eq!(pos_1, Vec3::new(115.0, 0.0, 0.0));

        // 3. Mid-transition (blend = 0.5): Position deforms smoothly halfway
        let blend_mid = 0.5;
        let pos_mid = base_p + norm_p * (blend_mid * disp);
        assert_eq!(pos_mid, Vec3::new(107.5, 0.0, 0.0));
    }
}

