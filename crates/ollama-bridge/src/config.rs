//! Bridge configuration — Ollama/LM Studio proxy settings.

use serde::{Deserialize, Serialize};

/// Bridge server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    /// Address to bind the bridge HTTP server (e.g., "127.0.0.1:11435").
    pub bind_address: String,
    /// Upstream Ollama/LM Studio endpoint (e.g., "http://127.0.0.1:11434").
    pub upstream_url: String,
    /// Enable TCM Z-axis computation on every response.
    pub tcm_enabled: bool,
    /// TCM Z-axis: centroid mean (μ).
    pub tcm_mu_centroid: f64,
    /// TCM Z-axis: spread σ.
    pub tcm_sigma_spread: f64,
    /// Maximum request body size in bytes (default 16 MiB).
    pub max_body_size: usize,
    /// Request timeout in seconds (default 120).
    pub timeout_secs: u64,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:11435".to_string(),
            upstream_url: "http://127.0.0.1:11434".to_string(),
            tcm_enabled: true,
            tcm_mu_centroid: 0.0,
            tcm_sigma_spread: 1.0,
            max_body_size: 16 * 1024 * 1024, // 16 MiB
            timeout_secs: 120,
        }
    }
}

impl BridgeConfig {
    /// Load configuration from environment variables.
    ///
    /// Supported env vars:
    /// - `ED2K_BRIDGE_BIND` — bind address
    /// - `ED2K_BRIDGE_UPSTREAM` — upstream Ollama URL
    /// - `ED2K_BRIDGE_TCM_ENABLED` — "true"/"false"
    /// - `ED2K_BRIDGE_TCM_MU` — centroid mean
    /// - `ED2K_BRIDGE_TCM_SIGMA` — spread
    /// - `ED2K_BRIDGE_TIMEOUT` — timeout in seconds
    pub fn from_env() -> Self {
        use std::env;
        Self {
            bind_address: env::var("ED2K_BRIDGE_BIND").unwrap_or_default("127.0.0.1:11435".to_string()),
            upstream_url: env::var("ED2K_BRIDGE_UPSTREAM").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string()),
            tcm_enabled: env::var("ED2K_BRIDGE_TCM_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
            tcm_mu_centroid: env::var("ED2K_BRIDGE_TCM_MU")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0),
            tcm_sigma_spread: env::var("ED2K_BRIDGE_TCM_SIGMA")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1.0),
            timeout_secs: env::var("ED2K_BRIDGE_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(120),
            ..Default::default()
        }
    }
}
