use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use num_complex::Complex;
use rustfft::FftPlanner;

mod signal;
use signal::{ModulationType, MultitonePhase, SignalGenerator, SignalParams};

fn load_icon() -> egui::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let icon = image::load_from_memory(include_bytes!("../assets/icon.png"))
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = icon.dimensions();
        let rgba = icon.into_raw();
        (rgba, width, height)
    };

    egui::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "IVSG",
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

    // Multitone Parameters
    multitone_count: usize,
    multitone_spacing: f64,
    multitone_phase: MultitonePhase,
    seed: u64,

    // View Settings
    time_domain_unit: TimeDomainUnit,
}

#[derive(PartialEq)]
enum SpectrumScale {
    Linear,
    Decibel,
}

#[derive(PartialEq, Debug)]
enum TimeDomainUnit {
    Seconds,
    Samples,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            frequency: 1000.0,
            amplitude: 1.0,
            sample_rate: 100000.0,
            fft_planner: FftPlanner::new(),
            num_samples: 1000,
            spectrum_scale: SpectrumScale::Decibel,
            mod_type: ModulationType::CW,
            am_mod_freq: 100.0,
            am_mod_index: 0.5,
            fm_mod_freq: 100.0,
            fm_deviation: 1000.0,
            pm_mod_index: 1.0,
            pulse_freq: 1000.0,
            pulse_duty_cycle: 0.5,
            multitone_count: 10,
            multitone_spacing: 1000.0,
            multitone_phase: MultitonePhase::Random,
            seed: 0,
            time_domain_unit: TimeDomainUnit::Seconds,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top Panel for Controls
        egui::TopBottomPanel::top("controls_panel").show(ctx, |ui| {
            ui.heading("Common Parameters");

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
                ui.radio_value(&mut self.mod_type, ModulationType::Multitone, "Multitone");
            });

