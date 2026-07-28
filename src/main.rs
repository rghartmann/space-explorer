use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::WindowMode;
use bevy::post_process::bloom::{Bloom, BloomPrefilter};











// --- Components & Resources ---

#[derive(Component)]
struct Ship;

#[derive(Component)]
struct EngineSound;

#[derive(Component)]
struct PilotCamera;

#[derive(Component)]
struct Sun;

#[derive(Component)]
struct Planet {
    _name: &'static str,
    orbit_radius: f32,
    orbit_speed: f32,
    rotation_speed: f32,
}

#[derive(Component)]
struct Starfield;

#[derive(Resource)]
struct FlightState {
    velocity: Vec3,
    yaw: f32,   // Pilot look yaw
    pitch: f32, // Pilot look pitch
}

impl Default for FlightState {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.001, 0.001, 0.003)))
        .init_resource::<FlightState>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Space Explorer - Realistic Solar System Flight".into(),
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
                orbit_planets_system,
                engine_sound_system,
            ),
        )
        .run();
}

fn generate_engine_hum_wav() -> Vec<u8> {
    let sample_rate = 44100u32;
    let duration_secs = 2.0f32;
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let num_channels = 1u16;
    let bits_per_sample = 16u16;

    let data_size = (num_samples * num_channels as usize * 2) as u32;
    let file_size = 36 + data_size;

    let mut bytes = Vec::with_capacity(44 + data_size as usize);

    // RIFF Header
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&file_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");

    // fmt chunk
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes()); // Chunk size 16
    bytes.extend_from_slice(&1u16.to_le_bytes());  // Audio format 1 (PCM)
    bytes.extend_from_slice(&num_channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * num_channels as u32 * bits_per_sample as u32 / 8;
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align = num_channels * bits_per_sample / 8;
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&bits_per_sample.to_le_bytes());

    // data chunk
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());

    // Generate phase-locked harmonic sine waves for a smooth continuous engine hum
    let tau = std::f32::consts::TAU;

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;

        // 1. Deep sub-bass drone (55.0 Hz - 110 full cycles in 2s)
        let sub = (tau * 55.0 * t).sin() * 0.38;

        // 2. Throbbing reactor pulse beat (56.0 Hz - 112 full cycles in 2s -> 1 Hz pulse)
        let sub_beat = (tau * 56.0 * t).sin() * 0.22;

        // 3. Main engine body tone (110.0 Hz - 220 full cycles in 2s)
        let hum1 = (tau * 110.0 * t).sin() * 0.24;

        // 4. Core resonance (165.0 Hz - 330 full cycles in 2s)
        let hum2 = (tau * 165.0 * t).sin() * 0.14;

        // 5. Upper core tone (220.0 Hz - 440 full cycles in 2s)
        let hum3 = (tau * 220.0 * t).sin() * 0.08;

        // 6. High turbine overtones (330.0 Hz & 440.0 Hz)
        let turbine1 = (tau * 330.0 * t).sin() * 0.04;
        let turbine2 = (tau * 440.0 * t).sin() * 0.02;

        // 7. Extra plasma shimmer (523.5 Hz - 1047 cycles in 2s)
        let plasma = (tau * 523.5 * t).sin() * 0.015;

        let sample_f32 = sub + sub_beat + hum1 + hum2 + hum3 + turbine1 + turbine2 + plasma;
        let clamped = sample_f32.clamp(-1.0, 1.0);
        let sample_i16 = (clamped * 28000.0) as i16;

        bytes.extend_from_slice(&sample_i16.to_le_bytes());
    }

    bytes
}

