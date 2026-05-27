//! Local Biometric Analyzer — 100% Device-Side Processing.
//!
//! Extracts rPPG (cardiovascular variability), microexpressions (FACS-lite),
//! and voice patterns from local media streams. All processing occurs within
//! the WASM/Edge boundary. Zero telemetry. Zero network I/O.
//!
//! **Privacy Invariant:** Biometric data is processed and discarded locally.
//! Only aggregated metrics (stress_index, coherence, dominant_frequency) are
//! exposed via `BiometricState`.
//!
//! **Feature Gate:** `v3.0-resonance-interface`

use thiserror::Error;

/// Errors specific to biometric analysis.
#[derive(Debug, Error, Clone)]
pub enum AnalyzerError {
    #[error("Stream too short for analysis: {len} samples (minimum {min})")]
    StreamTooShort { len: usize, min: usize },

    #[error("Invalid biometric value: {field} = {value} (expected range [{min}, {max}])")]
    InvalidValue { field: String, value: f32, min: f32, max: f32 },

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Processing failed: {0}")]
    ProcessingFailed(String),

    #[error("Telemetry violation: biometric data must remain LOCAL_ONLY")]
    TelemetryViolation,
}

/// Aggregated biometric state — privacy-preserving summary.
///
/// Contains only derived metrics, never raw biometric data.
/// All values are normalized to [0, 1] range for cooperation.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BiometricState {
    /// Stress index: 0.0 (calm) → 1.0 (high stress)
    pub stress_index: f32,
    /// Physiological coherence: 0.0 (chaotic) → 1.0 (coherent)
    pub coherence: f32,
    /// Dominant frequency in Hz (e.g., heart rate, breathing rate)
    pub dominant_frequency: f32,
    /// Valence: -1.0 (negative) → 1.0 (positive)
    pub valence: f32,
    /// Arousal: 0.0 (low) → 1.0 (high)
    pub arousal: f32,
}

impl BiometricState {
    /// Create a validated BiometricState.
    pub fn new(
        stress_index: f32,
        coherence: f32,
        dominant_frequency: f32,
        valence: f32,
        arousal: f32,
    ) -> Result<Self, AnalyzerError> {
        if !(0.0..=1.0).contains(&stress_index) {
            return Err(AnalyzerError::InvalidValue {
                field: "stress_index".into(),
                value: stress_index,
                min: 0.0,
                max: 1.0,
            });
        }
        if !(0.0..=1.0).contains(&coherence) {
            return Err(AnalyzerError::InvalidValue {
                field: "coherence".into(),
                value: coherence,
                min: 0.0,
                max: 1.0,
            });
        }
        if !(0.0..=5.0).contains(&dominant_frequency) {
            return Err(AnalyzerError::InvalidValue {
                field: "dominant_frequency".into(),
                value: dominant_frequency,
                min: 0.0,
                max: 5.0,
            });
        }
        if !(-1.0..=1.0).contains(&valence) {
            return Err(AnalyzerError::InvalidValue {
                field: "valence".into(),
                value: valence,
                min: -1.0,
                max: 1.0,
            });
        }
        if !(0.0..=1.0).contains(&arousal) {
            return Err(AnalyzerError::InvalidValue {
                field: "arousal".into(),
                value: arousal,
                min: 0.0,
                max: 1.0,
            });
        }
        Ok(Self {
            stress_index,
            coherence,
            dominant_frequency,
            valence,
            arousal,
        })
    }

    /// Compute homeostasis score: higher = more balanced.
    pub fn homeostasis_score(&self) -> f32 {
        // Homeostasis = coherence * (1 - stress) * (1 - |valence|) * (1 - arousal_deviation)
        let valence_balance = 1.0 - self.valence.abs();
        let arousal_balance = 1.0 - (self.arousal - 0.5).abs() * 2.0;
        (self.coherence * (1.0 - self.stress_index) * valence_balance * arousal_balance).clamp(0.0, 1.0)
    }
}

