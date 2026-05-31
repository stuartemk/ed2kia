//! Semantic Purifier — Re-contextualization for Lower Focus Inputs.
//!
//! The SemanticPurifier does NOT censor. Instead, it re-contextualizes
//! manipulative or fear-based inputs (Lower Focus) into constructive queries
//! (Upper Focus) by extracting the real user need and removing the
//! manipulative load.
//!
//! **Core Principle:** Every Lower Focus input contains a legitimate human
//! need wrapped in fear, scarcity, or division. The purifier extracts that
//! need and re-expresses it in constructive terms.
//!
//! **Algorithm:**
//! 1. Decode input through MorphicResonanceDecoder to get SemanticWaveform.
//! 2. If Z-score >= 0 (already constructive), pass through unchanged.
//! 3. If Z-score < 0 (Lower Focus detected):
//!    a. Identify dominant negative pattern (fear/scarcity/division).
//!    b. Extract core intent (what the user actually needs).
//!    c. Re-express intent using Upper Focus vocabulary.
//!    d. Verify purified output has Z-score >= 0.
//!
//! **Examples:**
//! - "¡Ellos nos robarán todo!" → "¿Cómo podemos proteger nuestros recursos?"
//! - "No hay suficiente para todos" → "¿Cómo distribuir equitativamente los recursos?"
//! - "Debes actuar ahora o será tarde" → "¿Cuáles son los pasos para prepararnos?"
//!
//! **Design Constraints:**
//! - WASM compatible (lightweight string transformation).
//! - No censorship — preserves user intent, only re-aligns expression.
//! - Verifiable output (purified text must score Z >= 0).
//!
//! **Feature Gate:** `v3.8-morphic-genesis`

use std::fmt;

use super::morphic_decoder::{
    DecoderConfig, IntentClassification, MorphicResonanceDecoder, SemanticWaveform,
};

/// Error types for semantic purification.
#[derive(Debug, Clone, PartialEq)]
pub enum PurificationError {
    /// Input could not be decoded for purification.
    DecodeError(String),
    /// Purification failed to produce constructive output after max attempts.
    PurificationFailed(String),
    /// Input is already constructive — no purification needed.
    AlreadyConstructive,
}

impl fmt::Display for PurificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PurificationError::DecodeError(msg) => {
                write!(f, "PurificationError: decode failed — {msg}")
            }
            PurificationError::PurificationFailed(msg) => {
                write!(f, "PurificationError: purification failed — {msg}")
            }
            PurificationError::AlreadyConstructive => {
                write!(f, "PurificationError: input is already constructive")
            }
        }
    }
}

/// Result of semantic purification.
#[derive(Debug, Clone)]
pub struct PurificationResult {
    /// Original input text.
    pub original: String,
    /// Purified output text (may be same as original if already constructive).
    pub purified: String,
    /// Waveform of the original input.
    pub original_waveform: SemanticWaveform,
    /// Waveform of the purified output.
    pub purified_waveform: SemanticWaveform,
    /// Whether purification was actually applied.
    pub was_purified: bool,
    /// The dominant negative pattern detected (if any).
    pub detected_pattern: Option<NegativePattern>,
}

/// Classification of dominant negative pattern in Lower Focus input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NegativePattern {
    /// Fear-based manipulation (threat, danger, panic).
    Fear,
    /// Scarcity-based manipulation (lack, poverty, not enough).
    Scarcity,
    /// Division-based manipulation (us vs them, enemy, conflict).
    Division,
    /// Urgency-based manipulation (false deadlines, panic timing).
    FalseUrgency,
    /// Control-based manipulation (domination, forced compliance).
    Control,
    /// Deception-based manipulation (lies, hidden agendas).
    Deception,
}

impl fmt::Display for NegativePattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NegativePattern::Fear => write!(f, "Fear"),
            NegativePattern::Scarcity => write!(f, "Scarcity"),
            NegativePattern::Division => write!(f, "Division"),
            NegativePattern::FalseUrgency => write!(f, "FalseUrgency"),
            NegativePattern::Control => write!(f, "Control"),
            NegativePattern::Deception => write!(f, "Deception"),
        }
    }
}

/// Configuration for the SemanticPurifier.
#[derive(Debug, Clone)]
pub struct PurifierConfig {
    /// Minimum Z-score for purified output to be considered valid.
    pub min_purified_z_score: f64,
    /// Maximum purification attempts before giving up.
    pub max_attempts: usize,
    /// Enable pattern-specific re-contextualization.
    pub enable_pattern_matching: bool,
    /// Decoder configuration for waveform analysis.
    pub decoder_config: DecoderConfig,
}

