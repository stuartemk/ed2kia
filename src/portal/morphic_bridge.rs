//! Morphic Bridge — Local Purification before Network Interaction.
//!
//! Connects the MorphicResonanceDecoder and SemanticPurifier to the
//! SymbioticPortal, enabling local purification of user input before
//! any network interaction occurs.
//!
//! **Architecture:**
//! ```
//! User Input → MorphicResonanceDecoder → SemanticWaveform
//!     ↓ (if Lower Focus detected)
//! SemanticPurifier → Purified Text → Re-verify Waveform
//!     ↓ (if Z >= 0)
//! SymbioticPortal → Network Interaction
//! ```
//!
//! **Key Properties:**
//! - 100% local processing (no telemetry sent for raw input).
//! - WASM compatible (runs in Web Worker via SymbioticPortal).
//! - Non-blocking (async purification pipeline).
//! - Verifiable (purified output must score Z >= 0).
//!
//! **Feature Gate:** `v3.8-morphic-genesis`

#[cfg(all(feature = "v3.8-morphic-genesis", target_arch = "wasm32"))]
use wasm_bindgen::prelude::*;

#[cfg(feature = "v3.8-morphic-genesis")]
use crate::semantics::morphic_decoder::{
    MorphicResonanceDecoder,
    SemanticWaveform,
    DecoderConfig,
};
#[cfg(feature = "v3.8-morphic-genesis")]
use crate::semantics::semantic_purifier::{
    SemanticPurifier,
    PurifierConfig,
    NegativePattern,
};

/// Result of the morphic bridge pipeline.
#[cfg(feature = "v3.8-morphic-genesis")]
#[derive(Debug, Clone)]
pub struct BridgeResult {
    /// Original user input.
    pub input: String,
    /// Final text sent to network (purified if needed).
    pub output: String,
    /// Waveform of the final output.
    pub waveform: SemanticWaveform,
    /// Whether purification was applied.
    pub was_purified: bool,
    /// Detected negative pattern (if any).
    pub detected_pattern: Option<NegativePattern>,
    /// Pipeline status.
    pub status: BridgeStatus,
}

/// Status of the morphic bridge pipeline.
#[cfg(feature = "v3.8-morphic-genesis")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeStatus {
    /// Input was already constructive — passed through directly.
    Passed,
    /// Input was purified successfully.
    Purified,
    /// Input could not be purified — blocked from network.
    Blocked,
    /// Pipeline encountered an error.
    Error,
}

impl std::fmt::Display for BridgeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeStatus::Passed => write!(f, "Passed"),
            BridgeStatus::Purified => write!(f, "Purified"),
            BridgeStatus::Blocked => write!(f, "Blocked"),
            BridgeStatus::Error => write!(f, "Error"),
        }
    }
}

/// Error types for the morphic bridge.
#[cfg(feature = "v3.8-morphic-genesis")]
#[derive(Debug, Clone, PartialEq)]
pub enum BridgeError {
    /// Input decoding failed.
    DecodeError(String),
    /// Purification failed after all attempts.
    PurificationFailed(String),
    /// Purified output still scores below threshold.
    ThresholdNotMet {
        z_score: f64,
        threshold: f64,
    },
}

#[cfg(feature = "v3.8-morphic-genesis")]
impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeError::DecodeError(msg) => {
                write!(f, "BridgeError: decode failed — {msg}")
            }
            BridgeError::PurificationFailed(msg) => {
                write!(f, "BridgeError: purification failed — {msg}")
            }
            BridgeError::ThresholdNotMet { z_score, threshold } => {
                write!(
                    f,
                    "BridgeError: purified Z-score ({z_score}) below threshold ({threshold})"
                )
            }
        }
    }
}

/// Configuration for the MorphicBridge.
#[cfg(feature = "v3.8-morphic-genesis")]
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    /// Minimum Z-score for network passage.
    pub min_z_score: f64,
    /// Enable automatic purification.
    pub auto_purify: bool,
    /// Block inputs that cannot be purified.
    pub block_unpurifiable: bool,
    /// Decoder configuration.
    pub decoder_config: DecoderConfig,
    /// Purifier configuration.
    pub purifier_config: PurifierConfig,
}

