import math
import struct
import wave
import random
import os

def note_to_freq(midi_note):
    return 440.0 * (2.0 ** ((midi_note - 69) / 12.0))

class StereoReverb:
    def __init__(self, sample_rate, delay_ms_list, decay=0.6, wet=0.32):
        self.sample_rate = sample_rate
        self.delays_l = [int(ms * sample_rate / 1000.0) for ms in delay_ms_list]
        self.delays_r = [int((ms + 7.3) * sample_rate / 1000.0) for ms in delay_ms_list]
        self.buffers_l = [[0.0] * d for d in self.delays_l]
        self.buffers_r = [[0.0] * d for d in self.delays_r]
        self.indices_l = [0] * len(self.delays_l)
        self.indices_r = [0] * len(self.delays_r)
        self.decay = decay
        self.wet = wet

    def process(self, left, right):
        rev_l = 0.0
        rev_r = 0.0

        for i in range(len(self.delays_l)):
            buf_l = self.buffers_l[i]
            idx_l = self.indices_l[i]
            out_l = buf_l[idx_l]
            buf_l[idx_l] = left + out_l * self.decay
            self.indices_l[i] = (idx_l + 1) % self.delays_l[i]
            rev_l += out_l

            buf_r = self.buffers_r[i]
            idx_r = self.indices_r[i]
            out_r = buf_r[idx_r]
            buf_r[idx_r] = right + out_r * self.decay
            self.indices_r[i] = (idx_r + 1) % self.delays_r[i]
            rev_r += out_r

        rev_l /= len(self.delays_l)
        rev_r /= len(self.delays_r)

        out_left = left * (1.0 - self.wet) + rev_l * self.wet
        out_right = right * (1.0 - self.wet) + rev_r * self.wet
        return out_left, out_right

def generate_piano_note(midi_note, duration_sec, velocity=0.7, sample_rate=44100):
    f0 = note_to_freq(midi_note)
    num_samples = int(sample_rate * duration_sec)
    buffer_l = [0.0] * num_samples
    buffer_r = [0.0] * num_samples

    # Panning based on pitch: low notes pan left (-0.35), high notes pan right (+0.35)
    pan = ((midi_note - 60) / 40.0)
    pan = max(-0.4, min(0.4, pan))
    gain_l = math.cos((pan + 0.5) * math.pi / 2.0)
    gain_r = math.sin((pan + 0.5) * math.pi / 2.0)

    # Harmonics profile for piano
    # Inharmonicity constant B
    B = 0.00015
    harmonics = [
        (1, 1.00, 3.5),
        (2, 0.50 * (velocity ** 0.5), 2.8),
        (3, 0.28 * velocity, 2.0),
        (4, 0.16 * velocity, 1.5),
        (5, 0.09 * (velocity ** 1.2), 1.1),
        (6, 0.05 * (velocity ** 1.4), 0.8),
        (7, 0.03 * (velocity ** 1.5), 0.6),
        (8, 0.015 * (velocity ** 1.8), 0.4),
    ]

    # Hammer noise pulse duration ~5ms
    hammer_samples = int(sample_rate * 0.005)

    for i in range(num_samples):
        t = i / sample_rate
        
        # Envelope: Attack 4ms, decay exponential
        if t < 0.004:
            env = t / 0.004
        else:
            env = 1.0

        sample_val = 0.0
        for h_num, h_amp, base_decay in harmonics:
            freq = f0 * h_num * math.sqrt(1.0 + B * (h_num ** 2))
            decay = math.exp(-t / base_decay)
            sample_val += math.sin(2.0 * math.pi * freq * t) * h_amp * decay

        # Add subtle soft hammer noise at start
        if i < hammer_samples:
            hammer_env = (1.0 - (i / hammer_samples))
            noise = (random.random() * 2.0 - 1.0) * 0.08 * velocity * hammer_env
            sample_val += noise

        sample_val *= env * velocity * 0.22

        buffer_l[i] = sample_val * gain_l
        buffer_r[i] = sample_val * gain_r

    return buffer_l, buffer_r