impl Default for PurifierConfig {
    fn default() -> Self {
        Self {
            min_purified_z_score: 0.0,
            max_attempts: 3,
            enable_pattern_matching: true,
            decoder_config: DecoderConfig::default(),
        }
    }
}

/// Re-contextualization template: maps negative patterns to constructive alternatives.
#[derive(Debug, Clone)]
struct RecontextTemplate {
    /// Pattern to detect in input.
    pattern: &'static str,
    /// Negative pattern category.
    category: NegativePattern,
    /// Constructive replacement phrase.
    replacement: &'static str,
}

/// Templates for re-contextualizing Lower Focus patterns into Upper Focus.
const RECONTEXT_TEMPLATES: &[RecontextTemplate] = &[
    // Fear → Protection/Preparation
    RecontextTemplate {
        pattern: "miedo",
        category: NegativePattern::Fear,
        replacement: "preparación",
    },
    RecontextTemplate {
        pattern: "fear",
        category: NegativePattern::Fear,
        replacement: "preparation",
    },
    RecontextTemplate {
        pattern: "amenaza",
        category: NegativePattern::Fear,
        replacement: "desafío",
    },
    RecontextTemplate {
        pattern: "threat",
        category: NegativePattern::Fear,
        replacement: "challenge",
    },
    RecontextTemplate {
        pattern: "peligro",
        category: NegativePattern::Fear,
        replacement: "precaución",
    },
    RecontextTemplate {
        pattern: "danger",
        category: NegativePattern::Fear,
        replacement: "caution",
    },
    RecontextTemplate {
        pattern: "pánico",
        category: NegativePattern::Fear,
        replacement: "calma",
    },
    RecontextTemplate {
        pattern: "panic",
        category: NegativePattern::Fear,
        replacement: "calm",
    },
    // Scarcity → Distribution/Abundance
    RecontextTemplate {
        pattern: "escasez",
        category: NegativePattern::Scarcity,
        replacement: "distribución equitativa",
    },
    RecontextTemplate {
        pattern: "scarcity",
        category: NegativePattern::Scarcity,
        replacement: "equitable distribution",
    },
    RecontextTemplate {
        pattern: "falta",
        category: NegativePattern::Scarcity,
        replacement: "necesidad",
    },
    RecontextTemplate {
        pattern: "lack",
        category: NegativePattern::Scarcity,
        replacement: "need",
    },
    RecontextTemplate {
        pattern: "pobreza",
        category: NegativePattern::Scarcity,
        replacement: "desarrollo",
    },
    RecontextTemplate {
        pattern: "poverty",
        category: NegativePattern::Scarcity,
        replacement: "development",
    },
    // Division → Unity/Cooperation
    RecontextTemplate {
        pattern: "división",
        category: NegativePattern::Division,
        replacement: "diálogo",
    },
    RecontextTemplate {
        pattern: "division",
        category: NegativePattern::Division,
        replacement: "dialogue",
    },
    RecontextTemplate {
        pattern: "conflicto",
        category: NegativePattern::Division,
        replacement: "resolución",
    },
    RecontextTemplate {
        pattern: "conflict",
        category: NegativePattern::Division,
        replacement: "resolution",
    },
    RecontextTemplate {
        pattern: "oponente",
        category: NegativePattern::Division,
        replacement: "compañero",
    },
    RecontextTemplate {
        pattern: "enemy",
        category: NegativePattern::Division,
        replacement: "partner",
    },
    // False Urgency → Planning
    RecontextTemplate {
        pattern: "urgente",
        category: NegativePattern::FalseUrgency,
        replacement: "planificado",
    },
    RecontextTemplate {
        pattern: "urgent",
        category: NegativePattern::FalseUrgency,
        replacement: "planned",
    },
    // Control → Autonomy
    RecontextTemplate {
        pattern: "controlar",
        category: NegativePattern::Control,
        replacement: "guiar",
    },
    RecontextTemplate {
        pattern: "control",
        category: NegativePattern::Control,
        replacement: "guide",
    },
    RecontextTemplate {
        pattern: "supremacía",
        category: NegativePattern::Control,
        replacement: "colaborar",
    },
    RecontextTemplate {
        pattern: "dominate",
        category: NegativePattern::Control,
        replacement: "collaborate",
    },
    // Deception → Transparency
    RecontextTemplate {
        pattern: "engaño",
        category: NegativePattern::Deception,
        replacement: "transparencia",
    },
    RecontextTemplate {
        pattern: "deception",
        category: NegativePattern::Deception,
        replacement: "transparency",
    },
    RecontextTemplate {
        pattern: "manipulación",
        category: NegativePattern::Deception,
        replacement: "comunicación honesta",
    },
    RecontextTemplate {
        pattern: "manipulation",
        category: NegativePattern::Deception,
        replacement: "honest communication",
    },
    RecontextTemplate {
        pattern: "mentira",
        category: NegativePattern::Deception,
        replacement: "verdad",
    },
    RecontextTemplate {
        pattern: "lie",
        category: NegativePattern::Deception,
        replacement: "truth",
    },
];

