//! 信号生成モジュール
//!
//! このモジュールは、様々な変調方式をサポートする信号生成機能を提供します。
//! CW、AM、FM、PM、パルス、マルチトーン信号の生成が可能です。

use num_complex::Complex;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

/// 変調方式の種類を定義する列挙型
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum ModulationType {
    /// CW (Continuous Wave) - 連続波
    CW,
    /// AM (Amplitude Modulation) - 振幅変調
    AM,
    /// FM (Frequency Modulation) - 周波数変調
    FM,
    /// PM (Phase Modulation) - 位相変調
    PM,
    /// Pulse - パルス変調
    Pulse,
    /// Multitone - マルチトーン信号
    Multitone,
}

/// マルチトーン信号の初期位相設定を定義する列挙型
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum MultitonePhase {
    /// ゼロ位相 - すべてのトーンの初期位相を0に設定
    Zero,
    /// ランダム位相 - 各トーンの初期位相をランダムに設定
    Random,
    /// Schroeder位相 - PAPR（ピーク対平均電力比）を最小化する位相設定
    Schroeder,
}

/// 信号生成に必要なパラメータを保持する構造体
pub struct SignalParams {
    /// 搬送波周波数 (Hz)
    pub frequency: f64,
    /// サンプリングレート (Hz)
    pub sample_rate: f64,
    /// 変調方式
    pub mod_type: ModulationType,
    /// 変調周波数 (Hz) - AM/FM/PM/Pulseで使用
    pub mod_freq: f64,
    /// 変調強度 - AM: 変調指数, FM: 偏移量(Hz), PM: 変調指数(Beta), Pulse: デューティサイクル
    pub mod_strength: f64,
    /// マルチトーンのトーン数
    pub multitone_count: usize,
    /// マルチトーンの周波数間隔 (Hz)
    pub multitone_spacing: f64,
    /// マルチトーンの初期位相設定
    pub multitone_phase: MultitonePhase,
    /// ランダム位相生成用のシード値
    pub seed: u64,
}

/// 信号を生成するジェネレータ構造体
///
/// 内部状態（位相）を保持し、連続的にサンプルを生成できます。
pub struct SignalGenerator {
    /// 搬送波の現在位相 (ラジアン)
    phase: f64,
    /// 変調信号の現在位相 (ラジアン)
    mod_phase: f64,
    /// マルチトーン信号の各トーンの位相 (ラジアン)
    multitone_phases: Vec<f64>,
}