def generate_ambient_piano_track(output_path):
    sample_rate = 44100
    bpm = 58
    beat_duration = 60.0 / bpm
    bar_duration = beat_duration * 4.0
    total_bars = 8
    loop_duration = total_bars * bar_duration
    num_samples = int(sample_rate * loop_duration)

    left_master = [0.0] * num_samples
    right_master = [0.0] * num_samples

    # Harmonic progression (Dmaj9 -> Bm9 -> Gmaj7 -> Aadd9)
    events = [
        # (bar, beat, midi_note, duration_beats, velocity)
        # Bar 1: Dmaj9 bass/chord
        (0, 0.0, 38, 7.5, 0.75),  # D2
        (0, 0.0, 45, 7.0, 0.55),  # A2
        (0, 0.5, 54, 6.0, 0.45),  # F#3
        (0, 1.0, 57, 5.5, 0.45),  # A3
        (0, 1.5, 61, 5.0, 0.50),  # C#4
        (0, 2.0, 64, 4.5, 0.50),  # E4
        (0, 2.5, 78, 4.0, 0.65),  # F#5 (melody)

        # Bar 2 melody accent
        (1, 0.0, 81, 4.0, 0.68),  # A5
        (1, 2.0, 85, 3.5, 0.60),  # C#6
        (1, 3.0, 83, 3.0, 0.55),  # B5

        # Bar 3: Bm9 bass/chord
        (2, 0.0, 35, 7.5, 0.75),  # B1
        (2, 0.0, 42, 7.0, 0.55),  # F#2
        (2, 0.5, 50, 6.0, 0.45),  # D3
        (2, 1.0, 54, 5.5, 0.45),  # F#3
        (2, 1.5, 57, 5.0, 0.50),  # A3
        (2, 2.0, 61, 4.5, 0.50),  # C#4
        (2, 2.5, 78, 4.0, 0.62),  # F#5

        # Bar 4 melody accent
        (3, 1.0, 76, 3.5, 0.58),  # E5
        (3, 2.5, 74, 3.0, 0.60),  # D5
        (3, 3.5, 78, 2.5, 0.62),  # F#5

        # Bar 5: Gmaj7 bass/chord
        (4, 0.0, 31, 7.5, 0.78),  # G1
        (4, 0.0, 38, 7.0, 0.55),  # D2
        (4, 0.5, 55, 6.0, 0.45),  # G3
        (4, 1.0, 59, 5.5, 0.45),  # B3
        (4, 1.5, 62, 5.0, 0.50),  # D4
        (4, 2.0, 66, 4.5, 0.50),  # F#4
        (4, 2.5, 81, 4.0, 0.68),  # A5

        # Bar 6 melody accent
        (5, 1.0, 79, 3.5, 0.60),  # G5
        (5, 2.5, 78, 3.0, 0.62),  # F#5
        (5, 3.5, 74, 2.5, 0.55),  # D5

        # Bar 7: Aadd9 bass/chord
        (6, 0.0, 33, 7.5, 0.75),  # A1
        (6, 0.0, 40, 7.0, 0.55),  # E2
        (6, 0.5, 57, 6.0, 0.45),  # A3
        (6, 1.0, 61, 5.5, 0.45),  # C#4
        (6, 1.5, 64, 5.0, 0.50),  # E4
        (6, 2.0, 71, 4.5, 0.52),  # B4
        (6, 2.5, 76, 4.0, 0.60),  # E5

        # Bar 8 resolving melody into loop start
        (7, 1.0, 74, 4.0, 0.58),  # D5
        (7, 2.5, 71, 3.5, 0.52),  # B4
        (7, 3.5, 66, 3.0, 0.48),  # F#4
    ]

    for bar, beat, midi_note, duration_beats, vel in events:
        start_time = (bar * 4.0 + beat) * beat_duration
        duration_sec = duration_beats * beat_duration
        note_l, note_r = generate_piano_note(midi_note, duration_sec, vel, sample_rate)

        start_idx = int(start_time * sample_rate)
        for i in range(len(note_l)):
            idx = (start_idx + i)
            target_idx = idx % num_samples
            left_master[target_idx] += note_l[i]
            right_master[target_idx] += note_r[i]

    # Apply stereo ambient space reverb
    reverb = StereoReverb(sample_rate, [47.0, 67.0, 91.0, 113.0, 137.0], decay=0.68, wet=0.35)
    processed_l = [0.0] * num_samples
    processed_r = [0.0] * num_samples

    # Warmup reverb for seamless looping
    for i in range(num_samples):
        reverb.process(left_master[i], right_master[i])

    for i in range(num_samples):
        l_out, r_out = reverb.process(left_master[i], right_master[i])
        processed_l[i] = l_out
        processed_r[i] = r_out

    # Normalize max peak to 0.82
    max_peak = 0.0001
    for i in range(num_samples):
        max_peak = max(max_peak, abs(processed_l[i]), abs(processed_r[i]))

    scale = 0.82 / max_peak

    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    with wave.open(output_path, 'wb') as wav_out:
        wav_out.setnchannels(2)
        wav_out.setsampwidth(2) # 16-bit
        wav_out.setframerate(sample_rate)

        frames = bytearray()
        for i in range(num_samples):
            l_sample = int(max(-1.0, min(1.0, processed_l[i] * scale)) * 32767.0)
            r_sample = int(max(-1.0, min(1.0, processed_r[i] * scale)) * 32767.0)
            frames.extend(struct.pack('<hh', l_sample, r_sample))

        wav_out.writeframes(frames)
    print(f"Generated piano track: {output_path} ({loop_duration:.2f}s, {num_samples} samples)")

