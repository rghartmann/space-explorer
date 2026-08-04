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
use bevy::window::WindowMode;

use crate::audio::AudioPlugin;
use crate::flight::FlightPlugin;
use crate::hud::HudPlugin;
use crate::lod::PlanetLodPlugin;
use crate::particles::ParticlePlugin;
use crate::rendering::RenderingPlugin;
use crate::resources::{AppState, AutoPilotState, FlightState, LoadingAssets};
use crate::scene::ScenePlugin;

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
        .init_state::<AppState>()
        .init_resource::<FlightState>()
        .init_resource::<AutoPilotState>()
        .init_resource::<LoadingAssets>()
        .add_plugins((
            ScenePlugin,
            FlightPlugin,
            RenderingPlugin,
            HudPlugin,
            AudioPlugin,
            ParticlePlugin,
            PlanetLodPlugin,
        ))
        .run();
}

