use bevy::input::mouse::MouseMotion;
use bevy::ecs::message::MessageReader;
use bevy::prelude::*;

use crate::components::{Moon, PilotCamera, Planet, Ship};
use crate::resources::{AutoPilotState, FlightState};

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
    let sensitivity = 0.0018;
    let key_speed = 1.0 * dt;

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

    flight_state.target_yaw = (flight_state.target_yaw + look_target.x).clamp(-1.57, 1.57);
    flight_state.target_pitch = (flight_state.target_pitch + look_target.y).clamp(-1.2, 1.2);

    let smooth_factor = (30.0 * dt).min(1.0);
    flight_state.yaw += (flight_state.target_yaw - flight_state.yaw) * smooth_factor;
    flight_state.pitch += (flight_state.target_pitch - flight_state.pitch) * smooth_factor;

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
    autopilot: Res<AutoPilotState>,
    mut flight_state: ResMut<FlightState>,
    mut ship_query: Query<&mut Transform, With<Ship>>,
) {
    let dt = time.delta_secs();
    let Ok(mut ship_transform) = ship_query.single_mut() else { return; };

    // Ship Manual Steering (Q / E keys)
    let mut steer_input = 0.0;
    if keyboard.pressed(KeyCode::KeyQ) {
        steer_input += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyE) {
        steer_input -= 1.0;
    }

    let target_rot_speed = steer_input * 0.70;
    flight_state.angular_velocity.x = flight_state.angular_velocity.x.lerp(target_rot_speed, (3.0 * dt).min(1.0));
    ship_transform.rotate_y(flight_state.angular_velocity.x * dt);

    // Defer linear movement to autopilot if active
    if autopilot.active {
        ship_transform.translation += flight_state.velocity * dt;
        return;
    }

    // DOUBLED THRUST SPEEDS: 800 km/s base thrust | 7000 km/s warp boost
    let mut speed = 800.0;
    if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
        speed = 7000.0;
    }

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
        let accel_rate = if speed > 2000.0 { 2.5 } else { 1.8 };
        flight_state.velocity = flight_state.velocity.lerp(input_dir.normalize() * speed, (accel_rate * dt).min(1.0));
    } else {
        flight_state.velocity = flight_state.velocity.lerp(Vec3::ZERO, (0.5 * dt).min(1.0));
    }

    ship_transform.translation += flight_state.velocity * dt;
}

pub fn autopilot_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut autopilot: ResMut<AutoPilotState>,
) {
    let planet_keys = [
        (KeyCode::Digit1, 1, "Mercury"),
        (KeyCode::Digit2, 2, "Venus"),
        (KeyCode::Digit3, 3, "Earth"),
        (KeyCode::Digit4, 4, "Mars"),
        (KeyCode::Digit5, 5, "Jupiter"),
        (KeyCode::Digit6, 6, "Saturn"),
        (KeyCode::Digit7, 7, "Uranus"),
        (KeyCode::Digit8, 8, "Neptune"),
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

    for planet in &planet_query {
        if planet.index == target_idx {
            target_pos = planet.world_pos;
            target_radius = planet.radius;
            found = true;
            break;
        }
    }

    if !found {
        return;
    }

    let dt = time.delta_secs();
    let to_target = target_pos - ship_transform.translation;
    let distance = to_target.length();
    let arrival_dist = target_radius * 15.0 + 800.0;

    if distance <= arrival_dist {
        autopilot.arrived = true;
        flight_state.velocity = flight_state.velocity.lerp(Vec3::ZERO, (2.0 * dt).min(1.0));
        return;
    }

    autopilot.arrived = false;
    let target_dir = to_target.normalize();

    let target_rot = Quat::from_rotation_arc(Vec3::NEG_Z, target_dir);
    ship_transform.rotation = ship_transform.rotation.slerp(target_rot, (2.5 * dt).min(1.0));

    // DOUBLED AUTOPILOT CRUISE SPEED: Up to 150,000 km/s for deep space transit
    let max_cruise_speed = 150_000.0;
    let decel_start_dist = 600_000.0;

    let target_speed = if distance < decel_start_dist {
        let t = (distance / decel_start_dist).clamp(0.05, 1.0);
        max_cruise_speed * t
    } else {
        max_cruise_speed
    };

    flight_state.velocity = flight_state.velocity.lerp(target_dir * target_speed, (1.8 * dt).min(1.0));
}

pub fn celestial_collision_system(
    mut flight_state: ResMut<FlightState>,
    mut ship_query: Query<&mut Transform, With<Ship>>,
    planet_query: Query<&Planet>,
) {
    let Ok(mut ship_transform) = ship_query.single_mut() else { return; };

    for planet in &planet_query {
        let planet_pos = planet.world_pos;
        let min_dist = planet.radius * 1.5 + 20.0;
        let dist = ship_transform.translation.distance(planet_pos);

        if dist < min_dist {
            let push_dir = (ship_transform.translation - planet_pos).normalize_or_zero();
            ship_transform.translation = planet_pos + push_dir * min_dist;
            flight_state.velocity = push_dir * 100.0;
        }
    }
}

pub fn orbit_planets_system(time: Res<Time>, mut query: Query<(&Planet, &mut Transform)>) {
    for (planet, mut transform) in &mut query {
        transform.rotate_y(planet.rotation_speed * time.delta_secs());
    }
}

pub fn orbit_moons_system(time: Res<Time>, mut query: Query<(&Moon, &mut Transform)>) {
    for (moon, mut transform) in &mut query {
        transform.rotate_y(moon.rotation_speed * time.delta_secs());
    }
}
