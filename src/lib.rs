//! ed2kIA v1.6.0-stable
//! Building on v1.5.0 STABLE — Cross-Chain Interoperability, Advanced ML Alignment, Governance v6
//!
//! Descentralized distributed interpretability network using Sparse Autoencoders.
//!
//! This library unifies all modules from Fases 1-9 plus v1.1.0 sprints into a single stable API.
//! All feature flags are enabled by default via the `stable` feature.
//!
//! # Features
//!
//! - `stable` (default): All validated modules from Fases 1-9 + v1.1.0 sprints
//! - `cuda` / `metal`: Hardware acceleration for Candle backend
//! - `debug` / `test-mocks`: Development-only features
//!
//! # License
//!
//! Apache 2.0 + Ethical Use Clause

#![allow(dead_code)]

// ============================================================================
// Version
// ============================================================================

/// Semantic version of ed2kIA
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Sprint identifier for build tracking
pub const SPRINT_IDENTIFIER: &str = "v1.6.0-stable";

// ============================================================================
// Fase 1: Core Modules (P2P, SAE, Bridge)
// ============================================================================

/// P2P networking layer (libp2p-based)
pub mod p2p {
    pub mod swarm;
    pub mod protocol;
    #[cfg(feature = "v1.8-sprint2")]
    pub mod geographic_routing;
}

/// WASM mobile bridge (feature-gated)
#[cfg(feature = "v1.8-sprint2")]
#[path = "wasm/mobile_bridge.rs"]
pub mod mobile_bridge;

/// Sparse Autoencoder (SAE) loading and routing
pub mod sae {
    pub mod loader;
    pub mod router;
    #[cfg(feature = "v1.2-sprint1")]
    pub mod fine_tuning_engine;
    #[cfg(feature = "v1.2-sprint2")]
    pub mod distributed_finetune;
    #[cfg(feature = "v1.2-sprint2")]
    pub mod checkpoint_manager;
    #[cfg(feature = "v1.2-sprint2")]
    pub mod fault_tolerance;
    #[cfg(feature = "v1.3-sprint3")]
    pub mod fine_tuning_v3;
    #[cfg(feature = "v1.3-sprint3")]
    pub mod cross_model_aligner;
    #[cfg(feature = "v1.3-sprint3")]
    pub mod adaptive_checkpoint;
    #[cfg(feature = "v1.4-sprint3")]
    pub mod fine_tuning_v4;
    #[cfg(feature = "v1.4-sprint3")]
    pub mod cross_model_aligner_v2;
    #[cfg(feature = "v1.4-sprint3")]
    pub mod adaptive_checkpoint_v2;
    #[cfg(feature = "v1.5-sprint1")]
    pub mod fine_tuning_v5;
    #[cfg(feature = "v1.5-sprint1")]
    pub mod cross_model_aligner_v3;
    #[cfg(feature = "v1.5-sprint1")]
    pub mod adaptive_checkpoint_v3;
    #[cfg(feature = "v1.5-sprint3")]
    pub mod fine_tuning_v6;
    #[cfg(feature = "v1.5-sprint3")]
    pub mod cross_model_aligner_v4;
    #[cfg(feature = "v1.5-sprint3")]
    pub mod adaptive_checkpoint_v4;
    #[cfg(feature = "v1.6-sprint3")]
    pub mod fine_tuning_v7;
    #[cfg(feature = "v1.6-sprint3")]
    pub mod cross_model_aligner_v5;
    #[cfg(feature = "v1.6-sprint3")]
    pub mod adaptive_checkpoint_v5;
}

