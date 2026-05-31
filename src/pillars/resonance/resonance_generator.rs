//! Resonance Generator — Morphic Resonance Response Synthesis.
//!
//! Generates binaural beats, isochronic tones, and SCT-validated semantic responses
//! for homeostasis induction. All synthesis occurs 100% locally via WASM/Edge.
//!
//! **Constructive Directive:** All semantic responses use constructive, cooperative language.
//! Prohibited words: diplomacia, vencer, atacar, revolución, destruir, enemigo, guerra, dominar, esconderse, evadir.
//!
//! **Feature Gate:** `v3.0-resonance-interface`

use crate::pillars::resonance::biometric_analyzer::BiometricState;
use crate::pillars::resonance::homeostasis_engine::{EngineError, HomeostasisDelta};
use thiserror::Error;

/// Errors specific to resonance generation.
#[derive(Debug, Error, Clone)]
pub enum ResonanceError {
    #[error("Invalid frequency: {freq_hz:.1} Hz (expected range [{min_hz}, {max_hz}])")]
    InvalidFrequency {
        freq_hz: f32,
        min_hz: f32,
        max_hz: f32,
    },

    #[error("Invalid duration: {duration_s:.2}s (expected range [{min_s}, {max_s}])")]
    InvalidDuration {
        duration_s: f32,
        min_s: f32,
        max_s: f32,
    },

    #[error("Invalid amplitude: {amp} (expected range [{min}, {max}])")]
    InvalidAmplitude { amp: f32, min: f32, max: f32 },

    #[error("SCT Guard rejection: Z = {z:.3} — resonance response ethically invalid")]
    SctRejection { z: f32 },

    #[error("Homeostasis delta required for resonance generation")]
    MissingDelta,

    #[error("Telemetry violation: resonance data must remain LOCAL_ONLY")]
    TelemetryViolation,

    #[error("Homeostasis engine error: {0}")]
    Homeostasis(#[from] EngineError),
}

/// Binaural beat configuration.
///
/// Binaural beats are created by presenting two slightly different frequencies
/// to each ear. The brain perceives a third frequency equal to the difference.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BinauralBeat {
    /// Base frequency for left ear (Hz).
    pub left_freq_hz: f32,
    /// Base frequency for right ear (Hz).
    pub right_freq_hz: f32,
    /// Perceived beat frequency (left - right, Hz).
    pub beat_freq_hz: f32,
    /// Duration in seconds.
    pub duration_s: f32,
    /// Amplitude (0.0 = silent, 1.0 = full).
    pub amplitude: f32,
}

impl BinauralBeat {
    /// Create a validated binaural beat.
    pub fn new(
        base_freq_hz: f32,
        beat_freq_hz: f32,
        duration_s: f32,
        amplitude: f32,
    ) -> Result<Self, ResonanceError> {
        const MIN_FREQ: f32 = 20.0;
        const MAX_FREQ: f32 = 20000.0;
        const MIN_DURATION: f32 = 0.1;
        const MAX_DURATION: f32 = 300.0;

        if !(MIN_FREQ..=MAX_FREQ).contains(&base_freq_hz) {
            return Err(ResonanceError::InvalidFrequency {
                freq_hz: base_freq_hz,
                min_hz: MIN_FREQ,
                max_hz: MAX_FREQ,
            });
        }
        if !(0.5..=40.0).contains(&beat_freq_hz) {
            return Err(ResonanceError::InvalidFrequency {
                freq_hz: beat_freq_hz,
                min_hz: 0.5,
                max_hz: 40.0,
            });
        }
        if !(MIN_DURATION..=MAX_DURATION).contains(&duration_s) {
            return Err(ResonanceError::InvalidDuration {
                duration_s,
                min_s: MIN_DURATION,
                max_s: MAX_DURATION,
            });
        }
        if !(0.0..=1.0).contains(&amplitude) {
            return Err(ResonanceError::InvalidAmplitude {
                amp: amplitude,
                min: 0.0,
                max: 1.0,
            });
        }

        let left_freq = base_freq_hz;
        let right_freq = base_freq_hz - beat_freq_hz;

        Ok(Self {
            left_freq_hz: left_freq,
            right_freq_hz: right_freq,
            beat_freq_hz,
            duration_s,
            amplitude,
        })
    }

