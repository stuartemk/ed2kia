//! Symbiotic Portal — Zero-Friction Onboarding via WASM Client.
//!
//! **Stuartian Law 4 (Simbiosis):** Portal simbiótico para integración armoniosa humano-red.
//! **Stuartian Law 3 (Eficiencia):** Cero fricción — Web Worker isolation, async message bridge.
//!
//! ### Feature Gates
//! | Feature | Módulo | Descripción |
//! |---|---|---|
//! | `v3.7-symbiotic-portal` | wasm_client | SymbioticPortal + Web Worker bridge for OmniNode in browser |
//! | `v3.7-symbiotic-portal` | ui_bridge | CE Wallet + Dashboard bindings (Alpine.js/Vanilla.js) |
//! | `v3.8-morphic-genesis` | morphic_bridge | MorphicBridge — Connects MRD + Purifier to SymbioticPortal |

#[cfg(target_arch = "wasm32")]
pub mod wasm_client;

#[cfg(target_arch = "wasm32")]
pub mod ui_bridge;

#[cfg(feature = "v3.8-morphic-genesis")]
pub mod morphic_bridge;
