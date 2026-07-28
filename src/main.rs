mod audio;
mod cockpit;
mod components;
mod flight;
mod rendering;
mod resources;
mod scene;

use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy::window::WindowMode;

use audio::engine_sound_system;
use cockpit::{
    animate_cockpit_buttons_system, animate_cockpit_screens_system, exit_on_esc,
    update_celestial_labels_system, update_hud_system,
};
use flight::{
    autopilot_flight_system, autopilot_input_system, celestial_collision_system, orbit_asteroids_system,
    orbit_moons_system, orbit_planets_system, pilot_freelook_system, ship_flight_system,
};
use rendering::{logarithmic_distance_render_system, update_planet_area_lights_system};
use resources::{AutoPilotState, FlightState};
use scene::setup_scene;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.001, 0.001, 0.003)))
        .init_resource::<FlightState>()
        .init_resource::<AutoPilotState>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Space Explorer - Solar System Exploration & Auto-Pilot".into(),
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                (
                    autopilot_input_system,
                    autopilot_flight_system,
                    ship_flight_system,
                    celestial_collision_system,
                    pilot_freelook_system,
                )
                    .chain(),
                exit_on_esc,
                orbit_planets_system,
                orbit_moons_system,
                orbit_asteroids_system,
                engine_sound_system,
                animate_cockpit_screens_system,
                animate_cockpit_buttons_system,
                update_hud_system,
            ),
        )
        .add_systems(
            PostUpdate,
            (
                logarithmic_distance_render_system.before(TransformSystems::Propagate),
                update_planet_area_lights_system.before(TransformSystems::Propagate),
                update_celestial_labels_system,
            )
                .chain(),
        )
        .run();
}
