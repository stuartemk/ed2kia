//! Security Module — Sprint 70: Civilization-Scale Architecture
//!
//! Anti-capture mechanisms including geo-diversity weighting,
//! anti-Sybil detection, and chaos engineering fault injection.

pub mod anti_capture;

pub use anti_capture::{AntiCapture, CaptureConfig, CaptureError, NodeRisk};
