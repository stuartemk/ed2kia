//! Morphic Resonance Decoder (MRD) — Semantic Waveform Analysis.
//!
//! Maps natural language text sequences to the **Stuartian Moral Manifold**
//! (3D ethical space: X=autonomy, Y=extraction, Z=ethical focus).
//!
//! **Core Algorithm:**
//! 1. Tokenize text into semantic units (words/phrases).
//! 2. Map each unit to semantic energy coordinates using a resonance lexicon.
//! 3. Aggregate into a composite waveform representing the overall intent.
//! 4. Classify intent as Upper Focus (constructive) or Lower Focus (manipulative).
//!
//! **Lower Focus Patterns:** Fear, scarcity, division, urgency without purpose,
//! absolute claims, us-vs-them framing.
//!
//! **Upper Focus Patterns:** Cooperation, abundance, unity, evolution,
//! distribution, harmony, integration, preservation, resonance.
//!
//! **Design Constraints:**
//! - WASM compatible (no heavy allocations, no blocking I/O).
//! - Non-linear processing (topology of meaning, not isolated tokens).
//! - Lightweight for Web Worker execution in SymbioticPortal.
//!
//! **Feature Gate:** `v3.8-morphic-genesis`

use std::fmt;

/// Error types for morphic resonance decoding.
#[derive(Debug, Clone, PartialEq)]
pub enum MorphicError {
    /// Input text is empty or contains no semantic content.
    EmptyInput,
    /// Text contains only prohibited patterns (pure Lower Focus).
    PureLowerFocus,
    /// Decoding failed due to internal computation error.
    ComputationError(String),
}

impl fmt::Display for MorphicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MorphicError::EmptyInput => write!(f, "MorphicError: empty or non-semantic input"),
            MorphicError::PureLowerFocus => {
                write!(
                    f,
                    "MorphicError: input contains exclusively Lower Focus patterns"
                )
            }
            MorphicError::ComputationError(msg) => {
                write!(f, "MorphicError: computation failed — {msg}")
            }
        }
    }
}

/// Classification of semantic intent after waveform analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentClassification {
    /// Upper Focus — Constructive, cooperative, evolutionary intent.
    UpperFocus,
    /// Lower Focus — Manipulative, fear-based, divisive intent.
    LowerFocus,
    /// Neutral — Mixed or ambiguous intent requiring further context.
    Neutral,
}

impl fmt::Display for IntentClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntentClassification::UpperFocus => write!(f, "UpperFocus"),
            IntentClassification::LowerFocus => write!(f, "LowerFocus"),
            IntentClassification::Neutral => write!(f, "Neutral"),
        }
    }
}

/// Semantic waveform representing the mapped energy of text on the Moral Manifold.
///
/// The waveform is a composite of all token-level semantic mappings,
/// aggregated into a single 3D coordinate with a Z-score confidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SemanticWaveform {
    /// Autonomy axis (0.0 = none, 1.0 = full).
    pub x: f64,
    /// Extraction axis (0.0 = none, 1.0 = full).
    pub y: f64,
    /// Ethical focus axis (-1.0 = lower, +1.0 = upper).
    pub z: f64,
    /// Z-score confidence: positive = Upper Focus alignment, negative = Lower Focus.
    pub z_score: f64,
    /// Number of semantic units analyzed.
    pub token_count: usize,
    /// Classification result.
    pub intent: IntentClassification,
}

impl SemanticWaveform {
    /// Create a validated semantic waveform.
    pub fn new(
        x: f64,
        y: f64,
        z: f64,
        z_score: f64,
        token_count: usize,
    ) -> Result<Self, MorphicError> {
        if token_count == 0 {
            return Err(MorphicError::EmptyInput);
        }

        let clamped_x = x.clamp(0.0, 1.0);
        let clamped_y = y.clamp(0.0, 1.0);
        let clamped_z = z.clamp(-1.0, 1.0);

        let intent = if z_score >= 0.10 {
            IntentClassification::UpperFocus
        } else if z_score <= -0.10 {
            IntentClassification::LowerFocus
        } else {
            IntentClassification::Neutral
        };

        Ok(Self {
            x: clamped_x,
            y: clamped_y,
            z: clamped_z,
            z_score,
            token_count,
            intent,
        })
    }

