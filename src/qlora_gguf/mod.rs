//! QLoRA/GGUF Module — Quantized LoRA adapters over GGUF base models.
//!
//! **Stuartian Law 3 (Inteligencia Holística):** Cero desperdicio computacional.
//! Los modelos base GGUF son inmutables; solo se distribuyen diffs QLoRA en KB/MB.
//!
//! **Feature Gate:** `v2.1-qlora-gguf`

pub mod adapter;
pub mod loader;
pub mod payload;

pub use adapter::{QloraAdapter, QloraAdapterError};
pub use loader::{GgufLoader, GgufLoaderError};
pub use payload::{QloraPayload, QloraPayloadError};
