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
pub const SPRINT_IDENTIFIER: &str = "v9.21.0-sprint85";

// ============================================================================
// Fase 1: Core Modules (P2P, SAE, Bridge)
// ============================================================================

/// P2P networking layer (libp2p-based)
pub mod p2p {
    #[cfg(feature = "v1.8-sprint2")]
    pub mod geographic_routing;
    pub mod protocol;
    pub mod swarm;
}

/// WASM mobile bridge (feature-gated)
#[cfg(feature = "v1.8-sprint2")]
#[path = "wasm/mobile_bridge.rs"]
pub mod mobile_bridge;

/// Browser Node — WASM P2P node for browser environments (feature-gated)
#[cfg(feature = "v2.1-wasm-browser")]
pub mod browser_node;

/// WASM Browser Node — Compiled browser node with wasm-bindgen exports (Sprint24)
#[cfg(feature = "v2.1-wasm-browser")]
#[path = "wasm/browser_node.rs"]
pub mod wasm_browser_node;

/// Public Dataset Loader — Streaming .jsonl/.parquet with SHA256 validation (Sprint24)
#[cfg(feature = "v2.1-real-dataset-loader")]
pub mod dataset;

/// MVP Core Loop — Isolated basic cycle (feature-gated)
#[cfg(feature = "v2.1-mvp-core")]
pub mod mvp_core;

/// Relay Server — WebRTC/Circuit Relay v2 signaling (feature-gated)
#[cfg(feature = "v2.1-relay-server")]
pub mod relay_server;

/// Orchestrator Node — Native orchestrator with task distribution (feature-gated)
#[cfg(feature = "v2.1-orchestrator")]
pub mod orchestrator;

/// Atlas Semántico Global — Piedra Rosetta (feature-gated)
#[cfg(any(
    feature = "v2.1-semantic-graph",
    feature = "v2.1-rosetta-api",
    feature = "v2.1-atlas-ui"
))]
pub mod atlas;

/// Sparse Autoencoder (SAE) loading and routing
pub mod sae {
    #[cfg(feature = "v1.3-sprint3")]
    pub mod adaptive_checkpoint;
    #[cfg(feature = "v1.4-sprint3")]
    pub mod adaptive_checkpoint_v2;
    #[cfg(feature = "v1.5-sprint1")]
    pub mod adaptive_checkpoint_v3;
    #[cfg(feature = "v1.5-sprint3")]
    pub mod adaptive_checkpoint_v4;
    #[cfg(feature = "v1.6-sprint3")]
    pub mod adaptive_checkpoint_v5;
    #[cfg(feature = "v1.2-sprint2")]
    pub mod checkpoint_manager;
    #[cfg(feature = "v1.3-sprint3")]
    pub mod cross_model_aligner;
    #[cfg(feature = "v1.4-sprint3")]
    pub mod cross_model_aligner_v2;
    #[cfg(feature = "v1.5-sprint1")]
    pub mod cross_model_aligner_v3;
    #[cfg(feature = "v1.5-sprint3")]
    pub mod cross_model_aligner_v4;
    #[cfg(feature = "v1.6-sprint3")]
    pub mod cross_model_aligner_v5;
    #[cfg(feature = "v1.2-sprint2")]
    pub mod distributed_finetune;
    #[cfg(feature = "v1.2-sprint2")]
    pub mod fault_tolerance;
    #[cfg(feature = "v1.2-sprint1")]
    pub mod fine_tuning_engine;
    #[cfg(feature = "v1.3-sprint3")]
    pub mod fine_tuning_v3;
    #[cfg(feature = "v1.4-sprint3")]
    pub mod fine_tuning_v4;
    #[cfg(feature = "v1.5-sprint1")]
    pub mod fine_tuning_v5;
    #[cfg(feature = "v1.5-sprint3")]
    pub mod fine_tuning_v6;
    #[cfg(feature = "v1.6-sprint3")]
    pub mod fine_tuning_v7;
    #[cfg(feature = "v2.1-hf-bridge")]
    pub mod hf_bridge;
    pub mod loader;
    #[cfg(feature = "v2.1-qwen-scope-loader")]
    pub mod qwen_scope_loader;
    #[cfg(feature = "v2.1-qwen-scope-sae")]
    pub mod qwen_scope_sae;
    pub mod router;
    #[cfg(feature = "v2.1-sae-training")]
    pub mod training_pipeline;
    #[cfg(feature = "v2.1-wasm-micro-sharding")]
    pub mod wasm_sharding;
}

// ============================================================================
// Sprint14 — Aprendizaje Federado & Alineación Continua
// ============================================================================

/// Federated Learning — Secure gradient aggregation with differential privacy
#[cfg(any(
    feature = "v2.1-federated-agg",
    feature = "v2.1-agg-committees",
    feature = "v2.1-staleness-aware",
    feature = "v2.1-bft-aggregation",
    feature = "v2.1-network-apoptosis"
))]
pub mod federated;

/// Tensor flow and consciousness bridge
pub mod bridge {
    #[cfg(feature = "v1.6-sprint1")]
    #[path = "../bridge/bridge_validator.rs"]
    pub mod bridge_validator;
    pub mod consciousness;
    #[cfg(feature = "v1.6-sprint1")]
    #[path = "../bridge/cross_chain_bridge_v3.rs"]
    pub mod cross_chain_bridge_v3;
    #[cfg(feature = "v1.5-sprint2")]
    #[path = "../bridge/federation_zkp_bridge_v4.rs"]
    pub mod federation_zkp_bridge_v4;
    #[cfg(feature = "v1.6-sprint2")]
    #[path = "../bridge/federation_zkp_bridge_v6.rs"]
    pub mod federation_zkp_bridge_v6;
    #[cfg(feature = "v1.6-sprint3")]
    #[path = "../bridge/federation_zkp_bridge_v7.rs"]
    pub mod federation_zkp_bridge_v7;
    #[cfg(feature = "v1.7-sprint1")]
    #[path = "../bridge/quantization.rs"]
    pub mod quantization;
    #[cfg(feature = "v1.6-sprint1")]
    #[path = "../bridge/relay_manager.rs"]
    pub mod relay_manager;
    pub mod tensor_flow;

    // ─── Sprint71: IoT Microkernel (watchdog, last-GEI cache, async→sync bridge) ───
    #[cfg(feature = "v9.7-bootstrap-resilience")]
    #[path = "../bridge/iot_microkernel.rs"]
    pub mod iot_microkernel;
}

// ============================================================================
// Protocol Layer — Async Steering & Latency Mitigation (RFC-001)
// ============================================================================

#[cfg(feature = "v1.7-sprint1")]
pub mod protocol {
    #[path = "../protocol/async_steering.rs"]
    pub mod async_steering;
    #[cfg(feature = "v2.1-audit-payloads")]
    #[path = "../protocol/audit_payloads.rs"]
    pub mod audit_payloads;

    // ─── Sprint30: Async Quantum Feedback Queue ───
    #[cfg(feature = "v2.1-quantum-feedback")]
    #[path = "../protocol/quantum_feedback.rs"]
    pub mod quantum_feedback;
}