/// Tensor flow and consciousness bridge
pub mod bridge {
    pub mod tensor_flow;
    pub mod consciousness;
    #[cfg(feature = "v1.5-sprint2")]
    #[path = "../bridge/federation_zkp_bridge_v4.rs"]
    pub mod federation_zkp_bridge_v4;
    #[cfg(feature = "v1.6-sprint1")]
    #[path = "../bridge/cross_chain_bridge_v3.rs"]
    pub mod cross_chain_bridge_v3;
    #[cfg(feature = "v1.6-sprint1")]
    #[path = "../bridge/bridge_validator.rs"]
    pub mod bridge_validator;
    #[cfg(feature = "v1.6-sprint1")]
    #[path = "../bridge/relay_manager.rs"]
    pub mod relay_manager;
    #[cfg(feature = "v1.6-sprint2")]
    #[path = "../bridge/federation_zkp_bridge_v6.rs"]
    pub mod federation_zkp_bridge_v6;
    #[cfg(feature = "v1.6-sprint3")]
    #[path = "../bridge/federation_zkp_bridge_v7.rs"]
    pub mod federation_zkp_bridge_v7;
    #[cfg(feature = "v1.7-sprint1")]
    #[path = "../bridge/quantization.rs"]
    pub mod quantization;
}

// ============================================================================
// Protocol Layer — Async Steering & Latency Mitigation (RFC-001)
// ============================================================================

#[cfg(feature = "v1.7-sprint1")]
pub mod protocol {
    #[path = "../protocol/async_steering.rs"]
    pub mod async_steering;
}

// ============================================================================
// Fase 2: Interpretation, Feedback & Consensus
// ============================================================================

/// Feature interpretation and semantic mapping
pub mod interpret {
    pub mod feature_analyzer;
    pub mod semantic_map;
}

/// Distributed consensus validation
pub mod consensus {
    pub mod validator;
    pub mod merkle;
}

// ============================================================================
// Fase 3: Security, ZKP, Human-in-the-Loop
// ============================================================================

/// Security modules (WASM sandbox, memory guard)
pub mod security {
    pub mod wasm_sandbox;
    pub mod memory_guard;
    pub mod wasm_sandbox_v2;
    pub mod wasm_profiler;
}

/// Zero-Knowledge Proof circuits and verification
pub mod zkp {
    pub mod circuit;
    pub mod verifier;
    pub mod async_prover;
    pub mod verifier_pool;
    pub mod batch_accumulator;
    #[cfg(feature = "v1.3-sprint2")]
    #[path = "../zkp/async_zkp_v4.rs"]
    pub mod async_zkp_v4;
    #[cfg(feature = "v1.3-sprint3")]
    #[path = "../zkp/async_zkp_v5.rs"]
    pub mod async_zkp_v5;
    #[cfg(feature = "v1.4-sprint1")]
    #[path = "../zkp/halo2_engine.rs"]
    pub mod halo2_engine;
    #[cfg(feature = "v1.4-sprint1")]
    #[path = "../zkp/circuit_optimizer.rs"]
    pub mod circuit_optimizer;
    #[cfg(feature = "v1.4-sprint1")]
    #[path = "../zkp/proof_aggregator.rs"]
    pub mod proof_aggregator;
    #[cfg(feature = "v1.4-sprint3")]
    #[path = "../zkp/async_zkp_v8.rs"]
    pub mod async_zkp_v8;
    #[cfg(feature = "v1.4-sprint3")]
    #[path = "../zkp/cross_federation_verification.rs"]
    pub mod cross_federation_verification;
    #[cfg(feature = "v1.5-sprint2")]
    #[path = "../zkp/async_zkp_v10.rs"]
    pub mod async_zkp_v10;
    #[cfg(feature = "v1.5-sprint3")]
    #[path = "../zkp/async_zkp_v11.rs"]
    pub mod async_zkp_v11;
    #[cfg(feature = "v1.5-sprint3")]
    #[path = "../zkp/cross_federation_verifier_v2.rs"]
    pub mod cross_federation_verifier_v2;
    #[cfg(feature = "v1.6-sprint2")]
    #[path = "../zkp/async_zkp_v13.rs"]
    pub mod async_zkp_v13;
    #[cfg(feature = "v1.6-sprint3")]
    #[path = "../zkp/async_zkp_v14.rs"]
    pub mod async_zkp_v14;
}

