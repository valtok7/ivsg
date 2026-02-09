//! IVSG - Interactive Vector Signal Generator
//!
//! IVSGは、様々な変調方式をサポートする対話的なベクトル信号生成器です。
//! CW、AM、FM、PM、パルス、マルチトーン信号を生成し、
//! 時間領域・周波数領域でリアルタイムに可視化できます。
//! 生成した信号はCSVまたはバイナリ形式でエクスポート可能です。

use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use num_complex::Complex;
use rustfft::FftPlanner;
use serde::{Deserialize, Serialize};

mod signal;
use signal::{ModulationType, MultitonePhase, SignalGenerator, SignalParams};

/// アプリケーションアイコンを読み込む
///
/// assets/icon.pngからアイコンを読み込み、egui形式に変換します。
///
/// # 戻り値
/// egui形式のアイコンデータ
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

/// アプリケーションのエントリーポイント
///
/// eframeフレームワークを使用してGUIアプリケーションを起動します。
/// ウィンドウサイズは1200x800ピクセルで初期化されます。
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

/// IVSGアプリケーションのメイン構造体
///
/// 信号生成パラメータ、UI状態、プロット設定などを保持します。
struct MyApp {
    // === 基本パラメータ ===
    /// 搬送波周波数 (Hz)
    frequency: f64,
    /// 信号振幅
    amplitude: f64,
    /// サンプリングレート (Hz)
    sample_rate: f64,

    // === 内部状態 ===
    /// FFT計算用のプランナー
    fft_planner: FftPlanner<f64>,
    /// 生成するサンプル数
    num_samples: usize,
    /// スペクトラム表示のスケール（線形/dB）
    spectrum_scale: SpectrumScale,

    // === 変調設定 ===
    /// 変調方式
    mod_type: ModulationType,

    // === AM変調パラメータ ===
    /// AM変調周波数 (Hz)
    am_mod_freq: f64,
    /// AM変調指数 (0-1)
    am_mod_index: f64,

    // === FM変調パラメータ ===
    /// FM変調周波数 (Hz)
    fm_mod_freq: f64,
    /// FM周波数偏移 (Hz)
    fm_deviation: f64,

    // === PM変調パラメータ ===
    /// PM変調指数 (Beta)
    pm_mod_index: f64,

    // === パルス変調パラメータ ===
    /// パルス周波数 (Hz)
    pulse_freq: f64,
    /// パルスデューティサイクル (0-1)
    pulse_duty_cycle: f64,

    // === マルチトーンパラメータ ===
    /// トーン数
    multitone_count: usize,
    /// トーン間隔 (Hz)
    multitone_spacing: f64,
    /// 初期位相設定
    multitone_phase: MultitonePhase,
    /// ランダム位相生成用シード
    seed: u64,

    // === 表示設定 ===
    /// 時間軸の単位（秒/サンプル数）
    time_domain_unit: TimeDomainUnit,
    /// 時間領域プロット表示フラグ
    show_time_domain: bool,
    /// 周波数領域プロット表示フラグ
    show_freq_domain: bool,

    // === プロット制御用の内部状態 ===
    /// 前回の時間軸単位（単位変更検出用）
    last_time_domain_unit: TimeDomainUnit,
    /// 前回のプロット範囲
    last_plot_bounds: Option<egui_plot::PlotBounds>,
    /// 強制的に設定するプロット範囲（単位変更時に使用）
    forced_plot_bounds: Option<egui_plot::PlotBounds>,
}

/// スペクトラム表示のスケール設定
#[derive(PartialEq, Serialize, Deserialize)]
enum SpectrumScale {
    /// 線形スケール
    Linear,
    /// デシベル（dB）スケール
    Decibel,
}

/// 時間軸の単位設定
#[derive(PartialEq, Debug, Serialize, Deserialize, Clone, Copy)]
enum TimeDomainUnit {
    /// 秒単位で表示
    Seconds,
    /// サンプル数単位で表示
    Samples,
}

/// アプリケーションパラメータの保存/復元用構造体
///
/// すべてのユーザー設定可能なパラメータを含み、JSON形式でシリアライズ可能です。
#[derive(Serialize, Deserialize)]
struct AppParams {
    frequency: f64,
    amplitude: f64,
    sample_rate: f64,
    num_samples: usize,
    spectrum_scale: SpectrumScale,
    mod_type: ModulationType,
    am_mod_freq: f64,
    am_mod_index: f64,
    fm_mod_freq: f64,
    fm_deviation: f64,
    pm_mod_index: f64,
    pulse_freq: f64,
    pulse_duty_cycle: f64,
    multitone_count: usize,
    multitone_spacing: f64,
    multitone_phase: MultitonePhase,
    seed: u64,
    time_domain_unit: TimeDomainUnit,
    show_time_domain: bool,
    show_freq_domain: bool,
}

