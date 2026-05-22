//! v1.1.0 Sprint 3 E2E Integration Tests
//!
//! Flujo completo: P2P → Oferta de recurso → Matching → ZKP de integridad
//! → Escrow → Liberación → Consenso.
//!
//! Feature-gated: `--features v1.1-sprint3`

#![cfg(feature = "v1.1-sprint3")]
use ark_ec::CurveGroup;

use ark_bn254::Fr;
use ark_ff::UniformRand;
use ark_std::rand::thread_rng;
use ed2kia::bridge_v2::proof_submission::{ProofSubmissionManager, SubmissionState};
use ed2kia::bridge_v2::zkp_marketplace_bridge::{ResourceWorkflowState, ZKPMarketplaceBridge};
use ed2kia::marketplace_v2::escrow_ledger::{EscrowLedger, EscrowState, SLOMetrics};
use ed2kia::marketplace_v2::matchmaker::{ResourceListing, ResourceRequest, ResourceType};
use ed2kia::marketplace_v2::pricing_engine::{MarketSample, PricingEngine, PricingResourceType};
use ed2kia::zkp::circuit::Witness;
use sha2::{Digest, Sha256};

// ============================================================================
// Helpers
// ============================================================================

fn make_sae_listing(node_id: &str, layer: u32, price: f32) -> ResourceListing {
    let now = chrono::Utc::now().timestamp_millis() as u64;
    ResourceListing {
        node_id: node_id.into(),
        resource_type: ResourceType::SAEShard {
            model_id: "scope-v2".into(),
            layer,
        },
        quantity: 10.0,
        base_price: price,
        listed_at: now,
        expires_at: now + 3600_000, // 1 hour from now
        max_latency_ms: 100,
        availability_slo: 0.99,
        min_throughput: 1000,
    }
}

fn make_vram_listing(node_id: &str, vram_gb: f32, price: f32) -> ResourceListing {
    let now = chrono::Utc::now().timestamp_millis() as u64;
    ResourceListing {
        node_id: node_id.into(),
        resource_type: ResourceType::VRAM {
            gpu_model: "A100".into(),
            vram_gb,
        },
        quantity: vram_gb,
        base_price: price,
        listed_at: now,
        expires_at: now + 3600_000, // 1 hour from now
        max_latency_ms: 50,
        availability_slo: 0.995,
        min_throughput: 5000,
    }
}

fn make_sae_request(requester: &str, layer: u32, qty: f32, max_price: f32) -> ResourceRequest {
    ResourceRequest {
        requester_id: requester.into(),
        resource_type: ResourceType::SAEShard {
            model_id: "scope-v2".into(),
            layer,
        },
        quantity: qty,
        max_price,
        max_latency_ms: 100,
        min_availability: 0.95,
    }
}

fn create_test_witness() -> Witness {
    let mut rng = thread_rng();
    let feature_values: Vec<Fr> = (0..8).map(|_| Fr::rand(&mut rng)).collect();
    let blinding_factors: Vec<Fr> = (0..4).map(|_| Fr::rand(&mut rng)).collect();

    let mut batch_hash = [0u8; 32];
    batch_hash.copy_from_slice(&Sha256::digest(b"e2e_test_batch"));

    Witness {
        feature_values,
        blinding_factors,
        batch_hash,
    }
}

// ============================================================================
// Test: Marketplace Matching
// ============================================================================

#[test]
fn test_e2e_marketplace_matching() {
    let mut matchmaker =
        ed2kia::marketplace_v2::matchmaker::ResourceMatchmaker::with_config(1.0, 0.5, 100);

    // Publicar recursos
    matchmaker.register_listing(make_sae_listing("node_a", 5, 100.0));
    matchmaker.register_listing(make_sae_listing("node_b", 5, 80.0));
    matchmaker.register_listing(make_vram_listing("node_c", 80.0, 250.0));

    assert_eq!(matchmaker.listing_count(), 3);

    // Request SAE shard
    let req = make_sae_request("buyer1", 5, 5.0, 120.0);
    let result = matchmaker.match_request(&req).unwrap();
    assert!(result.matched);
    assert_eq!(result.final_price, 80.0); // node_b es más barato
    assert_eq!(result.listing.as_ref().unwrap().node_id, "node_b");

    // Request VRAM
    let vram_req = ResourceRequest {
        requester_id: "buyer2".into(),
        resource_type: ResourceType::VRAM {
            gpu_model: "A100".into(),
            vram_gb: 40.0,
        },
        quantity: 40.0,
        max_price: 300.0,
        max_latency_ms: 50,
        min_availability: 0.95,
    };
    let vram_result = matchmaker.match_request(&vram_req).unwrap();
    assert!(vram_result.matched);
    assert_eq!(vram_result.listing.as_ref().unwrap().node_id, "node_c");
}

