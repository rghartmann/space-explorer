use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;
use bevy::transform::TransformSystems;

use crate::components::{
    AutoPilotHudText, AutopilotWarningBanner, CelestialDestinationType, CelestialLabel, Moon, PilotCamera, Planet, Sun,
};
use crate::flight::SPEED_OF_LIGHT;
use crate::resources::{AppState, AutoPilotState, FlightControlMode, FlightState};

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (update_hud_system, exit_on_esc)
                .run_if(in_state(AppState::InGame).or_else(in_state(AppState::Loading))),
        )
        .add_systems(
            PostUpdate,
            update_celestial_labels_system
                .after(TransformSystems::Propagate)
                .run_if(in_state(AppState::InGame)),
        );
    }
}

pub fn format_dual_space_distance(dist_km: f32) -> String {
    let au = dist_km / 149_597_870.7;
    if dist_km >= 149_597_870.7 * 0.1 {
        format!("{:.2} AU ({:.1}M km)", au, dist_km / 1_000_000.0)
    } else if dist_km >= 1_000_000.0 {
        format!("{:.3} AU ({:.2}M km)", au, dist_km / 1_000_000.0)
    } else if dist_km >= 10_000.0 {
        format!("{:.4} AU ({:.0} km)", au, dist_km)
    } else {
        format!("{:.5} AU ({:.0} km)", au, dist_km)
    }
}

pub fn format_dual_space_distance_compact(dist_km: f32) -> String {
    let au = dist_km / 149_597_870.7;
    if dist_km >= 149_597_870.7 * 0.05 {
        format!("{:.2}AU ({:.0}M km)", au, dist_km / 1_000_000.0)
    } else if dist_km >= 100_000.0 {
        format!("{:.3}AU ({:.0}k km)", au, dist_km / 1_000.0)
    } else {
        format!("{:.4}AU ({:.0} km)", au, dist_km)
    }
}

pub fn exit_on_esc(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut app_exit: MessageWriter<AppExit>,
    mut cursor_query: Query<&mut bevy::window::CursorOptions, With<Window>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        for mut cursor in &mut cursor_query {
            cursor.visible = true;
            cursor.grab_mode = bevy::window::CursorGrabMode::None;
        }
        app_exit.write(AppExit::Success);
    }
}

