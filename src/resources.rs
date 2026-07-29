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
    pub angular_velocity: Vec3, // x: yaw, y: pitch, z: roll
    #[allow(dead_code)]
    pub yaw: f32,          // Pilot look yaw
    #[allow(dead_code)]
    pub pitch: f32,        // Pilot look pitch
    #[allow(dead_code)]
    pub roll: f32,         // Pilot look roll
    #[allow(dead_code)]
    pub target_yaw: f32,   // Target look yaw
    #[allow(dead_code)]
    pub target_pitch: f32, // Target look pitch
    #[allow(dead_code)]
    pub target_roll: f32,  // Target look roll
    pub boost_mode: bool,  // Boost mode active state (toggled via Space)
    pub rapid_decel: bool, // Rapid deceleration state after exiting boost
    pub orbit_roll: f32,   // Roll angle maintained during orbit mode
}

#[derive(Resource, Default)]
pub struct AutoPilotState {
    pub active: bool,
    pub destination_index: Option<usize>,
    pub destination_name: &'static str,
    pub arrived: bool,
    pub engine_stopped: bool,
    pub prev_destination_pos: Option<Vec3>,
    pub current_waypoint: Option<Vec3>,     // Waypoint for path-finding avoidance
    pub positioning_timer: f32,             // Delay timer when positioning into orbit
    pub positioning_in_progress: bool,      // Positioning transition flag
    pub leaving_orbit_timer: f32,           // Delay timer when leaving orbit
    pub leaving_orbit_in_progress: bool,    // Leaving orbit transition flag
    pub orbit_speed_multiplier: f32,        // Controlled by W/S in orbit mode
    pub entering_orbit_timer: f32,          // Timer for center "Entering Orbit Mode" popup label
}