/// Marketplace v2 — Resource matching, escrow ledger, adaptive pricing
#[cfg(feature = "stable")]
pub mod marketplace_v2 {
    #[path = "../marketplace/matchmaker.rs"]
    pub mod matchmaker;
    #[path = "../marketplace/escrow_ledger.rs"]
    pub mod escrow_ledger;
    #[path = "../marketplace/pricing_engine.rs"]
    pub mod pricing_engine;
}

/// Bridge — ZKP ↔ Marketplace integration and proof submission
#[cfg(feature = "stable")]
pub mod bridge_v2 {
    #[path = "../bridge/zkp_marketplace_bridge.rs"]
    pub mod zkp_marketplace_bridge;
    #[path = "../bridge/proof_submission.rs"]
    pub mod proof_submission;
}

/// Pool ZKP Bridge — Cross-pool ZKP verification
#[cfg(feature = "v1.3-sprint2")]
#[path = "bridge/pool_zkp_bridge.rs"]
pub mod pool_zkp_bridge;

/// Federation ZKP Bridge — Cross-shard ZKP verification for federation
#[cfg(feature = "v1.3-sprint3")]
#[path = "bridge/federation_zkp_bridge.rs"]
pub mod federation_zkp_bridge;

/// Human-in-the-loop feedback and concept updates
pub mod human {
    pub mod feedback_cli;
    pub mod concept_updater;
}

// ============================================================================
// Fase 4: Scaling, RLHF, Web UI, Monitoring
// ============================================================================

/// Peer scaling and bootstrap management
#[cfg(feature = "stable")]
pub mod scaling {
    pub mod peer_manager;
    pub mod bootstrap;
    #[cfg(feature = "phase8-sprint2")]
    pub mod cross_model;
    pub mod predictive_balancer;
}

/// RLHF training loop and feedback store
#[cfg(feature = "stable")]
pub mod rlhf {
    pub mod feedback_store;
    pub mod trainer_loop;
}

/// Web server and API routes
#[cfg(feature = "stable")]
pub mod web {
    pub mod server;
    pub mod routes;
    pub mod realtime;
    pub mod ws_alignment_stream;
    pub mod sse_metrics;
    pub mod ws_dashboard_stream;
    #[cfg(feature = "v1.5-sprint2")]
    pub mod ws_federation_stream;
    #[cfg(feature = "v1.6-sprint2")]
    #[path = "../web/ws_federation_stream_v2.rs"]
    pub mod ws_federation_stream_v2;
}

/// Prometheus metrics and health monitoring
#[cfg(feature = "stable")]
pub mod monitoring {
    pub mod metrics;
    pub mod health;
    pub mod streaming_metrics;
}

/// Runtime optimization — Tokio tuning, task scheduling, worker pools
#[cfg(feature = "v1.4-sprint1")]
pub mod runtime {
    #[path = "../runtime/tokio_optimizer.rs"]
    pub mod tokio_optimizer;
    #[path = "../runtime/task_scheduler.rs"]
    pub mod task_scheduler;
    #[path = "../runtime/worker_pool.rs"]
    pub mod worker_pool;
}

/// Storage — LZ4 compression, checkpoint cache, gradient archive
#[cfg(feature = "v1.4-sprint1")]
pub mod storage {
    #[path = "../storage/lz4_compressor.rs"]
    pub mod lz4_compressor;
    #[path = "../storage/checkpoint_cache.rs"]
    pub mod checkpoint_cache;
    #[path = "../storage/gradient_archive.rs"]
    pub mod gradient_archive;
}

/// Monitoring v2 — Advanced metrics, health checking, alert engine
#[cfg(feature = "v1.4-sprint1")]
pub mod monitoring_v2 {
    #[path = "../monitoring/advanced_metrics.rs"]
    pub mod advanced_metrics;
    #[path = "../monitoring/health_checker.rs"]
    pub mod health_checker;
    #[path = "../monitoring/alert_engine.rs"]
    pub mod alert_engine;
}

