use bevy::input::mouse::MouseMotion;
use bevy::ecs::message::MessageReader;
use bevy::prelude::*;

use crate::components::{Asteroid, Moon, PilotCamera, Planet, Ship, StopEngineButton, Sun};
use crate::resources::{AutoPilotState, FlightState};

#[allow(dead_code)]
pub const SPEED_OF_LIGHT: f32 = 299_792.458; // Speed of light in km/s
pub const STANDARD_MAX_SPEED: f32 = 12_000.0; // 12,000 km/s speed cap
pub const MAX_SPEED_CAP: f32 = 449_688.687;  // 1.5x speed of light cap

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
    mut autopilot: ResMut<AutoPilotState>,
    mut camera_query: Query<&mut Transform, With<PilotCamera>>,
    mut ship_query: Query<&mut Transform, (With<Ship>, Without<PilotCamera>)>,
    planet_query: Query<&Planet>,
) {
    let mut mouse_delta = Vec2::ZERO;
    for event in mouse_events.read() {
        mouse_delta += event.delta;
    }

    let dt = time.delta_secs();

    // Disengage autopilot if user moves mouse significantly during autopilot navigation
    if autopilot.active && !autopilot.arrived && mouse_delta.length_squared() > 10.0 {
        autopilot.active = false;
        autopilot.arrived = false;
        autopilot.engine_stopped = false;
    }

    let Ok(mut ship_transform) = ship_query.single_mut() else { return; };

    // When auto-pilot destination is active and in-transit, autopilot aligns ship direction toward target
    if autopilot.active && !autopilot.arrived {
        if let Some(target_idx) = autopilot.target_index {
            let mut target_pos = Vec3::ZERO;
            let mut found = false;

            if target_idx == 0 {
                target_pos = Vec3::ZERO;
                found = true;
            } else {
                for planet in &planet_query {
                    if planet.index == target_idx {
                        target_pos = planet.world_pos;
                        found = true;
                        break;
                    }
                }
            }

            if found {
                let to_target = (target_pos - flight_state.world_pos).normalize_or_zero();
                if to_target != Vec3::ZERO {
                    let target_rot = Quat::from_rotation_arc(Vec3::NEG_Z, to_target);
                    let rot_decay = 1.0 - (-3.0 * dt).exp();
                    ship_transform.rotation = ship_transform.rotation.slerp(target_rot, rot_decay);
                }
            }
        }
    } else if !autopilot.arrived && !autopilot.engine_stopped {
        // Manual steering: Mouse movement controls ship direction (yaw & pitch)
        let sensitivity = 0.0015;
        let key_speed = 1.2 * dt;

        let mut yaw_input = -mouse_delta.x * sensitivity;
        let mut pitch_input = -mouse_delta.y * sensitivity;

        if keyboard.pressed(KeyCode::ArrowLeft) || keyboard.pressed(KeyCode::KeyA) {
            yaw_input += key_speed;
        }
        if keyboard.pressed(KeyCode::ArrowRight) || keyboard.pressed(KeyCode::KeyD) {
            yaw_input -= key_speed;
        }
        if keyboard.pressed(KeyCode::ArrowUp) {
            pitch_input += key_speed;
        }
        if keyboard.pressed(KeyCode::ArrowDown) {
            pitch_input -= key_speed;
        }

        let rot_decay = 1.0 - (-12.0 * dt).exp();
        flight_state.angular_velocity.x = flight_state.angular_velocity.x.lerp(yaw_input, rot_decay);
        flight_state.angular_velocity.y = flight_state.angular_velocity.y.lerp(pitch_input, rot_decay);

        if flight_state.angular_velocity.x.abs() > 0.00001 {
            ship_transform.rotate_local_y(flight_state.angular_velocity.x);
        }
        if flight_state.angular_velocity.y.abs() > 0.00001 {
            ship_transform.rotate_local_x(flight_state.angular_velocity.y);
        }
    }

    // Dynamic cockpit camera lean for pilot perspective
    let lean_yaw = -flight_state.angular_velocity.x * 0.2;
    let lean_pitch = -flight_state.angular_velocity.y * 0.2;
    let lean_roll = -flight_state.angular_velocity.x * 0.35;

    if let Ok(mut cam_transform) = camera_query.single_mut() {
        let target_cam_rot = Quat::from_euler(EulerRot::YXZ, lean_yaw, lean_pitch, lean_roll);
        let cam_decay = 1.0 - (-8.0 * dt).exp();
        cam_transform.rotation = cam_transform.rotation.slerp(target_cam_rot, cam_decay);
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

    // Space Key: Toggle Boost Mode (first press) / Rapid Deceleration (second press)
    if keyboard.just_pressed(KeyCode::Space) {
        if flight_state.boost_mode {
            // Pressing Space again while in boost mode decelerates quickly
            flight_state.boost_mode = false;
            flight_state.rapid_decel = true;
        } else {
            // First press of Space enters boost mode
            flight_state.boost_mode = true;
            flight_state.rapid_decel = false;
            if autopilot.active || autopilot.arrived || autopilot.engine_stopped {
                autopilot.active = false;
                autopilot.arrived = false;
                autopilot.engine_stopped = false;
            }
        }
    }

    // Manual W / S Thrust Controls (Disengage autopilot if active)
    let is_manual_thrust = keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::KeyS);
    if (autopilot.active || autopilot.engine_stopped || autopilot.arrived) && is_manual_thrust {
        autopilot.active = false;
        autopilot.arrived = false;
        autopilot.engine_stopped = false;
    }

    // Defer linear movement calculation to autopilot if active and navigating or orbiting
    if autopilot.active || autopilot.arrived || autopilot.engine_stopped {
        if autopilot.active && !autopilot.arrived {
            let current_vel = flight_state.velocity;
            flight_state.world_pos += current_vel * dt;
            ship_transform.translation = Vec3::ZERO;
        }
        return;
    }

    let forward = ship_transform.forward().as_vec3();
    let mut current_speed = flight_state.velocity.length();

    if flight_state.boost_mode {
        // Boost mode acceleration (up to 449,688 km/s warp speed)
        let boost_accel = 120_000.0;
        current_speed = (current_speed + boost_accel * dt).min(MAX_SPEED_CAP);
        flight_state.velocity = forward * current_speed;
    } else if flight_state.rapid_decel {
        // Rapid deceleration (triggered by pressing Space while boosting)
        let decel_rate = 180_000.0;
        current_speed = (current_speed - decel_rate * dt).max(0.0);
        flight_state.velocity = forward * current_speed;
        if current_speed <= STANDARD_MAX_SPEED || current_speed == 0.0 {
            flight_state.rapid_decel = false;
        }
    } else if keyboard.pressed(KeyCode::KeyW) {
        // W key: Accelerate forward
        let accel_rate = 15_000.0;
        current_speed = (current_speed + accel_rate * dt).min(STANDARD_MAX_SPEED);
        flight_state.velocity = forward * current_speed;
    } else if keyboard.pressed(KeyCode::KeyS) {
        // S key: Decelerate
        let decel_rate = 25_000.0;
        current_speed = (current_speed - decel_rate * dt).max(0.0);
        flight_state.velocity = forward * current_speed;
    } else {
        // Space coasting: smoothly align velocity vector with forward direction as ship turns
        if current_speed > 0.1 {
            flight_state.velocity = flight_state.velocity.lerp(forward * current_speed, (6.0 * dt).min(1.0));
            let decay = 1.0 - (-0.15 * dt).exp();
            flight_state.velocity = flight_state.velocity.lerp(Vec3::ZERO, decay);
        } else {
            flight_state.velocity = Vec3::ZERO;
        }
    }

    let current_vel = flight_state.velocity;
    flight_state.world_pos += current_vel * dt;
    ship_transform.translation = Vec3::ZERO;
}