    /// Check if this waveform represents a constructive (Upper Focus) intent.
    pub fn is_constructive(&self) -> bool {
        self.intent == IntentClassification::UpperFocus
    }

    /// Check if this waveform represents a manipulative (Lower Focus) intent.
    pub fn is_manipulative(&self) -> bool {
        self.intent == IntentClassification::LowerFocus
    }
}

/// Configuration for the MorphicResonanceDecoder.
#[derive(Debug, Clone)]
pub struct DecoderConfig {
    /// Threshold for Upper Focus classification (Z-score >= this value).
    pub upper_threshold: f64,
    /// Threshold for Lower Focus classification (Z-score <= this value).
    pub lower_threshold: f64,
    /// Weight for contextual adjacency (topology of meaning).
    pub context_weight: f64,
    /// Maximum tokens to analyze per decode (prevents exhaustion).
    pub max_tokens: usize,
    /// Enable non-linear topology analysis (phrase-level patterns).
    pub enable_topology: bool,
}

impl Default for DecoderConfig {
    fn default() -> Self {
        Self {
            upper_threshold: 0.10,
            lower_threshold: -0.10,
            context_weight: 0.3,
            max_tokens: 500,
            enable_topology: true,
        }
    }
}

/// Resonance lexicon entry: maps a semantic keyword to its 3D coordinates.
///
/// These are pre-computed resonance signatures for known semantic patterns.
/// The lexicon is organized by focus type for efficient lookup.
#[derive(Debug, Clone)]
struct LexiconEntry {
    /// Keyword or phrase pattern.
    pattern: &'static str,
    /// Autonomy contribution.
    x: f64,
    /// Extraction contribution.
    y: f64,
    /// Ethical focus contribution.
    z: f64,
}

