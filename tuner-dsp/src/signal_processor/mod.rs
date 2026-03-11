use crate::raudio_util::AudioBuffer;
use crate::raudio_util::BLOCK_SIZE;
use crate::raudio_util::generate_wave;

use num_complex::Complex32;
use std::f32::consts::PI;

pub struct FftHandler {
    hanning_window: Box<[f32; BLOCK_SIZE]>,
    complex_block: Box<[Complex32; 2 * BLOCK_SIZE]>,
    precomputed_angles: Box<[Complex32; BLOCK_SIZE]>,
}

impl FftHandler {
    pub fn new() -> Self {
        let hanning_window: Vec<f32> = (0..BLOCK_SIZE)
            .map(|x| {
                let n = x as f32;
                let N = BLOCK_SIZE as f32;

                0.5 * (1.0 - ((2.0 * PI * n) / N).cos())
            })
            .collect();
        let boxed_hanning: Box<[f32; BLOCK_SIZE]> = hanning_window.try_into().unwrap();
        let complex_data = vec![Complex32::new(0.0, 0.0); 2 * BLOCK_SIZE].into_boxed_slice();
        let boxed_complex_data: Box<[Complex32; 2 * BLOCK_SIZE]> = complex_data.try_into().unwrap();

        let angles: Vec<Complex32> = (0..BLOCK_SIZE)
            .map(|x| {
                let angle = -2.0 * std::f32::consts::PI * (x as f32) / BLOCK_SIZE as f32;
                Complex32::new(angle.cos(), angle.sin())
            })
            .collect();
        let boxed_angles: Box<[Complex32; BLOCK_SIZE]> = angles.try_into().unwrap();

        Self {
            hanning_window: boxed_hanning,
            complex_block: boxed_complex_data,
            precomputed_angles: boxed_angles,
        }
    }

    pub fn process_block(&mut self, audioblock: &[f32]) {
        assert!(audioblock.len() == BLOCK_SIZE);

        // for loops have an internal "if statement" that slows down the loop, using zip instead
        // we can use "zero-cost abstraction" in Rust using [..] which "unpacks" the Box into a
        // &[f32] iterable slice
        for ((complex_out, &audio), &window) in self.complex_block[..BLOCK_SIZE]
            .iter_mut()
            .zip(audioblock.iter())
            .zip(self.hanning_window[..].iter())
        {
            *complex_out = Complex32::new(audio * window, 0.0);
        }

        for complex_out in self.complex_block[BLOCK_SIZE..].iter_mut() {
            *complex_out = Complex32::new(0.0, 0.0);
        }
    }

    pub fn process_block_raw(&mut self, audioblock: &[f32]) {
        assert!(audioblock.len() == BLOCK_SIZE);

        for (complex_out, &audio) in self.complex_block[..BLOCK_SIZE]
            .iter_mut()
            .zip(audioblock.iter())
        {
            *complex_out = Complex32::new(audio, 0.0);
        }
        for complex_out in self.complex_block[BLOCK_SIZE..].iter_mut() {
            *complex_out = Complex32::new(0.0, 0.0);
        }
    }

    pub fn fft(&mut self, inverse: bool) {
        let n = 2 * BLOCK_SIZE;
        assert!(n.is_power_of_two(), "FFT buffer must be a power of two");
        let bits = n.trailing_zeros();

        for i in 0..n {
            let rev = i.reverse_bits() >> (usize::BITS - bits);

            if i < rev {
                self.complex_block.swap(i, rev);
            }
        }
        let direction = if inverse { 1.0 } else { -1.0 };
        let mut step = 2;
        while step <= n {
            let half_step = step / 2;

            let angle = direction * 2.0 * PI / (step as f32);
            let w_m = Complex32::new(angle.cos(), angle.sin());

            for i in (0..n).step_by(step) {
                let mut w = Complex32::new(1.0, 0.0);

                for j in 0..half_step {
                    let even = i + j;
                    let odd = i + j + half_step;

                    let t = w * self.complex_block[odd];
                    let u = self.complex_block[even];
                    self.complex_block[even] = u + t;
                    self.complex_block[odd] = u - t;

                    w = w * w_m;
                }
            }
            step *= 2;
        }

        if inverse {
            let scale = 1.0 / (n as f32);
            for val in self.complex_block[..].iter_mut() {
                val.re *= scale;
                val.im *= scale;
            }
        }
    }

