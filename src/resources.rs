use bevy::prelude::*;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Loading,
    InGame,
}

#[derive(Resource, Default)]
pub struct LoadingAssets {
    pub handles: Vec<UntypedHandle>,
}

#[derive(Resource, Default)]
pub struct FlightState {
    pub world_pos: Vec3,       // Real physical position in solar system space
    pub previous_pos: Vec3,    // Previous frame world position for swept collision
    pub velocity: Vec3,
    pub angular_velocity: Vec2,
    #[allow(dead_code)]
    pub yaw: f32,          // Pilot look yaw
    #[allow(dead_code)]
    pub pitch: f32,        // Pilot look pitch
    #[allow(dead_code)]
    pub target_yaw: f32,   // Target look yaw
    #[allow(dead_code)]
    pub target_pitch: f32, // Target look pitch
    pub boost_mode: bool,  // Boost mode active state (toggled via Space)
    pub rapid_decel: bool, // Rapid deceleration state after exiting boost
}

#[derive(Resource, Default)]
pub struct AutoPilotState {
    pub active: bool,
    pub target_index: Option<usize>,
    pub target_name: &'static str,
    pub arrived: bool,
    pub engine_stopped: bool,
}