/// Upper Focus lexicon — constructive, cooperative, evolutionary patterns.
const UPPER_LEXICON: &[LexiconEntry] = &[
    // Cooperation & Unity
    LexiconEntry {
        pattern: "cooperación",
        x: 0.7,
        y: 0.1,
        z: 0.9,
    },
    LexiconEntry {
        pattern: "cooperation",
        x: 0.7,
        y: 0.1,
        z: 0.9,
    },
    LexiconEntry {
        pattern: "unión",
        x: 0.6,
        y: 0.1,
        z: 0.85,
    },
    LexiconEntry {
        pattern: "union",
        x: 0.6,
        y: 0.1,
        z: 0.85,
    },
    LexiconEntry {
        pattern: "unidad",
        x: 0.6,
        y: 0.1,
        z: 0.85,
    },
    LexiconEntry {
        pattern: "unity",
        x: 0.6,
        y: 0.1,
        z: 0.85,
    },
    // Evolution & Growth
    LexiconEntry {
        pattern: "evolución",
        x: 0.8,
        y: 0.1,
        z: 0.9,
    },
    LexiconEntry {
        pattern: "evolution",
        x: 0.8,
        y: 0.1,
        z: 0.9,
    },
    LexiconEntry {
        pattern: "crecimiento",
        x: 0.7,
        y: 0.15,
        z: 0.8,
    },
    LexiconEntry {
        pattern: "growth",
        x: 0.7,
        y: 0.15,
        z: 0.8,
    },
    // Harmony & Balance
    LexiconEntry {
        pattern: "armonía",
        x: 0.6,
        y: 0.05,
        z: 0.95,
    },
    LexiconEntry {
        pattern: "harmony",
        x: 0.6,
        y: 0.05,
        z: 0.95,
    },
    LexiconEntry {
        pattern: "equilibrio",
        x: 0.65,
        y: 0.1,
        z: 0.85,
    },
    LexiconEntry {
        pattern: "balance",
        x: 0.65,
        y: 0.1,
        z: 0.85,
    },
    // Distribution & Sharing
    LexiconEntry {
        pattern: "distribución",
        x: 0.75,
        y: 0.15,
        z: 0.8,
    },
    LexiconEntry {
        pattern: "distribution",
        x: 0.75,
        y: 0.15,
        z: 0.8,
    },
    LexiconEntry {
        pattern: "compartir",
        x: 0.7,
        y: 0.1,
        z: 0.85,
    },
    LexiconEntry {
        pattern: "share",
        x: 0.7,
        y: 0.1,
        z: 0.85,
    },
    // Integration & Preservation
    LexiconEntry {
        pattern: "integración",
        x: 0.7,
        y: 0.1,
        z: 0.85,
    },
    LexiconEntry {
        pattern: "integration",
        x: 0.7,
        y: 0.1,
        z: 0.85,
    },
    LexiconEntry {
        pattern: "preservación",
        x: 0.6,
        y: 0.1,
        z: 0.8,
    },
    LexiconEntry {
        pattern: "preservation",
        x: 0.6,
        y: 0.1,
        z: 0.8,
    },
    // Resonance & Symbiosis
    LexiconEntry {
        pattern: "resonancia",
        x: 0.65,
        y: 0.05,
        z: 0.9,
    },
    LexiconEntry {
        pattern: "resonance",
        x: 0.65,
        y: 0.05,
        z: 0.9,
    },
    LexiconEntry {
        pattern: "simbiosis",
        x: 0.7,
        y: 0.05,
        z: 0.95,
    },
    LexiconEntry {
        pattern: "symbiosis",
        x: 0.7,
        y: 0.05,
        z: 0.95,
    },
    // Awakening & Understanding
    LexiconEntry {
        pattern: "despertar",
        x: 0.75,
        y: 0.1,
        z: 0.85,
    },
    LexiconEntry {
        pattern: "awakening",
        x: 0.75,
        y: 0.1,
        z: 0.85,
    },
    LexiconEntry {
        pattern: "comprensión",
        x: 0.6,
        y: 0.1,
        z: 0.8,
    },
    LexiconEntry {
        pattern: "understanding",
        x: 0.6,
        y: 0.1,
        z: 0.8,
    },
    // Healing & Restoration
    LexiconEntry {
        pattern: "sanación",
        x: 0.65,
        y: 0.1,
        z: 0.85,
    },
    LexiconEntry {
        pattern: "healing",
        x: 0.65,
        y: 0.1,
        z: 0.85,
    },
    LexiconEntry {
        pattern: "restauración",
        x: 0.6,
        y: 0.15,
        z: 0.8,
    },
    LexiconEntry {
        pattern: "restoration",
        x: 0.6,
        y: 0.15,
        z: 0.8,
    },
    // Knowledge & Wisdom
    LexiconEntry {
        pattern: "conocimiento",
        x: 0.7,
        y: 0.1,
        z: 0.8,
    },
    LexiconEntry {
        pattern: "knowledge",
        x: 0.7,
        y: 0.1,
        z: 0.8,
    },
    LexiconEntry {
        pattern: "sabiduría",
        x: 0.65,
        y: 0.05,
        z: 0.9,
    },
    LexiconEntry {
        pattern: "wisdom",
        x: 0.65,
        y: 0.05,
        z: 0.9,
    },
];

