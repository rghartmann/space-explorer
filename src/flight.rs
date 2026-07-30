use bevy::input::mouse::MouseMotion;
use bevy::ecs::message::MessageReader;
use bevy::prelude::*;

use crate::components::{Asteroid, Moon, PilotCamera, Planet, Ship, Sun};
use crate::resources::{AppState, AutoPilotState, FlightState};

pub const SPEED_OF_LIGHT: f32 = 299_792.458; // Speed of light in km/s (1.0c)
pub const STANDARD_MAX_SPEED: f32 = 600_000.0; // 600,000 km/s (~2.0c impulse speed cap)
pub const MAX_SPEED_CAP: f32 = 15_000_000.0;  // 15,000,000 km/s (~50.0c FTL warp boost cap)

pub struct FlightPlugin;

impl Plugin for FlightPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
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
            )
                .run_if(in_state(AppState::InGame)),
        );
    }
}

pub fn compute_orbit_boundary(radius: f32) -> f32 {
    if radius <= 1000.0 {
        radius * 2.5 + 500.0
    } else if radius <= 10000.0 {
        radius * 2.2 + 1500.0
    } else if radius <= 100000.0 {
        radius * 2.0 + 12000.0
    } else {
        radius * 3.0 + 250_000.0
    }
}

pub fn rotation_looking_to(dir: Vec3) -> Quat {
    let dir = dir.normalize_or_zero();
    if dir == Vec3::ZERO {
        return Quat::IDENTITY;
    }
    Quat::from_rotation_arc(Vec3::NEG_Z, dir)
}

pub fn hide_cursor_system(
    mut cursor_query: Query<&mut bevy::window::CursorOptions, With<Window>>,
) {
    for mut cursor in &mut cursor_query {
        if cursor.visible || cursor.grab_mode != bevy::window::CursorGrabMode::Locked {
            cursor.visible = false;
            cursor.grab_mode = bevy::window::CursorGrabMode::Locked;
        }
    }
}

pub fn pilot_freelook_system(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_events: MessageReader<MouseMotion>,
    mut flight_state: ResMut<FlightState>,
    autopilot: Res<AutoPilotState>,
    mut camera_query: Query<&mut Transform, With<PilotCamera>>,
    mut ship_query: Query<&mut Transform, (With<Ship>, Without<PilotCamera>)>,
    planet_query: Query<&Planet>,
    moon_query: Query<&Moon>,
    sun_query: Query<&Sun>,
) {
    let mut mouse_delta = Vec2::ZERO;
    for event in mouse_events.read() {
        mouse_delta += event.delta;
    }

    let dt = time.delta_secs();

    let Ok(mut ship_transform) = ship_query.single_mut() else { return; };

    // When auto-pilot destination is active and in-transit, autopilot keeps target directly centered on camera
    if autopilot.active && !autopilot.arrived {
        let mut target_pos = Vec3::ZERO;
        let mut found = false;

        if let Some(waypoint) = autopilot.current_waypoint {
            target_pos = waypoint;
            found = true;
        } else if let Some(destination_idx) = autopilot.destination_index {
            if destination_idx == 0 {
                target_pos = sun_query.iter().next().map(|_| Vec3::ZERO).unwrap_or(Vec3::ZERO);
                found = true;
            } else if destination_idx == 100 {
                for moon in &moon_query {
                    if moon.name == autopilot.destination_name {
                        target_pos = moon.world_pos;
                        found = true;
                        break;
                    }
                }
            } else {
                for planet in &planet_query {
                    if planet.index == destination_idx {
                        target_pos = planet.world_pos;
                        found = true;
                        break;
                    }
                }
            }
        }

        if found {
            let to_target = (target_pos - flight_state.world_pos).normalize_or_zero();
            if to_target != Vec3::ZERO {
                let current_forward = ship_transform.forward().as_vec3();
                let rot_diff = Quat::from_rotation_arc(current_forward, to_target);
                let destination_rot = rot_diff * ship_transform.rotation;
                let rot_decay = 1.0 - (-12.0 * dt).exp();
                ship_transform.rotation = ship_transform.rotation.slerp(destination_rot, rot_decay);
                flight_state.angular_velocity = Vec3::ZERO;
            }
        }
    } else {
        // Steering & freelook input (Manual flight & Orbit Mode)
        let sensitivity = 0.0015;
        let key_speed = 1.2 * dt;

        let mut yaw_input = -mouse_delta.x * sensitivity;
        let mut pitch_input = -mouse_delta.y * sensitivity;
        let mut roll_input = 0.0;

        if keyboard.pressed(KeyCode::ArrowLeft) {
            yaw_input += key_speed;
        }
        if keyboard.pressed(KeyCode::ArrowRight) {
            yaw_input -= key_speed;
        }
        if keyboard.pressed(KeyCode::ArrowUp) {
            pitch_input += key_speed;
        }
        if keyboard.pressed(KeyCode::ArrowDown) {
            pitch_input -= key_speed;
        }

        if !autopilot.arrived && !autopilot.engine_stopped {
            if keyboard.pressed(KeyCode::KeyA) {
                yaw_input += key_speed;
            }
            if keyboard.pressed(KeyCode::KeyD) {
                yaw_input -= key_speed;
            }
            if keyboard.pressed(KeyCode::KeyQ) {
                roll_input += key_speed;
            }
            if keyboard.pressed(KeyCode::KeyE) {
                roll_input -= key_speed;
            }
        }

        if keyboard.pressed(KeyCode::KeyZ) {
            roll_input += key_speed;
        }
        if keyboard.pressed(KeyCode::KeyC) {
            roll_input -= key_speed;
        }

        let rot_decay = 1.0 - (-12.0 * dt).exp();
        flight_state.angular_velocity.x = flight_state.angular_velocity.x.lerp(yaw_input, rot_decay);
        flight_state.angular_velocity.y = flight_state.angular_velocity.y.lerp(pitch_input, rot_decay);
        flight_state.angular_velocity.z = flight_state.angular_velocity.z.lerp(roll_input, rot_decay);

        if !autopilot.arrived && !autopilot.engine_stopped {
            if flight_state.angular_velocity.x.abs() > 0.00001 {
                ship_transform.rotate_local_y(flight_state.angular_velocity.x);
            }
            if flight_state.angular_velocity.y.abs() > 0.00001 {
                ship_transform.rotate_local_x(flight_state.angular_velocity.y);
            }
            if flight_state.angular_velocity.z.abs() > 0.00001 {
                ship_transform.rotate_local_z(flight_state.angular_velocity.z);
            }
        }
    }

    // Dynamic 3rd person camera banking, sway & subtle warp screen shake
    let lean_yaw = -flight_state.angular_velocity.x * 0.15;
    let lean_pitch = -flight_state.angular_velocity.y * 0.15;
    let lean_roll = -flight_state.angular_velocity.x * 0.25 - flight_state.angular_velocity.z * 0.20;

    if let Some(mut cam_transform) = camera_query.iter_mut().next() {
        let base_pos = Vec3::new(0.0, 1.2, 4.0);
        let base_rot = Quat::from_rotation_x(-0.16);
        let dynamic_rot = Quat::from_euler(EulerRot::YXZ, lean_yaw, lean_pitch, lean_roll);
        let mut target_rot = base_rot * dynamic_rot;
        let mut target_pos = base_pos + Vec3::new(-flight_state.angular_velocity.x * 3.0, flight_state.angular_velocity.y * 2.0, 0.0);

        // Subtle camera screen shake when FTL warp mode is active
        if flight_state.boost_mode {
            let t = time.elapsed_secs() * 55.0;
            let shake_offset = Vec3::new(
                (t * 1.3).sin() * 0.035 + (t * 2.7).cos() * 0.020,
                (t * 1.7).cos() * 0.030 + (t * 3.1).sin() * 0.015,
                (t * 2.1).sin() * 0.025,
            );
            let shake_rot = Quat::from_euler(EulerRot::YXZ, (t * 1.9).sin() * 0.005, (t * 2.3).cos() * 0.005, (t * 1.5).cos() * 0.007);
            target_pos += shake_offset;
            target_rot = target_rot * shake_rot;
        }

        let cam_decay = 1.0 - (-8.0 * dt).exp();
        cam_transform.rotation = cam_transform.rotation.slerp(target_rot, cam_decay);
        cam_transform.translation = cam_transform.translation.lerp(target_pos, cam_decay);
    }
}

