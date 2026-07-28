use bevy::input::mouse::MouseMotion;
use bevy::ecs::message::MessageReader;
use bevy::prelude::*;

use crate::components::{Asteroid, Moon, PilotCamera, Planet, Ship, Sun};
use crate::resources::{AutoPilotState, FlightState};

#[allow(dead_code)]
pub const SPEED_OF_LIGHT: f32 = 299_792.458; // Speed of light in km/s
pub const STANDARD_MAX_SPEED: f32 = 12_000.0; // 12,000 km/s speed cap (travels Sun -> Mercury in 7.7s, <= 10s)
pub const MAX_SPEED_CAP: f32 = 449_688.687;  // 1.5x speed of light cap (1.5 * 299,792.458 km/s)

pub fn pilot_freelook_system(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_events: MessageReader<MouseMotion>,
    mut flight_state: ResMut<FlightState>,
    mut camera_query: Query<&mut Transform, With<PilotCamera>>,
) {
    let mut mouse_delta = Vec2::ZERO;
    for event in mouse_events.read() {
        mouse_delta += event.delta;
    }

    let dt = time.delta_secs();
    let sensitivity = 0.0012; // Gentle head turn sensitivity
    let key_speed = 0.6 * dt;   // Gentle key pan speed

    let mut look_target = Vec2::ZERO;

    if mouse_delta != Vec2::ZERO {
        look_target.x -= mouse_delta.x * sensitivity;
        look_target.y -= mouse_delta.y * sensitivity;
    }

    if keyboard.pressed(KeyCode::KeyI) || keyboard.pressed(KeyCode::ArrowUp) {
        look_target.y += key_speed;
    }
    if keyboard.pressed(KeyCode::KeyK) || keyboard.pressed(KeyCode::ArrowDown) {
        look_target.y -= key_speed;
    }
    if keyboard.pressed(KeyCode::KeyJ) || keyboard.pressed(KeyCode::ArrowLeft) {
        look_target.x += key_speed;
    }
    if keyboard.pressed(KeyCode::KeyL) || keyboard.pressed(KeyCode::ArrowRight) {
        look_target.x -= key_speed;
    }

    // Natural pilot head rotation limits inside cockpit (yaw: ±77 deg, pitch: ±43 deg)
    flight_state.target_yaw = (flight_state.target_yaw + look_target.x).clamp(-1.35, 1.35);
    flight_state.target_pitch = (flight_state.target_pitch + look_target.y).clamp(-0.75, 0.75);

    let decay = 1.0 - (-8.0 * dt).exp(); // Gentle smooth neck movement decay
    flight_state.yaw += (flight_state.target_yaw - flight_state.yaw) * decay;
    flight_state.pitch += (flight_state.target_pitch - flight_state.pitch) * decay;

    if let Ok(mut cam_transform) = camera_query.single_mut() {
        cam_transform.rotation = Quat::from_euler(
            EulerRot::YXZ,
            flight_state.yaw,
            flight_state.pitch,
            0.0,
        );
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

    // Disengage autopilot if user inputs manual steering or thrust controls
    let is_manual_steering = keyboard.pressed(KeyCode::KeyQ) || keyboard.pressed(KeyCode::KeyE);
    let is_manual_thrust = keyboard.pressed(KeyCode::KeyW)
        || keyboard.pressed(KeyCode::KeyS)
        || keyboard.pressed(KeyCode::KeyA)
        || keyboard.pressed(KeyCode::KeyD)
        || keyboard.pressed(KeyCode::Space);

    if autopilot.active && (is_manual_steering || is_manual_thrust) {
        autopilot.active = false;
        autopilot.arrived = false;
    }

    // Ship Manual Steering (Q / E keys)
    let mut steer_input = 0.0;
    if keyboard.pressed(KeyCode::KeyQ) {
        steer_input += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyE) {
        steer_input -= 1.0;
    }

    let target_rot_speed = steer_input * 0.70;
    let rot_decay = 1.0 - (-2.5 * dt).exp();
    flight_state.angular_velocity.x = flight_state.angular_velocity.x.lerp(target_rot_speed, rot_decay);
    ship_transform.rotate_y(flight_state.angular_velocity.x * dt);

    // Defer linear movement calculation to autopilot if active
    if autopilot.active {
        let current_vel = flight_state.velocity;
        flight_state.world_pos += current_vel * dt;
        ship_transform.translation = Vec3::ZERO;
        return;
    }

    // Flight logic with 8,000 km/s² standard thrust & 120,000 km/s² warp boost acceleration
    let is_boosting = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    let forward = ship_transform.forward().as_vec3();
    let right = ship_transform.right().as_vec3();
    let up = ship_transform.up().as_vec3();

    let mut input_dir = Vec3::ZERO;

    if keyboard.pressed(KeyCode::KeyW) {
        input_dir += forward;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        input_dir -= forward;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        input_dir -= right;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        input_dir += right;
    }
    if keyboard.pressed(KeyCode::Space) {
        input_dir += up;
    }

    if input_dir != Vec3::ZERO {
        // Continuous acceleration: standard 15,000 km/s², warp boost 120,000 km/s²
        let accel_rate = if is_boosting { 120_000.0 } else { 15_000.0 };
        flight_state.velocity += input_dir.normalize() * accel_rate * dt;
    } else {
        // Smooth space coasting / inertia glide
        let decay = 1.0 - (-0.20 * dt).exp();
        flight_state.velocity = flight_state.velocity.lerp(Vec3::ZERO, decay);
    }

    let current_max = if is_boosting { MAX_SPEED_CAP } else { STANDARD_MAX_SPEED };
    if flight_state.velocity.length() > current_max {
        flight_state.velocity = flight_state.velocity.normalize() * current_max;
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
        }
    }
}