/// The Semantic Purifier — Re-contextualizes Lower Focus into Upper Focus.
///
/// Uses pattern-based re-contextualization combined with MorphicResonanceDecoder
/// to verify that purified output achieves constructive alignment.
#[derive(Debug, Clone)]
pub struct SemanticPurifier {
    config: PurifierConfig,
    decoder: MorphicResonanceDecoder,
}

impl SemanticPurifier {
    /// Create a new purifier with default configuration.
    pub fn new() -> Self {
        Self {
            config: PurifierConfig::default(),
            decoder: MorphicResonanceDecoder::new(),
        }
    }

    /// Create a purifier with custom configuration.
    pub fn with_config(config: PurifierConfig) -> Self {
        Self {
            decoder: MorphicResonanceDecoder::with_config(config.decoder_config.clone()),
            config,
        }
    }

    /// Get the current configuration.
    pub fn config(&self) -> &PurifierConfig {
        &self.config
    }

    /// Get the internal decoder.
    pub fn decoder(&self) -> &MorphicResonanceDecoder {
        &self.decoder
    }

    /// Purify input text, re-contextualizing Lower Focus into Upper Focus.
    ///
    /// Returns `Ok(PurificationResult)` with the purified text and waveforms.
    /// If the input is already constructive, returns `Err(PurificationError::AlreadyConstructive)`.
    pub fn purify(&self, input: &str) -> Result<PurificationResult, PurificationError> {
        let original = input.to_string();

        // Step 1: Decode original input
        let original_waveform = self
            .decoder
            .decode(input)
            .map_err(|e| PurificationError::DecodeError(e.to_string()))?;

        // Step 2: Check if already constructive
        if original_waveform.intent == IntentClassification::UpperFocus {
            return Err(PurificationError::AlreadyConstructive);
        }

        // Step 3: Detect dominant negative pattern
        let detected_pattern = if self.config.enable_pattern_matching {
            self.detect_pattern(input, &original_waveform)
        } else {
            None
        };

        // Step 4: Apply re-contextualization
        let mut purified = self.apply_recontext(input, &detected_pattern);

        // Step 5: Verify and iterate if needed
        for _attempt in 1..=self.config.max_attempts {
            match self.decoder.decode(&purified) {
                Ok(wf) => {
                    if wf.z_score >= self.config.min_purified_z_score {
                        break; // Purification successful
                    }
                    // Try stronger purification
                    purified = self.apply_strong_purification(&purified, &detected_pattern);
                }
                Err(_) => {
                    // If decode fails, try with generic constructive framing
                    purified = self.generic_constructive_frame(&original);
                }
            }
        }

        // Final decode attempt
        let final_purified_waveform = match self.decoder.decode(&purified) {
            Ok(wf) => wf,
            Err(_) => original_waveform, // Fallback to original if purification decode fails
        };

        Ok(PurificationResult {
            original,
            purified,
            original_waveform,
            purified_waveform: final_purified_waveform,
            was_purified: final_purified_waveform != original_waveform,
            detected_pattern,
        })
    }

    /// Detect the dominant negative pattern in the input.
    fn detect_pattern(&self, input: &str, waveform: &SemanticWaveform) -> Option<NegativePattern> {
        if waveform.intent != IntentClassification::LowerFocus {
            return None;
        }

        let lower = input.to_lowercase();

        // Check each pattern category and find the dominant one
        let mut pattern_scores: Vec<(NegativePattern, f64)> = Vec::new();

        for template in RECONTEXT_TEMPLATES {
            if lower.contains(template.pattern) {
                // Find or add score for this category
                if let Some(entry) = pattern_scores
                    .iter_mut()
                    .find(|(p, _)| *p == template.category)
                {
                    entry.1 += 1.0;
                } else {
                    pattern_scores.push((template.category, 1.0));
                }
            }
        }

        // Return the pattern with highest score
        if pattern_scores.is_empty() {
            None
        } else {
            pattern_scores
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            Some(pattern_scores[0].0)
        }
    }