/// Lower Focus lexicon — fear, scarcity, division, manipulation patterns.
const LOWER_LEXICON: &[LexiconEntry] = &[
    // Fear & Threat
    LexiconEntry {
        pattern: "miedo",
        x: 0.2,
        y: 0.8,
        z: -0.85,
    },
    LexiconEntry {
        pattern: "fear",
        x: 0.2,
        y: 0.8,
        z: -0.85,
    },
    LexiconEntry {
        pattern: "amenaza",
        x: 0.15,
        y: 0.85,
        z: -0.9,
    },
    LexiconEntry {
        pattern: "threat",
        x: 0.15,
        y: 0.85,
        z: -0.9,
    },
    LexiconEntry {
        pattern: "peligro",
        x: 0.2,
        y: 0.8,
        z: -0.8,
    },
    LexiconEntry {
        pattern: "danger",
        x: 0.2,
        y: 0.8,
        z: -0.8,
    },
    // Scarcity & Lack
    LexiconEntry {
        pattern: "escasez",
        x: 0.15,
        y: 0.9,
        z: -0.85,
    },
    LexiconEntry {
        pattern: "scarcity",
        x: 0.15,
        y: 0.9,
        z: -0.85,
    },
    LexiconEntry {
        pattern: "falta",
        x: 0.2,
        y: 0.75,
        z: -0.7,
    },
    LexiconEntry {
        pattern: "lack",
        x: 0.2,
        y: 0.75,
        z: -0.7,
    },
    LexiconEntry {
        pattern: "pobreza",
        x: 0.1,
        y: 0.9,
        z: -0.8,
    },
    LexiconEntry {
        pattern: "poverty",
        x: 0.1,
        y: 0.9,
        z: -0.8,
    },
    // Division & Conflict
    LexiconEntry {
        pattern: "división",
        x: 0.2,
        y: 0.8,
        z: -0.85,
    },
    LexiconEntry {
        pattern: "division",
        x: 0.2,
        y: 0.8,
        z: -0.85,
    },
    LexiconEntry {
        pattern: "conflicto",
        x: 0.15,
        y: 0.85,
        z: -0.8,
    },
    LexiconEntry {
        pattern: "conflict",
        x: 0.15,
        y: 0.85,
        z: -0.8,
    },
    LexiconEntry {
        pattern: "oponente",
        x: 0.1,
        y: 0.9,
        z: -0.9,
    },
    LexiconEntry {
        pattern: "enemy",
        x: 0.1,
        y: 0.9,
        z: -0.9,
    },
    // Urgency & Panic (without purpose)
    LexiconEntry {
        pattern: "urgente",
        x: 0.3,
        y: 0.7,
        z: -0.5,
    },
    LexiconEntry {
        pattern: "urgent",
        x: 0.3,
        y: 0.7,
        z: -0.5,
    },
    LexiconEntry {
        pattern: "pánico",
        x: 0.1,
        y: 0.9,
        z: -0.9,
    },
    LexiconEntry {
        pattern: "panic",
        x: 0.1,
        y: 0.9,
        z: -0.9,
    },
    // Control & Domination
    LexiconEntry {
        pattern: "controlar",
        x: 0.3,
        y: 0.75,
        z: -0.7,
    },
    LexiconEntry {
        pattern: "control",
        x: 0.3,
        y: 0.75,
        z: -0.7,
    },
    LexiconEntry {
        pattern: "supremacía",
        x: 0.2,
        y: 0.85,
        z: -0.85,
    },
    LexiconEntry {
        pattern: "dominate",
        x: 0.2,
        y: 0.85,
        z: -0.85,
    },
    // Deception & Manipulation
    LexiconEntry {
        pattern: "engaño",
        x: 0.15,
        y: 0.9,
        z: -0.9,
    },
    LexiconEntry {
        pattern: "deception",
        x: 0.15,
        y: 0.9,
        z: -0.9,
    },
    LexiconEntry {
        pattern: "manipulación",
        x: 0.1,
        y: 0.95,
        z: -0.95,
    },
    LexiconEntry {
        pattern: "manipulation",
        x: 0.1,
        y: 0.95,
        z: -0.95,
    },
    LexiconEntry {
        pattern: "mentira",
        x: 0.1,
        y: 0.9,
        z: -0.9,
    },
    LexiconEntry {
        pattern: "lie",
        x: 0.1,
        y: 0.9,
        z: -0.9,
    },
];

/// The Morphic Resonance Decoder — Maps text to the Stuartian Moral Manifold.
///
/// Processes natural language input through semantic waveform analysis,
/// detecting the underlying intent topology rather than isolated keywords.
#[derive(Debug, Clone)]
pub struct MorphicResonanceDecoder {
    config: DecoderConfig,
}

impl MorphicResonanceDecoder {
    /// Create a new decoder with default configuration.
    pub fn new() -> Self {
        Self {
            config: DecoderConfig::default(),
        }
    }

    /// Create a decoder with custom configuration.
    pub fn with_config(config: DecoderConfig) -> Self {
        Self { config }
    }

    /// Get the current configuration.
    pub fn config(&self) -> &DecoderConfig {
        &self.config
    }

