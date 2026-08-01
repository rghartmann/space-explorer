# Space Explorer - Solar System Simulator

**Space Explorer** is an immersive 3D real-time space flight simulator built in Rust powered by the Bevy Engine. Command a 3D starship avatar rendered from a custom Blender model in 3rd-person perspective, and navigate across realistic astronomical scale model representations of our Solar System—from the central Sun out to distant Kuiper Belt dwarf planets.

---

## Features

- **3rd-Person Spaceship Avatar & Camera**: Experience real-time space flight from a sleek 3rd-person camera perspective anchored behind a detailed 3D starship avatar (rendered from `assets/models/spaceship.blend`).
- **Solar System Scale**: Explore the Sun, 8 major planets, major moons (Moon, Io, Europa, Charon), and dwarf planets (Ceres, Pluto, Haumea, Makemake, Eris) alongside procedural main belt asteroids and space dust clouds.
- **Logarithmic Render Engine**: Hybrid rendering pipeline smoothly blending physical 1:1 scale (close proximity) with logarithmic depth mapping (deep space) to visualize astronomical distances without precision loss.
- **Flight & Orbital Mechanics**:
  - Manual Thrusters & Pitch/Yaw mouse steering.
  - FTL Warp Boost and Rapid Braking systems.
  - Planetary Orbit Entry/Exit mode (`[O]`) with orbital rotation controls.
  - AutoNav Target Locking (`[0-9]`) with real-time distance and Speed-of-Light gauges.
- **Celestial HUD Projection**: Dynamic 3D-to-2D viewport projection displaying real-time distance labels and navigation markers for celestial bodies.

---

## Flight Controls

| Key / Input | Action |
| :--- | :--- |
| **W** | Main Forward Thrusters (Accelerate) |
| **S** | Retro-Thrusters (Decelerate / Reverse Thrust) |
| **X / Backspace** | Emergency Full Retro-Stop (Full Stop in Vacuum) |
| **Spacebar** | Toggle 50.0c FTL Warp Boost / Rapid Braking |
| **Mouse Motion** | Flight Steering (Pitch & Yaw) |
| **[0] - [9]** | Engage AutoNav Autopilot to Destination (Sun to Pluto) |
| **O** | Enter / Leave Planetary Orbit |
| **W A S D** | Orbit Pitch & Yaw Navigation (while in Orbit Mode) |
| **Q / E** | Orbit Altitude Closer / Farther (while in Orbit Mode) |
| **Esc** | Exit Simulator |

> [!NOTE]
> **Vacuum Physics (Newton's First Law)**: In deep space vacuum, releasing thruster controls (`W`/`S`) maintains your exact constant speed and trajectory indefinitely. Use **S** (retro-thrusters) or **X / Backspace** (full stop) to decelerate.

---

## Technology Stack

- **Language**: Rust (2024 Edition)
- **Game Engine**: [Bevy Engine v0.19.0](https://bevyengine.org/)
- **Graphics Pipeline**: Modern 3D WebGPU/Vulkan/Metal PBR rendering pipeline with emissive materials and distance fog.
- **Audio**: Bevy spatial sound engine supporting looped audio tracks and procedural hum synthesis.

---

## Credits & Attributions

This project incorporates public domain science datasets, NASA planetary maps, and ambient audio assets.

### ☀️ Central Sun (4K Surface & Animation)
- **Sun Surface Map & Solar Animation Loop**:
  - High-resolution 4K Carrington surface map and 16-frame Extreme Ultraviolet (304 Å EUV) chromosphere animation loop sourced from **NASA Scientific Visualization Studio (SVS)** [SVS 30362: Full Map of the Sun's Surface](https://svs.gsfc.nasa.gov/30362).
  - Datasets captured by **NASA Solar Dynamics Observatory (SDO / AIA)** and **NASA STEREO (EUVI)** spacecraft.

### 🪐 Planetary & Moons Textures
- **Mercury, Venus, Earth, Moon, Mars, Jupiter, Saturn, Uranus, Neptune, Pluto**:
  - Maps sourced and adapted from **NASA / JPL-Caltech**, **USGS Astrogeology Science Center**, and [Solar System Scope](https://www.solarsystemscope.com/textures/).
- **Dwarf Planets (Ceres, Haumea, Makemake, Eris, Charon)**:
  - Surface imagery adapted from NASA Spacecraft Data (**Dawn**, **New Horizons**) and ESA/Hubble Space Telescope observations.
- **Saturn Ring System**:
  - Radial alpha transparency ring map provided by **NASA / JPL / Space Science Institute**.

### 🌌 Deep Space Spheremap & Skybox
- **Deep Space Skybox Spheremap**:
  - High-resolution 10K panoramic starfield spheremap courtesy of **European Southern Observatory (ESO)** and **NASA Deep Space Imagery**.

### 🚀 3D Spaceship Model
- **Spaceship Avatar**:
  - Rendered from custom Blender 3D model (`assets/models/spaceship.blend`).

### 🔊 Audio & Soundscapes
- **Ambient Space Music (`ambient_piano.wav`)**:
  - Deep space ambient piano soundscape.
- **Engine Sound (`engine_hum.wav`)**:
  - Procedurally generated low-frequency sub-bass thruster hum.

---

## Release History

- **`v0.1.0` (Stable Baseline)**: Initial stable release featuring simplified 3rd-person spaceflight navigation controls, AutoNav target locking, celestial HUD projection, dynamic Sun animation, and multi-body planetary rendering. Revert target point for future developments.

---

## License

This project is open-source under the MIT License. Planetary imagery and astronomical datasets remain subject to their respective NASA/JPL/ESO public domain attributions.