// ============================================================================
// Fase 5: Governance, Reputation, Ecosystem, Bootstrap
// ============================================================================

/// Proposal and voting governance
#[cfg(feature = "stable")]
pub mod governance {
    pub mod proposal;
    pub mod voting;
    pub mod liquid_v2;
    pub mod voting_mechanism;
    #[cfg(feature = "v1.2-sprint2")]
    pub mod dao_v3;
    #[cfg(feature = "v1.2-sprint2")]
    pub mod hybrid_voting;
    #[cfg(feature = "v1.2-sprint2")]
    pub mod proposal_executor;
    #[cfg(feature = "v1.3-sprint2")]
    pub mod dao_ledger_v2;
    #[cfg(feature = "v1.3-sprint2")]
    pub mod technical_staking;
    #[cfg(feature = "v1.3-sprint2")]
    pub mod proposal_tracker;
}

/// Reputation ledger and scoring
#[cfg(feature = "stable")]
pub mod reputation {
    pub mod ledger;
    pub mod scoring;
    #[cfg(feature = "v1.8-sprint1")]
    #[path = "../reputation/proof_schema.rs"]
    pub mod proof_schema;
}

// ============================================================================
// API Explorer — v1.8 "ChatGPT Moment" Sprint
// ============================================================================

/// API Explorer v1 — 3D concept visualization endpoints
#[cfg(feature = "v1.8-sprint1")]
pub mod api_explorer {
    #[path = "../api/explorer_v1.rs"]
    pub mod explorer_v1;
}

/// Ecosystem sync (HuggingFace, model registry)
#[cfg(feature = "stable")]
pub mod ecosystem {
    pub mod hf_sync;
    pub mod model_registry;
}

/// Network bootstrap and seed registry
#[cfg(feature = "stable")]
pub mod bootstrap {
    pub mod seed_registry;
    pub mod network_init;
}

// ============================================================================
// Fase 6: Interoperability, Federation, Staking, API
// ============================================================================

/// Cross-model interoperability adapters
#[cfg(feature = "stable")]
pub mod interoperability {
    pub mod adapter;
    pub mod schema;
    pub mod schema_registry;
    pub mod onnx_adapter;
    pub mod capability_registry;
    pub mod cross_model_router;
    pub mod adaptive_router_v2;
}

/// Interoperability Layer v2 — Cross-federation communication with schema negotiation
#[cfg(feature = "v1.6-sprint1")]
pub mod interop {
    #[path = "../interop/interop_layer_v2.rs"]
    pub mod interop_layer_v2;
    #[path = "../interop/protocol_adapter.rs"]
    pub mod protocol_adapter;
    #[path = "../interop/schema_negotiator.rs"]
    pub mod schema_negotiator;
}

/// State Sync v2 — State synchronization with Merkle verification and snapshot management
#[cfg(feature = "v1.6-sprint1")]
pub mod state {
    #[path = "../state/state_sync_v2.rs"]
    pub mod state_sync_v2;
    #[path = "../state/merkle_aggregator.rs"]
    pub mod merkle_aggregator;
    #[path = "../state/snapshot_manager.rs"]
    pub mod snapshot_manager;
}