pub fn ship_flight_system(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut autopilot: ResMut<AutoPilotState>,
    mut flight_state: ResMut<FlightState>,
    mut ship_query: Query<&mut Transform, With<Ship>>,
) {
    let dt = time.delta_secs();
    let Ok(mut ship_transform) = ship_query.single_mut() else { return; };

    // Record previous world position for swept line-segment collision detection
    flight_state.previous_pos = flight_state.world_pos;

    // Space Key: Cancel Auto-Pilot & Restore Manual Controls if active, else Toggle FTL Boost Mode
    let is_autopilot_engaged = autopilot.active
        || autopilot.arrived
        || autopilot.engine_stopped
        || autopilot.positioning_in_progress
        || autopilot.leaving_orbit_in_progress;

    if keyboard.just_pressed(KeyCode::Space) {
        if is_autopilot_engaged {
            // Cancel auto-pilot immediately and restore manual controls
            autopilot.active = false;
            autopilot.arrived = false;
            autopilot.engine_stopped = false;
            autopilot.positioning_in_progress = false;
            autopilot.positioning_timer = 0.0;
            autopilot.leaving_orbit_in_progress = false;
            autopilot.leaving_orbit_timer = 0.0;
            autopilot.current_waypoint = None;
            autopilot.destination_index = None;
            autopilot.prev_destination_pos = None;
            autopilot.orbit_initialized = false;
            flight_state.boost_mode = false;
            flight_state.rapid_decel = false;
            flight_state.angular_velocity = Vec3::ZERO;
        } else if flight_state.boost_mode {
            // Pressing Space again while in boost mode decelerates quickly towards standard speed/stop
            flight_state.boost_mode = false;
            flight_state.rapid_decel = true;
        } else {
            // First press of Space enters FTL boost mode
            flight_state.boost_mode = true;
            flight_state.rapid_decel = false;
        }
    }

    // KeyX / Backspace: Emergency Full Retro-Stop (bring ship to 0 km/s)
    if keyboard.just_pressed(KeyCode::KeyX) || keyboard.just_pressed(KeyCode::Backspace) {
        flight_state.boost_mode = false;
        flight_state.rapid_decel = false;
        flight_state.velocity = Vec3::ZERO;
    }

    if flight_state.boost_mode {
        // Smooth FTL acceleration towards FTL warp speed cap (50.0c)
        let target_speed = MAX_SPEED_CAP;
        let accel_rate = 1.0 - (-5.0 * dt).exp();
        let forward = ship_transform.forward().as_vec3();
        let current_speed = flight_state.velocity.length();
        let new_speed = current_speed.lerp(target_speed, accel_rate).max(100_000.0);
        flight_state.velocity = forward * new_speed;
    } else if flight_state.rapid_decel {
        // Retro-Thruster Rapid Braking towards 0
        let decel_rate = 1.0 - (-8.0 * dt).exp();
        let current_speed = flight_state.velocity.length();
        let new_speed = current_speed.lerp(0.0, decel_rate);
        if new_speed <= 20.0 {
            flight_state.rapid_decel = false;
            flight_state.velocity = Vec3::ZERO;
        } else {
            let dir = flight_state.velocity.normalize_or_zero();
            flight_state.velocity = dir * new_speed;
        }
    } else if !autopilot.active && !autopilot.arrived && !autopilot.engine_stopped {
        // Manual Impulse Propulsion controls in Vacuum (Newtonian Mechanics: ZERO drag!)
        let accel_power = 120_000.0 * dt; // Responsive thruster acceleration (120,000 km/s^2)
        let forward = ship_transform.forward().as_vec3();

        // W Key: Forward Main Thrusters (add velocity in ship forward direction)
        if keyboard.pressed(KeyCode::KeyW) {
            flight_state.velocity += forward * accel_power;
        }

        // S Key: Reverse Retro-Thrusters (decelerate velocity towards 0, or apply reverse thrust)
        if keyboard.pressed(KeyCode::KeyS) {
            let current_speed = flight_state.velocity.length();
            if current_speed > 10.0 {
                let decel_amount = accel_power * 1.25;
                let new_speed = (current_speed - decel_amount).max(0.0);
                let vel_dir = flight_state.velocity / current_speed;
                flight_state.velocity = vel_dir * new_speed;
            } else {
                flight_state.velocity -= forward * (accel_power * 0.5);
            }
        }

        // Vacuum Inertia (Newton's First Law): Releasing controls maintains 100% constant speed!
        // No artificial drag/damping is applied.

        // Sub-light impulse speed cap (600,000 km/s / ~2.0c)
        if flight_state.velocity.length() > STANDARD_MAX_SPEED {
            flight_state.velocity = flight_state.velocity.normalize() * STANDARD_MAX_SPEED;
        }
    }

    let current_vel = flight_state.velocity;
    flight_state.world_pos += current_vel * dt;
    ship_transform.translation = Vec3::ZERO;
}

