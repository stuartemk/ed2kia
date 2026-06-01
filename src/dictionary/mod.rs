//! Universal Feature Dictionary — Sprint 70: Civilization-Scale Architecture
//!
//! Cross-model feature merging via FedAvg weighted by CE×SCT-Z,
//! contrastive disentanglement to prevent feature collapse,
//! and Lyapunov stability verification.

pub mod universal_feature_dict;

pub use universal_feature_dict::{FeatureEntry, MergeConfig, MergeError, UniversalFeatureDict};
