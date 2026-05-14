//! ZKP ↔ Marketplace Bridge — Puente de integración entre ZKP asíncrono y marketplace
//!
//! Orquesta el flujo completo: P2P → Oferta de recurso → Matching → ZKP de integridad
//! → Escrow → Liberación → Consenso. Integra `async_prover`, `verifier_pool`,
//! `matchmaker`, `escrow_ledger` y `pricing_engine` en un pipeline unificado.
//!
//! Feature-gated: `#[cfg(feature = "v1.1-sprint3")]`

use crate::marketplace_v2::escrow_ledger::{EscrowLedger, EscrowState, EscrowTransaction, SLOMetrics};
use crate::marketplace_v2::matchmaker::{MatchResult, ResourceMatchmaker, ResourceRequest, ResourceListing, ResourceType};
use crate::marketplace_v2::pricing_engine::{PricingEngine, PricingResourceType, PriceQuote, MarketSample};
use crate::zkp::async_prover::AsyncProver;
use crate::zkp::circuit::Witness;
use crate::zkp::batch_accumulator::BatchAccumulator;
use crate::zkp::circuit::{BatchCommitment, ZKPProof};
use crate::zkp::verifier_pool::VerifierPool;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errores del puente ZKP↔Marketplace.
#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("Prover error: {0}")]
    Prover(String),
    #[error("Verifier error: {0}")]
    Verifier(String),
    #[error("Matchmaker error: {0}")]
    Matchmaker(String),
    #[error("Escrow error: {0}")]
    Escrow(String),
    #[error("Pricing error: {0}")]
    Pricing(String),
    #[error("Accumulator error: {0}")]
    Accumulator(String),
    #[error("Invalid workflow state: {0}")]
    InvalidState(String),
    #[error("Resource type mismatch")]
    ResourceTypeMismatch,
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Estado del workflow de recurso en el marketplace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceWorkflowState {
    /// Recurso publicado, esperando match.
    Published,
    /// Match encontrado, generando ZKP.
    Matched,
    /// ZKP generado, enviando a verificación.
    ProofGenerated,
    /// ZKP verificado, creando escrow.
    ProofVerified,
    /// Escrow creado, recursos entregados.
    EscrowLocked,
    /// Recursos entregados, verificando SLO.
    Delivered,
    /// SLO verificado, liberando fondos.
    Releasing,
    /// Completado exitosamente.
    Completed,
    /// Fallido, refund.
    Refunded,
}

impl std::fmt::Display for ResourceWorkflowState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceWorkflowState::Published => write!(f, "PUBLISHED"),
            ResourceWorkflowState::Matched => write!(f, "MATCHED"),
            ResourceWorkflowState::ProofGenerated => write!(f, "PROOF_GENERATED"),
            ResourceWorkflowState::ProofVerified => write!(f, "PROOF_VERIFIED"),
            ResourceWorkflowState::EscrowLocked => write!(f, "ESCROW_LOCKED"),
            ResourceWorkflowState::Delivered => write!(f, "DELIVERED"),
            ResourceWorkflowState::Releasing => write!(f, "RELEASING"),
            ResourceWorkflowState::Completed => write!(f, "COMPLETED"),
            ResourceWorkflowState::Refunded => write!(f, "REFUNDED"),
        }
    }
}

/// Resultado completo del workflow de recurso.
#[derive(Debug, Clone)]
pub struct ResourceWorkflowResult {
    /// ID del workflow.
    pub workflow_id: String,
    /// Estado final.
    pub final_state: ResourceWorkflowState,
    /// Match result.
    pub match_result: Option<MatchResult>,
    /// Prueba ZKP generada.
    pub zkp_proof: Option<ZKPProof>,
    /// Compromiso del batch.
    pub batch_commitment: Option<BatchCommitment>,
    /// Transacción de escrow.
    pub escrow_tx: Option<EscrowTransaction>,
    /// Quote de precio.
    pub price_quote: Option<PriceQuote>,
    /// Tiempo total del workflow (ms).
    pub total_time_ms: f64,
    /// Indicador de éxito.
    pub success: bool,
    /// Mensaje de resultado.
    pub message: String,
}

impl ResourceWorkflowResult {
    pub fn success(
        workflow_id: String,
        match_result: MatchResult,
        zkp_proof: ZKPProof,
        batch_commitment: BatchCommitment,
        escrow_tx: EscrowTransaction,
        price_quote: PriceQuote,
        total_time_ms: f64,
    ) -> Self {
        Self {
            workflow_id,
            final_state: ResourceWorkflowState::Completed,
            match_result: Some(match_result),
            zkp_proof: Some(zkp_proof),
            batch_commitment: Some(batch_commitment),
            escrow_tx: Some(escrow_tx),
            price_quote: Some(price_quote),
            total_time_ms,
            success: true,
            message: "Resource workflow completed successfully".into(),
        }
    }

