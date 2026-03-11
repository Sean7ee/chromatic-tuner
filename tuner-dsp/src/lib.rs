pub mod pitch_detector;
pub mod raudio_util;
pub mod signal_processor;

pub const BLOCK_SIZE: usize = 4096;
pub const SAMPLING_RATE: f32 = 44100.0; // or 48000
pub const ALPHA: f32 = 0.25;

use pitch_detector::PitchDetector;
use pitch_detector::PitchTracker;
use raudio_util::AudioBuffer;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmTuner {
    pd: PitchDetector,
    ab: AudioBuffer,
    pt: PitchTracker,
}

#[wasm_bindgen]
impl WasmTuner {
    #[wasm_bindgen(constructor)]
    pub fn new(sampling_rate: f32, tau_thres: f32, min_freq: f32) -> Self {
        let pd = PitchDetector::new(sampling_rate, min_freq, tau_thres);
        let ab = AudioBuffer::new();
        let pt = PitchTracker::new(ALPHA);

        Self { pd, ab, pt }
    }

    pub fn process_audio(&mut self, buffer: &[f32]) -> f32 {
        let mut sum_sq = 0.0;
        for &sample in buffer {
            sum_sq += sample * sample;
        }
        let mean_sq = sum_sq / buffer.len() as f32;
        let noise_floor_thres = 0.0001;
        if mean_sq < noise_floor_thres {
            return 0.0;
        }
        self.ab.push(buffer);
        self.pd.read_from_buffer(&self.ab);
        let new_pitch = self.pd.get_pitch_yin();
        self.pt.update(new_pitch)
    }
}
