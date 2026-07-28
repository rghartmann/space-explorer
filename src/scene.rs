use bevy::prelude::*;

use crate::audio::{ensure_ambient_piano_file, ensure_engine_hum_file};
use crate::components::*;

pub fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // 2D HUD CAMERA & OVERLAY UI
    commands.spawn(Camera2d);

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
    let sun_mesh = meshes.add(Sphere::new(120.0).mesh().kind(bevy::render::mesh::SphereKind::Uv { sectors: 40, stacks: 20 }));
    let sun_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.85, 0.35),
        emissive: LinearRgba::new(25.0, 18.0, 4.0, 1.0),
        unlit: true,
        ..default()
    });

    commands.spawn((
        Sun { _radius: 120.0 },
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
    let mercury_mesh = meshes.add(Sphere::new(4.5).mesh().kind(bevy::render::mesh::SphereKind::Uv { sectors: 24, stacks: 12 }));
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
            _orbit_radius: 85_000.0,
            _orbit_speed: 0.15,
            rotation_speed: 0.05,
            world_pos: Vec3::new(85_000.0, 0.0, 0.0),
        },
        Mesh3d(mercury_mesh),
        MeshMaterial3d(mercury_mat),
        Transform::from_xyz(85_000.0, 0.0, 0.0),
    ));

    // --- PLANET 2: VENUS ---
    let venus_mesh = meshes.add(Sphere::new(11.0).mesh().kind(bevy::render::mesh::SphereKind::Uv { sectors: 32, stacks: 16 }));
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
            _orbit_radius: 150_000.0,
            _orbit_speed: 0.11,
            rotation_speed: 0.02,
            world_pos: Vec3::new(150_000.0, 0.0, 0.0),
        },
        Mesh3d(venus_mesh),
        MeshMaterial3d(venus_mat),
        Transform::from_xyz(150_000.0, 0.0, 0.0),
    ));

    // --- PLANET 3: EARTH & MOON ---
    let earth_mesh = meshes.add(Sphere::new(12.0).mesh().kind(bevy::render::mesh::SphereKind::Uv { sectors: 32, stacks: 16 }));
    let earth_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.38, 0.78),
        perceptual_roughness: 0.35,
        metallic: 0.1,
        ..default()
    });

    commands.spawn((
        Planet {
            _name: "Earth",
            index: 3,
            radius: 12.0,
            _orbit_radius: 240_000.0,
            _orbit_speed: 0.08,
            rotation_speed: 0.4,
            world_pos: Vec3::new(240_000.0, 0.0, 0.0),
        },
        Mesh3d(earth_mesh),
        MeshMaterial3d(earth_mat),
        Transform::from_xyz(240_000.0, 0.0, 0.0),
    ));

    // Earth's Moon
    let moon_mesh = meshes.add(Sphere::new(3.2).mesh().kind(bevy::render::mesh::SphereKind::Uv { sectors: 24, stacks: 12 }));
    let moon_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.68, 0.68, 0.65),
        perceptual_roughness: 0.85,
        ..default()
    });
    commands.spawn((
        Moon {
            _name: "Moon",
            _parent_index: 3,
            radius: 3.2,
            _orbit_radius: 65.0,
            _orbit_speed: 0.25,
            rotation_speed: 0.1,
            world_pos: Vec3::new(240_000.0 + 65.0, 0.0, 0.0),
        },
        Mesh3d(moon_mesh),
        MeshMaterial3d(moon_mat),
        Transform::from_xyz(240_000.0 + 65.0, 0.0, 0.0),
    ));

    // --- PLANET 4: MARS ---
    let mars_mesh = meshes.add(Sphere::new(6.8).mesh().kind(bevy::render::mesh::SphereKind::Uv { sectors: 32, stacks: 16 }));
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
            _orbit_radius: 380_000.0,
            _orbit_speed: 0.06,
            rotation_speed: 0.38,
            world_pos: Vec3::new(380_000.0, 0.0, 0.0),
        },
        Mesh3d(mars_mesh),
        MeshMaterial3d(mars_mat),
        Transform::from_xyz(380_000.0, 0.0, 0.0),
    ));

    // --- PLANET 5: JUPITER & GALILEAN MOONS ---
    let jupiter_mesh = meshes.add(Sphere::new(55.0).mesh().kind(bevy::render::mesh::SphereKind::Uv { sectors: 40, stacks: 20 }));
    let jupiter_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.82, 0.64, 0.45),
        perceptual_roughness: 0.5,
        ..default()
    });
    commands.spawn((
        Planet {
            _name: "Jupiter",
            index: 5,
            radius: 55.0,
            _orbit_radius: 950_000.0,
            _orbit_speed: 0.035,
            rotation_speed: 0.8,
            world_pos: Vec3::new(950_000.0, 0.0, 0.0),
        },
        Mesh3d(jupiter_mesh),
        MeshMaterial3d(jupiter_mat),
        Transform::from_xyz(950_000.0, 0.0, 0.0),
    ));

    // Io
    let io_mesh = meshes.add(Sphere::new(3.0).mesh().kind(bevy::render::mesh::SphereKind::Uv { sectors: 20, stacks: 10 }));
    let io_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.85, 0.2),
        perceptual_roughness: 0.7,
        ..default()
    });
    commands.spawn((
        Moon {
            _name: "Io",
            _parent_index: 5,
            radius: 3.0,
            _orbit_radius: 110.0,
            _orbit_speed: 0.45,
            rotation_speed: 0.3,
            world_pos: Vec3::new(950_000.0 + 110.0, 0.0, 0.0),
        },
        Mesh3d(io_mesh),
        MeshMaterial3d(io_mat),
        Transform::from_xyz(950_000.0 + 110.0, 0.0, 0.0),
    ));

    // Europa
    let europa_mesh = meshes.add(Sphere::new(2.6).mesh().kind(bevy::render::mesh::SphereKind::Uv { sectors: 20, stacks: 10 }));
    let europa_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.88, 0.95),
        perceptual_roughness: 0.2,
        metallic: 0.1,
        ..default()
    });
    commands.spawn((
        Moon {
            _name: "Europa",
            _parent_index: 5,
            radius: 2.6,
            _orbit_radius: 160.0,
            _orbit_speed: 0.32,
            rotation_speed: 0.25,
            world_pos: Vec3::new(950_000.0 + 160.0, 0.0, 0.0),
        },
        Mesh3d(europa_mesh),
        MeshMaterial3d(europa_mat),
        Transform::from_xyz(950_000.0 + 160.0, 0.0, 0.0),
    ));

    // --- PLANET 6: SATURN & RINGS ---
    let saturn_mesh = meshes.add(Sphere::new(45.0).mesh().kind(bevy::render::mesh::SphereKind::Uv { sectors: 40, stacks: 20 }));
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
                _orbit_radius: 1_800_000.0,
                _orbit_speed: 0.02,
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
    let uranus_mesh = meshes.add(Sphere::new(22.0).mesh().kind(bevy::render::mesh::SphereKind::Uv { sectors: 32, stacks: 16 }));
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
            _orbit_radius: 3_600_000.0,
            _orbit_speed: 0.012,
            rotation_speed: -0.5,
            world_pos: Vec3::new(3_600_000.0, 0.0, 0.0),
        },
        Mesh3d(uranus_mesh),
        MeshMaterial3d(uranus_mat),
        Transform::from_xyz(3_600_000.0, 0.0, 0.0),
    ));

    // --- PLANET 8: NEPTUNE ---
    let neptune_mesh = meshes.add(Sphere::new(21.0).mesh().kind(bevy::render::mesh::SphereKind::Uv { sectors: 32, stacks: 16 }));
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
            _orbit_radius: 6_500_000.0,
            _orbit_speed: 0.007,
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
    let star_mesh = meshes.add(Sphere::new(4.0).mesh().kind(bevy::render::mesh::SphereKind::Uv { sectors: 8, stacks: 4 }));
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