pub fn autopilot_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut autopilot: ResMut<AutoPilotState>,
    flight_state: Res<FlightState>,
    planet_query: Query<&Planet>,
    moon_query: Query<&Moon>,
    sun_query: Query<&Sun>,
) {
    let planet_keys = [
        (KeyCode::Digit0, 0, "Sun"),
        (KeyCode::Digit1, 1, "Mercury"),
        (KeyCode::Digit2, 2, "Venus"),
        (KeyCode::Digit3, 3, "Earth"),
        (KeyCode::Digit4, 4, "Mars"),
        (KeyCode::Digit5, 5, "Jupiter"),
        (KeyCode::Digit6, 6, "Saturn"),
        (KeyCode::Digit7, 7, "Uranus"),
        (KeyCode::Digit8, 8, "Neptune"),
        (KeyCode::Digit9, 9, "Pluto"),
        (KeyCode::KeyC, 10, "Ceres"),
        (KeyCode::KeyH, 11, "Haumea"),
        (KeyCode::KeyK, 12, "Makemake"),
        (KeyCode::KeyE, 13, "Eris"),
        (KeyCode::KeyM, 100, "Moon"),
    ];

    let is_in_orbit = autopilot.arrived || autopilot.engine_stopped;
    let shift_held = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    for (key, idx, name) in planet_keys {
        if keyboard.just_pressed(key) {
            if is_in_orbit && (key == KeyCode::KeyC || key == KeyCode::KeyE) && !shift_held {
                continue;
            }

            let mut target_pos = Vec3::ZERO;
            if idx == 100 {
                for moon in &moon_query {
                    if moon.name == name {
                        target_pos = moon.world_pos;
                        break;
                    }
                }
            } else if idx != 0 {
                for planet in &planet_query {
                    if planet.index == idx {
                        target_pos = planet.world_pos;
                        break;
                    }
                }
            }

            // Multi-body path-finding obstacle avoidance around Sun and all Planets along trajectory
            let start_pos = flight_state.world_pos;
            let to_dest = target_pos - start_pos;
            let dist = to_dest.length();
            let mut waypoint = None;

            if dist > 100.0 {
                let line_dir = to_dest / dist;
                let mut closest_obstacle_dist = f32::MAX;
                let mut chosen_waypoint = None;

                let sun_radius = sun_query.iter().next().map(|s| s.radius).unwrap_or(696340.0);

                // Check Sun at origin
                if idx != 0 {
                    let to_sun = -start_pos;
                    let proj = to_sun.dot(line_dir);
                    if proj > 1000.0 && proj < dist - 1000.0 {
                        let closest_pt = start_pos + line_dir * proj;
                        let clearance = closest_pt.length();
                        let min_clearance = (sun_radius * 2.8).max(250_000.0);
                        if clearance < min_clearance {
                            let perp = Vec3::Y.cross(line_dir).normalize_or_zero();
                            let bypass_dir = if perp != Vec3::ZERO { perp } else { Vec3::Y };
                            let wp = closest_pt + bypass_dir * (min_clearance * 1.5);
                            let d = start_pos.length();
                            if d < closest_obstacle_dist {
                                closest_obstacle_dist = d;
                                chosen_waypoint = Some(wp);
                            }
                        }
                    }
                }

                // Check Planets
                for planet in &planet_query {
                    if idx != planet.index {
                        let to_planet = planet.world_pos - start_pos;
                        let proj = to_planet.dot(line_dir);
                        if proj > 1000.0 && proj < dist - 1000.0 {
                            let closest_pt = start_pos + line_dir * proj;
                            let clearance = (closest_pt - planet.world_pos).length();
                            let min_clearance = (planet.radius * 2.5).max(10_000.0);
                            if clearance < min_clearance {
                                let perp = (closest_pt - planet.world_pos).normalize_or_zero();
                                let bypass_dir = if perp != Vec3::ZERO { perp } else { Vec3::Y };
                                let wp = planet.world_pos + bypass_dir * (min_clearance * 1.6);
                                let d = start_pos.distance(planet.world_pos);
                                if d < closest_obstacle_dist {
                                    closest_obstacle_dist = d;
                                    chosen_waypoint = Some(wp);
                                }
                            }
                        }
                    }
                }

                waypoint = chosen_waypoint;
            }

            autopilot.active = true;
            autopilot.destination_index = Some(idx);
            autopilot.destination_name = name;
            autopilot.arrived = false;
            autopilot.engine_stopped = false;
            autopilot.prev_destination_pos = None;
            autopilot.current_waypoint = waypoint;
            autopilot.positioning_in_progress = false;
            autopilot.positioning_timer = 0.0;
            autopilot.leaving_orbit_in_progress = false;
            autopilot.leaving_orbit_timer = 0.0;
            autopilot.orbit_speed_multiplier = 1.0;
            autopilot.orbit_initialized = false;
        }
    }
}

pub fn autopilot_pathfinding_system(
    mut autopilot: ResMut<AutoPilotState>,
    flight_state: Res<FlightState>,
    sun_query: Query<&Sun>,
    planet_query: Query<&Planet>,
    moon_query: Query<&Moon>,
) {
    if !autopilot.active || autopilot.arrived || autopilot.engine_stopped || autopilot.positioning_in_progress {
        return;
    }

    let Some(destination_idx) = autopilot.destination_index else { return; };

    let mut final_target_pos = Vec3::ZERO;
    let mut found = false;

    if destination_idx == 0 {
        final_target_pos = Vec3::ZERO;
        found = true;
    } else if destination_idx == 100 {
        for moon in &moon_query {
            if moon.name == autopilot.destination_name {
                final_target_pos = moon.world_pos;
                found = true;
                break;
            }
        }
    } else {
        for planet in &planet_query {
            if planet.index == destination_idx {
                final_target_pos = planet.world_pos;
                found = true;
                break;
            }
        }
    }

    if !found {
        return;
    }

    let start_pos = flight_state.world_pos;

    if let Some(wp) = autopilot.current_waypoint {
        let dist_to_wp = start_pos.distance(wp);
        if dist_to_wp < 15_000.0 {
            autopilot.current_waypoint = None;
        }
    }

    let current_target_pos = autopilot.current_waypoint.unwrap_or(final_target_pos);
    let to_target = current_target_pos - start_pos;
    let dist = to_target.length();

    if dist <= 1000.0 {
        return;
    }

    let line_dir = to_target / dist;
    let mut closest_obstacle_dist = f32::MAX;
    let mut chosen_waypoint = None;

    let sun_radius = sun_query.iter().next().map(|s| s.radius).unwrap_or(696340.0);

    if destination_idx != 0 {
        let to_sun = -start_pos;
        let proj = to_sun.dot(line_dir);
        if proj > 1000.0 && proj < dist - 1000.0 {
            let closest_pt = start_pos + line_dir * proj;
            let clearance = closest_pt.length();
            let min_clearance = (sun_radius * 2.8).max(250_000.0);
            if clearance < min_clearance {
                let perp = Vec3::Y.cross(line_dir).normalize_or_zero();
                let bypass_dir = if perp != Vec3::ZERO { perp } else { Vec3::Y };
                let wp = closest_pt + bypass_dir * (min_clearance * 1.5);
                let d = start_pos.length();
                if d < closest_obstacle_dist {
                    closest_obstacle_dist = d;
                    chosen_waypoint = Some(wp);
                }
            }
        }
    }

    for planet in &planet_query {
        if destination_idx != planet.index {
            let to_planet = planet.world_pos - start_pos;
            let proj = to_planet.dot(line_dir);
            if proj > 1000.0 && proj < dist - 1000.0 {
                let closest_pt = start_pos + line_dir * proj;
                let clearance = (closest_pt - planet.world_pos).length();
                let min_clearance = (planet.radius * 2.5).max(15_000.0);
                if clearance < min_clearance {
                    let perp = (closest_pt - planet.world_pos).normalize_or_zero();
                    let bypass_dir = if perp != Vec3::ZERO { perp } else { Vec3::Y };
                    let wp = planet.world_pos + bypass_dir * (min_clearance * 1.6);
                    let d = start_pos.distance(planet.world_pos);
                    if d < closest_obstacle_dist {
                        closest_obstacle_dist = d;
                        chosen_waypoint = Some(wp);
                    }
                }
            }
        }
    }

    if let Some(wp) = chosen_waypoint {
        autopilot.current_waypoint = Some(wp);
    }
}

