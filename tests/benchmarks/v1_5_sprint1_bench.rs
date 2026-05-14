//! v1.5.0 Sprint 1 Benchmarks
//!
//! Performance benchmarks for LP-118 through LP-121 modules.
//! Run with: cargo bench --features v1.5-sprint1
//!
//! | Benchmark | Module | Target |
//!|-----------|--------|--------|
//!| `bench_finetune_v5_sync` | Fine-Tuning v5 | <=120ms |
//!| `bench_pool_v4_allocation` | Cross-Chain Pools v4 | <=60ms |
//!| `bench_dynamic_router_v4` | Dynamic Router v4 | <=50ms |
//!| `bench_capacity_orchestrator` | Capacity Orchestrator | <=50ms |
//!| `bench_dao_ledger_v5` | DAO Ledger v5 | <=35ms |
//!| `bench_hybrid_governance` | Hybrid Governance | <=50ms |
//!| `bench_audit_trail_v2` | Audit Trail v2 | <=30ms |
//!| `bench_zkp_v9_proof` | Async ZKP v9 | <=600ms |
//!| `bench_zkp_v9_batch` | ZKP v9 Batch Processing | <=1000ms |
//!| `bench_full_pipeline` | Full E2E Pipeline | <=2000ms |

#[cfg(feature = "v1.5-sprint1")]
mod bench {
    use std::time::Instant;
    use std::collections::HashMap;

    // LP-118: SAE Fine-Tuning v5
    use ed2kia::sae::fine_tuning_v5::{FineTuningV5, FineTuningV5Config};

    // LP-119: Cross-Chain Pools v4 & Dynamic Routing
    use ed2kia::pools_v4::cross_chain_pools_v4::{CrossChainPoolsV4, PoolV4Config};
    use ed2kia::pools_v4::dynamic_router::{DynamicRouter, RouterConfig};
    use ed2kia::pools_v4::capacity_orchestrator::{CapacityOrchestrator, OrchestratorConfig};

    // LP-120: DAO Ledger v5 & Hybrid Governance
    use ed2kia::governance_v5::dao_ledger_v5::{DaoLedgerV5, DaoLedgerV5Config};
    use ed2kia::governance_v5::hybrid_governance::{HybridGovernance, HybridGovernanceConfig};
    use ed2kia::governance_v5::audit_trail_v2::{AuditTrailV2, AuditTrailConfig, AuditCategory, AuditSeverity};

    // LP-121: Async ZKP v9
    use ed2kia::zkp_v9::async_zkp_v9::{AsyncZKPV9, ZKPV9Config, ProofPriority};

    // =====================================================================
    // LP-118: Fine-Tuning v5 Benchmarks
    // =====================================================================

    pub fn bench_finetune_v5_sync() {
        let mut engine = FineTuningV5::new(FineTuningV5Config::default());
        engine.register_node("node-1".to_string(), 0.95, 0.9).unwrap();
        engine.register_model("model-1".to_string(), "node-1".to_string(), 128).unwrap();

        // Build gradients map
        let mut gradients = HashMap::new();
        gradients.insert("model-1".to_string(), vec![0.01f32; 128]);

        let start = Instant::now();
        for _ in 0..10 {
            engine.execute_round(gradients.clone()).unwrap();
        }
        let elapsed_ms = start.elapsed().as_millis();
        println!("bench_finetune_v5_sync: {}ms (10 rounds)", elapsed_ms);
        assert!(elapsed_ms <= 1200, "finetune_sync exceeds 1200ms for 10 rounds");
    }

    // =====================================================================
    // LP-119: Cross-Chain Pools v4 Benchmarks
    // =====================================================================

    pub fn bench_pool_v4_allocation() {
        let mut pools = CrossChainPoolsV4::new(PoolV4Config::default());
        pools.create_pool("pool-1".to_string()).unwrap();
        pools.add_chain_slot("pool-1", "chain-1".to_string(), 10000.0, 0.9).unwrap();

        let start = Instant::now();
        for i in 0..100 {
            pools.allocate("pool-1", 50.0).ok();
            if i % 20 == 0 {
                pools.update_load("pool-1", "chain-1", (i as f64) * 5.0).ok();
            }
        }
        let elapsed_ms = start.elapsed().as_millis();
        println!("bench_pool_v4_allocation: {}ms (100 ops)", elapsed_ms);
        assert!(elapsed_ms <= 600, "pool_routing exceeds 600ms for 100 ops");
    }

