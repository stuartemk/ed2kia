//! Dataset Module — Public dataset loading with streaming, chunking, and validation.
//!
//! This module provides infrastructure for loading public datasets (.jsonl/.parquet)
//! with SHA256 validation per chunk and fallback to dummy datasets.

pub mod public_loader;