    pub fn fft_precomputed_angles(&mut self) {
        let n = 2 * BLOCK_SIZE;
        assert!(n.is_power_of_two(), "FFT buffer should be a power of two");
        let bits = n.trailing_zeros();

        for i in 0..n {
            let rev = i.reverse_bits() >> (usize::BITS - bits);

            if i < rev {
                self.complex_block.swap(i, rev);
            }
        }
        let mut step = 2;
        while step <= n {
            let half_step = step / 2;

            let angle_stride = n / step;

            for i in (0..n).step_by(step) {
                for j in 0..half_step {
                    let even = i + j;
                    let odd = i + j + half_step;
                    let w = self.precomputed_angles[j * angle_stride];

                    let t = w * self.complex_block[odd];
                    let u = self.complex_block[even];
                    self.complex_block[even] = u + t;
                    self.complex_block[odd] = u - t;
                }
            }
            step *= 2;
        }
    }

    pub fn ifft_precomputed_angles(&mut self) {
        unimplemented!();
    }

    pub fn power_spectrum(&mut self) {
        // or multiplied by its conjugate complex num
        for complex_num in self.complex_block[..].iter_mut() {
            let power = complex_num.re * complex_num.re + complex_num.im * complex_num.im;
            *complex_num = Complex32::new(power as f32, 0.0);
        }
    }

    pub fn complex_data(&self) -> &[Complex32] {
        &self.complex_block[..]
    }

    pub fn argmax(&self) -> Option<usize> {
        self.complex_block[1..BLOCK_SIZE]
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                a.re.partial_cmp(&(b.re))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(index, _)| index)
    }
}

pub struct YinHandler {
    max_tau: u32,
    tau_thres: f32,
    sum_squared_block: Box<[f32; BLOCK_SIZE]>,
    cmndf_values: Box<[f32]>,
    acf_taus: Box<[f32]>,
    fft_core: FftHandler,
}

impl YinHandler {
    pub fn new(max_tau: u32, tau_thres: f32) -> Self {
        let heap_audio_block = vec![0.0; BLOCK_SIZE].into_boxed_slice();
        let boxed_audio_block: Box<[f32; BLOCK_SIZE]> = heap_audio_block.try_into().unwrap();

        let heap_acf_taus = vec![0.0; max_tau as usize].into_boxed_slice();
        let heap_df_vals = vec![0.0; max_tau as usize].into_boxed_slice();
        let fft_core = FftHandler::new();

        Self {
            max_tau,
            tau_thres,
            sum_squared_block: boxed_audio_block,
            cmndf_values: heap_df_vals,
            acf_taus: heap_acf_taus,
            fft_core,
        }
    }

    fn cumulative_sumsquared(&mut self, buffer: &[f32]) {
        let mut running_sum = 0.0;
        for (sum_sq_elem, audio_elem) in self.sum_squared_block.iter_mut().zip(buffer) {
            running_sum += audio_elem * audio_elem;
            *sum_sq_elem = running_sum;
        }
    }

    fn get_acf_taus(&mut self, buffer: &[f32]) {
        self.fft_core.process_block_raw(buffer);
        self.fft_core.fft(false);
        self.fft_core.power_spectrum();
        self.fft_core.fft(true); // inverse: true
        for (acf_tau, complex_val) in self.acf_taus[..]
            .iter_mut()
            .zip(self.fft_core.complex_data()[1..].iter())
        {
            *acf_tau = complex_val.re;
        }
    }

    fn get_cmndfs(&mut self) {
        let max_tau = (BLOCK_SIZE / 2).min(self.max_tau as usize);
        self.cmndf_values[0] = 1.0;
        let sum_sq = &self.sum_squared_block;

        let sum_sq_0 = sum_sq[0];
        let sum_sq_last = sum_sq[BLOCK_SIZE - 1];

        let mut running_sum = 0.0;

        for tau in 1..max_tau as usize {
            let e_0 = sum_sq[BLOCK_SIZE - tau - 1] - sum_sq_0;
            let e_tau = sum_sq_last - sum_sq[tau - 1];
            let df_val_tau = e_0 + e_tau - 2.0 * self.acf_taus[tau - 1];
            running_sum += df_val_tau;
            if running_sum > 0.0 {
                self.cmndf_values[tau] = (tau as f32) * df_val_tau / running_sum;
            } else {
                self.cmndf_values[tau] = 1.0;
            }
        }
    }