/// Configuration for the local biometric analyzer.
#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    /// Minimum samples required for rPPG analysis.
    pub min_rppg_samples: usize,
    /// Minimum samples required for voice analysis.
    pub min_voice_samples: usize,
    /// Minimum samples required for expression analysis.
    pub min_expression_samples: usize,
    /// rPPG bandpass filter low cutoff (Hz).
    pub rppg_low_hz: f32,
    /// rPPG bandpass filter high cutoff (Hz).
    pub rppg_high_hz: f32,
    /// Weight for emotional component in homeostasis fusion.
    pub emotional_weight: f32,
    /// Weight for cardiovascular component in homeostasis fusion.
    pub cardiovascular_weight: f32,
    /// Weight for vocal component in homeostasis fusion.
    pub vocal_weight: f32,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            min_rppg_samples: 256,
            min_voice_samples: 128,
            min_expression_samples: 16,
            rppg_low_hz: 0.7,
            rppg_high_hz: 2.5,
            emotional_weight: 0.4,
            cardiovascular_weight: 0.4,
            vocal_weight: 0.2,
        }
    }
}

/// Local Biometric Analyzer — processes biometric data 100% on-device.
///
/// **⚠️ LOCAL_ONLY:** All processing occurs within the WASM/Edge runtime.
/// Biometric data is never serialized for network transmission.
pub struct LocalBiometricAnalyzer {
    config: AnalyzerConfig,
    /// Circular buffer for rPPG (green channel) samples.
    rppg_buffer: Vec<f32>,
    /// Feature vector for voice analysis (pitch, jitter, shimmer).
    voice_features: Vec<f32>,
    /// Weight vector for FACS Action Units (AU1-AU12).
    expression_weights: Vec<f32>,
}

impl LocalBiometricAnalyzer {
    /// Create a new analyzer with default configuration.
    pub fn new() -> Self {
        Self::with_config(AnalyzerConfig::default())
    }

    /// Create with custom configuration.
    pub fn with_config(config: AnalyzerConfig) -> Self {
        Self {
            config,
            rppg_buffer: Vec::new(),
            voice_features: Vec::new(),
            expression_weights: Vec::new(),
        }
    }

    /// Analyze a simulated biometric stream (WASM-compatible, no external deps).
    ///
    /// In production, this receives `MediaStream` from browser WebAPI.
    /// For WASM compatibility, we accept raw sample buffers.
    pub fn analyze_stream(
        &mut self,
        rppg_samples: &[f32],
        voice_samples: &[f32],
        expression_samples: &[f32],
    ) -> Result<BiometricState, AnalyzerError> {
        // Validate minimum sample counts
        if rppg_samples.len() < self.config.min_rppg_samples {
            return Err(AnalyzerError::StreamTooShort {
                len: rppg_samples.len(),
                min: self.config.min_rppg_samples,
            });
        }
        if voice_samples.len() < self.config.min_voice_samples {
            return Err(AnalyzerError::StreamTooShort {
                len: voice_samples.len(),
                min: self.config.min_voice_samples,
            });
        }
        if expression_samples.len() < self.config.min_expression_samples {
            return Err(AnalyzerError::StreamTooShort {
                len: expression_samples.len(),
                min: self.config.min_expression_samples,
            });
        }

        // Process rPPG → cardiovascular metrics
        let (bpm, hrv, stress) = self.process_rppg(rppg_samples)?;

        // Process voice → vocal metrics
        let (_pitch, jitter, shimmer) = self.process_voice(voice_samples)?;

        // Process expressions → emotional metrics
        let (valence, arousal) = self.process_expressions(expression_samples)?;

        // Compute coherence from HRV and vocal stability
        let vocal_stability = 1.0 - (jitter + shimmer) / 2.0;
        let coherence = (hrv * 0.6 + vocal_stability * 0.4).clamp(0.0, 1.0);

        // Dominant frequency from BPM (Hz = BPM / 60)
        let dominant_frequency = bpm / 60.0;

        // Build BiometricState
        BiometricState::new(stress, coherence, dominant_frequency, valence, arousal)
    }

