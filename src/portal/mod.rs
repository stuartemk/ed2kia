//! Symbiotic Portal â€” Zero-Friction Onboarding via WASM Client.
//!
//! **Topological Law 4 (Simbiosis):** Portal simbiÃ³tico para integraciÃ³n armoniosa humano-red.
//! **Topological Law 3 (Eficiencia):** Cero fricciÃ³n â€” Web Worker isolation, async message bridge.
//!
//! ### Feature Gates
//! | Feature | MÃ³dulo | DescripciÃ³n |
//! |---|---|---|
//! | `v3.7-symbiotic-portal` | wasm_client | SymbioticPortal + Web Worker bridge for OmniNode in browser |
//! | `v3.7-symbiotic-portal` | ui_bridge | CE Wallet + Dashboard bindings (Alpine.js/Vanilla.js) |
//! | `v3.8-morphic-genesis` | morphic_bridge | MorphicBridge â€” Connects MRD + Purifier to SymbioticPortal |

#[cfg(target_arch = "wasm32")]
pub mod wasm_client;

#[cfg(target_arch = "wasm32")]
pub mod ui_bridge;

#[cfg(feature = "v3.8-morphic-genesis")]
pub mod morphic_bridge;
