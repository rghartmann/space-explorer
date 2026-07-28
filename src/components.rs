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
    pub _radius: f32,
}

#[derive(Component)]
pub struct Planet {
    pub _name: &'static str,
    pub index: usize, // 1 to 8
    pub radius: f32,
    pub _orbit_radius: f32,
    pub _orbit_speed: f32,
    pub rotation_speed: f32,
    pub world_pos: Vec3,
}

#[derive(Component)]
pub struct Moon {
    pub _name: &'static str,
    pub _parent_index: usize,
    pub radius: f32,
    pub _orbit_radius: f32,
    pub _orbit_speed: f32,
    pub rotation_speed: f32,
    pub world_pos: Vec3,
}

#[derive(Component)]
pub struct Starfield {
    pub world_pos: Vec3,
}

#[derive(Component)]
pub struct RadarSweepNeedle;

#[derive(Component)]
pub struct AutoPilotHudText;

#[derive(Component, PartialEq, Eq, Clone, Copy)]
pub enum CockpitButtonType {
    Thruster,
    Warp,
    Shields,
    AutoNav,
    Alert,
}

#[derive(Component)]
pub struct CockpitButton {
    pub button_type: CockpitButtonType,
    pub base_emissive: LinearRgba,
    pub active_emissive: LinearRgba,
}