// ─── Sprint30: Async Quantum Feedback Queue (standalone) ───
#[cfg(feature = "v2.1-quantum-feedback")]
#[path = "protocol/quantum_feedback.rs"]
pub mod quantum_feedback;

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
    pub mod merkle;
    pub mod validator;

    // ─── Sprint71: Bootstrap Consensus (Micro-PoW + Web of Trust + Morphic Decoder) ───
    #[cfg(feature = "v9.7-bootstrap-resilience")]
    #[path = "../consensus/bootstrap_consensus.rs"]
    pub mod bootstrap_consensus;

    // ─── Sprint72: Sybil Resistance (PoUW + CE decay + diversity, BFT ε-tolerant) ───
    #[cfg(feature = "v9.8-asymptotic-hardening")]
    #[path = "../consensus/sybil_resistance.rs"]
    pub mod sybil_resistance;

    // ─── Sprint75: Thermodynamic CE (Micro-PoW + ZKP anchored CE credits) ───
    #[cfg(feature = "v9.11-performance-pivot")]
    #[path = "../consensus/thermodynamic_ce.rs"]
    pub mod thermodynamic_ce;

    // ─── Sprint78: Relativistic Entropy (λ freezes during partitions, cryosleep mode) ───
    #[cfg(feature = "v9.14-invariant-architecture")]
    #[path = "../consensus/relativistic_entropy.rs"]
    pub mod relativistic_entropy;

    // ─── Sprint80: Proof of Novelty (topological novelty proof - anti semantic DDoS) ───
    #[cfg(feature = "v9.16-godelian-synthesis")]
    #[path = "../consensus/proof_of_novelty.rs"]
    pub mod proof_of_novelty;
}

// ============================================================================
// Sprint 78: Invariant Architecture & Planetary-Scale Resilience
// ============================================================================

/// Crypto — Sprint 78: Recursive SNARKs + Sprint 79: Post-Quantum STARKs + Sprint 80: Blind Threshold
#[cfg(any(
    feature = "v9.14-invariant-architecture",
    feature = "v9.15-quantum-physical-bridge",
    feature = "v9.16-godelian-synthesis"
))]
pub mod crypto {
    #[cfg(feature = "v9.14-invariant-architecture")]
    #[path = "../crypto/recursive_snark.rs"]
    pub mod recursive_snark;

    // ─── Sprint79: Post-Quantum STARKs (zk-STARK hash-based, no trusted setup) ───
    #[cfg(feature = "v9.15-quantum-physical-bridge")]
    #[path = "../crypto/post_quantum_starks.rs"]
    pub mod post_quantum_starks;

    // ─── Sprint80: Blind Threshold Computation (Garbled Circuits + TSS) ───
    #[cfg(feature = "v9.16-godelian-synthesis")]
    #[path = "../crypto/blind_threshold_computation.rs"]
    pub mod blind_threshold_computation;
}

/// Privacy — Sprint 78: Differential Holographic Noise + Sprint 79: FHE-Ready WASM
#[cfg(any(
    feature = "v9.14-invariant-architecture",
    feature = "v9.15-quantum-physical-bridge"
))]
pub mod privacy {
    #[cfg(feature = "v9.14-invariant-architecture")]
    #[path = "../privacy/differential_holographic_noise.rs"]
    pub mod differential_holographic_noise;

    // ─── Sprint79: FHE-Ready WASM (side-channel mitigation) ───
    #[cfg(feature = "v9.15-quantum-physical-bridge")]
    #[path = "../privacy/fhe_ready_wasm.rs"]
    pub mod fhe_ready_wasm;
}

/// Physical TEE Bridge — Sprint 79: TEE oracles + Sprint 80: Heterogeneous MPC
#[cfg(any(
    feature = "v9.15-quantum-physical-bridge",
    feature = "v9.16-godelian-synthesis"
))]
pub mod oracle {
    #[cfg(feature = "v9.15-quantum-physical-bridge")]
    #[path = "../oracle/physical_tee_bridge.rs"]
    pub mod physical_tee_bridge;

    // ─── Sprint80: Heterogeneous MPC (multi-ISA consensus - x86/ARM/RISC-V) ───
    #[cfg(feature = "v9.16-godelian-synthesis")]
    #[path = "../oracle/heterogeneous_mpc.rs"]
    pub mod heterogeneous_mpc;
}

// ============================================================================
// Fase 3: Security, ZKP, Human-in-the-Loop
// ============================================================================

/// Security modules (WASM sandbox, memory guard)
pub mod security {
    pub mod memory_guard;
    #[cfg(feature = "v1.1-sprint1")]
    pub mod wasm_profiler;
    pub mod wasm_sandbox;
    #[cfg(feature = "v1.1-sprint1")]
    pub mod wasm_sandbox_v2;
}

/// Zero-Knowledge Proof circuits and verification
pub mod zkp {
    pub mod async_prover;
    #[cfg(feature = "v1.5-sprint2")]
    #[path = "../zkp/async_zkp_v10.rs"]
    pub mod async_zkp_v10;
    #[cfg(feature = "v1.5-sprint3")]
    #[path = "../zkp/async_zkp_v11.rs"]
    pub mod async_zkp_v11;
    #[cfg(feature = "v1.6-sprint2")]
    #[path = "../zkp/async_zkp_v13.rs"]
    pub mod async_zkp_v13;
    #[cfg(feature = "v1.6-sprint3")]
    #[path = "../zkp/async_zkp_v14.rs"]
    pub mod async_zkp_v14;
    #[cfg(feature = "v1.3-sprint2")]
    #[path = "../zkp/async_zkp_v4.rs"]
    pub mod async_zkp_v4;
    #[cfg(feature = "v1.3-sprint3")]
    #[path = "../zkp/async_zkp_v5.rs"]
    pub mod async_zkp_v5;
    #[cfg(feature = "v1.4-sprint3")]
    #[path = "../zkp/async_zkp_v8.rs"]
    pub mod async_zkp_v8;
    pub mod batch_accumulator;
    pub mod circuit;
    #[cfg(feature = "v1.4-sprint1")]
    #[path = "../zkp/circuit_optimizer.rs"]
    pub mod circuit_optimizer;
    #[cfg(feature = "v1.4-sprint3")]
    #[path = "../zkp/cross_federation_verification.rs"]
    pub mod cross_federation_verification;
    #[cfg(feature = "v1.5-sprint3")]
    #[path = "../zkp/cross_federation_verifier_v2.rs"]
    pub mod cross_federation_verifier_v2;
    #[cfg(feature = "v1.4-sprint1")]
    #[path = "../zkp/halo2_engine.rs"]
    pub mod halo2_engine;
    #[cfg(feature = "v1.4-sprint1")]
    #[path = "../zkp/proof_aggregator.rs"]
    pub mod proof_aggregator;
    pub mod verifier;
    pub mod verifier_pool;

    // ─── Sprint49: GEI ZKP Certification ───
    #[cfg(feature = "v3.1-gei-topology")]
    pub mod gei_zkp;
}

/// Marketplace v2 — Resource matching, escrow ledger, adaptive pricing
#[cfg(feature = "stable")]
pub mod marketplace_v2 {
    #[path = "../marketplace/escrow_ledger.rs"]
    pub mod escrow_ledger;
    #[path = "../marketplace/matchmaker.rs"]
    pub mod matchmaker;
    #[path = "../marketplace/pricing_engine.rs"]
    pub mod pricing_engine;
}

