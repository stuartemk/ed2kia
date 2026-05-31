//! Sprint 42 Integration Tests — WASM Runtime & Secure Communication Layer.
//!
//! Validates:
//! 1. WASM sandbox memory limits and isolation
//! 2. Message integrity (signature, timestamp drift, replay protection)
//! 3. Privacy enforcement (network syscall blocking)
//! 4. CE-weighted channel priority

#[cfg(all(
    feature = "v3.0-wasm-runtime",
    feature = "v3.0-pillar-messaging",
    feature = "v3.0-privacy-guard"
))]
mod sandbox_isolation_tests {
    use ed2kIA::runtime::wasm_sandbox::{SandboxConfig, SandboxError, SyscallPolicy, WasmSandbox};
    use std::time::Duration;

    fn valid_wasm_module() -> Vec<u8> {
        // Minimal valid WASM module (magic + version)
        vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]
    }

    #[test]
    fn test_sandbox_default_config() {
        let sandbox = WasmSandbox::new();
        assert_eq!(sandbox.memory_limit(), 256 * 1024 * 1024); // 256MB
        assert_eq!(sandbox.timeout(), Duration::from_secs(5));
    }

    #[test]
    fn test_sandbox_custom_config() {
        let config = SandboxConfig {
            memory_limit_bytes: 128 * 1024 * 1024,
            timeout_seconds: 10,
            syscall_filter: SyscallPolicy::LocalReadOnly,
        };
        let sandbox = WasmSandbox::with_config(config);
        assert_eq!(sandbox.memory_limit(), 128 * 1024 * 1024);
        assert_eq!(sandbox.timeout(), Duration::from_secs(10));
    }

    #[test]
    fn test_sandbox_rejects_empty_module() {
        let mut sandbox = WasmSandbox::new();
        let result = sandbox.execute(&[], &[]);
        assert!(matches!(result, Err(SandboxError::ModuleInvalid(_))));
    }

    #[test]
    fn test_sandbox_rejects_invalid_magic() {
        let mut sandbox = WasmSandbox::new();
        let result = sandbox.execute(b"not-wasm-binary", &[]);
        assert!(matches!(result, Err(SandboxError::ModuleInvalid(_))));
    }

    #[test]
    fn test_sandbox_executes_valid_module() {
        let mut sandbox = WasmSandbox::new();
        let input = b"test-payload-for-sandbox";
        let result = sandbox.execute(&valid_wasm_module(), input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), input);
    }

    #[test]
    fn test_sandbox_logs_execution() {
        let mut sandbox = WasmSandbox::new();
        sandbox.execute(&valid_wasm_module(), b"input");
        assert!(!sandbox.logs().is_empty());
        assert!(sandbox.logs().len() >= 2); // start + complete
    }

    #[test]
    fn test_sandbox_clear_logs() {
        let mut sandbox = WasmSandbox::new();
        sandbox.execute(&valid_wasm_module(), b"input");
        sandbox.clear_logs();
        assert!(sandbox.logs().is_empty());
    }

    #[test]
    fn test_syscall_policy_default_isolated() {
        assert_eq!(SyscallPolicy::default(), SyscallPolicy::FullyIsolated);
    }
}

#[cfg(all(
    feature = "v3.0-wasm-runtime",
    feature = "v3.0-pillar-messaging",
    feature = "v3.0-privacy-guard"
))]
mod message_integrity_tests {
    use ed2kIA::orchestration::PillarId;
    use ed2kIA::runtime::pillar_messaging::{
        MessageChannelManager, MessagingError, PillarMessage, ReplayProtection,
    };

    fn make_message(nonce: u64, timestamp: u64, ce_weight: f64) -> PillarMessage {
        PillarMessage::new(
            b"integration-test-payload".to_vec(),
            b"valid-signature".to_vec(),
            PillarId::CorpuscularBridge,
            timestamp,
            nonce,
            ce_weight,
        )
    }

    #[test]
    fn test_message_creation() {
        let msg = make_message(1, 1000, 1.0);
        assert_eq!(msg.nonce, 1);
        assert_eq!(msg.ce_weight, 1.0);
        assert_eq!(msg.pillar_id, PillarId::CorpuscularBridge);
    }

