//! Bridge Configuration — Sprint 88: The Reality Engine & Empirical Proof Core
//!
//! Supports llama.cpp OpenAI-compatible API (`localhost:8080/v1/chat/completions`).

/// Configuration for the llama.cpp bridge.
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    /// Bind address for the bridge HTTP server.
    pub bind_address: String,
    /// Upstream llama.cpp server URL (OpenAI-compatible API).
    pub upstream_url: String,
    /// Whether TCM Z-axis computation is enabled.
    pub tcm_enabled: bool,
    /// TCM centroid mean for Z-axis calculation.
    pub tcm_mu_centroid: f64,
    /// TCM spread standard deviation for Z-axis calculation.
    pub tcm_sigma_spread: f64,
    /// Maximum request body size in bytes.
    pub max_body_size: usize,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            bind_address: std::env::var("ED2K_BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:3000".into()),
            upstream_url: std::env::var("ED2K_UPSTREAM_URL").unwrap_or_else(|_| "http://localhost:8080/v1/chat/completions".into()),
            tcm_enabled: std::env::var("ED2K_TCM_ENABLED").map_or(true, |v| v == "1" || v == "true"),
            tcm_mu_centroid: std::env::var("ED2K_TCM_MU").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0),
            tcm_sigma_spread: std::env::var("ED2K_TCM_SIGMA").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0),
            max_body_size: std::env::var("ED2K_MAX_BODY_SIZE").ok().and_then(|v| v.parse().ok()).unwrap_or(10 * 1024 * 1024),
            timeout_secs: std::env::var("ED2K_TIMEOUT_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(60),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BridgeConfig::default();
        assert_eq!(config.bind_address, "0.0.0.0:3000");
        assert!(config.upstream_url.contains("8080"));
        assert!(config.tcm_enabled);
    }

    #[test]
    fn test_config_clone() {
        let config = BridgeConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.bind_address, config.bind_address);
    }
}