def generate_engine_hum_track(output_path):
    sample_rate = 44100
    duration_sec = 4.0
    num_samples = int(sample_rate * duration_sec)
    
    left = [0.0] * num_samples
    right = [0.0] * num_samples
    tau = math.pi * 2.0

    # Rich multi-oscillator deep space engine hum
    for i in range(num_samples):
        t = i / sample_rate
        
        # Sub bass beating
        sub1 = math.sin(tau * 42.0 * t) * 0.35
        sub2 = math.sin(tau * 43.5 * t) * 0.25
        
        # Warm engine harmonics
        h1 = math.sin(tau * 84.0 * t) * 0.20
        h2 = math.sin(tau * 126.0 * t) * 0.12
        h3 = math.sin(tau * 168.0 * t) * 0.07
        h4 = math.sin(tau * 252.0 * t) * 0.04
        
        # High turbine shimmer
        turbine_l = math.sin(tau * 336.0 * t + 0.2) * 0.025
        turbine_r = math.sin(tau * 336.0 * t - 0.2) * 0.025
        
        # Soft low-frequency modulation (LFO)
        lfo = 1.0 + 0.08 * math.sin(tau * 0.5 * t)

        val_l = (sub1 + sub2 + h1 + h2 + h3 + h4 + turbine_l) * lfo
        val_r = (sub1 + sub2 + h1 + h2 + h3 + h4 + turbine_r) * lfo
        
        # Soft warmth saturation
        left[i] = math.tanh(val_l * 1.1)
        right[i] = math.tanh(val_r * 1.1)

    # Normalize max peak to 0.70
    max_peak = max(max(abs(x) for x in left), max(abs(x) for x in right))
    scale = 0.70 / max_peak

    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    with wave.open(output_path, 'wb') as wav_out:
        wav_out.setnchannels(2)
        wav_out.setsampwidth(2)
        wav_out.setframerate(sample_rate)

        frames = bytearray()
        for i in range(num_samples):
            l_sample = int(left[i] * scale * 32767.0)
            r_sample = int(right[i] * scale * 32767.0)
            frames.extend(struct.pack('<hh', l_sample, r_sample))

        wav_out.writeframes(frames)
    print(f"Generated engine hum track: {output_path} ({duration_sec:.2f}s)")

if __name__ == '__main__':
    generate_ambient_piano_track("assets/audio/ambient_piano.wav")
    generate_engine_hum_track("assets/audio/engine_hum.wav")