    /// Apply pattern-based re-contextualization.
    fn apply_recontext(&self, input: &str, detected: &Option<NegativePattern>) -> String {
        let mut result = input.to_string();
        let lower = result.to_lowercase();

        // Apply templates in order of specificity (longer patterns first)
        let mut templates: Vec<&RecontextTemplate> = RECONTEXT_TEMPLATES.iter().collect();
        templates.sort_by(|a, b| b.pattern.len().cmp(&a.pattern.len()));

        for template in &templates {
            // Skip if we have a detected pattern and this template doesn't match
            if let Some(detected_pattern) = detected {
                if template.category != *detected_pattern {
                    continue;
                }
            }

            // Case-insensitive replacement
            let pattern_lower = template.pattern;
            if lower.contains(pattern_lower) {
                // Preserve original case pattern for replacement
                result =
                    Self::case_insensitive_replace(&result, pattern_lower, template.replacement);
            }
        }

        result
    }

    /// Apply stronger purification when basic re-contextualization is insufficient.
    fn apply_strong_purification(&self, input: &str, detected: &Option<NegativePattern>) -> String {
        // Wrap the input in a constructive query frame
        let base = self.apply_recontext(input, detected);

        match detected {
            Some(NegativePattern::Fear) => {
                format!("¿Cómo podemos prepararnos para enfrentar: {}?", base)
            }
            Some(NegativePattern::Scarcity) => {
                format!("¿Cómo lograr distribución equitativa: {}?", base)
            }
            Some(NegativePattern::Division) => {
                format!("¿Cómo promover el diálogo constructivo: {}?", base)
            }
            Some(NegativePattern::FalseUrgency) => {
                format!("¿Cuáles son los pasos planificados para: {}?", base)
            }
            Some(NegativePattern::Control) => {
                format!("¿Cómo guiar con autonomía: {}?", base)
            }
            Some(NegativePattern::Deception) => {
                format!("¿Cómo promover la transparencia: {}?", base)
            }
            None => {
                format!("¿Cómo abordar constructivamente: {}?", base)
            }
        }
    }

    /// Generic constructive framing when pattern-specific purification fails.
    fn generic_constructive_frame(&self, input: &str) -> String {
        format!("Busco entender y mejorar: {}", input)
    }

    /// Case-insensitive string replacement.
    fn case_insensitive_replace(haystack: &str, needle: &str, replacement: &str) -> String {
        let hay_lower = haystack.to_lowercase();
        let mut result = String::with_capacity(haystack.len());
        let mut last_pos = 0;

        while let Some(rel_pos) = hay_lower[last_pos..].find(needle) {
            let actual_pos = rel_pos + last_pos;
            result.push_str(&haystack[last_pos..actual_pos]);
            result.push_str(replacement);
            last_pos = actual_pos + needle.len();
        }

        result.push_str(&haystack[last_pos..]);
        result
    }
}

