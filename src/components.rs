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

#[derive(Component)]
pub struct AutopilotMenuContainer;

#[derive(Component)]
pub struct AutopilotMenuItemButton {
    pub destination_key: usize,
    pub destination_name: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CelestialDestinationType {
    Sun,
    Planet(usize),
    Moon(&'static str),
}

pub struct AutopilotDestination {
    pub key_num: usize,
    pub name: &'static str,
    pub dest_type: CelestialDestinationType,
}

pub const AUTOPILOT_DESTINATIONS: &[AutopilotDestination] = &[
    AutopilotDestination { key_num: 0, name: "Sun", dest_type: CelestialDestinationType::Sun },
    AutopilotDestination { key_num: 1, name: "Mercury", dest_type: CelestialDestinationType::Planet(1) },
    AutopilotDestination { key_num: 2, name: "Venus", dest_type: CelestialDestinationType::Planet(2) },
    AutopilotDestination { key_num: 3, name: "Earth", dest_type: CelestialDestinationType::Planet(3) },
    AutopilotDestination { key_num: 4, name: "Moon", dest_type: CelestialDestinationType::Moon("Moon") },
    AutopilotDestination { key_num: 5, name: "Mars", dest_type: CelestialDestinationType::Planet(4) },
    AutopilotDestination { key_num: 6, name: "Ceres", dest_type: CelestialDestinationType::Planet(10) },
    AutopilotDestination { key_num: 7, name: "Jupiter", dest_type: CelestialDestinationType::Planet(5) },
    AutopilotDestination { key_num: 8, name: "Io", dest_type: CelestialDestinationType::Moon("Io") },
    AutopilotDestination { key_num: 9, name: "Europa", dest_type: CelestialDestinationType::Moon("Europa") },
    AutopilotDestination { key_num: 10, name: "Saturn", dest_type: CelestialDestinationType::Planet(6) },
    AutopilotDestination { key_num: 11, name: "Uranus", dest_type: CelestialDestinationType::Planet(7) },
    AutopilotDestination { key_num: 12, name: "Neptune", dest_type: CelestialDestinationType::Planet(8) },
    AutopilotDestination { key_num: 13, name: "Pluto", dest_type: CelestialDestinationType::Planet(9) },
    AutopilotDestination { key_num: 14, name: "Charon", dest_type: CelestialDestinationType::Moon("Charon") },
    AutopilotDestination { key_num: 15, name: "Haumea", dest_type: CelestialDestinationType::Planet(11) },
    AutopilotDestination { key_num: 16, name: "Makemake", dest_type: CelestialDestinationType::Planet(12) },
    AutopilotDestination { key_num: 17, name: "Eris", dest_type: CelestialDestinationType::Planet(13) },
];

pub fn get_destination_by_key(key: usize) -> Option<&'static AutopilotDestination> {
    AUTOPILOT_DESTINATIONS.iter().find(|d| d.key_num == key)
}

#[derive(Component)]
pub struct CelestialLabel {
    pub name: &'static str,
    pub key_prefix: String,
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