fn ensure_engine_hum_file() {
    let path = std::path::Path::new("assets/audio/engine_hum.wav");
    if !path.exists() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let wav_data = generate_engine_hum_wav();
        let _ = std::fs::write(path, wav_data);
    }
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // ----------------------------------------------------
    // ENGINE SOUND (Procedural Starship Engine Humming)
    // ----------------------------------------------------
    ensure_engine_hum_file();


    let audio_handle: Handle<AudioSource> = asset_server.load("audio/engine_hum.wav");

    commands.spawn((
        EngineSound,
        AudioPlayer(audio_handle),
        PlaybackSettings::LOOP,
    ));

    // ----------------------------------------------------
    // 1. SHIP & FIRST-PERSON PILOT COCKPIT PERSPECTIVE
    // ----------------------------------------------------
    // Ship starts 2,500 units away from the Sun for an expansive deep space approach
    let ship_entity = commands
        .spawn((
            Ship,
            Transform::from_xyz(0.0, 0.0, 2500.0),
            Visibility::default(),
        ))
        .id();

    // Spawn First-Person Camera with extended far frustum (50,000 units)
    let camera_entity = commands
        .spawn((
            PilotCamera,
            Camera3d::default(),
            Bloom {
                intensity: 0.35,
                prefilter: BloomPrefilter {
                    threshold: 0.1,
                    threshold_softness: 0.2,
                },
                ..Bloom::NATURAL
            },




            Projection::Perspective(PerspectiveProjection {
                far: 50_000.0,
                ..default()
            }),
            Transform::from_xyz(0.0, 0.5, 0.0), // Pilot eye level inside cockpit
        ))
        .id();



    commands.entity(ship_entity).add_child(camera_entity);

    // ----------------------------------------------------
    // COCKPIT INTERIOR FRAME & CANOPY GEOMETRY
    // ----------------------------------------------------
    // Lower Control Console Dashboard
    let console_mesh = meshes.add(Cuboid::from_size(Vec3::new(1.8, 0.4, 0.8)));
    let console_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.07, 0.1),
        metallic: 0.95,
        perceptual_roughness: 0.15,
        ..default()
    });

    let console_entity = commands
        .spawn((
            Mesh3d(console_mesh),
            MeshMaterial3d(console_mat),
            Transform::from_xyz(0.0, -0.45, -0.6)
                .with_rotation(Quat::from_rotation_x(0.2)),
        ))
        .id();
    commands.entity(camera_entity).add_child(console_entity);

    // Emissive Console Display Screens
    let screen_mesh = meshes.add(Cuboid::from_size(Vec3::new(0.35, 0.2, 0.02)));
    let screen_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.85, 1.0),
        emissive: LinearRgba::new(0.0, 4.0, 5.0, 1.0),
        unlit: true,
        ..default()
    });

    let screen_left = commands
        .spawn((
            Mesh3d(screen_mesh.clone()),
            MeshMaterial3d(screen_mat.clone()),
            Transform::from_xyz(-0.45, -0.3, -0.62)
                .with_rotation(Quat::from_rotation_y(0.3)),
        ))
        .id();

    let screen_right = commands
        .spawn((
            Mesh3d(screen_mesh),
            MeshMaterial3d(screen_mat),
            Transform::from_xyz(0.45, -0.3, -0.62)
                .with_rotation(Quat::from_rotation_y(-0.3)),
        ))
        .id();

    commands.entity(camera_entity).add_child(screen_left);
    commands.entity(camera_entity).add_child(screen_right);

    // Left & Right Window Canopy Structural Struts
    let strut_mesh = meshes.add(Cuboid::from_size(Vec3::new(0.05, 1.4, 0.05)));
    let strut_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.12, 0.16),
        metallic: 0.9,
        perceptual_roughness: 0.2,
        ..default()
    });

    let left_strut = commands
        .spawn((
            Mesh3d(strut_mesh.clone()),
            MeshMaterial3d(strut_mat.clone()),
            Transform::from_xyz(-0.75, 0.1, -0.5)
                .with_rotation(Quat::from_rotation_z(-0.35)),
        ))
        .id();

    let right_strut = commands
        .spawn((
            Mesh3d(strut_mesh),
            MeshMaterial3d(strut_mat),
            Transform::from_xyz(0.75, 0.1, -0.5)
                .with_rotation(Quat::from_rotation_z(0.35)),
        ))
        .id();

    commands.entity(camera_entity).add_child(left_strut);
    commands.entity(camera_entity).add_child(right_strut);

    // Sci-Fi HUD Targeting Reticle Overlay on Viewport Glass
    let hud_ring_mesh = meshes.add(Torus { minor_radius: 0.003, major_radius: 0.07 });
    let hud_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 0.95, 1.0, 0.85),
        emissive: LinearRgba::new(0.0, 5.0, 6.0, 1.0),
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
    // 2. THE IMMENSE SUN (NASA SDO High-Res Solar Surface)
    // ----------------------------------------------------
    let sun_mesh = meshes.add(Sphere::new(180.0)); // Immense solar radius
    let sun_texture_handle: Handle<Image> = asset_server.load("textures/sun.png");

    let sun_material = materials.add(StandardMaterial {
        base_color_texture: Some(sun_texture_handle.clone()),
        emissive_texture: Some(sun_texture_handle),
        emissive: LinearRgba::new(12.0, 7.5, 2.5, 1.0),
        unlit: false,
        ..default()
    });

    commands.spawn((
        Sun,
        Mesh3d(sun_mesh),
        MeshMaterial3d(sun_material),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // Outer Solar Glow Atmosphere Shell
    let sun_glow_mesh = meshes.add(Sphere::new(186.0));
    let sun_glow_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.6, 0.1, 0.25),
        emissive: LinearRgba::new(20.0, 10.0, 2.0, 1.0),
        unlit: true,
        ..default()
    });

    commands.spawn((
        Mesh3d(sun_glow_mesh),
        MeshMaterial3d(sun_glow_mat),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // Deep Space Ambient Illumination Component
    commands.spawn(AmbientLight {
        color: Color::srgb(0.004, 0.006, 0.012),
        brightness: 12.0,
        ..default()
    });

    // Sun Point Light casting intense solar light across the vast system
    commands.spawn((
        PointLight {
            intensity: 2_500_000_000.0,
            color: Color::srgb(1.0, 0.95, 0.85),
            range: 20_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // ----------------------------------------------------
    // 3. PLANETS (PROPORTIONAL DISTANCES & NASA TEXTURES)
    // ----------------------------------------------------

    // Mercury (Innermost rocky planet)
    let mercury_mesh = meshes.add(Sphere::new(5.5));
    let mercury_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.42, 0.4),
        perceptual_roughness: 0.9,
        metallic: 0.1,
        ..default()
    });

    commands.spawn((
        Planet {
            _name: "Mercury",
            orbit_radius: 450.0, // Scaled astronomical unit distance
            orbit_speed: 0.12,
            rotation_speed: 0.1,
        },
        Mesh3d(mercury_mesh),
        MeshMaterial3d(mercury_mat),
        Transform::from_xyz(450.0, 0.0, 0.0),
    ));

    // Venus (Dense shrouded atmosphere)
    let venus_mesh = meshes.add(Sphere::new(11.0));
    let venus_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.88, 0.75, 0.52),
        perceptual_roughness: 0.6,
        metallic: 0.0,
        ..default()
    });

    commands.spawn((
        Planet {
            _name: "Venus",
            orbit_radius: 750.0,
            orbit_speed: 0.08,
            rotation_speed: 0.05,
        },
        Mesh3d(venus_mesh),
        MeshMaterial3d(venus_mat),
        Transform::from_xyz(750.0, 0.0, 0.0),
    ));

    // Planet Earth (NASA Blue Marble Texture)
    let earth_mesh = meshes.add(Sphere::new(13.0));
    let earth_texture: Handle<Image> = asset_server.load("textures/earth.png");
    let earth_mat = materials.add(StandardMaterial {
        base_color_texture: Some(earth_texture),
        perceptual_roughness: 0.55,
        metallic: 0.05,
        ..default()
    });

    commands.spawn((
        Planet {
            _name: "Earth",
            orbit_radius: 1100.0, // 1.0 AU relative scale
            orbit_speed: 0.05,
            rotation_speed: 0.3,
        },
        Mesh3d(earth_mesh),
        MeshMaterial3d(earth_mat),
        Transform::from_xyz(1100.0, 0.0, 0.0),
    ));

    // Mars (NASA High-Res Orbital Map)
    let mars_mesh = meshes.add(Sphere::new(8.5));
    let mars_texture: Handle<Image> = asset_server.load("textures/mars.png");
    let mars_mat = materials.add(StandardMaterial {
        base_color_texture: Some(mars_texture),
        perceptual_roughness: 0.8,
        metallic: 0.0,
        ..default()
    });

    commands.spawn((
        Planet {
            _name: "Mars",
            orbit_radius: 1650.0, // 1.52 AU relative scale
            orbit_speed: 0.035,
            rotation_speed: 0.28,
        },
        Mesh3d(mars_mesh),
        MeshMaterial3d(mars_mat),
        Transform::from_xyz(1650.0, 0.0, 0.0),
    ));

    // Jupiter (NASA Juno Texture & Ring System)
    let jupiter_mesh = meshes.add(Sphere::new(45.0));
    let jupiter_texture: Handle<Image> = asset_server.load("textures/jupiter.png");
    let jupiter_mat = materials.add(StandardMaterial {
        base_color_texture: Some(jupiter_texture),
        perceptual_roughness: 0.45,
        metallic: 0.0,
        ..default()
    });

    let jupiter_entity = commands
        .spawn((
            Planet {
                _name: "Jupiter",
                orbit_radius: 3200.0, // 5.2 AU relative scale
                orbit_speed: 0.015,
                rotation_speed: 0.6,
            },
            Mesh3d(jupiter_mesh),
            MeshMaterial3d(jupiter_mat),
            Transform::from_xyz(3200.0, 0.0, 0.0),
        ))
        .id();

    // Jupiter Ring System
    let jupiter_ring_mesh = meshes.add(Torus { minor_radius: 1.8, major_radius: 65.0 });
    let jupiter_ring_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.85, 0.7, 0.5, 0.65),
        perceptual_roughness: 0.7,
        ..default()
    });

    let ring_entity = commands
        .spawn((
            Mesh3d(jupiter_ring_mesh),
            MeshMaterial3d(jupiter_ring_mat),
            Transform::from_rotation(Quat::from_rotation_x(0.35)),
        ))
        .id();
    commands.entity(jupiter_entity).add_child(ring_entity);

    // ----------------------------------------------------
    // 4. DEEP SPACE STARFIELD & DISTANCE-BASED STAR LIGHT
    // ----------------------------------------------------
    let star_colors = [
        Color::srgb(0.75, 0.85, 1.0), // O/B Blue Hypergiant
        Color::srgb(0.95, 0.98, 1.0), // A/F White Main Sequence
        Color::srgb(1.0, 0.95, 0.8),  // G Solar Yellow
        Color::srgb(1.0, 0.7, 0.4),   // K Orange Dwarf
        Color::srgb(1.0, 0.45, 0.35), // M Red Giant
    ];

    let mut rng_seed = 987654321u64;

    for i in 0..1500 {
        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let theta = ((rng_seed % 10000) as f32 / 10000.0) * std::f32::consts::TAU;

        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let phi = (((rng_seed % 10000) as f32 / 10000.0) - 0.5) * std::f32::consts::PI;

        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        // Distribute stars across vast deep space shell distances (3,000 to 18,000 units)
        let distance_layer = (rng_seed % 100) as f32 / 100.0;
        let radius = 3000.0 + distance_layer * 15000.0;

        let x = radius * phi.cos() * theta.cos();
        let y = radius * phi.sin();
        let z = radius * phi.cos() * theta.sin();

        // Distance attenuation: further stars are smaller pinpricks with tailored emissive brightness
        let star_size = (0.6 + (1.0 - distance_layer) * 2.2).clamp(0.4, 3.0);
        let emissive_power = 8.0 + (1.0 - distance_layer) * 24.0;
        let star_color = star_colors[i % star_colors.len()];

        let star_mesh = meshes.add(Sphere::new(star_size));
        let star_mat = materials.add(StandardMaterial {
            base_color: star_color,
            emissive: LinearRgba::from(star_color) * emissive_power,
            unlit: true,
            ..default()
        });

        commands.spawn((
            Starfield,
            Mesh3d(star_mesh),
            MeshMaterial3d(star_mat),
            Transform::from_xyz(x, y, z),
        ));
    }
}

