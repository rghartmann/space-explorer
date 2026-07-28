use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::render::mesh::SphereKind;
use bevy::window::WindowMode;

// --- Components & Resources ---

#[derive(Component)]
struct Ship;

#[derive(Component)]
struct EngineSound;

#[derive(Component)]
struct AmbientMusic;

#[derive(Component)]
struct PilotCamera;

#[derive(Component)]
struct Sun {
    radius: f32,
}

#[derive(Component)]
struct Planet {
    _name: &'static str,
    index: usize, // 1 to 8
    radius: f32,
    orbit_radius: f32,
    orbit_speed: f32,
    rotation_speed: f32,
    world_pos: Vec3,
}

#[derive(Component)]
struct Moon {
    _name: &'static str,
    parent_index: usize,
    radius: f32,
    orbit_radius: f32,
    orbit_speed: f32,
    rotation_speed: f32,
    world_pos: Vec3,
}

#[derive(Component)]
struct Starfield {
    world_pos: Vec3,
}

#[derive(Component)]
struct RadarSweepNeedle;

#[derive(Component)]
struct AutoPilotHudText;

#[derive(Component, PartialEq, Eq, Clone, Copy)]
enum CockpitButtonType {
    Thruster,
    Warp,
    Shields,
    AutoNav,
    Alert,
}

#[derive(Component)]
struct CockpitButton {
    button_type: CockpitButtonType,
    base_emissive: LinearRgba,
    active_emissive: LinearRgba,
}

#[derive(Resource)]
struct FlightState {
    velocity: Vec3,
    angular_velocity: Vec2,
    yaw: f32,   // Pilot look yaw
    pitch: f32, // Pilot look pitch
}

impl Default for FlightState {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            angular_velocity: Vec2::ZERO,
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

#[derive(Resource)]
struct AutoPilotState {
    active: bool,
    target_index: Option<usize>,
    target_name: &'static str,
    arrived: bool,
}

impl Default for AutoPilotState {
    fn default() -> Self {
        Self {
            active: false,
            target_index: None,
            target_name: "",
            arrived: false,
        }
    }
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.001, 0.001, 0.003)))
        .init_resource::<FlightState>()
        .init_resource::<AutoPilotState>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Space Explorer - Solar System Exploration & Auto-Pilot".into(),
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                exit_on_esc,
                pilot_freelook_system,
                ship_flight_system,
                autopilot_input_system,
                autopilot_flight_system,
                celestial_collision_system,
            ),
        )
        .add_systems(
            Update,
            (
                orbit_planets_system,
                orbit_moons_system,
                logarithmic_distance_render_system,
                engine_sound_system,
                animate_cockpit_screens_system,
                animate_cockpit_buttons_system,
                update_hud_system,
            ),
        )
        .run();
}

// ----------------------------------------------------
// PROCEDURAL AUDIO SYNTHESIZERS (WAV GENERATION)
// ----------------------------------------------------

fn generate_engine_hum_wav() -> Vec<u8> {
    let sample_rate = 44100;
    let duration_secs = 4.0;
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut pcm = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sub_bass = (2.0 * std::f32::consts::PI * 55.0 * t).sin();
        let mid_drone = (2.0 * std::f32::consts::PI * 110.0 * t).sin() * 0.4;
        let harmonic = (2.0 * std::f32::consts::PI * 165.0 * t).sin() * 0.15;
        let lfo = (2.0 * std::f32::consts::PI * 0.5 * t).sin() * 0.15 + 0.85;

        let sample = (sub_bass + mid_drone + harmonic) * 0.35 * lfo;
        let val = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
        pcm.push(val);
    }

    let data_len = (pcm.len() * 2) as u32;
    let file_len = 36 + data_len;
    let mut wav = Vec::with_capacity(44 + pcm.len() * 2);

    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_len.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&(sample_rate as u32).to_le_bytes());
    wav.extend_from_slice(&((sample_rate * 2) as u32).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());

    for sample in pcm {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}

fn ensure_engine_hum_file() {
    let dir = std::path::Path::new("assets/audio");
    if !dir.exists() {
        let _ = std::fs::create_dir_all(dir);
    }
    let file_path = dir.join("engine_hum.wav");
    if !file_path.exists() {
        let wav_data = generate_engine_hum_wav();
        let _ = std::fs::write(file_path, wav_data);
    }
}

fn generate_ambient_piano_wav() -> Vec<u8> {
    let sample_rate = 44100;
    let duration_secs = 12.0;
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut pcm = Vec::with_capacity(num_samples);

    let freqs = [130.81, 164.81, 196.00, 246.94, 293.66, 329.63, 392.00];

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let mut sample = 0.0;

        for (idx, &freq) in freqs.iter().enumerate() {
            let note_start = idx as f32 * 1.6;
            let note_time = t - note_start;

            if note_time > 0.0 {
                let env = (-note_time * 0.75).exp() * (1.0 - (-note_time * 25.0).exp());
                let fundamental = (2.0 * std::f32::consts::PI * freq * note_time).sin();
                let harmonic = (2.0 * std::f32::consts::PI * (freq * 2.0) * note_time).sin() * 0.25;
                sample += (fundamental + harmonic) * env * 0.18;
            }
        }

        let val = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
        pcm.push(val);
    }

    let data_len = (pcm.len() * 2) as u32;
    let file_len = 36 + data_len;
    let mut wav = Vec::with_capacity(44 + pcm.len() * 2);

    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_len.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&(sample_rate as u32).to_le_bytes());
    wav.extend_from_slice(&((sample_rate * 2) as u32).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());

    for sample in pcm {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}

fn ensure_ambient_piano_file() {
    let dir = std::path::Path::new("assets/audio");
    if !dir.exists() {
        let _ = std::fs::create_dir_all(dir);
    }
    let file_path = dir.join("ambient_piano.wav");
    if !file_path.exists() {
        let wav_data = generate_ambient_piano_wav();
        let _ = std::fs::write(file_path, wav_data);
    }
}