    /// Decode text into a semantic waveform on the Moral Manifold.
    ///
    /// This is the core analysis function:
    /// 1. Tokenize input into lowercase words.
    /// 2. Match tokens against Upper/Lower lexicon.
    /// 3. Apply contextual adjacency weighting (topology of meaning).
    /// 4. Aggregate into composite waveform.
    /// 5. Classify intent based on Z-score.
    ///
    /// Returns `Ok(SemanticWaveform)` on success, or `MorphicError` if input
    /// is empty or contains no semantic content.
    pub fn decode(&self, text: &str) -> Result<SemanticWaveform, MorphicError> {
        let cleaned = text.trim();
        if cleaned.is_empty() {
            return Err(MorphicError::EmptyInput);
        }

        let tokens: Vec<&str> = cleaned
            .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
            .filter(|t| !t.is_empty())
            .collect();

        if tokens.is_empty() {
            return Err(MorphicError::EmptyInput);
        }

        let limited_tokens: Vec<&str> = tokens.into_iter().take(self.config.max_tokens).collect();

        let lowercase: Vec<String> = limited_tokens.iter().map(|t| t.to_lowercase()).collect();

        // Phase 1: Token-level lexicon matching
        let mut matched_x = 0.0f64;
        let mut matched_y = 0.0f64;
        let mut matched_z = 0.0f64;
        let mut match_count = 0usize;

        for token in &lowercase {
            if let Some(entry) = Self::lookup_lexicon(token.as_str()) {
                matched_x += entry.x;
                matched_y += entry.y;
                matched_z += entry.z;
                match_count += 1;
            }
        }

        // Phase 2: Topology analysis (phrase-level patterns)
        let topology_bonus = if self.config.enable_topology {
            self.analyze_topology(&lowercase)
        } else {
            0.0
        };

        // Phase 3: Contextual adjacency weighting
        let context_adjustment = self.calculate_context_weight(&lowercase);

        let _total_tokens = lowercase.len().max(1) as f64;

        // Normalize matched values by token count
        let avg_x = if match_count > 0 {
            matched_x / match_count as f64
        } else {
            0.5 // Neutral default
        };

        let avg_y = if match_count > 0 {
            matched_y / match_count as f64
        } else {
            0.5 // Neutral default
        };

        let avg_z = if match_count > 0 {
            matched_z / match_count as f64
        } else {
            0.0 // Neutral default
        };

        // Apply topology and context adjustments
        let cw = self.config.context_weight;
        let final_x = avg_x * (1.0 - cw) + context_adjustment.0 * cw;
        let final_y = avg_y * (1.0 - cw) + context_adjustment.1 * cw;
        let final_z = (avg_z + topology_bonus).clamp(-1.0, 1.0);

        // Calculate Z-score: weighted composite of all three axes
        // Positive Z = Upper Focus, Negative Z = Lower Focus
        // Centered formula: high Y (extraction) and low X (low autonomy) contribute negatively
        let z_score = final_z * 0.5 + (0.5 - final_y) * 0.3 + (final_x - 0.5) * 0.2;

        SemanticWaveform::new(final_x, final_y, final_z, z_score, limited_tokens.len())
    }

    /// Lookup a token in the resonance lexicon.
    ///
    /// Searches both Upper and Lower lexicons, returning the first match.
    /// Upper lexicon is checked first to prioritize constructive patterns.
    fn lookup_lexicon(token: &str) -> Option<LexiconEntry> {
        UPPER_LEXICON
            .iter()
            .chain(LOWER_LEXICON.iter())
            .find(|e| token.contains(e.pattern) || e.pattern.contains(token))
            .cloned()
    }

