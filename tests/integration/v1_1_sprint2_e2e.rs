//! ed2kIA v1.1.0 Sprint 2 - E2E Integration Tests
//!
//! End-to-end tests for Dynamic SLO/SLA, Liquid Governance v2, and Realtime Telemetry.

#[cfg(feature = "v1.1-sprint2")]
mod e2e {
    use ed2kia::slo::dynamic_engine::*;
    use ed2kia::slo::contract_manager::*;
    use ed2kia::governance::liquid_v2::*;
    use ed2kia::governance::voting_mechanism::*;
    use ed2kia::web::realtime::*;
    use ed2kia::monitoring::streaming_metrics::*;

    // ========================================================================
    // Dynamic SLO Engine E2E
    // ========================================================================

    #[test]
    fn test_slo_engine_full_lifecycle() {
        let mut engine = DynamicSLOEngine::default_engine();

        // Add rule
        let rule = SLORule::new(
            "rule-1".to_string(),
            "CPU Usage".to_string(),
            "cpu_usage".to_string(),
            80.0,
            0.90,   // warning_threshold (fraction of threshold)
            1.20,   // critical_threshold (multiplier of threshold)
            60,
            3,
        );
        engine.add_rule(rule).expect("add rule");

        // Report healthy metric
        engine.report_metric("cpu_usage", 45.0);

        // Evaluate
        let results = engine.evaluate_cycle().expect("evaluate");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].compliance, SLOCompliance::Healthy);

        // Report breaching metric
        engine.report_metric("cpu_usage", 120.0);
        let results = engine.evaluate_cycle().expect("evaluate");
        assert!(results[0].compliance == SLOCompliance::Breach || results[0].compliance == SLOCompliance::Warning);

        // Stats
        let stats = engine.get_stats();
        assert_eq!(stats.total_rules, 1);
        assert_eq!(stats.active_rules, 1);
    }

    #[test]
    fn test_slo_cpu_fallback() {
        let config = DynamicSLOConfig {
            max_evaluation_ms: 50.0,
            cpu_fallback_threshold: 0.85,
            max_samples_per_rule: 100,
            cycle_interval: std::time::Duration::from_secs(10),
            enable_dynamic_thresholds: true,
            adjustment_rate: 0.1,
        };
        let mut engine = DynamicSLOEngine::new(config);

        let rule = SLORule::new(
            "rule-1".to_string(),
            "Latency".to_string(),
            "latency".to_string(),
            100.0,
            0.70,
            2.00,
            30,
            2,
        );
        engine.add_rule(rule).expect("add rule");
        engine.report_metric("latency", 50.0);

        // Normal evaluation
        assert!(!engine.get_stats().static_fallback_active);

        // Trigger CPU fallback
        engine.update_cpu_load(0.90);
        let _results = engine.evaluate_cycle();
        assert!(engine.get_stats().static_fallback_active);

        // Recovery
        engine.update_cpu_load(0.70);
        let _ = engine.evaluate_cycle();
        assert!(!engine.get_stats().static_fallback_active);
    }

    // ========================================================================
    // SLA Contract Manager E2E
    // ========================================================================

    #[test]
    fn test_sla_contract_full_lifecycle() {
        let mut manager = ContractManager::new();

        // Create contract using SLAContract::new()
        let metric = ContractMetric::new(
            "uptime".to_string(),
            "Uptime".to_string(),
            99.9,
            99.0,
            100.0,
            "%".to_string(),
        );
        let clause = ContractClause::new(
            "clause-1".to_string(),
            ClauseType::Penalty,
            "uptime".to_string(),
            99.5,
            "Penalty for low uptime".to_string(),
            1000.0,
        );
        let contract = SLAContract::new(
            "sla-1".to_string(),
            "API SLA".to_string(),
            "99.9% uptime".to_string(),
            "provider-1".to_string(),
            "consumer-1".to_string(),
            std::time::Duration::from_secs(86400),
            std::time::Duration::from_secs(3600),
            vec![metric],
            vec![clause],
            3,
        ).expect("create contract");
        let _ = manager.register_contract(contract);

        // Validate with compliant metrics
        let mut metrics = std::collections::HashMap::new();
        metrics.insert("uptime".to_string(), 99.95);
        let result = manager.validate_contract("sla-1", &metrics).expect("validate");
        assert!(result.compliant);

        // Stats
        let stats = manager.get_stats();
        assert_eq!(stats.total_contracts, 1);
    }

    #[test]
    fn test_sla_contract_violation() {
        let mut manager = ContractManager::new();

        let metric = ContractMetric::new(
            "latency".to_string(),
            "Latency".to_string(),
            100.0,
            0.0,
            200.0,
            "ms".to_string(),
        );
        let clause = ContractClause::new(
            "clause-penalty".to_string(),
            ClauseType::Penalty,
            "latency".to_string(),
            150.0,
            "High latency penalty".to_string(),
            500.0,
        );
        let contract = SLAContract::new(
            "sla-2".to_string(),
            "Latency SLA".to_string(),
            "< 100ms".to_string(),
            "p1".to_string(),
            "c1".to_string(),
            std::time::Duration::from_secs(86400),
            std::time::Duration::ZERO,
            vec![metric],
            vec![clause],
            5,
        ).expect("create contract");
        let _ = manager.register_contract(contract);

        // Validate with violating metrics
        let mut metrics = std::collections::HashMap::new();
        metrics.insert("latency".to_string(), 250.0);
        let result = manager.validate_contract("sla-2", &metrics).expect("validate");
        assert!(!result.compliant);
        assert!(result.total_penalty > 0.0);
    }

    // ========================================================================
    // Liquid Governance v2 E2E
    // ========================================================================

    #[test]
    fn test_governance_v2_full_flow() {
        let mut gov = LiquidGovernanceV2::new();

        // Register nodes using struct literals
        gov.register_node(NodeProfileV2 {
            node_id: "node-1".to_string(),
            trust_score: 0.9,
            staking_credits: 1000.0,
            uptime_history: 0.99,
            crypto_signature: "sig-1".to_string(),
            asn: "ASN1".to_string(),
            ip_prefix: "10.0.0".to_string(),
            voting_history: vec![],
            reputation_score: 0.95,
        }).expect("register node-1");

        gov.register_node(NodeProfileV2 {
            node_id: "node-2".to_string(),
            trust_score: 0.8,
            staking_credits: 500.0,
            uptime_history: 0.95,
            crypto_signature: "sig-2".to_string(),
            asn: "ASN2".to_string(),
            ip_prefix: "10.0.1".to_string(),
            voting_history: vec![],
            reputation_score: 0.85,
        }).expect("register node-2");

        gov.register_node(NodeProfileV2 {
            node_id: "node-3".to_string(),
            trust_score: 0.85,
            staking_credits: 800.0,
            uptime_history: 0.97,
            crypto_signature: "sig-3".to_string(),
            asn: "ASN3".to_string(),
            ip_prefix: "10.0.2".to_string(),
            voting_history: vec![],
            reputation_score: 0.90,
        }).expect("register node-3");

        // Create proposal using create_proposal(title, description, proposer, critical)
        let _proposal = gov.create_proposal("Upgrade", "System upgrade", "node-1", false)
            .expect("create proposal");

        // Cast votes
        gov.cast_vote("node-1", "prop-1", true).expect("vote node-1");
        gov.cast_vote("node-2", "prop-1", true).expect("vote node-2");
        gov.cast_vote("node-3", "prop-1", true).expect("vote node-3");

        // Stats
        let stats = gov.get_stats();
        assert_eq!(stats.total_proposals, 1);
    }

    #[test]
    fn test_governance_v2_critical_timelock() {
        let mut gov = LiquidGovernanceV2::new();

        gov.register_node(NodeProfileV2 {
            node_id: "node-1".to_string(),
            trust_score: 0.95,
            staking_credits: 2000.0,
            uptime_history: 0.99,
            crypto_signature: "sig-1".to_string(),
            asn: "ASN1".to_string(),
            ip_prefix: "10.0.0".to_string(),
            voting_history: vec![],
            reputation_score: 0.95,
        }).expect("register node-1");

        // Critical proposal has time-lock
        let _proposal = gov.create_proposal("Critical Change", "Emergency fix", "node-1", true)
            .expect("create proposal");

        gov.cast_vote("node-1", "prop-1", true).expect("vote");

        // Should fail due to time-lock
        let result = gov.execute_proposal("prop-1");
        assert!(result.is_err());
    }

    #[test]
    fn test_governance_v2_sybil_detection() {
        let mut gov = LiquidGovernanceV2::new();

        // Register nodes with same ASN and IP prefix (potential Sybil)
        gov.register_node(NodeProfileV2 {
            node_id: "sybil-1".to_string(),
            trust_score: 0.2,
            staking_credits: 100.0,
            uptime_history: 0.5,
            crypto_signature: "sig-a".to_string(),
            asn: "SAME_ASN".to_string(),
            ip_prefix: "10.0.0".to_string(),
            voting_history: vec![],
            reputation_score: 0.5,
        }).expect("register sybil-1");

        gov.register_node(NodeProfileV2 {
            node_id: "sybil-2".to_string(),
            trust_score: 0.2,
            staking_credits: 100.0,
            uptime_history: 0.5,
            crypto_signature: "sig-b".to_string(),
            asn: "SAME_ASN".to_string(),
            ip_prefix: "10.0.0".to_string(),
            voting_history: vec![],
            reputation_score: 0.5,
        }).expect("register sybil-2");

        let clusters = gov.detect_sybil_cluster();
        assert!(!clusters.is_empty());
    }

    // ========================================================================
    // Voting Mechanism E2E
    // ========================================================================

    #[test]
    fn test_voting_batch_processing() {
        let mut mechanism = VotingMechanism::default_mechanism();

        // Register voters
        mechanism.register_voter("voter-1", 100.0);
        mechanism.register_voter("voter-2", 50.0);
        mechanism.register_voter("voter-3", 75.0);

        // Submit votes
        mechanism.submit_vote("voter-1", "prop-1", VoteDirection::For, "sig-1")
            .expect("vote 1");
        mechanism.submit_vote("voter-2", "prop-1", VoteDirection::For, "sig-2")
            .expect("vote 2");
        mechanism.submit_vote("voter-3", "prop-1", VoteDirection::Against, "sig-3")
            .expect("vote 3");

        // Process batch
        let batch = mechanism.process_batch("prop-1").expect("batch");
        assert!(batch.decided.is_some());
        assert!(batch.total_weight_for > batch.total_weight_against);

        // Stats
        let stats = mechanism.get_stats();
        assert_eq!(stats.total_votes, 3);
    }

    // ========================================================================
    // Realtime Telemetry E2E
    // ========================================================================

    #[test]
    fn test_telemetry_full_pipeline() {
        let mut backend = TelemetryBackend::new();

        // Create sessions with different subscriptions
        let _ = backend.create_session(
            "metrics-client".to_string(),
            vec![TelemetryEventType::Metrics, TelemetryEventType::System],
        );
        let _ = backend.create_session(
            "gov-client".to_string(),
            vec![TelemetryEventType::Governance],
        );

        // Publish metrics event
        let result = backend.publish_event(
            TelemetryEventType::Metrics,
            serde_json::json!({"cpu": 75.0, "memory": 60.0}),
            Some("node-1".to_string()),
        );
        assert_eq!(result.events_sent, 1);
        assert_eq!(result.events_filtered, 1);

        // Publish governance event
        let result = backend.publish_event(
            TelemetryEventType::Governance,
            serde_json::json!({"proposal": "prop-1", "votes": 42}),
            Some("node-2".to_string()),
        );
        assert_eq!(result.events_sent, 1);
        assert_eq!(result.events_filtered, 1);

        // Catch-up events
        let catchup = backend.get_catchup_events(0, &[TelemetryEventType::Metrics]);
        assert_eq!(catchup.len(), 1);

        // Stats
        let stats = backend.get_stats();
        assert_eq!(stats.active_sessions, 2);
        assert_eq!(stats.total_events_sent, 2);
        assert_eq!(stats.current_sequence, 2);
    }

    #[test]
    fn test_telemetry_sse_formatting() {
        let event = TelemetryEvent::new(
            TelemetryEventType::Metrics,
            serde_json::json!({"latency": 42.5}),
            Some("node-1".to_string()),
            1,
        );
        let sse = TelemetryBackend::format_sse_event(&event);
        assert!(sse.contains("event: metrics"));
        assert!(sse.contains("data:"));
        assert!(sse.contains(&event.event_id));
        assert!(sse.ends_with("\n\n"));
    }

    // ========================================================================
    // Streaming Metrics E2E
    // ========================================================================

    #[test]
    fn test_streaming_metrics_pipeline() {
        let mut collector = StreamingMetricsCollector::new();

        // Register metrics
        collector.register_metric("cpu_usage".to_string(), MetricType::Gauge);
        collector.register_metric("request_count".to_string(), MetricType::Counter);
        collector.register_metric("latency_ms".to_string(), MetricType::Histogram);

        // Record data
        collector.record_simple("cpu_usage", 45.0);
        collector.record_simple("cpu_usage", 55.0);
        collector.record_simple("request_count", 100.0);
        collector.record_simple("request_count", 200.0);
        collector.record_simple("latency_ms", 10.0);
        collector.record_simple("latency_ms", 25.0);
        collector.record_simple("latency_ms", 45.0);

        // Aggregate
        let results = collector.aggregate_cycle();
        assert_eq!(results.len(), 3);

        // Verify values
        let cpu = results.iter().find(|r| r.0 == "cpu_usage").unwrap();
        assert_eq!(cpu.1, 55.0); // Gauge = last value

        let req = results.iter().find(|r| r.0 == "request_count").unwrap();
        assert_eq!(req.1, 200.0); // Counter = last value

        let lat = results.iter().find(|r| r.0 == "latency_ms").unwrap();
        assert_eq!(lat.1, 26.666666666666668); // Histogram = avg

        // Emit payload
        let payload = collector.emit_payload();
        assert!(payload.get("metrics").is_some());
        assert!(payload.get("stats").is_some());

        // Histogram summary
        let summary = collector.get_histogram_summary("latency_ms").unwrap();
        assert_eq!(summary.count, 3);
        assert_eq!(summary.sum, 80.0);
    }

    // ========================================================================
    // Cross-Module Integration
    // ========================================================================

    #[test]
    fn test_slo_to_telemetry_integration() {
        let mut engine = DynamicSLOEngine::default_engine();
        let mut backend = TelemetryBackend::new();

        // Setup SLO rule
        let rule = SLORule::new(
            "rule-latency".to_string(),
            "API Latency".to_string(),
            "api_latency".to_string(),
            100.0,
            0.70,
            2.00,
            60,
            3,
        );
        engine.add_rule(rule).expect("add rule");

        // Create telemetry session
        let _ = backend.create_session(
            "slo-monitor".to_string(),
            vec![TelemetryEventType::Slo],
        );

        // Report metric and evaluate
        engine.report_metric("api_latency", 150.0);
        let results = engine.evaluate_cycle().expect("evaluate");

        // Publish SLO result to telemetry
        let payload = serde_json::json!({
            "rule_id": results[0].rule_id,
            "compliance": results[0].compliance.to_string(),
            "value": results[0].current_value,
            "threshold": results[0].threshold,
        });
        let telemetry_result = backend.publish_event(
            TelemetryEventType::Slo,
            payload,
            Some("slo-engine".to_string()),
        );
        assert_eq!(telemetry_result.events_sent, 1);
    }

    #[test]
    fn test_governance_to_telemetry_integration() {
        let mut gov = LiquidGovernanceV2::new();
        let mut backend = TelemetryBackend::new();

        gov.register_node(NodeProfileV2 {
            node_id: "node-1".to_string(),
            trust_score: 0.9,
            staking_credits: 1000.0,
            uptime_history: 0.99,
            crypto_signature: "sig-1".to_string(),
            asn: "ASN1".to_string(),
            ip_prefix: "10.0.0".to_string(),
            voting_history: vec![],
            reputation_score: 0.95,
        }).expect("register node-1");

        let _ = backend.create_session(
            "gov-monitor".to_string(),
            vec![TelemetryEventType::Governance],
        );

        let _proposal = gov.create_proposal("Feature Flag", "Enable new feature", "node-1", false)
            .expect("create proposal");

        // Publish governance event
        let payload = serde_json::json!({
            "proposal_id": "prop-1",
            "action": "created",
            "proposer": "node-1",
        });
        let result = backend.publish_event(
            TelemetryEventType::Governance,
            payload,
            Some("node-1".to_string()),
        );
        assert_eq!(result.events_sent, 1);
    }

    #[test]
    fn test_streaming_metrics_with_telemetry() {
        let mut collector = StreamingMetricsCollector::new();
        let mut backend = TelemetryBackend::new();

        collector.register_metric("system_cpu".to_string(), MetricType::Gauge);
        collector.register_metric("system_mem".to_string(), MetricType::Gauge);

        let _ = backend.create_session(
            "metrics-subscriber".to_string(),
            vec![TelemetryEventType::Metrics],
        );

        // Record and aggregate
        collector.record_simple("system_cpu", 65.0);
        collector.record_simple("system_mem", 72.0);
        let results = collector.aggregate_cycle();

        // Emit to telemetry
        let payload = serde_json::json!({
            "metrics": results.iter().map(|(name, value, _)| {
                (name.clone(), *value)
            }).collect::<std::collections::HashMap<_, _>>(),
        });
        let result = backend.publish_event(
            TelemetryEventType::Metrics,
            payload,
            None,
        );
        assert_eq!(result.events_sent, 1);
    }
}