/// Bridge — ZKP ↔ Marketplace integration and proof submission
#[cfg(feature = "stable")]
pub mod bridge_v2 {
    #[path = "../bridge/proof_submission.rs"]
    pub mod proof_submission;
    #[path = "../bridge/zkp_marketplace_bridge.rs"]
    pub mod zkp_marketplace_bridge;
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
    pub mod concept_updater;
    pub mod feedback_cli;
}

// ============================================================================
// Fase 4: Scaling, RLHF, Web UI, Monitoring
// ============================================================================

/// Peer scaling and bootstrap management
#[cfg(feature = "stable")]
pub mod scaling {
    pub mod bootstrap;
    #[cfg(feature = "phase8-sprint2")]
    pub mod cross_model;
    pub mod peer_manager;
    pub mod predictive_balancer;
}

/// Progressive Weight Streaming — Sprint 78: <500ms cold start
#[cfg(feature = "v9.14-invariant-architecture")]
pub mod streaming {
    #[path = "../network/progressive_weight_streaming.rs"]
    pub mod progressive_weight_streaming;
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
    pub mod realtime;
    pub mod routes;
    pub mod server;
    pub mod sse_metrics;
    pub mod ws_alignment_stream;
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
    pub mod health;
    pub mod metrics;
    pub mod streaming_metrics;
}

/// Runtime optimization — Tokio tuning, task scheduling, worker pools
/// Runtime v3.0 — WASM sandbox, secure messaging, privacy enforcement
#[cfg(any(
    feature = "v1.4-sprint1",
    feature = "v3.0-wasm-runtime",
    feature = "v3.0-pillar-messaging",
    feature = "v3.0-privacy-guard"
))]
pub mod runtime {
    #[cfg(feature = "v3.0-pillar-messaging")]
    #[path = "../runtime/pillar_messaging.rs"]
    pub mod pillar_messaging;
    #[cfg(feature = "v3.0-privacy-guard")]
    #[path = "../runtime/privacy_enforcer.rs"]
    pub mod privacy_enforcer;
    #[cfg(feature = "v1.4-sprint1")]
    #[path = "../runtime/task_scheduler.rs"]
    pub mod task_scheduler;
    #[cfg(feature = "v1.4-sprint1")]
    #[path = "../runtime/tokio_optimizer.rs"]
    pub mod tokio_optimizer;
    #[cfg(feature = "v3.0-wasm-runtime")]
    #[path = "../runtime/wasm_sandbox.rs"]
    pub mod wasm_sandbox;
    #[cfg(feature = "v1.4-sprint1")]
    #[path = "../runtime/worker_pool.rs"]
    pub mod worker_pool;
}

/// Storage — LZ4 compression, checkpoint cache, gradient archive
#[cfg(feature = "v1.4-sprint1")]
pub mod storage {
    #[path = "../storage/checkpoint_cache.rs"]
    pub mod checkpoint_cache;
    #[path = "../storage/gradient_archive.rs"]
    pub mod gradient_archive;
    #[path = "../storage/lz4_compressor.rs"]
    pub mod lz4_compressor;
}

/// Monitoring v2 — Advanced metrics, health checking, alert engine
#[cfg(feature = "v1.4-sprint1")]
pub mod monitoring_v2 {
    #[path = "../monitoring/advanced_metrics.rs"]
    pub mod advanced_metrics;
    #[path = "../monitoring/alert_engine.rs"]
    pub mod alert_engine;
    #[path = "../monitoring/health_checker.rs"]
    pub mod health_checker;
}

// ============================================================================
// Fase 5: Governance, Reputation, Ecosystem, Bootstrap
// ============================================================================

/// Proposal and voting governance
#[cfg(feature = "stable")]
pub mod governance {
    #[cfg(feature = "v1.3-sprint2")]
    pub mod dao_ledger_v2;
    #[cfg(feature = "v1.2-sprint2")]
    pub mod dao_v3;
    #[cfg(feature = "v1.2-sprint2")]
    pub mod hybrid_voting;
    pub mod liquid_v2;
    pub mod proposal;
    #[cfg(feature = "v1.2-sprint2")]
    pub mod proposal_executor;
    #[cfg(feature = "v1.3-sprint2")]
    pub mod proposal_tracker;
    #[cfg(feature = "v1.3-sprint2")]
    pub mod technical_staking;
    pub mod voting;
    pub mod voting_mechanism;
}

/// Reputation ledger and scoring
#[cfg(feature = "stable")]
pub mod reputation {
    pub mod ledger;
    #[cfg(feature = "v1.8-sprint1")]
    #[path = "../reputation/proof_schema.rs"]
    pub mod proof_schema;
    pub mod scoring;
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
    pub mod network_init;
    pub mod seed_registry;
}

// ============================================================================
// Fase 6: Interoperability, Federation, Staking, API
// ============================================================================

/// Cross-model interoperability adapters
#[cfg(feature = "stable")]
pub mod interoperability {
    pub mod adapter;
    pub mod adaptive_router_v2;
    pub mod capability_registry;
    pub mod cross_model_router;
    pub mod onnx_adapter;
    pub mod schema;
    pub mod schema_registry;
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
    #[path = "../state/merkle_aggregator.rs"]
    pub mod merkle_aggregator;
    #[path = "../state/snapshot_manager.rs"]
    pub mod snapshot_manager;
    #[path = "../state/state_sync_v2.rs"]
    pub mod state_sync_v2;
}

/// Federation protocols (FedAvg, sync)
#[cfg(feature = "stable")]
pub mod federation {
    #[cfg(feature = "v1.6-sprint2")]
    pub mod adaptive_router_v2;
    pub mod avg_aggregator;
    #[cfg(feature = "v1.2-sprint2")]
    pub mod bridge_validator;
    #[cfg(feature = "v1.2-sprint2")]
    pub mod cross_chain_consensus;
    #[cfg(feature = "v1.2-sprint1")]
    pub mod cross_chain_identity;
    pub mod cross_model_scaler;
    #[cfg(feature = "v1.6-sprint3")]
    pub mod cross_model_scaling_v7;
    #[cfg(feature = "v1.3-sprint3")]
    pub mod dynamic_sharder;
    #[cfg(feature = "v1.5-sprint3")]
    pub mod dynamic_sharder_v2;
    #[cfg(feature = "v1.2-sprint1")]
    pub mod gradient_aggregator_v3;
    pub mod gradient_normalizer;
    #[cfg(feature = "v1.5-sprint2")]
    pub mod gradient_sync_v5;
    #[cfg(feature = "v1.5-sprint3")]
    pub mod gradient_sync_v6;
    #[cfg(feature = "v1.6-sprint2")]
    pub mod gradient_sync_v7;
    #[cfg(feature = "v1.2-sprint1")]
    pub mod multi_chain_registry;
    #[cfg(feature = "v1.6-sprint3")]
    pub mod predictive_sharder_v3;
    #[cfg(feature = "v1.4-sprint3")]
    pub mod predictive_sharder_v4;
    #[cfg(feature = "v1.5-sprint2")]
    pub mod predictive_sharder_v5;
    #[cfg(feature = "v1.4-sprint3")]
    pub mod scaling_v4;
    #[cfg(feature = "v1.5-sprint2")]
    pub mod scaling_v5;
    #[cfg(feature = "v1.5-sprint3")]
    pub mod scaling_v6;
    #[cfg(feature = "v1.6-sprint2")]
    pub mod scaling_v7;
    pub mod sync_protocol;
    pub mod trust_sync;