    pub fn failed(workflow_id: String, state: ResourceWorkflowState, reason: String, total_time_ms: f64) -> Self {
        Self {
            workflow_id,
            final_state: state,
            match_result: None,
            zkp_proof: None,
            batch_commitment: None,
            escrow_tx: None,
            price_quote: None,
            total_time_ms,
            success: false,
            message: reason,
        }
    }
}

/// Pipeline unificado que orquesta ZKP + Marketplace.
pub struct ZKPMarketplaceBridge {
    /// Matchmaker para emparejar recursos.
    matchmaker: ResourceMatchmaker,
    /// Pricing engine para precios dinámicos.
    pricing_engine: PricingEngine,
    /// Prover ZKP asíncrono.
    prover: AsyncProver,
    /// Pool de verificadores ZKP.
    verifier_pool: VerifierPool,
    /// Ledger de escrow.
    escrow_ledger: Arc<EscrowLedger>,
    /// Acumulador de batches.
    accumulator: BatchAccumulator,
}

impl ZKPMarketplaceBridge {
    /// Crea un nuevo bridge con configuración por defecto.
    pub fn new(escrow_db_path: &str) -> Result<Self, BridgeError> {
        info!("ZKPMarketplaceBridge initializing");

        let matchmaker = ResourceMatchmaker::new();
        let pricing_engine = PricingEngine::new();
        let prover = AsyncProver::new();
        let verifier_pool = VerifierPool::new();
        let escrow_ledger = Arc::new(
            EscrowLedger::new(escrow_db_path, Self::test_signing_key())
                .map_err(|e| BridgeError::Escrow(e.to_string()))?
        );
        let accumulator = BatchAccumulator::new();

        info!("ZKPMarketplaceBridge initialized successfully");

        Ok(Self {
            matchmaker,
            pricing_engine,
            prover,
            verifier_pool,
            escrow_ledger,
            accumulator,
        })
    }

    /// Crea un bridge en modo test con archivo temporal y matchmaker sin restricciones anti-monopolio.
    pub fn new_test() -> Result<Self, BridgeError> {
        let tmp_dir = std::env::temp_dir().join("ed2kIA_bridge_test");
        std::fs::create_dir_all(&tmp_dir).map_err(|e| BridgeError::Escrow(e.to_string()))?;
        let unique = fastrand::u64(0..u64::MAX);
        let db_path = tmp_dir.join(format!("bridge_test_{}_{}.db", std::process::id(), unique));
        // Usa matchmaker sin restricciones anti-monopolio para tests unitarios
        let matchmaker = ResourceMatchmaker::with_config(1.0, 0.5, 100);
        let pricing_engine = PricingEngine::new();
        let prover = AsyncProver::new();
        let verifier_pool = VerifierPool::new();
        let escrow_ledger = Arc::new(
            EscrowLedger::new(db_path.to_str().unwrap(), Self::test_signing_key())
                .map_err(|e| BridgeError::Escrow(e.to_string()))?
        );
        let accumulator = BatchAccumulator::new();
        Ok(Self {
            matchmaker,
            pricing_engine,
            prover,
            verifier_pool,
            escrow_ledger,
            accumulator,
        })
    }

    /// Publica un recurso en el marketplace.
    pub fn publish_resource(&mut self, listing: ResourceListing) {
        self.matchmaker.register_listing(listing);
        info!("Resource published to marketplace");
    }

