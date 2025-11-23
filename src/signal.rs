use num_complex::Complex;
use std::f64::consts::PI;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ModulationType {
    CW,
    AM,
    FM,
}

pub struct SignalGenerator {
    phase: f64,
    mod_phase: f64,
}

impl SignalGenerator {
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            mod_phase: 0.0,
        }
    }

    pub fn next_sample(
        &mut self,
        frequency: f64,
        sample_rate: f64,
        mod_type: ModulationType,
        mod_freq: f64,
        mod_strength: f64,
    ) -> Complex<f64> {
        // Update modulation phase
        let mod_phase_increment = 2.0 * PI * mod_freq / sample_rate;
        self.mod_phase += mod_phase_increment;
        if self.mod_phase > 2.0 * PI {
            self.mod_phase -= 2.0 * PI;
        }

        let mut current_freq = frequency;
        let mut amplitude_factor = 1.0;

        match mod_type {
            ModulationType::CW => {}
            ModulationType::AM => {
                // AM: A(t) = 1 + m * cos(mod_phase)
                // mod_strength is modulation index (0.0 to 1.0 usually, but can be higher)
                amplitude_factor = 1.0 + mod_strength * self.mod_phase.cos();
            }
            ModulationType::FM => {
                // FM: Instantaneous frequency = fc + dev * cos(mod_phase)
                // mod_strength is frequency deviation in Hz
                current_freq = frequency + mod_strength * self.mod_phase.cos();
            }
        }

        let phase_increment = 2.0 * PI * current_freq / sample_rate;
        self.phase += phase_increment;
        if self.phase > 2.0 * PI {
            self.phase -= 2.0 * PI;
        }

        Complex::from_polar(amplitude_factor, self.phase)
    }

    pub fn generate_block(
        &mut self,
        frequency: f64,
        sample_rate: f64,
        count: usize,
        mod_type: ModulationType,
        mod_freq: f64,
        mod_strength: f64,
    ) -> Vec<Complex<f64>> {
        let mut block = Vec::with_capacity(count);
        for _ in 0..count {
            block.push(self.next_sample(frequency, sample_rate, mod_type, mod_freq, mod_strength));
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
        let sample_rate = 1000.0;
        let frequency = 100.0;

        // Generate 1 second of data
        let samples = gen.generate_block(
            frequency,
            sample_rate,
            sample_rate as usize,
            ModulationType::CW,
            0.0,
            0.0,
        );

        // Check if the phase completes 100 cycles
        // Phase of last sample should be close to 0 (modulo 2pi) if it's exactly integer cycles
        // But let's check the period.
        // 100 Hz means 1 cycle every 10 samples.
        // samples[0] is phase ~ 0.
        // samples[10] should be phase ~ 0.

        let s0 = samples[0];
        let s10 = samples[10];

        // Allow small error due to floating point
        let epsilon = 1e-5;
        assert!((s0.re - s10.re).abs() < epsilon);
        assert!((s0.im - s10.im).abs() < epsilon);
    }
}