    /// Process rPPG samples → (BPM, HRV, stress_index).
    fn process_rppg(&self, samples: &[f32]) -> Result<(f32, f32, f32), AnalyzerError> {
        // Simulated rPPG processing:
        // 1. Bandpass filter (0.7-2.5 Hz) — isolate cardiac signal
        // 2. Peak detection — estimate BPM
        // 3. HRV from inter-beat intervals
        // 4. Stress index from HRV (low HRV = high stress)

        // Simple peak detection: count sign changes in first derivative
        let mut peaks = 0;
        for window in samples.windows(3) {
            if window[1] > window[0] && window[1] > window[2] {
                peaks += 1;
            }
        }

        // Estimate BPM: assume 30Hz sample rate, 10-second window
        let sample_rate = 30.0;
        let duration = samples.len() as f32 / sample_rate;
        let bpm = if duration > 0.0 {
            (peaks as f32 / duration) * 60.0
        } else {
            0.0
        };

        // Clamp BPM to physiological range (40-180)
        let bpm = bpm.clamp(40.0, 180.0);

        // HRV: coefficient of variation of inter-peak intervals
        // Higher HRV = better coherence
        let hrv = if peaks > 1 {
            let interval = samples.len() as f32 / peaks as f32;
            // Simulated HRV based on signal variance
            let variance = samples.iter()
                .map(|s| {
                    let mean = samples.iter().sum::<f32>() / samples.len() as f32;
                    (s - mean) * (s - mean)
                })
                .sum::<f32>()
                / samples.len() as f32;
            (variance.sqrt() / interval).min(1.0)
        } else {
            0.0
        };

        // Stress index: inverse of HRV, adjusted for BPM deviation from 60
        let bpm_deviation = (bpm - 60.0).abs() / 60.0;
        let stress = (1.0 - hrv + bpm_deviation) / 2.0;
        let stress = stress.clamp(0.0, 1.0);

        Ok((bpm, hrv, stress))
    }

    /// Process voice samples → (pitch, jitter, shimmer).
    fn process_voice(&self, samples: &[f32]) -> Result<(f32, f32, f32), AnalyzerError> {
        // Simulated voice analysis:
        // 1. Pitch estimation (zero-crossing rate)
        // 2. Jitter (cycle-to-cycle pitch variation)
        // 3. Shimmer (cycle-to-cycle amplitude variation)

        // Zero-crossing rate for pitch estimation
        let mut crossings = 0;
        for window in samples.windows(2) {
            if (window[0] >= 0.0) != (window[1] >= 0.0) {
                crossings += 1;
            }
        }
        let zcr = crossings as f32 / samples.len() as f32;
        let pitch = (zcr * 2.0).min(1.0);

        // Jitter: variation in zero-crossing intervals
        let mut intervals = Vec::new();
        let mut last_crossing = 0;
        for (i, window) in samples.windows(2).enumerate() {
            if (window[0] >= 0.0) != (window[1] >= 0.0) {
                intervals.push((i - last_crossing) as f32);
                last_crossing = i;
            }
        }
        let jitter = if intervals.len() > 1 {
            let mean_interval = intervals.iter().sum::<f32>() / intervals.len() as f32;
            let variance = intervals.iter()
                .map(|&x| (x - mean_interval) * (x - mean_interval))
                .sum::<f32>()
                / intervals.len() as f32;
            (variance.sqrt() / mean_interval).min(1.0)
        } else {
            0.0
        };

        // Shimmer: amplitude variation
        let mut amplitudes = Vec::new();
        for window in samples.chunks(10) {
            let max_amp = window.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
            amplitudes.push(max_amp);
        }
        let shimmer = if amplitudes.len() > 1 {
            let mean_amp = amplitudes.iter().sum::<f32>() / amplitudes.len() as f32;
            if mean_amp > 0.0 {
                let variance = amplitudes.iter()
                    .map(|&x| (x - mean_amp) * (x - mean_amp))
                    .sum::<f32>()
                    / amplitudes.len() as f32;
                (variance.sqrt() / mean_amp).min(1.0)
            } else {
                0.0
            }
        } else {
            0.0
        };

        Ok((pitch, jitter, shimmer))
    }

    /// Process expression samples → (valence, arousal).
    fn process_expressions(&self, samples: &[f32]) -> Result<(f32, f32), AnalyzerError> {
        // Simulated FACS-lite analysis:
        // Samples represent AU (Action Unit) intensities [AU1..AU12]
        // AU1 (inner brow), AU2 (outer brow), AU4 (brow lowerer)
        // AU6 (cheek raiser), AU12 (lip corner puller)

        let mean = samples.iter().sum::<f32>() / samples.len() as f32;
        let variance = samples.iter()
            .map(|s| (s - mean) * (s - mean))
            .sum::<f32>()
            / samples.len() as f32;

        // Valence: positive AUs (AU6, AU12) vs negative AUs (AU1, AU4)
        let positive_au = samples.get(5).copied().unwrap_or(0.0)
            + samples.get(11).copied().unwrap_or(0.0);
        let negative_au = samples.first().copied().unwrap_or(0.0)
            + samples.get(3).copied().unwrap_or(0.0);
        let valence = if positive_au + negative_au > 0.0 {
            (positive_au - negative_au) / (positive_au + negative_au)
        } else {
            0.0
        };
        let valence = valence.clamp(-1.0, 1.0);

        // Arousal: overall activation level
        let arousal = (mean + variance.sqrt()) / 2.0;
        let arousal = arousal.clamp(0.0, 1.0);

        Ok((valence, arousal))
    }