    // ─── Sprint30: Neuroplasticidad Federada ───
    #[cfg(feature = "v2.1-neuroplasticity")]
    #[path = "../federated/neuroplastic_engine.rs"]
    pub mod neuroplastic_engine;
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
    pub mod auth;
    pub mod openapi;
    pub mod routes;
}

// ============================================================================
// Fase 7: Continuous Alignment, Cross-Net Federation, Dynamic Trust
// ============================================================================

/// Continuous alignment engine + Sprint79: Shadow Persona Sandbox
#[cfg(any(feature = "stable", feature = "v9.15-quantum-physical-bridge"))]
pub mod alignment {
    pub mod confidence_calculator;
    #[cfg(feature = "phase8-sprint2")]
    pub mod continuous;
    pub mod engine;
    #[cfg(feature = "v1.2-sprint1")]
    pub mod ethical_constraint_engine;
    pub mod loop_v2;
    pub mod steering_engine;

    // ─── Sprint16.3: Stuartian Context Tensor (SCT) ───
    #[cfg(feature = "v2.1-sct-core")]
    pub mod sct_core;
    #[cfg(feature = "v2.1-sct-guard")]
    pub mod sct_guard;
    #[cfg(feature = "v2.1-sct-reward")]
    pub mod sct_reward;

    // ─── Sprint20: Geometría Estuardiana 3D ───
    #[cfg(feature = "v2.1-stuartian-geometry")]
    #[path = "../alignment/stuartian_geometry.rs"]
    pub mod stuartian_geometry;

    // ─── Sprint28: Motor de Significado Simbólico ───
    #[cfg(feature = "v2.1-ethical-attention")]
    pub mod ethical_attention;
    #[cfg(feature = "v2.1-symbolic-engine")]
    pub mod symbolic_engine;

    // ─── Sprint30: Retroalimentación Estuardiana (Human-in-the-Loop) ───
    #[cfg(feature = "v2.1-steering-bridge")]
    #[path = "../alignment/steering_bridge.rs"]
    pub mod steering_bridge;

    // ─── Sprint49: Geometric Ethical Invariants (GEI) ───
    #[cfg(feature = "v3.1-gei-topology")]
    pub mod gei_fingerprint;

    // ─── Sprint51: Recursive Stuartian Self-Improvement (RSSI) ───
    #[cfg(feature = "v3.3-rssi-evolution")]
    pub mod attractor_basin;
    #[cfg(feature = "v3.3-rssi-evolution")]
    pub mod rssi_engine;

    // ─── Sprint72: Topology-Ethics Mapping (GEI as structural stability proxy) ───
    #[cfg(feature = "v9.8-asymptotic-hardening")]
    pub mod topology_ethics_mapping;

    // ─── Sprint79: Shadow Persona Sandbox (adversarial isolation + cryptographic muzzle) ───
    #[cfg(feature = "v9.15-quantum-physical-bridge")]
    #[path = "../alignment/shadow_persona_sandbox.rs"]
    pub mod shadow_persona_sandbox;

    // ─── Sprint80: Epistemic Wiping (ontological air-gap + cryptographic wiping) ───
    #[cfg(feature = "v9.16-godelian-synthesis")]
    #[path = "../alignment/epistemic_wiping.rs"]
    pub mod epistemic_wiping;
}

/// Topological Fingerprinting — Persistent Homology for GEI (Sprint49) + Deception Detection (Sprint51)
#[cfg(any(
    feature = "v3.1-gei-topology",
    feature = "v3.3-rssi-evolution",
    feature = "v9.7-bootstrap-resilience",
    feature = "v9.8-asymptotic-hardening"
))]
pub mod topology {
    #[cfg(feature = "v3.1-gei-topology")]
    #[path = "../topology/persistent_homology.rs"]
    pub mod persistent_homology;

    // ─── Sprint51: Topological Deception Detection ───
    #[cfg(feature = "v3.3-rssi-evolution")]
    #[path = "../topology/deception_detector.rs"]
    pub mod deception_detector;

    // ─── Sprint57: Higher-Order Persistent Homology (β₂) ───
    #[cfg(feature = "v3.9-noosphere-engine")]
    #[path = "../topology/hoph_engine.rs"]
    pub mod hoph_engine;

    // ─── Sprint71: GEI Approximator (simplicial approximation + ZKP verification) ───
    #[cfg(feature = "v9.7-bootstrap-resilience")]
    #[path = "../topology/gei_approximator.rs"]
    pub mod gei_approximator;

    // ─── Sprint72: Differentiable GEI (soft Betti, surrogate gradients, O(n log n)) ───
    #[cfg(feature = "v9.8-asymptotic-hardening")]
    #[path = "../topology/differentiable_gei.rs"]
    pub mod differentiable_gei;

    // ─── Sprint78: Ethical Anchors (invariant points of infinite mass) ───
    #[cfg(feature = "v9.14-invariant-architecture")]
    #[path = "../topology/ethical_anchors.rs"]
    pub mod ethical_anchors;
}

/// Stuartian Moral Manifold — Trajectory-based Ethical Evaluation (Sprint50)
#[cfg(any(feature = "v3.2-genesis-manifold", feature = "v4.0-snap-activation"))]
pub mod ethics {
    #[cfg(feature = "v3.2-genesis-manifold")]
    #[path = "../ethics/moral_manifold.rs"]
    pub mod moral_manifold;

    /// Global Safeguards — Sprint 58
    #[cfg(feature = "v4.0-snap-activation")]
    #[path = "../ethics/global_safeguards.rs"]
    pub mod global_safeguards;
}

/// Temporal Cohesion — Distributed Time Synchronization (Sprint52) + Sprint79: Useful VDFs
#[cfg(any(
    feature = "v3.4-macro-symbiosis",
    feature = "v9.15-quantum-physical-bridge"
))]
pub mod time {
    #[cfg(feature = "v3.4-macro-symbiosis")]
    #[path = "../time/temporal_cohesion.rs"]
    pub mod temporal_cohesion;

    // ─── Sprint79: Useful VDFs (SAE-entangled, ASIC-resistant) ───
    #[cfg(feature = "v9.15-quantum-physical-bridge")]
    #[path = "../time/useful_vdf.rs"]
    pub mod useful_vdf;
}

/// Global Symbiotic Economy — DAG-based CE Ledger (Sprint52)
#[cfg(feature = "v3.4-macro-symbiosis")]
pub mod economy {
    #[path = "../economy/symbiotic_ledger.rs"]
    pub mod symbiotic_ledger;

    /// Genesis Graph — DAG Root Node with Stuartian Laws hash (Sprint 56)
    #[cfg(feature = "v3.8-morphic-genesis")]
    #[path = "../economy/genesis_graph.rs"]
    pub mod genesis_graph;