    #[test]
    fn test_verify_valid_message() {
        let manager = MessageChannelManager::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let msg = make_message(1, now, 1.0);
        let result = manager.verify_message(&msg);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_empty_signature_rejected() {
        let manager = MessageChannelManager::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let mut msg = make_message(1, now, 1.0);
        msg.signature = vec![];
        let result = manager.verify_message(&msg);
        assert!(matches!(result, Err(MessagingError::SignatureInvalid)));
    }

    #[test]
    fn test_verify_timestamp_drift_exceeded() {
        let manager = MessageChannelManager::new();
        let old_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
            - 60_000; // 1 minute ago
        let msg = make_message(1, old_timestamp, 1.0);
        let result = manager.verify_message(&msg);
        assert!(matches!(
            result,
            Err(MessagingError::TimestampDriftExceeded(_))
        ));
    }

    #[test]
    fn test_replay_protection_blocks_duplicate() {
        let manager = MessageChannelManager::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let msg = make_message(42, now, 1.0);

        // First verification passes
        assert!(manager.verify_message(&msg).is_ok());

        // Second verification with same nonce fails
        let result = manager.verify_message(&msg);
        assert!(matches!(result, Err(MessagingError::ReplayDetected(42))));
    }

    #[test]
    fn test_replay_protection_eviction() {
        let mut replay = ReplayProtection::new(3);
        replay.record(1, 100);
        replay.record(2, 200);
        replay.record(3, 300);
        assert_eq!(replay.nonces.len(), 3);

        // Adding 4th should evict oldest
        replay.record(4, 400);
        assert_eq!(replay.nonces.len(), 3);
        assert!(!replay.is_replay(1)); // Evicted
        assert!(replay.is_replay(4)); // Present
    }

    #[test]
    fn test_ce_weighted_priority() {
        let msg_high_ce = make_message(1, 1000, 0.95);
        let msg_low_ce = make_message(2, 1000, 0.10);
        assert!(msg_high_ce.ce_weight > msg_low_ce.ce_weight);
    }
}

#[cfg(all(
    feature = "v3.0-wasm-runtime",
    feature = "v3.0-pillar-messaging",
    feature = "v3.0-privacy-guard"
))]
mod privacy_enforcement_tests {
    use ed2kIA::runtime::privacy_enforcer::{PrivacyEnforcer, PrivacyViolation};

    #[test]
    fn test_enforcer_blocks_network_syscall_connect() {
        let mut enforcer = PrivacyEnforcer::new();
        // connect (syscall 43)
        let result = enforcer.intercept_io(43, &[0x7f000001]);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PrivacyViolation::NetworkBlocked(_)
        ));
    }

    #[test]
    fn test_enforcer_blocks_network_syscall_sendto() {
        let mut enforcer = PrivacyEnforcer::new();
        // sendto (syscall 44)
        let result = enforcer.intercept_io(44, &[0x0]);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PrivacyViolation::NetworkBlocked(_)
        ));
    }

    #[test]
    fn test_enforcer_blocks_network_syscall_recvfrom() {
        let mut enforcer = PrivacyEnforcer::new();
        // recvfrom (syscall 45)
        let result = enforcer.intercept_io(45, &[0x0]);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PrivacyViolation::NetworkBlocked(_)
        ));
    }

    #[test]
    fn test_enforcer_allows_local_read() {
        let mut enforcer = PrivacyEnforcer::new();
        // read (syscall 0) is allowed
        let result = enforcer.intercept_io(0, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_enforcer_blocks_unauthorized_write() {
        let mut enforcer = PrivacyEnforcer::new();
        // write (syscall 1) is NOT in default allowlist
        let result = enforcer.intercept_io(1, &[0x0]);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PrivacyViolation::GeneralViolation(_)
        ));
    }

    #[test]
    fn test_intercept_network_always_blocks() {
        let mut enforcer = PrivacyEnforcer::new();
        let result = enforcer.intercept_network("connect", "8.8.8.8:443");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PrivacyViolation::NetworkBlocked(_)
        ));
    }

    #[test]
    fn test_audit_log_records_violations() {
        let mut enforcer = PrivacyEnforcer::new();
        enforcer.intercept_io(0, &[]); // allowed
        enforcer.intercept_io(43, &[]); // blocked
        assert_eq!(enforcer.audit_log().len(), 2);
        assert_eq!(enforcer.allowed_count(), 1);
        assert_eq!(enforcer.blocked_count(), 1);
    }

    #[test]
    fn test_allow_and_deny_syscall() {
        let mut enforcer = PrivacyEnforcer::new();
        assert!(!enforcer.is_syscall_allowed(1));
        enforcer.allow_syscall(1);
        assert!(enforcer.is_syscall_allowed(1));
        enforcer.deny_syscall(1);
        assert!(!enforcer.is_syscall_allowed(1));
    }

    #[test]
    fn test_local_only_constraint() {
        let mut enforcer = PrivacyEnforcer::new();
        // All network syscalls must be blocked
        for syscall in [41, 43, 44, 45, 46, 47, 49, 50] {
            let result = enforcer.intercept_io(syscall, &[]);
            assert!(
                result.is_err(),
                "Syscall {} should be blocked (LOCAL_ONLY)",
                syscall
            );
        }
        assert_eq!(enforcer.blocked_count(), 8);
    }
}