    /// Determine the brainwave band for this beat frequency.
    pub fn brainwave_band(&self) -> &'static str {
        if self.beat_freq_hz < 4.0 {
            "delta" // Deep sleep, healing
        } else if self.beat_freq_hz < 8.0 {
            "theta" // Meditation, creativity
        } else if self.beat_freq_hz < 14.0 {
            "alpha" // Relaxation, calm focus
        } else if self.beat_freq_hz < 30.0 {
            "beta" // Active thinking, alertness
        } else {
            "gamma" // High-level cognition, insight
        }
    }
}

/// Isochronic tone configuration.
///
/// Isochronic tones are single tones that turn on and off rapidly,
/// creating a distinct pulsing effect.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IsochronicTone {
    /// Base frequency (Hz).
    pub base_freq_hz: f32,
    /// Pulse rate (Hz).
    pub pulse_rate_hz: f32,
    /// Duration in seconds.
    pub duration_s: f32,
    /// Amplitude (0.0 = silent, 1.0 = full).
    pub amplitude: f32,
}

impl IsochronicTone {
    /// Create a validated isochronic tone.
    pub fn new(
        base_freq_hz: f32,
        pulse_rate_hz: f32,
        duration_s: f32,
        amplitude: f32,
    ) -> Result<Self, ResonanceError> {
        const MIN_FREQ: f32 = 100.0;
        const MAX_FREQ: f32 = 5000.0;

        if !(MIN_FREQ..=MAX_FREQ).contains(&base_freq_hz) {
            return Err(ResonanceError::InvalidFrequency {
                freq_hz: base_freq_hz,
                min_hz: MIN_FREQ,
                max_hz: MAX_FREQ,
            });
        }
        if !(0.5..=40.0).contains(&pulse_rate_hz) {
            return Err(ResonanceError::InvalidFrequency {
                freq_hz: pulse_rate_hz,
                min_hz: 0.5,
                max_hz: 40.0,
            });
        }

        Ok(Self {
            base_freq_hz,
            pulse_rate_hz,
            duration_s,
            amplitude,
        })
    }

    /// Determine the brainwave band for this pulse rate.
    pub fn brainwave_band(&self) -> &'static str {
        if self.pulse_rate_hz < 4.0 {
            "delta"
        } else if self.pulse_rate_hz < 8.0 {
            "theta"
        } else if self.pulse_rate_hz < 14.0 {
            "alpha"
        } else if self.pulse_rate_hz < 30.0 {
            "beta"
        } else {
            "gamma"
        }
    }
}

/// SCT-validated semantic response for resonance.
///
/// Contains constructive, cooperative language only.
/// All responses are validated by SCT Guard (Z >= 0).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SemanticResponse {
    /// Response text in constructive language.
    pub text: String,
    /// SCT Z-axis value (>= 0 = approved).
    pub sct_z: f32,
    /// Recommended brainwave band for this response.
    pub brainwave_band: String,
    /// Confidence score (0.0 = uncertain, 1.0 = confident).
    pub confidence: f32,
}

impl SemanticResponse {
    /// Create a validated semantic response.
    pub fn new(
        text: String,
        sct_z: f32,
        brainwave_band: String,
        confidence: f32,
    ) -> Result<Self, ResonanceError> {
        if sct_z < 0.0 {
            return Err(ResonanceError::SctRejection { z: sct_z });
        }
        if !(0.0..=1.0).contains(&confidence) {
            return Err(ResonanceError::InvalidAmplitude {
                amp: confidence,
                min: 0.0,
                max: 1.0,
            });
        }

        // Prohibited word check
        let prohibited = [
            "diplomacia",
            "vencer",
            "atacar",
            "revolución",
            "destruir",
            "enemigo",
            "guerra",
            "dominar",
            "esconderse",
            "evadir",
        ];
        let text_lower = text.to_lowercase();
        for word in &prohibited {
            if text_lower.contains(word) {
                return Err(ResonanceError::SctRejection { z: -1.0 });
            }
        }

        Ok(Self {
            text,
            sct_z,
            brainwave_band,
            confidence,
        })
    }
}

/// Complete resonance response package.
///
/// Combines audio (binaural/isochronic) and semantic components
/// for holistic homeostasis induction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResonanceResponse {
    /// Binaural beat component (optional).
    pub binaural: Option<BinauralBeat>,
    /// Isochronic tone component (optional).
    pub isochronic: Option<IsochronicTone>,
    /// Semantic response component.
    pub semantic: SemanticResponse,
    /// Overall homeostasis target score.
    pub homeostasis_target: f32,
    /// SCT Z-axis validation score.
    pub sct_z: f32,
}

