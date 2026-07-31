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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FlightControlMode {
    #[default]
    Manual,
    AutopilotTransit,
    OrbitPositioning,
    OrbitLocked,
}

#[derive(Resource, Default)]
pub struct FlightState {
    pub world_pos: Vec3,       // Real physical position in solar system space
    pub previous_pos: Vec3,    // Previous frame world position for swept collision
    pub velocity: Vec3,
    pub angular_velocity: Vec3, // x: yaw, y: pitch, z: roll
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
    pub orbit_yaw: f32,                     // Spherical orbit yaw angle (rad)
    pub orbit_pitch: f32,                   // Spherical orbit pitch angle (rad), clamped to [-1.54, 1.54]
    pub orbit_distance: f32,                // Current orbit radius distance from celestial body
    pub orbit_initialized: bool,            // Whether spherical angles have been initialized for current orbit
}

impl AutoPilotState {
    pub fn mode(&self) -> FlightControlMode {
        if self.arrived || self.engine_stopped {
            FlightControlMode::OrbitLocked
        } else if self.positioning_in_progress {
            FlightControlMode::OrbitPositioning
        } else if self.active {
            FlightControlMode::AutopilotTransit
        } else {
            FlightControlMode::Manual
        }
    }

    pub fn is_in_orbit(&self) -> bool {
        self.arrived || self.engine_stopped || self.positioning_in_progress
    }

    pub fn is_engaged(&self) -> bool {
        self.active || self.arrived || self.engine_stopped || self.positioning_in_progress || self.leaving_orbit_in_progress
    }

    pub fn reset_all(&mut self) {
        self.active = false;
        self.arrived = false;
        self.engine_stopped = false;
        self.positioning_in_progress = false;
        self.positioning_timer = 0.0;
        self.leaving_orbit_in_progress = false;
        self.leaving_orbit_timer = 0.0;
        self.current_waypoint = None;
        self.destination_index = None;
        self.prev_destination_pos = None;
        self.orbit_initialized = false;
        self.orbit_speed_multiplier = 1.0;
        self.orbit_distance = 0.0;
    }
}