pub fn stop_engine_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut autopilot: ResMut<AutoPilotState>,
    mut flight_state: ResMut<FlightState>,
    sun_query: Query<&Sun>,
    planet_query: Query<&Planet>,
    moon_query: Query<&Moon>,
) {
    let triggered = keyboard.just_pressed(KeyCode::KeyO);

    if triggered {
        if autopilot.engine_stopped || autopilot.arrived || autopilot.active || autopilot.positioning_in_progress {
            // Initiate graceful exit from orbit mode / cancel autopilot
            autopilot.leaving_orbit_in_progress = true;
            autopilot.leaving_orbit_timer = 1.2;
            autopilot.arrived = false;
            autopilot.engine_stopped = false;
            autopilot.active = false;
            autopilot.positioning_in_progress = false;
            autopilot.positioning_timer = 0.0;
            autopilot.current_waypoint = None;
            autopilot.orbit_initialized = false;
        } else {
            // Determine target destination body position and radius
            let mut dest_pos = Vec3::ZERO;
            let mut dest_radius = 32790.0;
            let mut dest_idx = 0;
            let mut dest_name = "Sun";
            let mut found = false;

            if let Some(idx) = autopilot.destination_index {
                if idx == 0 {
                    dest_pos = Vec3::ZERO;
                    if let Ok(sun) = sun_query.single() {
                        dest_radius = sun.radius;
                    }
                    found = true;
                } else if idx == 100 {
                    for moon in &moon_query {
                        if moon.name == autopilot.destination_name {
                            dest_pos = moon.world_pos;
                            dest_radius = moon.radius;
                            dest_idx = 100;
                            dest_name = moon.name;
                            found = true;
                            break;
                        }
                    }
                } else {
                    for planet in &planet_query {
                        if planet.index == idx {
                            dest_pos = planet.world_pos;
                            dest_radius = planet.radius;
                            dest_idx = planet.index;
                            dest_name = planet.name;
                            found = true;
                            break;
                        }
                    }
                }
            }

            if !found {
                // Find closest celestial body
                let mut min_dist = flight_state.world_pos.distance(Vec3::ZERO);
                dest_pos = Vec3::ZERO;
                if let Ok(sun) = sun_query.single() {
                    dest_radius = sun.radius;
                }
                dest_idx = 0;
                dest_name = "Sun";

                for planet in &planet_query {
                    let dist = flight_state.world_pos.distance(planet.world_pos);
                    if dist < min_dist {
                        min_dist = dist;
                        dest_pos = planet.world_pos;
                        dest_radius = planet.radius;
                        dest_idx = planet.index;
                        dest_name = planet.name;
                    }
                }

                for moon in &moon_query {
                    let dist = flight_state.world_pos.distance(moon.world_pos);
                    if dist < min_dist {
                        min_dist = dist;
                        dest_pos = moon.world_pos;
                        dest_radius = moon.radius;
                        dest_idx = 100;
                        dest_name = moon.name;
                    }
                }
            }

            let dist = flight_state.world_pos.distance(dest_pos);
            let orbit_entry_threshold = (compute_orbit_boundary(dest_radius) * 2.5).max(10_000.0);

            // Only enter orbit mode if ship is within proximity threshold of the celestial body
            if dist <= orbit_entry_threshold {
                flight_state.velocity = Vec3::ZERO;
                autopilot.active = true;
                autopilot.destination_index = Some(dest_idx);
                autopilot.destination_name = dest_name;
                autopilot.positioning_in_progress = true;
                autopilot.positioning_timer = 1.0;
                autopilot.entering_orbit_timer = 2.5;
                autopilot.arrived = false;
                autopilot.engine_stopped = false;
                autopilot.prev_destination_pos = None;
            }
        }
    }
}