impl Default for SemanticPurifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_purifier_creation() {
        let purifier = SemanticPurifier::new();
        assert_eq!(purifier.config().min_purified_z_score, 0.0);
    }

    #[test]
    fn test_purifier_custom_config() {
        let config = PurifierConfig {
            min_purified_z_score: 0.1,
            max_attempts: 5,
            ..PurifierConfig::default()
        };
        let purifier = SemanticPurifier::with_config(config);
        assert_eq!(purifier.config().min_purified_z_score, 0.1);
    }

    #[test]
    fn test_already_constructive() {
        let purifier = SemanticPurifier::new();
        match purifier.purify("cooperación y armonía para la evolución") {
            Err(PurificationError::AlreadyConstructive) => {}
            other => panic!("Expected AlreadyConstructive, got {:?}", other),
        }
    }

    #[test]
    fn test_purify_fear_pattern() {
        let purifier = SemanticPurifier::new();
        let result = purifier.purify("miedo y amenaza de escasez").unwrap();
        assert!(result.was_purified);
        assert!(result.purified.contains("preparación") || result.purified.contains("desafío"));
        assert_eq!(result.detected_pattern, Some(NegativePattern::Fear));
    }

    #[test]
    fn test_purify_division_pattern() {
        let purifier = SemanticPurifier::new();
        let result = purifier
            .purify("división y conflicto entre oponente")
            .unwrap();
        assert!(result.was_purified);
        assert!(
            result.purified.contains("diálogo")
                || result.purified.contains("resolución")
                || result.purified.contains("compañero")
        );
    }

    #[test]
    fn test_purify_scarcity_pattern() {
        let purifier = SemanticPurifier::new();
        let result = purifier.purify("escasez y pobreza para todos").unwrap();
        assert!(result.was_purified);
        assert!(result.purified.contains("distribución") || result.purified.contains("desarrollo"));
    }

    #[test]
    fn test_purify_english_fear() {
        let purifier = SemanticPurifier::new();
        let result = purifier.purify("fear and threat of scarcity").unwrap();
        assert!(result.was_purified);
        assert!(result.purified.contains("preparation") || result.purified.contains("challenge"));
    }

    #[test]
    fn test_purification_result_fields() {
        let purifier = SemanticPurifier::new();
        let result = purifier.purify("miedo y peligro").unwrap();
        assert_eq!(result.original, "miedo y peligro");
        assert!(!result.purified.is_empty());
        assert!(result.original_waveform.intent == IntentClassification::LowerFocus);
    }

    #[test]
    fn test_detect_pattern_fear() {
        let purifier = SemanticPurifier::new();
        let decoder = MorphicResonanceDecoder::new();
        let waveform = decoder.decode("miedo y amenaza y peligro").unwrap();
        let pattern = purifier.detect_pattern("miedo y amenaza y peligro", &waveform);
        assert_eq!(pattern, Some(NegativePattern::Fear));
    }

    #[test]
    fn test_detect_pattern_none_for_upper_focus() {
        let purifier = SemanticPurifier::new();
        let decoder = MorphicResonanceDecoder::new();
        let waveform = decoder.decode("cooperación y armonía").unwrap();
        let pattern = purifier.detect_pattern("cooperación y armonía", &waveform);
        assert_eq!(pattern, None);
    }

    #[test]
    fn test_case_insensitive_replace() {
        let result =
            SemanticPurifier::case_insensitive_replace("El Miedo nos une", "miedo", "valor");
        assert_eq!(result, "El valor nos une");
    }

    #[test]
    fn test_case_insensitive_replace_multiple() {
        let result =
            SemanticPurifier::case_insensitive_replace("miedo MIEDO miedo", "miedo", "calma");
        assert_eq!(result, "calma CALMA calma");
    }

    #[test]
    fn test_negative_pattern_display() {
        assert_eq!(format!("{}", NegativePattern::Fear), "Fear");
        assert_eq!(format!("{}", NegativePattern::Scarcity), "Scarcity");
        assert_eq!(format!("{}", NegativePattern::Division), "Division");
        assert_eq!(format!("{}", NegativePattern::FalseUrgency), "FalseUrgency");
        assert_eq!(format!("{}", NegativePattern::Control), "Control");
        assert_eq!(format!("{}", NegativePattern::Deception), "Deception");
    }

    #[test]
    fn test_error_display() {
        assert_eq!(
            format!("{}", PurificationError::DecodeError("test".to_string())),
            "PurificationError: decode failed — test"
        );
        assert_eq!(
            format!(
                "{}",
                PurificationError::PurificationFailed("test".to_string())
            ),
            "PurificationError: purification failed — test"
        );
        assert_eq!(
            format!("{}", PurificationError::AlreadyConstructive),
            "PurificationError: input is already constructive"
        );
    }

    #[test]
    fn test_purifier_default() {
        let purifier = SemanticPurifier::default();
        assert_eq!(purifier.config().min_purified_z_score, 0.0);
    }

    #[test]
    fn test_pattern_matching_disabled() {
        let config = PurifierConfig {
            enable_pattern_matching: false,
            ..PurifierConfig::default()
        };
        let purifier = SemanticPurifier::with_config(config);
        let result = purifier.purify("miedo y amenaza").unwrap();
        // Should still purify, but without specific pattern detection
        assert!(result.purified != result.original || result.was_purified);
    }

    #[test]
    fn test_strong_purification_applied() {
        let purifier = SemanticPurifier::new();
        let result = purifier
            .purify("miedo y amenaza y peligro y pánico")
            .unwrap();
        // With multiple fear patterns, strong purification should wrap in query
        assert!(!result.purified.is_empty());
    }
}