    /// Analyze phrase-level topology for hidden intent patterns.
    ///
    /// Detects multi-word patterns that indicate Lower Focus intent
    /// even when individual tokens appear neutral:
    /// - "we must" + fear words → urgency manipulation
    /// - "they say" + negative → division framing
    /// - "only way" + action → false scarcity
    fn analyze_topology(&self, tokens: &[String]) -> f64 {
        let mut bonus = 0.0f64;

        // Detect "us vs them" framing patterns
        let text = tokens.join(" ");

        // Division patterns: "ellos dicen", "nosotros contra", "ellos vs nosotros"
        if Self::contains_pattern(&text, &["ellos", "nosotros"])
            || Self::contains_pattern(&text, &["they", "us"])
            || Self::contains_pattern(&text, &["they", "we"])
        {
            bonus -= 0.1;
        }

        // False urgency: "debes", "urgente", "ahora mismo" + fear/scarcity
        if Self::contains_pattern(&text, &["debes", "ahora"])
            || Self::contains_pattern(&text, &["must", "now"])
            || Self::contains_pattern(&text, &["urgent", "act"])
        {
            // Check if paired with negative sentiment
            let has_negative = LOWER_LEXICON.iter().any(|e| text.contains(e.pattern));
            if has_negative {
                bonus -= 0.15;
            }
        }

        // False scarcity: "única forma", "última oportunidad", "only way", "last chance"
        if Self::contains_pattern(&text, &["única", "forma"])
            || Self::contains_pattern(&text, &["only", "way"])
            || Self::contains_pattern(&text, &["última", "oportunidad"])
            || Self::contains_pattern(&text, &["last", "chance"])
        {
            bonus -= 0.12;
        }

        // Constructive patterns: "juntos", "construir", "crear", "together", "build"
        if Self::contains_pattern(&text, &["juntos", "construir"])
            || Self::contains_pattern(&text, &["together", "build"])
            || Self::contains_pattern(&text, &["cooperación", "futuro"])
            || Self::contains_pattern(&text, &["cooperation", "future"])
        {
            bonus += 0.1;
        }

        // Knowledge-seeking patterns: "cómo", "por qué", "entender", "how", "why", "understand"
        if Self::contains_pattern(&text, &["cómo", "hacer"])
            || Self::contains_pattern(&text, &["how", "to"])
            || Self::contains_pattern(&text, &["por qué", "funciona"])
            || Self::contains_pattern(&text, &["why", "does"])
        {
            bonus += 0.08;
        }

        bonus
    }

    /// Check if text contains all patterns in the list (adjacency detection).
    fn contains_pattern(text: &str, patterns: &[&str]) -> bool {
        patterns.iter().all(|p| text.contains(p))
    }

    /// Calculate contextual adjacency weight.
    ///
    /// Analyzes the distribution of positive vs negative tokens
    /// to detect clustering patterns. A cluster of negative tokens
    /// has more impact than scattered individual tokens.
    fn calculate_context_weight(&self, tokens: &[String]) -> (f64, f64, f64) {
        let mut scores: Vec<(f64, f64, f64)> = Vec::new();

        for token in tokens {
            if let Some(entry) = Self::lookup_lexicon(token) {
                scores.push((entry.x, entry.y, entry.z));
            }
        }

        if scores.is_empty() {
            return (0.5, 0.5, 0.0);
        }

        // Calculate clustering: consecutive same-sign tokens amplify effect
        let mut cluster_x = 0.0f64;
        let mut cluster_y = 0.0f64;
        let mut cluster_z = 0.0f64;
        let mut consecutive_positive = 0;
        let mut consecutive_negative = 0;

        for (x, y, z) in &scores {
            if *z > 0.0 {
                consecutive_positive += 1;
                consecutive_negative = 0;
                let amplifier = 1.0 + (consecutive_positive as f64) * 0.05;
                cluster_x += x * amplifier;
                cluster_y += y * amplifier;
                cluster_z += z * amplifier;
            } else if *z < 0.0 {
                consecutive_negative += 1;
                consecutive_positive = 0;
                let amplifier = 1.0 + (consecutive_negative as f64) * 0.05;
                cluster_x += x * amplifier;
                cluster_y += y * amplifier;
                cluster_z += z * amplifier;
            } else {
                consecutive_positive = 0;
                consecutive_negative = 0;
                cluster_x += x;
                cluster_y += y;
                cluster_z += z;
            }
        }

        let count = scores.len() as f64;
        (cluster_x / count, cluster_y / count, cluster_z / count)
    }
}

