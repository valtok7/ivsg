use num_complex::Complex;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::f64::consts::PI;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ModulationType {
    CW,
    AM,
    FM,
    PM,
    Pulse,
    Multitone,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MultitonePhase {
    Zero,
    Random,
    Schroeder,
}

pub struct SignalParams {
    pub frequency: f64,
    pub sample_rate: f64,
    pub mod_type: ModulationType,
    pub mod_freq: f64,
    pub mod_strength: f64,
    pub multitone_count: usize,
    pub multitone_spacing: f64,
    pub multitone_phase: MultitonePhase,
    pub seed: u64,
}

pub struct SignalGenerator {
    phase: f64,
    mod_phase: f64,
    multitone_phases: Vec<f64>,
}

impl SignalGenerator {
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            mod_phase: 0.0,
            multitone_phases: Vec::new(),
        }
    }

    pub fn next_sample(&mut self, params: &SignalParams) -> Complex<f64> {
        if params.mod_type == ModulationType::Multitone {
            return self.next_multitone_sample(params);
        }

        // Update modulation phase
        let mod_phase_increment = 2.0 * PI * params.mod_freq / params.sample_rate;
        self.mod_phase += mod_phase_increment;
        if self.mod_phase > 2.0 * PI {
            self.mod_phase -= 2.0 * PI;
        }

        let mut current_freq = params.frequency;
        let mut amplitude_factor = 1.0;

        match params.mod_type {
            ModulationType::CW => {}
            ModulationType::AM => {
                amplitude_factor = 1.0 + params.mod_strength * self.mod_phase.cos();
            }
            ModulationType::FM => {
                current_freq = params.frequency + params.mod_strength * self.mod_phase.cos();
            }
            ModulationType::PM => {
                // PM handled at phase output
            }
            ModulationType::Pulse => {
                if self.mod_phase < params.mod_strength * 2.0 * PI {
                    amplitude_factor = 1.0;
                } else {
                    amplitude_factor = 0.0;
                }
            }
            ModulationType::Multitone => unreachable!(),
        }

        let phase_increment = 2.0 * PI * current_freq / params.sample_rate;
        self.phase += phase_increment;
        if self.phase > 2.0 * PI {
            self.phase -= 2.0 * PI;
        }

        let mut final_phase = self.phase;
        if params.mod_type == ModulationType::PM {
            final_phase += params.mod_strength * self.mod_phase.cos();
        }

        Complex::from_polar(amplitude_factor, final_phase)
    }

    fn next_multitone_sample(&mut self, params: &SignalParams) -> Complex<f64> {
        // Initialize phases if needed
        if self.multitone_phases.len() != params.multitone_count {
            self.multitone_phases = Vec::with_capacity(params.multitone_count);
            let n = params.multitone_count as f64;

            match params.multitone_phase {
                MultitonePhase::Zero => {
                    for _ in 0..params.multitone_count {
                        self.multitone_phases.push(0.0);
                    }
                }
                MultitonePhase::Random => {
                    let mut rng = StdRng::seed_from_u64(params.seed);
                    for _ in 0..params.multitone_count {
                        self.multitone_phases.push(rng.gen_range(0.0..2.0 * PI));
                    }
                }
                MultitonePhase::Schroeder => {
                    for k in 0..params.multitone_count {
                        let k_f = k as f64;
                        // Schroeder phase: -pi * k * (k - 1) / N
                        let phi = -PI * k_f * (k_f - 1.0) / n;
                        self.multitone_phases.push(phi);
                    }
                }
            }
        }

        // Sum tones
        let mut i_sum = 0.0;
        let mut q_sum = 0.0;

        let center_offset = (params.multitone_count as f64 - 1.0) / 2.0;

        for (k, phase) in self.multitone_phases.iter_mut().enumerate() {
            let freq_offset = (k as f64 - center_offset) * params.multitone_spacing;
            let tone_freq = params.frequency + freq_offset;

            let phase_inc = 2.0 * PI * tone_freq / params.sample_rate;
            *phase += phase_inc;
            if *phase > 2.0 * PI {
                *phase -= 2.0 * PI;
            }

            let (sin, cos) = phase.sin_cos();
            i_sum += cos;
            q_sum += sin;
        }

        // Normalize by N so that max amplitude is 1.0, matching other modes.
        let scale = 1.0 / params.multitone_count as f64;
        Complex::new(i_sum * scale, q_sum * scale)
    }

    pub fn generate_block(&mut self, params: &SignalParams, count: usize) -> Vec<Complex<f64>> {
        let mut block = Vec::with_capacity(count);
        for _ in 0..count {
            block.push(self.next_sample(params));
        }
        block
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frequency() {
        let mut gen = SignalGenerator::new();
        let params = SignalParams {
            frequency: 100.0,
            sample_rate: 1000.0,
            mod_type: ModulationType::CW,
            mod_freq: 0.0,
            mod_strength: 0.0,
            multitone_count: 1,
            multitone_spacing: 0.0,
            multitone_phase: MultitonePhase::Zero,
            seed: 0,
        };

        // Generate 1 second of data
        let samples = gen.generate_block(&params, params.sample_rate as usize);

        let s0 = samples[0];
        let s10 = samples[10];

        let epsilon = 1e-5;
        assert!((s0.re - s10.re).abs() < epsilon);
        assert!((s0.im - s10.im).abs() < epsilon);
    }
}