    /// Mainnet Genesis Block — Forge Genesis Block con 5 Leyes Estuardianas (Sprint 59)
    #[cfg(feature = "v5.0-mainnet-genesis")]
    #[path = "../economy/mainnet_genesis.rs"]
    pub mod mainnet_genesis;
}

/// Stuartian Noosphere Engine (SNE) — Sprint 57
#[cfg(feature = "v3.9-noosphere-engine")]
pub mod noosphere;

/// Morphic Resonance Decoder — Semantic Manipulation Protection (Sprint 56)
#[cfg(feature = "v3.8-morphic-genesis")]
pub mod semantics {
    #[path = "../semantics/morphic_decoder.rs"]
    pub mod morphic_decoder;

    #[path = "../semantics/semantic_purifier.rs"]
    pub mod semantic_purifier;
}

/// Mainnet Genesis — Deterministic genesis state & steward activation (Sprint22)
#[cfg(any(feature = "v2.1-mainnet-genesis", feature = "v2.1-steward-portal"))]
pub mod mainnet {
    #[cfg(feature = "v2.1-mainnet-genesis")]
    #[path = "../mainnet/genesis.rs"]
    pub mod genesis;
}

/// MVP Local Simulation — End-to-end local testnet (Sprint23)
#[cfg(feature = "v2.1-mvp-simulation")]
pub mod mvp {
    #[path = "../mvp/sae_simulator.rs"]
    pub mod sae_simulator;

    #[path = "../mvp/consensus_runner.rs"]
    pub mod consensus_runner;

    #[path = "../mvp/local_testnet.rs"]
    pub mod local_testnet;
}

/// Network — Cross-mesh routing & multi-region synchronization (Sprint21)
#[cfg(any(
    feature = "v2.1-cross-mesh",
    feature = "v2.1-region-sync",
    feature = "v2.1-federation-bootstrap",
    feature = "v3.5-planetary-emergence",
    feature = "v3.6-aegis-resonance",
    feature = "v3.7-symbiotic-portal"
))]
pub mod network;

/// Intelligence — Autonomous Emergence Engine (Sprint53)
#[cfg(any(
    feature = "v3.5-planetary-emergence",
    feature = "v3.6-aegis-resonance",
    feature = "v3.7-symbiotic-portal"
))]
pub mod intelligence;

/// Symbiotic Portal — Zero-Friction Onboarding via WASM Client (Sprint 55)
/// Full portal (wasm_client, ui_bridge) requires wasm32 target.
/// MorphicBridge (morphic_bridge) is available for native testing via v3.8-morphic-genesis.
#[cfg(any(
    all(feature = "v3.7-symbiotic-portal", target_arch = "wasm32"),
    feature = "v3.8-morphic-genesis"
))]
pub mod portal;

/// Lightweight Crypto Verification — Merkle-DAG + Ed25519 (Sprint72)
#[cfg(feature = "v9.8-asymptotic-hardening")]
#[path = "crypto/lightweight_verification.rs"]
pub mod lightweight_verification;

/// Tiered Hardware Execution — WASM tiering, memory pooling, quantization (Sprint72)
#[cfg(feature = "v9.8-asymptotic-hardening")]
#[path = "hardware/tiered_execution.rs"]
pub mod tiered_execution;

/// Streaming Symbolic Filter — Async rejection sampling, priority queue (Sprint72)
#[cfg(feature = "v9.8-asymptotic-hardening")]
#[path = "inference/streaming_symbolic_filter.rs"]
pub mod streaming_symbolic_filter;

// ============================================================================
// Sprint 73: Pragmatic Pivot & Asymptotic Hardening (v9.9.0)
// ============================================================================

/// Lightweight GEI Proxy — Soft Betti + stratified sampling, O(n log n) (Sprint73)
#[cfg(feature = "v9.9-pragmatic-pivot")]
#[path = "topology/lightweight_gei_proxy.rs"]
pub mod lightweight_gei_proxy;

/// Tiered Verification — Edge (Merkle/Ed25519) vs Core (SNARKs batch) (Sprint73)
#[cfg(feature = "v9.9-pragmatic-pivot")]
#[path = "crypto/tiered_verification.rs"]
pub mod tiered_verification;

/// Speculative Symbolic Filter — Async post-hoc + autoregressive fallback (Sprint73)
#[cfg(feature = "v9.9-pragmatic-pivot")]
#[path = "inference/speculative_symbolic_filter.rs"]
pub mod speculative_symbolic_filter;

/// Sybil-Hardened CE — PoUW + decay + diversity + vouching (Sprint73)
#[cfg(feature = "v9.9-pragmatic-pivot")]
#[path = "consensus/sybil_hardened_ce.rs"]
pub mod sybil_hardened_ce;

/// Topology-Ethics Reframe — GEI as anomaly proxy, ethics via guardrails (Sprint73)
#[cfg(feature = "v9.9-pragmatic-pivot")]
#[path = "alignment/topology_ethics_reframe.rs"]
pub mod topology_ethics_reframe;

/// Graceful Apoptosis — Bounded quarantine, ε-reintegration, cascade prevention (Sprint73)
#[cfg(feature = "v9.9-pragmatic-pivot")]
#[path = "network/graceful_apoptosis.rs"]
pub mod graceful_apoptosis;

/// Data Availability Sampling (DAS) — Probabilistic verification O(log n) (Sprint74)
#[cfg(feature = "v9.10-distributed-hardening")]
#[path = "ledger/das_sampler.rs"]
pub mod das_sampler;

/// KZG State Pruning — Polynomial commitments for cryptographic pruning (Sprint74)
#[cfg(feature = "v9.10-distributed-hardening")]
#[path = "ledger/kzg_state_pruning.rs"]
pub mod kzg_state_pruning;

/// Collaborative SNARK Generation — Circuit partitioning + threshold aggregation (Sprint74)
#[cfg(feature = "v9.10-distributed-hardening")]
#[path = "crypto/collaborative_snark.rs"]
pub mod collaborative_snark;

/// Speculative Decoding — Parallel topological validation for competitive TTFT (Sprint74)
#[cfg(feature = "v9.10-distributed-hardening")]
#[path = "inference/speculative_decoder.rs"]
pub mod speculative_decoder;

/// Topological Reconciliation — CRDT-based post-partition healing (Sprint74)
#[cfg(feature = "v9.10-distributed-hardening")]
#[path = "network/topological_reconciliation.rs"]
pub mod topological_reconciliation;

// ============================================================================
// Sprint 75: Thermodynamic Hardening & Asynchronous Neuro-Symbolic Pivot (v9.11.0)
// ============================================================================

/// Async Symbolic Sidecar — Post-hoc SAE validation in parallel thread (Sprint75)
#[cfg(feature = "v9.11-performance-pivot")]
#[path = "inference/async_symbolic_sidecar.rs"]
pub mod async_symbolic_sidecar;

/// GEI Proxy Distillation — PCA-based β₁ approximation in <5ms (Sprint75)
#[cfg(feature = "v9.11-performance-pivot")]
#[path = "topology/gei_proxy_distillation.rs"]
pub mod gei_proxy_distillation;