/// Federation protocols (FedAvg, sync)
#[cfg(feature = "stable")]
pub mod federation {
    pub mod avg_aggregator;
    pub mod sync_protocol;
    pub mod gradient_normalizer;
    pub mod trust_sync;
    pub mod cross_model_scaler;
    #[cfg(feature = "v1.2-sprint1")]
    pub mod multi_chain_registry;
    #[cfg(feature = "v1.2-sprint1")]
    pub mod cross_chain_identity;
    #[cfg(feature = "v1.2-sprint1")]
    pub mod gradient_aggregator_v3;
    #[cfg(feature = "v1.2-sprint2")]
    pub mod cross_chain_consensus;
    #[cfg(feature = "v1.2-sprint2")]
    pub mod bridge_validator;
    #[cfg(feature = "v1.3-sprint3")]
    pub mod dynamic_sharder;
    #[cfg(feature = "v1.4-sprint3")]
    pub mod scaling_v4;
    #[cfg(feature = "v1.4-sprint3")]
    pub mod predictive_sharder_v4;
    #[cfg(feature = "v1.5-sprint2")]
    pub mod scaling_v5;
    #[cfg(feature = "v1.5-sprint2")]
    pub mod predictive_sharder_v5;
    #[cfg(feature = "v1.5-sprint2")]
    pub mod gradient_sync_v5;
    #[cfg(feature = "v1.5-sprint3")]
    pub mod scaling_v6;
    #[cfg(feature = "v1.5-sprint3")]
    pub mod dynamic_sharder_v2;
    #[cfg(feature = "v1.5-sprint3")]
    pub mod gradient_sync_v6;
    #[cfg(feature = "v1.6-sprint2")]
    pub mod scaling_v7;
    #[cfg(feature = "v1.6-sprint2")]
    pub mod adaptive_router_v2;
    #[cfg(feature = "v1.6-sprint2")]
    pub mod gradient_sync_v7;
    #[cfg(feature = "v1.6-sprint3")]
    pub mod cross_model_scaling_v7;
    #[cfg(feature = "v1.6-sprint3")]
    pub mod predictive_sharder_v3;
}

/// Staking proof and registry
#[cfg(feature = "stable")]
pub mod staking {
    pub mod proof;
    pub mod registry;
}

/// API v2 (OpenAPI, routes, auth)
#[cfg(feature = "stable")]
pub mod api {
    pub mod openapi;
    pub mod routes;
    pub mod auth;
}

// ============================================================================
// Fase 7: Continuous Alignment, Cross-Net Federation, Dynamic Trust
// ============================================================================

/// Continuous alignment engine
#[cfg(feature = "stable")]
pub mod alignment {
    pub mod engine;
    #[cfg(feature = "phase8-sprint2")]
    pub mod continuous;
    pub mod loop_v2;
    pub mod steering_engine;
    pub mod confidence_calculator;
    #[cfg(feature = "v1.2-sprint1")]
    pub mod ethical_constraint_engine;
}

/// Cross-network federation bridge and trust scoring
#[cfg(feature = "stable")]
pub mod federation_v2 {
    #[path = "../federation/bridge.rs"]
    pub mod bridge;
    #[path = "../federation/trust_scoring.rs"]
    pub mod trust_scoring;
}

// ============================================================================
// Fase 8: Marketplace, UI Backend, SLO Engine, Cross-Model Scaling
// ============================================================================

/// Resource marketplace engine
#[cfg(feature = "stable")]
pub mod marketplace {
    pub mod engine;
}

/// UI Backend (REST + WebSocket)
#[cfg(feature = "stable")]
pub mod ui {
    pub mod backend;
    pub mod realtime_backend;
    pub mod dashboard_v2;
    pub mod dashboard_v6;
    #[cfg(feature = "v1.6-sprint2")]
    #[path = "../ui/dashboard_v7.rs"]
    pub mod dashboard_v7;
}

/// SLO tracking and enforcement
#[cfg(feature = "stable")]
pub mod slo {
    pub mod engine;
    #[cfg(feature = "phase8-sprint2")]
    pub mod enforcer;
    pub mod dynamic_engine;
    pub mod contract_manager;
}

// ============================================================================
// Fase 9: Liquid Governance, Real-Time UI, Async ZKP Federation
// ============================================================================

/// Liquid governance with delegation and anti-Sybil
#[cfg(feature = "stable")]
pub mod governance_v2 {
    #[path = "../governance/liquid.rs"]
    pub mod liquid;
}

/// Real-time UI backend (WebSocket/SSE)
#[cfg(feature = "stable")]
pub mod ui_v2 {
    #[path = "../ui/realtime.rs"]
    pub mod realtime;
}

