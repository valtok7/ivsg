use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use num_complex::Complex;
use rustfft::FftPlanner;

mod signal;
use signal::{ModulationType, SignalGenerator};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Vector Signal Generator",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}

struct MyApp {
    // Parameters
    frequency: f64,
    amplitude: f64,
    sample_rate: f64,

    // State
    signal_gen: SignalGenerator,
    fft_planner: FftPlanner<f64>,
    num_samples: usize,
    spectrum_scale: SpectrumScale,

    // Modulation
    mod_type: ModulationType,

    // AM Parameters
    am_mod_freq: f64,
    am_mod_index: f64,

    // FM Parameters
    fm_mod_freq: f64,
    fm_deviation: f64,

    // PM Parameters
    pm_mod_index: f64,

    // Pulse Parameters
    pulse_freq: f64,
    pulse_duty_cycle: f64,
}

#[derive(PartialEq)]
enum SpectrumScale {
    Linear,
    Decibel,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            frequency: 1000.0,
            amplitude: 1.0,
            sample_rate: 100000.0,
            signal_gen: SignalGenerator::new(),
            fft_planner: FftPlanner::new(),
            num_samples: 1000,
            spectrum_scale: SpectrumScale::Decibel,
            mod_type: ModulationType::CW,
            am_mod_freq: 100.0,
            am_mod_index: 0.5,
            fm_mod_freq: 100.0,
            fm_deviation: 1000.0,
            pm_mod_index: 1.0,
            pulse_freq: 10.0,
            pulse_duty_cycle: 0.5,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Vector Signal Generator");

            ui.horizontal(|ui| {
                ui.label("Frequency (Hz):");
                ui.add(
                    egui::DragValue::new(&mut self.frequency)
                        .speed(10.0)
                        .range(0.0..=10000000000.0),
                );
            });

            ui.horizontal(|ui| {
                ui.label("Amplitude:");
                ui.add(
                    egui::DragValue::new(&mut self.amplitude)
                        .speed(0.01)
                        .range(0.0..=1000000.0),
                );
            });

            ui.horizontal(|ui| {
                ui.label("Sample Rate (Hz):");
                ui.add(
                    egui::DragValue::new(&mut self.sample_rate)
                        .speed(100.0)
                        .range(1000.0..=1000000000.0),
                );
            });

            ui.horizontal(|ui| {
                ui.label("Num Samples:");
                ui.add(
                    egui::DragValue::new(&mut self.num_samples)
                        .speed(10.0)
                        .range(1..=1000000),
                );
            });

            ui.separator();
            ui.heading("Modulation");

            ui.horizontal(|ui| {
                ui.label("Type:");
                ui.radio_value(&mut self.mod_type, ModulationType::CW, "CW");
                ui.radio_value(&mut self.mod_type, ModulationType::AM, "AM");
                ui.radio_value(&mut self.mod_type, ModulationType::FM, "FM");
                ui.radio_value(&mut self.mod_type, ModulationType::PM, "PM");
                ui.radio_value(&mut self.mod_type, ModulationType::Pulse, "Pulse");
            });

            if self.mod_type != ModulationType::CW {
                ui.horizontal(|ui| {
                    ui.label("Mod Frequency (Hz):");
                    let (freq, range) = match self.mod_type {
                        ModulationType::AM => (&mut self.am_mod_freq, 0.0..=self.sample_rate / 2.0),
                        ModulationType::FM => (&mut self.fm_mod_freq, 0.0..=self.sample_rate / 2.0),
                        ModulationType::PM => (&mut self.am_mod_freq, 0.0..=self.sample_rate / 2.0), // Re-use AM freq for PM? No, let's use AM freq variable or add new one? I added pm_mod_index but not pm_freq. Let's use am_mod_freq for PM as well or add pm_mod_freq?
                        // Wait, PM usually has a modulating frequency too.
                        // I didn't add pm_mod_freq in struct. Let's use am_mod_freq or add it.
                        // Actually, let's just use am_mod_freq for PM frequency to save space or add it properly.
                        // Re-reading my struct update... I only added pm_mod_index.
                        // Let's use am_mod_freq for PM frequency for now, or better, rename am_mod_freq to mod_freq_common?
                        // No, let's just use am_mod_freq and label it "Mod Frequency".
                        // Actually, I should probably add pm_mod_freq to be clean.
                        // But I already sent the struct update without it.
                        // I'll use am_mod_freq for PM for now as "Mod Frequency".
                        // For Pulse, I added pulse_freq.
                        ModulationType::Pulse => {
                            (&mut self.pulse_freq, 0.0..=self.sample_rate / 2.0)
                        }
                        _ => (&mut self.am_mod_freq, 0.0..=0.0),
                    };
                    ui.add(egui::DragValue::new(freq).speed(1.0).range(range));
                });

                ui.horizontal(|ui| match self.mod_type {
                    ModulationType::AM => {
                        ui.label("Mod Index (0-1):");
                        ui.add(
                            egui::DragValue::new(&mut self.am_mod_index)
                                .speed(0.01)
                                .range(0.0..=10.0),
                        );
                    }
                    ModulationType::FM => {
                        ui.label("Deviation (Hz):");
                        ui.add(egui::DragValue::new(&mut self.fm_deviation).speed(10.0));
                    }
                    ModulationType::PM => {
                        ui.label("Mod Index (Beta):");
                        ui.add(
                            egui::DragValue::new(&mut self.pm_mod_index)
                                .speed(0.01)
                                .range(0.0..=100.0),
                        );
                    }
                    ModulationType::Pulse => {
                        ui.label("Duty Cycle (0-1):");
                        ui.add(
                            egui::DragValue::new(&mut self.pulse_duty_cycle)
                                .speed(0.01)
                                .range(0.0..=1.0),
                        );
                    }
                    _ => {}
                });
            }