impl ResonanceResponse {
    /// Check if this response is ethically approved.
    pub fn is_approved(&self) -> bool {
        self.sct_z >= 0.0 && self.semantic.sct_z >= 0.0
    }
}

/// Configuration for the Resonance Generator.
#[derive(Debug, Clone)]
pub struct ResonanceConfig {
    /// Base frequency for binaural beats (Hz).
    pub binaural_base_freq: f32,
    /// Base frequency for isochronic tones (Hz).
    pub isochronic_base_freq: f32,
    /// Default duration for audio components (seconds).
    pub default_duration_s: f32,
    /// Default amplitude for audio components.
    pub default_amplitude: f32,
    /// SCT Z-axis threshold for ethical approval.
    pub sct_z_threshold: f32,
    /// Enable binaural beats.
    pub enable_binaural: bool,
    /// Enable isochronic tones.
    pub enable_isochronic: bool,
    /// Enable semantic responses.
    pub enable_semantic: bool,
}

impl Default for ResonanceConfig {
    fn default() -> Self {
        Self {
            binaural_base_freq: 200.0,
            isochronic_base_freq: 200.0,
            default_duration_s: 60.0,
            default_amplitude: 0.7,
            sct_z_threshold: 0.0,
            enable_binaural: true,
            enable_isochronic: true,
            enable_semantic: true,
        }
    }
}

/// Resonance Generator — synthesizes morphic resonance responses 100% locally.
///
/// Maps homeostasis deltas to audio frequencies and semantic responses
/// for homeostasis induction. All outputs are SCT-validated.
///
/// **⚠️ LOCAL_ONLY:** All resonance data processed and delivered locally.
pub struct ResonanceGenerator {
    config: ResonanceConfig,
}

impl ResonanceGenerator {
    /// Create a new generator with default configuration.
    pub fn new() -> Self {
        Self::with_config(ResonanceConfig::default())
    }

    /// Create with custom configuration.
    pub fn with_config(config: ResonanceConfig) -> Self {
        Self { config }
    }

    /// Generate a complete resonance response from homeostasis delta.
    ///
    /// Maps physiological deviation to appropriate frequencies and semantic responses.
    pub fn generate_response(
        &self,
        delta: &HomeostasisDelta,
        current_state: &BiometricState,
    ) -> Result<ResonanceResponse, ResonanceError> {
        if !delta.is_ethically_approved() {
            return Err(ResonanceError::SctRejection { z: delta.sct_z });
        }

        // Determine target brainwave band based on homeostasis needs
        let target_band = self.select_brainwave_band(delta, current_state);
        let target_freq = self.band_to_frequency(&target_band);

        // Generate binaural component
        let binaural = if self.config.enable_binaural {
            Some(BinauralBeat::new(
                self.config.binaural_base_freq,
                target_freq,
                self.config.default_duration_s,
                self.config.default_amplitude,
            )?)
        } else {
            None
        };

        // Generate isochronic component
        let isochronic = if self.config.enable_isochronic {
            Some(IsochronicTone::new(
                self.config.isochronic_base_freq,
                target_freq,
                self.config.default_duration_s,
                self.config.default_amplitude,
            )?)
        } else {
            None
        };

        // Generate semantic response
        let semantic = if self.config.enable_semantic {
            self.generate_semantic(delta, current_state, &target_band)?
        } else {
            SemanticResponse::new(
                "Resonancia activa.".to_string(),
                delta.sct_z,
                target_band.clone(),
                0.8,
            )?
        };

        Ok(ResonanceResponse {
            binaural,
            isochronic,
            semantic,
            homeostasis_target: self.config.default_amplitude,
            sct_z: delta.sct_z,
        })
    }

    /// Select the optimal brainwave band based on homeostasis needs.
    fn select_brainwave_band(&self, _delta: &HomeostasisDelta, state: &BiometricState) -> String {
        // High stress + low coherence → Alpha (relaxation)
        if state.stress_index > 0.6 && state.coherence < 0.4 {
            return "alpha".to_string();
        }

        // Very high stress → Theta (deep relaxation)
        if state.stress_index > 0.8 {
            return "theta".to_string();
        }

        // Low arousal → Beta (alertness)
        if state.arousal < 0.3 {
            return "beta".to_string();
        }

        // Negative valence → Alpha (calm positivity)
        if state.valence < -0.3 {
            return "alpha".to_string();
        }

        // High coherence + low stress → Gamma (insight)
        if state.coherence > 0.8 && state.stress_index < 0.2 {
            return "gamma".to_string();
        }

        // Default: Alpha for general relaxation
        "alpha".to_string()
    }

