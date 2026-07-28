use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

use crate::components::{
    AutoPilotHudText, CockpitButton, CockpitButtonType, Planet, RadarSweepNeedle, Ship,
};
use crate::resources::{AutoPilotState, FlightState};

pub fn exit_on_esc(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        app_exit.write(AppExit::Success);
    }
}

pub fn animate_cockpit_screens_system(
    time: Res<Time>,
    mut needle_query: Query<&mut Transform, With<RadarSweepNeedle>>,
) {
    let dt = time.delta_secs();

    // Smooth continuous radar sweep needle rotation (no flashing/strobing)
    for mut transform in &mut needle_query {
        transform.rotate_local_z(-1.2 * dt);
    }
}

pub fn animate_cockpit_buttons_system(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    autopilot: Res<AutoPilotState>,
    button_query: Query<(&CockpitButton, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dt = time.delta_secs();

    let is_thrusting = keyboard.pressed(KeyCode::KeyW)
        || keyboard.pressed(KeyCode::KeyS)
        || keyboard.pressed(KeyCode::Space);
    let is_boosting = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    let is_steering = keyboard.pressed(KeyCode::KeyQ)
        || keyboard.pressed(KeyCode::KeyE)
        || keyboard.pressed(KeyCode::KeyA)
        || keyboard.pressed(KeyCode::KeyD);

    for (btn, mat_handle) in &button_query {
        if let Some(mut mat) = materials.get_mut(mat_handle) {
            let active = match btn.button_type {
                CockpitButtonType::Thruster => is_thrusting,
                CockpitButtonType::Warp => is_boosting,
                CockpitButtonType::AutoNav => is_steering || autopilot.active,
                CockpitButtonType::Shields => true,
                CockpitButtonType::Alert => is_boosting,
            };

            let target = if active {
                btn.active_emissive
            } else {
                btn.base_emissive
            };

            let lerp_speed = 2.0;
            mat.emissive = LinearRgba::new(
                mat.emissive.red + (target.red - mat.emissive.red) * (lerp_speed * dt).min(1.0),
                mat.emissive.green + (target.green - mat.emissive.green) * (lerp_speed * dt).min(1.0),
                mat.emissive.blue + (target.blue - mat.emissive.blue) * (lerp_speed * dt).min(1.0),
                1.0,
            );
        }
    }
}

pub fn update_hud_system(
    autopilot: Res<AutoPilotState>,
    flight_state: Res<FlightState>,
    ship_query: Query<&Transform, With<Ship>>,
    planet_query: Query<&Planet>,
    mut text_query: Query<&mut Text, With<AutoPilotHudText>>,
) {
    let Ok(ship_transform) = ship_query.single() else { return; };
    let speed = flight_state.velocity.length();

    for mut text in &mut text_query {
        if autopilot.active {
            if let Some(target_idx) = autopilot.target_index {
                let mut dist_str = String::from("CALCULATING...");
                for planet in &planet_query {
                    if planet.index == target_idx {
                        let dist = ship_transform.translation.distance(planet.world_pos);
                        dist_str = format!("{:.0} km", dist * 10.0);
                        break;
                    }
                }

                let status_label = if autopilot.arrived {
                    "PARKING ORBIT REACHED"
                } else {
                    "EN ROUTE"
                };

                **text = format!(
                    "AUTOPILOT: [{}] TARGET: {} | DISTANCE: {} | SPEED: {:.0} km/s | STATUS: {}",
                    target_idx,
                    autopilot.target_name.to_uppercase(),
                    dist_str,
                    speed * 5.0,
                    status_label
                );
            }
        } else {
            **text = format!(
                "FLIGHT STATUS: MANUAL CONTROL | SPEED: {:.0} km/s | PRESS [1-8] TO ENGAGE AUTOPILOT",
                speed * 5.0
            );
        }
    }
}