pub fn autopilot_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut autopilot: ResMut<AutoPilotState>,
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
    ];

    for (key, idx, name) in planet_keys {
        if keyboard.just_pressed(key) {
            autopilot.active = true;
            autopilot.target_index = Some(idx);
            autopilot.target_name = name;
            autopilot.arrived = false;
            autopilot.engine_stopped = false;
        }
    }
}

pub fn stop_engine_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut interaction_query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<StopEngineButton>)>,
    mut autopilot: ResMut<AutoPilotState>,
    mut flight_state: ResMut<FlightState>,
    planet_query: Query<&Planet>,
) {
    let mut triggered = keyboard.just_pressed(KeyCode::KeyO);

    for (interaction, mut bg_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *bg_color = BackgroundColor(Color::srgba(1.0, 0.2, 0.2, 0.95));
                triggered = true;
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgba(0.9, 0.2, 0.2, 0.85));
            }
            Interaction::None => {
                *bg_color = BackgroundColor(Color::srgba(0.8, 0.15, 0.15, 0.75));
            }
        }
    }

    if triggered {
        // Stop engine thrust and enter planet orbit
        flight_state.velocity = Vec3::ZERO;
        autopilot.active = true;
        autopilot.arrived = true;
        autopilot.engine_stopped = true;

        if autopilot.target_index.is_none() {
            let mut min_dist = flight_state.world_pos.distance(Vec3::ZERO);
            let mut closest_idx = 0;
            let mut closest_name = "Sun";

            for planet in &planet_query {
                let dist = flight_state.world_pos.distance(planet.world_pos);
                if dist < min_dist {
                    min_dist = dist;
                    closest_idx = planet.index;
                    closest_name = planet.name;
                }
            }

            autopilot.target_index = Some(closest_idx);
            autopilot.target_name = closest_name;
        }
    }
}