// ----------------------------------------------------
// SCENE SETUP
// ----------------------------------------------------

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // 2D HUD CAMERA & OVERLAY UI
    commands.spawn(Camera2d::default());

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(16.0),
                left: Val::Px(16.0),
                right: Val::Px(16.0),
                padding: UiRect::all(Val::Px(12.0)),
                border: UiRect::all(Val::Px(1.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.04, 0.09, 0.85)),
            BorderColor::all(Color::srgba(0.0, 0.8, 1.0, 0.5)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("AUTOPILOT DESTINATION SELECT: [1] Mercury | [2] Venus | [3] Earth | [4] Mars | [5] Jupiter | [6] Saturn | [7] Uranus | [8] Neptune"),
                TextFont {
                    font_size: 13.5.into(),
                    ..default()
                },
                TextColor(Color::srgb(0.0, 0.88, 1.0)),
            ));

            parent.spawn((
                Text::new("FLIGHT CONTROLS: WASD (Thrust) | SHIFT (High-Speed Boost) | Q/E (Steer Yaw) | Mouse / IJKL / Arrows (Freelook) | ESC (Exit)"),
                TextFont {
                    font_size: 12.0.into(),
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.8, 0.9)),
            ));

            parent.spawn((
                AutoPilotHudText,
                Text::new("FLIGHT STATUS: MANUAL CONTROL | SPEED: 0 km/s | PRESS [1-8] TO ENGAGE AUTOPILOT"),
                TextFont {
                    font_size: 13.0.into(),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.85, 0.2)),
            ));
        });

    // AUDIO ENGINE STARTUP
    ensure_engine_hum_file();
    ensure_ambient_piano_file();

    let engine_hum_handle: Handle<AudioSource> = asset_server.load("audio/engine_hum.wav");
    commands.spawn((
        EngineSound,
        AudioPlayer(engine_hum_handle),
        PlaybackSettings::LOOP,
    ));

    let ambient_piano_handle: Handle<AudioSource> = asset_server.load("audio/ambient_piano.wav");
    commands.spawn((
        AmbientMusic,
        AudioPlayer(ambient_piano_handle),
        PlaybackSettings::LOOP.with_volume(bevy::audio::Volume::Linear(0.35)),
    ));

    // ----------------------------------------------------
    // SHIP & FIRST-PERSON PANORAMIC COCKPIT PERSPECTIVE
    // ----------------------------------------------------
    let ship_entity = commands
        .spawn((
            Ship,
            Transform::from_xyz(240_000.0 + 3_500.0, 400.0, 1_200.0),
            Visibility::default(),
        ))
        .id();

    let camera_entity = commands
        .spawn((
            PilotCamera,
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                far: 100_000.0,
                ..default()
            }),
            Transform::from_xyz(0.0, 0.5, 0.0),
        ))
        .id();

    commands.entity(ship_entity).add_child(camera_entity);

    // Cockpit Ambient Light
    let cockpit_light = commands
        .spawn((
            PointLight {
                intensity: 30.0,
                color: Color::srgb(0.3, 0.7, 1.0),
                range: 3.5,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_xyz(0.0, 0.3, -0.4),
        ))
        .id();
    commands.entity(camera_entity).add_child(cockpit_light);

    // ----------------------------------------------------
    // COCKPIT DASHBOARD & CONSOLE FRAMEWORK (MINI-SCALE FOR PANORAMIC WINDOW)
    // ----------------------------------------------------
    let console_mesh = meshes.add(Cuboid::from_size(Vec3::new(1.1, 0.18, 0.45)));
    let console_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.04, 0.05, 0.08),
        metallic: 0.95,
        perceptual_roughness: 0.2,
        ..default()
    });

    let console_entity = commands
        .spawn((
            Mesh3d(console_mesh),
            MeshMaterial3d(console_mat),
            Transform::from_xyz(0.0, -0.48, -0.65)
                .with_rotation(Quat::from_rotation_x(0.35)),
        ))
        .id();
    commands.entity(camera_entity).add_child(console_entity);

    // Scaled-down Control Panel Inset
    let panel_texture: Handle<Image> = asset_server.load("textures/control_panel.jpg");
    let panel_mesh = meshes.add(Cuboid::from_size(Vec3::new(0.32, 0.01, 0.32)));
    let panel_mat = materials.add(StandardMaterial {
        base_color_texture: Some(panel_texture.clone()),
        emissive_texture: Some(panel_texture),
        emissive: LinearRgba::new(0.35, 0.35, 0.35, 1.0),
        metallic: 0.8,
        perceptual_roughness: 0.25,
        ..default()
    });

    let center_panel = commands
        .spawn((
            Mesh3d(panel_mesh),
            MeshMaterial3d(panel_mat),
            Transform::from_xyz(0.0, -0.38, -0.62)
                .with_rotation(Quat::from_rotation_x(0.35)),
        ))
        .id();
    commands.entity(camera_entity).add_child(center_panel);

    // Screens Frame Material
    let screen_frame_mesh = meshes.add(Cuboid::from_size(Vec3::new(0.28, 0.20, 0.02)));
    let screen_frame_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.08, 0.09, 0.12),
        metallic: 0.9,
        perceptual_roughness: 0.3,
        ..default()
    });

    // LEFT SCREEN: TACTICAL NAV RADAR (STEADY EMISSIVE DISPLAY)
    let left_frame = commands
        .spawn((
            Mesh3d(screen_frame_mesh.clone()),
            MeshMaterial3d(screen_frame_mat.clone()),
            Transform::from_xyz(-0.38, -0.34, -0.64)
                .with_rotation(Quat::from_rotation_y(0.25)),
        ))
        .id();
    commands.entity(camera_entity).add_child(left_frame);

    let nav_texture: Handle<Image> = asset_server.load("textures/nav_screen.jpg");
    let nav_screen_mesh = meshes.add(Cuboid::from_size(Vec3::new(0.26, 0.18, 0.005)));
    let nav_screen_mat = materials.add(StandardMaterial {
        base_color_texture: Some(nav_texture.clone()),
        emissive_texture: Some(nav_texture),
        emissive: LinearRgba::new(0.5, 1.2, 1.6, 1.0),
        perceptual_roughness: 0.1,
        metallic: 0.1,
        ..default()
    });

    let nav_screen = commands
        .spawn((
            Mesh3d(nav_screen_mesh),
            MeshMaterial3d(nav_screen_mat),
            Transform::from_xyz(-0.38, -0.34, -0.625)
                .with_rotation(Quat::from_rotation_y(0.25)),
        ))
        .id();
    commands.entity(camera_entity).add_child(nav_screen);

    // Radar Sweep Needle
    let needle_mesh = meshes.add(Cuboid::from_size(Vec3::new(0.11, 0.002, 0.003)));
    let needle_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 1.0, 0.9),
        emissive: LinearRgba::new(0.0, 2.0, 2.5, 1.0),
        unlit: true,
        ..default()
    });

    let radar_needle = commands
        .spawn((
            RadarSweepNeedle,
            Mesh3d(needle_mesh),
            MeshMaterial3d(needle_mat),
            Transform::from_xyz(-0.38, -0.34, -0.62)
                .with_rotation(Quat::from_rotation_y(0.25)),
        ))
        .id();
    commands.entity(camera_entity).add_child(radar_needle);

    // RIGHT SCREEN: SHIP DIAGNOSTICS (STEADY EMISSIVE DISPLAY)
    let right_frame = commands
        .spawn((
            Mesh3d(screen_frame_mesh),
            MeshMaterial3d(screen_frame_mat.clone()),
            Transform::from_xyz(0.38, -0.34, -0.64)
                .with_rotation(Quat::from_rotation_y(-0.25)),
        ))
        .id();
    commands.entity(camera_entity).add_child(right_frame);

    let diag_texture: Handle<Image> = asset_server.load("textures/diag_screen.jpg");
    let diag_screen_mesh = meshes.add(Cuboid::from_size(Vec3::new(0.26, 0.18, 0.005)));
    let diag_screen_mat = materials.add(StandardMaterial {
        base_color_texture: Some(diag_texture.clone()),
        emissive_texture: Some(diag_texture),
        emissive: LinearRgba::new(1.4, 0.8, 0.3, 1.0),
        perceptual_roughness: 0.1,
        metallic: 0.1,
        ..default()
    });

    let diag_screen = commands
        .spawn((
            Mesh3d(diag_screen_mesh),
            MeshMaterial3d(diag_screen_mat),
            Transform::from_xyz(0.38, -0.34, -0.625)
                .with_rotation(Quat::from_rotation_y(-0.25)),
        ))
        .id();
    commands.entity(camera_entity).add_child(diag_screen);

    // COCKPIT BUTTONS (STEADY, NON-STROBING INDICATORS)
    let button_cap_mesh = meshes.add(Cuboid::from_size(Vec3::new(0.038, 0.015, 0.038)));
    let button_base_mesh = meshes.add(Cuboid::from_size(Vec3::new(0.045, 0.008, 0.045)));
    let metal_frame_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.14, 0.18),
        metallic: 0.95,
        perceptual_roughness: 0.15,
        ..default()
    });

    // 1. Thruster Button
    let btn_thruster_tex: Handle<Image> = asset_server.load("textures/button_thruster.jpg");
    let btn_thruster_mat = materials.add(StandardMaterial {
        base_color_texture: Some(btn_thruster_tex.clone()),
        emissive_texture: Some(btn_thruster_tex),
        emissive: LinearRgba::new(0.0, 0.8, 1.0, 1.0),
        perceptual_roughness: 0.2,
        metallic: 0.4,
        ..default()
    });

    let b1_base = commands
        .spawn((
            Mesh3d(button_base_mesh.clone()),
            MeshMaterial3d(metal_frame_mat.clone()),
            Transform::from_xyz(-0.12, -0.37, -0.61)
                .with_rotation(Quat::from_rotation_x(0.35)),
        ))
        .id();
    commands.entity(camera_entity).add_child(b1_base);

    let b1_cap = commands
        .spawn((
            CockpitButton {
                button_type: CockpitButtonType::Thruster,
                base_emissive: LinearRgba::new(0.0, 0.8, 1.0, 1.0),
                active_emissive: LinearRgba::new(0.0, 1.3, 1.6, 1.0),
            },
            Mesh3d(button_cap_mesh.clone()),
            MeshMaterial3d(btn_thruster_mat),
            Transform::from_xyz(-0.12, -0.36, -0.61)
                .with_rotation(Quat::from_rotation_x(0.35)),
        ))
        .id();
    commands.entity(camera_entity).add_child(b1_cap);

    // 2. Warp Button
    let btn_warp_tex: Handle<Image> = asset_server.load("textures/button_warp.jpg");
    let btn_warp_mat = materials.add(StandardMaterial {
        base_color_texture: Some(btn_warp_tex.clone()),
        emissive_texture: Some(btn_warp_tex),
        emissive: LinearRgba::new(1.0, 0.5, 0.0, 1.0),
        perceptual_roughness: 0.2,
        metallic: 0.4,
        ..default()
    });

    let b2_base = commands
        .spawn((
            Mesh3d(button_base_mesh.clone()),
            MeshMaterial3d(metal_frame_mat.clone()),
            Transform::from_xyz(0.12, -0.37, -0.61)
                .with_rotation(Quat::from_rotation_x(0.35)),
        ))
        .id();
    commands.entity(camera_entity).add_child(b2_base);

    let b2_cap = commands
        .spawn((
            CockpitButton {
                button_type: CockpitButtonType::Warp,
                base_emissive: LinearRgba::new(1.0, 0.5, 0.0, 1.0),
                active_emissive: LinearRgba::new(1.5, 0.8, 0.0, 1.0),
            },
            Mesh3d(button_cap_mesh.clone()),
            MeshMaterial3d(btn_warp_mat),
            Transform::from_xyz(0.12, -0.36, -0.61)
                .with_rotation(Quat::from_rotation_x(0.35)),
        ))
        .id();
    commands.entity(camera_entity).add_child(b2_cap);

    // 3. Shield Generator Button
    let btn_shield_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.8, 0.3),
        emissive: LinearRgba::new(0.0, 1.0, 0.3, 1.0),
        perceptual_roughness: 0.2,
        metallic: 0.3,
        ..default()
    });

    let b3_base = commands
        .spawn((
            Mesh3d(button_base_mesh.clone()),
            MeshMaterial3d(metal_frame_mat.clone()),
            Transform::from_xyz(-0.07, -0.37, -0.61)
                .with_rotation(Quat::from_rotation_x(0.35)),
        ))
        .id();
    commands.entity(camera_entity).add_child(b3_base);

    let b3_cap = commands
        .spawn((
            CockpitButton {
                button_type: CockpitButtonType::Shields,
                base_emissive: LinearRgba::new(0.0, 1.0, 0.3, 1.0),
                active_emissive: LinearRgba::new(0.0, 1.4, 0.4, 1.0),
            },
            Mesh3d(button_cap_mesh.clone()),
            MeshMaterial3d(btn_shield_mat),
            Transform::from_xyz(-0.07, -0.36, -0.61)
                .with_rotation(Quat::from_rotation_x(0.35)),
        ))
        .id();
    commands.entity(camera_entity).add_child(b3_cap);

    // 4. Auto-Nav Steering Button
    let btn_autonav_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.5, 0.9),
        emissive: LinearRgba::new(0.0, 0.6, 1.4, 1.0),
        perceptual_roughness: 0.2,
        metallic: 0.3,
        ..default()
    });

    let b4_base = commands
        .spawn((
            Mesh3d(button_base_mesh.clone()),
            MeshMaterial3d(metal_frame_mat.clone()),
            Transform::from_xyz(0.07, -0.37, -0.61)
                .with_rotation(Quat::from_rotation_x(0.35)),
        ))
        .id();
    commands.entity(camera_entity).add_child(b4_base);

    let b4_cap = commands
        .spawn((
            CockpitButton {
                button_type: CockpitButtonType::AutoNav,
                base_emissive: LinearRgba::new(0.0, 0.6, 1.4, 1.0),
                active_emissive: LinearRgba::new(0.0, 1.0, 2.0, 1.0),
            },
            Mesh3d(button_cap_mesh.clone()),
            MeshMaterial3d(btn_autonav_mat),
            Transform::from_xyz(0.07, -0.36, -0.61)
                .with_rotation(Quat::from_rotation_x(0.35)),
        ))
        .id();
    commands.entity(camera_entity).add_child(b4_cap);

    // 5. Alert Warning Button
    let btn_alert_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.1, 0.1),
        emissive: LinearRgba::new(0.9, 0.1, 0.1, 1.0),
        perceptual_roughness: 0.2,
        metallic: 0.3,
        ..default()
    });

    let b5_base = commands
        .spawn((
            Mesh3d(button_base_mesh),
            MeshMaterial3d(metal_frame_mat.clone()),
            Transform::from_xyz(0.16, -0.37, -0.61)
                .with_rotation(Quat::from_rotation_x(0.35)),
        ))
        .id();
    commands.entity(camera_entity).add_child(b5_base);

    let b5_cap = commands
        .spawn((
            CockpitButton {
                button_type: CockpitButtonType::Alert,
                base_emissive: LinearRgba::new(0.9, 0.1, 0.1, 1.0),
                active_emissive: LinearRgba::new(1.4, 0.2, 0.2, 1.0),
            },
            Mesh3d(button_cap_mesh),
            MeshMaterial3d(btn_alert_mat),
            Transform::from_xyz(0.16, -0.36, -0.61)
                .with_rotation(Quat::from_rotation_x(0.35)),
        ))
        .id();
    commands.entity(camera_entity).add_child(b5_cap);

    // Toggle Switches & LEDs
    let toggle_base_mesh = meshes.add(Cylinder::new(0.007, 0.01));
    let toggle_pin_mesh = meshes.add(Cylinder::new(0.0025, 0.03));
    let led_bead_mesh = meshes.add(Sphere::new(0.004));

    let chrome_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.88, 0.92),
        metallic: 1.0,
        perceptual_roughness: 0.05,
        ..default()
    });

    let led_colors = [
        LinearRgba::new(0.0, 1.4, 0.3, 1.0),
        LinearRgba::new(0.0, 1.0, 1.6, 1.0),
        LinearRgba::new(1.4, 0.8, 0.0, 1.0),
        LinearRgba::new(1.6, 0.1, 0.1, 1.0),
    ];

    for (i, &led_color) in led_colors.iter().enumerate() {
        let x_pos = -0.16 + (i as f32) * 0.03;

        let t_base = commands
            .spawn((
                Mesh3d(toggle_base_mesh.clone()),
                MeshMaterial3d(metal_frame_mat.clone()),
                Transform::from_xyz(x_pos, -0.37, -0.58)
                    .with_rotation(Quat::from_rotation_x(0.35)),
            ))
            .id();
        commands.entity(camera_entity).add_child(t_base);

        let angle = if i % 2 == 0 { 0.35 } else { -0.35 };
        let t_pin = commands
            .spawn((
                Mesh3d(toggle_pin_mesh.clone()),
                MeshMaterial3d(chrome_mat.clone()),
                Transform::from_xyz(x_pos, -0.36, -0.58)
                    .with_rotation(Quat::from_rotation_z(angle) * Quat::from_rotation_x(0.35)),
            ))
            .id();
        commands.entity(camera_entity).add_child(t_pin);

        let led_mat = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            emissive: led_color,
            unlit: true,
            ..default()
        });

        let led = commands
            .spawn((
                Mesh3d(led_bead_mesh.clone()),
                MeshMaterial3d(led_mat),
                Transform::from_xyz(x_pos, -0.36, -0.59),
            ))
            .id();
        commands.entity(camera_entity).add_child(led);
    }

    // PANORAMIC OBSERVATION BAY CANOPY STRUTS (THIN PERIPHERAL FRAMING)
    let strut_mesh = meshes.add(Cuboid::from_size(Vec3::new(0.02, 1.6, 0.02)));
    let strut_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.08, 0.1, 0.14),
        metallic: 0.95,
        perceptual_roughness: 0.15,
        ..default()
    });

    let left_strut = commands
        .spawn((
            Mesh3d(strut_mesh.clone()),
            MeshMaterial3d(strut_mat.clone()),
            Transform::from_xyz(-1.15, 0.1, -0.5)
                .with_rotation(Quat::from_rotation_z(-0.38)),
        ))
        .id();

    let right_strut = commands
        .spawn((
            Mesh3d(strut_mesh),
            MeshMaterial3d(strut_mat),
            Transform::from_xyz(1.15, 0.1, -0.5)
                .with_rotation(Quat::from_rotation_z(0.38)),
        ))
        .id();

    commands.entity(camera_entity).add_child(left_strut);
    commands.entity(camera_entity).add_child(right_strut);

    let hud_ring_mesh = meshes.add(Torus { minor_radius: 0.002, major_radius: 0.045 });
    let hud_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 0.95, 1.0, 0.85),
        emissive: LinearRgba::new(0.0, 1.8, 2.2, 1.0),
        unlit: true,
        ..default()
    });

    let hud_ring = commands
        .spawn((
            Mesh3d(hud_ring_mesh),
            MeshMaterial3d(hud_mat),
            Transform::from_xyz(0.0, 0.0, -1.0)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        ))
        .id();
    commands.entity(camera_entity).add_child(hud_ring);

    // ----------------------------------------------------
    // 2. THE SUN (UNLIT HIGH-RESOLUTION SURFACE TEXTURE)
    // ----------------------------------------------------
    let sun_mesh = meshes.add(Sphere::new(120.0).mesh().kind(SphereKind::Uv { sectors: 40, stacks: 20 }));
    let sun_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.85, 0.35),
        emissive: LinearRgba::new(25.0, 18.0, 4.0, 1.0),
        unlit: true,
        ..default()
    });

    commands.spawn((
        Sun { radius: 120.0 },
        Mesh3d(sun_mesh),
        MeshMaterial3d(sun_mat),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // Sun Core Sunlight (PointLight)
    commands.spawn((
        PointLight {
            intensity: 8_000_000.0,
            color: Color::srgb(1.0, 0.96, 0.88),
            range: 10_000_000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // ----------------------------------------------------
    // 3. EIGHT PLANETS & MOONS IN EXPANDED ORBITS
    // ----------------------------------------------------

    // --- PLANET 1: MERCURY ---
    let mercury_mesh = meshes.add(Sphere::new(4.5).mesh().kind(SphereKind::Uv { sectors: 24, stacks: 12 }));
    let mercury_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.52, 0.50),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        Planet {
            _name: "Mercury",
            index: 1,
            radius: 4.5,
            orbit_radius: 85_000.0,
            orbit_speed: 0.15,
            rotation_speed: 0.05,
            world_pos: Vec3::new(85_000.0, 0.0, 0.0),
        },
        Mesh3d(mercury_mesh),
        MeshMaterial3d(mercury_mat),
        Transform::from_xyz(85_000.0, 0.0, 0.0),
    ));

    // --- PLANET 2: VENUS ---
    let venus_mesh = meshes.add(Sphere::new(11.0).mesh().kind(SphereKind::Uv { sectors: 32, stacks: 16 }));
    let venus_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.78, 0.55),
        perceptual_roughness: 0.4,
        ..default()
    });
    commands.spawn((
        Planet {
            _name: "Venus",
            index: 2,
            radius: 11.0,
            orbit_radius: 150_000.0,
            orbit_speed: 0.11,
            rotation_speed: 0.02,
            world_pos: Vec3::new(150_000.0, 0.0, 0.0),
        },
        Mesh3d(venus_mesh),
        MeshMaterial3d(venus_mat),
        Transform::from_xyz(150_000.0, 0.0, 0.0),
    ));

    // --- PLANET 3: EARTH & MOON ---
    let earth_mesh = meshes.add(Sphere::new(12.0).mesh().kind(SphereKind::Uv { sectors: 32, stacks: 16 }));
    let earth_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.38, 0.78),
        perceptual_roughness: 0.35,
        metallic: 0.1,
        ..default()
    });

    let earth_entity = commands
        .spawn((
            Planet {
                _name: "Earth",
                index: 3,
                radius: 12.0,
                orbit_radius: 240_000.0,
                orbit_speed: 0.08,
                rotation_speed: 0.4,
                world_pos: Vec3::new(240_000.0, 0.0, 0.0),
            },
            Mesh3d(earth_mesh),
            MeshMaterial3d(earth_mat),
            Transform::from_xyz(240_000.0, 0.0, 0.0),
        ))
        .id();

    // Earth's Moon
    let moon_mesh = meshes.add(Sphere::new(3.2).mesh().kind(SphereKind::Uv { sectors: 24, stacks: 12 }));
    let moon_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.68, 0.68, 0.65),
        perceptual_roughness: 0.85,
        ..default()
    });
    let moon_entity = commands
        .spawn((
            Moon {
                _name: "Moon",
                parent_index: 3,
                radius: 3.2,
                orbit_radius: 65.0,
                orbit_speed: 0.25,
                rotation_speed: 0.1,
                world_pos: Vec3::new(240_000.0 + 65.0, 0.0, 0.0),
            },
            Mesh3d(moon_mesh),
            MeshMaterial3d(moon_mat),
            Transform::from_xyz(65.0, 0.0, 0.0),
        ))
        .id();
    commands.entity(earth_entity).add_child(moon_entity);

    // --- PLANET 4: MARS ---
    let mars_mesh = meshes.add(Sphere::new(6.8).mesh().kind(SphereKind::Uv { sectors: 32, stacks: 16 }));
    let mars_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.32, 0.15),
        perceptual_roughness: 0.75,
        ..default()
    });
    commands.spawn((
        Planet {
            _name: "Mars",
            index: 4,
            radius: 6.8,
            orbit_radius: 380_000.0,
            orbit_speed: 0.06,
            rotation_speed: 0.38,
            world_pos: Vec3::new(380_000.0, 0.0, 0.0),
        },
        Mesh3d(mars_mesh),
        MeshMaterial3d(mars_mat),
        Transform::from_xyz(380_000.0, 0.0, 0.0),
    ));

    // --- PLANET 5: JUPITER & GALILEAN MOONS ---
    let jupiter_mesh = meshes.add(Sphere::new(55.0).mesh().kind(SphereKind::Uv { sectors: 40, stacks: 20 }));
    let jupiter_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.82, 0.64, 0.45),
        perceptual_roughness: 0.5,
        ..default()
    });
    let jupiter_entity = commands
        .spawn((
            Planet {
                _name: "Jupiter",
                index: 5,
                radius: 55.0,
                orbit_radius: 950_000.0,
                orbit_speed: 0.035,
                rotation_speed: 0.8,
                world_pos: Vec3::new(950_000.0, 0.0, 0.0),
            },
            Mesh3d(jupiter_mesh),
            MeshMaterial3d(jupiter_mat),
            Transform::from_xyz(950_000.0, 0.0, 0.0),
        ))
        .id();

    // Io
    let io_mesh = meshes.add(Sphere::new(3.0).mesh().kind(SphereKind::Uv { sectors: 20, stacks: 10 }));
    let io_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.85, 0.2),
        perceptual_roughness: 0.7,
        ..default()
    });
    let io_entity = commands
        .spawn((
            Moon {
                _name: "Io",
                parent_index: 5,
                radius: 3.0,
                orbit_radius: 110.0,
                orbit_speed: 0.45,
                rotation_speed: 0.3,
                world_pos: Vec3::new(950_000.0 + 110.0, 0.0, 0.0),
            },
            Mesh3d(io_mesh),
            MeshMaterial3d(io_mat),
            Transform::from_xyz(110.0, 0.0, 0.0),
        ))
        .id();
    commands.entity(jupiter_entity).add_child(io_entity);

    // Europa
    let europa_mesh = meshes.add(Sphere::new(2.6).mesh().kind(SphereKind::Uv { sectors: 20, stacks: 10 }));
    let europa_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.88, 0.95),
        perceptual_roughness: 0.2,
        metallic: 0.1,
        ..default()
    });
    let europa_entity = commands
        .spawn((
            Moon {
                _name: "Europa",
                parent_index: 5,
                radius: 2.6,
                orbit_radius: 160.0,
                orbit_speed: 0.32,
                rotation_speed: 0.25,
                world_pos: Vec3::new(950_000.0 + 160.0, 0.0, 0.0),
            },
            Mesh3d(europa_mesh),
            MeshMaterial3d(europa_mat),
            Transform::from_xyz(160.0, 0.0, 0.0),
        ))
        .id();
    commands.entity(jupiter_entity).add_child(europa_entity);

    // --- PLANET 6: SATURN & RINGS ---
    let saturn_mesh = meshes.add(Sphere::new(45.0).mesh().kind(SphereKind::Uv { sectors: 40, stacks: 20 }));
    let saturn_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.90, 0.76, 0.48),
        perceptual_roughness: 0.45,
        ..default()
    });
    let saturn_entity = commands
        .spawn((
            Planet {
                _name: "Saturn",
                index: 6,
                radius: 45.0,
                orbit_radius: 1_800_000.0,
                orbit_speed: 0.02,
                rotation_speed: 0.75,
                world_pos: Vec3::new(1_800_000.0, 0.0, 0.0),
            },
            Mesh3d(saturn_mesh),
            MeshMaterial3d(saturn_mat),
            Transform::from_xyz(1_800_000.0, 0.0, 0.0),
        ))
        .id();

    // Saturn Ring System
    let saturn_outer_ring_mesh = meshes.add(Torus { minor_radius: 2.2, major_radius: 82.0 });
    let saturn_outer_ring_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.92, 0.78, 0.48, 0.85),
        perceptual_roughness: 0.4,
        ..default()
    });
    let saturn_outer_ring = commands
        .spawn((
            Mesh3d(saturn_outer_ring_mesh),
            MeshMaterial3d(saturn_outer_ring_mat),
            Transform::from_rotation(Quat::from_rotation_x(0.45)),
        ))
        .id();
    commands.entity(saturn_entity).add_child(saturn_outer_ring);

    // --- PLANET 7: URANUS ---
    let uranus_mesh = meshes.add(Sphere::new(22.0).mesh().kind(SphereKind::Uv { sectors: 32, stacks: 16 }));
    let uranus_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.82, 0.88),
        perceptual_roughness: 0.3,
        ..default()
    });
    commands.spawn((
        Planet {
            _name: "Uranus",
            index: 7,
            radius: 22.0,
            orbit_radius: 3_600_000.0,
            orbit_speed: 0.012,
            rotation_speed: -0.5,
            world_pos: Vec3::new(3_600_000.0, 0.0, 0.0),
        },
        Mesh3d(uranus_mesh),
        MeshMaterial3d(uranus_mat),
        Transform::from_xyz(3_600_000.0, 0.0, 0.0),
    ));

    // --- PLANET 8: NEPTUNE ---
    let neptune_mesh = meshes.add(Sphere::new(21.0).mesh().kind(SphereKind::Uv { sectors: 32, stacks: 16 }));
    let neptune_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.42, 0.92),
        perceptual_roughness: 0.25,
        ..default()
    });
    commands.spawn((
        Planet {
            _name: "Neptune",
            index: 8,
            radius: 21.0,
            orbit_radius: 6_500_000.0,
            orbit_speed: 0.007,
            rotation_speed: 0.55,
            world_pos: Vec3::new(6_500_000.0, 0.0, 0.0),
        },
        Mesh3d(neptune_mesh),
        MeshMaterial3d(neptune_mat),
        Transform::from_xyz(6_500_000.0, 0.0, 0.0),
    ));

    // ----------------------------------------------------
    // 4. DEEP SPACE STARFIELD (COMPACT PROCEDURAL SPHERES)
    // ----------------------------------------------------
    let star_mesh = meshes.add(Sphere::new(4.0).mesh().kind(SphereKind::Uv { sectors: 8, stacks: 4 }));
    let star_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::new(12.0, 14.0, 18.0, 1.0),
        unlit: true,
        ..default()
    });

    let mut rng_seed: u64 = 987654321;
    let num_stars = 1200;

    for _ in 0..num_stars {
        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let theta = (rng_seed as f32 / u64::MAX as f32) * std::f32::consts::TAU;

        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let phi = ((rng_seed as f32 / u64::MAX as f32) * 2.0 - 1.0).acos();

        let dist = 10_000_000.0;
        let x = dist * phi.sin() * theta.cos();
        let y = dist * phi.sin() * theta.sin();
        let z = dist * phi.cos();

        commands.spawn((
            Starfield {
                world_pos: Vec3::new(x, y, z),
            },
            Mesh3d(star_mesh.clone()),
            MeshMaterial3d(star_mat.clone()),
            Transform::from_xyz(x, y, z),
        ));
    }
}

