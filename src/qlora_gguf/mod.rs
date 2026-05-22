//! QLoRA/GGUF Module — Quantized LoRA adapters over GGUF base models.
//!
//! **Stuartian Law 3 (Inteligencia Holística):** Cero desperdicio computacional.
//! Los modelos base GGUF son inmutables; solo se distribuyen diffs QLoRA en KB/MB.
//!
//! **Feature Gate:** `v2.1-qlora-gguf`
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  GGUF Base Model (inmutable, memory-mapped)                │
//! │  ┌───────────────────────────────────────────────────────┐  │
//! │  │  model.gguf  →  SHA256  →  GgufBaseModel             │  │
//! │  └───────────────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//!                            ↓
//! ┌─────────────────────────────────────────────────────────────┐
//! │  QLoRA Adapter (W' = W + B @ A)                            │
//! │  ┌───────────────────────────────────────────────────────┐  │
//! │  │  A: (d_model × r)  B: (r × d_model)  α: scale       │  │
//! │  └───────────────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//!                            ↓
//! ┌─────────────────────────────────────────────────────────────┐
//! │  QLoRA Payload (zstd compressed, bincode serialized)       │
//! │  ┌───────────────────────────────────────────────────────┐  │
//! │  │  bincode → zstd → GossipSub (≤1MB)                   │  │
//! │  └───────────────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//! ```

pub mod adapter;
pub mod loader;
pub mod payload;

// Loader exports
pub use loader::{GgufLoader, GgufLoaderError, GgufModelInfo};

#[cfg(feature = "v2.1-qlora-gguf")]
pub use loader::GgufBaseModel;

// Adapter exports
pub use adapter::{AdapterInfo, QloraAdapter, QloraAdapterError, QuantizationType};

// Payload exports
pub use payload::{QloraPayload, QloraPayloadError, MAX_PAYLOAD_BYTES};
