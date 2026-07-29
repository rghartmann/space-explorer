use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::CascadeShadowConfigBuilder;
use bevy::post_process::bloom::{Bloom, BloomPrefilter};
use bevy::prelude::*;

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
                Text::new("FLIGHT CONTROLS: W/S (Accel/Decel) | MOUSE (Steer Pitch/Yaw) | A/D (Orbit Yaw) | Z/C (Roll) | Q/E (Orbit Range) | SPACE (Warp Boost) | [O] Enter/Leave Orbit | ESC (Exit)"),
                TextFont {
                    font_size: 11.5.into(),
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.8, 0.9)),
            ));

            parent.spawn((
                AutoPilotHudText,
                Text::new("FLIGHT STATUS: MANUAL CONTROL | SPEED: 0 km/s | PRESS [0-9/C/H/K/E/M] TO ENGAGE AUTOPILOT | PRESS [O] TO ENTER/LEAVE ORBIT"),
                TextFont {
                    font_size: 13.0.into(),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.85, 0.2)),
            ));

            // Line 4: Orbit Mode indicator text following the exact pattern of other HUD control labels (Red font)
            parent.spawn((
                OrbitModeBanner,
                OrbitModeInfoText,
                Text::new("ORBIT MODE: IN ORBIT MODE | DESTINATION: EARTH | SPEED: 1.00x | CONTROLS: [W/S] Speed | [A/D] Yaw | [Z/C] Roll | [Q/E] Range | [O] Exit Orbit"),
                TextFont {
                    font_size: 12.0.into(),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.25, 0.25)), // Red font
                Node {
                    display: Display::None,
                    ..default()
                },
                Visibility::Hidden,
            ));
        });

    // CENTER SCREEN NOTIFICATION: "Entering Orbit Mode" (brief red label when autopilot reaches destination)
    commands
        .spawn((
            EnteringOrbitLabel,
            Node {
                position_type: PositionType::Absolute,
                justify_self: JustifySelf::Center,
                align_self: AlignSelf::Center,
                padding: UiRect::axes(Val::Px(22.0), Val::Px(10.0)),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.85, 0.1, 0.1, 0.95)),
            BorderColor::all(Color::srgba(1.0, 0.4, 0.4, 1.0)),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Entering Orbit Mode"),
                TextFont {
                    font_size: 22.0.into(),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
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
    // SHIP AVATAR & 3RD-PERSON CAMERA PERSPECTIVE
    // ----------------------------------------------------
    // Calculate initial ship position near Earth facing the Sun at Vec3::ZERO
    let temp_earth_radius = 1.000000 * 240_000.0;
    let temp_earth_angle = next_orbit_angle();
    let temp_earth_pos = Vec3::new(temp_earth_radius * temp_earth_angle.cos(), 0.0, temp_earth_radius * temp_earth_angle.sin());
    let initial_spawn_pos = temp_earth_pos + Vec3::new(8_000.0, 400.0, 2_500.0);
    let dir_to_sun = (Vec3::ZERO - initial_spawn_pos).normalize_or_zero();
    let initial_ship_rot = Quat::from_rotation_arc(Vec3::NEG_Z, dir_to_sun);

    let ship_entity = commands
        .spawn((
            Ship,
            Transform::from_translation(Vec3::ZERO).with_rotation(initial_ship_rot),
            Visibility::default(),
        ))
        .id();

    // 3D Spaceship Avatar (Scaled down to 0.14, nose pointing forward -Z into space)
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
            Camera3d::default(),
            Camera {
                order: 0,
                ..default()
            },
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
                far: 150_000.0,
                ..default()
            }),
            Transform::from_xyz(0.0, 1.2, 4.0)
                .looking_at(Vec3::new(0.0, 0.1, -5.0), Vec3::Y),
            DistanceFog {
                color: Color::srgba(0.0005, 0.001, 0.003, 1.0),
                falloff: FogFalloff::Exponential { density: 0.0000005 },
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


    // Ambient Fill Light for deep space
    commands.spawn(AmbientLight {
        color: Color::srgba(0.03, 0.04, 0.08, 1.0),
        brightness: 120.0,
        affects_lightmapped_meshes: false,
    });

    // Dynamic Sunlight (Cascaded Directional Shadow Light)
    let cascade_config = CascadeShadowConfigBuilder {
        num_cascades: 4,
        maximum_distance: 30_000.0,
        minimum_distance: 0.1,
        first_cascade_far_bound: 15.0,
        overlap_proportion: 0.2,
    }.build();

    commands.spawn((
        SunDirectionalLight,
        DirectionalLight {
            color: Color::srgb(1.0, 0.97, 0.92),
            illuminance: 100_000.0,
            shadow_maps_enabled: true,
            contact_shadows_enabled: true,
            ..default()
        },
        cascade_config,
        Transform::IDENTITY,
    ));


    // ----------------------------------------------------
    // 2. THE SUN (NASA HIGH-RES RADIANT SURFACE & COLOSSAL SOLAR CORONA)
    // ----------------------------------------------------
    let sun_tex: Handle<Image> = asset_server.load("textures/sun.jpg");
    let sun_mesh = meshes.add(create_uv_sphere(32790.0, 192, 96));
    let sun_mat = materials.add(StandardMaterial {
        base_color_texture: Some(sun_tex.clone()),
        emissive_texture: Some(sun_tex),
        emissive: LinearRgba::new(35.0, 25.0, 6.0, 1.0),
        unlit: true,
        ..default()
    });

    let sun_entity = commands.spawn((
        Sun { radius: 32790.0 },
        Mesh3d(sun_mesh),
        MeshMaterial3d(sun_mat),
        Transform::from_xyz(0.0, 0.0, 0.0),
    )).id();

    // Solar Corona Glow Atmosphere
    let corona_mesh = meshes.add(create_uv_sphere(43720.0, 32, 16));
    let corona_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.65, 0.15, 0.35),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let corona_entity = commands.spawn((
        Mesh3d(corona_mesh),
        MeshMaterial3d(corona_mat),
        Transform::IDENTITY,
    )).id();
    commands.entity(sun_entity).add_child(corona_entity);

    // Sun Core Sunlight (PointLight)
    commands.spawn((
        PointLight {
            intensity: 50_000_000.0,
            color: Color::srgb(1.0, 0.96, 0.88),
            range: 50_000_000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // ----------------------------------------------------
    // 3. PLANETS, DWARF PLANETS & MOONS (TITANIC MASSIVE SCALES & REALISTIC SLOW ROTATION)
    // ----------------------------------------------------

    // --- PLANET 1: MERCURY (0.3871 AU) ---
    let mercury_orbit_radius = 0.387098 * 240_000.0; // 92,903.52 km
    let mercury_orbit_speed = 0.15;
    let mercury_angle = next_orbit_angle();
    let mercury_pos = Vec3::new(mercury_orbit_radius * mercury_angle.cos(), 0.0, mercury_orbit_radius * mercury_angle.sin());
    let mercury_tex: Handle<Image> = asset_server.load("textures/mercury.jpg");
    let mercury_mesh = meshes.add(create_uv_sphere(114.9, 192, 96));
    let mercury_mat = materials.add(StandardMaterial {
        base_color_texture: Some(mercury_tex.clone()),
        perceptual_roughness: 0.9,
        ..default()
    });
    let mercury_entity = commands.spawn((
        Planet {
            name: "Mercury",
            index: 1,
            radius: 114.9,
            orbit_radius: mercury_orbit_radius,
            orbit_speed: mercury_orbit_speed,
            orbit_angle: mercury_angle,
            rotation_speed: 0.0012,
            world_pos: mercury_pos,
        },
        PlanetLod::new(1, false, "", 6.0, mercury_tex.clone(), 192, 96),
        Mesh3d(mercury_mesh),
        MeshMaterial3d(mercury_mat),
        Transform::from_translation(mercury_pos),
    )).id();
    spawn_planet_area_light(&mut commands, mercury_entity, mercury_pos, 114.9);

    // --- PLANET 2: VENUS (0.7233 AU) ---
    let venus_orbit_radius = 0.723332 * 240_000.0; // 173,599.68 km
    let venus_orbit_speed = 0.11;
    let venus_angle = next_orbit_angle();
    let venus_pos = Vec3::new(venus_orbit_radius * venus_angle.cos(), 0.0, venus_orbit_radius * venus_angle.sin());
    let venus_tex: Handle<Image> = asset_server.load("textures/venus.jpg");
    let venus_mesh = meshes.add(create_uv_sphere(285.0, 256, 128));
    let venus_mat = materials.add(StandardMaterial {
        base_color_texture: Some(venus_tex.clone()),
        perceptual_roughness: 0.4,
        ..default()
    });
    let venus_entity = commands.spawn((
        Planet {
            name: "Venus",
            index: 2,
            radius: 285.0,
            orbit_radius: venus_orbit_radius,
            orbit_speed: venus_orbit_speed,
            orbit_angle: venus_angle,
            rotation_speed: -0.0006,
            world_pos: venus_pos,
        },
        PlanetLod::new(2, false, "", 8.0, venus_tex.clone(), 256, 128),
        Mesh3d(venus_mesh),
        MeshMaterial3d(venus_mat),
        Transform::from_translation(venus_pos),
    )).id();
    spawn_planet_area_light(&mut commands, venus_entity, venus_pos, 285.0);

    // Venus Atmospheric Scattering Shell
    let venus_atmo_mesh = meshes.add(create_uv_sphere(292.0, 96, 48));
    let venus_atmo_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.9, 0.75, 0.4, 0.28),
        emissive: LinearRgba::new(2.5, 1.8, 0.5, 1.0),
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    let venus_atmo = commands.spawn((
        Mesh3d(venus_atmo_mesh),
        MeshMaterial3d(venus_atmo_mat),
        Transform::IDENTITY,
    )).id();
    commands.entity(venus_entity).add_child(venus_atmo);

    // --- PLANET 3: EARTH & MOON (1.0000 AU) ---
    let earth_orbit_radius = temp_earth_radius;
    let earth_orbit_speed = 0.08;
    let earth_angle = temp_earth_angle;
    let earth_pos = temp_earth_pos;
    let earth_tex: Handle<Image> = asset_server.load("textures/earth.jpg");
    let earth_mesh = meshes.add(create_uv_sphere(300.0, 256, 128));
    let earth_mat = materials.add(StandardMaterial {
        base_color_texture: Some(earth_tex.clone()),
        emissive_texture: Some(earth_tex.clone()),
        emissive: LinearRgba::new(0.22, 0.22, 0.26, 1.0),
        perceptual_roughness: 0.35,
        metallic: 0.05,
        reflectance: 0.35,
        ..default()
    });

    let earth_entity = commands.spawn((
        Planet {
            name: "Earth",
            index: 3,
            radius: 300.0,
            orbit_radius: earth_orbit_radius,
            orbit_speed: earth_orbit_speed,
            orbit_angle: earth_angle,
            rotation_speed: 0.003,
            world_pos: earth_pos,
        },
        PlanetLod::new(3, false, "", 10.0, earth_tex.clone(), 256, 128),
        Mesh3d(earth_mesh),
        MeshMaterial3d(earth_mat),
        Transform::from_translation(earth_pos),
    )).id();
    spawn_planet_area_light(&mut commands, earth_entity, earth_pos, 300.0);

    // Earth Atmospheric Rayleigh Scattering Glow Shell
    let earth_atmo_mesh = meshes.add(create_uv_sphere(307.5, 128, 64));
    let earth_atmo_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.55, 1.0, 0.35),
        emissive: LinearRgba::new(0.8, 2.5, 6.0, 1.0),
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    let earth_atmo = commands.spawn((
        Mesh3d(earth_atmo_mesh),
        MeshMaterial3d(earth_atmo_mat),
        Transform::IDENTITY,
    )).id();
    commands.entity(earth_entity).add_child(earth_atmo);


    // Position ship near Earth's starting position facing Sun
    flight_state.world_pos = initial_spawn_pos;
    flight_state.previous_pos = flight_state.world_pos;

    // Earth's Moon (NASA 2K Texture)
    let moon_orbit_radius = 1800.0;
    let moon_orbit_speed = 0.25;
    let moon_angle = next_orbit_angle();
    let moon_pos = earth_pos + Vec3::new(moon_orbit_radius * moon_angle.cos(), 0.0, moon_orbit_radius * moon_angle.sin());
    let moon_tex: Handle<Image> = asset_server.load("textures/moon.jpg");
    let moon_mesh = meshes.add(create_uv_sphere(81.8, 192, 96));
    let moon_mat = materials.add(StandardMaterial {
        base_color_texture: Some(moon_tex.clone()),
        perceptual_roughness: 0.85,
        ..default()
    });
    let moon_entity = commands.spawn((
        Moon {
            name: "Moon",
            parent_index: 3,
            radius: 81.8,
            orbit_radius: moon_orbit_radius,
            orbit_speed: moon_orbit_speed,
            orbit_angle: moon_angle,
            rotation_speed: 0.0012,
            world_pos: moon_pos,
        },
        PlanetLod::new(3, true, "Moon", 5.0, moon_tex.clone(), 192, 96),
        Mesh3d(moon_mesh),
        MeshMaterial3d(moon_mat),
        Transform::from_translation(moon_pos),
    )).id();
    spawn_planet_area_light(&mut commands, moon_entity, moon_pos, 81.8);
    spawn_planet_area_light(&mut commands, moon_entity, moon_pos, 81.8);

    // --- PLANET 4: MARS (1.5237 AU) ---
    let mars_orbit_radius = 1.523680 * 240_000.0; // 365,683.20 km
    let mars_orbit_speed = 0.06;
    let mars_angle = next_orbit_angle();
    let mars_pos = Vec3::new(mars_orbit_radius * mars_angle.cos(), 0.0, mars_orbit_radius * mars_angle.sin());
    let mars_tex: Handle<Image> = asset_server.load("textures/mars.jpg");
    let mars_mesh = meshes.add(create_uv_sphere(159.6, 256, 128));
    let mars_mat = materials.add(StandardMaterial {
        base_color_texture: Some(mars_tex.clone()),
        perceptual_roughness: 0.75,
        ..default()
    });
    let mars_entity = commands.spawn((
        Planet {
            name: "Mars",
            index: 4,
            radius: 159.6,
            orbit_radius: mars_orbit_radius,
            orbit_speed: mars_orbit_speed,
            orbit_angle: mars_angle,
            rotation_speed: 0.003,
            world_pos: mars_pos,
        },
        PlanetLod::new(4, false, "", 9.0, mars_tex.clone(), 256, 128),
        Mesh3d(mars_mesh),
        MeshMaterial3d(mars_mat),
        Transform::from_translation(mars_pos),
    )).id();
    spawn_planet_area_light(&mut commands, mars_entity, mars_pos, 159.6);

    // --- DWARF PLANET 1: CERES (2.767 AU - MAIN ASTEROID BELT) ---
    let ceres_orbit_radius = 2.767000 * 240_000.0; // 664,080.00 km
    let ceres_orbit_speed = 0.07;
    let ceres_angle = next_orbit_angle();
    let ceres_pos = Vec3::new(ceres_orbit_radius * ceres_angle.cos(), 0.0, ceres_orbit_radius * ceres_angle.sin());
    let ceres_tex: Handle<Image> = asset_server.load("textures/ceres.jpg");
    let ceres_mesh = meshes.add(create_uv_sphere(22.3, 192, 96));
    let ceres_mat = materials.add(StandardMaterial {
        base_color_texture: Some(ceres_tex.clone()),
        perceptual_roughness: 0.85,
        ..default()
    });
    let ceres_entity = commands.spawn((
        Planet {
            name: "Ceres",
            index: 10,
            radius: 22.3,
            orbit_radius: ceres_orbit_radius,
            orbit_speed: ceres_orbit_speed,
            orbit_angle: ceres_angle,
            rotation_speed: 0.0035,
            world_pos: ceres_pos,
        },
        PlanetLod::new(10, false, "", 3.0, ceres_tex.clone(), 192, 96),
        Mesh3d(ceres_mesh),
        MeshMaterial3d(ceres_mat),
        Transform::from_translation(ceres_pos),
    )).id();
    spawn_planet_area_light(&mut commands, ceres_entity, ceres_pos, 22.3);

    // --- MAIN ASTEROID BELT (2.1 AU to 3.3 AU: ~500,000 - ~800,000 KM) ---
    let asteroid_tex: Handle<Image> = asset_server.load("textures/asteroid.jpg");
    let asteroid_base_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asteroid_tex),
        perceptual_roughness: 0.9,
        metallic: 0.1,
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
        let dist = 500_000.0 + (belt_rng as f32 / u64::MAX as f32) * 300_000.0;

        belt_rng = belt_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let y_offset = ((belt_rng as f32 / u64::MAX as f32) * 2.0 - 1.0) * 8_000.0;

        let ast_x = angle.cos() * dist;
        let ast_z = angle.sin() * dist;
        let ast_pos = Vec3::new(ast_x, y_offset, ast_z);

        belt_rng = belt_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let ast_radius = 8.0 + (belt_rng as f32 / u64::MAX as f32) * 32.0;

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
    let jupiter_orbit_radius = 5.204400 * 240_000.0; // 1,249,056.00 km
    let jupiter_orbit_speed = 0.035;
    let jupiter_angle = next_orbit_angle();
    let jupiter_pos = Vec3::new(jupiter_orbit_radius * jupiter_angle.cos(), 0.0, jupiter_orbit_radius * jupiter_angle.sin());
    let jupiter_tex: Handle<Image> = asset_server.load("textures/jupiter.jpg");
    let jupiter_mesh = meshes.add(create_uv_sphere(3292.0, 384, 192));
    let jupiter_mat = materials.add(StandardMaterial {
        base_color_texture: Some(jupiter_tex.clone()),
        perceptual_roughness: 0.5,
        ..default()
    });
    let jupiter_entity = commands.spawn((
        Planet {
            name: "Jupiter",
            index: 5,
            radius: 3292.0,
            orbit_radius: jupiter_orbit_radius,
            orbit_speed: jupiter_orbit_speed,
            orbit_angle: jupiter_angle,
            rotation_speed: 0.006,
            world_pos: jupiter_pos,
        },
        PlanetLod::new(5, false, "", 16.0, jupiter_tex.clone(), 384, 192),
        Mesh3d(jupiter_mesh),
        MeshMaterial3d(jupiter_mat),
        Transform::from_translation(jupiter_pos),
    )).id();
    spawn_planet_area_light(&mut commands, jupiter_entity, jupiter_pos, 3292.0);

    // Io
    let io_orbit_radius = 19800.0;
    let io_orbit_speed = 0.45;
    let io_angle = next_orbit_angle();
    let io_pos = jupiter_pos + Vec3::new(io_orbit_radius * io_angle.cos(), 0.0, io_orbit_radius * io_angle.sin());
    let io_mesh = meshes.add(create_uv_sphere(85.8, 192, 96));
    let io_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.85, 0.2),
        perceptual_roughness: 0.7,
        ..default()
    });
    let io_entity = commands.spawn((
        Moon {
            name: "Io",
            parent_index: 5,
            radius: 85.8,
            orbit_radius: io_orbit_radius,
            orbit_speed: io_orbit_speed,
            orbit_angle: io_angle,
            rotation_speed: 0.0025,
            world_pos: io_pos,
        },
        PlanetLod::new(5, true, "Io", 4.5, jupiter_tex.clone(), 192, 96),
        Mesh3d(io_mesh),
        MeshMaterial3d(io_mat),
        Transform::from_translation(io_pos),
    )).id();
    spawn_planet_area_light(&mut commands, io_entity, io_pos, 85.8);

    // Europa
    let europa_orbit_radius = 31600.0;
    let europa_orbit_speed = 0.32;
    let europa_angle = next_orbit_angle();
    let europa_pos = jupiter_pos + Vec3::new(europa_orbit_radius * europa_angle.cos(), 0.0, europa_orbit_radius * europa_angle.sin());
    let europa_mesh = meshes.add(create_uv_sphere(73.5, 192, 96));
    let europa_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.88, 0.95),
        perceptual_roughness: 0.2,
        metallic: 0.1,
        ..default()
    });
    let europa_entity = commands.spawn((
        Moon {
            name: "Europa",
            parent_index: 5,
            radius: 73.5,
            orbit_radius: europa_orbit_radius,
            orbit_speed: europa_orbit_speed,
            orbit_angle: europa_angle,
            rotation_speed: 0.0022,
            world_pos: europa_pos,
        },
        PlanetLod::new(5, true, "Europa", 3.5, jupiter_tex.clone(), 192, 96),
        Mesh3d(europa_mesh),
        MeshMaterial3d(europa_mat),
        Transform::from_translation(europa_pos),
    )).id();
    spawn_planet_area_light(&mut commands, europa_entity, europa_pos, 73.5);

    // --- PLANET 6: SATURN & REALISTIC 2D RING SYSTEM WITH DUST & ROCKS (9.5826 AU) ---
    let saturn_orbit_radius = 9.582600 * 240_000.0; // 2,299,824.00 km
    let saturn_orbit_speed = 0.02;
    let saturn_angle = next_orbit_angle();
    let saturn_pos = Vec3::new(saturn_orbit_radius * saturn_angle.cos(), 0.0, saturn_orbit_radius * saturn_angle.sin());
    let saturn_tex: Handle<Image> = asset_server.load("textures/saturn.jpg");
    let saturn_mesh = meshes.add(create_uv_sphere(2742.0, 384, 192));
    let saturn_mat = materials.add(StandardMaterial {
        base_color_texture: Some(saturn_tex.clone()),
        perceptual_roughness: 0.45,
        ..default()
    });
    let saturn_entity = commands
        .spawn((
            Planet {
                name: "Saturn",
                index: 6,
                radius: 2742.0,
                orbit_radius: saturn_orbit_radius,
                orbit_speed: saturn_orbit_speed,
                orbit_angle: saturn_angle,
                rotation_speed: 0.0055,
                world_pos: saturn_pos,
            },
            PlanetLod::new(6, false, "", 14.0, saturn_tex.clone(), 384, 192),
            Mesh3d(saturn_mesh),
            MeshMaterial3d(saturn_mat),
            Transform::from_translation(saturn_pos),
        ))
        .id();
    spawn_planet_area_light(&mut commands, saturn_entity, saturn_pos, 2742.0);

    // Saturn Ring System (Transparent 2D Ring Plane Disk with Radial Texture)
    let saturn_ring_tex: Handle<Image> = asset_server.load("textures/saturn_ring.png");
    let ring_plane_mesh = meshes.add(create_flat_ring_mesh(3290.4, 6306.6, 256, 16));
    let ring_mat = materials.add(StandardMaterial {
        base_color_texture: Some(saturn_ring_tex.clone()),
        emissive_texture: Some(saturn_ring_tex.clone()),
        emissive: LinearRgba::new(0.3, 0.26, 0.2, 1.0),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        double_sided: true,
        perceptual_roughness: 0.35,
        metallic: 0.1,
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
        ..default()
    });

    let mut ring_rng: u64 = 999111222;
    for _ in 0..180 {
        ring_rng = ring_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let angle = (ring_rng as f32 / u64::MAX as f32) * std::f32::consts::TAU;

        ring_rng = ring_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let rad = 3350.0 + (ring_rng as f32 / u64::MAX as f32) * 2800.0;

        ring_rng = ring_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let y_off = ((ring_rng as f32 / u64::MAX as f32) * 2.0 - 1.0) * 12.0;

        ring_rng = ring_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let rock_scale = 3.0 + (ring_rng as f32 / u64::MAX as f32) * 12.0;

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
    let uranus_orbit_radius = 19.201000 * 240_000.0; // 4,608,240.00 km
    let uranus_orbit_speed = 0.012;
    let uranus_angle = next_orbit_angle();
    let uranus_pos = Vec3::new(uranus_orbit_radius * uranus_angle.cos(), 0.0, uranus_orbit_radius * uranus_angle.sin());
    let uranus_tex: Handle<Image> = asset_server.load("textures/uranus.jpg");
    let uranus_mesh = meshes.add(create_uv_sphere(1194.3, 256, 128));
    let uranus_mat = materials.add(StandardMaterial {
        base_color_texture: Some(uranus_tex.clone()),
        perceptual_roughness: 0.3,
        ..default()
    });
    let uranus_entity = commands.spawn((
        Planet {
            name: "Uranus",
            index: 7,
            radius: 1194.3,
            orbit_radius: uranus_orbit_radius,
            orbit_speed: uranus_orbit_speed,
            orbit_angle: uranus_angle,
            rotation_speed: -0.0045,
            world_pos: uranus_pos,
        },
        PlanetLod::new(7, false, "", 8.0, uranus_tex.clone(), 256, 128),
        Mesh3d(uranus_mesh),
        MeshMaterial3d(uranus_mat),
        Transform::from_translation(uranus_pos),
    )).id();
    spawn_planet_area_light(&mut commands, uranus_entity, uranus_pos, 1194.3);

    // --- PLANET 8: NEPTUNE (30.047 AU) ---
    let neptune_orbit_radius = 30.047000 * 240_000.0; // 7,211,280.00 km
    let neptune_orbit_speed = 0.007;
    let neptune_angle = next_orbit_angle();
    let neptune_pos = Vec3::new(neptune_orbit_radius * neptune_angle.cos(), 0.0, neptune_orbit_radius * neptune_angle.sin());
    let neptune_tex: Handle<Image> = asset_server.load("textures/neptune.jpg");
    let neptune_mesh = meshes.add(create_uv_sphere(1159.4, 256, 128));
    let neptune_mat = materials.add(StandardMaterial {
        base_color_texture: Some(neptune_tex.clone()),
        perceptual_roughness: 0.25,
        ..default()
    });
    let neptune_entity = commands.spawn((
        Planet {
            name: "Neptune",
            index: 8,
            radius: 1159.4,
            orbit_radius: neptune_orbit_radius,
            orbit_speed: neptune_orbit_speed,
            orbit_angle: neptune_angle,
            rotation_speed: 0.005,
            world_pos: neptune_pos,
        },
        PlanetLod::new(8, false, "", 8.0, neptune_tex.clone(), 256, 128),
        Mesh3d(neptune_mesh),
        MeshMaterial3d(neptune_mat),
        Transform::from_translation(neptune_pos),
    )).id();
    spawn_planet_area_light(&mut commands, neptune_entity, neptune_pos, 1159.4);

    // --- DWARF PLANET 2: PLUTO & MOON CHARON (39.482 AU) ---
    let pluto_orbit_radius = 39.482000 * 240_000.0; // 9,475,680.00 km
    let pluto_orbit_speed = 0.005;
    let pluto_angle = next_orbit_angle();
    let pluto_pos = Vec3::new(pluto_orbit_radius * pluto_angle.cos(), 0.0, pluto_orbit_radius * pluto_angle.sin());
    let pluto_tex: Handle<Image> = asset_server.load("textures/pluto.jpg");
    let pluto_mesh = meshes.add(create_uv_sphere(56.0, 192, 96));
    let pluto_mat = materials.add(StandardMaterial {
        base_color_texture: Some(pluto_tex.clone()),
        perceptual_roughness: 0.7,
        ..default()
    });
    let pluto_entity = commands.spawn((
        Planet {
            name: "Pluto",
            index: 9,
            radius: 56.0,
            orbit_radius: pluto_orbit_radius,
            orbit_speed: pluto_orbit_speed,
            orbit_angle: pluto_angle,
            rotation_speed: 0.0018,
            world_pos: pluto_pos,
        },
        PlanetLod::new(9, false, "", 4.0, pluto_tex.clone(), 192, 96),
        Mesh3d(pluto_mesh),
        MeshMaterial3d(pluto_mat),
        Transform::from_translation(pluto_pos),
    )).id();
    spawn_planet_area_light(&mut commands, pluto_entity, pluto_pos, 56.0);

    // Charon
    let charon_orbit_radius = 924.0;
    let charon_orbit_speed = 0.2;
    let charon_angle = next_orbit_angle();
    let charon_pos = pluto_pos + Vec3::new(charon_orbit_radius * charon_angle.cos(), 0.0, charon_orbit_radius * charon_angle.sin());
    let charon_mesh = meshes.add(create_uv_sphere(28.5, 192, 96));
    let charon_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.48, 0.45),
        perceptual_roughness: 0.85,
        ..default()
    });
    let charon_entity = commands.spawn((
        Moon {
            name: "Charon",
            parent_index: 9,
            radius: 28.5,
            orbit_radius: charon_orbit_radius,
            orbit_speed: charon_orbit_speed,
            orbit_angle: charon_angle,
            rotation_speed: 0.0012,
            world_pos: charon_pos,
        },
        PlanetLod::new(9, true, "Charon", 2.5, pluto_tex.clone(), 192, 96),
        Mesh3d(charon_mesh),
        MeshMaterial3d(charon_mat),
        Transform::from_translation(charon_pos),
    )).id();
    spawn_planet_area_light(&mut commands, charon_entity, charon_pos, 28.5);

    // --- DWARF PLANET 3: HAUMEA (43.218 AU) ---
    let haumea_orbit_radius = 43.218000 * 240_000.0; // 10,372,320.00 km
    let haumea_orbit_speed = 0.004;
    let haumea_angle = next_orbit_angle();
    let haumea_pos = Vec3::new(haumea_orbit_radius * haumea_angle.cos(), 0.0, haumea_orbit_radius * haumea_angle.sin());
    let haumea_tex: Handle<Image> = asset_server.load("textures/haumea.jpg");
    let haumea_mesh = meshes.add(create_uv_sphere(37.4, 192, 96));
    let haumea_mat = materials.add(StandardMaterial {
        base_color_texture: Some(haumea_tex.clone()),
        perceptual_roughness: 0.5,
        metallic: 0.1,
        ..default()
    });
    let haumea_entity = commands.spawn((
        Planet {
            name: "Haumea",
            index: 11,
            radius: 37.4,
            orbit_radius: haumea_orbit_radius,
            orbit_speed: haumea_orbit_speed,
            orbit_angle: haumea_angle,
            rotation_speed: 0.0095,
            world_pos: haumea_pos,
        },
        PlanetLod::new(11, false, "", 3.5, haumea_tex.clone(), 192, 96),
        Mesh3d(haumea_mesh),
        MeshMaterial3d(haumea_mat),
        Transform::from_translation(haumea_pos).with_scale(Vec3::new(1.35, 0.85, 1.0)),
    )).id();
    spawn_planet_area_light(&mut commands, haumea_entity, haumea_pos, 37.4);

    // --- DWARF PLANET 4: MAKEMAKE (45.563 AU) ---
    let makemake_orbit_radius = 45.563000 * 240_000.0; // 10,935,120.00 km
    let makemake_orbit_speed = 0.0035;
    let makemake_angle = next_orbit_angle();
    let makemake_pos = Vec3::new(makemake_orbit_radius * makemake_angle.cos(), 0.0, makemake_orbit_radius * makemake_angle.sin());
    let makemake_tex: Handle<Image> = asset_server.load("textures/makemake.jpg");
    let makemake_mesh = meshes.add(create_uv_sphere(33.7, 192, 96));
    let makemake_mat = materials.add(StandardMaterial {
        base_color_texture: Some(makemake_tex.clone()),
        perceptual_roughness: 0.65,
        ..default()
    });
    let makemake_entity = commands.spawn((
        Planet {
            name: "Makemake",
            index: 12,
            radius: 33.7,
            orbit_radius: makemake_orbit_radius,
            orbit_speed: makemake_orbit_speed,
            orbit_angle: makemake_angle,
            rotation_speed: 0.0028,
            world_pos: makemake_pos,
        },
        PlanetLod::new(12, false, "", 3.0, makemake_tex.clone(), 192, 96),
        Mesh3d(makemake_mesh),
        MeshMaterial3d(makemake_mat),
        Transform::from_translation(makemake_pos),
    )).id();
    spawn_planet_area_light(&mut commands, makemake_entity, makemake_pos, 33.7);

    // --- DWARF PLANET 5: ERIS (67.781 AU) ---
    let eris_orbit_radius = 67.781000 * 240_000.0; // 16,267,440.00 km
    let eris_orbit_speed = 0.0025;
    let eris_angle = next_orbit_angle();
    let eris_pos = Vec3::new(eris_orbit_radius * eris_angle.cos(), 0.0, eris_orbit_radius * eris_angle.sin());
    let eris_tex: Handle<Image> = asset_server.load("textures/eris.jpg");
    let eris_mesh = meshes.add(create_uv_sphere(54.8, 192, 96));
    let eris_mat = materials.add(StandardMaterial {
        base_color_texture: Some(eris_tex.clone()),
        perceptual_roughness: 0.3,
        metallic: 0.1,
        ..default()
    });
    let eris_entity = commands.spawn((
        Planet {
            name: "Eris",
            index: 13,
            radius: 54.8,
            orbit_radius: eris_orbit_radius,
            orbit_speed: eris_orbit_speed,
            orbit_angle: eris_angle,
            rotation_speed: 0.0022,
            world_pos: eris_pos,
        },
        PlanetLod::new(13, false, "", 3.0, eris_tex.clone(), 192, 96),
        Mesh3d(eris_mesh),
        MeshMaterial3d(eris_mat),
        Transform::from_translation(eris_pos),
    )).id();
    spawn_planet_area_light(&mut commands, eris_entity, eris_pos, 54.8);

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
        let dist = 100_000.0 + (dust_rng as f32 / u64::MAX as f32) * 16_000_000.0;

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
    let skybox_mesh = meshes.add(create_uv_sphere(95_000.0, 128, 64));

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

    let rotation = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);

    if let Some(VertexAttributeValues::Float32x3(positions)) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
        for p in positions.iter_mut() {
            let rotated = rotation * Vec3::from_slice(p);
            *p = rotated.to_array();
        }
    }

    if let Some(VertexAttributeValues::Float32x3(normals)) = mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL) {
        for n in normals.iter_mut() {
            let rotated = rotation * Vec3::from_slice(n);
            *n = rotated.to_array();
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
                intensity: 15_000_000.0,
                color: Color::srgb(1.0, 0.96, 0.88),
                range: radius * 12.0,
                radius: radius * 1.8,
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

    commands.spawn((
        Camera2d::default(),
        Camera {
            order: 100,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.001, 0.001, 0.003)),
            ..default()
        },
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

#[allow(dead_code)]
fn create_dotted_circle_mesh(num_dots: usize, circle_radius: f32, dot_radius: f32) -> Mesh {
    use bevy::asset::RenderAssetUsages;
    use bevy::mesh::PrimitiveTopology;
    use std::f32::consts::PI;

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    let rings = 4;
    let segments = 6;

    for i in 0..num_dots {
        let angle = (i as f32 / num_dots as f32) * 2.0 * PI;
        let center = Vec3::new(circle_radius * angle.cos(), circle_radius * angle.sin(), 0.0);

        let base_index = positions.len() as u32;

        for r in 0..=rings {
            let v = (r as f32 / rings as f32) * PI;
            for s in 0..=segments {
                let u = (s as f32 / segments as f32) * 2.0 * PI;

                let nx = v.sin() * u.cos();
                let ny = v.sin() * u.sin();
                let nz = v.cos();
                let normal = Vec3::new(nx, ny, nz);

                let pos = center + normal * dot_radius;

                positions.push([pos.x, pos.y, pos.z]);
                normals.push([normal.x, normal.y, normal.z]);
                uvs.push([u / (2.0 * PI), v / PI]);
            }
        }

        let stride = segments + 1;
        for r in 0..rings {
            for s in 0..segments {
                let first = base_index + (r * stride + s) as u32;
                let second = first + stride as u32;

                indices.push(first);
                indices.push(second);
                indices.push(first + 1);

                indices.push(second);
                indices.push(second + 1);
                indices.push(first + 1);
            }
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::mesh::VertexAttributeValues;

    #[test]
    fn test_sphere_uvs() {
        let mesh = create_uv_sphere(1.0, 4, 4);

        let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
            VertexAttributeValues::Float32x3(p) => p,
            _ => panic!(),
        };
        let uvs = match mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap() {
            VertexAttributeValues::Float32x2(u) => u,
            _ => panic!(),
        };
        for (p, uv) in positions.iter().zip(uvs.iter()) {
            println!("uv: {:?}, pos: [{:.2}, {:.2}, {:.2}]", uv, p[0], p[1], p[2]);
        }
    }
}

