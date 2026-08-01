use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::{Bloom, BloomPrefilter};
use bevy::prelude::*;
use bevy::camera::visibility::NoFrustumCulling;

use crate::audio::{ensure_ambient_piano_file, ensure_engine_hum_file};
use crate::components::*;
use crate::lod::PlanetLod;
use crate::resources::{AppState, FlightState, LoadingAssets};

pub fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut flight_state: ResMut<FlightState>,
) {
    let mut orbit_rng: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(987654321);

    let mut next_orbit_angle = || -> f32 {
        orbit_rng = orbit_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        (orbit_rng as f32 / u64::MAX as f32) * std::f32::consts::TAU
    };







    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(12.0),
                right: Val::Px(12.0),
                padding: UiRect::all(Val::Px(10.0)),
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
                Text::new("AUTOPILOT DESTINATIONS: [0] Sun | [1] Mercury | [2] Venus | [3] Earth | [M] Moon | [4] Mars | [5] Jupiter | [6] Saturn | [7] Uranus | [8] Neptune | [9] Pluto | [C] Ceres | [H] Haumea | [K] Makemake | [E] Eris"),
                TextFont {
                    font_size: 12.0.into(),
                    ..default()
                },
                TextColor(Color::srgb(0.0, 0.88, 1.0)),
            ));

            parent.spawn((
                Text::new("FLIGHT CONTROLS: W/S (Accel/Decel - Cap 2,000 km/s) | MOUSE / ARROWS / A-D (Pitch/Yaw) | Q-E / Z-X (Roll) | SPACE (Warp 100x c / Stop Autopilot) | ESC (Exit)"),
                TextFont {
                    font_size: 11.5.into(),
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.8, 0.9)),
            ));

            parent.spawn((
                AutoPilotHudText,
                Text::new("FLIGHT STATUS: MANUAL CONTROL | SPEED: 0 km/s | PRESS [0-9/C/H/K/E/M] TO ENGAGE AUTOPILOT | PRESS SPACE TO STOP AUTOPILOT"),
                TextFont {
                    font_size: 13.0.into(),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.85, 0.2)),
            ));
        });

    // Center Warning Banner for Autopilot Status & Undock Prompt (positioned above spaceship)
    commands
        .spawn((
            AutopilotWarningBanner,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(36.0),
                left: Val::Auto,
                right: Val::Auto,
                align_self: AlignSelf::Center,
                justify_self: JustifySelf::Center,
                padding: UiRect::axes(Val::Px(24.0), Val::Px(10.0)),
                border: UiRect::all(Val::Px(1.5)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.08, 0.16, 0.92)),
            BorderColor::all(Color::srgba(1.0, 0.6, 0.0, 0.95)),
            GlobalZIndex(20),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("[!] AUTOPILOT ENGAGED [!]"),
                TextFont {
                    font_size: 15.0.into(),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.7, 0.1)),
            ));
            parent.spawn((
                Text::new("Press [SPACE] to Undock / Cancel"),
                TextFont {
                    font_size: 12.0.into(),
                    ..default()
                },
                TextColor(Color::srgb(0.85, 0.95, 1.0)),
            ));
        });

    // 2D NAVIGATION LABELS FOR CELESTIAL BODIES IN THE DISTANCE
    let labels_info = [
        ("[0] ", "Sun", CelestialDestinationType::Sun),
        ("[1] ", "Mercury", CelestialDestinationType::Planet(1)),
        ("[2] ", "Venus", CelestialDestinationType::Planet(2)),
        ("[3] ", "Earth", CelestialDestinationType::Planet(3)),
        ("[M] ", "Moon", CelestialDestinationType::Moon("Moon")),
        ("[4] ", "Mars", CelestialDestinationType::Planet(4)),
        ("[C] ", "Ceres", CelestialDestinationType::Planet(10)),
        ("[5] ", "Jupiter", CelestialDestinationType::Planet(5)),
        ("", "Io", CelestialDestinationType::Moon("Io")),
        ("", "Europa", CelestialDestinationType::Moon("Europa")),
        ("[6] ", "Saturn", CelestialDestinationType::Planet(6)),
        ("[7] ", "Uranus", CelestialDestinationType::Planet(7)),
        ("[8] ", "Neptune", CelestialDestinationType::Planet(8)),
        ("[9] ", "Pluto", CelestialDestinationType::Planet(9)),
        ("", "Charon", CelestialDestinationType::Moon("Charon")),
        ("[H] ", "Haumea", CelestialDestinationType::Planet(11)),
        ("[K] ", "Makemake", CelestialDestinationType::Planet(12)),
        ("[E] ", "Eris", CelestialDestinationType::Planet(13)),
    ];

    for (prefix, name, destination_type) in labels_info {
        commands
            .spawn((
                CelestialLabel {
                    name,
                    key_prefix: prefix,
                    destination_type,
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(-9999.0),
                    top: Val::Px(-9999.0),
                    padding: UiRect::ZERO,
                    border: UiRect::ZERO,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Visibility::Hidden,
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new(format!("{}{}", prefix, name.to_uppercase())),
                    TextFont {
                        font_size: 9.5.into(),
                        ..default()
                    },
                    TextColor(Color::srgba(0.0, 0.75, 0.85, 0.65)),
                ));
            });
    }

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
    // SHIP AVATAR & 3RD-PERSON CAMERA PERSPECTIVE
    // ----------------------------------------------------
    // ----------------------------------------------------
    // ----------------------------------------------------
    // SHIP AVATAR & 3RD-PERSON CAMERA PERSPECTIVE
    // ----------------------------------------------------
    // Pre-calculate Earth position to align initial ship spawn direction towards Earth
    let earth_orbit_radius = 1.0 * AU; // 149,597,870.7 km
    let earth_orbit_speed = 0.1500;
    let earth_angle = next_orbit_angle();
    let earth_pos = Vec3::new(earth_orbit_radius * earth_angle.cos(), 0.0, earth_orbit_radius * earth_angle.sin());

    // Calculate initial deep-space ship spawn position (~2.2 AU / 329.1M km from Sun) facing Earth
    let initial_spawn_dist = 2.2 * AU;
    let initial_spawn_pos = earth_pos.normalize() * initial_spawn_dist + Vec3::new(0.0, 150_000.0, 0.0);
    let dir_to_earth = (earth_pos - initial_spawn_pos).normalize_or_zero();
    let initial_ship_rot = crate::flight::rotation_looking_to(dir_to_earth);

    flight_state.world_pos = initial_spawn_pos;
    flight_state.previous_pos = initial_spawn_pos;

    let ship_entity = commands
        .spawn((
            Ship,
            Transform::from_translation(Vec3::ZERO).with_rotation(initial_ship_rot),
            Visibility::default(),
        ))
        .id();

    // 3D Spaceship Avatar (Scaled to ~0.050 km / 50m length for modern manned spacecraft like SpaceX Starship)
    let avatar_entity = commands
        .spawn((
            WorldAssetRoot(asset_server.load(bevy::gltf::GltfAssetLabel::Scene(0).from_asset("models/spaceship.glb"))),
            Transform::from_scale(Vec3::splat(0.14))
                .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        ))
        .id();
    commands.entity(ship_entity).add_child(avatar_entity);

    // Attach particle emitters & dynamic lights at rear engine positions on spaceship
    crate::particles::spawn_thruster_emitters(&mut commands, ship_entity);


    // 3rd-Person Camera positioned behind and slightly above the spaceship avatar
    let camera_entity = commands
        .spawn((
            PilotCamera,
            Camera {
                order: 0,
                ..default()
            },
            Camera3d::default(),
            Tonemapping::default(),
            Bloom {
                intensity: 0.12,
                low_frequency_boost: 0.3,
                high_pass_frequency: 1.0,
                prefilter: BloomPrefilter {
                    threshold: 1.2,
                    threshold_softness: 0.2,
                },
                ..default()
            },
            Projection::Perspective(PerspectiveProjection {
                near: 0.1,
                far: 10_000_000.0,
                ..default()
            }),
            Transform::from_xyz(0.0, 1.2, 4.0)
                .looking_at(Vec3::new(0.0, 0.1, -5.0), Vec3::Y),
            DistanceFog {
                color: Color::srgba(0.0005, 0.001, 0.003, 1.0),
                falloff: FogFalloff::Exponential { density: 0.0000001 },
                ..default()
            },
        ))
        .id();

    commands.entity(ship_entity).add_child(camera_entity);

    // Soft Fill Light for 3D Spaceship PBR Texture Visibility
    let ship_light = commands
        .spawn((
            PointLight {
                intensity: 1_200.0,
                color: Color::srgb(0.9, 0.95, 1.0),
                range: 12.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_xyz(0.0, 2.0, 3.0),
        ))
        .id();
    commands.entity(ship_entity).add_child(ship_light);


    // Ambient Fill Light for deep space (Subtle & Dramatic)
    commands.spawn(AmbientLight {
        color: Color::srgba(0.04, 0.06, 0.12, 1.0),
        brightness: 1.0,
        affects_lightmapped_meshes: false,
    });

    // Dynamic Sunlight
    commands.spawn((
        SunDirectionalLight,
        DirectionalLight {
            color: Color::srgb(1.0, 0.97, 0.92),
            illuminance: 15_000.0,
            shadow_maps_enabled: false,
            contact_shadows_enabled: false,
            ..default()
        },
        Transform::IDENTITY,
    ));


    // ----------------------------------------------------
    // 2. THE SUN (NASA SVS 30362 HIGH-RES RADIANT SURFACE & 16-FRAME ANIMATION LOOP)
    // ----------------------------------------------------
    let sun_tex: Handle<Image> = asset_server.load("textures/sun.jpg");

    let mut sun_anim_handles: Vec<Handle<Image>> = Vec::with_capacity(16);
    for i in 0..16 {
        sun_anim_handles.push(asset_server.load(format!("textures/sun_anim/frame_{:02}.jpg", i)));
    }

    let sun_mesh = meshes.add(create_uv_sphere(696340.0, 192, 96));
    let sun_mat = materials.add(StandardMaterial {
        base_color: Color::linear_rgb(8.0, 6.5, 3.0),
        base_color_texture: Some(sun_tex.clone()),
        emissive_texture: Some(sun_tex),
        emissive: LinearRgba::new(45.0, 32.0, 10.0, 1.0),
        unlit: true,
        ..default()
    });

    let sun_entity = commands.spawn((
        Sun { radius: 696340.0 },
        SunAnimation {
            frame_handles: sun_anim_handles,
            current_frame: 0,
            frame_timer: Timer::from_seconds(0.35, TimerMode::Repeating),
            pulse_timer: 0.0,
        },
        Mesh3d(sun_mesh),
        MeshMaterial3d(sun_mat),
        NoFrustumCulling,
        Transform::from_xyz(0.0, 0.0, 0.0),
    )).id();

    // Solar Corona Soft Atmospheric Aura (Photorealistic Subtle Glow)
    let corona_mesh = meshes.add(create_uv_sphere(780000.0, 48, 24));
    let corona_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.55, 0.12, 0.08),
        emissive: LinearRgba::new(8.0, 4.0, 1.0, 0.08),
        alpha_mode: AlphaMode::Add,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let corona_entity = commands.spawn((
        Mesh3d(corona_mesh),
        MeshMaterial3d(corona_mat),
        NoFrustumCulling,
        Transform::IDENTITY,
    )).id();
    commands.entity(sun_entity).add_child(corona_entity);

    // Sun Core Sunlight (PointLight)
    commands.spawn((
        PointLight {
            intensity: 100_000.0,
            color: Color::srgb(1.0, 0.96, 0.88),
            range: 5_000_000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // ----------------------------------------------------
    // 3. PLANETS, DWARF PLANETS & MOONS (REAL 1:1 KILOMETER SCALES & REALISTIC ROTATIONS)
    // ----------------------------------------------------

    // --- PLANET 1: MERCURY (0.3871 AU) ---
    let mercury_orbit_radius = 0.387098 * AU; // 57,909,175.0 km
    let mercury_orbit_speed = 0.6225; // 4.15x Earth speed
    let mercury_angle = next_orbit_angle();
    let mercury_pos = Vec3::new(mercury_orbit_radius * mercury_angle.cos(), 0.0, mercury_orbit_radius * mercury_angle.sin());
    let mercury_tex: Handle<Image> = asset_server.load("textures/mercury.jpg");
    let mercury_mesh = meshes.add(create_uv_sphere(2439.7, 192, 96));
    let mercury_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.35, 0.35),
        base_color_texture: Some(mercury_tex.clone()),
        perceptual_roughness: 0.95,
        reflectance: 0.08,
        ..default()
    });
    let mercury_entity = commands.spawn((
        Planet {
            name: "Mercury",
            index: 1,
            radius: 2439.7,
            orbit_radius: mercury_orbit_radius,
            orbit_speed: mercury_orbit_speed,
            orbit_angle: mercury_angle,
            rotation_speed: 0.00005, // 58.65d spin
            world_pos: mercury_pos,
        },
        PlanetLod::new(1, false, "", 18.0, mercury_tex.clone(), 192, 96),
        Mesh3d(mercury_mesh),
        MeshMaterial3d(mercury_mat),
        Transform::from_translation(mercury_pos),
    )).id();
    spawn_planet_area_light(&mut commands, mercury_entity, mercury_pos, 2439.7);

    // --- PLANET 2: VENUS (0.7233 AU) ---
    let venus_orbit_radius = 0.723332 * AU; // 108,208,614.0 km
    let venus_orbit_speed = 0.2430; // 1.62x Earth speed
    let venus_angle = next_orbit_angle();
    let venus_pos = Vec3::new(venus_orbit_radius * venus_angle.cos(), 0.0, venus_orbit_radius * venus_angle.sin());
    let venus_tex: Handle<Image> = asset_server.load("textures/venus.jpg");
    let venus_mesh = meshes.add(create_uv_sphere(6051.8, 192, 96));
    let venus_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.36, 0.32),
        base_color_texture: Some(venus_tex.clone()),
        perceptual_roughness: 0.90,
        reflectance: 0.10,
        ..default()
    });
    let venus_entity = commands.spawn((
        Planet {
            name: "Venus",
            index: 2,
            radius: 6051.8,
            orbit_radius: venus_orbit_radius,
            orbit_speed: venus_orbit_speed,
            orbit_angle: venus_angle,
            rotation_speed: -0.00001, // Retrograde spin (243d)
            world_pos: venus_pos,
        },
        PlanetLod::new(2, false, "", 22.0, venus_tex.clone(), 192, 96),
        Mesh3d(venus_mesh),
        MeshMaterial3d(venus_mat),
        Transform::from_translation(venus_pos),
    )).id();
    spawn_planet_area_light(&mut commands, venus_entity, venus_pos, 6051.8);

    // --- PLANET 3: EARTH (1.0000 AU) & THE MOON ---
    let earth_tex: Handle<Image> = asset_server.load("textures/earth.jpg");
    let earth_mesh = meshes.add(create_uv_sphere(6371.0, 192, 96));
    let earth_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.42, 0.42),
        base_color_texture: Some(earth_tex.clone()),
        perceptual_roughness: 0.70,
        metallic: 0.05,
        reflectance: 0.12,
        ..default()
    });
    let earth_entity = commands.spawn((
        Planet {
            name: "Earth",
            index: 3,
            radius: 6371.0,
            orbit_radius: earth_orbit_radius,
            orbit_speed: earth_orbit_speed,
            orbit_angle: earth_angle,
            rotation_speed: 0.002, // 24h spin
            world_pos: earth_pos,
        },
        PlanetLod::new(3, false, "", 25.0, earth_tex.clone(), 192, 96),
        Mesh3d(earth_mesh),
        MeshMaterial3d(earth_mat),
        Transform::from_translation(earth_pos),
    )).id();
    spawn_planet_area_light(&mut commands, earth_entity, earth_pos, 6371.0);

    // Earth's Atmosphere Layer (Glow Shell)
    let earth_atmo_mesh = meshes.add(create_uv_sphere(6471.0, 64, 32));
    let earth_atmo_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.5, 1.0, 0.18),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let earth_atmo_entity = commands.spawn((
        Mesh3d(earth_atmo_mesh),
        MeshMaterial3d(earth_atmo_mat),
        Transform::IDENTITY,
    )).id();
    commands.entity(earth_entity).add_child(earth_atmo_entity);

    // Earth's Satellite: The Moon (0.00257 AU / 384,400 km orbit around Earth)
    let moon_orbit_radius = 384400.0; // 384,400 km orbit
    let moon_orbit_speed = 0.0547; // 27.3d orbit
    let moon_angle = next_orbit_angle();
    let moon_pos = earth_pos + Vec3::new(moon_orbit_radius * moon_angle.cos(), 0.0, moon_orbit_radius * moon_angle.sin());
    let moon_tex: Handle<Image> = asset_server.load("textures/moon.jpg");
    let moon_mesh = meshes.add(create_uv_sphere(1737.4, 96, 48));
    let moon_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.32, 0.32),
        base_color_texture: Some(moon_tex.clone()),
        perceptual_roughness: 0.95,
        reflectance: 0.06,
        ..default()
    });
    let moon_entity = commands.spawn((
        Moon {
            name: "Moon",
            parent_index: 3,
            radius: 1737.4,
            orbit_radius: moon_orbit_radius,
            orbit_speed: moon_orbit_speed,
            orbit_angle: moon_angle,
            rotation_speed: 0.0547, // Synchronous rotation
            world_pos: moon_pos,
        },
        PlanetLod::new(3, true, "Moon", 12.0, moon_tex.clone(), 96, 48),
        Mesh3d(moon_mesh),
        MeshMaterial3d(moon_mat),
        Transform::from_translation(moon_pos),
    )).id();
    spawn_planet_area_light(&mut commands, moon_entity, moon_pos, 1737.4);

    // --- PLANET 4: MARS (1.5237 AU) ---
    let mars_orbit_radius = 1.523_68 * AU; // 227,938,284.0 km
    let mars_orbit_speed = 0.0798; // 0.53x Earth speed
    let mars_angle = next_orbit_angle();
    let mars_pos = Vec3::new(mars_orbit_radius * mars_angle.cos(), 0.0, mars_orbit_radius * mars_angle.sin());
    let mars_tex: Handle<Image> = asset_server.load("textures/mars.jpg");
    let mars_mesh = meshes.add(create_uv_sphere(3389.5, 192, 96));
    let mars_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.40, 0.35, 0.30),
        base_color_texture: Some(mars_tex.clone()),
        perceptual_roughness: 0.90,
        reflectance: 0.08,
        ..default()
    });
    let mars_entity = commands.spawn((
        Planet {
            name: "Mars",
            index: 4,
            radius: 3389.5,
            orbit_radius: mars_orbit_radius,
            orbit_speed: mars_orbit_speed,
            orbit_angle: mars_angle,
            rotation_speed: 0.0019, // 24.6h spin
            world_pos: mars_pos,
        },
        PlanetLod::new(4, false, "", 20.0, mars_tex.clone(), 192, 96),
        Mesh3d(mars_mesh),
        MeshMaterial3d(mars_mat),
        Transform::from_translation(mars_pos),
    )).id();
    spawn_planet_area_light(&mut commands, mars_entity, mars_pos, 3389.5);

    // --- DWARF PLANET: CERES (2.767 AU) ---
    let ceres_orbit_radius = 2.767 * AU; // 413,937,308.0 km
    let ceres_orbit_speed = 0.0326;
    let ceres_angle = next_orbit_angle();
    let ceres_pos = Vec3::new(ceres_orbit_radius * ceres_angle.cos(), 0.0, ceres_orbit_radius * ceres_angle.sin());
    let ceres_tex: Handle<Image> = asset_server.load("textures/ceres.jpg");
    let ceres_mesh = meshes.add(create_uv_sphere(473.0, 96, 48));
    let ceres_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.32, 0.32),
        base_color_texture: Some(ceres_tex.clone()),
        perceptual_roughness: 0.95,
        reflectance: 0.06,
        ..default()
    });
    let ceres_entity = commands.spawn((
        Planet {
            name: "Ceres",
            index: 10,
            radius: 473.0,
            orbit_radius: ceres_orbit_radius,
            orbit_speed: ceres_orbit_speed,
            orbit_angle: ceres_angle,
            rotation_speed: 0.005, // 9h spin
            world_pos: ceres_pos,
        },
        PlanetLod::new(10, false, "", 8.0, ceres_tex.clone(), 96, 48),
        Mesh3d(ceres_mesh),
        MeshMaterial3d(ceres_mat),
        Transform::from_translation(ceres_pos).with_scale(Vec3::new(1.0, 0.923, 1.0)),
    )).id();
    spawn_planet_area_light(&mut commands, ceres_entity, ceres_pos, 473.0);

    // --- MAIN ASTEROID BELT (2.1 AU to 3.3 AU: ~500,000 - ~800,000 KM) ---
    let asteroid_tex: Handle<Image> = asset_server.load("textures/asteroid.jpg");
    let asteroid_base_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asteroid_tex),
        perceptual_roughness: 0.95,
        reflectance: 0.08,
        ..default()
    });

    // Create 6 distinct deformed asteroid shapes
    let mut asteroid_meshes = Vec::new();
    let shape_seeds = [12345u64, 67890u64, 13579u64, 24680u64, 98765u64, 43210u64];

    for &seed in &shape_seeds {
        let mut mesh = create_uv_sphere(1.0, 16, 12);
        if let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        {
            let mut s = seed;
            for pos in positions.iter_mut() {
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
                let noise = 0.75 + ((s % 100) as f32 / 100.0) * 0.5;
                pos[0] *= noise;
                pos[1] *= noise * 0.85;
                pos[2] *= noise * 1.1;
            }
        }
        asteroid_meshes.push(meshes.add(mesh));
    }

    let mut belt_rng: u64 = 888777666;
    let num_asteroids = 450;

    for i in 0..num_asteroids {
        belt_rng = belt_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let angle = (belt_rng as f32 / u64::MAX as f32) * std::f32::consts::TAU;

        belt_rng = belt_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let dist = (2.2 * AU) + (belt_rng as f32 / u64::MAX as f32) * (1.0 * AU);

        belt_rng = belt_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let y_offset = ((belt_rng as f32 / u64::MAX as f32) * 2.0 - 1.0) * 8_000.0;

        let ast_x = angle.cos() * dist;
        let ast_z = angle.sin() * dist;
        let ast_pos = Vec3::new(ast_x, y_offset, ast_z);

        belt_rng = belt_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let ast_radius = 20.0 + (belt_rng as f32 / u64::MAX as f32) * 80.0;

        belt_rng = belt_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let rx = (belt_rng as f32 / u64::MAX as f32) * 2.0 - 1.0;
        belt_rng = belt_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let ry = (belt_rng as f32 / u64::MAX as f32) * 2.0 - 1.0;
        belt_rng = belt_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let rz = (belt_rng as f32 / u64::MAX as f32) * 2.0 - 1.0;
        let rot_axis = Vec3::new(rx, ry, rz).normalize_or_zero();

        belt_rng = belt_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let rot_speed = 0.005 + (belt_rng as f32 / u64::MAX as f32) * 0.025;

        let mesh_handle = asteroid_meshes[i % asteroid_meshes.len()].clone();

        commands.spawn((
            Asteroid {
                radius: ast_radius,
                rotation_axis: rot_axis,
                rotation_speed: rot_speed,
                world_pos: ast_pos,
            },
            Mesh3d(mesh_handle),
            MeshMaterial3d(asteroid_base_mat.clone()),
            Transform::from_translation(ast_pos).with_scale(Vec3::splat(ast_radius)),
        ));
    }

    // --- PLANET 5: JUPITER & GALILEAN MOONS (5.2044 AU) ---
    let jupiter_orbit_radius = 5.204_4 * AU; // 778,567,160.0 km
    let jupiter_orbit_speed = 0.012645; // 0.0843x Earth speed
    let jupiter_angle = next_orbit_angle();
    let jupiter_pos = Vec3::new(jupiter_orbit_radius * jupiter_angle.cos(), 0.0, jupiter_orbit_radius * jupiter_angle.sin());
    let jupiter_tex: Handle<Image> = asset_server.load("textures/jupiter.jpg");
    let jupiter_mesh = meshes.add(create_uv_sphere(69911.0, 384, 192));
    let jupiter_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.35, 0.32),
        base_color_texture: Some(jupiter_tex.clone()),
        perceptual_roughness: 0.88,
        reflectance: 0.10,
        ..default()
    });
    let jupiter_entity = commands.spawn((
        Planet {
            name: "Jupiter",
            index: 5,
            radius: 69911.0,
            orbit_radius: jupiter_orbit_radius,
            orbit_speed: jupiter_orbit_speed,
            orbit_angle: jupiter_angle,
            rotation_speed: 0.00726, // 9.92h fast spin
            world_pos: jupiter_pos,
        },
        PlanetLod::new(5, false, "", 40.0, jupiter_tex.clone(), 384, 192),
        Mesh3d(jupiter_mesh),
        MeshMaterial3d(jupiter_mat),
        Transform::from_translation(jupiter_pos).with_scale(Vec3::new(1.0, 0.935, 1.0)), // Oblate Spheroid (f=0.0649)
    )).id();
    spawn_planet_area_light(&mut commands, jupiter_entity, jupiter_pos, 69911.0);

    // Io
    let io_orbit_radius = 421700.0; // 421,700 km orbit around Jupiter
    let io_orbit_speed = 0.45;
    let io_angle = next_orbit_angle();
    let io_pos = jupiter_pos + Vec3::new(io_orbit_radius * io_angle.cos(), 0.0, io_orbit_radius * io_angle.sin());
    let io_mesh = meshes.add(create_uv_sphere(1821.6, 192, 96));
    let io_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.85, 0.2),
        perceptual_roughness: 0.7,
        reflectance: 0.20,
        ..default()
    });
    let io_entity = commands.spawn((
        Moon {
            name: "Io",
            parent_index: 5,
            radius: 1821.6,
            orbit_radius: io_orbit_radius,
            orbit_speed: io_orbit_speed,
            orbit_angle: io_angle,
            rotation_speed: 0.00169, // Tidally locked
            world_pos: io_pos,
        },
        PlanetLod::new(5, true, "Io", 11.25, jupiter_tex.clone(), 192, 96),
        Mesh3d(io_mesh),
        MeshMaterial3d(io_mat),
        Transform::from_translation(io_pos),
    )).id();
    spawn_planet_area_light(&mut commands, io_entity, io_pos, 1821.6);

    // Europa
    let europa_orbit_radius = 670900.0; // 670,900 km orbit around Jupiter
    let europa_orbit_speed = 0.32;
    let europa_angle = next_orbit_angle();
    let europa_pos = jupiter_pos + Vec3::new(europa_orbit_radius * europa_angle.cos(), 0.0, europa_orbit_radius * europa_angle.sin());
    let europa_mesh = meshes.add(create_uv_sphere(1560.8, 192, 96));
    let europa_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.82, 0.85, 0.92),
        perceptual_roughness: 0.65,
        reflectance: 0.25,
        ..default()
    });
    let europa_entity = commands.spawn((
        Moon {
            name: "Europa",
            parent_index: 5,
            radius: 1560.8,
            orbit_radius: europa_orbit_radius,
            orbit_speed: europa_orbit_speed,
            orbit_angle: europa_angle,
            rotation_speed: 0.00084, // Tidally locked
            world_pos: europa_pos,
        },
        PlanetLod::new(5, true, "Europa", 8.75, jupiter_tex.clone(), 192, 96),
        Mesh3d(europa_mesh),
        MeshMaterial3d(europa_mat),
        Transform::from_translation(europa_pos),
    )).id();
    spawn_planet_area_light(&mut commands, europa_entity, europa_pos, 1560.8);

    // --- PLANET 6: SATURN & REALISTIC 2D RING SYSTEM WITH DUST & ROCKS (9.5826 AU) ---
    let saturn_orbit_radius = 9.582_6 * AU; // 1,433,536,554.0 km
    let saturn_orbit_speed = 0.005085; // 0.0339x Earth speed
    let saturn_angle = next_orbit_angle();
    let saturn_pos = Vec3::new(saturn_orbit_radius * saturn_angle.cos(), 0.0, saturn_orbit_radius * saturn_angle.sin());
    let saturn_tex: Handle<Image> = asset_server.load("textures/saturn.jpg");
    let saturn_mesh = meshes.add(create_uv_sphere(58232.0, 384, 192));
    let saturn_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.33, 0.28),
        base_color_texture: Some(saturn_tex.clone()),
        perceptual_roughness: 0.88,
        reflectance: 0.10,
        ..default()
    });
    let saturn_entity = commands
        .spawn((
            Planet {
                name: "Saturn",
                index: 6,
                radius: 58232.0,
                orbit_radius: saturn_orbit_radius,
                orbit_speed: saturn_orbit_speed,
                orbit_angle: saturn_angle,
                rotation_speed: 0.00683, // 10.55h spin
                world_pos: saturn_pos,
            },
            PlanetLod::new(6, false, "", 35.0, saturn_tex.clone(), 384, 192),
            Mesh3d(saturn_mesh),
            MeshMaterial3d(saturn_mat),
            Transform::from_translation(saturn_pos).with_scale(Vec3::new(1.0, 0.902, 1.0)), // Oblate Spheroid (f=0.0980)
        ))
        .id();
    spawn_planet_area_light(&mut commands, saturn_entity, saturn_pos, 58232.0);

    // Saturn Ring System (Transparent 2D Ring Plane Disk with Radial Texture: Real 74,500 km to 136,775 km boundary)
    let saturn_ring_tex: Handle<Image> = asset_server.load("textures/saturn_ring.png");
    let ring_plane_mesh = meshes.add(create_flat_ring_mesh(74500.0, 136775.0, 256, 16));
    let ring_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.38, 0.35, 0.30, 0.80),
        base_color_texture: Some(saturn_ring_tex.clone()),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        double_sided: true,
        perceptual_roughness: 0.85,
        reflectance: 0.10,
        ..default()
    });

    let ring_tilt = Quat::from_rotation_x(0.465); // Realistic ~26.7 degree axial tilt

    let ring_plane = commands
        .spawn((
            Mesh3d(ring_plane_mesh),
            MeshMaterial3d(ring_mat),
            Transform::from_rotation(ring_tilt),
        ))
        .id();
    commands.entity(saturn_entity).add_child(ring_plane);

    // Saturn Ring Dust & Rock Particles embedded inside the 2D ring plane
    let ring_rock_mesh = meshes.add(create_uv_sphere(1.0, 12, 8));
    let ring_rock_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/asteroid.jpg")),
        perceptual_roughness: 0.8,
        reflectance: 0.1,
        ..default()
    });

    let mut ring_rng: u64 = 999111222;
    for _ in 0..180 {
        ring_rng = ring_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let angle = (ring_rng as f32 / u64::MAX as f32) * std::f32::consts::TAU;

        ring_rng = ring_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let rad = 76000.0 + (ring_rng as f32 / u64::MAX as f32) * 58000.0;

        ring_rng = ring_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let y_off = ((ring_rng as f32 / u64::MAX as f32) * 2.0 - 1.0) * 30.0;

        ring_rng = ring_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let rock_scale = 7.5 + (ring_rng as f32 / u64::MAX as f32) * 30.0;

        let local_pos = Vec3::new(angle.cos() * rad, y_off, angle.sin() * rad);
        let tilted_pos = ring_tilt * local_pos;

        let dust_p = commands
            .spawn((
                Mesh3d(ring_rock_mesh.clone()),
                MeshMaterial3d(ring_rock_mat.clone()),
                Transform::from_translation(tilted_pos).with_scale(Vec3::splat(rock_scale)),
            ))
            .id();
        commands.entity(saturn_entity).add_child(dust_p);
    }

    // --- PLANET 7: URANUS (19.201 AU) ---
    let uranus_orbit_radius = 19.201 * AU; // 2,872,428,715.0 km
    let uranus_orbit_speed = 0.001785; // 0.0119x Earth speed
    let uranus_angle = next_orbit_angle();
    let uranus_pos = Vec3::new(uranus_orbit_radius * uranus_angle.cos(), 0.0, uranus_orbit_radius * uranus_angle.sin());
    let uranus_tex: Handle<Image> = asset_server.load("textures/uranus.jpg");
    let uranus_mesh = meshes.add(create_uv_sphere(25362.0, 256, 128));
    let uranus_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.40, 0.42),
        base_color_texture: Some(uranus_tex.clone()),
        perceptual_roughness: 0.85,
        reflectance: 0.10,
        ..default()
    });
    let uranus_entity = commands.spawn((
        Planet {
            name: "Uranus",
            index: 7,
            radius: 25362.0,
            orbit_radius: uranus_orbit_radius,
            orbit_speed: uranus_orbit_speed,
            orbit_angle: uranus_angle,
            rotation_speed: -0.00418, // Retrograde spin 17.24h
            world_pos: uranus_pos,
        },
        PlanetLod::new(7, false, "", 20.0, uranus_tex.clone(), 256, 128),
        Mesh3d(uranus_mesh),
        MeshMaterial3d(uranus_mat),
        Transform::from_translation(uranus_pos).with_scale(Vec3::new(1.0, 0.977, 1.0)),
    )).id();
    spawn_planet_area_light(&mut commands, uranus_entity, uranus_pos, 25362.0);

    // --- PLANET 8: NEPTUNE (30.047 AU) ---
    let neptune_orbit_radius = 30.047 * AU; // 4,494,967,221.0 km
    let neptune_orbit_speed = 0.000915; // 0.0061x Earth speed
    let neptune_angle = next_orbit_angle();
    let neptune_pos = Vec3::new(neptune_orbit_radius * neptune_angle.cos(), 0.0, neptune_orbit_radius * neptune_angle.sin());
    let neptune_tex: Handle<Image> = asset_server.load("textures/neptune.jpg");
    let neptune_mesh = meshes.add(create_uv_sphere(24622.0, 256, 128));
    let neptune_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.35, 0.45),
        base_color_texture: Some(neptune_tex.clone()),
        perceptual_roughness: 0.85,
        reflectance: 0.10,
        ..default()
    });
    let neptune_entity = commands.spawn((
        Planet {
            name: "Neptune",
            index: 8,
            radius: 24622.0,
            orbit_radius: neptune_orbit_radius,
            orbit_speed: neptune_orbit_speed,
            orbit_angle: neptune_angle,
            rotation_speed: 0.00447, // 16.11h spin
            world_pos: neptune_pos,
        },
        PlanetLod::new(8, false, "", 20.0, neptune_tex.clone(), 256, 128),
        Mesh3d(neptune_mesh),
        MeshMaterial3d(neptune_mat),
        Transform::from_translation(neptune_pos).with_scale(Vec3::new(1.0, 0.983, 1.0)),
    )).id();
    spawn_planet_area_light(&mut commands, neptune_entity, neptune_pos, 24622.0);

    // --- DWARF PLANET 2: PLUTO & MOON CHARON (39.482 AU) ---
    let pluto_orbit_radius = 39.482 * AU; // 5,906,382,920.0 km
    let pluto_orbit_speed = 0.00060; // 0.0040x Earth speed
    let pluto_angle = next_orbit_angle();
    let pluto_pos = Vec3::new(pluto_orbit_radius * pluto_angle.cos(), 0.0, pluto_orbit_radius * pluto_angle.sin());
    let pluto_tex: Handle<Image> = asset_server.load("textures/pluto.jpg");
    let pluto_mesh = meshes.add(create_uv_sphere(1188.3, 192, 96));
    let pluto_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.35, 0.32),
        base_color_texture: Some(pluto_tex.clone()),
        perceptual_roughness: 0.88,
        reflectance: 0.10,
        ..default()
    });
    let pluto_entity = commands.spawn((
        Planet {
            name: "Pluto",
            index: 9,
            radius: 1188.3,
            orbit_radius: pluto_orbit_radius,
            orbit_speed: pluto_orbit_speed,
            orbit_angle: pluto_angle,
            rotation_speed: 0.00047, // 6.39d spin
            world_pos: pluto_pos,
        },
        PlanetLod::new(9, false, "", 10.0, pluto_tex.clone(), 192, 96),
        Mesh3d(pluto_mesh),
        MeshMaterial3d(pluto_mat),
        Transform::from_translation(pluto_pos),
    )).id();
    spawn_planet_area_light(&mut commands, pluto_entity, pluto_pos, 1188.3);

    // Charon
    let charon_orbit_radius = 19591.0; // 19,591 km orbit
    let charon_orbit_speed = 0.2;
    let charon_angle = next_orbit_angle();
    let charon_pos = pluto_pos + Vec3::new(charon_orbit_radius * charon_angle.cos(), 0.0, charon_orbit_radius * charon_angle.sin());
    let charon_mesh = meshes.add(create_uv_sphere(606.0, 192, 96));
    let charon_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.48, 0.45),
        perceptual_roughness: 0.9,
        reflectance: 0.12,
        ..default()
    });
    let charon_entity = commands.spawn((
        Moon {
            name: "Charon",
            parent_index: 9,
            radius: 606.0,
            orbit_radius: charon_orbit_radius,
            orbit_speed: charon_orbit_speed,
            orbit_angle: charon_angle,
            rotation_speed: 0.00047, // Tidally locked
            world_pos: charon_pos,
        },
        PlanetLod::new(9, true, "Charon", 6.25, pluto_tex.clone(), 192, 96),
        Mesh3d(charon_mesh),
        MeshMaterial3d(charon_mat),
        Transform::from_translation(charon_pos),
    )).id();
    spawn_planet_area_light(&mut commands, charon_entity, charon_pos, 606.0);

    // --- DWARF PLANET 3: HAUMEA (43.218 AU) ---
    let haumea_orbit_radius = 43.218 * AU; // 6,465,321,155.0 km
    let haumea_orbit_speed = 0.000525; // 0.0035x Earth speed
    let haumea_angle = next_orbit_angle();
    let haumea_pos = Vec3::new(haumea_orbit_radius * haumea_angle.cos(), 0.0, haumea_orbit_radius * haumea_angle.sin());
    let haumea_tex: Handle<Image> = asset_server.load("textures/haumea.jpg");
    let haumea_mesh = meshes.add(create_uv_sphere(1050.0, 192, 96));
    let haumea_mat = materials.add(StandardMaterial {
        base_color_texture: Some(haumea_tex.clone()),
        perceptual_roughness: 0.75,
        reflectance: 0.18,
        ..default()
    });
    let haumea_entity = commands.spawn((
        Planet {
            name: "Haumea",
            index: 11,
            radius: 1050.0,
            orbit_radius: haumea_orbit_radius,
            orbit_speed: haumea_orbit_speed,
            orbit_angle: haumea_angle,
            rotation_speed: 0.01840, // Ultra-fast 3.91h spin!
            world_pos: haumea_pos,
        },
        PlanetLod::new(11, false, "", 8.75, haumea_tex.clone(), 192, 96),
        Mesh3d(haumea_mesh),
        MeshMaterial3d(haumea_mat),
        Transform::from_translation(haumea_pos).with_scale(Vec3::new(1.35, 0.85, 1.0)), // Triaxial Ellipsoid (1,050 x 840 x 537 km)
    )).id();
    spawn_planet_area_light(&mut commands, haumea_entity, haumea_pos, 1050.0);

    // --- DWARF PLANET 4: MAKEMAKE (45.563 AU) ---
    let makemake_orbit_radius = 45.563 * AU; // 6,816,027,787.0 km
    let makemake_orbit_speed = 0.00048; // 0.0032x Earth speed
    let makemake_angle = next_orbit_angle();
    let makemake_pos = Vec3::new(makemake_orbit_radius * makemake_angle.cos(), 0.0, makemake_orbit_radius * makemake_angle.sin());
    let makemake_tex: Handle<Image> = asset_server.load("textures/makemake.jpg");
    let makemake_mesh = meshes.add(create_uv_sphere(715.0, 192, 96));
    let makemake_mat = materials.add(StandardMaterial {
        base_color_texture: Some(makemake_tex.clone()),
        perceptual_roughness: 0.75,
        reflectance: 0.18,
        ..default()
    });
    let makemake_entity = commands.spawn((
        Planet {
            name: "Makemake",
            index: 12,
            radius: 715.0,
            orbit_radius: makemake_orbit_radius,
            orbit_speed: makemake_orbit_speed,
            orbit_angle: makemake_angle,
            rotation_speed: 0.00320, // 22.48h spin
            world_pos: makemake_pos,
        },
        PlanetLod::new(12, false, "", 7.5, makemake_tex.clone(), 192, 96),
        Mesh3d(makemake_mesh),
        MeshMaterial3d(makemake_mat),
        Transform::from_translation(makemake_pos),
    )).id();
    spawn_planet_area_light(&mut commands, makemake_entity, makemake_pos, 715.0);

    // --- DWARF PLANET 5: ERIS (67.781 AU) ---
    let eris_orbit_radius = 67.781 * AU; // 10,139,893,275.0 km
    let eris_orbit_speed = 0.00027; // 0.0018x Earth speed
    let eris_angle = next_orbit_angle();
    let eris_pos = Vec3::new(eris_orbit_radius * eris_angle.cos(), 0.0, eris_orbit_radius * eris_angle.sin());
    let eris_tex: Handle<Image> = asset_server.load("textures/eris.jpg");
    let eris_mesh = meshes.add(create_uv_sphere(1163.0, 192, 96));
    let eris_mat = materials.add(StandardMaterial {
        base_color_texture: Some(eris_tex.clone()),
        perceptual_roughness: 0.75,
        reflectance: 0.18,
        ..default()
    });
    let eris_entity = commands.spawn((
        Planet {
            name: "Eris",
            index: 13,
            radius: 1163.0,
            orbit_radius: eris_orbit_radius,
            orbit_speed: eris_orbit_speed,
            orbit_angle: eris_angle,
            rotation_speed: 0.00278, // 25.90h spin
            world_pos: eris_pos,
        },
        PlanetLod::new(13, false, "", 7.5, eris_tex.clone(), 192, 96),
        Mesh3d(eris_mesh),
        MeshMaterial3d(eris_mat),
        Transform::from_translation(eris_pos),
    )).id();
    spawn_planet_area_light(&mut commands, eris_entity, eris_pos, 1163.0);

    // ----------------------------------------------------
    // 4. INTERPLANETARY SPACE DUST PARTICLES (SPRINKLED AROUND SYSTEM)
    // ----------------------------------------------------
    let dust_particle_mesh = meshes.add(Sphere::new(5.0));
    let dust_particle_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.8, 0.9, 1.0, 0.4),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    let mut dust_rng: u64 = 555444333;
    let num_dust = 350;

    for _ in 0..num_dust {
        dust_rng = dust_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let theta = (dust_rng as f32 / u64::MAX as f32) * std::f32::consts::TAU;

        dust_rng = dust_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let dist = (0.8 * AU) + (dust_rng as f32 / u64::MAX as f32) * (60.0 * AU);

        dust_rng = dust_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let y = ((dust_rng as f32 / u64::MAX as f32) * 2.0 - 1.0) * 200_000.0;

        let dx = theta.cos() * dist;
        let dz = theta.sin() * dist;

        dust_rng = dust_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let scale = 1.0 + (dust_rng as f32 / u64::MAX as f32) * 4.0;

        commands.spawn((
            SpaceDust {
                world_pos: Vec3::new(dx, y, dz),
                size_scale: scale,
            },
            Mesh3d(dust_particle_mesh.clone()),
            MeshMaterial3d(dust_particle_mat.clone()),
            Transform::from_xyz(dx, y, dz).with_scale(Vec3::splat(scale)),
        ));
    }

    // ----------------------------------------------------
    // 5. DEEP SPACE SKYBOX SPHERE (SpaceSpheremaps Spheremap) & STARFIELD
    // ----------------------------------------------------
    let skybox_tex: Handle<Image> = asset_server.load("textures/space_skybox.png");
    let skybox_mesh = meshes.add(create_uv_sphere(900_000.0, 128, 64));

    let skybox_mat = materials.add(StandardMaterial {
        base_color_texture: Some(skybox_tex),
        unlit: true,
        cull_mode: None,
        double_sided: true,
        ..default()
    });

    commands.spawn((
        SkyboxSphere,
        Mesh3d(skybox_mesh),
        MeshMaterial3d(skybox_mat),
        NoFrustumCulling,
        Transform::from_scale(Vec3::new(-1.0, 1.0, 1.0)),
    ));

    let star_mesh_small = meshes.add(Circle::new(14.0));
    let star_mesh_medium = meshes.add(Circle::new(28.0));
    let star_mesh_large = meshes.add(Circle::new(48.0));

    let star_colors = [
        Color::linear_rgb(25.0, 32.0, 50.0), // Ice Blue Diamond Star
        Color::linear_rgb(45.0, 45.0, 55.0), // Radiant Pure White Core Star
        Color::linear_rgb(50.0, 40.0, 20.0), // Solar Gold Star
        Color::linear_rgb(55.0, 20.0, 12.0), // Crimson Supergiant Star
        Color::linear_rgb(30.0, 42.0, 60.0), // Electric Cyan Star
        Color::linear_rgb(40.0, 35.0, 50.0), // Vivid Violet-White Star
    ];
    let star_mats: Vec<Handle<StandardMaterial>> = star_colors
        .iter()
        .map(|c| {
            materials.add(StandardMaterial {
                base_color: *c,
                emissive: LinearRgba::new(c.to_linear().red * 3.0, c.to_linear().green * 3.0, c.to_linear().blue * 3.0, 1.0),
                unlit: true,
                cull_mode: None,
                double_sided: true,
                ..default()
            })
        })
        .collect();

    let mut rng_seed: u64 = 987654321;
    let num_stars = 3600;
    let star_meshes = [star_mesh_small.clone(), star_mesh_medium.clone(), star_mesh_large.clone()];

    for i in 0..num_stars {
        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let theta = (rng_seed as f32 / u64::MAX as f32) * std::f32::consts::TAU;

        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let phi = ((rng_seed as f32 / u64::MAX as f32) * 2.0 - 1.0).acos();

        let dir = Vec3::new(
            phi.sin() * theta.cos(),
            phi.sin() * theta.sin(),
            phi.cos(),
        ).normalize();

        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let size_scale = 0.8 + (rng_seed as f32 / u64::MAX as f32) * 2.2;
        let mat_handle = star_mats[i % star_mats.len()].clone();
        let mesh_handle = star_meshes[i % star_meshes.len()].clone();

        let d_vis = 85_000.0;
        let initial_pos = dir * d_vis;

        commands.spawn((
            Starfield {
                direction: dir,
                size_scale,
            },
            Mesh3d(mesh_handle),
            MeshMaterial3d(mat_handle),
            Transform::from_translation(initial_pos).looking_at(Vec3::ZERO, Vec3::Y),
        ));
    }
}

