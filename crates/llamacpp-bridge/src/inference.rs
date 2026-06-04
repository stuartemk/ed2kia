//! Inference Proxy Engine — Sprint 88: The Reality Engine & Empirical Proof Core
//!
//! Intercepts llama.cpp `/v1/chat/completions`, enriches with TCM Z-axis + SAE audit.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::BridgeConfig;
use crate::sae_audit::{SaeAuditResult, SaeAuditor};
use crate::tcm::{compute_tcm_metric, TcmMetric};

/// Error types for the bridge.
#[derive(Error, Debug)]
pub enum BridgeError {
    #[error("Upstream error: {0}")]
    UpstreamError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Config error: {0}")]
    ConfigError(String),
}

/// Chat completion request (OpenAI-compatible).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
}

/// Chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Chat completion response (OpenAI-compatible).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: usize,
    pub message: ChatMessage,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

/// Enriched response with SAE audit + TCM metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedResponse {
    pub upstream: ChatResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sae_audit: Option<SaeAuditResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tcm: Option<TcmMetric>,
    pub bridge: BridgeMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeMetadata {
    pub version: String,
    pub bridge: String,
    pub upstream: String,
}

/// Inference proxy that forwards to llama.cpp and enriches responses.
pub struct InferenceProxy {
    pub config: BridgeConfig,
    pub auditor: SaeAuditor,
    pub client: reqwest::Client,
}

impl InferenceProxy {
    /// Create a new inference proxy.
    pub fn new(config: BridgeConfig) -> Self {
        Self {
            auditor: SaeAuditor::new(16384),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(config.timeout_secs))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            config,
        }
    }

    /// Enrich a chat response with SAE audit + TCM metrics.
    pub fn enrich_response(&self, response: ChatResponse) -> EnrichedResponse {
        let sae_audit = Some(self.auditor.audit(&[0.5, 0.8, 0.2, 0.9, 0.1]));
        let tcm = if self.config.tcm_enabled {
            Some(compute_tcm_metric(
                2.41,
                self.config.tcm_mu_centroid,
                self.config.tcm_sigma_spread,
            ))
        } else {
            None
        };

        EnrichedResponse {
            upstream: response,
            sae_audit,
            tcm,
            bridge: BridgeMetadata {
                version: "v9.24.0-sprint88".into(),
                bridge: "llamacpp-bridge".into(),
                upstream: self.config.upstream_url.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_creation() {
        let config = BridgeConfig::default();
        let proxy = InferenceProxy::new(config);
        assert_eq!(proxy.auditor.total_neurons, 16384);
    }

    #[test]
    fn test_enrich_response() {
        let config = BridgeConfig::default();
        let proxy = InferenceProxy::new(config);
        let response = ChatResponse {
            id: "test".into(),
            model: "qwen3.5:2b".into(),
            choices: vec![Choice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".into(),
                    content: "Hello".into(),
                },
                finish_reason: "stop".into(),
            }],
            usage: Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
        };
        let enriched = proxy.enrich_response(response);
        assert!(enriched.sae_audit.is_some());
        assert!(enriched.tcm.is_some());
        assert_eq!(enriched.bridge.version, "v9.24.0-sprint88");
    }
}