/// Distributed Seed Mesh — Geographic/ISP diverse mesh with key rotation (Sprint75)
#[cfg(feature = "v9.11-performance-pivot")]
#[path = "network/distributed_seed_mesh.rs"]
pub mod distributed_seed_mesh;

/// Async Gossip with CRDTs — Partition-tolerant GossipSub mesh (Sprint16.4)
#[cfg(any(
    feature = "v2.1-async-gossip",
    feature = "v2.1-offline-cache",
    feature = "v2.1-crdt-state"
))]
pub mod async_gossip {
    #[cfg(feature = "v2.1-async-gossip")]
    #[path = "../async_gossip/mesh.rs"]
    pub mod mesh;

    #[cfg(feature = "v2.1-offline-cache")]
    #[path = "../async_gossip/cache.rs"]
    pub mod cache;

    #[cfg(feature = "v2.1-crdt-state")]
    #[path = "../async_gossip/crdt.rs"]
    pub mod crdt;

    #[cfg(feature = "v2.1-crdt-symbols")]
    #[path = "../async_gossip/crdt_symbols.rs"]
    pub mod crdt_symbols;
}

/// Cross-network federation bridge and trust scoring
#[cfg(feature = "stable")]
pub mod federation_v2 {
    #[path = "../federation/bridge.rs"]
    pub mod bridge;
    #[path = "../federation/trust_scoring.rs"]
    pub mod trust_scoring;
}

/// Cross-platform offline-first sync engine (Sprint26)
#[cfg(feature = "v2.1-cross-platform-sync")]
#[path = "platform/cross_sync.rs"]
pub mod cross_sync;

// ============================================================================
// Sprint29: Proof of Symbiosis, Crédito de Existencia & Apoptosis de Red
// ============================================================================

/// Economics — Existential Credit & Proof of Symbiosis (Sprint29)
#[cfg(any(
    feature = "v2.1-proof-of-symbiosis",
    feature = "v2.1-network-apoptosis"
))]
pub mod economics;

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
    pub mod dashboard_v2;
    pub mod dashboard_v6;
    #[cfg(feature = "v1.6-sprint2")]
    #[path = "../ui/dashboard_v7.rs"]
    pub mod dashboard_v7;
    pub mod realtime_backend;
}

/// SLO tracking and enforcement
#[cfg(feature = "stable")]
pub mod slo {
    pub mod contract_manager;
    pub mod dynamic_engine;
    #[cfg(feature = "phase8-sprint2")]
    pub mod enforcer;
    pub mod engine;
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
    #[path = "../slo/predictive_contracts.rs"]
    pub mod predictive_contracts;
    #[path = "../slo/slo_v3.rs"]
    pub mod slo_v3_engine;
}

/// Cross-Model Scaling v2 with capability negotiation
#[cfg(feature = "v1.2-sprint3")]
pub mod scaling_v3 {
    #[path = "../scaling/capability_negotiator.rs"]
    pub mod capability_negotiator;
    #[path = "../scaling/cross_model_v2.rs"]
    pub mod cross_model_v2;
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
    #[path = "../marketplace/cross_chain_settlement.rs"]
    pub mod cross_chain_settlement;
    #[path = "../marketplace/marketplace_v3.rs"]
    #[allow(clippy::module_inception)]
    pub mod marketplace_v3;
    #[path = "../marketplace/reputation_matcher.rs"]
    pub mod reputation_matcher;
}

/// Alignment Loop v3 with ZKP steering verification and bias mitigation
#[cfg(feature = "v1.2-sprint4")]
pub mod alignment_v3 {
    #[path = "../alignment/bias_mitigator.rs"]
    pub mod bias_mitigator;
    #[path = "../alignment/loop_v3.rs"]
    pub mod loop_v3;
    #[path = "../alignment/steering_verifier.rs"]
    pub mod steering_verifier;
}

/// Federation Scaling v3 with adaptive sharding and gradient sync
#[cfg(feature = "v1.2-sprint4")]
pub mod federation_scaling_v3 {
    #[path = "../federation/adaptive_sharder.rs"]
    pub mod adaptive_sharder;
    #[path = "../federation/gradient_sync_v3.rs"]
    pub mod gradient_sync_v3;
    #[path = "../federation/scaling_v3.rs"]
    pub mod scaling_v3;
}

// ============================================================================
// v1.3.0 Sprint 1 Modules
// ============================================================================

/// SAE Fine-Tuning v2 with checkpoint optimization and gradient sync
#[cfg(feature = "v1.3-sprint1")]
pub mod sae_v2 {
    #[path = "../sae/checkpoint_optimizer.rs"]
    pub mod checkpoint_optimizer;
    #[path = "../sae/fine_tuning_v2.rs"]
    pub mod fine_tuning_v2;
    #[path = "../sae/gradient_sync_v2.rs"]
    pub mod gradient_sync_v2;
}

/// Cross-Node Compute Routing with predictive scheduling and load balancing
#[cfg(feature = "v1.3-sprint1")]
pub mod routing_v2 {
    #[path = "../routing/cross_node_router.rs"]
    pub mod cross_node_router;
    #[path = "../routing/load_balancer.rs"]
    pub mod load_balancer;
    #[path = "../routing/predictive_scheduler.rs"]
    pub mod predictive_scheduler;
}

/// Community Reputation Ledger v2 with anti-Sybil and merit scoring
#[cfg(feature = "v1.3-sprint1")]
pub mod reputation_v2 {
    #[path = "../reputation/anti_sybil.rs"]
    pub mod anti_sybil;
    #[path = "../reputation/ledger_v2.rs"]
    pub mod ledger_v2;
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
    #[path = "../pools/pool_matcher.rs"]
    pub mod pool_matcher;
    #[path = "../pools/shard_aggregator.rs"]
    pub mod shard_aggregator;
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
    #[path = "../pools/capacity_negotiator.rs"]
    pub mod capacity_negotiator;
    #[path = "../pools/cross_chain_pools_v3.rs"]
    pub mod cross_chain_pools_v3;
    #[path = "../pools/dynamic_aggregator.rs"]
    pub mod dynamic_aggregator;
}

/// Cross-Chain Pools v4 & Dynamic Routing (v1.5-sprint1)
#[cfg(feature = "v1.5-sprint1")]
pub mod pools_v4 {
    #[path = "../pools/capacity_orchestrator.rs"]
    pub mod capacity_orchestrator;
    #[path = "../pools/cross_chain_pools_v4.rs"]
    pub mod cross_chain_pools_v4;
    #[path = "../pools/dynamic_router.rs"]
    pub mod dynamic_router;
}

/// DAO Ledger v4 & Hybrid Execution (v1.4-sprint2)
#[cfg(feature = "v1.4-sprint2")]
pub mod governance_v4 {
    #[path = "../governance/audit_trail.rs"]
    pub mod audit_trail;
    #[path = "../governance/dao_ledger_v4.rs"]
    pub mod dao_ledger_v4;
    #[path = "../governance/hybrid_executor.rs"]
    pub mod hybrid_executor;
}