pub fn autopilot_flight_system(
    time: Res<Time>,
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
            target_radius = 1800.0;
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

    // Safe orbital arrival distance proportional to body radius
    let arrival_dist = if target_idx == 0 {
        target_radius * 2.5 + 600.0
    } else {
        target_radius * 1.8 + 80.0
    };

    let target_dir = to_target.normalize_or_zero();
    let rot_decay = 1.0 - (-2.5 * dt).exp();

    if distance <= arrival_dist {
        autopilot.arrived = true;

        // Decelerate & enter parking orbit following around target celestial body (Sun or Planet)
        let orbit_speed = 0.20; // rad/s orbital revolution rate
        let current_offset = flight_state.world_pos - target_pos;
        let current_dist = current_offset.length();
        let safe_dir = if current_dist > 0.1 { current_offset / current_dist } else { Vec3::Z };

        let rot = Quat::from_rotation_y(orbit_speed * dt);
        let new_dir = rot * safe_dir;
        
        // Update ship's physical position in orbit around target
        flight_state.world_pos = target_pos + new_dir * arrival_dist;

        // Set tangential orbital velocity
        let tangent = Vec3::Y.cross(new_dir).normalize_or_zero();
        let orbit_linear_speed = arrival_dist * orbit_speed;
        flight_state.velocity = tangent * orbit_linear_speed;

        // Turn ship to continuously face towards the target body while orbiting
        let look_dir = (target_pos - flight_state.world_pos).normalize_or_zero();
        if look_dir != Vec3::ZERO {
            let target_rot = Quat::from_rotation_arc(Vec3::NEG_Z, look_dir);
            ship_transform.rotation = ship_transform.rotation.slerp(target_rot, rot_decay);
        }
        return;
    }

    autopilot.arrived = false;
    if target_dir != Vec3::ZERO {
        let target_rot = Quat::from_rotation_arc(Vec3::NEG_Z, target_dir);
        ship_transform.rotation = ship_transform.rotation.slerp(target_rot, rot_decay);
    }

    // Autopilot cruise speed: guarantees Sun -> Mercury (92,904 km) takes <= 10 seconds (~7.7s)
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