    /// Get the current configuration.
    pub fn config(&self) -> &AnalyzerConfig {
        &self.config
    }

    /// Clear all buffers (privacy: discard biometric data after processing).
    pub fn clear_buffers(&mut self) {
        self.rppg_buffer.clear();
        self.voice_features.clear();
        self.expression_weights.clear();
    }
}

impl Default for LocalBiometricAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rppg_samples(len: usize) -> Vec<f32> {
        // Simulated cardiac signal: sinusoidal at ~1.2 Hz (72 BPM)
        (0..len)
            .map(|i| {
                let t = i as f32 / 30.0; // 30 Hz sample rate
                (t * 1.2 * 2.0 * std::f32::consts::PI).sin() * 0.5
                    + (t * 0.3 * 2.0 * std::f32::consts::PI).sin() * 0.1 // noise
            })
            .collect()
    }

    fn make_voice_samples(len: usize) -> Vec<f32> {
        // Simulated voice: mixed frequencies
        (0..len)
            .map(|i| {
                let t = i as f32 / 16000.0; // 16 kHz
                (t * 150.0 * 2.0 * std::f32::consts::PI).sin() * 0.3
                    + (t * 300.0 * 2.0 * std::f32::consts::PI).sin() * 0.1
            })
            .collect()
    }

    fn make_expression_samples() -> Vec<f32> {
        // Simulated AU intensities [AU1..AU12]
        vec![0.1, 0.2, 0.0, 0.1, 0.3, 0.8, 0.2, 0.1, 0.0, 0.1, 0.2, 0.9, 0.05, 0.15, 0.1, 0.7]
    }

    #[test]
    fn test_analyzer_creation() {
        let analyzer = LocalBiometricAnalyzer::new();
        assert_eq!(analyzer.config().min_rppg_samples, 256);
    }

    #[test]
    fn test_analyzer_custom_config() {
        let config = AnalyzerConfig {
            min_rppg_samples: 512,
            ..AnalyzerConfig::default()
        };
        let analyzer = LocalBiometricAnalyzer::with_config(config);
        assert_eq!(analyzer.config().min_rppg_samples, 512);
    }

    #[test]
    fn test_analyze_valid_stream() {
        let mut analyzer = LocalBiometricAnalyzer::new();
        let rppg = make_rppg_samples(300);
        let voice = make_voice_samples(200);
        let expr = make_expression_samples();

        let state = analyzer.analyze_stream(&rppg, &voice, &expr).unwrap();
        assert!(state.stress_index >= 0.0 && state.stress_index <= 1.0);
        assert!(state.coherence >= 0.0 && state.coherence <= 1.0);
        assert!(state.dominant_frequency >= 0.0);
        assert!(state.valence >= -1.0 && state.valence <= 1.0);
        assert!(state.arousal >= 0.0 && state.arousal <= 1.0);
    }

    #[test]
    fn test_analyze_short_rppg_rejected() {
        let mut analyzer = LocalBiometricAnalyzer::new();
        let rppg = make_rppg_samples(100);
        let voice = make_voice_samples(200);
        let expr = make_expression_samples();

        match analyzer.analyze_stream(&rppg, &voice, &expr) {
            Err(AnalyzerError::StreamTooShort { .. }) => {},
            other => panic!("Expected StreamTooShort, got {:?}", other),
        }
    }

    #[test]
    fn test_analyze_short_voice_rejected() {
        let mut analyzer = LocalBiometricAnalyzer::new();
        let rppg = make_rppg_samples(300);
        let voice = make_voice_samples(50);
        let expr = make_expression_samples();

        match analyzer.analyze_stream(&rppg, &voice, &expr) {
            Err(AnalyzerError::StreamTooShort { .. }) => {},
            other => panic!("Expected StreamTooShort, got {:?}", other),
        }
    }

    #[test]
    fn test_analyze_short_expression_rejected() {
        let mut analyzer = LocalBiometricAnalyzer::new();
        let rppg = make_rppg_samples(300);
        let voice = make_voice_samples(200);
        let expr = vec![0.1, 0.2];

        match analyzer.analyze_stream(&rppg, &voice, &expr) {
            Err(AnalyzerError::StreamTooShort { .. }) => {},
            other => panic!("Expected StreamTooShort, got {:?}", other),
        }
    }

    #[test]
    fn test_biometric_state_homeostasis() {
        let calm = BiometricState::new(0.1, 0.9, 1.0, 0.3, 0.4).unwrap();
        let stressed = BiometricState::new(0.9, 0.1, 2.0, -0.8, 0.9).unwrap();
        assert!(calm.homeostasis_score() > stressed.homeostasis_score());
    }

    #[test]
    fn test_biometric_state_invalid_stress() {
        match BiometricState::new(1.5, 0.5, 1.0, 0.0, 0.5) {
            Err(AnalyzerError::InvalidValue { field, .. }) => {
                assert_eq!(field, "stress_index");
            },
            other => panic!("Expected InvalidValue, got {:?}", other),
        }
    }

    #[test]
    fn test_biometric_state_invalid_valence() {
        match BiometricState::new(0.5, 0.5, 1.0, 1.5, 0.5) {
            Err(AnalyzerError::InvalidValue { field, .. }) => {
                assert_eq!(field, "valence");
            },
            other => panic!("Expected InvalidValue, got {:?}", other),
        }
    }

    #[test]
    fn test_clear_buffers() {
        let mut analyzer = LocalBiometricAnalyzer::new();
        analyzer.rppg_buffer = vec![1.0, 2.0, 3.0];
        analyzer.voice_features = vec![0.5];
        analyzer.expression_weights = vec![0.1];
        analyzer.clear_buffers();
        assert!(analyzer.rppg_buffer.is_empty());
        assert!(analyzer.voice_features.is_empty());
        assert!(analyzer.expression_weights.is_empty());
    }

    #[test]
    fn test_rppg_processing() {
        let analyzer = LocalBiometricAnalyzer::new();
        let samples = make_rppg_samples(300);
        let (bpm, hrv, stress) = analyzer.process_rppg(&samples).unwrap();
        assert!(bpm >= 40.0 && bpm <= 180.0);
        assert!(hrv >= 0.0 && hrv <= 1.0);
        assert!(stress >= 0.0 && stress <= 1.0);
    }

    #[test]
    fn test_voice_processing() {
        let analyzer = LocalBiometricAnalyzer::new();
        let samples = make_voice_samples(200);
        let (pitch, jitter, shimmer) = analyzer.process_voice(&samples).unwrap();
        assert!(pitch >= 0.0 && pitch <= 1.0);
        assert!(jitter >= 0.0);
        assert!(shimmer >= 0.0);
    }

    #[test]
    fn test_expression_processing() {
        let analyzer = LocalBiometricAnalyzer::new();
        let samples = make_expression_samples();
        let (valence, arousal) = analyzer.process_expressions(&samples).unwrap();
        assert!(valence >= -1.0 && valence <= 1.0);
        assert!(arousal >= 0.0 && arousal <= 1.0);
    }

    #[test]
    fn test_positive_expression_valence() {
        let analyzer = LocalBiometricAnalyzer::new();
        // High positive AUs (AU6=0.9, AU12=0.9), low negative AUs
        let samples = vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.9, 0.0, 0.0, 0.0, 0.0, 0.0, 0.9];
        let (valence, _arousal) = analyzer.process_expressions(&samples).unwrap();
        assert!(valence > 0.0, "Positive expressions should yield positive valence");
    }

    #[test]
    fn test_negative_expression_valence() {
        let analyzer = LocalBiometricAnalyzer::new();
        // High negative AUs (AU1=0.9, AU4=0.9), no positive AUs
        let samples = vec![0.9, 0.0, 0.0, 0.9, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let (valence, _arousal) = analyzer.process_expressions(&samples).unwrap();
        assert!(valence < 0.0, "Negative expressions should yield negative valence");
    }

    #[test]
    fn test_error_display() {
        let e = AnalyzerError::StreamTooShort { len: 10, min: 100 };
        assert!(e.to_string().contains("Stream too short"));
        let e = AnalyzerError::TelemetryViolation;
        assert!(e.to_string().contains("LOCAL_ONLY"));
    }

    #[test]
    fn test_default() {
        let analyzer = LocalBiometricAnalyzer::default();
        assert_eq!(analyzer.config().min_rppg_samples, 256);
    }
}