    /// Convert brainwave band name to target frequency (Hz).
    fn band_to_frequency(&self, band: &str) -> f32 {
        match band {
            "delta" => 2.0,  // Deep sleep
            "theta" => 6.0,  // Meditation
            "alpha" => 10.0, // Relaxation
            "beta" => 20.0,  // Alertness
            "gamma" => 40.0, // Insight
            _ => 10.0,       // Default alpha
        }
    }

    /// Generate SCT-validated semantic response.
    fn generate_semantic(
        &self,
        delta: &HomeostasisDelta,
        state: &BiometricState,
        band: &str,
    ) -> Result<SemanticResponse, ResonanceError> {
        let text = self.construct_semantic_text(delta, state, band);
        let confidence = self.calculate_confidence(delta, state);

        SemanticResponse::new(text, delta.sct_z, band.to_string(), confidence)
    }

    /// Construct constructive semantic text based on biometric state.
    fn construct_semantic_text(
        &self,
        _delta: &HomeostasisDelta,
        state: &BiometricState,
        band: &str,
    ) -> String {
        // Build response based on current state needs
        let mut parts = Vec::new();

        // Address stress level
        if state.stress_index > 0.6 {
            parts.push("Detecto patrones de tensión elevada. La frecuencia actual apoya la restauración del equilibrio interno.");
        } else if state.stress_index < 0.3 {
            parts.push("Los patrones fisiológicos muestran un estado de calma constructiva.");
        }

        // Address coherence
        if state.coherence < 0.4 {
            parts.push("La coherencia cardiovascular se encuentra en fase de integración. La resonancia facilita la sincronización.");
        } else if state.coherence > 0.7 {
            parts.push("La coherencia actual refleja una integración fisiológica sólida.");
        }

        // Address valence
        if state.valence < -0.3 {
            parts.push(
                "La resonancia actual promueve la transformación hacia estados de mayor bienestar.",
            );
        } else if state.valence > 0.3 {
            parts.push("Los indicadores emocionales muestran una tendencia constructiva.");
        }

        // Add band-specific guidance
        match band {
            "alpha" => parts.push("Ondas alfa para relajación y enfoque sereno."),
            "theta" => parts.push("Ondas theta para relajación profunda y renovación interna."),
            "beta" => parts.push("Ondas beta para claridad mental y atención activa."),
            "gamma" => parts.push("Ondas gamma para integración cognitiva y percepción ampliada."),
            "delta" => parts.push("Ondas delta para restauración profunda y recuperación."),
            _ => parts.push("Frecuencia de resonancia activa."),
        }

        parts.join(" ")
    }

    /// Calculate confidence score based on delta quality.
    fn calculate_confidence(&self, delta: &HomeostasisDelta, state: &BiometricState) -> f32 {
        // Higher confidence when SCT Z is high and state is coherent
        let sct_factor = (delta.sct_z + 1.0) / 2.0; // Normalize [-1, 1] to [0, 1]
        let coherence_factor = state.coherence;
        let magnitude_factor = 1.0 - delta.deviation_magnitude();

        (sct_factor * 0.4 + coherence_factor * 0.3 + magnitude_factor * 0.3).clamp(0.1, 1.0)
    }

    /// Get current configuration.
    pub fn config(&self) -> &ResonanceConfig {
        &self.config
    }
}

