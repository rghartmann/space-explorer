use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct FlightState {
    pub velocity: Vec3,
    pub angular_velocity: Vec2,
    pub yaw: f32,          // Pilot look yaw
    pub pitch: f32,        // Pilot look pitch
    pub target_yaw: f32,   // Target look yaw
    pub target_pitch: f32, // Target look pitch
}

#[derive(Resource, Default)]
pub struct AutoPilotState {
    pub active: bool,
    pub target_index: Option<usize>,
    pub target_name: &'static str,
    pub arrived: bool,
}
