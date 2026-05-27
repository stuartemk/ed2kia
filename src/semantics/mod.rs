//! Morphic Semantics — Semantic Manipulation Protection for Symbiotic Portal.
//!
//! Provides morphic resonance decoding and semantic purification for detecting
//! and re-aligning natural language inputs toward constructive resonance.
//!
//! **Feature Gate:** `v3.8-morphic-genesis`

#[cfg(feature = "v3.8-morphic-genesis")]
pub mod morphic_decoder;
#[cfg(feature = "v3.8-morphic-genesis")]
pub mod semantic_purifier;

#[cfg(feature = "v3.8-morphic-genesis")]
pub use morphic_decoder::{
    MorphicResonanceDecoder,
    SemanticWaveform,
    IntentClassification,
    DecoderConfig,
    MorphicError,
};
#[cfg(feature = "v3.8-morphic-genesis")]
pub use semantic_purifier::{
    SemanticPurifier,
    PurificationResult,
    PurifierConfig,
    PurificationError,
};
