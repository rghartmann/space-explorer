use bevy::prelude::*;

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
#[allow(dead_code)]
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
    pub target_world_pos: Vec3,
    pub planet_radius: f32,
}

#[derive(Component)]
pub struct RadarSweepNeedle;

#[derive(Component)]
pub struct AutoPilotHudText;

#[derive(Component)]
pub struct StopEngineButton;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CelestialTargetType {
    Sun,
    Planet(usize),
    Moon(&'static str),
}

#[derive(Component)]
pub struct CelestialLabel {
    pub name: &'static str,
    pub key_prefix: &'static str,
    pub target_type: CelestialTargetType,
}

#[derive(Component, PartialEq, Eq, Clone, Copy)]
pub enum CockpitButtonType {
    Thruster,
    Warp,
    Shields,
    AutoNav,
    Alert,
    OrbitStop,
}

#[derive(Component)]
pub struct CockpitButton {
    pub button_type: CockpitButtonType,
    pub base_emissive: LinearRgba,
    pub active_emissive: LinearRgba,
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

