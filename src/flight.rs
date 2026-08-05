use bevy::ecs::message::MessageReader;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

use crate::components::{
    get_destination_by_key, Asteroid, AutopilotMenuContainer, AutopilotMenuItemButton, FlyingDragon, Moon, PilotCamera, Planet, Ship, Sun,
};
use crate::resources::{AppState, AutoPilotState, AutopilotMenuState, FlightState};

pub const SPEED_OF_LIGHT: f32 = 299_792.47; // Speed of light in km/s (1.0c)
pub const STANDARD_MAX_SPEED: f32 = SPEED_OF_LIGHT * 0.02; // 2% of c (~5,995.85 km/s non-warp impulse speed cap)
pub const MAX_SPEED_CAP: f32 = 149_896_235.0; // 149,896,235 km/s (500.0c FTL warp boost cap)
pub const DRAGON_RENDER_DISTANCE: f32 = 1_500_000.0; // Distance threshold (km) to render and animate Aphora's dragons

pub struct FlightPlugin;

impl Plugin for FlightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AutopilotMenuState>()
            .add_systems(
                Update,
                (
                    (
                        (orbit_planets_system, orbit_moons_system, orbit_asteroids_system, animate_flying_dragons_system, play_dragon_animations_system),
                        autopilot_input_system,
                        autopilot_menu_button_system,
                        autopilot_pathfinding_system,
                        autopilot_flight_system,
                        ship_flight_system,
                        celestial_collision_system,
                        pilot_freelook_system,
                    )
                        .chain(),
                    update_cursor_system,
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

/// Computes the maximum warp speed cap based on distance to destination.
/// Nearby destinations (<= 1M km) cap at 10.0x c, while far outer targets
/// (>= 1,000M km / 1B km) accelerate up to the maximum 500.0x c.
pub fn compute_warp_speed_cap(distance_km: f32) -> f32 {
    let min_warp_c = 10.0;
    let max_warp_c = 500.0;

    let min_dist = 1_000_000.0;
    let max_dist = 1_000_000_000.0;

    if distance_km <= min_dist {
        min_warp_c * SPEED_OF_LIGHT
    } else if distance_km >= max_dist {
        max_warp_c * SPEED_OF_LIGHT
    } else {
        let t = ((distance_km - min_dist) / (max_dist - min_dist)).clamp(0.0, 1.0);
        let factor = t.powf(0.75);
        let target_c = min_warp_c + (max_warp_c - min_warp_c) * factor;
        target_c * SPEED_OF_LIGHT
    }
}

pub fn get_nearest_celestial_distance(
    pos: Vec3,
    sun_query: &Query<&Sun>,
    planet_query: &Query<&Planet>,
    moon_query: &Query<&Moon>,
) -> f32 {
    let mut min_dist = f32::MAX;

    for sun in sun_query {
        let dist = pos.length();
        let surface_dist = (dist - sun.radius).max(0.0);
        if surface_dist < min_dist {
            min_dist = surface_dist;
        }
    }

    for planet in planet_query {
        let dist = pos.distance(planet.world_pos);
        let surface_dist = (dist - planet.radius).max(0.0);
        if surface_dist < min_dist {
            min_dist = surface_dist;
        }
    }

    for moon in moon_query {
        let dist = pos.distance(moon.world_pos);
        let surface_dist = (dist - moon.radius).max(0.0);
        if surface_dist < min_dist {
            min_dist = surface_dist;
        }
    }

    if min_dist == f32::MAX {
        1_000_000_000.0
    } else {
        min_dist
    }
}

pub fn rotation_looking_to(dir: Vec3) -> Quat {
    let dir = dir.normalize_or_zero();
    if dir == Vec3::ZERO {
        return Quat::IDENTITY;
    }
    let up_ref = if dir.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
    let right = dir.cross(up_ref).normalize();
    let up = right.cross(dir).normalize();
    Quat::from_mat3(&Mat3::from_cols(right, up, -dir))
}

pub fn get_celestial_target_full_info(
    destination_idx: usize,
    destination_name: &str,
    sun_query: &Query<&Sun>,
    planet_query: &Query<&Planet>,
    moon_query: &Query<&Moon>,
) -> Option<(Vec3, f32, &'static str, Option<usize>)> {
    if destination_name == "Sun" || (destination_name.is_empty() && destination_idx == 0) {
        let sun = sun_query.iter().next()?;
        return Some((Vec3::ZERO, sun.radius, "Sun", None));
    }

    if !destination_name.is_empty() {
        for moon in moon_query {
            if moon.name == destination_name {
                return Some((
                    moon.world_pos,
                    moon.radius,
                    moon.name,
                    Some(moon.parent_index),
                ));
            }
        }

        for planet in planet_query {
            if planet.name == destination_name {
                return Some((
                    planet.world_pos,
                    planet.radius,
                    planet.name,
                    Some(planet.index),
                ));
            }
        }
    } else {
        for planet in planet_query {
            if planet.index == destination_idx {
                return Some((
                    planet.world_pos,
                    planet.radius,
                    planet.name,
                    Some(planet.index),
                ));
            }
        }
    }

    None
}

pub fn get_celestial_target_info(
    destination_idx: usize,
    destination_name: &str,
    sun_query: &Query<&Sun>,
    planet_query: &Query<&Planet>,
    moon_query: &Query<&Moon>,
) -> Option<(Vec3, f32)> {
    get_celestial_target_full_info(
        destination_idx,
        destination_name,
        sun_query,
        planet_query,
        moon_query,
    )
    .map(|(pos, rad, _, _)| (pos, rad))
}

pub fn update_cursor_system(
    menu_state: Res<AutopilotMenuState>,
    mut cursor_query: Query<&mut bevy::window::CursorOptions, With<Window>>,
) {
    for mut cursor in &mut cursor_query {
        if menu_state.visible {
            if !cursor.visible {
                cursor.visible = true;
                cursor.grab_mode = bevy::window::CursorGrabMode::None;
            }
        } else if cursor.visible {
            cursor.visible = false;
            cursor.grab_mode = bevy::window::CursorGrabMode::Locked;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn pilot_freelook_system(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_events: MessageReader<MouseMotion>,
    mut flight_state: ResMut<FlightState>,
    mut autopilot: ResMut<AutoPilotState>,
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

    // When auto-pilot is active during transit, keep camera focused smoothly on target body
    if autopilot.active {
        let mut target_pos = Vec3::ZERO;
        let mut found = false;

        if let Some(waypoint) = autopilot.current_waypoint {
            target_pos = waypoint;
            found = true;
        } else if let Some((pos, _)) = autopilot.destination_index.and_then(|idx| {
            get_celestial_target_info(
                idx,
                autopilot.destination_name,
                &sun_query,
                &planet_query,
                &moon_query,
            )
        }) {
            target_pos = pos;
            found = true;
        }

        if found {
            let to_target = (target_pos - flight_state.world_pos).normalize_or_zero();
            if to_target != Vec3::ZERO {
                let destination_rot = rotation_looking_to(to_target);
                let current_forward = ship_transform.forward().as_vec3();
                let angle_diff = current_forward.angle_between(to_target);

                let rot_decay = if autopilot.aligned {
                    1.0 - (-12.0 * dt).exp()
                } else {
                    1.0 - (-7.0 * dt).exp()
                };
                ship_transform.rotation = ship_transform.rotation.slerp(destination_rot, rot_decay);
                flight_state.angular_velocity = Vec3::ZERO;

                if angle_diff < 0.04 {
                    autopilot.aligned = true;
                }
            } else {
                autopilot.aligned = true;
            }
        }
    } else {
        // Steering & freelook input (Manual flight mode)
        let sensitivity = 0.0015;
        let key_speed = 1.2 * dt;

        let mut yaw_input = -mouse_delta.x * sensitivity;
        let mut pitch_input = -mouse_delta.y * sensitivity;
        let mut roll_input = 0.0;

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

        if keyboard.pressed(KeyCode::KeyQ) || keyboard.pressed(KeyCode::KeyZ) {
            roll_input += key_speed;
        }
        if keyboard.pressed(KeyCode::KeyE) || keyboard.pressed(KeyCode::KeyX) {
            roll_input -= key_speed;
        }

        let rot_decay = 1.0 - (-12.0 * dt).exp();
        flight_state.angular_velocity.x = flight_state.angular_velocity.x.lerp(yaw_input, rot_decay);
        flight_state.angular_velocity.y = flight_state.angular_velocity.y.lerp(pitch_input, rot_decay);
        flight_state.angular_velocity.z = flight_state.angular_velocity.z.lerp(roll_input, rot_decay);

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
            target_rot *= shake_rot;
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
    planet_query: Query<&Planet>,
    moon_query: Query<&Moon>,
    sun_query: Query<&Sun>,
) {
    let dt = time.delta_secs();
    let Ok(mut ship_transform) = ship_query.single_mut() else { return; };

    flight_state.previous_pos = flight_state.world_pos;

    let is_autopilot_engaged = autopilot.is_engaged();

    if is_autopilot_engaged {
        if keyboard.just_pressed(KeyCode::Space)
            || (autopilot.arrived
                && (keyboard.just_pressed(KeyCode::KeyW)
                    || keyboard.just_pressed(KeyCode::KeyS)
                    || keyboard.just_pressed(KeyCode::KeyA)
                    || keyboard.just_pressed(KeyCode::KeyD)
                    || keyboard.just_pressed(KeyCode::KeyQ)
                    || keyboard.just_pressed(KeyCode::KeyE)
                    || keyboard.just_pressed(KeyCode::KeyZ)
                    || keyboard.just_pressed(KeyCode::KeyX)))
        {
            autopilot.reset_all();
            flight_state.boost_mode = false;
            flight_state.rapid_decel = false;
            flight_state.angular_velocity = Vec3::ZERO;
        }
    } else if keyboard.just_pressed(KeyCode::Space) {
        if flight_state.boost_mode {
            flight_state.boost_mode = false;
            flight_state.rapid_decel = true;
        } else {
            flight_state.boost_mode = true;
            flight_state.rapid_decel = false;
        }
    }

    if !is_autopilot_engaged {
        if keyboard.just_pressed(KeyCode::KeyX) || keyboard.just_pressed(KeyCode::Backspace) {
            flight_state.boost_mode = false;
            flight_state.rapid_decel = false;
            flight_state.velocity = Vec3::ZERO;
        }

        if flight_state.boost_mode {
            let target_distance = get_nearest_celestial_distance(
                flight_state.world_pos,
                &sun_query,
                &planet_query,
                &moon_query,
            );
            let target_speed = compute_warp_speed_cap(target_distance);
            let accel_rate = 1.0 - (-2.5 * dt).exp();
            let forward = ship_transform.forward().as_vec3();
            let current_speed = flight_state.velocity.length();
            let new_speed = current_speed.lerp(target_speed, accel_rate).max(STANDARD_MAX_SPEED);
            flight_state.velocity = forward * new_speed;
        } else if flight_state.rapid_decel {
            let decel_rate = 1.0 - (-3.5 * dt).exp();
            let current_speed = flight_state.velocity.length();
            let new_speed = current_speed.lerp(0.0, decel_rate);
            if new_speed <= 20.0 {
                flight_state.rapid_decel = false;
                flight_state.velocity = Vec3::ZERO;
            } else {
                let dir = flight_state.velocity.normalize_or_zero();
                flight_state.velocity = dir * new_speed;
            }
        } else {
            let accel_power = 1_500.0 * dt;
            let forward = ship_transform.forward().as_vec3();

            if keyboard.pressed(KeyCode::KeyW) {
                flight_state.velocity += forward * accel_power;
            }

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

            if flight_state.velocity.length() > STANDARD_MAX_SPEED {
                flight_state.velocity = flight_state.velocity.normalize() * STANDARD_MAX_SPEED;
            }
        }

        let current_vel = flight_state.velocity;
        flight_state.world_pos += current_vel * dt;
    }

    ship_transform.translation = Vec3::ZERO;
}

pub fn trigger_autopilot_destination(
    dest_key: usize,
    dest_name: &'static str,
    autopilot: &mut AutoPilotState,
    flight_state: &FlightState,
    sun_query: &Query<&Sun>,
    planet_query: &Query<&Planet>,
    moon_query: &Query<&Moon>,
) {
    let mut target_pos = Vec3::ZERO;
    if let Some((pos, _)) = get_celestial_target_info(
        dest_key,
        dest_name,
        sun_query,
        planet_query,
        moon_query,
    ) {
        target_pos = pos;
    }

    let start_pos = flight_state.world_pos;
    let to_dest = target_pos - start_pos;
    let dist = to_dest.length();
    let mut waypoint = None;

    if dist > 100.0 {
        let line_dir = to_dest / dist;
        let mut closest_obstacle_dist = f32::MAX;
        let mut chosen_waypoint = None;

        let sun_radius = sun_query.iter().next().map(|s| s.radius).unwrap_or(696340.0);

        if dest_name != "Sun" && dest_key != 0 {
            let to_sun = -start_pos;
            let proj = to_sun.dot(line_dir);
            let min_clearance = (sun_radius * 2.8).max(250_000.0);
            if proj > 100.0 && proj < dist - 1000.0 {
                let closest_pt = start_pos + line_dir * proj;
                let clearance = closest_pt.length();
                if clearance < min_clearance {
                    let offset = closest_pt;
                    let perp = if offset.length() > 10.0 {
                        offset.normalize()
                    } else {
                        let mut side = line_dir.cross(Vec3::Y);
                        if side.length_squared() < 0.001 {
                            side = line_dir.cross(Vec3::Z);
                        }
                        side.normalize()
                    };
                    let wp = perp * (min_clearance * 1.5);
                    let d = start_pos.length();
                    if d < closest_obstacle_dist {
                        closest_obstacle_dist = d;
                        chosen_waypoint = Some(wp);
                    }
                }
            }
        }

        for planet in planet_query {
            if planet.name != dest_name {
                let to_planet = planet.world_pos - start_pos;
                let proj = to_planet.dot(line_dir);
                let min_clearance = (planet.radius * 2.8).max(compute_orbit_boundary(planet.radius) + 5000.0);
                if proj > 100.0 && proj < dist - 1000.0 {
                    let closest_pt = start_pos + line_dir * proj;
                    let clearance = (closest_pt - planet.world_pos).length();
                    if clearance < min_clearance {
                        let offset = closest_pt - planet.world_pos;
                        let perp = if offset.length() > 10.0 {
                            offset.normalize()
                        } else {
                            let mut side = line_dir.cross(Vec3::Y);
                            if side.length_squared() < 0.001 {
                                side = line_dir.cross(Vec3::Z);
                            }
                            side.normalize()
                        };
                        let wp = planet.world_pos + perp * (min_clearance * 1.5);
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
    autopilot.arrived = false;
    autopilot.aligned = false;
    autopilot.destination_index = Some(dest_key);
    autopilot.destination_name = dest_name;
    autopilot.prev_destination_pos = None;
    autopilot.current_waypoint = waypoint;
}

#[allow(clippy::too_many_arguments)]
pub fn autopilot_input_system(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut menu_state: ResMut<AutopilotMenuState>,
    mut autopilot: ResMut<AutoPilotState>,
    flight_state: Res<FlightState>,
    mut query: Query<(&AutopilotMenuContainer, &mut Visibility)>,
    sun_query: Query<&Sun>,
    planet_query: Query<&Planet>,
    moon_query: Query<&Moon>,
) {
    if keyboard.just_pressed(KeyCode::KeyM) {
        menu_state.visible = !menu_state.visible;
        menu_state.input_buffer.clear();
        for (_, mut vis) in &mut query {
            *vis = if menu_state.visible { Visibility::Inherited } else { Visibility::Hidden };
        }
        return;
    }

    if !menu_state.visible {
        return;
    }

    if menu_state.buffer_timer > 0.0 {
        menu_state.buffer_timer -= time.delta_secs();
        if menu_state.buffer_timer <= 0.0 {
            if let Ok(dest_key) = menu_state.input_buffer.parse::<usize>() {
                if let Some(dest) = get_destination_by_key(dest_key) {
                    trigger_autopilot_destination(
                        dest.key_num,
                        dest.name,
                        &mut autopilot,
                        &flight_state,
                        &sun_query,
                        &planet_query,
                        &moon_query,
                    );
                    menu_state.visible = false;
                    for (_, mut vis) in &mut query {
                        *vis = Visibility::Hidden;
                    }
                }
            }
            menu_state.input_buffer.clear();
        }
    }

    let digit_keys = [
        (KeyCode::Digit0, '0'), (KeyCode::Digit1, '1'), (KeyCode::Digit2, '2'),
        (KeyCode::Digit3, '3'), (KeyCode::Digit4, '4'), (KeyCode::Digit5, '5'),
        (KeyCode::Digit6, '6'), (KeyCode::Digit7, '7'), (KeyCode::Digit8, '8'),
        (KeyCode::Digit9, '9'), (KeyCode::Numpad0, '0'), (KeyCode::Numpad1, '1'),
        (KeyCode::Numpad2, '2'), (KeyCode::Numpad3, '3'), (KeyCode::Numpad4, '4'),
        (KeyCode::Numpad5, '5'), (KeyCode::Numpad6, '6'), (KeyCode::Numpad7, '7'),
        (KeyCode::Numpad8, '8'), (KeyCode::Numpad9, '9'),
    ];

    for (key, ch) in digit_keys {
        if keyboard.just_pressed(key) {
            menu_state.input_buffer.push(ch);
            menu_state.buffer_timer = 0.25;

            if let Ok(dest_key) = menu_state.input_buffer.parse::<usize>() {
                if dest_key == 99 || (dest_key <= 17 && dest_key != 1) {
                    if let Some(dest) = get_destination_by_key(dest_key) {
                        trigger_autopilot_destination(
                            dest.key_num,
                            dest.name,
                            &mut autopilot,
                            &flight_state,
                            &sun_query,
                            &planet_query,
                            &moon_query,
                        );
                        menu_state.visible = false;
                        menu_state.input_buffer.clear();
                        menu_state.buffer_timer = 0.0;
                        for (_, mut vis) in &mut query {
                            *vis = Visibility::Hidden;
                        }
                        return;
                    }
                }
            }
        }
    }
}

pub fn autopilot_menu_button_system(
    mut interaction_query: Query<
        (&Interaction, &AutopilotMenuItemButton, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut menu_state: ResMut<AutopilotMenuState>,
    mut autopilot: ResMut<AutoPilotState>,
    flight_state: Res<FlightState>,
    mut container_query: Query<(&AutopilotMenuContainer, &mut Visibility)>,
    sun_query: Query<&Sun>,
    planet_query: Query<&Planet>,
    moon_query: Query<&Moon>,
) {
    for (interaction, item, mut bg_color, mut border_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                if let Some(dest) = get_destination_by_key(item.destination_key) {
                    trigger_autopilot_destination(
                        dest.key_num,
                        dest.name,
                        &mut autopilot,
                        &flight_state,
                        &sun_query,
                        &planet_query,
                        &moon_query,
                    );
                    menu_state.visible = false;
                    menu_state.input_buffer.clear();
                    menu_state.buffer_timer = 0.0;
                    for (_, mut vis) in &mut container_query {
                        *vis = Visibility::Hidden;
                    }
                }
            }
            Interaction::Hovered => {
                if item.destination_key == 99 {
                    *bg_color = BackgroundColor(Color::srgba(0.50, 0.10, 0.75, 0.85));
                    *border_color = BorderColor::all(Color::srgba(0.95, 0.35, 1.0, 0.95));
                } else {
                    *bg_color = BackgroundColor(Color::srgba(0.0, 0.5, 0.75, 0.65));
                    *border_color = BorderColor::all(Color::srgba(0.0, 0.9, 1.0, 0.9));
                }
            }
            Interaction::None => {
                if item.destination_key == 99 {
                    *bg_color = BackgroundColor(Color::srgba(0.25, 0.05, 0.40, 0.85));
                    *border_color = BorderColor::all(Color::srgba(0.85, 0.25, 1.0, 0.85));
                } else {
                    *bg_color = BackgroundColor(Color::srgba(0.05, 0.1, 0.18, 0.65));
                    *border_color = BorderColor::all(Color::srgba(0.0, 0.7, 0.9, 0.25));
                }
            }
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
    if !autopilot.active {
        return;
    }

    let Some(destination_idx) = autopilot.destination_index else { return; };

    let (final_target_pos, _, dest_name, parent_planet_idx) = match get_celestial_target_full_info(
        destination_idx,
        autopilot.destination_name,
        &sun_query,
        &planet_query,
        &moon_query,
    ) {
        Some(info) => info,
        None => return,
    };

    let start_pos = flight_state.world_pos;

    if let Some(wp) = autopilot.current_waypoint {
        let dist_to_wp = start_pos.distance(wp);
        if dist_to_wp < 15_000.0 {
            autopilot.current_waypoint = None;
        }
    }

    let to_final = final_target_pos - start_pos;
    let total_dist = to_final.length();

    if total_dist <= 1000.0 {
        autopilot.current_waypoint = None;
        return;
    }

    let line_dir = to_final / total_dist;
    let mut closest_obstacle_dist = f32::MAX;
    let mut chosen_waypoint = None;

    let sun_radius = sun_query.iter().next().map(|s| s.radius).unwrap_or(696340.0);

    // 1. Check Sun
    if dest_name != "Sun" && destination_idx != 0 {
        let to_sun = -start_pos;
        let proj = to_sun.dot(line_dir);
        let min_clearance = (sun_radius * 2.8).max(250_000.0);

        if proj > 100.0 && proj < total_dist - 1000.0 {
            let closest_pt = start_pos + line_dir * proj;
            let clearance = closest_pt.length();
            if clearance < min_clearance {
                let offset = closest_pt;
                let perp = if offset.length() > 10.0 {
                    offset.normalize()
                } else {
                    let mut side = line_dir.cross(Vec3::Y);
                    if side.length_squared() < 0.001 {
                        side = line_dir.cross(Vec3::Z);
                    }
                    side.normalize()
                };
                let wp = perp * (min_clearance * 1.5);
                let d = start_pos.length();
                if d < closest_obstacle_dist {
                    closest_obstacle_dist = d;
                    chosen_waypoint = Some(wp);
                }
            }
        }
    }

    // 2. Check Planets
    for planet in &planet_query {
        if destination_idx != planet.index && dest_name != planet.name && Some(planet.index) != parent_planet_idx {
            let to_planet = planet.world_pos - start_pos;
            let proj = to_planet.dot(line_dir);
            let min_clearance = (planet.radius * 2.8).max(compute_orbit_boundary(planet.radius) + 5000.0);

            if proj > 100.0 && proj < total_dist - 1000.0 {
                let closest_pt = start_pos + line_dir * proj;
                let clearance = (closest_pt - planet.world_pos).length();
                if clearance < min_clearance {
                    let offset = closest_pt - planet.world_pos;
                    let perp = if offset.length() > 10.0 {
                        offset.normalize()
                    } else {
                        let mut side = line_dir.cross(Vec3::Y);
                        if side.length_squared() < 0.001 {
                            side = line_dir.cross(Vec3::Z);
                        }
                        side.normalize()
                    };
                    let wp = planet.world_pos + perp * (min_clearance * 1.5);
                    let d = start_pos.distance(planet.world_pos);
                    if d < closest_obstacle_dist {
                        closest_obstacle_dist = d;
                        chosen_waypoint = Some(wp);
                    }
                }
            }
        }
    }

    // 3. Check Moons
    for moon in &moon_query {
        if dest_name != moon.name {
            if destination_idx == moon.parent_index && start_pos.distance(final_target_pos) < 100_000.0 {
                continue;
            }
            let to_moon = moon.world_pos - start_pos;
            let proj = to_moon.dot(line_dir);
            let min_clearance = (moon.radius * 2.8).max(compute_orbit_boundary(moon.radius) + 2000.0);

            if proj > 100.0 && proj < total_dist - 1000.0 {
                let closest_pt = start_pos + line_dir * proj;
                let clearance = (closest_pt - moon.world_pos).length();
                if clearance < min_clearance {
                    let offset = closest_pt - moon.world_pos;
                    let perp = if offset.length() > 10.0 {
                        offset.normalize()
                    } else {
                        let mut side = line_dir.cross(Vec3::Y);
                        if side.length_squared() < 0.001 {
                            side = line_dir.cross(Vec3::Z);
                        }
                        side.normalize()
                    };
                    let wp = moon.world_pos + perp * (min_clearance * 1.5);
                    let d = start_pos.distance(moon.world_pos);
                    if d < closest_obstacle_dist {
                        closest_obstacle_dist = d;
                        chosen_waypoint = Some(wp);
                    }
                }
            }
        }
    }

    if let Some(new_wp) = chosen_waypoint {
        let old_wp = autopilot.current_waypoint;
        autopilot.current_waypoint = Some(new_wp);

        if let Some(old) = old_wp {
            if old.distance(new_wp) > 5000.0 {
                let to_old = (old - start_pos).normalize_or_zero();
                let to_new = (new_wp - start_pos).normalize_or_zero();
                if to_old.angle_between(to_new) > 0.25 {
                    autopilot.aligned = false;
                }
            }
        } else {
            let to_final = (final_target_pos - start_pos).normalize_or_zero();
            let to_new = (new_wp - start_pos).normalize_or_zero();
            if to_final.angle_between(to_new) > 0.25 {
                autopilot.aligned = false;
            }
        }
    } else {
        autopilot.current_waypoint = None;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn autopilot_flight_system(
    time: Res<Time>,
    mut autopilot: ResMut<AutoPilotState>,
    mut flight_state: ResMut<FlightState>,
    mut ship_query: Query<&mut Transform, With<Ship>>,
    planet_query: Query<&Planet>,
    moon_query: Query<&Moon>,
    sun_query: Query<&Sun>,
) {
    let dt = time.delta_secs();

    if !autopilot.active {
        return;
    }

    let mut destination_pos = Vec3::ZERO;
    let mut destination_radius = 6371.0;
    let mut found = false;

    if let Some((pos, radius)) = autopilot.destination_index.and_then(|destination_idx| {
        get_celestial_target_info(
            destination_idx,
            autopilot.destination_name,
            &sun_query,
            &planet_query,
            &moon_query,
        )
    }) {
        destination_pos = pos;
        destination_radius = radius;
        found = true;
    }

    if !found {
        return;
    }

    let destination_vel = if let Some(prev) = autopilot.prev_destination_pos {
        let delta = destination_pos - prev;
        if dt > 0.00001 && delta.length() < 500_000.0 {
            delta / dt
        } else {
            Vec3::ZERO
        }
    } else {
        Vec3::ZERO
    };

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
    let real_distance_to_dest = (destination_pos - flight_state.world_pos).length();

    let arrival_dist = compute_orbit_boundary(destination_radius);
    let target_dir = to_target.normalize_or_zero();

    // Check alignment state and rotate ship towards target vector
    if let Some(mut ship_transform) = ship_query.iter_mut().next() {
        if target_dir != Vec3::ZERO {
            let destination_rot = rotation_looking_to(target_dir);
            let current_forward = ship_transform.forward().as_vec3();
            let angle_diff = current_forward.angle_between(target_dir);
            if angle_diff < 0.05 {
                autopilot.aligned = true;
            } else {
                let rot_decay = 1.0 - (-10.0 * dt).exp();
                ship_transform.rotation = ship_transform.rotation.slerp(destination_rot, rot_decay);
            }
        } else {
            autopilot.aligned = true;
        }
    } else {
        autopilot.aligned = true;
    }

    let Ok(mut ship_transform) = ship_query.single_mut() else {
        // Headless execution fallback without ship transform component
        if autopilot.arrived {
            flight_state.world_pos = destination_pos;
            return;
        }
        let decel_start_dist = (arrival_dist * 3.0).max(50_000.0);
        let warp_speed_cap = compute_warp_speed_cap(real_distance_to_dest);
        if real_distance_to_dest > decel_start_dist + 10_000.0 && real_distance_to_dest > 30_000.0 {
            flight_state.boost_mode = true;
        }
        let step_vel = target_dir * warp_speed_cap;
        flight_state.velocity = step_vel;
        flight_state.world_pos += step_vel * dt;
        return;
    };

    // Smooth arrived state position holding following the planet's orbital position
    if autopilot.arrived {
        let offset = flight_state.world_pos - destination_pos;
        let safe_dir = if offset.length() > 0.001 { offset.normalize() } else { Vec3::Z };
        let target_world_pos = destination_pos + safe_dir * arrival_dist;
        let pos_lerp = 1.0 - (-15.0 * dt).exp();
        flight_state.world_pos = flight_state.world_pos.lerp(target_world_pos, pos_lerp);
        flight_state.velocity = destination_vel;
        flight_state.boost_mode = false;
        flight_state.rapid_decel = false;
        autopilot.prev_destination_pos = Some(destination_pos);
        ship_transform.translation = Vec3::ZERO;
        return;
    }

    if !autopilot.aligned {
        // Alignment rotation phase: hold back acceleration until ship rotates towards target vector
        flight_state.boost_mode = false;
        flight_state.rapid_decel = false;

        let vel_decay = 1.0 - (-8.0 * dt).exp();
        flight_state.velocity = flight_state.velocity.lerp(destination_vel, vel_decay);

        let current_rel_pos = flight_state.world_pos - destination_pos;
        let current_rel_vel = flight_state.velocity - destination_vel;
        let new_rel_pos = current_rel_pos + current_rel_vel * dt;
        flight_state.world_pos = destination_pos + new_rel_pos;
        ship_transform.translation = Vec3::ZERO;
        return;
    }

    let rel_dist_to_arrival = (real_distance_to_dest - arrival_dist).max(0.0);

    // Smooth arrival transition trigger when close to boundary (within 50 km)
    if rel_dist_to_arrival <= 50.0 {
        autopilot.arrived = true;
        let offset = flight_state.world_pos - destination_pos;
        let safe_dir = if offset.length() > 0.001 { offset.normalize() } else { Vec3::Z };
        flight_state.world_pos = destination_pos + safe_dir * arrival_dist;
        flight_state.velocity = destination_vel;
        flight_state.boost_mode = false;
        flight_state.rapid_decel = false;
        autopilot.prev_destination_pos = Some(destination_pos);
        ship_transform.translation = Vec3::ZERO;
        return;
    }

    autopilot.prev_destination_pos = Some(destination_pos);

    let decel_start_dist = (arrival_dist * 3.0).max(50_000.0);

    if real_distance_to_dest > decel_start_dist + 10_000.0 && real_distance_to_dest > 30_000.0 {
        flight_state.boost_mode = true;
        flight_state.rapid_decel = false;
    } else if real_distance_to_dest <= decel_start_dist {
        flight_state.boost_mode = false;
    }

    let min_approach_speed = 1200.0;
    let warp_speed_cap = compute_warp_speed_cap(real_distance_to_dest);
    let max_cruise_speed = if flight_state.boost_mode {
        warp_speed_cap
    } else {
        (rel_dist_to_arrival * 2.5).clamp(min_approach_speed, warp_speed_cap)
    };

    let max_safe_rel_speed = if dt > 0.0001 {
        (rel_dist_to_arrival * 0.85) / dt
    } else {
        warp_speed_cap
    };

    let target_rel_speed = max_cruise_speed.min(max_safe_rel_speed);
    let target_vel = destination_vel + target_dir * target_rel_speed;

    let decay_rate = if real_distance_to_dest <= decel_start_dist {
        let progress = (1.0 - (real_distance_to_dest / decel_start_dist)).clamp(0.0, 1.0);
        4.0 + 4.0 * progress
    } else {
        4.0
    };
    let vel_decay = 1.0 - (-decay_rate * dt).exp();
    flight_state.velocity = flight_state.velocity.lerp(target_vel, vel_decay);

    // Hard cap velocity length during deceleration to strictly prevent single-frame FTL overshoots
    if real_distance_to_dest <= decel_start_dist {
        let max_allowed_total_speed = destination_vel.length() + max_safe_rel_speed;
        if flight_state.velocity.length() > max_allowed_total_speed {
            let vel_dir = flight_state.velocity.normalize_or_zero();
            flight_state.velocity = vel_dir * max_allowed_total_speed;
        }
    }

    if flight_state.velocity.length() > warp_speed_cap {
        flight_state.velocity = flight_state.velocity.normalize() * warp_speed_cap;
    }

    let current_rel_pos = flight_state.world_pos - destination_pos;
    let current_rel_vel = flight_state.velocity - destination_vel;
    let new_rel_pos = current_rel_pos + current_rel_vel * dt;
    flight_state.world_pos = destination_pos + new_rel_pos;
    ship_transform.translation = Vec3::ZERO;
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
    let is_ap_active = autopilot.active;
    let dest_idx = autopilot.destination_index;
    let dest_name = autopilot.destination_name;

    let mut check_collision = |body_pos: Vec3, body_radius: f32, body_vel: Vec3, is_target_destination: bool| -> bool {
        let arrival_dist = compute_orbit_boundary(body_radius);
        let check_radius = if is_ap_active && is_target_destination {
            arrival_dist
        } else {
            body_radius + 3.0
        };

        let t = if segment_len_sq > 0.0001 {
            ((body_pos - old_pos).dot(segment_vec) / segment_len_sq).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let closest_pt = old_pos + segment_vec * t;
        let dist = closest_pt.distance(body_pos);

        if dist < check_radius {
            let mut push_dir = (closest_pt - body_pos).normalize_or_zero();
            if push_dir == Vec3::ZERO {
                push_dir = (old_pos - body_pos).normalize_or_zero();
            }
            if push_dir == Vec3::ZERO {
                push_dir = Vec3::Y;
            }

            if autopilot.active && is_target_destination {
                flight_state.world_pos = body_pos + push_dir * arrival_dist;
                flight_state.velocity = body_vel;
                flight_state.boost_mode = false;
                flight_state.rapid_decel = false;
                autopilot.arrived = true;
            } else {
                let collision_radius = body_radius + 3.0;
                flight_state.world_pos = body_pos + push_dir * (collision_radius + 1.0);
                let radial_vel = flight_state.velocity.dot(push_dir);
                if radial_vel < 0.0 {
                    flight_state.velocity -= push_dir * radial_vel;
                }
                if autopilot.active {
                    println!("COLLISION ABORT! body_pos={:?}, body_radius={}, dest_idx={:?}, dest_name={}", body_pos, body_radius, dest_idx, dest_name);
                    autopilot.reset_all();
                }
            }
            ship_transform.translation = Vec3::ZERO;
            return true;
        }
        false
    };

    let parent_planet_idx = if dest_idx == Some(100) {
        let mut p_idx = None;
        for moon in &moon_query {
            if moon.name == dest_name {
                p_idx = Some(moon.parent_index);
                break;
            }
        }
        p_idx
    } else {
        None
    };

    for sun in &sun_query {
        let is_target = is_ap_active && (dest_name == "Sun" || (dest_name.is_empty() && dest_idx == Some(0)));
        if check_collision(Vec3::ZERO, sun.radius, Vec3::ZERO, is_target) {
            return;
        }
    }

    for planet in &planet_query {
        let is_target = is_ap_active && (dest_name == planet.name || (dest_name.is_empty() && dest_idx == Some(planet.index)) || (dest_idx == Some(100) && Some(planet.index) == parent_planet_idx));
        if check_collision(planet.world_pos, planet.radius, Vec3::ZERO, is_target) {
            return;
        }
    }

    for moon in &moon_query {
        let is_target = is_ap_active && dest_idx == Some(100) && moon.name == dest_name;
        if check_collision(moon.world_pos, moon.radius, Vec3::ZERO, is_target) {
            return;
        }
    }

    for asteroid in &asteroid_query {
        if check_collision(asteroid.world_pos, asteroid.radius, Vec3::ZERO, false) {
            return;
        }
    }
}

pub const AXIAL_ROTATION_SCALE: f32 = 0.2625;
pub const PLANET_ORBIT_TIME_SCALE: f32 = 0.000008;
pub const MOON_ORBIT_TIME_SCALE: f32 = 0.0001;

pub fn orbit_planets_system(time: Res<Time>, mut query: Query<(&mut Planet, &mut Transform)>) {
    let dt = time.delta_secs();
    for (mut planet, mut transform) in &mut query {
        transform.rotate_y(planet.rotation_speed * AXIAL_ROTATION_SCALE * dt);
        planet.orbit_angle += planet.orbit_speed * PLANET_ORBIT_TIME_SCALE * dt;
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
        moon.orbit_angle += moon.orbit_speed * MOON_ORBIT_TIME_SCALE * dt;

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

pub fn orbit_asteroids_system(
    time: Res<Time>,
    flight_state: Res<FlightState>,
    mut query: Query<(&Asteroid, &mut Transform)>,
) {
    let dt = time.delta_secs();
    let cam_pos = flight_state.world_pos;
    for (asteroid, mut transform) in &mut query {
        if asteroid.world_pos.distance(cam_pos) > 200_000.0 {
            continue;
        }
        transform.rotate(Quat::from_axis_angle(
            asteroid.rotation_axis,
            asteroid.rotation_speed * AXIAL_ROTATION_SCALE * dt,
        ));
    }
}

pub fn animate_flying_dragons_system(
    time: Res<Time>,
    flight_state: Res<FlightState>,
    planet_query: Query<&Planet>,
    mut query: Query<(&mut FlyingDragon, &mut Transform, &mut Visibility)>,
) {
    // Find Aphora's world position
    let mut aphora_pos = None;
    for planet in &planet_query {
        if planet.index == 99 || planet.name == "Aphora" {
            aphora_pos = Some(planet.world_pos);
            break;
        }
    }

    let Some(aphora_pos) = aphora_pos else { return; };
    let dist_to_aphora = flight_state.world_pos.distance(aphora_pos);

    // CPU Optimization: If spaceship is far away from Aphora, hide dragons and skip transform math
    if dist_to_aphora > DRAGON_RENDER_DISTANCE {
        for (_, _, mut vis) in &mut query {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
        }
        return;
    }

    let dt = time.delta_secs();
    let t = time.elapsed_secs();

    for (mut dragon, mut transform, mut vis) in &mut query {
        if *vis != Visibility::Inherited {
            *vis = Visibility::Inherited;
        }

        dragon.angle += dragon.fly_speed * dt;
        // Bounded orbit radius to strictly prevent surface collision (Aphora radius = 52,000 km)
        let r = (dragon.orbit_radius + (t * 1.5 + dragon.phase_offset).sin() * 600.0).max(53_800.0);
        let base_x = r * dragon.angle.cos();
        let base_z = r * dragon.angle.sin();
        let base_y = (t * 1.2 + dragon.phase_offset).sin() * 800.0 + (dragon.angle * 2.0).cos() * 500.0;

        let tilt_rot = Quat::from_rotation_z(dragon.tilt);
        let raw_pos = Vec3::new(base_x, base_y, base_z);
        let final_pos = tilt_rot * raw_pos;

        let base_tangent = Vec3::new(
            -r * dragon.angle.sin(),
            (t * 1.2 + dragon.phase_offset).cos() * 1000.0,
            r * dragon.angle.cos(),
        )
        .normalize_or_zero();
        let final_tangent = tilt_rot * base_tangent;

        let rot = Quat::from_rotation_arc(Vec3::NEG_Z, final_tangent);

        transform.translation = final_pos;
        transform.rotation = rot;
    }
}

pub fn play_dragon_animations_system(
    mut player_query: Query<&mut AnimationPlayer, Added<AnimationPlayer>>,
) {
    for mut player in &mut player_query {
        player.play(AnimationNodeIndex::new(0)).repeat();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_z_axis_roll_controls() {
        let mut app = App::new();
        app.add_plugins(bevy::input::InputPlugin);
        app.init_resource::<Time>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<FlightState>();
        app.init_resource::<AutoPilotState>();

        let ship_entity = app.world_mut().spawn((Ship, Transform::IDENTITY)).id();

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

        assert!(
            flight_state.angular_velocity.z > 0.0,
            "Angular velocity Z should be positive when rolling left"
        );
        assert!(
            ship_transform.rotation != Quat::IDENTITY,
            "Ship transform rotation should change after Z-axis roll input"
        );

        let mut keyboard_x = ButtonInput::<KeyCode>::default();
        keyboard_x.press(KeyCode::KeyX);
        app.insert_resource(keyboard_x);

        schedule.run(app.world_mut());

        let flight_state_x = app.world().resource::<FlightState>();
        assert!(
            flight_state_x.angular_velocity.z < 0.0,
            "Angular velocity Z should be negative when pressing KeyX for Roll Right"
        );
    }

    #[test]
    fn test_dynamic_in_transit_pathfinding_obstacle_avoidance() {
        let mut app = App::new();
        app.init_resource::<Time>();

        let flight_state = FlightState {
            world_pos: Vec3::new(-100_000.0, 0.0, 0.0),
            ..Default::default()
        };
        app.insert_resource(flight_state);

        let autopilot = AutoPilotState {
            active: true,
            destination_index: Some(2),
            destination_name: "TargetPlanet",
            ..Default::default()
        };
        app.insert_resource(autopilot);

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
            "Dynamic path-finding should generate a detour waypoint when an obstacle planet blocks trajectory"
        );
    }

    #[test]
    fn test_space_key_stops_autopilot_and_restores_manual_controls() {
        let mut app = App::new();
        app.add_plugins(bevy::input::InputPlugin);
        app.init_resource::<Time>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<FlightState>();

        let autopilot = AutoPilotState {
            active: true,
            destination_index: Some(3),
            destination_name: "Earth",
            ..Default::default()
        };
        app.insert_resource(autopilot);

        app.world_mut().spawn((Ship, Transform::IDENTITY));

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
            !updated_flight_state.boost_mode,
            "Pressing Space to cancel autopilot should restore manual flight controls, not engage boost mode"
        );
    }

    #[test]
    fn test_earth_autopilot_warp_deceleration_and_release_controls() {
        let mut app = App::new();
        app.add_plugins(bevy::input::InputPlugin);
        app.init_resource::<Time>();
        app.init_resource::<ButtonInput<KeyCode>>();

        let earth_radius = 6371.0;
        let earth_pos = Vec3::new(149_597_870.7, 0.0, 0.0);
        let arrival_boundary = compute_orbit_boundary(earth_radius);
        let start_pos = earth_pos + Vec3::new(arrival_boundary + 50_000.0, 0.0, 0.0);

        let flight_state = FlightState {
            world_pos: start_pos,
            ..Default::default()
        };
        app.insert_resource(flight_state);

        let autopilot = AutoPilotState {
            active: true,
            destination_index: Some(3),
            destination_name: "Earth",
            ..Default::default()
        };
        app.insert_resource(autopilot);

        app.world_mut().spawn(Planet {
            index: 3,
            name: "Earth",
            radius: earth_radius,
            orbit_radius: 149_597_870.7,
            orbit_speed: 0.15,
            rotation_speed: 0.01,
            orbit_angle: 0.0,
            world_pos: earth_pos,
        });

        app.world_mut().spawn((Ship, Transform::default()));

        let mut schedule = Schedule::default();
        schedule.add_systems((autopilot_flight_system, ship_flight_system).chain());

        let mut arrived_at_earth = false;
        for _ in 0..1200 {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_millis(16));
            schedule.run(app.world_mut());

            let current_ap = app.world().resource::<AutoPilotState>();
            if current_ap.arrived {
                arrived_at_earth = true;
                let current_fs = app.world().resource::<FlightState>();
                let dist = current_fs.world_pos.distance(earth_pos);
                assert!(
                    (dist - arrival_boundary).abs() < 5000.0,
                    "Autopilot should reach Earth surface arrival boundary (dist={dist}, boundary={arrival_boundary})"
                );
                break;
            }
        }

        assert!(
            arrived_at_earth,
            "Autopilot to Earth should reach arrived state upon approaching surface"
        );

        // Press Space to undock
        let mut keyboard = ButtonInput::<KeyCode>::default();
        keyboard.press(KeyCode::Space);
        app.insert_resource(keyboard);
        schedule.run(app.world_mut());

        let final_ap = app.world().resource::<AutoPilotState>();
        assert!(!final_ap.active, "Pressing Space must undock from Earth");
    }

    #[test]
    fn test_all_planets_autopilot_approach_consistency() {
        let test_bodies = [
            (1, "Mercury", 2439.7, 57_909_050.0),
            (2, "Venus", 6051.8, 108_208_000.0),
            (3, "Earth", 6371.0, 149_597_870.7),
            (4, "Mars", 3389.5, 227_939_200.0),
        ];

        for (idx, name, radius, orbit_r) in test_bodies {
            let mut app = App::new();
            app.add_plugins(bevy::input::InputPlugin);
            app.init_resource::<Time>();
            app.init_resource::<ButtonInput<KeyCode>>();

            let body_pos = Vec3::new(orbit_r, 0.0, 0.0);
            let boundary = compute_orbit_boundary(radius);
            let start_pos = body_pos + Vec3::new(boundary + 20_000.0, 0.0, 0.0);

            let flight_state = FlightState {
                world_pos: start_pos,
                ..Default::default()
            };
            app.insert_resource(flight_state);

            let autopilot = AutoPilotState {
                active: true,
                destination_index: Some(idx),
                destination_name: name,
                ..Default::default()
            };
            app.insert_resource(autopilot);

            app.world_mut().spawn(Planet {
                index: idx,
                name,
                radius,
                orbit_radius: orbit_r,
                orbit_speed: 0.1,
                rotation_speed: 0.01,
                orbit_angle: 0.0,
                world_pos: body_pos,
            });

            app.world_mut().spawn((Ship, Transform::default()));

            let mut schedule = Schedule::default();
            schedule.add_systems((autopilot_flight_system, ship_flight_system).chain());

            let mut arrived = false;
            for _ in 0..1500 {
                app.world_mut()
                    .resource_mut::<Time>()
                    .advance_by(std::time::Duration::from_millis(16));
                schedule.run(app.world_mut());

                let current_ap = app.world().resource::<AutoPilotState>();
                if current_ap.arrived {
                    arrived = true;
                    break;
                }
            }

            assert!(
                arrived,
                "Autopilot to celestial body {name} (index {idx}) should enter arrived state when close to surface"
            );
        }
    }

    #[test]
    fn test_collision_with_obstacle_body_aborts_autopilot() {
        let mut app = App::new();
        app.add_plugins(bevy::input::InputPlugin);
        app.init_resource::<Time>();
        app.init_resource::<ButtonInput<KeyCode>>();

        let obstacle_pos = Vec3::new(1000.0, 0.0, 0.0);
        let obstacle_radius = 500.0;

        let flight_state = FlightState {
            previous_pos: obstacle_pos + Vec3::new(505.0, 0.0, 0.0),
            world_pos: obstacle_pos + Vec3::new(501.0, 0.0, 0.0),
            ..Default::default()
        };
        app.insert_resource(flight_state);

        let autopilot = AutoPilotState {
            active: true,
            destination_index: Some(2),
            destination_name: "TargetPlanet",
            ..Default::default()
        };
        app.insert_resource(autopilot);

        app.world_mut().spawn(Planet {
            index: 1,
            name: "ObstaclePlanet",
            radius: obstacle_radius,
            orbit_radius: 1000.0,
            orbit_speed: 0.0,
            rotation_speed: 0.0,
            orbit_angle: 0.0,
            world_pos: obstacle_pos,
        });

        app.world_mut().spawn((Ship, Transform::default()));

        let mut schedule = Schedule::default();
        schedule.add_systems(celestial_collision_system);
        schedule.run(app.world_mut());

        let final_ap = app.world().resource::<AutoPilotState>();

        assert!(
            !final_ap.active,
            "Collision detection with an obstacle body MUST set autopilot.active to false"
        );
    }

    #[test]
    fn test_flight_control_mode_enum_and_helpers() {
        let mut ap = AutoPilotState::default();
        assert_eq!(ap.mode(), crate::resources::FlightControlMode::Manual);
        assert!(!ap.is_engaged());

        ap.active = true;
        assert_eq!(ap.mode(), crate::resources::FlightControlMode::AutopilotTransit);
        assert!(ap.is_engaged());

        ap.arrived = true;
        assert_eq!(ap.mode(), crate::resources::FlightControlMode::AutopilotArrived);

        ap.reset_all();
        assert_eq!(ap.mode(), crate::resources::FlightControlMode::Manual);
        assert!(!ap.is_engaged());
    }

    #[test]
    fn test_mars_long_distance_warp_no_overshoot() {
        let mut app = App::new();
        app.add_plugins(bevy::input::InputPlugin);
        app.add_plugins(bevy::time::TimePlugin);
        app.init_resource::<ButtonInput<KeyCode>>();

        let mars_radius = 3389.5;
        let mars_orbit_r = 227_939_200.0;
        let mars_pos = Vec3::new(mars_orbit_r, 0.0, 0.0);
        let arrival_boundary = compute_orbit_boundary(mars_radius);

        // Start 50 million km away from Mars with initial high velocity (simulating FTL warp)
        let start_pos = mars_pos + Vec3::new(50_000_000.0, 0.0, 0.0);

        let flight_state = FlightState {
            world_pos: start_pos,
            velocity: Vec3::new(-10_000_000.0, 0.0, 0.0),
            boost_mode: true,
            ..Default::default()
        };
        app.insert_resource(flight_state);

        let autopilot = AutoPilotState {
            active: true,
            destination_index: Some(4),
            destination_name: "Mars",
            ..Default::default()
        };
        app.insert_resource(autopilot);

        app.world_mut().spawn(Planet {
            index: 4,
            name: "Mars",
            radius: mars_radius,
            orbit_radius: mars_orbit_r,
            orbit_speed: 0.1,
            rotation_speed: 0.01,
            orbit_angle: 0.0,
            world_pos: mars_pos,
        });

        app.world_mut().spawn((Ship, Transform::default()));

        let mut schedule = Schedule::default();
        schedule.add_systems((
            orbit_planets_system,
            autopilot_flight_system,
            ship_flight_system,
            celestial_collision_system,
        ).chain());

        let mut arrived_at_mars = false;
        let mut min_distance_recorded = f32::MAX;

        for _frame in 0..5000 {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_millis(16));
            schedule.run(app.world_mut());

            let fs = app.world().resource::<FlightState>();
            let dist = fs.world_pos.distance(mars_pos);
            if dist < min_distance_recorded {
                min_distance_recorded = dist;
            }

            let ap = app.world().resource::<AutoPilotState>();
            if ap.arrived {
                arrived_at_mars = true;
                break;
            }
        }

        assert!(
            arrived_at_mars,
            "Autopilot to Mars must enter arrived state"
        );
        assert!(
            min_distance_recorded >= arrival_boundary - 1.0,
            "Autopilot MUST NOT overshoot inside Mars surface boundary (min dist={min_distance_recorded}, arrival_boundary={arrival_boundary})"
        );
    }

    #[test]
    fn test_autopilot_menu_toggle_with_m_key() {
        let mut app = App::new();
        app.add_plugins(bevy::input::InputPlugin);
        app.init_resource::<Time>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<FlightState>();
        app.init_resource::<AutoPilotState>();
        app.init_resource::<AutopilotMenuState>();

        let menu_entity = app.world_mut().spawn((AutopilotMenuContainer, Visibility::Hidden)).id();

        let mut keyboard = ButtonInput::<KeyCode>::default();
        keyboard.press(KeyCode::KeyM);
        app.insert_resource(keyboard);

        let mut schedule = Schedule::default();
        schedule.add_systems(autopilot_input_system);
        schedule.run(app.world_mut());

        let menu_state = app.world().resource::<AutopilotMenuState>();
        assert!(menu_state.visible, "Pressing M should open the autopilot menu");

        let vis = app.world().entity(menu_entity).get::<Visibility>().unwrap();
        assert_eq!(*vis, Visibility::Inherited, "Menu container visibility should update to Inherited");

        let mut keyboard_m_again = ButtonInput::<KeyCode>::default();
        keyboard_m_again.press(KeyCode::KeyM);
        app.insert_resource(keyboard_m_again);
        schedule.run(app.world_mut());

        let menu_state_closed = app.world().resource::<AutopilotMenuState>();
        assert!(!menu_state_closed.visible, "Pressing M again should close the autopilot menu");
    }

    #[test]
    fn test_autopilot_menu_numeric_key_selection() {
        let mut app = App::new();
        app.add_plugins(bevy::input::InputPlugin);
        app.init_resource::<Time>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<FlightState>();
        app.init_resource::<AutoPilotState>();

        let menu_state = AutopilotMenuState {
            visible: true,
            ..Default::default()
        };
        app.insert_resource(menu_state);

        app.world_mut().spawn(Planet {
            index: 3,
            name: "Earth",
            radius: 6371.0,
            orbit_radius: 149_597_870.7,
            orbit_speed: 0.1,
            rotation_speed: 0.01,
            orbit_angle: 0.0,
            world_pos: Vec3::ZERO,
        });

        let menu_entity = app.world_mut().spawn((AutopilotMenuContainer, Visibility::Inherited)).id();

        let mut keyboard = ButtonInput::<KeyCode>::default();
        keyboard.press(KeyCode::Digit3);
        app.insert_resource(keyboard);

        let mut schedule = Schedule::default();
        schedule.add_systems(autopilot_input_system);
        schedule.run(app.world_mut());

        let ap = app.world().resource::<AutoPilotState>();
        assert!(ap.active, "Choosing a destination via numeric key should engage autopilot");
        assert_eq!(ap.destination_name, "Earth");

        let updated_menu_state = app.world().resource::<AutopilotMenuState>();
        assert!(!updated_menu_state.visible, "Choosing a destination must hide the menu");

        let vis = app.world().entity(menu_entity).get::<Visibility>().unwrap();
        assert_eq!(*vis, Visibility::Hidden, "Menu container visibility should be Hidden");
    }

    #[test]
    fn test_aphora_key_99_autopilot_destination() {
        use crate::components::CelestialDestinationType;
        let dest = get_destination_by_key(99);
        assert!(dest.is_some(), "Key 99 should exist in AUTOPILOT_DESTINATIONS");
        let dest = dest.unwrap();
        assert_eq!(dest.name, "Aphora");
        assert_eq!(dest.dest_type, CelestialDestinationType::Planet(99));
    }

    #[test]
    fn test_max_speed_500x_c_and_dragon_distance_rendering() {
        assert!(
            (MAX_SPEED_CAP - (500.0 * SPEED_OF_LIGHT)).abs() < 1000.0,
            "MAX_SPEED_CAP should be 500x c, found MAX_SPEED_CAP={}, 500c={}",
            MAX_SPEED_CAP,
            500.0 * SPEED_OF_LIGHT
        );
        assert_eq!(DRAGON_RENDER_DISTANCE, 1_500_000.0);
        assert!(
            (STANDARD_MAX_SPEED - (0.02 * SPEED_OF_LIGHT)).abs() < 0.001,
            "STANDARD_MAX_SPEED should be 2% of speed of light (0.02c)"
        );
    }

    #[test]
    fn test_saturn_key_10_autopilot_destination_resolves_to_saturn_not_ceres() {
        let dest = get_destination_by_key(10);
        assert!(dest.is_some(), "Key 10 should exist in AUTOPILOT_DESTINATIONS");
        let dest = dest.unwrap();
        assert_eq!(dest.name, "Saturn");

        let mut app = App::new();
        // Spawn Ceres with planet index 10
        app.world_mut().spawn(Planet {
            index: 10,
            name: "Ceres",
            radius: 470.0,
            orbit_radius: 0.0,
            orbit_speed: 0.0,
            rotation_speed: 0.0,
            orbit_angle: 0.0,
            world_pos: Vec3::new(1000.0, 0.0, 0.0),
        });
        // Spawn Saturn with planet index 6
        app.world_mut().spawn(Planet {
            index: 6,
            name: "Saturn",
            radius: 58232.0,
            orbit_radius: 0.0,
            orbit_speed: 0.0,
            rotation_speed: 0.0,
            orbit_angle: 0.0,
            world_pos: Vec3::new(5000.0, 0.0, 0.0),
        });

        fn test_system(
            sun_query: Query<&Sun>,
            planet_query: Query<&Planet>,
            moon_query: Query<&Moon>,
        ) {
            let target = get_celestial_target_full_info(
                10,
                "Saturn",
                &sun_query,
                &planet_query,
                &moon_query,
            );
            assert!(target.is_some(), "Target info should be found for Saturn");
            let (pos, radius, name, idx) = target.unwrap();
            assert_eq!(name, "Saturn", "Target name should be Saturn, not Ceres");
            assert_eq!(pos, Vec3::new(5000.0, 0.0, 0.0), "Target pos should match Saturn pos");
            assert_eq!(radius, 58232.0);
            assert_eq!(idx, Some(6));
        }

        let mut schedule = Schedule::default();
        schedule.add_systems(test_system);
        schedule.run(app.world_mut());
    }

    #[test]
    fn test_axial_rotation_scale_increased() {
        assert_eq!(
            AXIAL_ROTATION_SCALE, 0.2625,
            "AXIAL_ROTATION_SCALE should be scaled by 5% (0.25 * 1.05)"
        );
    }

    #[test]
    fn test_smoothed_high_speed_acceleration_and_deceleration() {
        let mut app = App::new();
        app.add_plugins(bevy::input::InputPlugin);
        app.init_resource::<Time>();
        app.init_resource::<ButtonInput<KeyCode>>();

        let flight_state = FlightState {
            boost_mode: true,
            velocity: Vec3::ZERO,
            ..Default::default()
        };
        app.insert_resource(flight_state);
        app.init_resource::<AutoPilotState>();
        app.world_mut().spawn((Ship, Transform::IDENTITY));

        let mut schedule = Schedule::default();
        schedule.add_systems(ship_flight_system);

        // Advance 1 frame (16ms)
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(16));
        schedule.run(app.world_mut());

        let fs1 = app.world().resource::<FlightState>();
        let speed1 = fs1.velocity.length();

        // Ensure smooth gradual acceleration rather than instant snap to max cap
        assert!(
            speed1 > 0.0 && speed1 < MAX_SPEED_CAP,
            "Boost mode acceleration should build up smoothly, speed1={speed1}"
        );
    }

    #[test]
    fn test_autopilot_smooth_rotation_alignment_before_acceleration() {
        let mut app = App::new();
        app.add_plugins(bevy::input::InputPlugin);
        app.init_resource::<Time>();
        app.init_resource::<ButtonInput<KeyCode>>();

        let ship_transform = Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)); // Facing +X
        app.world_mut().spawn((Ship, ship_transform));
        app.init_resource::<FlightState>();

        let autopilot = AutoPilotState {
            active: true,
            destination_index: Some(3),
            destination_name: "Earth",
            aligned: false,
            ..Default::default()
        };
        app.insert_resource(autopilot);

        app.world_mut().spawn(Planet {
            index: 3,
            name: "Earth",
            radius: 6371.0,
            orbit_radius: 149_597_870.7,
            orbit_speed: 0.0,
            rotation_speed: 0.0,
            orbit_angle: 0.0,
            world_pos: Vec3::new(0.0, 0.0, -100_000.0), // Target is along -Z
        });

        let mut schedule = Schedule::default();
        schedule.add_systems((autopilot_flight_system, pilot_freelook_system).chain());

        // Frame 1: Ship should hold back acceleration while rotating to align
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(16));
        schedule.run(app.world_mut());

        let fs1 = app.world().resource::<FlightState>();
        assert!(
            !fs1.boost_mode,
            "FTL boost mode MUST NOT engage until ship completes alignment rotation towards target"
        );

        // Advance frames until aligned
        for _ in 0..60 {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_millis(16));
            schedule.run(app.world_mut());
        }

        let ap_aligned = app.world().resource::<AutoPilotState>();
        assert!(
            ap_aligned.aligned,
            "Autopilot should transition to aligned state once ship rotates towards target"
        );
    }

    #[test]
    fn test_pitch_yaw_leveling_irrespective_of_orientation() {
        let dir_to_target = Vec3::new(1.0, 0.5, -1.0).normalize();
        let rot = rotation_looking_to(dir_to_target);
        
        let (_yaw, _pitch, roll) = rot.to_euler(EulerRot::YXZ);
        assert!(
            roll.abs() < 0.001,
            "rotation_looking_to MUST level pitch/yaw and eliminate roll (roll={roll})"
        );
        let fwd = rot * Vec3::NEG_Z;
        assert!(
            (fwd - dir_to_target).length() < 0.001,
            "rotation_looking_to MUST align ship forward vector with target direction"
        );
    }

    #[test]
    fn test_pathfinding_obstacle_directly_in_front_bypass() {
        let mut app = App::new();
        app.init_resource::<Time>();

        let start_pos = Vec3::new(0.0, 0.0, 100_000.0);
        let obstacle_pos = Vec3::new(0.0, 0.0, 50_000.0); // Directly in front of ship
        let target_pos = Vec3::new(0.0, 0.0, -500_000.0); // Directly behind obstacle

        let flight_state = FlightState {
            world_pos: start_pos,
            ..Default::default()
        };
        app.insert_resource(flight_state);

        let autopilot = AutoPilotState {
            active: true,
            destination_index: Some(2),
            destination_name: "TargetPlanet",
            ..Default::default()
        };
        app.insert_resource(autopilot);

        app.world_mut().spawn(Planet {
            index: 1,
            name: "ObstaclePlanet",
            radius: 10_000.0,
            orbit_radius: 0.0,
            orbit_speed: 0.0,
            rotation_speed: 0.0,
            orbit_angle: 0.0,
            world_pos: obstacle_pos,
        });

        app.world_mut().spawn(Planet {
            index: 2,
            name: "TargetPlanet",
            radius: 6000.0,
            orbit_radius: 0.0,
            orbit_speed: 0.0,
            rotation_speed: 0.0,
            orbit_angle: 0.0,
            world_pos: target_pos,
        });

        let mut schedule = Schedule::default();
        schedule.add_systems(autopilot_pathfinding_system);
        schedule.run(app.world_mut());

        let ap = app.world().resource::<AutoPilotState>();
        assert!(
            ap.current_waypoint.is_some(),
            "Pathfinding MUST generate a bypass waypoint when destination is directly behind a celestial body in front of ship"
        );

        let wp = ap.current_waypoint.unwrap();
        let dist_wp_to_obstacle = wp.distance(obstacle_pos);
        let min_clearance = 10_000.0 * 2.8;
        assert!(
            dist_wp_to_obstacle >= min_clearance,
            "Bypass waypoint must provide generous safety clearance outside obstacle radius (dist={dist_wp_to_obstacle}, min={min_clearance})"
        );
    }

    #[test]
    fn test_distance_based_warp_acceleration_phases() {
        let speed_10c = 10.0 * SPEED_OF_LIGHT;
        let speed_500c = 500.0 * SPEED_OF_LIGHT;

        // Nearby destination (e.g. Moon or close orbit <= 1M km) engages 10x c phase
        let cap_close = compute_warp_speed_cap(500_000.0);
        assert!(
            (cap_close - speed_10c).abs() < 1.0,
            "Warp speed cap for nearby targets (500k km) MUST be 10x c (found {})",
            cap_close / SPEED_OF_LIGHT
        );

        let cap_1m = compute_warp_speed_cap(1_000_000.0);
        assert!(
            (cap_1m - speed_10c).abs() < 1.0,
            "Warp speed cap for 1M km targets MUST be 10x c"
        );

        // Mid-distance inner planet transfers engage intermediate acceleration phases
        let cap_50m = compute_warp_speed_cap(50_000_000.0);
        let c_50m = cap_50m / SPEED_OF_LIGHT;
        assert!(
            c_50m > 10.0 && c_50m < 100.0,
            "Warp cap for 50M km should be intermediate phase (found {}c)",
            c_50m
        );

        let cap_150m = compute_warp_speed_cap(150_000_000.0);
        let c_150m = cap_150m / SPEED_OF_LIGHT;
        assert!(
            c_150m > c_50m && c_150m < 250.0,
            "Warp cap for 150M km should scale higher than 50M km phase (found {}c vs {}c)",
            c_150m, c_50m
        );

        // Far destinations (>= 1B km) engage maximum 500x c phase
        let cap_1b = compute_warp_speed_cap(1_000_000_000.0);
        assert!(
            (cap_1b - speed_500c).abs() < 1.0,
            "Warp speed cap for distant targets (1B km) MUST reach 500x c"
        );

        let cap_5b = compute_warp_speed_cap(5_000_000_000.0);
        assert!(
            (cap_5b - speed_500c).abs() < 1.0,
            "Warp speed cap for deep space targets (5B km) MUST cap at 500x c"
        );
    }
}