    // =====================================================================
    // LP-119: Dynamic Router v4 Benchmarks
    // =====================================================================

    pub fn bench_dynamic_router_v4() {
        let mut router = DynamicRouter::new(RouterConfig::default());
        for i in 0..10 {
            router.register_route(format!("target-{}", i), 0.8 + (i as f64) * 0.01).unwrap();
        }

        let start = Instant::now();
        for i in 0..100 {
            for j in 0..10 {
                router.record_latency(&format!("target-{}", j), 10.0 + (j as f64) * 2.0 + (i as f64 % 5.0)).ok();
            }
            router.decide().ok();
        }
        let elapsed_ms = start.elapsed().as_millis();
        println!("bench_dynamic_router_v4: {}ms (100 decisions)", elapsed_ms);
        assert!(elapsed_ms <= 500, "router exceeds 500ms for 100 decisions");
    }

    // =====================================================================
    // LP-119: Capacity Orchestrator Benchmarks
    // =====================================================================

    pub fn bench_capacity_orchestrator() {
        let mut orch = CapacityOrchestrator::new(OrchestratorConfig::default());
        for i in 0..5 {
            orch.register_pool(format!("pool-{}", i), 2000.0).unwrap();
        }

        let start = Instant::now();
        for i in 0..100 {
            for j in 0..5 {
                orch.update_demand(&format!("pool-{}", j), 500.0 + (i as f64) * 10.0).ok();
            }
            orch.decide(1000.0).ok();
        }
        let elapsed_ms = start.elapsed().as_millis();
        println!("bench_capacity_orchestrator: {}ms (100 decisions)", elapsed_ms);
        assert!(elapsed_ms <= 500, "orchestrator exceeds 500ms for 100 decisions");
    }

    // =====================================================================
    // LP-120: DAO Ledger v5 Benchmarks
    // =====================================================================

    pub fn bench_dao_ledger_v5() {
        let config = DaoLedgerV5Config {
            quorum_threshold: 0.01,
            approval_threshold: 0.40,
            ..DaoLedgerV5Config::default()
        };
        let mut ledger = DaoLedgerV5::new(config);

        let start = Instant::now();
        for i in 0..50 {
            let prop_id = format!("prop-{}", i);
            ledger.create_proposal(
                prop_id.clone(),
                format!("actor-{}", i % 5),
                format!("Proposal {}", i),
                format!("Description for proposal {}", i),
                false,
            ).unwrap();
            ledger.cast_vote(
                format!("vote-{}", i),
                prop_id.clone(),
                format!("voter-{}", i % 10),
                0.85,
                true,
            ).unwrap();
            if i % 3 == 0 {
                ledger.execute_proposal(&prop_id).ok();
            }
        }
        let elapsed_ms = start.elapsed().as_millis();
        println!("bench_dao_ledger_v5: {}ms (50 proposals)", elapsed_ms);
        assert!(elapsed_ms <= 350, "dao_ledger exceeds 350ms for 50 proposals");
    }

    // =====================================================================
    // LP-120: Hybrid Governance Benchmarks
    // =====================================================================

    pub fn bench_hybrid_governance() {
        let config = HybridGovernanceConfig {
            default_timelock_hours: 0,
            ..HybridGovernanceConfig::default()
        };
        let mut gov = HybridGovernance::new(config);

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let start = Instant::now();
        for i in 0..100 {
            let session_id = format!("session-{}", i);
            let proposal_id = format!("prop-{}", i);
            gov.create_session(session_id.clone(), proposal_id, false, now_ms).ok();
            gov.validate_off_chain(&session_id, now_ms).ok();
            gov.register_on_chain(&session_id, now_ms).ok();
            gov.execute(&session_id, now_ms).ok();
        }
        let elapsed_ms = start.elapsed().as_millis();
        println!("bench_hybrid_governance: {}ms (100 sessions)", elapsed_ms);
        assert!(elapsed_ms <= 500, "hybrid_governance exceeds 500ms for 100 sessions");
    }