pub fn autopilot_flight_system(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut autopilot: ResMut<AutoPilotState>,
    mut flight_state: ResMut<FlightState>,
    mut ship_query: Query<&mut Transform, With<Ship>>,
    sun_query: Query<&Sun>,
    planet_query: Query<&Planet>,
) {
    if !autopilot.active {
        return;
    }

    let Some(target_idx) = autopilot.target_index else { return; };
    let Ok(mut ship_transform) = ship_query.single_mut() else { return; };

    let mut target_pos = Vec3::ZERO;
    let mut target_radius = 10.0;
    let mut found = false;

    if target_idx == 0 {
        target_pos = Vec3::ZERO;
        if let Ok(sun) = sun_query.single() {
            target_radius = sun.radius;
        } else {
            target_radius = 32790.0;
        }
        found = true;
    } else {
        for planet in &planet_query {
            if planet.index == target_idx {
                target_pos = planet.world_pos;
                target_radius = planet.radius;
                found = true;
                break;
            }
        }
    }

    if !found {
        return;
    }

    let dt = time.delta_secs();
    let to_target = target_pos - flight_state.world_pos;
    let distance = to_target.length();

    let arrival_dist = if target_idx == 0 {
        target_radius * 1.5 + 200.0
    } else {
        target_radius * 1.25 + 25.0
    };

    if distance <= arrival_dist || autopilot.engine_stopped {
        autopilot.arrived = true;
        autopilot.engine_stopped = true;

        // Decelerate & enter parking low orbit around target celestial body (Sun or Planet)
        // Q and E keys allow following planet's orbit left (Q) or right (E)
        let current_offset = flight_state.world_pos - target_pos;
        let current_dist = current_offset.length();
        let safe_dir = if current_dist > 0.1 { current_offset / current_dist } else { Vec3::Z };

        let lerp_speed = 2.5;
        let lerp_factor = (lerp_speed * dt).min(1.0);
        let new_dist = current_dist + (arrival_dist - current_dist) * lerp_factor;

        let mut orbit_input = 0.0;
        if keyboard.pressed(KeyCode::KeyQ) {
            orbit_input -= 1.0; // Follow orbit left around planet
        }
        if keyboard.pressed(KeyCode::KeyE) {
            orbit_input += 1.0; // Follow orbit right around planet
        }

        let orbit_speed = 0.45; // rad/s
        let rot_angle = orbit_input * orbit_speed * dt;
        let rot_quat = Quat::from_rotation_y(rot_angle);
        let new_dir = rot_quat * safe_dir;

        // Update ship's physical position in orbit around target
        flight_state.world_pos = target_pos + new_dir * new_dist;

        if orbit_input != 0.0 {
            let tangent = Vec3::Y.cross(new_dir).normalize_or_zero() * orbit_input.signum();
            flight_state.velocity = tangent * (new_dist * orbit_speed);
        } else {
            flight_state.velocity = Vec3::ZERO;
        }

        // Turn ship to face towards the target body while orbiting
        let look_dir = (target_pos - flight_state.world_pos).normalize_or_zero();
        if look_dir != Vec3::ZERO {
            let target_rot = Quat::from_rotation_arc(Vec3::NEG_Z, look_dir);
            let rot_decay = 1.0 - (-3.0 * dt).exp();
            ship_transform.rotation = ship_transform.rotation.slerp(target_rot, rot_decay);
        }
        return;
    }

    autopilot.arrived = false;
    let target_dir = to_target.normalize_or_zero();

    let min_cruise_speed = 12_000.0;
    let max_cruise_speed = (distance * 0.18).clamp(min_cruise_speed, MAX_SPEED_CAP);
    let decel_start_dist = (arrival_dist * 8.0).clamp(15_000.0, 150_000.0);
    let min_approach_speed = 600.0;

    let target_speed = if distance > decel_start_dist {
        max_cruise_speed
    } else {
        let progress = ((distance - arrival_dist) / (decel_start_dist - arrival_dist)).clamp(0.0, 1.0);
        min_approach_speed + (max_cruise_speed - min_approach_speed) * progress.powf(1.4)
    };

    let vel_decay = 1.0 - (-2.5 * dt).exp();
    flight_state.velocity = flight_state.velocity.lerp(target_dir * target_speed, vel_decay);

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

            // Position ship right above surface
            flight_state.world_pos = body_pos + push_dir * collision_radius;
            ship_transform.translation = Vec3::ZERO;

            // Surface skimming: eliminate radial velocity into body while retaining smooth tangential glide
            let radial_vel = flight_state.velocity.dot(push_dir);
            if radial_vel < 0.0 {
                flight_state.velocity -= push_dir * radial_vel;
            }

            if autopilot.active {
                autopilot.active = false;
                autopilot.arrived = false;
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

pub fn orbit_planets_system(time: Res<Time>, mut query: Query<(&mut Planet, &mut Transform)>) {
    let dt = time.delta_secs();
    for (mut planet, mut transform) in &mut query {
        transform.rotate_y(planet.rotation_speed * dt);
        planet.orbit_angle += planet.orbit_speed * 0.05 * dt;
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
        transform.rotate_y(moon.rotation_speed * dt);
        moon.orbit_angle += moon.orbit_speed * 0.1 * dt;

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
    for (asteroid, mut transform) in &mut query {
        transform.rotate(Quat::from_axis_angle(asteroid.rotation_axis, asteroid.rotation_speed * time.delta_secs()));
    }
}