/// Async ZKP Federation with proof batching
#[cfg(feature = "stable")]
pub mod federation_v3 {
    #[path = "../federation/async_zkp.rs"]
    pub mod async_zkp;
}

// ============================================================================
// v1.1.0 Sprint 1: FedAvg v2 with Gradient Compression
// ============================================================================

/// FedAvg v2 with gradient compression and optimized Krum.
#[cfg(feature = "stable")]
pub mod federation_v2_sprint1 {
    #[path = "../federation/avg_aggregator_v2.rs"]
    pub mod avg_aggregator_v2;
    #[path = "../federation/gradient_compressor.rs"]
    pub mod gradient_compressor;
}

// ============================================================================
// v1.2.0 Sprint 3: SLO/SLA v3, Cross-Model Scaling v2, UI Dashboard v3 & Async ZKP v2
// ============================================================================

/// SLO/SLA v3 with predictive contracts
#[cfg(feature = "v1.2-sprint3")]
pub mod slo_v3 {
    #[path = "../slo/slo_v3.rs"]
    pub mod slo_v3_engine;
    #[path = "../slo/predictive_contracts.rs"]
    pub mod predictive_contracts;
}

/// Cross-Model Scaling v2 with capability negotiation
#[cfg(feature = "v1.2-sprint3")]
pub mod scaling_v3 {
    #[path = "../scaling/cross_model_v2.rs"]
    pub mod cross_model_v2;
    #[path = "../scaling/capability_negotiator.rs"]
    pub mod capability_negotiator;
}

/// UI Dashboard v3 with cross-chain/DAO/fine-tuning streams
#[cfg(feature = "v1.2-sprint3")]
pub mod ui_v3 {
    #[path = "../ui/dashboard_v3.rs"]
    pub mod dashboard_v3;
}

/// Async ZKP v2 optimized for cross-chain proofs
#[cfg(feature = "v1.2-sprint3")]
pub mod zkp_v3 {
    #[path = "../zkp/async_zkp_v2.rs"]
    pub mod async_zkp_v2;
    #[path = "../zkp/cross_chain_proof_optimizer.rs"]
    pub mod cross_chain_proof_optimizer;
}

// ============================================================================
// v1.2.0 Sprint 4: Marketplace v3, Alignment Loop v3, Federation Scaling v3 & Final Consolidation
// ============================================================================

/// Marketplace v3 with cross-chain settlement and reputation matching
#[cfg(feature = "v1.2-sprint4")]
pub mod marketplace_v3 {
    #[path = "../marketplace/marketplace_v3.rs"]
    #[allow(clippy::module_inception)]
    pub mod marketplace_v3;
    #[path = "../marketplace/cross_chain_settlement.rs"]
    pub mod cross_chain_settlement;
    #[path = "../marketplace/reputation_matcher.rs"]
    pub mod reputation_matcher;
}

/// Alignment Loop v3 with ZKP steering verification and bias mitigation
#[cfg(feature = "v1.2-sprint4")]
pub mod alignment_v3 {
    #[path = "../alignment/loop_v3.rs"]
    pub mod loop_v3;
    #[path = "../alignment/steering_verifier.rs"]
    pub mod steering_verifier;
    #[path = "../alignment/bias_mitigator.rs"]
    pub mod bias_mitigator;
}

/// Federation Scaling v3 with adaptive sharding and gradient sync
#[cfg(feature = "v1.2-sprint4")]
pub mod federation_scaling_v3 {
    #[path = "../federation/scaling_v3.rs"]
    pub mod scaling_v3;
    #[path = "../federation/adaptive_sharder.rs"]
    pub mod adaptive_sharder;
    #[path = "../federation/gradient_sync_v3.rs"]
    pub mod gradient_sync_v3;
}

// ============================================================================
// v1.3.0 Sprint 1 Modules
// ============================================================================