    // =====================================================================
    // LP-120: Audit Trail v2 Benchmarks
    // =====================================================================

    pub fn bench_audit_trail_v2() {
        let mut audit = AuditTrailV2::new(AuditTrailConfig::default());

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let start = Instant::now();
        for i in 0..200 {
            audit.append_entry(
                format!("entry-{}", i),
                AuditCategory::Governance,
                AuditSeverity::Info,
                format!("actor-{}", i % 10),
                format!("Action {}", i),
                now_ms + i as u64,
            ).unwrap();
        }
        audit.verify_chain().ok();
        let elapsed_ms = start.elapsed().as_millis();
        println!("bench_audit_trail_v2: {}ms (200 entries + verify)", elapsed_ms);
        assert!(elapsed_ms <= 300, "audit_trail exceeds 300ms for 200 entries");
    }

    // =====================================================================
    // LP-121: Async ZKP v9 Benchmarks
    // =====================================================================

    pub fn bench_zkp_v9_proof() {
        let mut zkp = AsyncZKPV9::new(ZKPV9Config::default());
        zkp.register_federation("fed-1".to_string(), 0.9).unwrap();

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let start = Instant::now();
        for i in 0..10 {
            zkp.submit_proof(
                format!("proof-{}", i),
                "fed-1".to_string(),
                ProofPriority::High,
                1024,
                now_ms + i as u64,
            ).unwrap();
        }
        zkp.process_proofs(now_ms + 10);
        let elapsed_ms = start.elapsed().as_millis();
        println!("bench_zkp_v9_proof: {}ms (10 proofs)", elapsed_ms);
        assert!(elapsed_ms <= 600, "zkp_proof exceeds 600ms for 10 proofs");
    }

    pub fn bench_zkp_v9_batch() {
        let mut zkp = AsyncZKPV9::new(ZKPV9Config::default());
        zkp.register_federation("fed-1".to_string(), 0.9).unwrap();
        zkp.register_federation("fed-2".to_string(), 0.85).unwrap();

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let start = Instant::now();
        for i in 0..50 {
            zkp.submit_proof(
                format!("proof-{}", i),
                if i % 2 == 0 { "fed-1".to_string() } else { "fed-2".to_string() },
                ProofPriority::Normal,
                512,
                now_ms + i as u64,
            ).unwrap();
        }
        zkp.process_proofs(now_ms + 50);
        let elapsed_ms = start.elapsed().as_millis();
        println!("bench_zkp_v9_batch: {}ms (50 proofs batched)", elapsed_ms);
        assert!(elapsed_ms <= 1000, "zkp_batch exceeds 1000ms for 50 proofs");
    }

    // =====================================================================
    // Full Pipeline Benchmark
    // =====================================================================

