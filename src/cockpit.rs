use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

use crate::components::{
    AutoPilotHudText, CelestialLabel, CelestialTargetType, CockpitButton, CockpitButtonType, Moon,
    PilotCamera, Planet, RadarSweepNeedle, Sun,
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

    // Smooth continuous radar sweep needle rotation
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
    planet_query: Query<&Planet>,
    mut text_query: Query<&mut Text, With<AutoPilotHudText>>,
) {
    let speed = flight_state.velocity.length();
    let speed_of_light = 299_792.458;
    
    let speed_str = if speed > speed_of_light {
        format!("{:.0} km/s ({:.2}x c - FTL WARP)", speed, speed / speed_of_light)
    } else if speed > 1000.0 {
        let c_percent = (speed / speed_of_light * 100.0).min(99.9999);
        format!("{:.0} km/s ({:.4}% c)", speed, c_percent)
    } else {
        format!("{:.0} km/s", speed)
    };

    for mut text in &mut text_query {
        if autopilot.active {
            if let Some(target_idx) = autopilot.target_index {
                let mut dist_str = String::from("CALCULATING...");

                if target_idx == 0 {
                    let dist = flight_state.world_pos.distance(Vec3::ZERO);
                    dist_str = format!("{:.0} km", dist * 10.0);
                } else {
                    for planet in &planet_query {
                        if planet.index == target_idx {
                            let dist = flight_state.world_pos.distance(planet.world_pos);
                            dist_str = format!("{:.0} km", dist * 10.0);
                            break;
                        }
                    }
                }

                let status_label = if autopilot.arrived {
                    "PARKING ORBIT REACHED"
                } else {
                    "EN ROUTE"
                };

                **text = format!(
                    "AUTOPILOT: [{}] TARGET: {} | DISTANCE: {} | SPEED: {} | STATUS: {}",
                    target_idx,
                    autopilot.target_name.to_uppercase(),
                    dist_str,
                    speed_str,
                    status_label
                );
            }
        } else {
            **text = format!(
                "FLIGHT STATUS: MANUAL CONTROL | SPEED: {} | PRESS [0-9] TO ENGAGE AUTOPILOT",
                speed_str
            );
        }
    }
}

pub fn update_celestial_labels_system(
    flight_state: Res<FlightState>,
    camera_query: Query<(&Camera, &GlobalTransform), With<PilotCamera>>,
    sun_query: Query<(&Sun, &Transform)>,
    planet_query: Query<(&Planet, &Transform)>,
    moon_query: Query<(&Moon, &Transform)>,
    mut label_query: Query<(Entity, &CelestialLabel, &mut Node, &mut Visibility, &Children)>,
    mut text_query: Query<&mut Text>,
    mut gizmos: Gizmos,
    window_query: Query<&Window>,
) {
    let Ok((camera, cam_global_transform)) = camera_query.single() else { return; };
    let Ok(window) = window_query.single() else { return; };

    let win_w = window.width();
    let win_h = window.height();
    let half_w = win_w * 0.5;
    let half_h = win_h * 0.5;

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
        let (body_world_pos, rendered_transform_pos) = match label.target_type {
            CelestialTargetType::Sun => {
                let Ok((_sun, transform)) = sun_query.single() else { continue; };
                (Vec3::ZERO, transform.translation)
            }
            CelestialTargetType::Planet(idx) => {
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
            CelestialTargetType::Moon(mname) => {
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
                let dist_str = if real_dist > 100_000.0 {
                    format!("{:.2}M km", (real_dist * 10.0) / 1_000_000.0)
                } else {
                    format!("{:.0} km", real_dist * 10.0)
                };

                let text_content = format!("{}{} {}", label.key_prefix, label.name.to_uppercase(), dist_str);

                for child in children.iter() {
                    if let Ok(mut text) = text_query.get_mut(child) {
                        **text = text_content.clone();
                    }
                }

                let label_width = (text_content.len() as f32 * 6.8 + 14.0).clamp(95.0, 220.0);
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
        // Top HUD banner area obstacle
        LabelRect { x: 10.0, y: 10.0, w: 780.0, h: 80.0 },
    ];

    let mut resolved_positions: std::collections::HashMap<Entity, (Vec2, Vec2, f32)> = std::collections::HashMap::new();

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

        resolved_positions.insert(label_data.entity, (label_data.viewport_pos, Vec2::new(chosen_x, chosen_y), w));
    }

    // 4. Update Node position and Visibility, draw leader lines for all labels
    for (entity, _label, mut node, mut vis, _children) in &mut label_query {
        if let Some(&(viewport_pos, label_pos, label_w)) = resolved_positions.get(&entity) {
            node.left = Val::Px(label_pos.x);
            node.top = Val::Px(label_pos.y);
            *vis = Visibility::Inherited;

            let start_2d = Vec2::new(viewport_pos.x - half_w, half_h - viewport_pos.y);
            let line_end_x = if label_pos.x > viewport_pos.x {
                label_pos.x - 4.0
            } else {
                label_pos.x + label_w + 4.0
            };
            let end_2d = Vec2::new(line_end_x - half_w, half_h - (label_pos.y + 6.0));
            gizmos.line_2d(start_2d, end_2d, Color::srgba(0.0, 0.75, 0.85, 0.35));
        } else {
            *vis = Visibility::Hidden;
        }
    }
}
