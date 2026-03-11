// Raudio_util is a utility crate for the pitch_detector module
// Provides the circular buffer for audio samples; provides a window view into the samples that are
// read.

pub const BLOCK_SIZE: usize = 4096;
pub const SAMPLING_RATE: f32 = 44100.0;

// A circular buffer with a few differences for holding audiostream data
// 1. no read_index -> we always read the entire block. Wherever the write_index is we divide the
//    buffer into 2 arrays, ..write_index is new data, write_index..BLOCK_SIZE is old data we
//    connect the two together to provide a contiguous view of the buffer when we read
// 2. Boxed data stored on the heap to abide by ownership rules
pub struct AudioBuffer {
    data: Box<[f32; BLOCK_SIZE]>,
    write_index: usize, // since we read the entire buffer every time, we don't necessarily need a read index
}

impl AudioBuffer {
    pub fn new() -> Self {
        let heap_data = vec![0.0; BLOCK_SIZE].into_boxed_slice();
        // outright defining a Box<[f32; BLOCK_SIZE]> allocates this box in the stack and then
        // moves it into the heap, we want to avoid this as this opens up stack overflow
        // vulnerability.
        let sized_data: Box<[f32; BLOCK_SIZE]> = heap_data.try_into().unwrap();
        Self {
            data: sized_data,
            write_index: 0,
        }
    }

    pub fn push(&mut self, mut incoming: &[f32]) {
        let mut input_len = incoming.len();
        if input_len > BLOCK_SIZE {
            let incoming_start_index = input_len - BLOCK_SIZE;
            incoming = &incoming[incoming_start_index..];
            // can mutate the "view" (mut in front of the pointer incoming which means we can change
            // the address
            input_len = BLOCK_SIZE;
        }
        let space_left = BLOCK_SIZE - self.write_index;

        if input_len <= space_left {
            // fits without wrapping
            self.data[self.write_index..self.write_index + input_len].copy_from_slice(incoming);
            self.write_index = (self.write_index + input_len) & (BLOCK_SIZE - 1);
            // numerically identical to % ONLY FOR BLOCK_SIZEs THAT ARE POWERS OF 2
        } else {
            // needs to wrap
            self.data[self.write_index..BLOCK_SIZE].copy_from_slice(&incoming[..space_left]);
            let remaining = input_len - space_left;
            self.data[0..remaining].copy_from_slice(&incoming[space_left..]);
            self.write_index = remaining;
        }
    }

    pub fn read_block_to(&self, audioblock: &mut [f32]) {
        // we want a writable reference to audioblock
        let oldest = &self.data[self.write_index..BLOCK_SIZE];
        let newest = &self.data[0..self.write_index];
        let split = oldest.len();

        audioblock[..split].copy_from_slice(oldest);
        audioblock[split..].copy_from_slice(newest);
        // need copy to store readable block in contiguous memory
        // copy_from_slice is a simd operation that moves bytes so much better than waiting for allocation
        // .concat() incurs overhead since it calls for allocation
    }

    pub fn clear_buffer(&mut self) {
        self.data.fill(0.0);
        self.write_index = 0;
    }
}

pub fn generate_wave(tgt_freq: f32, duration: f32, sampling_rate: f32) -> Vec<f32> {
    let total_samples = (sampling_rate * duration) as usize;
    let wave: Vec<f32> = (0..total_samples)
        .map(|n| {
            let t = (n as f32) / sampling_rate;
            (2.0 * std::f32::consts::PI * tgt_freq * t).sin()
        })
        .collect();
    wave
}

pub fn generate_decaying_wave(
    tgt_freq: f32,
    duration: f32,
    sampling_rate: f32,
    decay_rate: f32,
) -> Vec<f32> {
    let total_samples = (sampling_rate * duration) as usize;
    let wave: Vec<f32> = (0..total_samples)
        .map(|n| {
            let t = (n as f32) / sampling_rate;
            let signal = (2.0 * std::f32::consts::PI * tgt_freq * t).sin();
            let envelope = (-decay_rate * (t / duration)).exp();
            envelope * signal
        })
        .collect();

    wave
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sin_cos() {
        let pi = std::f32::consts::PI;
        let angle = pi / 2.0;
        let sine_value = angle.sin();
        let cosine_value = angle.cos();
        assert_eq!(sine_value, 1.0);
        assert!(cosine_value <= std::f32::EPSILON);
    }

    #[test]
    fn test_log() {
        let x: f32 = 4.0;
        assert_eq!(x.log2(), 2.0);
    }

    #[test]
    fn test_audio_buffer_init() {
        let new_audio_buffer = AudioBuffer::new();
        assert_eq!(new_audio_buffer.write_index, 0);
        assert_eq!((*new_audio_buffer.data).len(), BLOCK_SIZE);
    }

    #[test]
    fn test_audio_buffer_write() {
        let mut nab = AudioBuffer::new();
        let to_write = vec![1.0; 128];
        nab.push(&to_write);
        assert_eq!(nab.write_index, 128);
    }

    #[test]
    fn test_audio_buffer_write_larger_than_BLOCK_SIZE() {
        let mut nab = AudioBuffer::new();
        let to_write: Vec<f32> = (0..5000).map(|i| i as f32).collect();
        nab.push(&to_write);
        assert_eq!(nab.write_index, 0);
        let result: Vec<f32> = (904..5000).map(|j| j as f32).collect();
        let boxed_result: Box<[f32; BLOCK_SIZE]> = result.try_into().unwrap();
        assert_eq!(nab.data, boxed_result);
    }

    #[test]
    fn test_audio_buffer_read() {
        let mut nab = AudioBuffer::new();
        let to_write: Vec<f32> = (0..4096).map(|i| i as f32).collect();
        nab.push(&to_write);
        assert_eq!(nab.write_index, 0);
        let mut read_buffer = [0.0; BLOCK_SIZE];
        nab.read_block_to(&mut read_buffer);
        println!("read: {:?}", read_buffer);
    }

    #[test]
    fn wave_generation() {
        let wave = generate_wave(440.0, 1.0, SAMPLING_RATE);
        assert_eq!(wave.len(), 44100);
        assert_eq!(wave[0], 0.0);
    }

    #[test]
    fn wave_buffer_performance() {
        let wave = generate_wave(440.0, 0.1, SAMPLING_RATE);
        assert_eq!(wave.len(), 4410);

        let mut buffer = AudioBuffer::new();
        let chunk_size = 128;
        let mut callbacks_simulated = 0;

        for chunk in wave.chunks(chunk_size) {
            buffer.push(chunk);
            callbacks_simulated += 1;
        }

        assert_eq!(callbacks_simulated, 35);
        assert_eq!(buffer.write_index, 314);

        buffer.clear_buffer();
        assert_eq!(buffer.write_index, 0);
    }
}