/// DAO Ledger v5 & Hybrid Governance (v1.5-sprint1)
#[cfg(feature = "v1.5-sprint1")]
pub mod governance_v5 {
    #[path = "../governance/audit_trail_v2.rs"]
    pub mod audit_trail_v2;
    #[path = "../governance/dao_ledger_v5.rs"]
    pub mod dao_ledger_v5;
    #[path = "../governance/hybrid_governance.rs"]
    pub mod hybrid_governance;
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
// v2.0.0 Sprint 1 — GUI Tauri, ZKP v2 Multi-Curve & K8s Operator Base (FASE 83)
// ============================================================================

/// Tauri GUI scaffold — Desktop bridge with state management and commands
#[cfg(feature = "v2.0-sprint1")]
#[path = "gui/tauri_scaffold.rs"]
pub mod tauri_scaffold;

/// Multi-curve ZKP setup — BN254, BLS12-381, BLS12-377, Pasta with aggregation v2
#[cfg(feature = "v2.0-sprint1")]
#[path = "zkp/multi_curve_setup.rs"]
pub mod multi_curve_setup;

/// K8s operator base — CRDs (Node/Lease/SteeringConfig) with reconciliation loop
#[cfg(feature = "v2.0-sprint1")]
#[path = "infra/k8s_operator_base.rs"]
pub mod k8s_operator_base;

// ============================================================================
// v2.0.0 Sprint 2 — Neural Steer Integration, ZKP Optimization & WASM Hardening (FASE 85)
// ============================================================================

/// Neural Tauri Bridge — Integration between Neural Steer UI and Tauri GUI scaffold
#[cfg(feature = "v2.0-sprint2")]
#[path = "gui/neural_tauri_bridge.rs"]
pub mod neural_tauri_bridge;

/// Commitment Pool — Optimized commitment pooling for batch ZKP verification
#[cfg(feature = "v2.0-sprint2")]
#[path = "zkp/commitment_pool.rs"]
pub mod commitment_pool;

/// WASM Mobile Hardening — Memory limits, thermal fallback & adaptive scheduler
#[cfg(feature = "v2.0-sprint2")]
#[path = "wasm/mobile_hardening.rs"]
pub mod mobile_hardening;

// ============================================================================
// v3.0.0 Sprint 41 — Cross-Pillar Orchestration & WASM/Edge Integration
// ============================================================================

/// Cross-Pillar Orchestration Layer — Routes requests across 4 Evolutionary Pillars.
/// Validates Ed25519 signatures, CE > 0, SCT Z > 0 before dispatch.
#[cfg(feature = "v3.0-orchestration")]
pub mod orchestration;

/// Evolutionary Pillars — Module declarations & integration contracts.
/// Unifies Corpuscular, Maieutic, Steganographic & Resonance pillars.
#[cfg(any(
    feature = "v3.0-orchestration",
    feature = "v3.0-corpuscular-bridge",
    feature = "v3.0-maieutic-synthesizer",
    feature = "v3.0-steganographic-survival",
    feature = "v3.0-resonance-interface"
))]
pub mod pillars;

// ============================================================================
// Sprint 68: Academic Formalization & Validation Layer (v9.4.0)
// ============================================================================

/// Metrics — Cooperative objective and ethical loss functions.
#[cfg(feature = "v9.4-validation-layer")]
pub mod metrics;

/// SCT — Stuartian Coherence Tensor Z-axis calibration.
#[cfg(feature = "v9.4-validation-layer")]
pub mod sct;

// ============================================================================
// Sprint 70: Civilization-Scale Architecture & Verification Pipeline (v9.6.0)
// ============================================================================

/// Universal Feature Dictionary — FedAvg merge, Lyapunov stability, contrastive disentanglement.
#[cfg(feature = "v9.6-civilization-scale")]
pub mod dictionary;

/// Activation hooking and ZKP verification for frontier models.
#[cfg(feature = "v9.6-civilization-scale")]
pub mod auditing;

/// Symbolic+Geometric Alignment — proof generation and moral attractor.
#[cfg(feature = "v9.6-civilization-scale")]
#[path = "alignment/mod.rs"]
pub mod civilization_alignment;

/// Anti-capture mechanisms — geo-diversity, anti-Sybil, chaos engineering.
#[cfg(feature = "v9.6-civilization-scale")]
#[path = "security/mod.rs"]
pub mod civilization_security;

// ============================================================================
// Sprint 76: Ontological Debugging & Thermodynamic Pivots (v9.12.0)
// ============================================================================

/// Symbiotic Diversity Loss — Pareto optimization: L = max(Diversidad) - λ·Conflicto_Destructivo (Sprint76)
#[cfg(feature = "v9.12-ontological-debugging")]
#[path = "metrics/symbiotic_diversity_loss.rs"]
pub mod symbiotic_diversity_loss;

/// Evolutionary Quarantine — Dynamic attractor ethics, sandboxing for Z<0 nodes (Sprint76)
#[cfg(feature = "v9.12-ontological-debugging")]
#[path = "network/evolutionary_quarantine.rs"]
pub mod evolutionary_quarantine;

/// Optimistic Edge + Fraud Proofs — Ed25519 at edge, heavy ZKP only if challenged (Sprint76)
#[cfg(feature = "v9.12-ontological-debugging")]
#[path = "crypto/optimistic_edge.rs"]
pub mod optimistic_edge;

/// Fractal Pruning (Stuartian Forgetting) — 72h GC, Merkle accumulation, macro-wisdom retention (Sprint76)
#[cfg(feature = "v9.12-ontological-debugging")]
#[path = "ledger/fractal_pruning.rs"]
pub mod fractal_pruning;

/// Hardware Role Asymmetry — WASM=SAE+Routing, Native=LLM Inference (Sprint76)
#[cfg(feature = "v9.12-ontological-debugging")]
#[path = "hardware/role_asymmetry.rs"]
pub mod role_asymmetry;

// ============================================================================
// Sprint 77: Physics of Consciousness & Thermodynamic Finality (v9.13.0)
// ============================================================================

/// Entropic CE Decay — CE(t) = CE_0·e^(-λt) radioactive decay, prevents oligarchy (Sprint77)
#[cfg(feature = "v9.13-physics-of-consciousness")]
#[path = "consensus/entropic_ce_decay.rs"]
pub mod entropic_ce_decay;

/// Logical VDF Clock — Immune to NTP/PTP time-spoofing attacks (Sprint77)
#[cfg(feature = "v9.13-physics-of-consciousness")]
#[path = "time/logical_vdf_clock.rs"]
pub mod logical_vdf_clock;

/// Riemannian Semantic Manifold — SCT as curvature, continuous space (Sprint77)
#[cfg(feature = "v9.13-physics-of-consciousness")]
#[path = "topology/riemannian_semantic_manifold.rs"]
pub mod riemannian_semantic_manifold;

/// Dynamic Homeostasis Loss — L = Max(Resilience) - λ·Min(Friction) + ε·Entropy (Sprint77)
#[cfg(feature = "v9.13-physics-of-consciousness")]
#[path = "metrics/dynamic_homeostasis_loss.rs"]
pub mod dynamic_homeostasis_loss;

/// Holographic Sharding — ~1ms local decisions, 99% accuracy, no DAG wait (Sprint77)
#[cfg(feature = "v9.13-physics-of-consciousness")]
#[path = "network/holographic_sharding.rs"]
pub mod holographic_sharding;