pub fn autopilot_flight_system(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_events: MessageReader<MouseMotion>,
    mut autopilot: ResMut<AutoPilotState>,
    mut flight_state: ResMut<FlightState>,
    mut ship_query: Query<&mut Transform, With<Ship>>,
    planet_query: Query<&Planet>,
    moon_query: Query<&Moon>,
    sun_query: Query<&Sun>,
) {
    let dt = time.delta_secs();

    if autopilot.entering_orbit_timer > 0.0 {
        autopilot.entering_orbit_timer = (autopilot.entering_orbit_timer - dt).max(0.0);
    }

    // Handle leaving orbit transition delay
    if autopilot.leaving_orbit_in_progress {
        autopilot.leaving_orbit_timer -= dt;
        if autopilot.leaving_orbit_timer <= 0.0 {
            autopilot.leaving_orbit_in_progress = false;
            autopilot.leaving_orbit_timer = 0.0;
            autopilot.destination_index = None;
            autopilot.prev_destination_pos = None;
        } else {
            let Ok(mut ship_transform) = ship_query.single_mut() else { return; };
            let forward = ship_transform.forward().as_vec3();
            let speed = 4000.0 * (1.2 - autopilot.leaving_orbit_timer);
            flight_state.velocity = forward * speed;
            let current_vel = flight_state.velocity;
            flight_state.world_pos += current_vel * dt;
            ship_transform.translation = Vec3::ZERO;
            return;
        }
    }

    if !autopilot.active && !autopilot.arrived && !autopilot.engine_stopped && !autopilot.positioning_in_progress {
        return;
    }

    let mut destination_pos = Vec3::ZERO;
    let mut destination_radius = 100.0;
    let mut found = false;

    let Some(destination_idx) = autopilot.destination_index else { return; };
    let Ok(mut ship_transform) = ship_query.single_mut() else { return; };

    if destination_idx == 100 {
        for moon in &moon_query {
            if moon.name == autopilot.destination_name {
                destination_pos = moon.world_pos;
                destination_radius = moon.radius;
                found = true;
                break;
            }
        }
    } else if destination_idx == 0 {
        destination_pos = Vec3::ZERO;
        if let Ok(sun) = sun_query.single() {
            destination_radius = sun.radius;
        } else {
            destination_radius = 32790.0;
        }
        found = true;
    } else {
        for planet in &planet_query {
            if planet.index == destination_idx {
                destination_pos = planet.world_pos;
                destination_radius = planet.radius;
                found = true;
                break;
            }
        }
    }

    if !found {
        return;
    }

    // Path-finding waypoint guidance
    let effective_target_pos = if let Some(waypoint) = autopilot.current_waypoint {
        let dist_to_wp = flight_state.world_pos.distance(waypoint);
        if dist_to_wp < 15_000.0 {
            autopilot.current_waypoint = None;
            destination_pos
        } else {
            waypoint
        }
    } else {
        destination_pos
    };

    let to_target = effective_target_pos - flight_state.world_pos;
    let distance = to_target.length();
    let real_distance_to_dest = (destination_pos - flight_state.world_pos).length();

    let arrival_dist = compute_orbit_boundary(destination_radius);

    let prev_pos = autopilot.prev_destination_pos.unwrap_or(destination_pos);

    // Handle positioning transition delay when entering orbit
    if autopilot.positioning_in_progress {
        autopilot.positioning_timer -= dt;
        let look_dir = (destination_pos - flight_state.world_pos).normalize_or_zero();
        if look_dir != Vec3::ZERO {
            let current_forward = ship_transform.forward().as_vec3();
            let rot_diff = Quat::from_rotation_arc(current_forward, look_dir);
            let target_rot = rot_diff * ship_transform.rotation;
            let rot_decay = 1.0 - (-6.0 * dt).exp();
            ship_transform.rotation = ship_transform.rotation.slerp(target_rot, rot_decay);
        }
        let vel_decay = 1.0 - (-7.0 * dt).exp();
        flight_state.velocity = flight_state.velocity.lerp(Vec3::ZERO, vel_decay);
        let current_vel = flight_state.velocity;
        flight_state.world_pos += current_vel * dt;
        ship_transform.translation = Vec3::ZERO;

        // Keep ship safely outside arrival boundary during orbit insertion positioning
        let offset = flight_state.world_pos - destination_pos;
        let dist_from_center = offset.length();
        if dist_from_center < arrival_dist {
            let safe_dir = if dist_from_center > 0.001 { offset / dist_from_center } else { Vec3::Z };
            flight_state.world_pos = destination_pos + safe_dir * arrival_dist;
        }

        if autopilot.positioning_timer <= 0.0 {
            autopilot.positioning_in_progress = false;
            autopilot.positioning_timer = 0.0;
            autopilot.arrived = true;
            autopilot.engine_stopped = true;
        }
        return;
    }

    if real_distance_to_dest <= arrival_dist && !autopilot.arrived && !autopilot.engine_stopped {
        autopilot.positioning_in_progress = true;
        autopilot.positioning_timer = 1.5;
        autopilot.entering_orbit_timer = 2.5;
        return;
    }

    if autopilot.arrived || autopilot.engine_stopped {
        autopilot.arrived = true;
        autopilot.engine_stopped = true;

        // Orbit Mode Controls
        let mut mouse_delta = Vec2::ZERO;
        for event in mouse_events.read() {
            mouse_delta += event.delta;
        }

        let mouse_sens = 0.0015;

        // A/D Keys rotate horizontally around planet
        let mut horiz_key_input = 0.0;
        if keyboard.pressed(KeyCode::KeyA) {
            horiz_key_input -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            horiz_key_input += 1.0;
        }

        // W/S Keys accelerate/decelerate orbit speed
        if keyboard.pressed(KeyCode::KeyW) {
            let mult = if autopilot.orbit_speed_multiplier <= 0.0 { 1.0 } else { autopilot.orbit_speed_multiplier };
            autopilot.orbit_speed_multiplier = (mult + 1.5 * dt).min(5.0);
        } else if keyboard.pressed(KeyCode::KeyS) {
            let mult = if autopilot.orbit_speed_multiplier <= 0.0 { 1.0 } else { autopilot.orbit_speed_multiplier };
            autopilot.orbit_speed_multiplier = (mult - 2.0 * dt).max(0.0);
        }

        let current_mult = if autopilot.orbit_speed_multiplier <= 0.0 { 1.0 } else { autopilot.orbit_speed_multiplier };
        let orbit_speed = 0.45 * current_mult; // rad/s

        // Q/E Keys adjust radial distance closer/farther
        let mut radial_input = 0.0;
        if keyboard.pressed(KeyCode::KeyQ) {
            radial_input -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyE) {
            radial_input += 1.0;
        }

        // Z/C Keys roll the ship around local Z axis
        if keyboard.pressed(KeyCode::KeyZ) {
            flight_state.orbit_roll += 1.5 * dt;
        }
        if keyboard.pressed(KeyCode::KeyC) {
            flight_state.orbit_roll -= 1.5 * dt;
        }

        let current_offset = flight_state.world_pos - prev_pos;
        let current_dist = current_offset.length();
        let safe_dir = if current_dist > 0.1 { current_offset / current_dist } else { Vec3::Z };

        // Initialize spherical orbit angles from current relative position on first orbit frame
        if !autopilot.orbit_initialized {
            autopilot.orbit_pitch = safe_dir.y.clamp(-0.999, 0.999).asin();
            autopilot.orbit_yaw = safe_dir.x.atan2(safe_dir.z);
            autopilot.orbit_initialized = true;
        }

        // Base automatic orbital revolution rate + manual Key A/D and mouse input
        let auto_orbit_rate = 1.0;
        let horiz_delta = (-mouse_delta.x * mouse_sens) + ((auto_orbit_rate + horiz_key_input) * orbit_speed * dt);
        let vert_delta = mouse_delta.y * mouse_sens;

        autopilot.orbit_yaw += horiz_delta;
        autopilot.orbit_pitch = (autopilot.orbit_pitch + vert_delta).clamp(-1.54, 1.54);

        let cos_p = autopilot.orbit_pitch.cos();
        let sin_p = autopilot.orbit_pitch.sin();
        let cos_y = autopilot.orbit_yaw.cos();
        let sin_y = autopilot.orbit_yaw.sin();

        let new_dir = Vec3::new(cos_p * sin_y, sin_p, cos_p * cos_y);

        let radial_speed = (arrival_dist * 0.25).clamp(50.0, 500_000.0);
        let min_dist = (destination_radius * 1.6).max(destination_radius + 50.0);
        let max_dist = arrival_dist * 5.0;

        let effective_dist = if current_dist > 0.1 { current_dist } else { arrival_dist };
        let new_dist = if radial_input != 0.0 {
            (effective_dist + radial_input * radial_speed * dt).clamp(min_dist, max_dist)
        } else {
            effective_dist.clamp(min_dist, max_dist)
        };

        flight_state.world_pos = destination_pos + new_dir * new_dist;

        let planet_vel = if dt > 0.00001 {
            (destination_pos - prev_pos) / dt
        } else {
            Vec3::ZERO
        };

        let orbit_tangent = Vec3::new(-sin_p * sin_y, cos_p, -sin_p * cos_y).normalize_or_zero();
        let tangential_vel = orbit_tangent * (new_dist * orbit_speed);
        let radial_vel = new_dir * (radial_input * radial_speed);
        flight_state.velocity = planet_vel + tangential_vel + radial_vel;

        autopilot.prev_destination_pos = Some(destination_pos);

        // Turn ship to face towards the target body while orbiting and apply Z/C roll
        let look_dir = (destination_pos - flight_state.world_pos).normalize_or_zero();
        if look_dir != Vec3::ZERO {
            let current_forward = ship_transform.forward().as_vec3();
            let rot_diff = Quat::from_rotation_arc(current_forward, look_dir);
            let base_rot = rot_diff * ship_transform.rotation;
            let roll_rot = Quat::from_rotation_z(flight_state.orbit_roll);
            let target_rot = base_rot * roll_rot;
            let rot_decay = 1.0 - (-5.0 * dt).exp();
            ship_transform.rotation = ship_transform.rotation.slerp(target_rot, rot_decay);
        }
        return;
    }

    autopilot.prev_destination_pos = Some(destination_pos);

    autopilot.arrived = false;
    let target_dir = to_target.normalize_or_zero();

    let min_cruise_speed = 12_000.0;
    let decel_start_dist = (arrival_dist * 4.5).clamp(15_000.0, 15_000_000.0);

    // Auto-engage warp mode when planet destination is far away
    if distance > decel_start_dist + 5_000.0 && distance > 60_000.0 {
        flight_state.boost_mode = true;
    } else if distance <= decel_start_dist {
        if flight_state.boost_mode {
            flight_state.boost_mode = false;
            flight_state.rapid_decel = true;
        }
    }

    let max_cruise_speed = if flight_state.boost_mode {
        MAX_SPEED_CAP * (distance / 400_000.0).clamp(1.0, 50.0)
    } else {
        (distance * 0.45).clamp(min_cruise_speed, MAX_SPEED_CAP)
    };

    let target_speed = if distance > decel_start_dist {
        max_cruise_speed
    } else {
        let progress = ((distance - arrival_dist) / (decel_start_dist - arrival_dist)).clamp(0.0, 1.0);
        let approach_curve = progress.powf(1.35); // Smooth non-linear braking curve down to 0 at orbit boundary
        max_cruise_speed * approach_curve
    };

    let vel_decay = 1.0 - (-6.0 * dt).exp(); // Smooth, responsive velocity transition towards target
    flight_state.velocity = flight_state.velocity.lerp(target_dir * target_speed, vel_decay);

    // Safety boundary: prevent autopilot trajectory from penetrating below arrival boundary
    let offset = flight_state.world_pos - destination_pos;
    let dist_from_center = offset.length();
    if dist_from_center < arrival_dist {
        let safe_dir = if dist_from_center > 0.001 { offset / dist_from_center } else { Vec3::Z };
        flight_state.world_pos = destination_pos + safe_dir * arrival_dist;
    }

    if flight_state.velocity.length() > MAX_SPEED_CAP {
        flight_state.velocity = flight_state.velocity.normalize() * MAX_SPEED_CAP;
    }
}