/// SAE Fine-Tuning v2 with checkpoint optimization and gradient sync
#[cfg(feature = "v1.3-sprint1")]
pub mod sae_v2 {
    #[path = "../sae/fine_tuning_v2.rs"]
    pub mod fine_tuning_v2;
    #[path = "../sae/checkpoint_optimizer.rs"]
    pub mod checkpoint_optimizer;
    #[path = "../sae/gradient_sync_v2.rs"]
    pub mod gradient_sync_v2;
}

/// Cross-Node Compute Routing with predictive scheduling and load balancing
#[cfg(feature = "v1.3-sprint1")]
pub mod routing_v2 {
    #[path = "../routing/cross_node_router.rs"]
    pub mod cross_node_router;
    #[path = "../routing/predictive_scheduler.rs"]
    pub mod predictive_scheduler;
    #[path = "../routing/load_balancer.rs"]
    pub mod load_balancer;
}

/// Community Reputation Ledger v2 with anti-Sybil and merit scoring
#[cfg(feature = "v1.3-sprint1")]
pub mod reputation_v2 {
    #[path = "../reputation/ledger_v2.rs"]
    pub mod ledger_v2;
    #[path = "../reputation/anti_sybil.rs"]
    pub mod anti_sybil;
    #[path = "../reputation/merit_scoring.rs"]
    pub mod merit_scoring;
}

/// Async ZKP v3 with federation bridge
#[cfg(feature = "v1.3-sprint1")]
pub mod zkp_v3_sprint1 {
    #[path = "../zkp/async_zkp_v3.rs"]
    pub mod async_zkp_v3;
    #[path = "../bridge/zkp_federation_bridge.rs"]
    pub mod zkp_federation_bridge;
}

/// Technical Cross-Chain Resource Pools (v1.3-sprint2)
#[cfg(feature = "v1.3-sprint2")]
pub mod pools {
    #[path = "../pools/cross_chain_resource_pool.rs"]
    pub mod cross_chain_resource_pool;
    #[path = "../pools/shard_aggregator.rs"]
    pub mod shard_aggregator;
    #[path = "../pools/pool_matcher.rs"]
    pub mod pool_matcher;
}

/// ZKP Aggregation & Neural Steer UI (v1.9-sprint2)
#[cfg(feature = "v1.9-sprint2")]
#[path = "zkp/proof_aggregation.rs"]
pub mod proof_aggregation;

#[cfg(feature = "v1.9-sprint2")]
#[path = "gui/neural_steer_ui.rs"]
pub mod neural_steer_ui;

/// UI Dashboard v4 & Real-time Streams (v1.3-sprint2)
#[cfg(feature = "v1.3-sprint2")]
pub mod ui_v4 {
    #[path = "../ui/dashboard_v4.rs"]
    pub mod dashboard_v4;
    #[path = "../ui/pool_stream_engine.rs"]
    pub mod pool_stream_engine;
}

/// Cross-Chain Technical Pools v3 (v1.4-sprint2)
#[cfg(feature = "v1.4-sprint2")]
pub mod pools_v3 {
    #[path = "../pools/cross_chain_pools_v3.rs"]
    pub mod cross_chain_pools_v3;
    #[path = "../pools/dynamic_aggregator.rs"]
    pub mod dynamic_aggregator;
    #[path = "../pools/capacity_negotiator.rs"]
    pub mod capacity_negotiator;
}

/// Cross-Chain Pools v4 & Dynamic Routing (v1.5-sprint1)
#[cfg(feature = "v1.5-sprint1")]
pub mod pools_v4 {
    #[path = "../pools/cross_chain_pools_v4.rs"]
    pub mod cross_chain_pools_v4;
    #[path = "../pools/dynamic_router.rs"]
    pub mod dynamic_router;
    #[path = "../pools/capacity_orchestrator.rs"]
    pub mod capacity_orchestrator;
}