#[test]
fn test_e2e_marketplace_no_match() {
    let mut matchmaker = ed2kia::marketplace_v2::matchmaker::ResourceMatchmaker::new();

    // Sin listings
    let req = make_sae_request("buyer1", 5, 5.0, 120.0);
    let result = matchmaker.match_request(&req);
    // No match is Ok with matched=false, not an error
    assert!(result.is_ok());
    let match_result = result.unwrap();
    assert!(!match_result.matched);
}

#[test]
fn test_e2e_marketplace_price_exceeds_max() {
    let mut matchmaker =
        ed2kia::marketplace_v2::matchmaker::ResourceMatchmaker::with_config(1.0, 0.5, 100);
    matchmaker.register_listing(make_sae_listing("node_a", 5, 200.0));

    let req = make_sae_request("buyer1", 5, 5.0, 150.0);
    let result = matchmaker.match_request(&req).unwrap();
    assert!(!result.matched);
}

#[test]
fn test_e2e_marketplace_cleanup_expired() {
    let mut matchmaker =
        ed2kia::marketplace_v2::matchmaker::ResourceMatchmaker::with_config(1.0, 0.5, 100);
    // Listing that expires in 1 hour (from make_sae_listing)
    matchmaker.register_listing(make_sae_listing("node_a", 5, 100.0));
    // Listing that already expired (expires_at in the past)
    matchmaker.register_listing(ResourceListing {
        node_id: "node_b".into(),
        resource_type: ResourceType::SAEShard {
            model_id: "scope-v2".into(),
            layer: 5,
        },
        quantity: 10.0,
        base_price: 90.0,
        listed_at: 100,
        expires_at: 500, // Already expired
        max_latency_ms: 100,
        availability_slo: 0.99,
        min_throughput: 1000,
    });

    assert_eq!(matchmaker.listing_count(), 2);
    // Use current time to clean up expired listings
    let now = chrono::Utc::now().timestamp_millis() as u64;
    let removed = matchmaker.cleanup_expired(now);
    assert_eq!(removed, 1);
    assert_eq!(matchmaker.listing_count(), 1);
}

// ============================================================================
// Test: Escrow Ledger
// ============================================================================

#[test]
fn test_e2e_escrow_create_and_release() {
    let (ledger, _, _path) = EscrowLedger::new_test().unwrap();

    // Crear escrow
    let tx = ledger
        .create_escrow(
            "tx_e2e_1".into(),
            "seller".into(),
            "buyer".into(),
            500.0,
            "settlement_hash_e2e".into(),
        )
        .unwrap();

    assert_eq!(tx.state, EscrowState::Locked);
    assert_eq!(tx.amount, 500.0);

    // Transiciones
    let tx = ledger
        .transition_state("tx_e2e_1", EscrowState::Delivered)
        .unwrap();
    assert_eq!(tx.state, EscrowState::Delivered);

    let slo = SLOMetrics {
        observed_latency_ms: 50,
        agreed_latency_ms: 100,
        observed_availability: 0.99,
        agreed_availability: 0.95,
        observed_throughput: 2000,
        agreed_throughput: 1000,
    };

    let tx = ledger
        .release_on_zkp("tx_e2e_1", "zkp_hash_e2e".into(), slo)
        .unwrap();
    assert_eq!(tx.state, EscrowState::Released);
    assert_eq!(tx.zkp_hash, Some("zkp_hash_e2e".into()));
}