pub fn celestial_collision_system(
    mut autopilot: ResMut<AutoPilotState>,
    mut flight_state: ResMut<FlightState>,
    mut ship_query: Query<&mut Transform, With<Ship>>,
    sun_query: Query<&Sun>,
    planet_query: Query<&Planet>,
    moon_query: Query<&Moon>,
    asteroid_query: Query<&Asteroid>,
) {
    let Ok(mut ship_transform) = ship_query.single_mut() else { return; };

    let old_pos = flight_state.previous_pos;
    let new_pos = flight_state.world_pos;
    let segment_vec = new_pos - old_pos;
    let segment_len_sq = segment_vec.length_squared();

    let mut check_collision = |body_pos: Vec3, body_radius: f32| -> bool {
        let collision_radius = body_radius + 3.0;
        let t = if segment_len_sq > 0.0001 {
            ((body_pos - old_pos).dot(segment_vec) / segment_len_sq).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let closest_pt = old_pos + segment_vec * t;
        let dist = closest_pt.distance(body_pos);

        if dist < collision_radius {
            let mut push_dir = (closest_pt - body_pos).normalize_or_zero();
            if push_dir == Vec3::ZERO {
                push_dir = (old_pos - body_pos).normalize_or_zero();
            }
            if push_dir == Vec3::ZERO {
                push_dir = Vec3::Y;
            }

            flight_state.world_pos = body_pos + push_dir * collision_radius;
            ship_transform.translation = Vec3::ZERO;

            let radial_vel = flight_state.velocity.dot(push_dir);
            if radial_vel < 0.0 {
                flight_state.velocity -= push_dir * radial_vel;
            }

            if autopilot.active {
                autopilot.active = false;
                autopilot.arrived = false;
                autopilot.prev_destination_pos = None;
            }
            return true;
        }
        false
    };

    for sun in &sun_query {
        if check_collision(Vec3::ZERO, sun.radius) {
            return;
        }
    }

    for planet in &planet_query {
        if check_collision(planet.world_pos, planet.radius) {
            return;
        }
    }

    for moon in &moon_query {
        if check_collision(moon.world_pos, moon.radius) {
            return;
        }
    }

    for asteroid in &asteroid_query {
        if check_collision(asteroid.world_pos, asteroid.radius) {
            return;
        }
    }
}

pub const AXIAL_ROTATION_SCALE: f32 = 0.25;

pub fn orbit_planets_system(time: Res<Time>, mut query: Query<(&mut Planet, &mut Transform)>) {
    let dt = time.delta_secs();
    for (mut planet, mut transform) in &mut query {
        transform.rotate_y(planet.rotation_speed * AXIAL_ROTATION_SCALE * dt);
        planet.orbit_angle += planet.orbit_speed * 0.025 * dt;
        planet.world_pos = Vec3::new(
            planet.orbit_radius * planet.orbit_angle.cos(),
            0.0,
            planet.orbit_radius * planet.orbit_angle.sin(),
        );
    }
}

pub fn orbit_moons_system(
    time: Res<Time>,
    planet_query: Query<&Planet>,
    mut moon_query: Query<(&mut Moon, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (mut moon, mut transform) in &mut moon_query {
        transform.rotate_y(moon.rotation_speed * AXIAL_ROTATION_SCALE * dt);
        moon.orbit_angle += moon.orbit_speed * 0.05 * dt;

        let mut parent_pos = Vec3::ZERO;
        for planet in &planet_query {
            if planet.index == moon.parent_index {
                parent_pos = planet.world_pos;
                break;
            }
        }

        moon.world_pos = parent_pos + Vec3::new(
            moon.orbit_radius * moon.orbit_angle.cos(),
            0.0,
            moon.orbit_radius * moon.orbit_angle.sin(),
        );
    }
}

pub fn orbit_asteroids_system(time: Res<Time>, mut query: Query<(&Asteroid, &mut Transform)>) {
    let dt = time.delta_secs();
    for (asteroid, mut transform) in &mut query {
        transform.rotate(Quat::from_axis_angle(
            asteroid.rotation_axis,
            asteroid.rotation_speed * AXIAL_ROTATION_SCALE * dt,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasd_orbit_following_moving_planet() {
        let mut app = App::new();
        app.add_plugins(bevy::input::InputPlugin);
        app.init_resource::<Time>();
        app.init_resource::<ButtonInput<KeyCode>>();

        let target_radius = 10.0;
        let safe_boundary = compute_orbit_boundary(target_radius);

        let mut flight_state = FlightState::default();
        let planet_pos_1 = Vec3::new(1000.0, 0.0, 0.0);
        let ship_pos_1 = Vec3::new(1000.0, 0.0, safe_boundary);
        flight_state.world_pos = ship_pos_1;
        app.insert_resource(flight_state);

        let mut autopilot = AutoPilotState::default();
        autopilot.active = true;
        autopilot.arrived = true;
        autopilot.engine_stopped = true;
        autopilot.destination_index = Some(1);
        autopilot.prev_destination_pos = Some(planet_pos_1);
        app.insert_resource(autopilot);

        let planet_pos_2 = Vec3::new(1200.0, 0.0, 50.0);
        app.world_mut().spawn(Planet {
            index: 1,
            name: "TestPlanet",
            radius: target_radius,
            orbit_radius: 1200.0,
            orbit_speed: 0.1,
            rotation_speed: 0.1,
            orbit_angle: 0.0,
            world_pos: planet_pos_2,
        });

        app.world_mut().spawn((Ship, Transform::default()));

        let mut keyboard = ButtonInput::<KeyCode>::default();
        keyboard.press(KeyCode::KeyD);
        app.insert_resource(keyboard);

        let mut schedule = Schedule::default();
        schedule.add_systems(autopilot_flight_system);
        schedule.run(app.world_mut());

        let new_flight_state = app.world().resource::<FlightState>();
        let new_autopilot = app.world().resource::<AutoPilotState>();

        let offset = new_flight_state.world_pos - planet_pos_2;

        assert_eq!(new_autopilot.prev_destination_pos, Some(planet_pos_2));
        assert!((new_flight_state.world_pos.x - 1200.0).abs() < 100.0);
        assert!((offset.length() - safe_boundary).abs() < 5.0);
    }

    #[test]
    fn test_qe_orbit_distance_adjustment() {
        let mut app = App::new();
        app.add_plugins(bevy::input::InputPlugin);
        app.init_resource::<Time>();
        app.init_resource::<ButtonInput<KeyCode>>();

        let target_radius = 100.0;
        let safe_boundary = compute_orbit_boundary(target_radius);
        let planet_pos = Vec3::new(1000.0, 0.0, 0.0);

        let mut flight_state = FlightState::default();
        flight_state.world_pos = planet_pos + Vec3::new(0.0, 0.0, safe_boundary);
        app.insert_resource(flight_state);

        let mut autopilot = AutoPilotState::default();
        autopilot.active = true;
        autopilot.arrived = true;
        autopilot.engine_stopped = true;
        autopilot.destination_index = Some(1);
        autopilot.prev_destination_pos = Some(planet_pos);
        app.insert_resource(autopilot);

        app.world_mut().spawn(Planet {
            index: 1,
            name: "TestPlanet",
            radius: target_radius,
            orbit_radius: 1000.0,
            orbit_speed: 0.1,
            rotation_speed: 0.1,
            orbit_angle: 0.0,
            world_pos: planet_pos,
        });

        app.world_mut().spawn((Ship, Transform::default()));

        // Test Q key: getting closer to the planet
        let mut keyboard = ButtonInput::<KeyCode>::default();
        keyboard.press(KeyCode::KeyQ);
        app.insert_resource(keyboard);

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(500));

        let mut schedule = Schedule::default();
        schedule.add_systems(autopilot_flight_system);
        schedule.run(app.world_mut());

        let fs_after_q = app.world().resource::<FlightState>();
        let dist_after_q = fs_after_q.world_pos.distance(planet_pos);
        assert!(
            dist_after_q < safe_boundary,
            "Distance after Q ({dist_after_q}) should be closer than safe_boundary ({safe_boundary})"
        );

        // Test E key: getting further away from the planet
        let mut keyboard_e = ButtonInput::<KeyCode>::default();
        keyboard_e.press(KeyCode::KeyE);
        app.insert_resource(keyboard_e);

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(1000));

        let mut schedule2 = Schedule::default();
        schedule2.add_systems(autopilot_flight_system);
        schedule2.run(app.world_mut());

        let fs_after_e = app.world().resource::<FlightState>();
        let dist_after_e = fs_after_e.world_pos.distance(planet_pos);
        assert!(
            dist_after_e > dist_after_q,
            "Distance after E ({dist_after_e}) should be further than distance after Q ({dist_after_q})"
        );
    }

    #[test]
    fn test_mouse_pitch_and_yaw_in_orbit_mode() {
        let mut app = App::new();
        app.add_plugins(bevy::input::InputPlugin);
        app.init_resource::<Time>();
        app.init_resource::<ButtonInput<KeyCode>>();

        let target_radius = 100.0;
        let safe_boundary = compute_orbit_boundary(target_radius);
        let planet_pos = Vec3::new(1000.0, 0.0, 0.0);

        let initial_ship_pos = planet_pos + Vec3::new(0.0, 0.0, safe_boundary);
        let mut flight_state = FlightState::default();
        flight_state.world_pos = initial_ship_pos;
        app.insert_resource(flight_state);

        let mut autopilot = AutoPilotState::default();
        autopilot.active = true;
        autopilot.arrived = true;
        autopilot.engine_stopped = true;
        autopilot.destination_index = Some(1);
        autopilot.prev_destination_pos = Some(planet_pos);
        app.insert_resource(autopilot);

        app.world_mut().spawn(Planet {
            index: 1,
            name: "TestPlanet",
            radius: target_radius,
            orbit_radius: 1000.0,
            orbit_speed: 0.1,
            rotation_speed: 0.1,
            orbit_angle: 0.0,
            world_pos: planet_pos,
        });

        app.world_mut().spawn((Ship, Transform::default()));

        // Write a MouseMotion event simulating horizontal yaw & vertical pitch
        app.world_mut().write_message(MouseMotion {
            delta: Vec2::new(50.0, -30.0),
        });

        let mut schedule = Schedule::default();
        schedule.add_systems(autopilot_flight_system);
        schedule.run(app.world_mut());

        let new_fs = app.world().resource::<FlightState>();
        let new_dist = new_fs.world_pos.distance(planet_pos);

        // Position should have rotated around the planet while maintaining orbit radius
        assert!(new_fs.world_pos != initial_ship_pos);
        assert!((new_dist - safe_boundary).abs() < 1.0);
    }

    #[test]
    fn test_z_axis_roll_controls() {
        let mut app = App::new();
        app.add_plugins(bevy::input::InputPlugin);
        app.init_resource::<Time>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<FlightState>();
        app.init_resource::<AutoPilotState>();

        let ship_entity = app.world_mut().spawn((Ship, Transform::IDENTITY)).id();

        // Press KeyQ (Roll Left)
        let mut keyboard = ButtonInput::<KeyCode>::default();
        keyboard.press(KeyCode::KeyQ);
        app.insert_resource(keyboard);

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(100));

        let mut schedule = Schedule::default();
        schedule.add_systems(pilot_freelook_system);
        schedule.run(app.world_mut());

        let flight_state = app.world().resource::<FlightState>();
        let ship_transform = app.world().entity(ship_entity).get::<Transform>().unwrap();

        // Angular velocity Z should be non-zero (roll left)
        assert!(
            flight_state.angular_velocity.z > 0.0,
            "Angular velocity Z should be positive when rolling left"
        );
        // Ship transform should have rotated around local Z axis
        assert!(
            ship_transform.rotation != Quat::IDENTITY,
            "Ship transform rotation should change after Z-axis roll input"
        );
    }

    #[test]
    fn test_celestial_body_slower_axial_rotation() {
        let mut app = App::new();
        app.init_resource::<Time>();

        let initial_transform = Transform::IDENTITY;
        let planet_entity = app
            .world_mut()
            .spawn((
                Planet {
                    index: 1,
                    name: "TestPlanet",
                    radius: 100.0,
                    orbit_radius: 1000.0,
                    orbit_speed: 0.1,
                    rotation_speed: 0.004,
                    orbit_angle: 0.0,
                    world_pos: Vec3::ZERO,
                },
                initial_transform,
            ))
            .id();

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));

        let mut schedule = Schedule::default();
        schedule.add_systems(orbit_planets_system);
        schedule.run(app.world_mut());

        let transform = app.world().entity(planet_entity).get::<Transform>().unwrap();
        let expected_rotation = Quat::from_rotation_y(0.004 * AXIAL_ROTATION_SCALE * 1.0);
        let diff = transform.rotation.angle_between(expected_rotation);
        assert!(
            diff < 1e-5,
            "Planet rotation should match expected scaled rotation rate (diff={diff})"
        );
    }

    #[test]
    fn test_autopilot_destination_camera_centering() {
        let mut app = App::new();
        app.add_plugins(bevy::input::InputPlugin);
        app.init_resource::<Time>();
        app.init_resource::<ButtonInput<KeyCode>>();

        let mut flight_state = FlightState::default();
        flight_state.world_pos = Vec3::new(10000.0, 0.0, 0.0);
        app.insert_resource(flight_state);

        let mut autopilot = AutoPilotState::default();
        autopilot.active = true;
        autopilot.destination_index = Some(1);
        autopilot.arrived = false;
        app.insert_resource(autopilot);

        let destination_pos = Vec3::new(0.0, 0.0, 0.0);
        app.world_mut().spawn(Planet {
            index: 1,
            name: "TargetPlanet",
            radius: 500.0,
            orbit_radius: 0.0,
            orbit_speed: 0.0,
            rotation_speed: 0.0,
            orbit_angle: 0.0,
            world_pos: destination_pos,
        });

        let ship_entity = app.world_mut().spawn((Ship, Transform::IDENTITY)).id();

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(500));

        let mut schedule = Schedule::default();
        schedule.add_systems(pilot_freelook_system);
        schedule.run(app.world_mut());

        let ship_transform = app.world().entity(ship_entity).get::<Transform>().unwrap();
        let expected_dir = (destination_pos - Vec3::new(10000.0, 0.0, 0.0)).normalize();
        let expected_rot = rotation_looking_to(expected_dir);
        let angle_diff = ship_transform.rotation.angle_between(expected_rot);

        assert!(
            angle_diff < 0.05,
            "Ship transform should align directly towards autopilot destination to keep it centered on camera (diff={angle_diff})"
        );
    }

    #[test]
    fn test_dynamic_in_transit_pathfinding_obstacle_avoidance() {
        let mut app = App::new();
        app.init_resource::<Time>();

        let mut flight_state = FlightState::default();
        flight_state.world_pos = Vec3::new(-100_000.0, 0.0, 0.0);
        app.insert_resource(flight_state);

        let mut autopilot = AutoPilotState::default();
        autopilot.active = true;
        autopilot.destination_index = Some(2);
        autopilot.destination_name = "TargetPlanet";
        autopilot.arrived = false;
        app.insert_resource(autopilot);

        // Destination at +100,000 X
        app.world_mut().spawn(Planet {
            index: 2,
            name: "TargetPlanet",
            radius: 6000.0,
            orbit_radius: 0.0,
            orbit_speed: 0.0,
            rotation_speed: 0.0,
            orbit_angle: 0.0,
            world_pos: Vec3::new(100_000.0, 0.0, 0.0),
        });

        // Intervening obstacle planet directly at origin (0, 0, 0)
        app.world_mut().spawn(Planet {
            index: 1,
            name: "ObstaclePlanet",
            radius: 10_000.0,
            orbit_radius: 0.0,
            orbit_speed: 0.0,
            rotation_speed: 0.0,
            orbit_angle: 0.0,
            world_pos: Vec3::ZERO,
        });

        let mut schedule = Schedule::default();
        schedule.add_systems(autopilot_pathfinding_system);
        schedule.run(app.world_mut());

        let updated_autopilot = app.world().resource::<AutoPilotState>();
        assert!(
            updated_autopilot.current_waypoint.is_some(),
            "Dynamic path-finding should generate a detour waypoint when an obstacle planet blocks the transit trajectory"
        );
    }

    #[test]
    fn test_space_key_stops_autopilot_and_restores_manual_controls() {
        let mut app = App::new();
        app.add_plugins(bevy::input::InputPlugin);
        app.init_resource::<Time>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<FlightState>();

        let mut autopilot = AutoPilotState::default();
        autopilot.active = true;
        autopilot.arrived = true;
        autopilot.engine_stopped = true;
        autopilot.destination_index = Some(3);
        autopilot.destination_name = "Earth";
        app.insert_resource(autopilot);

        app.world_mut().spawn((Ship, Transform::IDENTITY));

        // Press Space Key while autopilot / orbit is engaged
        let mut keyboard = ButtonInput::<KeyCode>::default();
        keyboard.press(KeyCode::Space);
        app.insert_resource(keyboard);

        let mut schedule = Schedule::default();
        schedule.add_systems(ship_flight_system);
        schedule.run(app.world_mut());

        let updated_autopilot = app.world().resource::<AutoPilotState>();
        let updated_flight_state = app.world().resource::<FlightState>();

        assert!(
            !updated_autopilot.active,
            "Autopilot active should be false after pressing Space"
        );
        assert!(
            !updated_autopilot.arrived,
            "Autopilot arrived should be false after pressing Space"
        );
        assert!(
            !updated_autopilot.engine_stopped,
            "Autopilot engine_stopped should be false after pressing Space"
        );
        assert!(
            !updated_flight_state.boost_mode,
            "Pressing Space to cancel autopilot should restore manual flight controls, not engage boost mode"
        );
    }

    #[test]
    fn test_rotation_looking_to_stability() {
        let dir1 = Vec3::new(0.0, 0.0, -1.0);
        let q1 = rotation_looking_to(dir1);
        let f1 = q1 * Vec3::NEG_Z;
        assert!((f1 - dir1).length() < 1e-4);

        let dir2 = Vec3::new(0.0, 1.0, 0.0);
        let q2 = rotation_looking_to(dir2);
        let f2 = q2 * Vec3::NEG_Z;
        assert!((f2 - dir2).length() < 1e-4);
    }

    #[test]
    fn test_orbit_boundary_increased_clearance() {
        let earth_radius = 6371.0;
        let earth_boundary = compute_orbit_boundary(earth_radius);
        assert!(
            earth_boundary > 2.0 * earth_radius,
            "Earth orbit boundary should be at least 2.0x Earth radius (found {earth_boundary})"
        );

        let sun_radius = 696340.0;
        let sun_boundary = compute_orbit_boundary(sun_radius);
        assert!(
            sun_boundary > 3.0 * sun_radius,
            "Sun orbit boundary should be at least 3.0x Sun radius (found {sun_boundary})"
        );
    }

    #[test]
    fn test_sun_orbit_entry_distance_and_gimbal_lock_free_rotation() {
        let sun_radius = 696340.0;
        let sun_boundary = compute_orbit_boundary(sun_radius);
        assert!(sun_boundary >= 2_000_000.0, "Sun orbit boundary should be >= 2,000,000 km to prevent visual crowding");

        let decel_start_dist = (sun_boundary * 4.5).clamp(15_000.0, 15_000_000.0);
        assert!(decel_start_dist > sun_boundary, "Deceleration start distance must be strictly greater than arrival distance for Sun");

        // Verify vertical directions produce stable quaternions without gimbal lock singularities
        let up_dir = Vec3::Y;
        let q_up = rotation_looking_to(up_dir);
        let f_up = q_up * Vec3::NEG_Z;
        assert!((f_up - up_dir).length() < 1e-4);

        let down_dir = Vec3::NEG_Y;
        let q_down = rotation_looking_to(down_dir);
        let f_down = q_down * Vec3::NEG_Z;
        assert!((f_down - down_dir).length() < 1e-4);
    }
}
