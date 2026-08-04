use std::f32::consts::PI;
use std::fs;
use std::path::Path;

use bevy::prelude::*;

use crate::components::EngineSound;
use crate::resources::{AppState, FlightState};

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            engine_sound_system.run_if(in_state(AppState::InGame)),
        );
    }
}

pub fn build_wav_bytes(pcm: &[i16], sample_rate: u32) -> Vec<u8> {
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
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());

    for &sample in pcm {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}

pub fn ensure_audio_file(file_name: &str, generator: impl FnOnce() -> Vec<u8>) {
    let dir = Path::new("assets/audio");
    if !dir.exists() {
        let _ = fs::create_dir_all(dir);
    }
    let file_path = dir.join(file_name);
    if !file_path.exists() {
        let wav_data = generator();
        let _ = fs::write(file_path, wav_data);
    }
}

pub fn generate_engine_hum_wav() -> Vec<u8> {
    let sample_rate = 44100;
    let duration_secs = 4.0;
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut pcm = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sub_bass = (2.0 * PI * 55.0 * t).sin();
        let mid_drone = (2.0 * PI * 110.0 * t).sin() * 0.4;
        let harmonic = (2.0 * PI * 165.0 * t).sin() * 0.15;
        let lfo = (2.0 * PI * 0.5 * t).sin() * 0.15 + 0.85;

        let sample = (sub_bass + mid_drone + harmonic) * 0.35 * lfo;
        let val = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
        pcm.push(val);
    }

    build_wav_bytes(&pcm, sample_rate)
}

pub fn ensure_engine_hum_file() {
    ensure_audio_file("engine_hum.wav", generate_engine_hum_wav);
}

pub fn generate_ambient_piano_wav() -> Vec<u8> {
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
                let fundamental = (2.0 * PI * freq * note_time).sin();
                let harmonic = (2.0 * PI * (freq * 2.0) * note_time).sin() * 0.25;
                sample += (fundamental + harmonic) * env * 0.18;
            }
        }

        let val = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
        pcm.push(val);
    }

    build_wav_bytes(&pcm, sample_rate)
}

pub fn ensure_ambient_piano_file() {
    ensure_audio_file("ambient_piano.wav", generate_ambient_piano_wav);
}

pub fn engine_sound_system(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_wav_bytes_header() {
        let pcm = vec![0i16; 100];
        let wav = build_wav_bytes(&pcm, 44100);

        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");

        let data_len = u32::from_le_bytes(wav[40..44].try_into().unwrap());
        assert_eq!(data_len, 200);
        assert_eq!(wav.len(), 44 + 200);
    }
}