impl Default for ResonanceGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_calm_state() -> BiometricState {
        BiometricState::new(0.1, 0.9, 1.0, 0.5, 0.3).unwrap()
    }

    fn make_stressed_state() -> BiometricState {
        BiometricState::new(0.8, 0.2, 2.5, -0.3, 0.9).unwrap()
    }

    fn make_positive_delta() -> HomeostasisDelta {
        HomeostasisDelta {
            stress_delta: -0.2,
            coherence_delta: 0.3,
            frequency_delta: 0.1,
            valence_delta: 0.4,
            arousal_delta: -0.1,
            homeostasis_score: 0.8,
            sct_z: 0.5,
            correction_magnitude: 0.2,
        }
    }

    fn make_negative_delta() -> HomeostasisDelta {
        HomeostasisDelta {
            stress_delta: 0.5,
            coherence_delta: -0.4,
            frequency_delta: 0.3,
            valence_delta: -0.3,
            arousal_delta: 0.2,
            homeostasis_score: 0.2,
            sct_z: -0.3,
            correction_magnitude: 0.7,
        }
    }

    #[test]
    fn test_generator_creation() {
        let gen = ResonanceGenerator::new();
        assert!(gen.config().enable_binaural);
        assert!(gen.config().enable_isochronic);
        assert!(gen.config().enable_semantic);
    }

    #[test]
    fn test_generator_custom_config() {
        let config = ResonanceConfig {
            enable_binaural: false,
            enable_isochronic: false,
            enable_semantic: true,
            ..Default::default()
        };
        let gen = ResonanceGenerator::with_config(config);
        assert!(!gen.config().enable_binaural);
        assert!(gen.config().enable_semantic);
    }

    #[test]
    fn test_binaural_creation() {
        let beat = BinauralBeat::new(200.0, 10.0, 60.0, 0.7).unwrap();
        assert_eq!(beat.left_freq_hz, 200.0);
        assert_eq!(beat.right_freq_hz, 190.0);
        assert_eq!(beat.beat_freq_hz, 10.0);
        assert_eq!(beat.brainwave_band(), "alpha");
    }

    #[test]
    fn test_binaural_invalid_base_freq() {
        match BinauralBeat::new(10.0, 10.0, 60.0, 0.7) {
            Err(ResonanceError::InvalidFrequency { freq_hz, .. }) => assert_eq!(freq_hz, 10.0),
            _ => panic!("Expected InvalidFrequency"),
        }
    }

    #[test]
    fn test_binaural_invalid_duration() {
        match BinauralBeat::new(200.0, 10.0, 0.01, 0.7) {
            Err(ResonanceError::InvalidDuration { duration_s, .. }) => assert_eq!(duration_s, 0.01),
            _ => panic!("Expected InvalidDuration"),
        }
    }

    #[test]
    fn test_binaural_invalid_amplitude() {
        match BinauralBeat::new(200.0, 10.0, 60.0, 1.5) {
            Err(ResonanceError::InvalidAmplitude { amp, .. }) => assert_eq!(amp, 1.5),
            _ => panic!("Expected InvalidAmplitude"),
        }
    }

    #[test]
    fn test_binaural_brainwave_bands() {
        let delta = BinauralBeat::new(200.0, 2.0, 60.0, 0.7).unwrap();
        assert_eq!(delta.brainwave_band(), "delta");

        let theta = BinauralBeat::new(200.0, 6.0, 60.0, 0.7).unwrap();
        assert_eq!(theta.brainwave_band(), "theta");

        let alpha = BinauralBeat::new(200.0, 10.0, 60.0, 0.7).unwrap();
        assert_eq!(alpha.brainwave_band(), "alpha");

        let beta = BinauralBeat::new(200.0, 20.0, 60.0, 0.7).unwrap();
        assert_eq!(beta.brainwave_band(), "beta");

        let gamma = BinauralBeat::new(200.0, 40.0, 60.0, 0.7).unwrap();
        assert_eq!(gamma.brainwave_band(), "gamma");
    }

    #[test]
    fn test_isochronic_creation() {
        let tone = IsochronicTone::new(200.0, 10.0, 60.0, 0.7).unwrap();
        assert_eq!(tone.base_freq_hz, 200.0);
        assert_eq!(tone.pulse_rate_hz, 10.0);
        assert_eq!(tone.brainwave_band(), "alpha");
    }

    #[test]
    fn test_isochronic_invalid_base_freq() {
        match IsochronicTone::new(50.0, 10.0, 60.0, 0.7) {
            Err(ResonanceError::InvalidFrequency { freq_hz, .. }) => assert_eq!(freq_hz, 50.0),
            _ => panic!("Expected InvalidFrequency"),
        }
    }

    #[test]
    fn test_semantic_creation() {
        let resp = SemanticResponse::new(
            "Respuesta constructiva.".to_string(),
            0.5,
            "alpha".to_string(),
            0.8,
        )
        .unwrap();
        assert_eq!(resp.sct_z, 0.5);
        assert_eq!(resp.confidence, 0.8);
    }

    #[test]
    fn test_semantic_sct_rejection() {
        match SemanticResponse::new("Texto.".to_string(), -0.5, "alpha".to_string(), 0.8) {
            Err(ResonanceError::SctRejection { z }) => assert_eq!(z, -0.5),
            _ => panic!("Expected SctRejection"),
        }
    }

    #[test]
    fn test_semantic_prohibited_word() {
        match SemanticResponse::new(
            "Esto menciona guerra en el texto.".to_string(),
            0.5,
            "alpha".to_string(),
            0.8,
        ) {
            Err(ResonanceError::SctRejection { z }) => assert_eq!(z, -1.0),
            _ => panic!("Expected SctRejection for prohibited word"),
        }
    }

    #[test]
    fn test_generate_response_valid() {
        let gen = ResonanceGenerator::new();
        let delta = make_positive_delta();
        let state = make_calm_state();

        let response = gen.generate_response(&delta, &state).unwrap();
        assert!(response.is_approved());
        assert!(response.binaural.is_some());
        assert!(response.isochronic.is_some());
    }

    #[test]
    fn test_generate_response_sct_rejected() {
        let gen = ResonanceGenerator::new();
        let delta = make_negative_delta();
        let state = make_stressed_state();

        match gen.generate_response(&delta, &state) {
            Err(ResonanceError::SctRejection { z }) => assert!(z < 0.0),
            _ => panic!("Expected SctRejection"),
        }
    }

    #[test]
    fn test_generate_response_no_audio() {
        let config = ResonanceConfig {
            enable_binaural: false,
            enable_isochronic: false,
            enable_semantic: true,
            ..Default::default()
        };
        let gen = ResonanceGenerator::with_config(config);
        let delta = make_positive_delta();
        let state = make_calm_state();

        let response = gen.generate_response(&delta, &state).unwrap();
        assert!(response.binaural.is_none());
        assert!(response.isochronic.is_none());
    }

    #[test]
    fn test_brainwave_selection_high_stress() {
        let gen = ResonanceGenerator::new();
        let delta = make_positive_delta();
        let stressed = make_stressed_state();

        let band = gen.select_brainwave_band(&delta, &stressed);
        assert_eq!(band, "alpha");
    }

    #[test]
    fn test_brainwave_selection_low_arousal() {
        let gen = ResonanceGenerator::new();
        let delta = make_positive_delta();
        let low_arousal = BiometricState::new(0.3, 0.6, 1.0, 0.2, 0.1).unwrap();

        let band = gen.select_brainwave_band(&delta, &low_arousal);
        assert_eq!(band, "beta");
    }

    #[test]
    fn test_brainwave_selection_high_coherence() {
        let gen = ResonanceGenerator::new();
        let delta = make_positive_delta();
        let coherent = BiometricState::new(0.1, 0.95, 1.0, 0.5, 0.5).unwrap();

        let band = gen.select_brainwave_band(&delta, &coherent);
        assert_eq!(band, "gamma");
    }

    #[test]
    fn test_band_to_frequency() {
        let gen = ResonanceGenerator::new();
        assert_eq!(gen.band_to_frequency("delta"), 2.0);
        assert_eq!(gen.band_to_frequency("theta"), 6.0);
        assert_eq!(gen.band_to_frequency("alpha"), 10.0);
        assert_eq!(gen.band_to_frequency("beta"), 20.0);
        assert_eq!(gen.band_to_frequency("gamma"), 40.0);
    }

    #[test]
    fn test_confidence_calculation() {
        let gen = ResonanceGenerator::new();
        let delta = make_positive_delta();
        let state = make_calm_state();

        let confidence = gen.calculate_confidence(&delta, &state);
        assert!(confidence > 0.0 && confidence <= 1.0);
    }

    #[test]
    fn test_response_is_approved() {
        let response = ResonanceResponse {
            binaural: None,
            isochronic: None,
            semantic: SemanticResponse::new("OK".to_string(), 0.5, "alpha".to_string(), 0.8)
                .unwrap(),
            homeostasis_target: 0.7,
            sct_z: 0.5,
        };
        assert!(response.is_approved());
    }

    #[test]
    fn test_response_not_approved() {
        let response = ResonanceResponse {
            binaural: None,
            isochronic: None,
            semantic: SemanticResponse::new("OK".to_string(), 0.5, "alpha".to_string(), 0.8)
                .unwrap(),
            homeostasis_target: 0.7,
            sct_z: -0.5,
        };
        assert!(!response.is_approved());
    }

    #[test]
    fn test_default() {
        let gen = ResonanceGenerator::default();
        assert!(gen.config().enable_binaural);
    }

    #[test]
    fn test_error_display() {
        match ResonanceError::MissingDelta {
            ResonanceError::MissingDelta => {
                let msg = format!("{}", ResonanceError::MissingDelta);
                assert!(msg.contains("delta"));
            }
            _ => unreachable!(),
        }
    }
}