// ----------------------------------------------------
// SYSTEM LOGIC & FLIGHT DYNAMICS
// ----------------------------------------------------

fn exit_on_esc(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        app_exit.write(AppExit::Success);
    }
}

fn pilot_freelook_system(
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
    let sensitivity = 0.002;
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

    flight_state.yaw = (flight_state.yaw + look_target.x).clamp(-1.57, 1.57);
    flight_state.pitch = (flight_state.pitch + look_target.y).clamp(-1.2, 1.2);

    if let Ok(mut cam_transform) = camera_query.single_mut() {
        cam_transform.rotation = Quat::from_euler(
            EulerRot::YXZ,
            flight_state.yaw,
            flight_state.pitch,
            0.0,
        );
    }
}

fn ship_flight_system(
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

// ----------------------------------------------------
// AUTO-PILOT INPUT & FLIGHT TRAJECTORY SYSTEMS
// ----------------------------------------------------

fn autopilot_input_system(
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

fn autopilot_flight_system(
    time: Res<Time>,
    mut autopilot: ResMut<AutoPilotState>,
    mut flight_state: ResMut<FlightState>,
    mut ship_query: Query<&mut Transform, With<Ship>>,
    planet_query: Query<(&Planet, &GlobalTransform)>,
) {
    if !autopilot.active {
        return;
    }

    let Some(target_idx) = autopilot.target_index else { return; };
    let Ok(mut ship_transform) = ship_query.single_mut() else { return; };

    let mut target_pos = Vec3::ZERO;
    let mut target_radius = 10.0;
    let mut found = false;

    for (planet, global_transform) in &planet_query {
        if planet.index == target_idx {
            target_pos = global_transform.translation();
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

// ----------------------------------------------------
// LOGARITHMIC DISTANCE RENDERING & CULLING SYSTEM
// ----------------------------------------------------

fn logarithmic_distance_render_system(
    camera_query: Query<(&GlobalTransform, &Camera), With<PilotCamera>>,
    mut planet_query: Query<(&Planet, &mut Transform, &mut Visibility), Without<PilotCamera>>,
    mut moon_query: Query<(&Moon, &mut Transform, &mut Visibility), (Without<Planet>, Without<PilotCamera>)>,
    mut star_query: Query<(&Starfield, &mut Transform, &mut Visibility), (Without<Planet>, Without<Moon>, Without<PilotCamera>)>,
) {
    let Ok((cam_global_transform, _camera)) = camera_query.single() else { return; };
    let cam_pos = cam_global_transform.translation();
    let cam_rot = cam_global_transform.rotation();

    let forward = cam_rot * Vec3::NEG_Z;
    let right = cam_rot * Vec3::X;
    let up = cam_rot * Vec3::Y;

    let k = 0.000035;
    let scale_const = 6500.0;

    // Render Planets
    for (planet, mut transform, mut vis) in &mut planet_query {
        let vec_to = planet.world_pos - cam_pos;
        let d_real = vec_to.length();

        if d_real < 1.0 {
            transform.translation = planet.world_pos;
            *vis = Visibility::Inherited;
            continue;
        }

        let dir = vec_to / d_real;
        let z_proj = dir.dot(forward);
        let x_proj = dir.dot(right);
        let y_proj = dir.dot(up);

        let half_fov_tan = 0.85;
        if z_proj <= 0.02 || x_proj.abs() / z_proj > half_fov_tan * 1.5 || y_proj.abs() / z_proj > half_fov_tan * 1.5 {
            *vis = Visibility::Hidden;
            continue;
        }

        *vis = Visibility::Inherited;
        let d_vis = scale_const * (1.0 + k * d_real).ln();
        transform.translation = cam_pos + dir * d_vis;

        let scale_factor = (d_vis / d_real).clamp(0.001, 1.0);
        let _vis_radius = planet.radius * scale_factor;
        let min_scale = (0.015 * d_vis) / planet.radius;
        let final_scale = scale_factor.max(min_scale);

        transform.scale = Vec3::splat(final_scale);
    }

    // Render Moons
    for (moon, mut transform, mut vis) in &mut moon_query {
        let vec_to = moon.world_pos - cam_pos;
        let d_real = vec_to.length();

        if d_real < 1.0 {
            transform.translation = moon.world_pos;
            *vis = Visibility::Inherited;
            continue;
        }

        let dir = vec_to / d_real;
        let z_proj = dir.dot(forward);

        if z_proj <= 0.02 {
            *vis = Visibility::Hidden;
            continue;
        }

        *vis = Visibility::Inherited;
        let d_vis = scale_const * (1.0 + k * d_real).ln();
        transform.translation = cam_pos + dir * d_vis;

        let scale_factor = (d_vis / d_real).clamp(0.001, 1.0);
        let min_scale = (0.012 * d_vis) / moon.radius;
        let final_scale = scale_factor.max(min_scale);

        transform.scale = Vec3::splat(final_scale);
    }

    // Render Starfield
    for (star, mut transform, mut vis) in &mut star_query {
        let vec_to = star.world_pos - cam_pos;
        let d_real = vec_to.length();
        let dir = vec_to / d_real;

        let z_proj = dir.dot(forward);
        if z_proj <= 0.01 {
            *vis = Visibility::Hidden;
            continue;
        }

        *vis = Visibility::Inherited;
        let d_vis = 85_000.0;
        transform.translation = cam_pos + dir * d_vis;
    }
}

fn celestial_collision_system(
    mut flight_state: ResMut<FlightState>,
    mut ship_query: Query<&mut Transform, With<Ship>>,
    planet_query: Query<(&Planet, &GlobalTransform)>,
) {
    let Ok(mut ship_transform) = ship_query.single_mut() else { return; };

    for (planet, global_transform) in &planet_query {
        let planet_pos = global_transform.translation();
        let min_dist = planet.radius * 1.5 + 20.0;
        let dist = ship_transform.translation.distance(planet_pos);

        if dist < min_dist {
            let push_dir = (ship_transform.translation - planet_pos).normalize_or_zero();
            ship_transform.translation = planet_pos + push_dir * min_dist;
            flight_state.velocity = push_dir * 100.0;
        }
    }
}

fn orbit_planets_system(time: Res<Time>, mut query: Query<(&Planet, &mut Transform)>) {
    for (planet, mut transform) in &mut query {
        transform.rotate_y(planet.rotation_speed * time.delta_secs());
    }
}

fn orbit_moons_system(time: Res<Time>, mut query: Query<(&Moon, &mut Transform)>) {
    for (moon, mut transform) in &mut query {
        transform.rotate_y(moon.rotation_speed * time.delta_secs());
    }
}

fn engine_sound_system(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    flight_state: Res<FlightState>,
    mut sink_query: Query<&mut AudioSink, With<EngineSound>>,
) {
    let dt = time.delta_secs();
    let speed = flight_state.velocity.length();
    let max_speed = 7000.0;
    let speed_ratio = (speed / max_speed).clamp(0.0, 1.0);

    let is_boosting = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    let thrust_boost = if is_boosting { 0.5 } else { 0.0 };

    let target_pitch = 0.85 + (speed_ratio * 0.95) + thrust_boost;
    let target_volume = 0.15 + (speed_ratio * 0.20) + (thrust_boost * 0.08);

    for mut sink in &mut sink_query {
        let current_pitch = sink.speed();
        let new_pitch = current_pitch + (target_pitch - current_pitch) * (4.0 * dt).min(1.0);

        sink.set_speed(new_pitch);
        sink.set_volume(bevy::audio::Volume::Linear(target_volume));
    }
}

// ----------------------------------------------------
// COCKPIT ANIMATION SYSTEMS (STEADY, NON-STROBING TINTS)
// ----------------------------------------------------

fn animate_cockpit_screens_system(
    time: Res<Time>,
    mut needle_query: Query<&mut Transform, With<RadarSweepNeedle>>,
) {
    let dt = time.delta_secs();

    // Smooth continuous radar sweep needle rotation (no flashing/strobing)
    for mut transform in &mut needle_query {
        transform.rotate_local_z(-1.2 * dt);
    }
}

fn animate_cockpit_buttons_system(
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

fn update_hud_system(
    autopilot: Res<AutoPilotState>,
    flight_state: Res<FlightState>,
    ship_query: Query<&Transform, With<Ship>>,
    planet_query: Query<(&Planet, &GlobalTransform)>,
    mut text_query: Query<&mut Text, With<AutoPilotHudText>>,
) {
    let Ok(ship_transform) = ship_query.single() else { return; };
    let speed = flight_state.velocity.length();

    for mut text in &mut text_query {
        if autopilot.active {
            if let Some(target_idx) = autopilot.target_index {
                let mut dist_str = String::from("CALCULATING...");
                for (planet, global_transform) in &planet_query {
                    if planet.index == target_idx {
                        let dist = ship_transform.translation.distance(global_transform.translation());
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
