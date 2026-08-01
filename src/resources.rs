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
    AutopilotArrived,
}

#[derive(Resource, Default)]
pub struct FlightState {
    pub world_pos: Vec3,       // Real physical position in solar system space
    pub previous_pos: Vec3,    // Previous frame world position for swept collision
    pub velocity: Vec3,
    pub angular_velocity: Vec3, // x: yaw, y: pitch, z: roll
    pub boost_mode: bool,  // Boost mode active state (toggled via Space)
    pub rapid_decel: bool, // Rapid deceleration state after exiting boost
}

#[derive(Resource, Default)]
pub struct AutoPilotState {
    pub active: bool,
    pub arrived: bool, // True when reached arrival boundary and following planet until undock
    pub destination_index: Option<usize>,
    pub destination_name: &'static str,
    pub prev_destination_pos: Option<Vec3>,
    pub current_waypoint: Option<Vec3>, // Waypoint for path-finding avoidance
}

impl AutoPilotState {
    pub fn mode(&self) -> FlightControlMode {
        if self.active {
            if self.arrived {
                FlightControlMode::AutopilotArrived
            } else {
                FlightControlMode::AutopilotTransit
            }
        } else {
            FlightControlMode::Manual
        }
    }

    pub fn is_engaged(&self) -> bool {
        self.active
    }

    pub fn reset_all(&mut self) {
        self.active = false;
        self.arrived = false;
        self.destination_index = None;
        self.destination_name = "";
        self.current_waypoint = None;
        self.prev_destination_pos = None;
    }
}