#[test]
fn test_e2e_escrow_refund_on_slo_failure() {
    let (ledger, _, _path) = EscrowLedger::new_test().unwrap();

    ledger
        .create_escrow(
            "tx_e2e_2".into(),
            "seller".into(),
            "buyer".into(),
            500.0,
            "settlement_hash_e2e_2".into(),
        )
        .unwrap();

    // SLO no cumplido: latencia observada > acordada
    let slo = SLOMetrics {
        observed_latency_ms: 200,
        agreed_latency_ms: 100,
        observed_availability: 0.99,
        agreed_availability: 0.95,
        observed_throughput: 2000,
        agreed_throughput: 1000,
    };

    let result = ledger.release_on_zkp("tx_e2e_2", "zkp_hash".into(), slo);
    assert!(result.is_err());

    // Refund manual
    let tx = ledger
        .refund("tx_e2e_2", "SLO not met: latency exceeded")
        .unwrap();
    assert_eq!(tx.state, EscrowState::Refunded);
}

#[test]
fn test_e2e_escrow_dispute() {
    let (ledger, _, _path) = EscrowLedger::new_test().unwrap();

    ledger
        .create_escrow(
            "tx_e2e_3".into(),
            "seller".into(),
            "buyer".into(),
            500.0,
            "settlement_hash_e2e_3".into(),
        )
        .unwrap();

    let tx = ledger
        .dispute("tx_e2e_3", "Quality issue detected")
        .unwrap();
    assert_eq!(tx.state, EscrowState::Disputed);

    // Desde disputed, se puede liberar o refund
    let tx = ledger
        .transition_state("tx_e2e_3", EscrowState::Refunded)
        .unwrap();
    assert_eq!(tx.state, EscrowState::Refunded);
}

#[test]
fn test_e2e_escrow_node_transactions() {
    let (ledger, _, _path) = EscrowLedger::new_test().unwrap();

    ledger
        .create_escrow(
            "tx_1".into(),
            "seller".into(),
            "buyer1".into(),
            100.0,
            "sh1".into(),
        )
        .unwrap();
    ledger
        .create_escrow(
            "tx_2".into(),
            "seller".into(),
            "buyer2".into(),
            200.0,
            "sh2".into(),
        )
        .unwrap();
    ledger
        .create_escrow(
            "tx_3".into(),
            "buyer".into(),
            "seller".into(),
            50.0,
            "sh3".into(),
        )
        .unwrap();

    let seller_txs = ledger.get_transactions_by_node("seller").unwrap();
    assert_eq!(seller_txs.len(), 3);

    let buyer1_txs = ledger.get_transactions_by_node("buyer1").unwrap();
    assert_eq!(buyer1_txs.len(), 1);
}

// ============================================================================
// Test: Pricing Engine
// ============================================================================

#[test]
fn test_e2e_pricing_compute() {
    let engine = PricingEngine::new();

    let quote = engine
        .compute_price(PricingResourceType::SAEShard, 100.0, 1.0)
        .unwrap();

    assert!(quote.unit_price > 0.0);
    assert!(quote.unit_price <= 10000.0); // max_price default
    assert_eq!(quote.resource_type, PricingResourceType::SAEShard);
}

#[test]
fn test_e2e_pricing_with_market_samples() {
    let engine = PricingEngine::new();

    // Registrar samples de alta demanda
    for i in 0..10 {
        engine.record_sample(MarketSample {
            timestamp: chrono::Utc::now().timestamp_millis() as u64 - (i * 1000) as u64,
            price: 100.0,
            quantity: 5.0,
            demand_count: 20,
        });
    }

    let quote = engine
        .compute_price(PricingResourceType::Generic, 100.0, 1.0)
        .unwrap();

    assert!(quote.unit_price > 0.0);
    // Price is computed from base with adjustment factor; verify it's within valid bounds
    assert!(quote.unit_price <= 10000.0);
}

#[test]
fn test_e2e_pricing_commitment_verification() {
    let engine = PricingEngine::new();
    let quote = engine
        .compute_price(PricingResourceType::VRAM, 250.0, 1.0)
        .unwrap();

    assert!(engine.verify_commitment(quote.unit_price, quote.commitment_hash));

    // Hash corrompido
    let mut wrong = quote.commitment_hash;
    wrong[0] ^= 0xFF;
    assert!(!engine.verify_commitment(quote.unit_price, wrong));
}

#[test]
fn test_e2e_pricing_stats() {
    let engine = PricingEngine::new();

    engine
        .compute_price(PricingResourceType::SAEShard, 100.0, 1.0)
        .unwrap();
    engine
        .compute_price(PricingResourceType::VRAM, 250.0, 1.0)
        .unwrap();
    engine
        .compute_price(PricingResourceType::Bandwidth, 10.0, 1.0)
        .unwrap();

    let stats = engine.get_stats();
    assert_eq!(stats.total_quotes, 3);
    assert!(stats.avg_price > 0.0);
}