impl AppParams {
    /// MyAppからパラメータを抽出してAppParamsを生成
    ///
    /// # 引数
    /// * `app` - 現在のアプリケーション状態
    ///
    /// # 戻り値
    /// シリアライズ可能なパラメータ構造体
    fn from_app(app: &MyApp) -> Self {
        Self {
            frequency: app.frequency,
            amplitude: app.amplitude,
            sample_rate: app.sample_rate,
            num_samples: app.num_samples,
            spectrum_scale: match app.spectrum_scale {
                SpectrumScale::Linear => SpectrumScale::Linear,
                SpectrumScale::Decibel => SpectrumScale::Decibel,
            },
            mod_type: app.mod_type,
            am_mod_freq: app.am_mod_freq,
            am_mod_index: app.am_mod_index,
            fm_mod_freq: app.fm_mod_freq,
            fm_deviation: app.fm_deviation,
            pm_mod_index: app.pm_mod_index,
            pulse_freq: app.pulse_freq,
            pulse_duty_cycle: app.pulse_duty_cycle,
            multitone_count: app.multitone_count,
            multitone_spacing: app.multitone_spacing,
            multitone_phase: app.multitone_phase,
            seed: app.seed,
            time_domain_unit: match app.time_domain_unit {
                TimeDomainUnit::Seconds => TimeDomainUnit::Seconds,
                TimeDomainUnit::Samples => TimeDomainUnit::Samples,
            },
            show_time_domain: app.show_time_domain,
            show_freq_domain: app.show_freq_domain,
        }
    }

    /// 保存されたパラメータをMyAppに適用
    ///
    /// # 引数
    /// * `app` - パラメータを適用する対象のアプリケーション
    fn apply_to_app(self, app: &mut MyApp) {
        app.frequency = self.frequency;
        app.amplitude = self.amplitude;
        app.sample_rate = self.sample_rate;
        app.num_samples = self.num_samples;
        app.spectrum_scale = match self.spectrum_scale {
            SpectrumScale::Linear => SpectrumScale::Linear,
            SpectrumScale::Decibel => SpectrumScale::Decibel,
        };
        app.mod_type = self.mod_type;
        app.am_mod_freq = self.am_mod_freq;
        app.am_mod_index = self.am_mod_index;
        app.fm_mod_freq = self.fm_mod_freq;
        app.fm_deviation = self.fm_deviation;
        app.pm_mod_index = self.pm_mod_index;
        app.pulse_freq = self.pulse_freq;
        app.pulse_duty_cycle = self.pulse_duty_cycle;
        app.multitone_count = self.multitone_count;
        app.multitone_spacing = self.multitone_spacing;
        app.multitone_phase = self.multitone_phase;
        app.seed = self.seed;
        app.time_domain_unit = match self.time_domain_unit {
            TimeDomainUnit::Seconds => TimeDomainUnit::Seconds,
            TimeDomainUnit::Samples => TimeDomainUnit::Samples,
        };
        app.show_time_domain = self.show_time_domain;
        app.show_freq_domain = self.show_freq_domain;
    }
}

impl Default for MyApp {
    /// MyAppのデフォルト値を設定
    ///
    /// 起動時のデフォルトパラメータ：
    /// - 搬送波周波数: 1kHz
    /// - 振幅: 1.0
    /// - サンプリングレート: 100kHz
    /// - サンプル数: 1000
    /// - 変調方式: CW（無変調）
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
            show_time_domain: true,
            show_freq_domain: true,
            last_time_domain_unit: TimeDomainUnit::Seconds,
            last_plot_bounds: None,
            forced_plot_bounds: None,
        }
    }
}

impl eframe::App for MyApp {
    /// アプリケーションのUIを更新
    ///
    /// フレームごとに呼び出され、UI描画と状態更新を行います。
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // === トップパネル：制御UI ===
        egui::TopBottomPanel::top("controls_panel").show(ctx, |ui| {
            // パラメータの保存/復元ボタン
            ui.horizontal(|ui| {
                // パラメータ保存
                if ui.button("Save Parameters").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("JSON", &["json"])
                        .set_file_name("params.json")
                        .save_file()
                    {
                        let params = AppParams::from_app(self);
                        if let Ok(json) = serde_json::to_string_pretty(&params) {
                            if let Err(e) = std::fs::write(&path, json) {
                                eprintln!("Failed to save parameters: {}", e);
                            }
                        }
                    }
                }
                // パラメータ復元
                if ui.button("Recall Parameters").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("JSON", &["json"])
                        .pick_file()
                    {
                        if let Ok(json) = std::fs::read_to_string(&path) {
                            if let Ok(params) = serde_json::from_str::<AppParams>(&json) {
                                params.apply_to_app(self);
                            } else {
                                eprintln!("Failed to parse parameters");
                            }
                        } else {
                            eprintln!("Failed to read parameters file");
                        }
                    }
                }
            });
            ui.separator();