pub fn create_uv_sphere(radius: f32, sectors: u32, stacks: u32) -> Mesh {
    use bevy::render::mesh::VertexAttributeValues;

    let mut mesh = Sphere::new(radius)
        .mesh()
        .kind(bevy::render::mesh::SphereKind::Uv { sectors, stacks })
        .build();

    if let Some(VertexAttributeValues::Float32x3(positions)) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
        for p in positions.iter_mut() {
            // Swap Y and Z so the UV sphere polar axis (+Z in Bevy UvSphere) aligns with +Y (Up)
            p.swap(1, 2);
        }
    }

    if let Some(VertexAttributeValues::Float32x3(normals)) = mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL) {
        for n in normals.iter_mut() {
            n.swap(1, 2);
        }
    }

    mesh
}

pub fn create_flat_ring_mesh(inner_radius: f32, outer_radius: f32, sectors: u32, rings: u32) -> Mesh {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::mesh::{Indices, PrimitiveTopology};

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for r in 0..=rings {
        let radius_ratio = r as f32 / rings as f32;
        let current_radius = inner_radius + (outer_radius - inner_radius) * radius_ratio;
        let u = radius_ratio;

        for s in 0..=sectors {
            let theta = (s as f32 / sectors as f32) * std::f32::consts::TAU;
            let x = theta.cos() * current_radius;
            let z = theta.sin() * current_radius;
            let y = 0.0;

            positions.push([x, y, z]);
            normals.push([0.0, 1.0, 0.0]);
            let v = s as f32 / sectors as f32;
            uvs.push([u, v]);
        }
    }

    let ring_verts = sectors + 1;
    for r in 0..rings {
        for s in 0..sectors {
            let current = r * ring_verts + s;
            let next = current + ring_verts;

            indices.push(current);
            indices.push(next);
            indices.push(current + 1);

            indices.push(current + 1);
            indices.push(next);
            indices.push(next + 1);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub fn spawn_planet_area_light(
    commands: &mut Commands,
    parent_entity: Entity,
    world_pos: Vec3,
    radius: f32,
) {
    let sun_dir_world = -world_pos.normalize_or_zero();
    let initial_local_pos = sun_dir_world * (radius * 3.5);

    let light_entity = commands
        .spawn((
            PlanetAreaLight {
                destination_world_pos: world_pos,
                planet_radius: radius,
            },
            PointLight {
                intensity: 0.0,
                color: Color::srgb(1.0, 0.96, 0.88),
                range: 0.0,
                radius: 0.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(initial_local_pos),
        ))
        .id();
    commands.entity(parent_entity).add_child(light_entity);
}

#[derive(Component)]
pub struct LoadingScreenUI;

#[derive(Component)]
pub struct LoadingTextUI;

pub fn setup_loading_screen(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut loading_assets: ResMut<LoadingAssets>,
) {
    ensure_engine_hum_file();
    ensure_ambient_piano_file();

    let asset_paths = [
        "audio/engine_hum.wav",
        "audio/ambient_piano.wav",
        "models/spaceship.glb",
        "textures/space_skybox.png",
        "textures/sun.jpg",
        "textures/mercury.jpg",
        "textures/venus.jpg",
        "textures/earth.jpg",
        "textures/moon.jpg",
        "textures/mars.jpg",
        "textures/ceres.jpg",
        "textures/asteroid.jpg",
        "textures/jupiter.jpg",
        "textures/saturn.jpg",
        "textures/saturn_ring.png",
        "textures/uranus.jpg",
        "textures/neptune.jpg",
        "textures/pluto.jpg",
        "textures/haumea.jpg",
        "textures/makemake.jpg",
        "textures/eris.jpg",
    ];

    loading_assets.handles.clear();
    for path in asset_paths {
        let untyped = if path.ends_with(".wav") {
            asset_server.load::<AudioSource>(path).untyped()
        } else if path.ends_with(".glb") {
            asset_server.load::<bevy::gltf::Gltf>(path).untyped()
        } else {
            asset_server.load::<Image>(path).untyped()
        };
        loading_assets.handles.push(untyped);
    }

    for i in 0..16 {
        let anim_path = format!("textures/sun_anim/frame_{:02}.jpg", i);
        loading_assets.handles.push(asset_server.load::<Image>(&anim_path).untyped());
    }

    commands.spawn((
        Camera {
            order: 100,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.001, 0.001, 0.003)),
            ..default()
        },
        Camera2d::default(),
        LoadingScreenUI,
    ));

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.001, 0.001, 0.003, 1.0)),
            LoadingScreenUI,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("SPACE EXPLORER - SOLAR SYSTEM SIMULATOR"),
                TextFont {
                    font_size: 28.0.into(),
                    ..default()
                },
                TextColor(Color::srgb(0.0, 0.85, 1.0)),
            ));

            parent.spawn((
                LoadingTextUI,
                Text::new("PRELOADING SOLAR SYSTEM RESOURCES (0%)..."),
                TextFont {
                    font_size: 16.0.into(),
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.85, 0.9)),
            ));
        });
}

