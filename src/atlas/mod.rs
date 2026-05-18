//! Atlas Semántico Global — Piedra Rosetta
//!
//! Semantic translation layer between SAE features and natural language tokens.
//!
//! - **graph**: In-memory semantic graph (`petgraph` + `dashmap`)
//! - **api**: Rosetta API (`axum` endpoints)
//! - **ui**: 3D visualizer (`web/atlas-visualizer.js`)

#[cfg(feature = "v2.1-semantic-graph")]
pub mod graph;

#[cfg(feature = "v2.1-rosetta-api")]
pub mod api;

#[cfg(feature = "v2.1-atlas-ui")]
pub mod ui {
    /// Placeholder for UI integration hooks.
    /// The actual visualizer lives in `web/atlas-visualizer.js`.
    pub const ATLAS_CANVAS_ID: &str = "atlas-canvas";
    pub const ATLAS_API_BASE_ENV: &str = "ATLAS_API_BASE";
}