#[cfg(feature = "v3.8-morphic-genesis")]
impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            min_z_score: 0.0,
            auto_purify: true,
            block_unpurifiable: true,
            decoder_config: DecoderConfig::default(),
            purifier_config: PurifierConfig::default(),
        }
    }
}

/// The Morphic Bridge — Local purification pipeline for SymbioticPortal.
///
/// Sits between user input and network interaction, ensuring that
/// all messages sent through the SymbioticPortal meet the minimum
/// ethical threshold (Z-score >= 0).
#[cfg(feature = "v3.8-morphic-genesis")]
#[derive(Debug, Clone)]
pub struct MorphicBridge {
    config: BridgeConfig,
    decoder: MorphicResonanceDecoder,
    purifier: SemanticPurifier,
}

#[cfg(feature = "v3.8-morphic-genesis")]
impl MorphicBridge {
    /// Create a new MorphicBridge with default configuration.
    pub fn new() -> Self {
        Self {
            config: BridgeConfig::default(),
            decoder: MorphicResonanceDecoder::new(),
            purifier: SemanticPurifier::new(),
        }
    }

    /// Create a MorphicBridge with custom configuration.
    pub fn with_config(config: BridgeConfig) -> Self {
        let purifier_config = config.purifier_config.clone();
        Self {
            decoder: MorphicResonanceDecoder::with_config(config.decoder_config.clone()),
            purifier: SemanticPurifier::with_config(purifier_config),
            config,
        }
    }

    /// Get the current configuration.
    pub fn config(&self) -> &BridgeConfig {
        &self.config
    }