            // === 共通パラメータセクション ===
            ui.heading("Common Parameters");

            // 周波数設定
            ui.horizontal(|ui| {
                ui.label("Frequency (Hz):");
                ui.add(
                    egui::DragValue::new(&mut self.frequency)
                        .speed(10.0)
                        .range(0.0..=10000000000.0),
                );
            });

            // 振幅設定
            ui.horizontal(|ui| {
                ui.label("Amplitude:");
                ui.add(
                    egui::DragValue::new(&mut self.amplitude)
                        .speed(0.01)
                        .range(0.0..=1000000.0),
                );
            });

            // サンプリングレート設定
            ui.horizontal(|ui| {
                ui.label("Sample Rate (Hz):");
                ui.add(
                    egui::DragValue::new(&mut self.sample_rate)
                        .speed(100.0)
                        .range(1000.0..=1000000000.0),
                );
            });

            // サンプル数設定
            ui.horizontal(|ui| {
                ui.label("Num Samples:");
                ui.add(
                    egui::DragValue::new(&mut self.num_samples)
                        .speed(10.0)
                        .range(1..=1000000),
                );
            });

            // 表示切替チェックボックス
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.show_time_domain, "Show Time Domain");
                ui.checkbox(&mut self.show_freq_domain, "Show Freq Domain");
            });

            ui.separator();

            // === 変調設定セクション ===
            ui.heading("Modulation");

            // 変調タイプ選択
            ui.horizontal(|ui| {
                ui.label("Type:");
                ui.radio_value(&mut self.mod_type, ModulationType::CW, "CW");
                ui.radio_value(&mut self.mod_type, ModulationType::AM, "AM");
                ui.radio_value(&mut self.mod_type, ModulationType::FM, "FM");
                ui.radio_value(&mut self.mod_type, ModulationType::PM, "PM");
                ui.radio_value(&mut self.mod_type, ModulationType::Pulse, "Pulse");
                ui.radio_value(&mut self.mod_type, ModulationType::Multitone, "Multitone");
            });

            // 変調タイプ別のパラメータ設定
            if self.mod_type != ModulationType::CW {
                if self.mod_type == ModulationType::Multitone {
                    // マルチトーン固有のパラメータ
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
                    // AM/FM/PM/Pulse共通の変調周波数設定
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

                    // 変調タイプ別の変調強度パラメータ
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

        // === 信号生成 ===
        // 可視化とエクスポートのためのデータを生成
        // パネル外で生成することで、ボトムパネル（エクスポート）と
        // セントラルパネル（プロット）の両方で使用可能にする
        let num_samples = self.num_samples;

        // 変調タイプに応じて変調パラメータを設定
        let (mod_freq, mod_strength) = match self.mod_type {
            ModulationType::CW => (0.0, 0.0),
            ModulationType::AM => (self.am_mod_freq, self.am_mod_index),
            ModulationType::FM => (self.fm_mod_freq, self.fm_deviation),
            ModulationType::PM => (self.am_mod_freq, self.pm_mod_index),
            ModulationType::Pulse => (self.pulse_freq, self.pulse_duty_cycle),
            ModulationType::Multitone => (0.0, 0.0),
        };

        // 信号生成パラメータを構築
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

        // 信号を生成
        let mut viz_gen = SignalGenerator::new();
        let samples = viz_gen.generate_block(&params, num_samples);

        // 振幅を適用
        let samples: Vec<Complex<f64>> = samples.iter().map(|s| s * self.amplitude).collect();

        // === ボトムパネル：エクスポート機能 ===
        egui::TopBottomPanel::bottom("export_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // CSV形式でエクスポート
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

                // バイナリ形式でエクスポート
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

        // === セントラルパネル：プロット表示 ===
        egui::CentralPanel::default().show(ctx, |ui| {
            // プロットの高さを計算
            // 2つのプロット（時間領域・周波数領域）を表示するため、
            // 利用可能な高さを2分割し、ラベルとスペース分を考慮
            let available_height = ui.available_height();
            let plot_height = (available_height - 60.0) / 2.0;

            // === 時間領域プロット ===
            if self.show_time_domain {
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

                // 時間軸単位の変更を検出し、プロット範囲を調整
                // ユーザーがズーム/パン操作した範囲を新しい単位でも維持する
                if self.time_domain_unit != self.last_time_domain_unit {
                    if let Some(bounds) = self.last_plot_bounds {
                        let min = bounds.min();
                        let max = bounds.max();

                        // X軸の範囲を新しい単位に変換
                        let (new_min_x, new_max_x) = match self.time_domain_unit {
                            TimeDomainUnit::Samples => {
                                // 秒 → サンプル数：サンプリングレートを掛ける
                                (min[0] * self.sample_rate, max[0] * self.sample_rate)
                            }
                            TimeDomainUnit::Seconds => {
                                // サンプル数 → 秒：サンプリングレートで割る
                                (min[0] / self.sample_rate, max[0] / self.sample_rate)
                            }
                        };

                        // Y軸は変更なし
                        let new_min_y = min[1];
                        let new_max_y = max[1];

                        // 新しい範囲を設定
                        self.forced_plot_bounds = Some(egui_plot::PlotBounds::from_min_max(
                            [new_min_x, new_min_y],
                            [new_max_x, new_max_y],
                        ));
                    }
                    self.last_time_domain_unit = self.time_domain_unit;
                }

                // プロットを描画
                let plot_response =
                    Plot::new("time_domain")
                        .height(plot_height)
                        .show(ui, |plot_ui| {
                            // 強制的な範囲設定がある場合は適用（単位変更時）
                            if let Some(bounds) = self.forced_plot_bounds {
                                plot_ui.set_plot_bounds(bounds);
                                self.forced_plot_bounds = None;
                            }

                            // I成分（実部）のプロットポイントを生成
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

                            // Q成分（虚部）のプロットポイントを生成
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

                            // I/Q成分をプロット
                            plot_ui.line(Line::new(i_points).name("I"));
                            plot_ui.line(Line::new(q_points).name("Q"));
                        });

                // 現在のプロット範囲を保存（単位変更検出用）
                self.last_plot_bounds = Some(plot_response.transform.bounds().clone());

                ui.separator();
            }

            // === 周波数領域プロット ===
            if self.show_freq_domain {
                ui.horizontal(|ui| {
                    ui.label("Frequency Domain");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.selectable_value(&mut self.spectrum_scale, SpectrumScale::Decibel, "dB");
                        ui.selectable_value(
                            &mut self.spectrum_scale,
                            SpectrumScale::Linear,
                            "Linear",
                        );
                        ui.label("Scale:");
                    });
                });

                // FFTを実行してスペクトラムを計算
                let fft = self.fft_planner.plan_fft_forward(num_samples);
                let mut spectrum = samples.clone();
                fft.process(&mut spectrum);

                // スペクトラムデータをプロット用に変換
                let mut fft_points: Vec<[f64; 2]> = Vec::with_capacity(num_samples);
                for i in 0..num_samples {
                    // FFT結果をシフトして周波数軸を中心に配置
                    let idx = (i + num_samples / 2) % num_samples;

                    // 周波数を計算（負の周波数を含む）
                    let freq = (i as f64 - num_samples as f64 / 2.0) * self.sample_rate
                        / num_samples as f64;

                    // 振幅を計算して正規化
                    let mut mag = spectrum[idx].norm() / num_samples as f64;

                    // スケール変換（線形またはdB）
                    if self.spectrum_scale == SpectrumScale::Decibel {
                        mag = 20.0 * mag.log10();
                        // ノイズフロアを-120dBでクランプ
                        if mag < -120.0 {
                            mag = -120.0;
                        }
                    }

                    fft_points.push([freq, mag]);
                }

                // スペクトラムをプロット
                Plot::new("freq_domain")
                    .height(plot_height)
                    .show(ui, |plot_ui| {
                        plot_ui.line(Line::new(PlotPoints::new(fft_points)).name("Magnitude"));
                    });
            }
        });
    }
}

/// サンプルをCSV形式でエクスポート
///
/// I/Q成分を2列のCSVファイルとして出力します。
/// ヘッダー行は含みません。
///
/// # 引数
/// * `path` - 出力先ファイルパス
/// * `samples` - エクスポートする複素数サンプル配列
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
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

/// サンプルをバイナリ形式でエクスポート
///
/// I/Q成分を32ビット浮動小数点数（リトルエンディアン）として出力します。
/// 各サンプルは8バイト（I: 4バイト + Q: 4バイト）で表現されます。
///
/// # 引数
/// * `path` - 出力先ファイルパス
/// * `samples` - エクスポートする複素数サンプル配列
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
fn export_to_bin(path: &std::path::Path, samples: &[Complex<f64>]) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::File::create(path)?;
    // バッファを事前確保（各サンプル8バイト = I(4バイト) + Q(4バイト)）
    let mut buffer = Vec::with_capacity(samples.len() * 8);

    for sample in samples {
        // f64をf32に変換してリトルエンディアンでバイト列化
        buffer.extend_from_slice(&(sample.re as f32).to_le_bytes());
        buffer.extend_from_slice(&(sample.im as f32).to_le_bytes());
    }

    file.write_all(&buffer)?;
    Ok(())
}