pub fn update_hud_system(
    autopilot: Res<AutoPilotState>,
    flight_state: Res<FlightState>,
    sun_query: Query<&Sun>,
    planet_query: Query<&Planet>,
    moon_query: Query<&Moon>,
    mut text_query: Query<&mut Text, With<AutoPilotHudText>>,
    mut banner_query: Query<(&mut Visibility, &Children), (With<AutopilotWarningBanner>, Without<AutoPilotHudText>)>,
    mut banner_text_query: Query<&mut Text, Without<AutoPilotHudText>>,
) {
    let speed = flight_state.velocity.length();
    let speed_of_light = SPEED_OF_LIGHT;

    for (mut vis, children) in &mut banner_query {
        if autopilot.active {
            *vis = Visibility::Inherited;
            let dest_name_upper = autopilot.destination_name.to_uppercase();

            let mut child_idx = 0;
            for child in children.iter() {
                if let Ok(mut text) = banner_text_query.get_mut(child) {
                    if child_idx == 0 {
                        if autopilot.arrived {
                            **text = format!("[!] ARRIVED AT {} - FOLLOWING PLANET [!]", dest_name_upper);
                        } else {
                            **text = format!("[!] AUTOPILOT ENGAGED - TRANSIT TO {} [!]", dest_name_upper);
                        }
                    } else if child_idx == 1 {
                        if autopilot.arrived {
                            **text = "Press [SPACE] to Undock".to_string();
                        } else {
                            **text = "Press [SPACE] to Cancel Autopilot".to_string();
                        }
                    }
                    child_idx += 1;
                }
            }
        } else {
            *vis = Visibility::Hidden;
        }
    }

    let speed_str = if flight_state.boost_mode {
        format!("{:.0} km/s ({:.2}x c - FTL WARP BOOST ACTIVE)", speed, speed / speed_of_light)
    } else if flight_state.rapid_decel {
        format!("{:.0} km/s (RAPID BRAKING)", speed)
    } else if speed > speed_of_light {
        format!("{:.0} km/s ({:.2}x c - FTL)", speed, speed / speed_of_light)
    } else if speed > 1000.0 {
        let c_percent = (speed / speed_of_light * 100.0).min(99.9999);
        format!("{:.0} km/s ({:.4}% c)", speed, c_percent)
    } else {
        format!("{:.0} km/s", speed)
    };

    let mode = autopilot.mode();

    for mut text in &mut text_query {
        if autopilot.active {
            if let Some(destination_idx) = autopilot.destination_index {
                let mut dist_str = String::from("CALCULATING...");
                let mut dest_world_pos = None;

                if let Some((pos, _)) = crate::flight::get_celestial_target_info(
                    destination_idx,
                    autopilot.destination_name,
                    &sun_query,
                    &planet_query,
                    &moon_query,
                ) {
                    dest_world_pos = Some(pos);
                }

                if let Some(dest_pos) = dest_world_pos {
                    let dist = flight_state.world_pos.distance(dest_pos);
                    dist_str = format_dual_space_distance(dist);
                }

                let eta_str = if let Some(dest_pos) = dest_world_pos {
                    let dist_km = flight_state.world_pos.distance(dest_pos);
                    if speed < 0.1 || autopilot.arrived {
                        "N/A".to_string()
                    } else {
                        let eta_secs = dist_km / speed;
                        if !eta_secs.is_finite() {
                            "N/A".to_string()
                        } else if eta_secs < 60.0 {
                            format!("{:.0}s", eta_secs)
                        } else if eta_secs < 3600.0 {
                            format!("{}m {:.0}s", (eta_secs / 60.0) as u32, eta_secs % 60.0)
                        } else if eta_secs < 86400.0 {
                            format!("{}h {}m", (eta_secs / 3600.0) as u32, ((eta_secs % 3600.0) / 60.0) as u32)
                        } else if eta_secs < 86400.0 * 999.0 {
                            format!("{}d {}h", (eta_secs / 86400.0) as u32, ((eta_secs % 86400.0) / 3600.0) as u32)
                        } else {
                            ">999d".to_string()
                        }
                    }
                } else {
                    "N/A".to_string()
                };

                let status_label = match mode {
                    FlightControlMode::AutopilotArrived => {
                        "ARRIVED & FOLLOWING PLANET (PRESS [SPACE] TO UNDOCK)"
                    }
                    FlightControlMode::AutopilotTransit => {
                        if autopilot.current_waypoint.is_some() {
                            "EN ROUTE (BYPASSING OBSTACLE VIA PATH-FINDING)"
                        } else {
                            "EN ROUTE TO DESTINATION (APPROACHING SURFACE...)"
                        }
                    }
                    FlightControlMode::Manual => "MANUAL FLIGHT MODE",
                };

                **text = format!(
                    "AUTOPILOT: DESTINATION: {} | DISTANCE: {} | SPEED: {} | ETA: {} | STATUS: {}",
                    autopilot.destination_name.to_uppercase(),
                    dist_str,
                    speed_str,
                    eta_str,
                    status_label
                );
            }
        } else {
            let mode_hint = if flight_state.boost_mode {
                "BOOST MODE | PRESS SPACE AGAIN TO BRAKE QUICKLY"
            } else {
                "W/S: ACCEL/DECEL | MOUSE/ARROWS/A-D: PITCH/YAW | Q-E/Z-X: ROLL | SPACE: WARP BOOST | [M]: AUTOPILOT MENU | SPACE: CANCEL AP"
            };
            **text = format!(
                "FLIGHT STATUS: MANUAL CONTROL | SPEED: {} | {}",
                speed_str, mode_hint
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update_celestial_labels_system(
    flight_state: Res<FlightState>,
    camera_query: Query<(&Camera, &GlobalTransform), With<PilotCamera>>,
    sun_query: Query<(&Sun, &Transform)>,
    planet_query: Query<(&Planet, &Transform)>,
    moon_query: Query<(&Moon, &Transform)>,
    mut label_query: Query<(Entity, &CelestialLabel, &mut Node, &mut Visibility, &Children)>,
    mut text_query: Query<&mut Text>,
    window_query: Query<&Window>,
) {
    let Ok((camera, cam_global_transform)) = camera_query.single() else { return; };
    let Ok(window) = window_query.single() else { return; };

    let win_w = window.width();
    let win_h = window.height();

    struct VisibleLabelData {
        entity: Entity,
        viewport_pos: Vec2,
        real_dist: f32,
        label_width: f32,
        label_height: f32,
    }

    let mut visible_labels: Vec<VisibleLabelData> = Vec::new();

    // 1. Project celestial body 3D positions to 2D viewport coordinates and update text
    for (entity, label, _node, _vis, children) in &label_query {
        let (body_world_pos, rendered_transform_pos) = match label.destination_type {
            CelestialDestinationType::Sun => {
                let Ok((_sun, transform)) = sun_query.single() else { continue; };
                (Vec3::ZERO, transform.translation)
            }
            CelestialDestinationType::Planet(idx) => {
                let mut found = None;
                for (planet, transform) in &planet_query {
                    if planet.index == idx {
                        found = Some((planet.world_pos, transform.translation));
                        break;
                    }
                }
                let Some(data) = found else { continue; };
                data
            }
            CelestialDestinationType::Moon(mname) => {
                let mut found = None;
                for (moon, transform) in &moon_query {
                    if moon.name == mname {
                        found = Some((moon.world_pos, transform.translation));
                        break;
                    }
                }
                let Some(data) = found else { continue; };
                data
            }
        };

        let real_dist = flight_state.world_pos.distance(body_world_pos);

        if let Ok(viewport_pos) = camera.world_to_viewport(cam_global_transform, rendered_transform_pos) {
            let margin = 20.0;
            if viewport_pos.x >= margin
                && viewport_pos.y >= margin
                && viewport_pos.x <= win_w - margin
                && viewport_pos.y <= win_h - margin
            {
                let dist_str = format_dual_space_distance_compact(real_dist);

                let text_content = format!("{}{} {}", label.key_prefix, label.name.to_uppercase(), dist_str);

                for child in children.iter() {
                    if let Ok(mut text) = text_query.get_mut(child) {
                        **text = text_content.clone();
                    }
                }

                let label_width = (text_content.len() as f32 * 6.8 + 14.0).clamp(110.0, 260.0);
                let label_height = 20.0;

                visible_labels.push(VisibleLabelData {
                    entity,
                    viewport_pos,
                    real_dist,
                    label_width,
                    label_height,
                });
            }
        }
    }

    // 2. Sort visible labels by distance (closer bodies get priority default placement)
    visible_labels.sort_by(|a, b| a.real_dist.partial_cmp(&b.real_dist).unwrap_or(std::cmp::Ordering::Equal));

    // 3. Prevent overlaps using AABB collision avoidance
    struct LabelRect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    }

    impl LabelRect {
        fn intersects(&self, other: &LabelRect) -> bool {
            let pad = 4.0;
            self.x < other.x + other.w + pad
                && self.x + self.w + pad > other.x
                && self.y < other.y + other.h + pad
                && self.y + self.h + pad > other.y
        }
    }

    let mut placed_rects: Vec<LabelRect> = vec![
        // Top HUD banner area obstacle (reduced height)
        LabelRect { x: 10.0, y: 10.0, w: 780.0, h: 55.0 },
    ];

    let mut resolved_positions: std::collections::HashMap<Entity, Vec2> = std::collections::HashMap::new();

    for label_data in &visible_labels {
        let vx = label_data.viewport_pos.x;
        let vy = label_data.viewport_pos.y;
        let w = label_data.label_width;
        let h = label_data.label_height;

        let candidates = [
            (45.0, -35.0),
            (45.0, 15.0),
            (-(w + 15.0), -35.0),
            (-(w + 15.0), 15.0),
            (45.0, -60.0),
            (45.0, 40.0),
            (-(w + 15.0), -60.0),
            (-(w + 15.0), 40.0),
        ];

        let mut chosen_x = vx + 45.0;
        let mut chosen_y = vy - 35.0;
        let mut found_candidate = false;

        for (dx, dy) in candidates {
            let lx = (vx + dx).clamp(12.0, win_w - w - 12.0);
            let ly = (vy + dy).clamp(90.0, win_h - h - 12.0);
            let rect = LabelRect { x: lx, y: ly, w, h };

            if !placed_rects.iter().any(|r| r.intersects(&rect)) {
                chosen_x = lx;
                chosen_y = ly;
                found_candidate = true;
                break;
            }
        }

        if !found_candidate {
            let mut step = 1;
            while step <= 15 {
                let dy_up = -35.0 - (step as f32 * 24.0);
                let ly_up = (vy + dy_up).clamp(90.0, win_h - h - 12.0);
                let lx_right = (vx + 45.0).clamp(12.0, win_w - w - 12.0);
                let rect_up = LabelRect { x: lx_right, y: ly_up, w, h };

                if !placed_rects.iter().any(|r| r.intersects(&rect_up)) {
                    chosen_x = lx_right;
                    chosen_y = ly_up;
                    break;
                }

                let dy_down = 15.0 + (step as f32 * 24.0);
                let ly_down = (vy + dy_down).clamp(90.0, win_h - h - 12.0);
                let rect_down = LabelRect { x: lx_right, y: ly_down, w, h };

                if !placed_rects.iter().any(|r| r.intersects(&rect_down)) {
                    chosen_x = lx_right;
                    chosen_y = ly_down;
                    break;
                }

                let lx_left = (vx - (w + 15.0)).clamp(12.0, win_w - w - 12.0);
                let rect_left_up = LabelRect { x: lx_left, y: ly_up, w, h };

                if !placed_rects.iter().any(|r| r.intersects(&rect_left_up)) {
                    chosen_x = lx_left;
                    chosen_y = ly_up;
                    break;
                }

                step += 1;
            }
        }

        placed_rects.push(LabelRect {
            x: chosen_x,
            y: chosen_y,
            w,
            h,
        });

        resolved_positions.insert(label_data.entity, Vec2::new(chosen_x, chosen_y));
    }

    // 4. Update Node position and Visibility for all labels
    for (entity, _label, mut node, mut vis, _children) in &mut label_query {
        if let Some(&label_pos) = resolved_positions.get(&entity) {
            node.left = Val::Px(label_pos.x);
            node.top = Val::Px(label_pos.y);
            *vis = Visibility::Inherited;
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_dual_space_distance() {
        let earth_sun_dist = 149_597_870.7; // 1.00 AU
        let formatted = format_dual_space_distance(earth_sun_dist);
        assert!(formatted.contains("1.00 AU"), "Formatted distance should contain '1.00 AU' (got '{formatted}')");
        assert!(formatted.contains("149.6M km"), "Formatted distance should contain '(149.6M km)' (got '{formatted}')");

        let near_dist = 384_400.0; // Moon orbit distance
        let formatted_near = format_dual_space_distance(near_dist);
        assert!(formatted_near.contains("AU"), "Formatted distance should contain AU (got '{formatted_near}')");
        assert!(formatted_near.contains("384400 km"), "Formatted distance should contain '384400 km' (got '{formatted_near}')");
    }

    #[test]
    fn test_format_dual_space_distance_compact() {
        let earth_sun_dist = 149_597_870.7;
        let compact = format_dual_space_distance_compact(earth_sun_dist);
        assert!(compact.contains("1.00AU"), "Compact distance should contain '1.00AU' (got '{compact}')");
        assert!(compact.contains("150M km"), "Compact distance should contain '150M km' (got '{compact}')");
    }
}