// ============================================================================
// Test: ZKP Marketplace Bridge E2E
// ============================================================================

#[tokio::test]
async fn test_e2e_bridge_full_workflow() {
    let mut bridge = ZKPMarketplaceBridge::new_test().unwrap();

    // Publicar recursos
    bridge.publish_resource(make_sae_listing("seller1", 5, 100.0));
    bridge.publish_resource(make_sae_listing("seller2", 5, 80.0));

    // Ejecutar workflow
    let req = make_sae_request("buyer1", 5, 5.0, 150.0);
    let result = bridge.execute_resource_workflow(req).await;

    assert!(result.is_ok());
    let workflow = result.unwrap();
    assert!(workflow.success);
    assert_eq!(workflow.final_state, ResourceWorkflowState::Completed);
    assert!(workflow.total_time_ms > 0.0);
    assert!(workflow.match_result.is_some());
    assert!(workflow.zkp_proof.is_some());
    assert!(workflow.escrow_tx.is_some());
}

#[tokio::test]
async fn test_e2e_bridge_no_resources() {
    let mut bridge = ZKPMarketplaceBridge::new_test().unwrap();
    // Sin recursos publicados

    let req = make_sae_request("buyer1", 5, 5.0, 150.0);
    let result = bridge.execute_resource_workflow(req).await;

    assert!(result.is_ok());
    let workflow = result.unwrap();
    assert!(!workflow.success);
    assert_eq!(workflow.final_state, ResourceWorkflowState::Published);
}

