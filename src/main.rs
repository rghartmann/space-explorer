mod audio;
mod components;
mod flight;
mod hud;
mod lod;
mod particles;
mod rendering;
mod resources;
mod scene;

use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy::window::WindowMode;

use audio::engine_sound_system;
use flight::{
    autopilot_flight_system, autopilot_input_system, autopilot_pathfinding_system, celestial_collision_system, hide_cursor_system,
    orbit_asteroids_system, orbit_moons_system, orbit_planets_system, pilot_freelook_system,
    ship_flight_system, stop_engine_input_system,
};
use hud::{exit_on_esc, update_celestial_labels_system, update_hud_system};
use lod::PlanetLodPlugin;
use particles::{setup_particle_assets, thruster_particle_system, update_thruster_particles_system};
use rendering::{
    animate_sun_surface_system, logarithmic_distance_render_system, update_directional_sunlight_system,
    update_planet_area_lights_system,
};
use resources::{AppState, AutoPilotState, FlightState, LoadingAssets};
use scene::{check_loading_status, setup_loading_screen, setup_scene};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.001, 0.001, 0.003)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Space Explorer - Solar System Exploration & Auto-Pilot".into(),
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PlanetLodPlugin)
        .init_state::<AppState>()
        .init_resource::<FlightState>()
        .init_resource::<AutoPilotState>()
        .init_resource::<LoadingAssets>()
        .add_systems(OnEnter(AppState::Loading), setup_loading_screen)
        .add_systems(
            Update,
            (
                check_loading_status,
                hide_cursor_system,
                exit_on_esc,
            )
                .run_if(in_state(AppState::Loading)),
        )
        .add_systems(OnEnter(AppState::InGame), (setup_scene, setup_particle_assets))
        .add_systems(
            Update,
            (
                (
                    (orbit_planets_system, orbit_moons_system, orbit_asteroids_system),
                    (autopilot_input_system, stop_engine_input_system),
                    autopilot_pathfinding_system,
                    autopilot_flight_system,
                    ship_flight_system,
                    celestial_collision_system,
                    pilot_freelook_system,
                )
                    .chain(),
                hide_cursor_system,
                exit_on_esc,
                engine_sound_system,
                thruster_particle_system,
                update_thruster_particles_system,
                update_hud_system,
                animate_sun_surface_system,
            )
                .run_if(in_state(AppState::InGame)),
        )

        .add_systems(
            PostUpdate,
            (
                logarithmic_distance_render_system.before(TransformSystems::Propagate),
                update_directional_sunlight_system.before(TransformSystems::Propagate),
                update_planet_area_lights_system.before(TransformSystems::Propagate),
                update_celestial_labels_system.after(TransformSystems::Propagate),
            )
                .run_if(in_state(AppState::InGame)),
        )
        .run();
}