    pub fn get_absolute_threshold_tau(&self) -> Option<usize> {
        let mut tau: usize = 1;
        while tau < self.max_tau as usize {
            if self.cmndf_values[tau] < self.tau_thres {
                while tau + 1 < self.max_tau as usize
                    && self.cmndf_values[tau + 1] < self.cmndf_values[tau]
                {
                    tau += 1;
                }
                return Some(tau);
            }
            tau += 1;
        }
        return None;
    }

    pub fn get_global_min(&self) -> Option<usize> {
        self.cmndf_values[..]
            .iter()
            .enumerate()
            .skip(1)
            .min_by(|(_, a), (_, b)| a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(index, _)| index)
    }

    pub fn interpolate_tau(&self, tau: usize) -> f32 {
        let p = self.cmndf_values[tau - 1];
        let q = self.cmndf_values[tau];
        let r = if tau + 1 < self.max_tau as usize {
            self.cmndf_values[tau + 1]
        } else {
            q
        };
        tau as f32 + ((p - r) / (2.0 * (p - 2.0 * q + r)))
    }

    pub fn interpolate_tau_cosine(&self, tau: usize) -> f32 {
        // only marginal improvement
        let tau_idx = tau as usize;

        // Bounds safety check: prevent panics on the absolute edges
        if tau_idx == 0 || tau_idx >= (self.max_tau as usize) - 1 {
            return tau as f32;
        }

        // Grab the three points of our valley
        let y_minus = self.cmndf_values[tau_idx - 1];
        let y_0 = self.cmndf_values[tau_idx];
        let y_plus = self.cmndf_values[tau_idx + 1];

        // Calculate the denominator of the Parabolic ratio
        let denom = y_minus - 2.0 * y_0 + y_plus;

        // Safety check: if the denominator is 0, the line is perfectly flat.
        // We cannot interpolate a flat line, so we just return the integer.
        if denom == 0.0 {
            return tau as f32;
        }

        // 1. Calculate the raw Parabolic ratio (P)
        let p_ratio = (y_minus - y_plus) / denom;

        // (Standard Parabolic Interpolation would just be: let delta = p_ratio / 2.0;)

        // 2. The Exact Cosine Correction
        let approx_period = tau as f32;
        let pi = std::f32::consts::PI;

        let inner_tan = (pi / approx_period).tan();
        let delta = (approx_period / (2.0 * pi)) * (p_ratio * inner_tan).atan();

        // Add the fractional offset to our integer tau
        approx_period + delta
    }

    pub fn yin_setup(&mut self, buffer: &[f32]) {
        self.cumulative_sumsquared(buffer);
        self.get_acf_taus(buffer);
        self.get_cmndfs();
    }
}

pub fn fft_sanity_check(tgt_freq: f32, result: f32, sampling_rate: f32, block_size: usize) -> bool {
    let bin_size = sampling_rate / block_size as f32;
    let epsilon = tgt_freq - result;
    let abs_e = epsilon.abs();
    abs_e <= bin_size * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_impulse() {
        let mut fft_core = FftHandler::new();
        let mut orig = vec![0.0; BLOCK_SIZE];
        orig[0] = 1.0;
        fft_core.process_block(&orig);
        fft_core.complex_block[0] = Complex32::new(1.0, 0.0);
        fft_core.fft(false);
        assert_eq!(fft_core.complex_block[0], fft_core.complex_block[1]);
        println!("{:?}", fft_core.complex_block[0]);
    }

    #[test]
    fn test_wave() {
        let mut ab = AudioBuffer::new();
        let target_freq = 220.0;
        let wave = generate_wave(target_freq, 0.5, 44100.0);
        ab.push(&wave);

        let mut buffer = [0.0; BLOCK_SIZE];
        ab.read_block_to(&mut buffer);

        let mut fft_core = FftHandler::new();
        fft_core.process_block(&buffer);
        fft_core.fft(false);
        fft_core.power_spectrum();
        let max_index = fft_core.argmax().unwrap() + 1;
        let result = (max_index as f32) * 44100.0 / (2.0 * BLOCK_SIZE as f32);
        assert!(fft_sanity_check(
            target_freq,
            result,
            44100.0,
            BLOCK_SIZE * 2
        ));
    }
}
