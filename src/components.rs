use bevy::prelude::*;

pub const AU: f32 = 149_597_870.7; // 1 Astronomical Unit in kilometers

#[derive(Component)]
pub struct Ship;

#[derive(Component)]
pub struct EngineSound;

#[derive(Component)]
pub struct AmbientMusic;

#[derive(Component)]
pub struct PilotCamera;

#[derive(Component)]
pub struct Sun {
    pub radius: f32,
}

#[derive(Component)]
pub struct SunAnimation {
    pub frame_handles: Vec<Handle<Image>>,
    pub current_frame: usize,
    pub frame_timer: Timer,
    pub pulse_timer: f32,
}

#[derive(Component)]
pub struct Planet {
    pub name: &'static str,
    pub index: usize, // 1 to 8
    pub radius: f32,
    pub orbit_radius: f32,
    pub orbit_speed: f32,
    pub orbit_angle: f32,
    pub rotation_speed: f32,
    pub world_pos: Vec3,
}

#[derive(Component)]
pub struct Moon {
    pub name: &'static str,
    pub parent_index: usize,
    pub radius: f32,
    pub orbit_radius: f32,
    pub orbit_speed: f32,
    pub orbit_angle: f32,
    pub rotation_speed: f32,
    pub world_pos: Vec3,
}

#[derive(Component)]
pub struct Starfield {
    pub direction: Vec3,
    pub size_scale: f32,
}

#[derive(Component)]
pub struct SkyboxSphere;

#[derive(Component)]
pub struct PlanetAreaLight {
    pub destination_world_pos: Vec3,
    pub planet_radius: f32,
}

#[derive(Component)]
pub struct AutoPilotHudText;

#[derive(Component)]
pub struct AutopilotWarningBanner;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CelestialDestinationType {
    Sun,
    Planet(usize),
    Moon(&'static str),
}

#[derive(Component)]
pub struct CelestialLabel {
    pub name: &'static str,
    pub key_prefix: &'static str,
    pub destination_type: CelestialDestinationType,
}

#[derive(Component)]
pub struct Asteroid {
    pub radius: f32,
    pub rotation_axis: Vec3,
    pub rotation_speed: f32,
    pub world_pos: Vec3,
}

#[derive(Component)]
pub struct SpaceDust {
    pub world_pos: Vec3,
    pub size_scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitterType {
    LeftEngine,
    RightEngine,
    CenterBoost,
}

#[derive(Component)]
pub struct ThrusterEmitter {
    pub emitter_type: EmitterType,
}

#[derive(Component)]
pub struct ThrusterLight {
    pub emitter_type: EmitterType,
}

#[derive(Component)]
pub struct ThrusterParticle {
    pub velocity: Vec3,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub initial_scale: f32,
    pub target_scale: f32,
    pub is_boost: bool,
    pub is_ring: bool,
}

#[derive(Component)]
pub struct SunDirectionalLight;