// ----------------------------------------------------
// SYSTEMS: ESC EXIT, FREELOOK, FLIGHT & ENGINE AUDIO
// ----------------------------------------------------

fn exit_on_esc(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
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

    let sensitivity = 0.003;
    let key_speed = 1.5 * time.delta_secs();

    // Mouse Freelook
    if mouse_delta != Vec2::ZERO {
        flight_state.yaw -= mouse_delta.x * sensitivity;
        flight_state.pitch -= mouse_delta.y * sensitivity;
    }

    // Keyboard Freelook (I/K for Pitch, J/L for Yaw, or Arrow Keys)
    if keyboard.pressed(KeyCode::KeyI) || keyboard.pressed(KeyCode::ArrowUp) {
        flight_state.pitch += key_speed;
    }
    if keyboard.pressed(KeyCode::KeyK) || keyboard.pressed(KeyCode::ArrowDown) {
        flight_state.pitch -= key_speed;
    }
    if keyboard.pressed(KeyCode::KeyJ) || keyboard.pressed(KeyCode::ArrowLeft) {
        flight_state.yaw += key_speed;
    }
    if keyboard.pressed(KeyCode::KeyL) || keyboard.pressed(KeyCode::ArrowRight) {
        flight_state.yaw -= key_speed;
    }

    // Clamp Pitch & Yaw so pilot remains seated looking out viewport
    flight_state.pitch = flight_state.pitch.clamp(-1.2, 1.2); // ~70 deg pitch limit
    flight_state.yaw = flight_state.yaw.clamp(-1.57, 1.57);   // ~90 deg yaw limit

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
    mut flight_state: ResMut<FlightState>,
    mut ship_query: Query<&mut Transform, With<Ship>>,
) {
    let dt = time.delta_secs();
    // Base speed scaled up for astronomical deep space travel
    let mut speed = 180.0;

    // Thruster Boost Mode (Holding Shift accelerates ship further)
    if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
        speed *= 1.8;
    }

    let Ok(mut ship_transform) = ship_query.single_mut() else { return; };

    // Ship Yaw Steering (Q / E keys)
    if keyboard.pressed(KeyCode::KeyQ) {
        ship_transform.rotate_y(0.8 * dt);
    }
    if keyboard.pressed(KeyCode::KeyE) {
        ship_transform.rotate_y(-0.8 * dt);
    }

    // Direction vectors relative to ship heading
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
        flight_state.velocity = flight_state.velocity.lerp(input_dir.normalize() * speed, 5.0 * dt);
    } else {
        flight_state.velocity = flight_state.velocity.lerp(Vec3::ZERO, 3.0 * dt);
    }

    ship_transform.translation += flight_state.velocity * dt;
}