    pub fn bench_full_pipeline() {
        let start = Instant::now();

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Fine-tuning
        let mut ft = FineTuningV5::new(FineTuningV5Config::default());
        ft.register_node("node-1".to_string(), 0.95, 0.9).unwrap();
        ft.register_model("model-1".to_string(), "node-1".to_string(), 64).unwrap();
        let mut gradients = HashMap::new();
        gradients.insert("model-1".to_string(), vec![0.01f32; 64]);
        for _ in 0..3 { ft.execute_round(gradients.clone()).unwrap(); }

        // Pools
        let mut pools = CrossChainPoolsV4::new(PoolV4Config::default());
        pools.create_pool("pool-1".to_string()).unwrap();
        pools.add_chain_slot("pool-1", "chain-1".to_string(), 5000.0, 0.9).unwrap();
        for _ in 0..10 { pools.allocate("pool-1", 100.0).ok(); }

        // Router
        let mut router = DynamicRouter::new(RouterConfig::default());
        router.register_route("chain-1".to_string(), 0.9).unwrap();
        router.record_latency("chain-1", 12.0).unwrap();
        router.decide().ok();

        // DAO
        let dao_config = DaoLedgerV5Config {
            quorum_threshold: 0.01,
            approval_threshold: 0.40,
            ..DaoLedgerV5Config::default()
        };
        let mut ledger = DaoLedgerV5::new(dao_config);
        ledger.create_proposal(
            "prop-1".to_string(),
            "system".to_string(),
            "Deploy".to_string(),
            "Deploy v1.5.0 changes".to_string(),
            false,
        ).unwrap();
        ledger.cast_vote("vote-1".to_string(), "prop-1".to_string(), "voter-1".to_string(), 0.95, true).unwrap();
        ledger.execute_proposal("prop-1").unwrap();

        // ZKP
        let mut zkp = AsyncZKPV9::new(ZKPV9Config::default());
        zkp.register_federation("fed-1".to_string(), 0.9).unwrap();
        zkp.submit_proof("proof-1".to_string(), "fed-1".to_string(), ProofPriority::High, 512, now_ms).unwrap();
        zkp.process_proofs(now_ms + 1);

        let elapsed_ms = start.elapsed().as_millis();
        println!("bench_full_pipeline: {}ms", elapsed_ms);
        assert!(elapsed_ms <= 2000, "full_pipeline exceeds 2000ms");
    }

    // =====================================================================
    // Main Runner
    // =====================================================================

    pub fn run_all() {
        println!("=== v1.5.0 Sprint 1 Benchmarks ===\n");

        println!("--- LP-118: Fine-Tuning v5 ---");
        bench_finetune_v5_sync();

        println!("\n--- LP-119: Pools v4 & Routing ---");
        bench_pool_v4_allocation();
        bench_dynamic_router_v4();
        bench_capacity_orchestrator();

        println!("\n--- LP-120: DAO Ledger v5 & Governance ---");
        bench_dao_ledger_v5();
        bench_hybrid_governance();
        bench_audit_trail_v2();

        println!("\n--- LP-121: Async ZKP v9 ---");
        bench_zkp_v9_proof();
        bench_zkp_v9_batch();

        println!("\n--- Full Pipeline ---");
        bench_full_pipeline();

        println!("\n=== All Benchmarks Passed ===");
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "v1.5-sprint1")]
    #[test]
    fn test_bench_finetune_v5_sync() {
        super::bench::bench_finetune_v5_sync();
    }

    #[cfg(feature = "v1.5-sprint1")]
    #[test]
    fn test_bench_pool_v4_allocation() {
        super::bench::bench_pool_v4_allocation();
    }

    #[cfg(feature = "v1.5-sprint1")]
    #[test]
    fn test_bench_dynamic_router_v4() {
        super::bench::bench_dynamic_router_v4();
    }

    #[cfg(feature = "v1.5-sprint1")]
    #[test]
    fn test_bench_capacity_orchestrator() {
        super::bench::bench_capacity_orchestrator();
    }

    #[cfg(feature = "v1.5-sprint1")]
    #[test]
    fn test_bench_dao_ledger_v5() {
        super::bench::bench_dao_ledger_v5();
    }

    #[cfg(feature = "v1.5-sprint1")]
    #[test]
    fn test_bench_hybrid_governance() {
        super::bench::bench_hybrid_governance();
    }

    #[cfg(feature = "v1.5-sprint1")]
    #[test]
    fn test_bench_audit_trail_v2() {
        super::bench::bench_audit_trail_v2();
    }

    #[cfg(feature = "v1.5-sprint1")]
    #[test]
    fn test_bench_zkp_v9_proof() {
        super::bench::bench_zkp_v9_proof();
    }

    #[cfg(feature = "v1.5-sprint1")]
    #[test]
    fn test_bench_zkp_v9_batch() {
        super::bench::bench_zkp_v9_batch();
    }

    #[cfg(feature = "v1.5-sprint1")]
    #[test]
    fn test_bench_full_pipeline() {
        super::bench::bench_full_pipeline();
    }
}

fn main() {
    #[cfg(feature = "v1.5-sprint1")]
    bench::run_all();

    #[cfg(not(feature = "v1.5-sprint1"))]
    println!("Benchmarks require feature 'v1.5-sprint1'");
}