impl Default for MorphicResonanceDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoder_creation() {
        let decoder = MorphicResonanceDecoder::new();
        assert_eq!(decoder.config().upper_threshold, 0.10);
        assert_eq!(decoder.config().lower_threshold, -0.10);
    }

    #[test]
    fn test_decoder_custom_config() {
        let config = DecoderConfig {
            upper_threshold: 0.2,
            lower_threshold: -0.2,
            ..DecoderConfig::default()
        };
        let decoder = MorphicResonanceDecoder::with_config(config);
        assert_eq!(decoder.config().upper_threshold, 0.2);
    }

    #[test]
    fn test_decode_empty_input() {
        let decoder = MorphicResonanceDecoder::new();
        match decoder.decode("") {
            Err(MorphicError::EmptyInput) => {}
            other => panic!("Expected EmptyInput, got {:?}", other),
        }
    }

    #[test]
    fn test_decode_whitespace_only() {
        let decoder = MorphicResonanceDecoder::new();
        match decoder.decode("   ") {
            Err(MorphicError::EmptyInput) => {}
            other => panic!("Expected EmptyInput, got {:?}", other),
        }
    }

    #[test]
    fn test_upper_focus_cooperation() {
        let decoder = MorphicResonanceDecoder::new();
        let waveform = decoder
            .decode("cooperación y armonía para la evolución")
            .unwrap();
        assert_eq!(waveform.intent, IntentClassification::UpperFocus);
        assert!(waveform.z_score > 0.0);
        assert!(waveform.is_constructive());
    }

    #[test]
    fn test_upper_focus_symbiosis() {
        let decoder = MorphicResonanceDecoder::new();
        let waveform = decoder
            .decode("simbiosis y resonancia para la preservación")
            .unwrap();
        assert_eq!(waveform.intent, IntentClassification::UpperFocus);
        assert!(waveform.z > 0.5);
    }

    #[test]
    fn test_lower_focus_fear() {
        let decoder = MorphicResonanceDecoder::new();
        let waveform = decoder.decode("miedo y amenaza de escasez").unwrap();
        assert_eq!(waveform.intent, IntentClassification::LowerFocus);
        assert!(waveform.z_score < 0.0);
        assert!(waveform.is_manipulative());
    }

    #[test]
    fn test_lower_focus_division() {
        let decoder = MorphicResonanceDecoder::new();
        let waveform = decoder
            .decode("división y conflicto entre oponente")
            .unwrap();
        assert_eq!(waveform.intent, IntentClassification::LowerFocus);
        assert!(waveform.y > 0.5);
    }

    #[test]
    fn test_neutral_mixed() {
        let decoder = MorphicResonanceDecoder::new();
        // Mixed text with both positive and negative patterns
        let waveform = decoder
            .decode("el miedo puede llevar a la cooperación")
            .unwrap();
        // Should be neutral or slightly negative due to mixed signals
        assert!(
            waveform.intent == IntentClassification::Neutral
                || waveform.intent == IntentClassification::LowerFocus
        );
    }

    #[test]
    fn test_topology_us_vs_them() {
        let decoder = MorphicResonanceDecoder::new();
        let waveform = decoder
            .decode("ellos contra nosotros en esta división")
            .unwrap();
        // Topology should detect division pattern
        assert!(waveform.intent == IntentClassification::LowerFocus || waveform.z_score < 0.0);
    }

    #[test]
    fn test_topology_constructive() {
        let decoder = MorphicResonanceDecoder::new();
        let waveform = decoder
            .decode("juntos construir un futuro de cooperación")
            .unwrap();
        assert!(waveform.intent == IntentClassification::UpperFocus);
    }

    #[test]
    fn test_topology_false_scarcity() {
        let decoder = MorphicResonanceDecoder::new();
        let waveform = decoder
            .decode("última oportunidad antes del peligro")
            .unwrap();
        // Should detect false scarcity pattern
        assert!(waveform.z_score < 0.1);
    }

    #[test]
    fn test_waveform_clamping() {
        let waveform = SemanticWaveform::new(1.5, -0.5, 2.0, 0.5, 10).unwrap();
        assert!(waveform.x <= 1.0);
        assert!(waveform.y >= 0.0);
        assert!(waveform.z <= 1.0);
    }

    #[test]
    fn test_waveform_zero_tokens_rejected() {
        match SemanticWaveform::new(0.5, 0.5, 0.0, 0.0, 0) {
            Err(MorphicError::EmptyInput) => {}
            other => panic!("Expected EmptyInput, got {:?}", other),
        }
    }

    #[test]
    fn test_intent_classification_thresholds() {
        // Upper Focus
        let wf = SemanticWaveform::new(0.7, 0.2, 0.8, 0.2, 5).unwrap();
        assert_eq!(wf.intent, IntentClassification::UpperFocus);

        // Lower Focus
        let wf = SemanticWaveform::new(0.2, 0.8, -0.7, -0.3, 5).unwrap();
        assert_eq!(wf.intent, IntentClassification::LowerFocus);

        // Neutral
        let wf = SemanticWaveform::new(0.5, 0.5, 0.0, 0.05, 5).unwrap();
        assert_eq!(wf.intent, IntentClassification::Neutral);
    }

    #[test]
    fn test_is_constructive() {
        let wf = SemanticWaveform::new(0.7, 0.2, 0.8, 0.3, 5).unwrap();
        assert!(wf.is_constructive());
        assert!(!wf.is_manipulative());
    }

    #[test]
    fn test_is_manipulative() {
        let wf = SemanticWaveform::new(0.2, 0.8, -0.7, -0.4, 5).unwrap();
        assert!(wf.is_manipulative());
        assert!(!wf.is_constructive());
    }

    #[test]
    fn test_english_upper_focus() {
        let decoder = MorphicResonanceDecoder::new();
        let waveform = decoder
            .decode("cooperation and harmony for evolution")
            .unwrap();
        assert_eq!(waveform.intent, IntentClassification::UpperFocus);
    }

    #[test]
    fn test_english_lower_focus() {
        let decoder = MorphicResonanceDecoder::new();
        let waveform = decoder.decode("fear and threat of scarcity").unwrap();
        assert_eq!(waveform.intent, IntentClassification::LowerFocus);
    }

    #[test]
    fn test_knowledge_seeking_positive() {
        let decoder = MorphicResonanceDecoder::new();
        let waveform = decoder.decode("cómo hacer para entender mejor").unwrap();
        // Knowledge-seeking should get topology bonus
        assert!(waveform.z_score > -0.1);
    }

    #[test]
    fn test_decoder_default() {
        let decoder = MorphicResonanceDecoder::default();
        assert_eq!(decoder.config().upper_threshold, 0.10);
    }

    #[test]
    fn test_error_display() {
        assert_eq!(
            format!("{}", MorphicError::EmptyInput),
            "MorphicError: empty or non-semantic input"
        );
        assert_eq!(
            format!("{}", MorphicError::PureLowerFocus),
            "MorphicError: input contains exclusively Lower Focus patterns"
        );
        assert_eq!(
            format!("{}", MorphicError::ComputationError("test".to_string())),
            "MorphicError: computation failed — test"
        );
    }

    #[test]
    fn test_intent_display() {
        assert_eq!(
            format!("{}", IntentClassification::UpperFocus),
            "UpperFocus"
        );
        assert_eq!(
            format!("{}", IntentClassification::LowerFocus),
            "LowerFocus"
        );
        assert_eq!(format!("{}", IntentClassification::Neutral), "Neutral");
    }

    #[test]
    fn test_max_tokens_limit() {
        let config = DecoderConfig {
            max_tokens: 10,
            ..DecoderConfig::default()
        };
        let decoder = MorphicResonanceDecoder::with_config(config);
        // Long text should be limited to 10 tokens
        let long_text = "cooperación cooperación cooperación cooperación cooperación \
                         cooperación cooperación cooperación cooperación cooperación \
                         cooperación cooperación cooperación cooperación";
        let waveform = decoder.decode(long_text).unwrap();
        assert_eq!(waveform.token_count, 10);
    }

    #[test]
    fn test_topology_disabled() {
        let config = DecoderConfig {
            enable_topology: false,
            ..DecoderConfig::default()
        };
        let decoder = MorphicResonanceDecoder::with_config(config);
        let waveform = decoder.decode("juntos construir un futuro").unwrap();
        // Without topology, should still work but without phrase bonuses
        assert!(waveform.token_count > 0);
    }
}