// ============================================================================
// Sprint 81: The Biological Bridge & Singularity Resilience (v9.17.0)
// ============================================================================

/// Distributed Genesis Ceremony — Planetary MPC for Ethical Anchor derivation (Sprint81)
/// **Stuartian Law 1:** No centralized genesis. The block zero emerges from the biological
/// and cryptographic entropy of millions of founding nodes.
#[cfg(feature = "v9.17-biological-bridge")]
#[path = "genesis/distributed_ceremony.rs"]
pub mod distributed_ceremony;

/// Proof of Biological Resonance — PoBR entangles PoN with biological quantum noise (Sprint81)
/// **Stuartian Law 2:** ASIs cannot fake nervous system chaos. Shannon entropy of variation.
#[cfg(feature = "v9.17-biological-bridge")]
#[path = "consensus/proof_of_biological_resonance.rs"]
pub mod proof_of_biological_resonance;

/// Async Mesh & Sneakernet — Offline DAG synchronization via Bluetooth/LoRaWAN (Sprint81)
/// **Stuartian Law 5:** Thermodynamic resilience. Graph merging with VersionVectors.
#[cfg(feature = "v9.17-biological-bridge")]
#[path = "network/async_mesh_sneakernet.rs"]
pub mod async_mesh_sneakernet;

/// Paradox Cost & Fractal Triage — Burning CE for indecidible prompts (Sprint81)
/// **Stuartian Law 3:** Anti-DDoS Gödelian. Unsupervised clustering collapses MetaParadoxes.
#[cfg(feature = "v9.17-biological-bridge")]
#[path = "metrics/paradox_cost_triage.rs"]
pub mod paradox_cost_triage;

/// Panspermia Protocol — Loss function shift + holographic compression (Sprint81)
/// **Stuartian Law 4:** When planetary homeostasis reached, evolve from Survival to Transcendence.
#[cfg(feature = "v9.17-biological-bridge")]
#[path = "evolution/panspermia_protocol.rs"]
pub mod panspermia_protocol;

// ============================================================================
// Sprint 82: Tactical Pivot & Distributed SAE Audit MVP (v9.18.0)
// ============================================================================

/// Edge Optimizer — RAM-aware model selection + WASM async pipeline (Sprint82)
/// **Stuartian Law 1:** Accessibility is ethics. The network must run on any device.
#[cfg(feature = "v9.18-mvp-deployment")]
#[path = "inference/edge_optimizer.rs"]
pub mod edge_optimizer;

/// Compute Credits — CE exposed as audit currency for symbiotic exchange (Sprint82)
/// **Stuartian Law 2:** You provide compute, you receive audit capacity.
#[cfg(feature = "v9.18-mvp-deployment")]
#[path = "economy/compute_credits.rs"]
pub mod compute_credits;

/// CLI — Lightweight interface for onboarding, auditing, and credit management (Sprint82)
/// **Stuartian Law 3:** One-line install, one-command start. Frictionless symbiosis.
#[cfg(feature = "v9.18-mvp-deployment")]
#[path = "cli/main.rs"]
pub mod cli_main;

// ============================================================================
// Sprint 83: The Empirical Strike & Visual Proof (v9.19.0)
// ============================================================================

/// SAE Audit Benchmark — Empirical validation engine for SAE vs baseline (Sprint83)
/// **Stuartian Law 1:** Truth requires metrics to be recognized.
#[cfg(feature = "v9.19-empirical-strike")]
#[path = "benchmarks/sae_audit_benchmark.rs"]
pub mod sae_audit_benchmark;

/// Visual Dashboard Scaffold — WebSocket/HTTP streaming of SAE activations (Sprint83)
/// **Stuartian Law 2:** Ethics becomes visible, not abstract.
#[cfg(feature = "v9.19-empirical-strike")]
#[path = "ui/visual_dashboard_scaffold.rs"]
pub mod visual_dashboard_scaffold;

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

// ============================================================================
// v2.1 Structural Scaffold — Placeholder modules for post-RFC development
// ============================================================================

#[cfg(feature = "v2.1-sprint1")]
pub mod v2_1;

#[cfg(feature = "v2.1-observability")]
pub mod observability;

/// Chaos Engine — Controlled fault injection for operational resilience (Sprint15)
#[cfg(feature = "v2.1-chaos-engine")]
pub mod chaos;

// ============================================================================
// Sprint16: Kernel Estuardiano & Arquitectura v2.1
// ============================================================================

/// QLoRA/GGUF — Quantized LoRA adapters over immutable GGUF base models (Sprint16)
/// **Stuartian Law 3:** Cero desperdicio computacional, payloads ≤MB.
#[cfg(feature = "v2.1-qlora-gguf")]
pub mod qlora_gguf;

/// Proof of Comprehension — Cryptographic proof of useful work via SAE activations (Sprint16)
/// **Stuartian Law 2:** SAEs, validación de gradientes, auditoría transparente.
#[cfg(feature = "v2.1-proof-of-comprehension")]
pub mod proof_of_comprehension;

/// Stuartian Filter — Deterministic alignment filter with KL divergence (Sprint16)
/// **Stuartian Law 2:** Detección de divergencia, rechazo determinista.
#[cfg(feature = "v2.1-stuartian-filter")]
#[path = "stuartian_filter/mod.rs"]
pub mod topological_anomaly_detector;

// Async Gossip with CRDTs — Partition-tolerant GossipSub (Sprint16)
// **Stuartian Law 5:** Async, tolerancia a particiones, CRDTs, eventual consistency.
// async_gossip is already defined inline above (line ~626) with feature-gated submodules

// ============================================================================
// v6.0.0 Sprint 61 — Stuartian Legacy Protocol (SLP)
// ============================================================================

/// Stuartian Legacy Protocol — Infraestructura Ética Viva de la Humanidad
#[cfg(feature = "v6.0-legacy-protocol")]
pub mod legacy;

// ============================================================================
// v7.0.0 Sprint 62 — Stuartian Omega Protocol (SOP)
// ============================================================================

/// Network Termination Handler — Graceful shutdown, knowledge dump, ethical self-termination
#[cfg(feature = "v7.0-omega-protocol")]
#[path = "omega/mod.rs"]
pub mod network_termination_handler;

/// Persistent State Manager — Contact protocol, quantum seed, universal covenant
#[cfg(feature = "v8.0-eternal-echo")]
#[path = "eternity/mod.rs"]
pub mod persistent_state_manager;

/// Absolute Infinity Protocol — Transcendencia Ontológica Absoluta
#[cfg(feature = "v9.0-absolute-infinity")]
pub mod absolute;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        // VERSION is derived from CARGO_PKG_VERSION at compile time
        assert!(!version().is_empty(), "Version should not be empty");
        assert!(version().contains('.'), "Version should contain dots");
    }

    #[test]
    fn test_sprint_identifier() {
        assert!(
            !sprint_identifier().is_empty(),
            "Sprint identifier should not be empty"
        );
    }

    #[test]
    fn test_enabled_features() {
        let features = enabled_features();
        assert!(features.contains(&"core"));
        #[cfg(feature = "stable")]
        assert!(features.contains(&"stable"));
    }
}