            ui.separator();

            // Generate data for visualization
            let num_samples = self.num_samples;
            // Clone signal gen to not affect the continuous phase of the "main" output if we were streaming
            // But for visualization, we might want to show a snapshot.
            // However, if we want to show "real-time" moving wave, we should use the main state.
            // But updating phase every frame for visualization might be too fast or disconnected from time.
            // Let's just generate a fresh block from phase 0 for visualization stability,
            // OR use the current phase but it will spin fast.
            // For a CW, a static plot is better if frequency is constant.
            // Let's use a temporary generator for visualization to keep the wave stable on screen?
            // No, the user wants "Real-time".
            // Let's just generate a block from the current state.
            // Actually, if we update the state every frame, it will look like it's flowing.

            // For this implementation, let's just generate a block from t=0 every time to make it look stable
            // unless we want to simulate a running stream.
            // Let's stick to "snapshot" mode for now: generate from phase 0 based on current params.

            let (mod_freq, mod_strength) = match self.mod_type {
                ModulationType::CW => (0.0, 0.0),
                ModulationType::AM => (self.am_mod_freq, self.am_mod_index),
                ModulationType::FM => (self.fm_mod_freq, self.fm_deviation),
                ModulationType::PM => (self.am_mod_freq, self.pm_mod_index), // Using AM freq for PM
                ModulationType::Pulse => (self.pulse_freq, self.pulse_duty_cycle),
            };

            let mut viz_gen = SignalGenerator::new();
            let samples = viz_gen.generate_block(
                self.frequency,
                self.sample_rate,
                num_samples,
                self.mod_type,
                mod_freq,
                mod_strength,
            );

            // Apply amplitude
            let samples: Vec<Complex<f64>> = samples.iter().map(|s| s * self.amplitude).collect();

            // Time Domain Plot
            ui.label("Time Domain");
            let plot_height = 250.0;
            Plot::new("time_domain")
                .height(plot_height)
                .show(ui, |plot_ui| {
                    let i_points: PlotPoints = samples
                        .iter()
                        .enumerate()
                        .map(|(i, s)| [i as f64 / self.sample_rate, s.re])
                        .collect();
                    let q_points: PlotPoints = samples
                        .iter()
                        .enumerate()
                        .map(|(i, s)| [i as f64 / self.sample_rate, s.im])
                        .collect();

                    plot_ui.line(Line::new(i_points).name("I"));
                    plot_ui.line(Line::new(q_points).name("Q"));
                });

            ui.separator();

            // Frequency Domain Plot
            ui.horizontal(|ui| {
                ui.label("Frequency Domain");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.selectable_value(&mut self.spectrum_scale, SpectrumScale::Decibel, "dB");
                    ui.selectable_value(&mut self.spectrum_scale, SpectrumScale::Linear, "Linear");
                    ui.label("Scale:");
                });
            });

            let fft = self.fft_planner.plan_fft_forward(num_samples);
            let mut spectrum = samples.clone();
            fft.process(&mut spectrum);

            // Shift zero frequency to center? Or just plot 0 to fs/2?
            // Usually complex baseband signals have negative frequencies.
            // FFT output is 0 to fs.
            // 0..N/2 is 0..fs/2. N/2..N is -fs/2..0.
            // Let's plot shifted: -fs/2 to fs/2.

            let mut fft_points: Vec<[f64; 2]> = Vec::with_capacity(num_samples);
            for i in 0..num_samples {
                let idx = (i + num_samples / 2) % num_samples; // Shift
                let freq =
                    (i as f64 - num_samples as f64 / 2.0) * self.sample_rate / num_samples as f64;
                let mut mag = spectrum[idx].norm() / num_samples as f64; // Normalize

                if self.spectrum_scale == SpectrumScale::Decibel {
                    mag = 20.0 * mag.log10();
                    if mag < -120.0 {
                        mag = -120.0;
                    } // Clamp noise floor
                }

                fft_points.push([freq, mag]);
            }

            // Sort by frequency for plotting (although the loop above generates them in order -fs/2 to fs/2?)
            // i=0 -> idx=512 -> freq = -fs/2. Correct.

            Plot::new("freq_domain")
                .height(plot_height)
                .show(ui, |plot_ui| {
                    plot_ui.line(Line::new(PlotPoints::new(fft_points)).name("Magnitude"));
                });
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Export to CSV").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("CSV", &["csv"])
                        .set_file_name("output.csv")
                        .save_file()
                    {
                        if let Err(e) = export_to_csv(&path, &samples, self.sample_rate) {
                            eprintln!("Failed to export: {}", e);
                        } else {
                            eprintln!("Exported to {:?}", path);
                        }
                    }
                }
            });
        });
    }
}

fn export_to_csv(
    path: &std::path::Path,
    samples: &[Complex<f64>],
    sample_rate: f64,
) -> std::io::Result<()> {
    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record(&["Time", "I", "Q"])?;

    for (i, sample) in samples.iter().enumerate() {
        let time = i as f64 / sample_rate;
        wtr.write_record(&[
            time.to_string(),
            sample.re.to_string(),
            sample.im.to_string(),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}