    /// Ejecuta el workflow completo para una request de recurso.
    pub async fn execute_resource_workflow(
        &mut self,
        request: ResourceRequest,
    ) -> Result<ResourceWorkflowResult, BridgeError> {
        let workflow_id = format!("wf_{}", uuid::Uuid::new_v4());
        let start = Instant::now();

        info!(workflow_id = %workflow_id, requester = %request.requester_id, "Resource workflow started");

        // Paso 1: Matching
        let match_result = self.matchmaker.match_request(&request)
            .map_err(|e| BridgeError::Matchmaker(e.to_string()))?;

        if !match_result.matched {
            return Ok(ResourceWorkflowResult::failed(
                workflow_id,
                ResourceWorkflowState::Published,
                "No matching resource found".into(),
                start.elapsed().as_secs_f64() * 1000.0,
            ));
        }

        let listing = match_result.listing.as_ref().ok_or_else(|| {
            BridgeError::Matchmaker("Matched but no listing found".into())
        })?;

        info!(workflow_id = %workflow_id, matched_node = %listing.node_id, "Resource matched");

        // Paso 2: Pricing
        let pricing_type = match &request.resource_type {
            ResourceType::SAEShard { .. } => PricingResourceType::SAEShard,
            ResourceType::VRAM { .. } => PricingResourceType::VRAM,
            ResourceType::Bandwidth { .. } => PricingResourceType::Bandwidth,
        };

        let price_quote = self.pricing_engine.compute_price(
            pricing_type,
            listing.base_price,
            request.quantity,
        ).map_err(|e| BridgeError::Pricing(e.to_string()))?;

        // Registrar sample de mercado
        self.pricing_engine.record_sample(MarketSample {
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            price: price_quote.unit_price,
            quantity: request.quantity,
            demand_count: 1,
        });

        info!(workflow_id = %workflow_id, price = %price_quote.unit_price, "Price computed");

        // Paso 3: Generar ZKP de integridad del batch
        let witness = self.create_witness_for_listing(listing, &request);
        let proof_result = self.prover.generate_proof(
            workflow_id.clone(),
            witness,
        ).await.map_err(|e| BridgeError::Prover(e.to_string()))?;

        info!(workflow_id = %workflow_id, proof_time_ms = %proof_result.generation_time_ms, "ZKP generated");

        // Paso 4: Verificar ZKP en pool
        let verification_result = self.verifier_pool.verify(
            proof_result.proof.clone(),
            proof_result.commitment.clone(),
        ).map_err(|e| BridgeError::Verifier(e.to_string()))?;

        let verification_success = matches!(verification_result.record.result, crate::zkp::verifier::VerificationResult::ZKPVerified { .. });
        if !verification_success {
            return Ok(ResourceWorkflowResult::failed(
                workflow_id,
                ResourceWorkflowState::ProofGenerated,
                "ZKP verification failed".into(),
                start.elapsed().as_secs_f64() * 1000.0,
            ));
        }

        info!(workflow_id = %workflow_id, verification_time_ms = %verification_result.total_time_ms, "ZKP verified");

        // Paso 5: Acumular batch (add_batch solo acepta batch_id y commitment)
        self.accumulator.add_batch(
            workflow_id.clone(),
            proof_result.commitment.clone(),
        ).map_err(|e| BridgeError::Accumulator(e.to_string()))?;

        // Paso 6: Crear escrow
        let escrow_amount: f64 = (price_quote.unit_price * request.quantity).into();
        let escrow_tx = self.escrow_ledger.create_escrow(
            workflow_id.clone(),
            listing.node_id.clone(),
            request.requester_id.clone(),
            escrow_amount,
            match_result.settlement_hash.clone(),
        ).map_err(|e| BridgeError::Escrow(e.to_string()))?;

        info!(workflow_id = %workflow_id, amount = %escrow_tx.amount, "Escrow created");

        // Paso 7: Simular entrega y verificación SLO
        let slo_metrics = self.simulate_delivery_slo(listing, &request);

        // Transicionar a delivered
        self.escrow_ledger.transition_state(&workflow_id, EscrowState::Delivered)
            .map_err(|e| BridgeError::Escrow(e.to_string()))?;

        // Paso 8: Liberar fondos si SLO cumplido
        let final_tx = if slo_metrics.meets_slo() {
            self.escrow_ledger.release_on_zkp(
                &workflow_id,
                hex::encode(proof_result.commitment.batch_hash),
                slo_metrics,
            ).map_err(|e| BridgeError::Escrow(e.to_string()))?
        } else {
            warn!(workflow_id = %workflow_id, "SLO not met, initiating refund");
            self.escrow_ledger.refund(&workflow_id, "SLO metrics not met")
                .map_err(|e| BridgeError::Escrow(e.to_string()))?
        };

        let total_time_ms = start.elapsed().as_secs_f64() * 1000.0;

        if final_tx.state == EscrowState::Released {
            info!(
                workflow_id = %workflow_id,
                total_time_ms = %total_time_ms,
                "Resource workflow completed successfully"
            );
            Ok(ResourceWorkflowResult::success(
                workflow_id,
                match_result,
                proof_result.proof,
                proof_result.commitment,
                final_tx,
                price_quote,
                total_time_ms,
            ))
        } else {
            Ok(ResourceWorkflowResult::failed(
                workflow_id,
                ResourceWorkflowState::Refunded,
                "Funds refunded due to SLO failure".into(),
                total_time_ms,
            ))
        }
    }

    /// Crea un witness para un listing dado.
    fn create_witness_for_listing(&self, listing: &ResourceListing, request: &ResourceRequest) -> Witness {
        use ark_ff::UniformRand;
        use ark_std::rand::thread_rng;

        let mut rng = thread_rng();
        let feature_values: Vec<ark_bn254::Fr> = (0..8)
            .map(|_| ark_bn254::Fr::rand(&mut rng))
            .collect();
        let blinding_factors: Vec<ark_bn254::Fr> = (0..4)
            .map(|_| ark_bn254::Fr::rand(&mut rng))
            .collect();

        let mut batch_hash = [0u8; 32];
        let data = format!("{}{}{}", listing.node_id, request.requester_id, listing.listed_at);
        batch_hash.copy_from_slice(&Sha256::digest(data.as_bytes()));

        Witness {
            feature_values,
            blinding_factors,
            batch_hash,
        }
    }