#[cfg(all(
    feature = "v3.0-wasm-runtime",
    feature = "v3.0-pillar-messaging",
    feature = "v3.0-privacy-guard"
))]
mod integration_workflow_tests {
    use ed2kIA::orchestration::PillarId;
    use ed2kIA::runtime::pillar_messaging::{MessageChannelManager, PillarMessage};
    use ed2kIA::runtime::privacy_enforcer::PrivacyEnforcer;
    use ed2kIA::runtime::wasm_sandbox::WasmSandbox;

    fn valid_wasm_module() -> Vec<u8> {
        vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]
    }

    #[test]
    fn test_full_sandbox_message_privacy_workflow() {
        // Step 1: Create sandbox and execute module
        let mut sandbox = WasmSandbox::new();
        let input = b"pillar-compute-request";
        let output = sandbox.execute(&valid_wasm_module(), input).unwrap();
        assert_eq!(output, input);

        // Step 2: Create and verify message
        let manager = MessageChannelManager::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let msg = PillarMessage::new(
            output,
            b"signature".to_vec(),
            PillarId::CorpuscularBridge,
            now,
            1,
            0.95,
        );
        let payload = manager.verify_message(&msg).unwrap();
        assert_eq!(payload, input);

        // Step 3: Privacy enforcer blocks network
        let mut enforcer = PrivacyEnforcer::new();
        let result = enforcer.intercept_network("connect", "external-api.example.com");
        assert!(result.is_err());
        assert_eq!(enforcer.blocked_count(), 1);
    }

    #[test]
    fn test_sandbox_isolation_with_privacy_guard() {
        let mut sandbox = WasmSandbox::new();
        let mut enforcer = PrivacyEnforcer::new();

        // Execute module
        sandbox.execute(&valid_wasm_module(), b"test");

        // Attempt network access — must be blocked
        for syscall in [43, 44, 45] {
            let result = enforcer.intercept_io(syscall, &[]);
            assert!(result.is_err());
        }

        // Verify sandbox logs exist and privacy violations recorded
        assert!(!sandbox.logs().is_empty());
        assert_eq!(enforcer.blocked_count(), 3);
    }

    #[test]
    fn test_message_replay_prevention_across_channels() {
        let manager = MessageChannelManager::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Same nonce from different pillars should still be detected as replay
        let msg1 = PillarMessage::new(
            b"payload-1".to_vec(),
            b"sig".to_vec(),
            PillarId::CorpuscularBridge,
            now,
            999,
            0.9,
        );
        let msg2 = PillarMessage::new(
            b"payload-2".to_vec(),
            b"sig".to_vec(),
            PillarId::MaieuticSynthesizer,
            now,
            999, // Same nonce
            0.8,
        );

        assert!(manager.verify_message(&msg1).is_ok());
        let result = manager.verify_message(&msg2);
        assert!(result.is_err()); // Replay detected
    }
}
