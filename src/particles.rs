use bevy::prelude::*;

use crate::components::{EmitterType, Ship, ThrusterEmitter, ThrusterLight, ThrusterParticle};
use crate::resources::{AutoPilotState, FlightState};

struct PseudoRng(u32);

impl PseudoRng {
    fn new(seed: u32) -> Self {
        Self(if seed == 0 { 123456789 } else { seed })
    }

    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    fn gen_range(&mut self, range: std::ops::Range<f32>) -> f32 {
        let norm = (self.next_u32() as f64 / 4294967295.0) as f32;
        range.start + norm * (range.end - range.start)
    }

    fn gen_bool(&mut self, p: f32) -> bool {
        self.gen_range(0.0..1.0) < p
    }

}

#[derive(Resource)]
pub struct ParticleAssets {
    pub sphere_mesh: Handle<Mesh>,
    pub ring_mesh: Handle<Mesh>,
    pub normal_mat: Handle<StandardMaterial>,
    pub boost_mat: Handle<StandardMaterial>,
    pub core_mat: Handle<StandardMaterial>,
    pub boost_ring_mat: Handle<StandardMaterial>,
}


pub fn setup_particle_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let sphere_mesh = meshes.add(Sphere::new(0.3).mesh());
    let ring_mesh = meshes.add(Torus::new(0.05, 0.35).mesh());

    // Cyan/Blue plasma material for standard thrusters
    let normal_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.85, 1.0, 0.85),
        emissive: LinearRgba::new(0.5, 2.5, 5.0, 1.0),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        ..default()
    });

    // Intense White-Core material for inner nozzle flame
    let core_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, 0.95),
        emissive: LinearRgba::new(8.0, 8.0, 8.0, 1.0),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        ..default()
    });

    // Magenta/Violet warp boost material for FTL 1.5x c mode
    let boost_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.25, 0.95, 0.9),
        emissive: LinearRgba::new(5.0, 0.8, 4.5, 1.0),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        ..default()
    });

    // FTL Shockwave ring material
    let boost_ring_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.85, 0.4, 1.0, 0.75),
        emissive: LinearRgba::new(4.0, 1.2, 6.0, 1.0),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        ..default()
    });

    commands.insert_resource(ParticleAssets {
        sphere_mesh,
        ring_mesh,
        normal_mat,
        boost_mat,
        core_mat,
        boost_ring_mat,
    });
}

pub fn spawn_thruster_emitters(
    commands: &mut Commands,
    ship_entity: Entity,
) {
    // Twin rear engine nozzles + central boost core offset precisely aligned with 0.14 scaled spaceship GLTF exhaust ports
    // Nose points -Z, back of ship is +Z
    let left_offset = Vec3::new(-0.144, 0.025, 0.75);
    let right_offset = Vec3::new(0.144, 0.025, 0.75);
    let center_offset = Vec3::new(0.0, 0.030, 0.73);

    // Left Thruster Emitter & Light
    let left_emitter = commands
        .spawn((
            ThrusterEmitter {
                emitter_type: EmitterType::LeftEngine,
            },
            Transform::from_translation(left_offset),
            Visibility::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                ThrusterLight {
                    emitter_type: EmitterType::LeftEngine,
                },
                PointLight {
                    intensity: 0.0,
                    color: Color::srgb(0.2, 0.8, 1.0),
                    range: 6.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::IDENTITY,
            ));
        })
        .id();

    // Right Thruster Emitter & Light
    let right_emitter = commands
        .spawn((
            ThrusterEmitter {
                emitter_type: EmitterType::RightEngine,
            },
            Transform::from_translation(right_offset),
            Visibility::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                ThrusterLight {
                    emitter_type: EmitterType::RightEngine,
                },
                PointLight {
                    intensity: 0.0,
                    color: Color::srgb(0.2, 0.8, 1.0),
                    range: 6.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::IDENTITY,
            ));
        })
        .id();

    // Center FTL Boost Emitter & Light
    let center_emitter = commands
        .spawn((
            ThrusterEmitter {
                emitter_type: EmitterType::CenterBoost,
            },
            Transform::from_translation(center_offset),
            Visibility::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                ThrusterLight {
                    emitter_type: EmitterType::CenterBoost,
                },
                PointLight {
                    intensity: 0.0,
                    color: Color::srgb(0.9, 0.3, 1.0),
                    range: 10.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::IDENTITY,
            ));
        })
        .id();

    commands.entity(ship_entity).add_child(left_emitter);
    commands.entity(ship_entity).add_child(right_emitter);
    commands.entity(ship_entity).add_child(center_emitter);
}


