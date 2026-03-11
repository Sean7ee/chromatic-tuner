// PitchDetector is a module that contains structs and methods for pitch detection
//
// needs access to math and pitch detection stuff here

use crate::raudio_util::AudioBuffer;
use crate::raudio_util::BLOCK_SIZE;
use crate::raudio_util::generate_decaying_wave;
use crate::raudio_util::generate_wave;
use crate::signal_processor::FftHandler;
use crate::signal_processor::YinHandler;

use num_complex::Complex32;
use std::f32::consts::PI;

pub const SAMPLING_RATE: f32 = 44100.0;

pub struct PitchTracker {
    alpha: f32,
    stable_pitch: f32,
}

impl PitchTracker {
    pub fn new(alpha: f32) -> Self {
        Self {
            alpha,
            stable_pitch: 0.0,
        }
    }

    pub fn update(&mut self, new_pitch: f32) -> f32 {
        if new_pitch <= 0.0 {
            self.stable_pitch *= 1.0 - self.alpha * 0.5;
            if self.stable_pitch < 1.0 {
                self.stable_pitch = 0.0;
            }
            return self.stable_pitch;
        }
        if self.stable_pitch == 0.0 {
            self.stable_pitch = new_pitch;
        } else {
            self.stable_pitch = self.alpha * new_pitch + (1.0 - self.alpha) * self.stable_pitch;
        }
        self.stable_pitch
    }
}

pub struct PitchDetector {
    sample_rate: f32,
    min_freq: f32,
    tau_thres: f32,
    max_tau: u32,
    audio_block: Box<[f32; BLOCK_SIZE]>,
    yin_core: YinHandler,
}

// assume that audio_block is full whenever we estimate the pitch
impl PitchDetector {
    pub fn new(sample_rate: f32, min_freq: f32, tau_thres: f32) -> PitchDetector {
        let max_tau = (sample_rate / min_freq) as u32;
        let heap_data = vec![0.0; BLOCK_SIZE].into_boxed_slice();
        let sized_data: Box<[f32; BLOCK_SIZE]> = heap_data.try_into().unwrap();
        let yin_core = YinHandler::new(max_tau, tau_thres);
        PitchDetector {
            sample_rate,
            min_freq,
            tau_thres,
            max_tau,
            audio_block: sized_data,
            yin_core,
        }
    }

    pub fn read_from_buffer(&mut self, buffer: &AudioBuffer) {
        buffer.read_block_to(&mut self.audio_block[..]);
    }

    pub fn get_pitch_yin(&mut self) -> f32 {
        self.yin_core.yin_setup(&self.audio_block[..]);
        let tau = self
            .yin_core
            .get_absolute_threshold_tau()
            .unwrap_or_else(|| self.yin_core.get_global_min().unwrap());

        let interpolated_tau = self.yin_core.interpolate_tau(tau);
        self.sample_rate / interpolated_tau // this is the frequency
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yin() {
        let mut ab = AudioBuffer::new();
        let target_freq = 440.0;
        let wave = generate_decaying_wave(target_freq, 0.5, 44100.0, 4.0);
        ab.push(&wave);

        let mut pd = PitchDetector::new(44100.0, 40.0, 0.1);
        pd.read_from_buffer(&ab);
        let result = pd.get_pitch_yin();
        assert!((result - target_freq) < 0.2);
    }
}
