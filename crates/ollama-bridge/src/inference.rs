//! Inference proxy — Intercepts Ollama `/api/generate` and `/api/chat` endpoints,
//! forwards to upstream, injects TCM Z-axis + SAE audit metrics into response.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::config::BridgeConfig;
use crate::sae_audit::{SaeAuditor, SaeAuditResult};
use crate::tcm::TcmMetric;

/// Request forwarded to upstream Ollama/LM Studio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub model: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<HashMap<String, serde_json::Value>>,
}

/// Response from upstream Ollama/LM Studio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateResponse {
    pub model: String,
    pub response: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_duration: Option<u64>,
}

/// Enriched response with ed2kIA SAE audit + TCM Z-axis metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedResponse {
    /// Original Ollama response.
    pub ollama: GenerateResponse,
    /// SAE audit metrics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sae_audit: Option<SaeAuditResult>,
    /// TCM Z-axis metric.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tcm: Option<TcmMetric>,
    /// Bridge metadata.
    pub bridge: BridgeMetadata,
}

/// Metadata about the bridge processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeMetadata {
    pub version: String,
    pub tcm_enabled: bool,
    pub sae_enabled: bool,
    pub upstream_url: String,
}

/// The inference proxy engine.
pub struct InferenceProxy {
    pub config: BridgeConfig,
    pub auditor: SaeAuditor,
    pub client: reqwest::Client,
}

impl InferenceProxy {
    pub fn new(config: BridgeConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .unwrap_or_else(|_| {
                reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(120))
                    .build()
                    .expect("Failed to create HTTP client")
            });
        Self {
            auditor: SaeAuditor::new(4096), // Default SAE width
            config,
            client,
        }
    }

    /// Forward a generate request to upstream Ollama and enrich the response.
    pub async fn generate(
        &self,
        req: &GenerateRequest,
    ) -> Result<EnrichedResponse, BridgeError> {
        let upstream_url = format!("{}/api/generate", self.config.upstream_url);

        let resp = self
            .client
            .post(&upstream_url)
            .json(req)
            .send()
            .await
            .map_err(|e| BridgeError::UpstreamError(e.to_string()))?;

        let status = resp.status();
        let body = resp
            .bytes()
            .await
            .map_err(|e| BridgeError::UpstreamError(e.to_string()))?;

        if !status.is_success() {
            return Err(BridgeError::UpstreamError(format!(
                "HTTP {}: {}",
                status,
                String::from_utf8_lossy(&body)
            )));
        }

        let ollama_resp: GenerateResponse =
            serde_json::from_slice(&body).map_err(|e| BridgeError::ParseError(e.to_string()))?;

        // Compute synthetic SAE activations from response token count.
        let activations = self.simulate_activations(&ollama_resp);
        let sae_audit = self.auditor.audit(&activations);

        // Compute TCM Z-axis.
        let tcm = if self.config.tcm_enabled {
            Some(TcmMetric::compute(
                sae_audit.mean_activation,
                self.config.tcm_mu_centroid,
                self.config.tcm_sigma_spread,
            ))
        } else {
            None
        };

        Ok(EnrichedResponse {
            ollama: ollama_resp,
            sae_audit: Some(sae_audit),
            tcm,
            bridge: BridgeMetadata {
                version: env!("CARGO_PKG_VERSION").to_string(),
                tcm_enabled: self.config.tcm_enabled,
                sae_enabled: true,
                upstream_url: self.config.upstream_url.clone(),
            },
        })
    }

    /// Simulate SAE activations from response characteristics.
    ///
    /// In production, this would interface with actual SAE inference.
    fn simulate_activations(&self, resp: &GenerateResponse) -> Vec<f64> {
        let n = self.auditor.total_neurons;
        let token_count = resp.eval_count.unwrap_or(1) as f64;
        // Use token count to modulate activation sparsity.
        let sparsity_factor = (token_count / 100.0).min(1.0);
        let mut activations = vec![0.0; n];
        let active_count = (n as f64 * sparsity_factor * 0.1) as usize;
        for i in 0..active_count {
            activations[i] = (i as f64 + 1.0) * sparsity_factor;
        }
        activations
    }
}

/// Bridge errors.
#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("Upstream error: {0}")]
    UpstreamError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Config error: {0}")]
    ConfigError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_creation() {
        let config = BridgeConfig::default();
        let proxy = InferenceProxy::new(config);
        assert_eq!(proxy.auditor.total_neurons, 4096);
    }

    #[test]
    fn test_simulate_activations() {
        let config = BridgeConfig::default();
        let proxy = InferenceProxy::new(config);
        let resp = GenerateResponse {
            model: "test".to_string(),
            response: "hello".to_string(),
            done: Some(true),
            total_duration: None,
            load_duration: None,
            prompt_eval_count: None,
            prompt_eval_duration: None,
            eval_count: Some(50),
            eval_duration: None,
        };
        let activations = proxy.simulate_activations(&resp);
        assert_eq!(activations.len(), 4096);
        let active = activations.iter().filter(|&&a| a > 0.0).count();
        assert!(active > 0);
    }
}