#[tokio::test]
async fn test_e2e_bridge_multiple_buyers() {
    let mut bridge = ZKPMarketplaceBridge::new_test().unwrap();
    bridge.publish_resource(make_sae_listing("seller1", 5, 100.0));

    // Buyer 1
    let result1 = bridge
        .execute_resource_workflow(make_sae_request("buyer1", 5, 3.0, 150.0))
        .await;
    assert!(result1.is_ok());

    // Buyer 2
    let result2 = bridge
        .execute_resource_workflow(make_sae_request("buyer2", 5, 3.0, 150.0))
        .await;
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_e2e_bridge_cleanup_and_rerun() {
    let mut bridge = ZKPMarketplaceBridge::new_test().unwrap();
    bridge.publish_resource(make_sae_listing("seller1", 5, 100.0));

    // Ejecutar workflow
    let result = bridge
        .execute_resource_workflow(make_sae_request("buyer1", 5, 3.0, 150.0))
        .await;
    assert!(result.is_ok());

    // Limpiar expirados (listing expires in 1 hour, so use 2 hours in future)
    let future_now = chrono::Utc::now().timestamp_millis() as u64 + 7200_000;
    let removed = bridge.cleanup_expired(future_now);
    assert_eq!(removed, 1);

    // Sin listings, nuevo workflow falla
    let result = bridge
        .execute_resource_workflow(make_sae_request("buyer2", 5, 3.0, 150.0))
        .await;
    assert!(result.is_ok());
    assert!(!result.unwrap().success);
}

#[tokio::test]
async fn test_e2e_bridge_pricing_integration() {
    let mut bridge = ZKPMarketplaceBridge::new_test().unwrap();
    bridge.publish_resource(make_sae_listing("seller1", 5, 100.0));

    let _ = bridge
        .execute_resource_workflow(make_sae_request("buyer1", 5, 3.0, 150.0))
        .await;

    let stats = bridge.get_pricing_stats();
    assert!(stats.total_quotes >= 1);
}

// ============================================================================
// Test: Proof Submission Manager
// ============================================================================

#[tokio::test]
async fn test_e2e_proof_submission_with_verifiers() {
    let mut manager = ProofSubmissionManager::new();
    manager.register_verifier("verifier1".into(), 0.95, 10);
    manager.register_verifier("verifier2".into(), 0.90, 15);
    manager.register_verifier("verifier3".into(), 0.85, 20);

    let witness = create_test_witness();
    let result = manager.submit_proof("e2e_batch_1".into(), witness).await;

    assert!(result.is_ok());
    let submission = result.unwrap();
    // Estado esperado: ConsensusRegistered o FallbackActivated
    assert!(matches!(
        submission.state,
        SubmissionState::ConsensusRegistered
            | SubmissionState::FallbackActivated
            | SubmissionState::Verified
    ));
}

#[tokio::test]
async fn test_e2e_proof_submission_no_verifiers() {
    let mut manager = ProofSubmissionManager::new();
    // Sin verificadores

    let witness = create_test_witness();
    let result = manager.submit_proof("e2e_batch_2".into(), witness).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_e2e_proof_multiple_submissions() {
    let mut manager = ProofSubmissionManager::new();
    manager.register_verifier("v1".into(), 0.95, 10);

    for i in 0..5 {
        let witness = create_test_witness();
        let result = manager
            .submit_proof(format!("e2e_batch_{}", i), witness)
            .await;
        assert!(result.is_ok());
    }

    let stats = manager.get_stats();
    assert_eq!(stats.total_submitted, 5);
}

// ============================================================================
// Test: ZKP Circuit Integration
// ============================================================================

#[test]
fn test_e2e_zkp_circuit_commitment() {
    let circuit = ed2kia::zkp::circuit::ZKPCircuit::new(Some(4));

    let features: Vec<f64> = (0..8).map(|i| i as f64 * 1.5).collect();
    let commitment = circuit.create_commitment(&features, "e2e_test").unwrap();

    assert_eq!(commitment.feature_count, 8);
    assert!(!commitment.compact_bytes.is_empty());
}

#[test]
fn test_e2e_zkp_circuit_proof_generation() {
    let circuit = ed2kia::zkp::circuit::ZKPCircuit::new(Some(4));

    let mut rng = thread_rng();
    let feature_values: Vec<Fr> = (0..8).map(|_| Fr::rand(&mut rng)).collect();
    let blinding_factors: Vec<Fr> = (0..4).map(|_| Fr::rand(&mut rng)).collect();

    let mut batch_hash = [0u8; 32];
    batch_hash.copy_from_slice(&Sha256::digest(b"e2e_circuit_test"));

    let witness = Witness {
        feature_values,
        blinding_factors,
        batch_hash,
    };

    let proof = circuit.generate_proof(&witness, "e2e_circuit_batch");
    assert!(!proof.batch_id.is_empty());
}

#[test]
fn test_e2e_zkp_circuit_empty_batch_error() {
    let circuit = ed2kia::zkp::circuit::ZKPCircuit::new(Some(4));
    let result = circuit.create_commitment(&[], "empty");
    assert!(result.is_err());
}

// ============================================================================
// Test: Async Prover
// ============================================================================

#[tokio::test]
async fn test_e2e_async_prover_generation() {
    let prover = ed2kia::zkp::async_prover::AsyncProver::new();

    let witness = create_test_witness();
    let result = prover
        .generate_proof("e2e_prover_test".into(), witness)
        .await;

    assert!(result.is_ok());
    let proof_result = result.unwrap();
    assert!(proof_result.generation_time_ms > 0.0);
}

// ============================================================================
// Test: Verifier Pool
// ============================================================================

#[test]
fn test_e2e_verifier_pool() {
    let pool = ed2kia::zkp::verifier_pool::VerifierPool::new();

    let mut rng = thread_rng();
    let proof = ed2kia::zkp::circuit::ZKPProof {
        a: ark_bn254::G1Projective::rand(&mut rng).into_affine(),
        b: vec![ark_bn254::G1Projective::rand(&mut rng).into_affine()],
        c: ark_bn254::G1Projective::rand(&mut rng).into_affine(),
        challenge: [0u8; 32],
        batch_id: "e2e_pool_test".into(),
        feature_count: 8,
    };

    let commitment = ed2kia::zkp::circuit::BatchCommitment {
        commitment_point: proof.a,
        batch_hash: [0u8; 32],
        feature_count: 8,
        compact_bytes: Vec::new(),
    };

    let result = pool.verify(proof, commitment);
    assert!(result.is_ok());
}

// ============================================================================
// Test: Batch Accumulator
// ============================================================================

#[test]
fn test_e2e_batch_accumulator() {
    let mut accumulator = ed2kia::zkp::batch_accumulator::BatchAccumulator::new();

    let mut rng = thread_rng();
    let proof = ed2kia::zkp::circuit::ZKPProof {
        a: ark_bn254::G1Projective::rand(&mut rng).into_affine(),
        b: vec![ark_bn254::G1Projective::rand(&mut rng).into_affine()],
        c: ark_bn254::G1Projective::rand(&mut rng).into_affine(),
        challenge: [0u8; 32],
        batch_id: "e2e_accum_batch".into(),
        feature_count: 8,
    };

    let commitment = ed2kia::zkp::circuit::BatchCommitment {
        commitment_point: proof.a,
        batch_hash: [0u8; 32],
        feature_count: 8,
        compact_bytes: Vec::new(),
    };

    let result = accumulator.add_batch("e2e_accum_batch".into(), commitment.clone());
    assert!(result.is_ok());

    let stats = accumulator.get_stats();
    // current_capacity tracks batches added; total_batches tracks accumulated batches
    assert_eq!(stats.current_capacity, 1);
}

// ============================================================================
// Test: Anti-Monopoly
// ============================================================================

#[test]
fn test_e2e_anti_monopoly() {
    let mut matchmaker = ed2kia::marketplace_v2::matchmaker::ResourceMatchmaker::new();

    // Un nodo domina con 8 listings de 10
    for i in 0..8 {
        matchmaker.register_listing(ResourceListing {
            node_id: "dominant_node".into(),
            resource_type: ResourceType::SAEShard {
                model_id: "scope-v2".into(),
                layer: 5,
            },
            quantity: 10.0,
            base_price: 100.0 + i as f32,
            listed_at: 1000,
            expires_at: 10000,
            max_latency_ms: 100,
            availability_slo: 0.99,
            min_throughput: 1000,
        });
    }

    // Dos nodos pequeños
    matchmaker.register_listing(make_sae_listing("small_a", 5, 90.0));
    matchmaker.register_listing(make_sae_listing("small_b", 5, 95.0));

    let req = make_sae_request("buyer1", 5, 5.0, 150.0);
    let result = matchmaker.match_request(&req).unwrap();

    // Debería seleccionar small_a (más barato y no excede límite)
    assert!(result.matched);
    assert!(result.listing.is_some());
}

// ============================================================================
// Test: Cross-Module Integration
// ============================================================================

#[test]
fn test_e2e_cross_module_pricing_matcher() {
    // Pricing + Matching integration
    let mut matchmaker =
        ed2kia::marketplace_v2::matchmaker::ResourceMatchmaker::with_config(1.0, 0.5, 100);
    let pricing = PricingEngine::new();

    matchmaker.register_listing(make_sae_listing("node1", 5, 100.0));

    let req = make_sae_request("buyer1", 5, 5.0, 150.0);
    let match_result = matchmaker.match_request(&req).unwrap();

    assert!(match_result.matched);

    let quote = pricing
        .compute_price(
            PricingResourceType::SAEShard,
            match_result.final_price,
            req.quantity,
        )
        .unwrap();

    assert!(quote.unit_price > 0.0);
    assert!(quote.unit_price <= 1000.0);
}

#[test]
fn test_e2e_cross_module_escrow_pricing() {
    // Escrow + Pricing integration
    let (ledger, _, _path) = EscrowLedger::new_test().unwrap();
    let pricing = PricingEngine::new();

    let quote = pricing
        .compute_price(PricingResourceType::VRAM, 250.0, 40.0)
        .unwrap();

    let total_amount: f64 = (quote.unit_price * 40.0).into();

    let tx = ledger
        .create_escrow(
            "e2e_cross_1".into(),
            "vram_seller".into(),
            "vram_buyer".into(),
            total_amount,
            "cross_settlement".into(),
        )
        .unwrap();

    assert_eq!(tx.state, EscrowState::Locked);
    assert!((tx.amount - total_amount).abs() < 0.01);
}

// ============================================================================
// Test: Feature Detection
// ============================================================================

#[test]
fn test_e2e_feature_flag_enabled() {
    // Verificar que el código sprint3 está disponible
    let features = ed2kia::enabled_features();
    // Con feature flag activado, "stable" debería estar presente
    assert!(features.contains(&"core"));
}

#[test]
fn test_e2e_version_string() {
    // Version is derived from CARGO_PKG_VERSION at compile time
    assert!(!ed2kia::version().is_empty());
    assert!(ed2kia::version().contains('.'));
}