/// DAO Ledger v4 & Hybrid Execution (v1.4-sprint2)
#[cfg(feature = "v1.4-sprint2")]
pub mod governance_v4 {
    #[path = "../governance/dao_ledger_v4.rs"]
    pub mod dao_ledger_v4;
    #[path = "../governance/hybrid_executor.rs"]
    pub mod hybrid_executor;
    #[path = "../governance/audit_trail.rs"]
    pub mod audit_trail;
}

/// DAO Ledger v5 & Hybrid Governance (v1.5-sprint1)
#[cfg(feature = "v1.5-sprint1")]
pub mod governance_v5 {
    #[path = "../governance/dao_ledger_v5.rs"]
    pub mod dao_ledger_v5;
    #[path = "../governance/hybrid_governance.rs"]
    pub mod hybrid_governance;
    #[path = "../governance/audit_trail_v2.rs"]
    pub mod audit_trail_v2;
}

/// Async ZKP v7 & Cross-Pool Verification (v1.4-sprint2)
#[cfg(feature = "v1.4-sprint2")]
pub mod zkp_v7 {
    #[path = "../zkp/async_zkp_v7.rs"]
    pub mod async_zkp_v7;
    #[path = "../zkp/cross_pool_verification.rs"]
    pub mod cross_pool_verification;
}

/// Async ZKP v9 & Cross-Federation Verification (v1.5-sprint1)
#[cfg(feature = "v1.5-sprint1")]
pub mod zkp_v9 {
    #[path = "../zkp/async_zkp_v9.rs"]
    pub mod async_zkp_v9;
}

/// v1.9.0 Sprint 1 — Production Hardening & Mobile GUI Foundation (FASE 69)
#[cfg(feature = "v1.9-sprint1")]
#[path = "gui/mobile_foundation.rs"]
pub mod mobile_foundation;

#[cfg(feature = "v1.9-sprint1")]
#[path = "zkp/circuit_optimization.rs"]
pub mod circuit_optimization;

/// WebSocket Pool Stream (v1.3-sprint2)
#[cfg(feature = "v1.3-sprint2")]
#[path = "web/ws_pool_stream.rs"]
pub mod ws_pool_stream;

/// Dashboard v6 — Federation Scaling v5, Async ZKP v10 & Bridge v4 metrics
#[cfg(feature = "v1.5-sprint2")]
#[path = "ui/dashboard_v6.rs"]
pub mod dashboard_v6;

/// WebSocket Federation Stream — Real-time streaming for v1.5-sprint2
#[cfg(feature = "v1.5-sprint2")]
#[path = "web/ws_federation_stream.rs"]
pub mod ws_federation_stream;

// ============================================================================
// Feature Detection Utilities
// ============================================================================

/// Returns a list of all enabled features
pub fn enabled_features() -> Vec<&'static str> {
    let mut features = Vec::new();

    // Core features (always enabled)
    features.push("core");

    // Stable features
    #[cfg(feature = "stable")]
    {
        features.push("stable");
        features.push("p2p");
        features.push("sae");
        features.push("consensus");
        features.push("alignment");
        features.push("federation");
        features.push("marketplace");
        features.push("ui");
        features.push("slo");
        features.push("governance");
    }

    // Hardware acceleration
    #[cfg(feature = "cuda")]
    features.push("cuda");

    #[cfg(feature = "metal")]
    features.push("metal");

    // Dev features
    #[cfg(feature = "debug")]
    features.push("debug");

    #[cfg(feature = "test-mocks")]
    features.push("test-mocks");

    features
}

/// Returns the current version string
pub fn version() -> &'static str {
    VERSION
}

/// Returns the sprint identifier
pub fn sprint_identifier() -> &'static str {
    SPRINT_IDENTIFIER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(version(), "1.3.0");
    }

    #[test]
    fn test_sprint_identifier() {
        assert_eq!(sprint_identifier(), "v1.6.0-stable");
    }

    #[test]
    fn test_enabled_features() {
        let features = enabled_features();
        assert!(features.contains(&"core"));
        #[cfg(feature = "stable")]
        assert!(features.contains(&"stable"));
    }
}