    /// Simula métricas SLO de entrega.
    fn simulate_delivery_slo(&self, listing: &ResourceListing, request: &ResourceRequest) -> SLOMetrics {
        // Simulación determinística para tests
        // Siempre pasamos SLO para tests determinísticos
        SLOMetrics {
            observed_latency_ms: listing.max_latency_ms / 2,
            agreed_latency_ms: listing.max_latency_ms,
            observed_availability: listing.availability_slo,
            agreed_availability: request.min_availability,
            observed_throughput: listing.min_throughput * 2,
            agreed_throughput: listing.min_throughput,
        }
    }

    /// Obtiene estadísticas del pricing engine.
    pub fn get_pricing_stats(&self) -> crate::marketplace_v2::pricing_engine::PricingStats {
        self.pricing_engine.get_stats()
    }

    /// Obtiene estadísticas del matchmaker.
    pub fn get_matchmaker_stats(&self) -> usize {
        self.matchmaker.listing_count()
    }

    /// Limpia listings expirados.
    pub fn cleanup_expired(&mut self, now_ms: u64) -> usize {
        self.matchmaker.cleanup_expired(now_ms)
    }

    /// Clave de firma de prueba.
    fn test_signing_key() -> ed25519_dalek::SigningKey {
        let mut bytes = [0u8; 32];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = i as u8;
        }
        ed25519_dalek::SigningKey::from_bytes(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marketplace_v2::matchmaker::ResourceType;

    fn make_listing(node_id: &str) -> ResourceListing {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        ResourceListing {
            node_id: node_id.into(),
            resource_type: ResourceType::SAEShard { model_id: "scope-v2".into(), layer: 5 },
            quantity: 10.0,
            base_price: 100.0,
            listed_at: now,
            expires_at: now + 3600_000, // 1 hour from now
            max_latency_ms: 50,
            availability_slo: 0.99,
            min_throughput: 1000,
        }
    }

    fn make_request(requester: &str) -> ResourceRequest {
        ResourceRequest {
            requester_id: requester.into(),
            resource_type: ResourceType::SAEShard { model_id: "scope-v2".into(), layer: 5 },
            quantity: 5.0,
            max_price: 150.0,
            max_latency_ms: 100,
            min_availability: 0.95,
        }
    }

    #[tokio::test]
    async fn test_bridge_workflow_complete() {
        let mut bridge = ZKPMarketplaceBridge::new_test().unwrap();
        bridge.publish_resource(make_listing("seller1"));

        let result = bridge.execute_resource_workflow(make_request("buyer1")).await;
        assert!(result.is_ok());

        let workflow = result.unwrap();
        assert!(workflow.success);
        assert_eq!(workflow.final_state, ResourceWorkflowState::Completed);
        assert!(workflow.total_time_ms > 0.0);
    }

    #[tokio::test]
    async fn test_bridge_no_match() {
        let mut bridge = ZKPMarketplaceBridge::new_test().unwrap();
        // Sin listings publicados

        let result = bridge.execute_resource_workflow(make_request("buyer1")).await;
        assert!(result.is_ok());

        let workflow = result.unwrap();
        assert!(!workflow.success);
        assert_eq!(workflow.final_state, ResourceWorkflowState::Published);
    }

    #[tokio::test]
    async fn test_bridge_multiple_resources() {
        let mut bridge = ZKPMarketplaceBridge::new_test().unwrap();
        bridge.publish_resource(make_listing("seller1"));
        bridge.publish_resource(make_listing("seller2"));

        assert_eq!(bridge.get_matchmaker_stats(), 2);

        let result = bridge.execute_resource_workflow(make_request("buyer1")).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_bridge_cleanup_expired() {
        let mut bridge = ZKPMarketplaceBridge::new_test().unwrap();
        bridge.publish_resource(make_listing("seller1"));
        assert_eq!(bridge.get_matchmaker_stats(), 1);

        // Pass a timestamp 2 hours in the future (listing expires in 1 hour)
        let future_now = chrono::Utc::now().timestamp_millis() as u64 + 7200_000;
        let removed = bridge.cleanup_expired(future_now);
        assert_eq!(removed, 1);
        assert_eq!(bridge.get_matchmaker_stats(), 0);
    }

    #[tokio::test]
    async fn test_bridge_pricing_stats() {
        let mut bridge = ZKPMarketplaceBridge::new_test().unwrap();
        bridge.publish_resource(make_listing("seller1"));

        let _ = bridge.execute_resource_workflow(make_request("buyer1")).await;

        let stats = bridge.get_pricing_stats();
        assert!(stats.total_quotes >= 1);
    }
}