pub fn check_loading_status(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    loading_assets: Res<LoadingAssets>,
    mut next_state: ResMut<NextState<AppState>>,
    loading_ui_query: Query<Entity, With<LoadingScreenUI>>,
    mut text_query: Query<&mut Text, With<LoadingTextUI>>,
) {
    let total = loading_assets.handles.len();
    if total == 0 {
        next_state.set(AppState::InGame);
        return;
    }

    let mut loaded_count = 0;
    for handle in &loading_assets.handles {
        let state = asset_server.load_state(handle.id());
        if matches!(state, bevy::asset::LoadState::Loaded | bevy::asset::LoadState::Failed(_)) {
            loaded_count += 1;
        }
    }

    let pct = (loaded_count as f32 / total as f32) * 100.0;
    for mut text in &mut text_query {
        **text = format!("PRELOADING SOLAR SYSTEM RESOURCES: {}/{} ({:.0}%)", loaded_count, total, pct);
    }

    if loaded_count >= total {
        for entity in &loading_ui_query {
            commands.entity(entity).despawn();
        }
        next_state.set(AppState::InGame);
    }
}

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Loading), setup_loading_screen)
            .add_systems(
                Update,
                check_loading_status.run_if(in_state(AppState::Loading)),
            )
            .add_systems(OnEnter(AppState::InGame), setup_scene);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::mesh::VertexAttributeValues;

    #[test]
    fn test_sphere_uvs() {
        let radius = 1.0;
        let mesh = create_uv_sphere(radius, 16, 8);

        let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
            VertexAttributeValues::Float32x3(p) => p,
            _ => panic!(),
        };
        let uvs = match mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap() {
            VertexAttributeValues::Float32x2(u) => u,
            _ => panic!(),
        };

        let mut found_north_pole = false;
        let mut found_south_pole = false;

        for (p, uv) in positions.iter().zip(uvs.iter()) {
            if (uv[1] - 0.0).abs() < 1e-4 {
                // North Pole (v=0.0) must map to +Y position [0, +radius, 0]
                assert!((p[1] - radius).abs() < 1e-4, "North pole v=0.0 must map to +Y position [0, {}, 0], got {:?}", radius, p);
                found_north_pole = true;
            } else if (uv[1] - 1.0).abs() < 1e-4 {
                // South Pole (v=1.0) must map to -Y position [0, -radius, 0]
                assert!((p[1] + radius).abs() < 1e-4, "South pole v=1.0 must map to -Y position [0, -{}, 0], got {:?}", radius, p);
                found_south_pole = true;
            }
        }

        assert!(found_north_pole, "Sphere mesh must contain North pole vertices (v=0.0)");
        assert!(found_south_pole, "Sphere mesh must contain South pole vertices (v=1.0)");
    }

    #[test]
    fn test_sun_anim_frame_00_validity() {
        let frame_path = std::path::Path::new("assets/textures/sun_anim/frame_00.jpg");
        assert!(frame_path.exists(), "frame_00.jpg should exist");
        let metadata = std::fs::metadata(frame_path).expect("Read metadata");
        assert!(
            metadata.len() > 300_000,
            "frame_00.jpg should be a full valid texture (>300KB), found {} bytes",
            metadata.len()
        );
    }

    #[test]
    fn test_initial_ship_spawn_position_facing_earth() {
        let earth_orbit_radius = 1.000000 * AU;
        let earth_angle = 0.5 * std::f32::consts::TAU;
        let earth_pos = Vec3::new(earth_orbit_radius * earth_angle.cos(), 0.0, earth_orbit_radius * earth_angle.sin());

        let initial_spawn_dist = 2.200000 * AU;
        let initial_spawn_pos = earth_pos.normalize() * initial_spawn_dist + Vec3::new(0.0, 150_000.0, 0.0);
        let dir_to_earth = (earth_pos - initial_spawn_pos).normalize_or_zero();

        let dist_to_sun = initial_spawn_pos.length();
        assert!(
            dist_to_sun > 2.0 * AU,
            "Ship spawn position should be >2.0 AU from Sun, found {:.2} AU",
            dist_to_sun / AU
        );

        let ship_rot = Quat::from_rotation_arc(Vec3::NEG_Z, dir_to_earth);
        let ship_forward = ship_rot * Vec3::NEG_Z;
        let dot_to_earth = ship_forward.dot(dir_to_earth);
        assert!(
            (dot_to_earth - 1.0).abs() < 1e-4,
            "Ship forward vector should face Earth directly, dot product={}",
            dot_to_earth
        );
    }
}