fn orbit_planets_system(time: Res<Time>, mut query: Query<(&Planet, &mut Transform)>) {
    let elapsed = time.elapsed_secs();
    for (planet, mut transform) in &mut query {
        let angle = elapsed * planet.orbit_speed;
        let x = angle.cos() * planet.orbit_radius;
        let z = angle.sin() * planet.orbit_radius;
        transform.translation.x = x;
        transform.translation.z = z;
        transform.rotate_y(planet.rotation_speed * time.delta_secs());
    }
}

fn engine_sound_system(
    time: Res<Time>,
    flight_state: Res<FlightState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut sink_query: Query<&mut AudioSink, With<EngineSound>>,
) {
    let dt = time.delta_secs();

    let max_speed = 324.0; // Scaled boost max speed
    let current_speed = flight_state.velocity.length();
    let speed_ratio = (current_speed / max_speed).clamp(0.0, 1.0);

    let is_thrusting = keyboard.pressed(KeyCode::KeyW)
        || keyboard.pressed(KeyCode::KeyS)
        || keyboard.pressed(KeyCode::KeyA)
        || keyboard.pressed(KeyCode::KeyD)
        || keyboard.pressed(KeyCode::Space)
        || keyboard.pressed(KeyCode::ShiftLeft)
        || keyboard.pressed(KeyCode::ShiftRight);

    let thrust_boost = if is_thrusting { 0.15 } else { 0.0 };

    // Target pitch: ~0.95 at idle drone -> ~1.40 under full throttle
    let target_pitch = 0.95 + (speed_ratio * 0.30) + thrust_boost;
    // Target volume: ~0.45 at idle -> ~0.90 under full throttle
    let target_volume = 0.45 + (speed_ratio * 0.35) + (thrust_boost * 0.10);

    for mut sink in &mut sink_query {
        let current_pitch = sink.speed();
        let new_pitch = current_pitch + (target_pitch - current_pitch) * (6.0 * dt).min(1.0);

        sink.set_speed(new_pitch);
        sink.set_volume(bevy::audio::Volume::Linear(target_volume));
    }
}
