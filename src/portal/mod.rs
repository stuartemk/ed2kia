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

pub mod wasm_client;
pub mod ui_bridge;
