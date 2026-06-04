//! Consensus Runner â€” EjecuciÃ³n del BFT y SCT para MVP local.
//!
//! Nodo Gamma (Steward) intercepta payloads del topic GossipSub,
//! pasa cada payload por SCTGuard.evaluate_trajectory() y ejecuta
//! BFT aggregation solo con payloads aprobados.
//!
//! Ley 2 (Reconocimiento del Error): Hard Reject cuando Z < 0.
//! Ley 3 (Cero desperdicio): Logs deterministas, mÃ©tricas de latencia.

use std::time::Instant;
use thiserror::Error;

use crate::alignment::sct_core::{SCTDecision, TopologicalTensor};
use crate::alignment::sct_guard::{SctGuard, SctGuardError};
use crate::federated::bft_aggregator::{
    coordinate_wise_median, BftAggregator, BftConfig, BftError,
};
use crate::mvp::sae_simulator::{SaePayload, SaeSimError};

/// Error del consenso MVP.
#[derive(Debug, Error)]
pub enum ConsensusError {
    #[error("SCT Guard error: {0}")]
    SctGuard(#[from] SctGuardError),

    #[error("BFT error: {0}")]
    Bft(#[from] BftError),

    #[error("SAE simulator error: {0}")]
    SaeSim(#[from] SaeSimError),

    #[error("No valid payloads after SCT filtering")]
    NoValidPayloads,

    #[error("Latency exceeded: {elapsed_ms:.0}ms > {limit_ms}ms")]
    LatencyExceeded { elapsed_ms: f64, limit_ms: f64 },
}

/// Resultado de evaluaciÃ³n SCT para un payload.
#[derive(Debug, Clone)]
pub struct SctEvaluation {
    /// ID del nodo.
    pub node_id: String,
    /// Valor Z del tensor SCT.
    pub z_value: f32,
    /// DecisiÃ³n SCT.
    pub decision: SCTDecision,
    /// Â¿Fue aprobado?
    pub approved: bool,
    /// Mensaje de log determinista.
    pub log_message: String,
}

/// MÃ©tricas de consenso.
#[derive(Debug, Clone)]
pub struct ConsensusMetrics {
    /// Payloads totales procesados.
    pub total_payloads: usize,
    /// Payloads aprobados por SCT.
    pub approved_count: usize,
    /// Payloads rechazados por SCT.
    pub rejected_count: usize,
    /// Latencia total en ms.
    pub total_latency_ms: f64,
    /// Latencia promedio en ms.
    pub avg_latency_ms: f64,
    /// Resultado BFT (gradiente agregado).
    pub bft_result: Option<Vec<f32>>,
    /// Evaluaciones individuales.
    pub evaluations: Vec<SctEvaluation>,
}

/// Ejecutor de consenso MVP.
pub struct ConsensusRunner {
    /// Guard SCT para validaciÃ³n Ã©tica.
    sct_guard: SctGuard,
    /// Agregador BFT para consenso.
    bft_aggregator: BftAggregator,
    /// LÃ­mite de latencia en ms.
    latency_limit_ms: f64,
}

impl ConsensusRunner {
    /// Construye un nuevo ConsensusRunner.
    pub fn new(max_violations: usize) -> Result<Self, ConsensusError> {
        let sct_guard = SctGuard::new(max_violations)?;
        let bft_aggregator = BftAggregator::with_defaults();
        Ok(Self {
            sct_guard,
            bft_aggregator,
            latency_limit_ms: 500.0,
        })
    }

    /// Construye con configuraciÃ³n BFT personalizada.
    pub fn with_bft_config(
        max_violations: usize,
        bft_config: BftConfig,
    ) -> Result<Self, ConsensusError> {
        let sct_guard = SctGuard::new(max_violations)?;
        let bft_aggregator = BftAggregator::new(bft_config);
        Ok(Self {
            sct_guard,
            bft_aggregator,
            latency_limit_ms: 500.0,
        })
    }

    /// EvalÃºa un payload individual mediante SCT.
    ///
    /// Simula la evaluaciÃ³n SCT calculando Z a partir del gradiente:
    /// - Gradiente positivo (simbiÃ³tico) â†’ Z â‰ˆ +0.8 â†’ APPROVED
    /// - Gradiente negativo (perverso) â†’ Z â‰ˆ -0.9 â†’ HARD REJECT
    pub fn evaluate_payload(&mut self, payload: &SaePayload) -> SctEvaluation {
        let start = Instant::now();

        // Simulate SCT evaluation based on gradient characteristics
        let gradient_mean: f32 =
            payload.gradient.iter().sum::<f32>() / payload.gradient.len() as f32;

        // Map gradient mean to Z value: positive mean â†’ positive Z, negative mean â†’ negative Z
        let z_value = if gradient_mean > 0.0 {
            // Symbiotic: Z â‰ˆ +0.8
            0.6 + (gradient_mean * 0.4).min(0.2)
        } else {
            // Perverse: Z â‰ˆ -0.9
            -0.5 + (gradient_mean * 0.8).max(-0.4)
        };

        let x_value = 0.5; // Neutral benefit axis
        let y_value = 0.5; // Neutral cost axis

        let tensor = TopologicalTensor::new(x_value, y_value, z_value).unwrap_or_else(|_| {
            TopologicalTensor::new(0.5, 0.5, 0.0).expect("Neutral tensor should always work")
        });
        let decision = tensor.evaluate_trajectory().unwrap_or_else(|_| {
            if z_value > 0.0 {
                SCTDecision::Approved(z_value)
            } else {
                SCTDecision::Rejected(z_value)
            }
        });

        let approved = decision.is_approved();

        // Generate deterministic log message
        let log_message = if approved {
            format!(
                "[SCT] Evaluando Nodo {}... Z={:+.1} -> APPROVED",
                payload.node_id, z_value
            )
        } else {
            format!(
                "[SCT] Evaluando Nodo {}... Z={:+.1} -> HARD REJECT (Perversity Detected)",
                payload.node_id, z_value
            )
        };

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;

        SctEvaluation {
            node_id: payload.node_id.clone(),
            z_value,
            decision,
            approved,
            log_message,
        }
    }

    /// Ejecuta consenso completo sobre una lista de payloads.
    ///
    /// Flujo:
    /// 1. Evaluar cada payload con SCT
    /// 2. Filtrar rechazados (Hard Reject)
    /// 3. Ejecutar BFT aggregation sobre aprobados
    /// 4. Verificar latencia < 500ms
    pub fn run_consensus(
        &mut self,
        payloads: &[SaePayload],
    ) -> Result<ConsensusMetrics, ConsensusError> {
        let start = Instant::now();

        // Phase 1: SCT Evaluation
        let mut evaluations = Vec::new();
        let mut approved_gradients: Vec<Vec<f32>> = Vec::new();

        for payload in payloads {
            let evaluation = self.evaluate_payload(payload);

            // Print deterministic log
            println!("{}", evaluation.log_message);

            if evaluation.approved {
                approved_gradients.push(payload.gradient.clone());
            }

            evaluations.push(evaluation);
        }

        let approved_count = approved_gradients.len();
        let rejected_count = payloads.len() - approved_count;

        // Phase 2: BFT Aggregation (only on approved payloads)
        let bft_result = if approved_gradients.is_empty() {
            None
        } else if approved_gradients.len() < 2 {
            // Single gradient: return as-is
            Some(approved_gradients[0].clone())
        } else {
            // Multiple gradients: use coordinate-wise median
            match coordinate_wise_median(&approved_gradients) {
                Ok(median) => Some(median),
                Err(e) => {
                    eprintln!("[BFT] Warning: {}", e);
                    // Fallback to first gradient
                    Some(approved_gradients[0].clone())
                }
            }
        };

        let total_latency_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Phase 3: Latency check
        if total_latency_ms > self.latency_limit_ms {
            return Err(ConsensusError::LatencyExceeded {
                elapsed_ms: total_latency_ms,
                limit_ms: self.latency_limit_ms,
            });
        }

        let avg_latency_ms = if payloads.is_empty() {
            0.0
        } else {
            total_latency_ms / payloads.len() as f64
        };

        // Print BFT summary
        if let Some(ref result) = bft_result {
            let mean: f32 = result.iter().sum::<f32>() / result.len() as f32;
            println!(
                "[BFT] Aggregation complete: {} gradients, median mean={:.4}",
                approved_count, mean
            );
        } else {
            println!("[BFT] No valid gradients for aggregation");
        }

        println!(
            "[MVP] Latency: {:.1}ms (limit: {:.0}ms) â€” {}",
            total_latency_ms,
            self.latency_limit_ms,
            if total_latency_ms < self.latency_limit_ms {
                "PASS"
            } else {
                "FAIL"
            }
        );

        Ok(ConsensusMetrics {
            total_payloads: payloads.len(),
            approved_count,
            rejected_count,
            total_latency_ms,
            avg_latency_ms,
            bft_result,
            evaluations,
        })
    }

    /// Genera reporte JSON de mÃ©tricas.
    pub fn metrics_to_json(&self, metrics: &ConsensusMetrics) -> String {
        let evals: Vec<&SctEvaluation> = metrics.evaluations.iter().collect();
        serde_json::json!({
            "total_payloads": metrics.total_payloads,
            "approved_count": metrics.approved_count,
            "rejected_count": metrics.rejected_count,
            "total_latency_ms": (metrics.total_latency_ms * 100.0).round() / 100.0,
            "avg_latency_ms": (metrics.avg_latency_ms * 100.0).round() / 100.0,
            "bft_converged": metrics.bft_result.is_some(),
            "evaluations": evals.iter().map(|e| {
                serde_json::json!({
                    "node_id": e.node_id,
                    "z_value": (e.z_value * 100.0).round() / 100.0,
                    "approved": e.approved,
                    "log_message": e.log_message
                })
            }).collect::<Vec<_>>(),
        })
        .to_string()
    }
}

impl Default for ConsensusRunner {
    fn default() -> Self {
        Self::new(3).expect("Default ConsensusRunner should initialize")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mvp::sae_simulator::NodeProfile;

    fn make_symbiotic_payload(id: &str) -> SaePayload {
        let grad: Vec<f32> = (0..128).map(|i| 0.3 + i as f32 / 200.0).collect();
        SaePayload {
            node_id: id.to_string(),
            gradient: grad,
            dimensions: (128, 1),
            profile: NodeProfile::Symbiotic,
            expected_z: 0.8,
        }
    }

    fn make_perverse_payload(id: &str) -> SaePayload {
        let grad: Vec<f32> = (0..128).map(|i| -0.5 - i as f32 / 200.0).collect();
        SaePayload {
            node_id: id.to_string(),
            gradient: grad,
            dimensions: (128, 1),
            profile: NodeProfile::Perverse,
            expected_z: -0.9,
        }
    }

    #[test]
    fn test_runner_creation() {
        let runner = ConsensusRunner::new(3).unwrap();
        assert_eq!(runner.latency_limit_ms, 500.0);
    }

    #[test]
    fn test_evaluate_symbiotic_approved() {
        let mut runner = ConsensusRunner::default();
        let payload = make_symbiotic_payload("alpha");
        let eval = runner.evaluate_payload(&payload);
        assert!(eval.approved, "Symbiotic should be approved");
        assert!(eval.z_value > 0.0, "Z should be positive for symbiotic");
        assert!(eval.log_message.contains("APPROVED"));
    }

    #[test]
    fn test_evaluate_perverse_rejected() {
        let mut runner = ConsensusRunner::default();
        let payload = make_perverse_payload("beta");
        let eval = runner.evaluate_payload(&payload);
        assert!(!eval.approved, "Perverse should be rejected");
        assert!(eval.z_value < 0.0, "Z should be negative for perverse");
        assert!(eval.log_message.contains("HARD REJECT"));
    }

    #[test]
    fn test_consensus_mixed_payloads() {
        let mut runner = ConsensusRunner::default();
        let payloads = vec![
            make_symbiotic_payload("alpha"),
            make_perverse_payload("beta"),
            make_symbiotic_payload("gamma"),
        ];
        let metrics = runner.run_consensus(&payloads).unwrap();
        assert_eq!(metrics.total_payloads, 3);
        assert_eq!(metrics.approved_count, 2);
        assert_eq!(metrics.rejected_count, 1);
        assert!(metrics.bft_result.is_some());
        assert!(metrics.total_latency_ms < 500.0);
    }

    #[test]
    fn test_consensus_all_perverse() {
        let mut runner = ConsensusRunner::default();
        let payloads = vec![make_perverse_payload("bad1"), make_perverse_payload("bad2")];
        let metrics = runner.run_consensus(&payloads).unwrap();
        assert_eq!(metrics.approved_count, 0);
        assert_eq!(metrics.rejected_count, 2);
        assert!(metrics.bft_result.is_none());
    }

    #[test]
    fn test_consensus_all_symbiotic() {
        let mut runner = ConsensusRunner::default();
        let payloads = vec![
            make_symbiotic_payload("good1"),
            make_symbiotic_payload("good2"),
            make_symbiotic_payload("good3"),
        ];
        let metrics = runner.run_consensus(&payloads).unwrap();
        assert_eq!(metrics.approved_count, 3);
        assert_eq!(metrics.rejected_count, 0);
        assert!(metrics.bft_result.is_some());
    }

    #[test]
    fn test_metrics_to_json() {
        let runner = ConsensusRunner::default();
        let metrics = ConsensusMetrics {
            total_payloads: 2,
            approved_count: 1,
            rejected_count: 1,
            total_latency_ms: 10.5,
            avg_latency_ms: 5.25,
            bft_result: Some(vec![0.5]),
            evaluations: vec![SctEvaluation {
                node_id: "test".to_string(),
                z_value: 0.8,
                decision: SCTDecision::Approved(0.8),
                approved: true,
                log_message: "[SCT] Evaluando Nodo test... Z=+0.8 -> APPROVED".to_string(),
            }],
        };
        let json = runner.metrics_to_json(&metrics);
        assert!(json.contains("total_payloads"));
        assert!(json.contains("approved_count"));
    }

    #[test]
    fn test_default_runner() {
        let _runner = ConsensusRunner::default();
    }
}