    /// Process user input through the morphic bridge pipeline.
    ///
    /// **Pipeline:**
    /// 1. Decode input → SemanticWaveform.
    /// 2. If Z-score >= threshold → pass through (BridgeStatus::Passed).
    /// 3. If Z-score < threshold AND auto_purify → purify.
    /// 4. Re-verify purified output.
    /// 5. If purified Z-score >= threshold → pass (BridgeStatus::Purified).
    /// 6. If purified Z-score < threshold → block (BridgeStatus::Blocked).
    ///
    /// Returns `Ok(BridgeResult)` with the final output and status.
    pub fn process(&self, input: &str) -> Result<BridgeResult, BridgeError> {
        let input_str = input.to_string();

        // Step 1: Decode input
        let waveform = self
            .decoder
            .decode(input)
            .map_err(|e| BridgeError::DecodeError(e.to_string()))?;

        // Step 2: Check if already constructive
        if waveform.z_score >= self.config.min_z_score {
            return Ok(BridgeResult {
                input: input_str,
                output: input.to_string(),
                waveform,
                was_purified: false,
                detected_pattern: None,
                status: BridgeStatus::Passed,
            });
        }

        // Step 3: Auto-purify if enabled
        if !self.config.auto_purify {
            if self.config.block_unpurifiable {
                return Ok(BridgeResult {
                    input: input_str,
                    output: String::new(),
                    waveform,
                    was_purified: false,
                    detected_pattern: None,
                    status: BridgeStatus::Blocked,
                });
            } else {
                // Pass through without purification (degraded mode)
                return Ok(BridgeResult {
                    input: input_str,
                    output: input.to_string(),
                    waveform,
                    was_purified: false,
                    detected_pattern: None,
                    status: BridgeStatus::Passed,
                });
            }
        }

        // Step 4: Attempt purification
        let purification_result = self.purifier.purify(input).map_err(|e| {
            // AlreadyConstructive is not expected here (we checked above)
            BridgeError::PurificationFailed(e.to_string())
        });

        match purification_result {
            Ok(result) => {
                // Step 5: Re-verify purified output
                let purified_waveform = result.purified_waveform;

                if purified_waveform.z_score >= self.config.min_z_score {
                    Ok(BridgeResult {
                        input: input_str,
                        output: result.purified,
                        waveform: purified_waveform,
                        was_purified: true,
                        detected_pattern: result.detected_pattern,
                        status: BridgeStatus::Purified,
                    })
                } else if self.config.block_unpurifiable {
                    Ok(BridgeResult {
                        input: input_str,
                        output: String::new(),
                        waveform: purified_waveform,
                        was_purified: true,
                        detected_pattern: result.detected_pattern,
                        status: BridgeStatus::Blocked,
                    })
                } else {
                    // Pass through degraded
                    Ok(BridgeResult {
                        input: input_str,
                        output: result.purified,
                        waveform: purified_waveform,
                        was_purified: true,
                        detected_pattern: result.detected_pattern,
                        status: BridgeStatus::Passed,
                    })
                }
            }
            Err(e) => {
                if self.config.block_unpurifiable {
                    Ok(BridgeResult {
                        input: input_str,
                        output: String::new(),
                        waveform,
                        was_purified: false,
                        detected_pattern: None,
                        status: BridgeStatus::Blocked,
                    })
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Quick check: Is this input constructive enough for the network?
    pub fn is_constructive(&self, input: &str) -> bool {
        match self.decoder.decode(input) {
            Ok(wf) => wf.z_score >= self.config.min_z_score,
            Err(_) => false,
        }
    }

    /// Get the Z-score for an input without full pipeline processing.
    pub fn get_z_score(&self, input: &str) -> Option<f64> {
        self.decoder.decode(input).ok().map(|wf| wf.z_score)
    }
}

#[cfg(feature = "v3.8-morphic-genesis")]
impl Default for MorphicBridge {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// WASM Bindings for SymbioticPortal Integration
// ============================================================================

/// WASM-exposed MorphicBridge for use in Web Worker context.
#[cfg(all(feature = "v3.8-morphic-genesis", target_arch = "wasm32"))]
#[wasm_bindgen]
pub struct WasmMorphicBridge {
    bridge: MorphicBridge,
}

#[cfg(all(feature = "v3.8-morphic-genesis", target_arch = "wasm32"))]
#[wasm_bindgen]
impl WasmMorphicBridge {
    /// Create a new WASM MorphicBridge.
    #[wasm_bindgen(constructor)]
    pub fn constructor() -> Self {
        Self {
            bridge: MorphicBridge::new(),
        }
    }

    /// Process input through the morphic bridge pipeline.
    ///
    /// Returns a JSON string with the BridgeResult.
    #[wasm_bindgen]
    pub fn process(&self, input: &str) -> Result<String, JsValue> {
        match self.bridge.process(input) {
            Ok(result) => Ok(Self::result_to_json(&result)),
            Err(e) => Err(JsValue::from_str(&format!(
                "{{\"error\":\"{}\"}}",
                e.to_string()
            ))),
        }
    }

    /// Quick check if input is constructive.
    #[wasm_bindgen]
    pub fn is_constructive(&self, input: &str) -> bool {
        self.bridge.is_constructive(input)
    }

    /// Get the Z-score for an input.
    #[wasm_bindgen]
    pub fn get_z_score(&self, input: &str) -> Option<f64> {
        self.bridge.get_z_score(input)
    }

    /// Convert BridgeResult to JSON string.
    fn result_to_json(result: &BridgeResult) -> String {
        let pattern_str = match &result.detected_pattern {
            Some(p) => format!("\"{}\"", p.to_string()),
            None => "null".to_string(),
        };

        format!(
            r#"{{"input":"{}","output":"{}","z_score":{},"was_purified":{},"detected_pattern":{},"status":"{}"}}"#,
            escape_json(&result.input),
            escape_json(&result.output),
            result.waveform.z_score,
            result.was_purified,
            pattern_str,
            result.status.to_string(),
        )
    }
}

/// Escape a string for JSON embedding.
#[cfg(all(feature = "v3.8-morphic-genesis", target_arch = "wasm32"))]
fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(all(feature = "v3.8-morphic-genesis", target_arch = "wasm32"))]
impl Default for WasmMorphicBridge {
    fn default() -> Self {
        Self::constructor()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_creation() {
        let bridge = MorphicBridge::new();
        assert_eq!(bridge.config().min_z_score, 0.0);
    }

    #[test]
    fn test_bridge_custom_config() {
        let config = BridgeConfig {
            min_z_score: 0.2,
            auto_purify: false,
            ..BridgeConfig::default()
        };
        let bridge = MorphicBridge::with_config(config);
        assert_eq!(bridge.config().min_z_score, 0.2);
    }

    #[test]
    fn test_process_constructive_input() {
        let bridge = MorphicBridge::new();
        let result = bridge.process("cooperación y armonía").unwrap();
        assert_eq!(result.status, BridgeStatus::Passed);
        assert!(!result.was_purified);
        assert_eq!(result.input, result.output);
    }

    #[test]
    fn test_process_lower_focus_input() {
        let bridge = MorphicBridge::new();
        let result = bridge.process("miedo y amenaza").unwrap();
        assert!(
            result.status == BridgeStatus::Purified
                || result.status == BridgeStatus::Blocked
        );
    }

    #[test]
    fn test_process_purifies_fear() {
        let bridge = MorphicBridge::new();
        let result = bridge.process("miedo y escasez").unwrap();
        if result.status == BridgeStatus::Purified {
            assert!(result.was_purified);
            assert!(result.output != result.input);
        }
    }

    #[test]
    fn test_is_constructive_positive() {
        let bridge = MorphicBridge::new();
        assert!(bridge.is_constructive("cooperación y evolución"));
    }

    #[test]
    fn test_is_constructive_negative() {
        let bridge = MorphicBridge::new();
        assert!(!bridge.is_constructive("miedo y amenaza y peligro"));
    }

    #[test]
    fn test_get_z_score() {
        let bridge = MorphicBridge::new();
        let score = bridge.get_z_score("cooperación y armonía");
        assert!(score.is_some());
        assert!(score.unwrap() > 0.0);
    }

    #[test]
    fn test_get_z_score_empty() {
        let bridge = MorphicBridge::new();
        assert!(bridge.get_z_score("").is_none());
    }

    #[test]
    fn test_bridge_default() {
        let bridge = MorphicBridge::default();
        assert_eq!(bridge.config().min_z_score, 0.0);
    }

    #[test]
    fn test_bridge_status_display() {
        assert_eq!(format!("{}", BridgeStatus::Passed), "Passed");
        assert_eq!(format!("{}", BridgeStatus::Purified), "Purified");
        assert_eq!(format!("{}", BridgeStatus::Blocked), "Blocked");
        assert_eq!(format!("{}", BridgeStatus::Error), "Error");
    }

    #[test]
    fn test_error_display() {
        assert_eq!(
            format!(
                "{}",
                BridgeError::DecodeError("test".to_string())
            ),
            "BridgeError: decode failed — test"
        );
        assert_eq!(
            format!(
                "{}",
                BridgeError::PurificationFailed("test".to_string())
            ),
            "BridgeError: purification failed — test"
        );
        let err = BridgeError::ThresholdNotMet {
            z_score: -0.5,
            threshold: 0.0,
        };
        assert_eq!(
            format!("{}", err),
            "BridgeError: purified Z-score (-0.5) below threshold (0)"
        );
    }

    #[test]
    fn test_auto_purify_disabled_pass_through() {
        let config = BridgeConfig {
            auto_purify: false,
            block_unpurifiable: false,
            ..BridgeConfig::default()
        };
        let bridge = MorphicBridge::with_config(config);
        let result = bridge.process("miedo y amenaza").unwrap();
        // Should pass through without purification
        assert_eq!(result.status, BridgeStatus::Passed);
        assert!(!result.was_purified);
    }

    #[test]
    fn test_auto_purify_disabled_block() {
        let config = BridgeConfig {
            auto_purify: false,
            block_unpurifiable: true,
            ..BridgeConfig::default()
        };
        let bridge = MorphicBridge::with_config(config);
        let result = bridge.process("miedo y amenaza").unwrap();
        // Should block without purification
        assert_eq!(result.status, BridgeStatus::Blocked);
        assert!(result.output.is_empty());
    }

    #[test]
    fn test_high_threshold() {
        let config = BridgeConfig {
            min_z_score: 0.5,
            ..BridgeConfig::default()
        };
        let bridge = MorphicBridge::with_config(config);
        // Even "cooperación" might not reach 0.5 threshold
        let result = bridge.process("cooperación").unwrap();
        assert!(
            result.status == BridgeStatus::Passed
                || result.status == BridgeStatus::Purified
        );
    }
}