pub fn thruster_particle_system(
    mut commands: Commands,
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    flight_state: Res<FlightState>,
    autopilot: Res<AutoPilotState>,
    particle_assets: Option<Res<ParticleAssets>>,
    ship_query: Query<Entity, With<Ship>>,
    emitter_query: Query<(&ThrusterEmitter, &Transform)>,
    mut light_query: Query<(&ThrusterLight, &mut PointLight)>,
) {
    let Some(assets) = particle_assets else { return; };
    let Ok(ship_entity) = ship_query.single() else { return; };

    let dt = time.delta_secs();
    let is_boosting = flight_state.boost_mode;
    let is_accelerating = keyboard.pressed(KeyCode::KeyW) || (autopilot.active && !autopilot.arrived && !autopilot.engine_stopped);

    // Dynamic light intensities and colors for thruster nozzles
    for (light_tag, mut light) in &mut light_query {
        if is_boosting {
            light.color = Color::srgb(0.95, 0.3, 1.0);
            let target_intensity = if light_tag.emitter_type == EmitterType::CenterBoost { 18_000.0 } else { 12_000.0 };
            light.intensity = light.intensity.lerp(target_intensity, (15.0 * dt).min(1.0));
            light.range = 10.0;
        } else if is_accelerating {
            if light_tag.emitter_type == EmitterType::CenterBoost {
                light.intensity = light.intensity.lerp(0.0, (15.0 * dt).min(1.0));
            } else {
                light.color = Color::srgb(0.2, 0.8, 1.0);
                light.intensity = light.intensity.lerp(7_000.0, (15.0 * dt).min(1.0));
                light.range = 6.0;
            }
        } else {
            light.intensity = light.intensity.lerp(0.0, (10.0 * dt).min(1.0));
        }
    }

    if !is_boosting && !is_accelerating {
        return;
    }

    let seed = (time.elapsed_secs_f64() * 100000.0) as u32;
    let mut rng = PseudoRng::new(seed);

    for (emitter, emitter_transform) in &emitter_query {
        // Center emitter only fires in boost mode
        if emitter.emitter_type == EmitterType::CenterBoost && !is_boosting {
            continue;
        }

        let origin_pos = emitter_transform.translation;

        // Particles per frame per emitter nozzle
        let spawn_count = if is_boosting {
            if emitter.emitter_type == EmitterType::CenterBoost { 3 } else { 2 }
        } else {
            1
        };

        for _ in 0..spawn_count {
            let offset_jitter = Vec3::new(
                rng.gen_range(-0.012..0.012),
                rng.gen_range(-0.012..0.012),
                rng.gen_range(-0.01..0.01),
            );
            let spawn_pos = origin_pos + offset_jitter;

            let (mesh_handle, mat_handle, speed, lifetime, initial_scale, target_scale, is_ring) = if is_boosting {
                let is_core = rng.gen_bool(0.3);
                let is_ring = rng.gen_bool(0.15);

                if is_ring {
                    (
                        assets.ring_mesh.clone(),
                        assets.boost_ring_mat.clone(),
                        rng.gen_range(12.0..20.0),
                        rng.gen_range(0.2..0.4),
                        0.08,
                        0.45,
                        true,
                    )
                } else if is_core {
                    (
                        assets.sphere_mesh.clone(),
                        assets.core_mat.clone(),
                        rng.gen_range(14.0..26.0),
                        rng.gen_range(0.1..0.22),
                        0.14,
                        0.02,
                        false,
                    )
                } else {
                    (
                        assets.sphere_mesh.clone(),
                        assets.boost_mat.clone(),
                        rng.gen_range(10.0..22.0),
                        rng.gen_range(0.18..0.35),
                        0.18,
                        0.04,
                        false,
                    )
                }
            } else {
                let is_core = rng.gen_bool(0.35);
                if is_core {
                    (
                        assets.sphere_mesh.clone(),
                        assets.core_mat.clone(),
                        rng.gen_range(8.0..15.0),
                        rng.gen_range(0.08..0.18),
                        0.10,
                        0.02,
                        false,
                    )
                } else {
                    (
                        assets.sphere_mesh.clone(),
                        assets.normal_mat.clone(),
                        rng.gen_range(5.0..12.0),
                        rng.gen_range(0.12..0.25),
                        0.14,
                        0.03,
                        false,
                    )
                }
            };


            // Emission direction: backward relative to ship orientation (+Z local)
            let spread_x = rng.gen_range(-0.15..0.15);
            let spread_y = rng.gen_range(-0.15..0.15);
            let local_vel = Vec3::new(spread_x * speed, spread_y * speed, speed);

            let particle_entity = commands
                .spawn((
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(mat_handle),
                    Transform::from_translation(spawn_pos)
                        .with_scale(Vec3::splat(initial_scale)),
                    ThrusterParticle {
                        velocity: local_vel,
                        lifetime: 0.0,
                        max_lifetime: lifetime,
                        initial_scale,
                        target_scale,
                        is_boost: is_boosting,
                        is_ring,
                    },

                ))
                .id();

            commands.entity(ship_entity).add_child(particle_entity);
        }
    }
}

pub fn update_thruster_particles_system(
    mut commands: Commands,
    time: Res<Time>,
    mut particle_query: Query<(Entity, &mut Transform, &mut ThrusterParticle)>,
) {
    let dt = time.delta_secs();

    for (entity, mut transform, mut particle) in &mut particle_query {
        particle.lifetime += dt;
        if particle.lifetime >= particle.max_lifetime {
            commands.entity(entity).despawn();
            continue;
        }

        let progress = (particle.lifetime / particle.max_lifetime).clamp(0.0, 1.0);

        // Move particle backward in local space
        transform.translation += particle.velocity * dt;

        // Scale interpolation (rings expand, plasma shrinks)
        let current_scale = particle.initial_scale + (particle.target_scale - particle.initial_scale) * progress;

        if particle.is_ring {
            // FTL Shockwave rings expand horizontally/vertically while retaining thin profile
            transform.scale = Vec3::new(current_scale * 1.5, current_scale * 1.5, 0.3);
        } else {
            // Particles taper & stretch slightly along travel direction (+Z)
            let stretch_z = if particle.is_boost { 1.8 } else { 1.2 };
            transform.scale = Vec3::new(current_scale, current_scale, current_scale * stretch_z);
        }
    }
}