            if self.mod_type != ModulationType::CW {
                if self.mod_type == ModulationType::Multitone {
                    ui.horizontal(|ui| {
                        ui.label("Count:");
                        ui.add(egui::DragValue::new(&mut self.multitone_count).range(1..=100));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Spacing (Hz):");
                        ui.add(egui::DragValue::new(&mut self.multitone_spacing).speed(10.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Initial Phase:");
                        egui::ComboBox::new("multitone_phase", "")
                            .selected_text(format!("{:?}", self.multitone_phase))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.multitone_phase,
                                    MultitonePhase::Zero,
                                    "Zero",
                                );
                                ui.selectable_value(
                                    &mut self.multitone_phase,
                                    MultitonePhase::Random,
                                    "Random",
                                );
                                ui.selectable_value(
                                    &mut self.multitone_phase,
                                    MultitonePhase::Schroeder,
                                    "Schroeder",
                                );
                            });
                    });
                    if self.multitone_phase == MultitonePhase::Random {
                        ui.horizontal(|ui| {
                            ui.label("Seed:");
                            ui.add(egui::DragValue::new(&mut self.seed));
                        });
                    }
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Mod Frequency (Hz):");
                        let (freq, range) = match self.mod_type {
                            ModulationType::AM => {
                                (&mut self.am_mod_freq, 0.0..=self.sample_rate / 2.0)
                            }
                            ModulationType::FM => {
                                (&mut self.fm_mod_freq, 0.0..=self.sample_rate / 2.0)
                            }
                            ModulationType::PM => {
                                (&mut self.am_mod_freq, 0.0..=self.sample_rate / 2.0)
                            }
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
            }
        });

        // Generate data for visualization and export
        // We do this outside the panels so we can pass it to the export button in the bottom panel
        // and the plots in the central panel.
        let num_samples = self.num_samples;
        let (mod_freq, mod_strength) = match self.mod_type {
            ModulationType::CW => (0.0, 0.0),
            ModulationType::AM => (self.am_mod_freq, self.am_mod_index),
            ModulationType::FM => (self.fm_mod_freq, self.fm_deviation),
            ModulationType::PM => (self.am_mod_freq, self.pm_mod_index),
            ModulationType::Pulse => (self.pulse_freq, self.pulse_duty_cycle),
            ModulationType::Multitone => (0.0, 0.0),
        };

        let params = SignalParams {
            frequency: self.frequency,
            sample_rate: self.sample_rate,
            mod_type: self.mod_type,
            mod_freq,
            mod_strength,
            multitone_count: self.multitone_count,
            multitone_spacing: self.multitone_spacing,
            multitone_phase: self.multitone_phase,
            seed: self.seed,
        };

        let mut viz_gen = SignalGenerator::new();
        let samples = viz_gen.generate_block(&params, num_samples);
        // Apply amplitude
        let samples: Vec<Complex<f64>> = samples.iter().map(|s| s * self.amplitude).collect();

        // Bottom Panel for Export
        egui::TopBottomPanel::bottom("export_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Export to CSV").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("CSV", &["csv"])
                        .set_file_name("output.csv")
                        .save_file()
                    {
                        if let Err(e) = export_to_csv(&path, &samples) {
                            eprintln!("Failed to export: {}", e);
                        } else {
                            eprintln!("Exported to {:?}", path);
                        }
                    }
                }

                if ui.button("Export to BIN").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Binary", &["bin"])
                        .set_file_name("output.bin")
                        .save_file()
                    {
                        if let Err(e) = export_to_bin(&path, &samples) {
                            eprintln!("Failed to export: {}", e);
                        } else {
                            eprintln!("Exported to {:?}", path);
                        }
                    }
                }
            });
        });

        // Central Panel for Plots
        egui::CentralPanel::default().show(ctx, |ui| {
            // Calculate available height for plots
            // We have two plots, so we divide by 2.
            // We also need to account for labels and spacing.
            let available_height = ui.available_height();
            let plot_height = (available_height - 60.0) / 2.0; // Subtracting some padding

            // Time Domain Plot
            ui.horizontal(|ui| {
                ui.label("Time Domain");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.selectable_value(
                        &mut self.time_domain_unit,
                        TimeDomainUnit::Samples,
                        "Samples",
                    );
                    ui.selectable_value(
                        &mut self.time_domain_unit,
                        TimeDomainUnit::Seconds,
                        "Time (s)",
                    );
                    ui.label("Unit:");
                });
            });
            Plot::new("time_domain")
                .height(plot_height)
                .show(ui, |plot_ui| {
                    let i_points: PlotPoints = samples
                        .iter()
                        .enumerate()
                        .map(|(i, s)| {
                            let x = match self.time_domain_unit {
                                TimeDomainUnit::Seconds => i as f64 / self.sample_rate,
                                TimeDomainUnit::Samples => i as f64,
                            };
                            [x, s.re]
                        })
                        .collect();
                    let q_points: PlotPoints = samples
                        .iter()
                        .enumerate()
                        .map(|(i, s)| {
                            let x = match self.time_domain_unit {
                                TimeDomainUnit::Seconds => i as f64 / self.sample_rate,
                                TimeDomainUnit::Samples => i as f64,
                            };
                            [x, s.im]
                        })
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

            Plot::new("freq_domain")
                .height(plot_height)
                .show(ui, |plot_ui| {
                    plot_ui.line(Line::new(PlotPoints::new(fft_points)).name("Magnitude"));
                });
        });
    }
}

fn export_to_csv(path: &std::path::Path, samples: &[Complex<f64>]) -> std::io::Result<()> {
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(path)?;

    for sample in samples.iter() {
        wtr.write_record(&[sample.re.to_string(), sample.im.to_string()])?;
    }
    wtr.flush()?;
    Ok(())
}

fn export_to_bin(path: &std::path::Path, samples: &[Complex<f64>]) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::File::create(path)?;
    let mut buffer = Vec::with_capacity(samples.len() * 8); // 2 * 4 bytes per sample

    for sample in samples {
        buffer.extend_from_slice(&(sample.re as f32).to_le_bytes());
        buffer.extend_from_slice(&(sample.im as f32).to_le_bytes());
    }

    file.write_all(&buffer)?;
    Ok(())
}