impl SignalGenerator {
    /// 新しいSignalGeneratorインスタンスを生成
    ///
    /// すべての位相を0で初期化します。
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            mod_phase: 0.0,
            multitone_phases: Vec::new(),
        }
    }

    /// 次のサンプルを生成
    ///
    /// 指定されたパラメータに基づいて、複素数形式のI/Qサンプルを1つ生成します。
    /// 内部状態（位相）を更新しながら連続的にサンプルを生成できます。
    ///
    /// # 引数
    /// * `params` - 信号生成パラメータ
    ///
    /// # 戻り値
    /// 複素数形式のI/Qサンプル (I=実部、Q=虚部)
    pub fn next_sample(&mut self, params: &SignalParams) -> Complex<f64> {
        // マルチトーン信号の場合は専用の処理に分岐
        if params.mod_type == ModulationType::Multitone {
            return self.next_multitone_sample(params);
        }

        // 変調信号の位相を更新
        let mod_phase_increment = 2.0 * PI * params.mod_freq / params.sample_rate;
        self.mod_phase += mod_phase_increment;
        if self.mod_phase > 2.0 * PI {
            self.mod_phase -= 2.0 * PI;
        }

        // 現在の周波数と振幅係数を初期化
        let mut current_freq = params.frequency;
        let mut amplitude_factor = 1.0;

        // 変調タイプに応じた処理
        match params.mod_type {
            ModulationType::CW => {
                // CW: 変調なし
            }
            ModulationType::AM => {
                // AM: 振幅を変調
                // A(t) = A₀[1 + m·cos(2πf_m·t)]
                amplitude_factor = 1.0 + params.mod_strength * self.mod_phase.cos();
            }
            ModulationType::FM => {
                // FM: 周波数を変調
                // f(t) = f_c + Δf·cos(2πf_m·t)
                current_freq = params.frequency + params.mod_strength * self.mod_phase.cos();
            }
            ModulationType::PM => {
                // PM: 位相を変調（位相出力時に処理）
            }
            ModulationType::Pulse => {
                // Pulse: デューティサイクルに基づいてON/OFFを切り替え
                if self.mod_phase < params.mod_strength * 2.0 * PI {
                    amplitude_factor = 1.0;
                } else {
                    amplitude_factor = 0.0;
                }
            }
            ModulationType::Multitone => unreachable!(),
        }

        // 搬送波の位相を更新
        let phase_increment = 2.0 * PI * current_freq / params.sample_rate;
        self.phase += phase_increment;
        if self.phase > 2.0 * PI {
            self.phase -= 2.0 * PI;
        }

        // 最終的な位相を計算（PM変調の場合は位相変調を適用）
        let mut final_phase = self.phase;
        if params.mod_type == ModulationType::PM {
            // PM: φ(t) = φ_c + β·cos(2πf_m·t)
            final_phase += params.mod_strength * self.mod_phase.cos();
        }

        // 極座標形式から複素数を生成 (振幅, 位相) -> I+jQ
        Complex::from_polar(amplitude_factor, final_phase)
    }

    /// マルチトーン信号の次のサンプルを生成
    ///
    /// 複数のトーン（正弦波）を合成してマルチトーン信号を生成します。
    /// 初回呼び出し時に、指定された初期位相設定に基づいて各トーンの位相を初期化します。
    ///
    /// # 引数
    /// * `params` - 信号生成パラメータ
    ///
    /// # 戻り値
    /// 複素数形式のI/Qサンプル
    fn next_multitone_sample(&mut self, params: &SignalParams) -> Complex<f64> {
        // 初回呼び出し時または設定変更時に位相を初期化
        if self.multitone_phases.len() != params.multitone_count {
            self.multitone_phases = Vec::with_capacity(params.multitone_count);
            let n = params.multitone_count as f64;

            match params.multitone_phase {
                MultitonePhase::Zero => {
                    // すべての位相を0に設定
                    for _ in 0..params.multitone_count {
                        self.multitone_phases.push(0.0);
                    }
                }
                MultitonePhase::Random => {
                    // シード値を使用してランダムな位相を生成
                    let mut rng = StdRng::seed_from_u64(params.seed);
                    for _ in 0..params.multitone_count {
                        self.multitone_phases.push(rng.random_range(0.0..2.0 * PI));
                    }
                }
                MultitonePhase::Schroeder => {
                    // Schroeder位相：PAPR（ピーク対平均電力比）を最小化
                    // φ_k = -π·k·(k-1)/N
                    for k in 0..params.multitone_count {
                        let k_f = k as f64;
                        let phi = -PI * k_f * (k_f - 1.0) / n;
                        self.multitone_phases.push(phi);
                    }
                }
            }
        }

        // すべてのトーンを合成
        let mut i_sum = 0.0;
        let mut q_sum = 0.0;

        // 中心周波数からのオフセットを計算（中心周波数を基準に対称に配置）
        let center_offset = (params.multitone_count as f64 - 1.0) / 2.0;

        for (k, phase) in self.multitone_phases.iter_mut().enumerate() {
            // 各トーンの周波数を計算
            // f_k = f_center + (k - N/2) * spacing
            let freq_offset = (k as f64 - center_offset) * params.multitone_spacing;
            let tone_freq = params.frequency + freq_offset;

            // 位相を更新
            let phase_inc = 2.0 * PI * tone_freq / params.sample_rate;
            *phase += phase_inc;
            if *phase > 2.0 * PI {
                *phase -= 2.0 * PI;
            }

            // I/Q成分を計算して合成
            let (sin, cos) = phase.sin_cos();
            i_sum += cos;
            q_sum += sin;
        }

        // トーン数で正規化して最大振幅を1.0に調整
        let scale = 1.0 / params.multitone_count as f64;
        Complex::new(i_sum * scale, q_sum * scale)
    }

    /// 指定された数のサンプルをブロックとして生成
    ///
    /// 内部状態を保持しながら連続的にサンプルを生成します。
    ///
    /// # 引数
    /// * `params` - 信号生成パラメータ
    /// * `count` - 生成するサンプル数
    ///
    /// # 戻り値
    /// 複素数形式のI/Qサンプルの配列
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

    /// 周波数の正確性をテスト
    ///
    /// 100Hzの信号を1000Hzでサンプリングした場合、
    /// 10サンプル後（0.01秒後）に位相が2π進むことを確認
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

        // 1秒分のデータを生成
        let samples = gen.generate_block(&params, params.sample_rate as usize);

        let s0 = samples[0];
        let s10 = samples[10];

        // 10サンプル = 1周期なので、同じ値になるはず
        let epsilon = 1e-5;
        assert!((s0.re - s10.re).abs() < epsilon);
        assert!((s0.im - s10.im).abs() < epsilon);
    }
}
