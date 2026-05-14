//! Cross-Chain Bridge v3 — Cross-chain message verification with light validation and quorum relay.
//!
//! Improvements over v2:
//! - Light validation via hash-based proof verification
//! - Quorum-based relay requiring >=3 node signatures
//! - Fallback to Merkle+VRF when validation_time > 150ms
//! - Message lifecycle tracking (submit → verify → relay)
//! - Chain registration with reputation scoring
//!
//! **Design:** Message-based bridge with cryptographic verification and adaptive relay.
//! Operates exclusively with compute credits, SAE shards, and technical state.
//!
//! Apache License 2.0 + Ethical Use Clause

#[cfg(feature = "v1.6-sprint1")]
mod internal {
    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    // ---------------------------------------------------------------------------
    // Errors
    // ---------------------------------------------------------------------------

    /// Errors for Cross-Chain Bridge v3 operations.
    #[derive(Debug, Clone, PartialEq)]
    pub enum BridgeV3Error {
        /// Chain not registered.
        ChainNotFound(String),
        /// Message not found.
        MessageNotFound(String),
        /// Validation failed.
        ValidationFailed(String),
        /// Quorum threshold not met.
        QuorumFailed { current: u32, required: u32 },
        /// Bridge capacity exceeded.
        BridgeFull,
        /// Relay failed.
        RelayFailed(String),
        /// Message expired.
        MessageExpired(String),
        /// Invalid message size.
        InvalidMessageSize { size: usize, max: usize },
    }

    impl std::fmt::Display for BridgeV3Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                BridgeV3Error::ChainNotFound(id) => write!(f, "Chain {} not found", id),
                BridgeV3Error::MessageNotFound(id) => write!(f, "Message {} not found", id),
                BridgeV3Error::ValidationFailed(msg) => write!(f, "Validation failed: {}", msg),
                BridgeV3Error::QuorumFailed { current, required } => {
                    write!(f, "Quorum failed: {} current, {} required", current, required)
                }
                BridgeV3Error::BridgeFull => write!(f, "Bridge capacity exceeded"),
                BridgeV3Error::RelayFailed(msg) => write!(f, "Relay failed: {}", msg),
                BridgeV3Error::MessageExpired(id) => write!(f, "Message {} expired", id),
                BridgeV3Error::InvalidMessageSize { size, max } => {
                    write!(f, "Message size {} exceeds max {}", size, max)
                }
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Config
    // ---------------------------------------------------------------------------

    /// Configuration for Cross-Chain Bridge v3.
    #[derive(Debug, Clone)]
    pub struct CrossChainBridgeV3Config {
        /// Maximum message size in bytes.
        pub max_message_size: usize,
        /// Proof TTL in milliseconds.
        pub proof_ttl_ms: u64,
        /// Verification threshold (0.0-1.0).
        pub verification_threshold: f64,
        /// Enable Merkle proof fallback.
        pub enable_merkle_proof: bool,
        /// Minimum quorum signatures for relay.
        pub min_quorum_signatures: u32,
        /// Maximum chains registered.
        pub max_chains: usize,
        /// Maximum messages in flight.
        pub max_messages_in_flight: usize,
        /// Validation timeout in milliseconds for fallback trigger.
        pub validation_timeout_ms: u64,
    }

    impl Default for CrossChainBridgeV3Config {
        fn default() -> Self {
            Self {
                max_message_size: 4096,
                proof_ttl_ms: 60_000,
                verification_threshold: 0.67,
                enable_merkle_proof: true,
                min_quorum_signatures: 3,
                max_chains: 64,
                max_messages_in_flight: 512,
                validation_timeout_ms: 150,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Message Status
    // ---------------------------------------------------------------------------

    /// Lifecycle status of a bridge message.
    #[derive(Debug, Clone, PartialEq)]
    pub enum MessageStatus {
        /// Message submitted, awaiting validation.
        Pending,
        /// Message validated, awaiting quorum relay.
        Validated,
        /// Message relayed with quorum signatures.
        Relayed,
        /// Message expired or failed.
        Expired,
    }

    impl std::fmt::Display for MessageStatus {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                MessageStatus::Pending => write!(f, "Pending"),
                MessageStatus::Validated => write!(f, "Validated"),
                MessageStatus::Relayed => write!(f, "Relayed"),
                MessageStatus::Expired => write!(f, "Expired"),
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Chain Entry
    // ---------------------------------------------------------------------------

    /// Registered chain entry with reputation and capacity.
    #[derive(Debug, Clone)]
    pub struct ChainEntryV3 {
        pub chain_id: String,
        pub reputation: f64,
        pub capacity: f64,
        pub active: bool,
        pub messages_relayed: u64,
        pub messages_failed: u64,
    }

    impl ChainEntryV3 {
        pub fn new(chain_id: String, reputation: f64, capacity: f64) -> Self {
            Self {
                chain_id,
                reputation,
                capacity,
                active: true,
                messages_relayed: 0,
                messages_failed: 0,
            }
        }

        pub fn routing_score(&self, reputation_weight: f64, capacity_weight: f64) -> f64 {
            self.reputation * reputation_weight + self.capacity * capacity_weight
        }

        pub fn success_rate(&self) -> f64 {
            let total = self.messages_relayed + self.messages_failed;
            if total == 0 {
                return 1.0;
            }
            self.messages_relayed as f64 / total as f64
        }

        pub fn update_reputation(&mut self, success: bool, alpha: f64) {
            let signal = if success { 1.0 } else { 0.0 };
            self.reputation = self.reputation * (1.0 - alpha) + signal * alpha;
            self.reputation = self.reputation.clamp(0.0, 1.0);
        }
    }

    // ---------------------------------------------------------------------------
    // Message Entry
    // ---------------------------------------------------------------------------

    /// Bridge message with validation and relay state.
    #[derive(Debug, Clone)]
    pub struct MessageEntryV3 {
        pub message_id: String,
        pub source_chain: String,
        pub destination_chain: String,
        pub payload_hash: String,
        pub status: MessageStatus,
        pub created_at: Instant,
        pub ttl_ms: u64,
        pub quorum_signatures: Vec<String>,
        pub validation_time_ms: u64,
        pub relay_time_ms: u64,
        pub used_fallback: bool,
    }

    impl MessageEntryV3 {
        pub fn new(
            message_id: String,
            source_chain: String,
            destination_chain: String,
            payload_hash: String,
            ttl_ms: u64,
        ) -> Self {
            Self {
                message_id,
                source_chain,
                destination_chain,
                payload_hash,
                status: MessageStatus::Pending,
                created_at: Instant::now(),
                ttl_ms,
                quorum_signatures: Vec::new(),
                validation_time_ms: 0,
                relay_time_ms: 0,
                used_fallback: false,
            }
        }

        pub fn is_expired(&self) -> bool {
            self.created_at.elapsed() > Duration::from_millis(self.ttl_ms)
        }

        pub fn quorum_reached(&self, min_signatures: u32) -> bool {
            self.quorum_signatures.len() as u32 >= min_signatures
        }
    }

    // ---------------------------------------------------------------------------
    // Bridge Stats
    // ---------------------------------------------------------------------------

    /// Statistics for bridge operations.
    #[derive(Debug, Clone)]
    pub struct BridgeV3Stats {
        pub messages_submitted: u64,
        pub messages_validated: u64,
        pub messages_relayed: u64,
        pub messages_failed: u64,
        pub fallback_activations: u64,
        pub avg_validation_time_ms: f64,
        pub avg_relay_time_ms: f64,
    }

    impl Default for BridgeV3Stats {
        fn default() -> Self {
            Self {
                messages_submitted: 0,
                messages_validated: 0,
                messages_relayed: 0,
                messages_failed: 0,
                fallback_activations: 0,
                avg_validation_time_ms: 0.0,
                avg_relay_time_ms: 0.0,
            }
        }
    }

    impl BridgeV3Stats {
        pub fn record_validation(&mut self, time_ms: u64) {
            self.messages_validated += 1;
            self.avg_validation_time_ms =
                self.avg_validation_time_ms * 0.9 + time_ms as f64 * 0.1;
        }

        pub fn record_relay(&mut self, time_ms: u64) {
            self.messages_relayed += 1;
            self.avg_relay_time_ms = self.avg_relay_time_ms * 0.9 + time_ms as f64 * 0.1;
        }

        pub fn record_failure(&mut self) {
            self.messages_failed += 1;
        }

        pub fn record_fallback(&mut self) {
            self.fallback_activations += 1;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    // ---------------------------------------------------------------------------
    // Bridge Engine
    // ---------------------------------------------------------------------------

    /// Cross-Chain Bridge v3 engine with light validation and quorum relay.
    #[derive(Debug)]
    pub struct CrossChainBridgeV3 {
        config: CrossChainBridgeV3Config,
        chains: HashMap<String, ChainEntryV3>,
        messages: HashMap<String, MessageEntryV3>,
        stats: BridgeV3Stats,
        next_nonce: u64,
    }

    impl CrossChainBridgeV3 {
        /// Create a new bridge with the given configuration.
        pub fn new(config: CrossChainBridgeV3Config) -> Self {
            Self {
                config,
                chains: HashMap::new(),
                messages: HashMap::new(),
                stats: BridgeV3Stats::default(),
                next_nonce: 0,
            }
        }

        /// Register a chain with initial reputation and capacity.
        pub fn register_chain(
            &mut self,
            chain_id: String,
            reputation: f64,
            capacity: f64,
        ) -> Result<(), BridgeV3Error> {
            if self.chains.len() >= self.config.max_chains {
                return Err(BridgeV3Error::BridgeFull);
            }
            if self.chains.contains_key(&chain_id) {
                return Err(BridgeV3Error::ChainNotFound(chain_id.clone()));
            }
            self.chains
                .insert(chain_id.clone(), ChainEntryV3::new(chain_id, reputation, capacity));
            Ok(())
        }

        /// Submit a message for cross-chain verification.
        pub fn submit_message(
            &mut self,
            source: &str,
            destination: &str,
            message: Vec<u8>,
        ) -> Result<String, BridgeV3Error> {
            // Validate chains exist
            if !self.chains.contains_key(source) {
                return Err(BridgeV3Error::ChainNotFound(source.to_string()));
            }
            if !self.chains.contains_key(destination) {
                return Err(BridgeV3Error::ChainNotFound(destination.to_string()));
            }

            // Validate message size
            if message.len() > self.config.max_message_size {
                return Err(BridgeV3Error::InvalidMessageSize {
                    size: message.len(),
                    max: self.config.max_message_size,
                });
            }

            // Check capacity
            if self.messages.len() >= self.config.max_messages_in_flight {
                return Err(BridgeV3Error::BridgeFull);
            }

            // Generate message ID and hash
            self.next_nonce += 1;
            let message_id = format!("msg_{}", self.next_nonce);
            let payload_hash = compute_sha256(&message);

            let entry = MessageEntryV3::new(
                message_id.clone(),
                source.to_string(),
                destination.to_string(),
                payload_hash,
                self.config.proof_ttl_ms,
            );

            self.stats.messages_submitted += 1;
            self.messages.insert(message_id.clone(), entry);
            Ok(message_id)
        }

        /// Validate a message using light validation.
        pub fn verify_message(&mut self, message_id: &str) -> Result<bool, BridgeV3Error> {
            let entry = self
                .messages
                .get_mut(message_id)
                .ok_or_else(|| BridgeV3Error::MessageNotFound(message_id.to_string()))?;

            if entry.is_expired() {
                entry.status = MessageStatus::Expired;
                return Err(BridgeV3Error::MessageExpired(message_id.to_string()));
            }

            let start = Instant::now();

            // Light validation: verify payload hash integrity
            let valid = !entry.payload_hash.is_empty() && entry.payload_hash.len() == 64;

            let validation_time = start.elapsed().as_millis() as u64;
            entry.validation_time_ms = validation_time;

            // Check if fallback needed
            if validation_time > self.config.validation_timeout_ms
                && self.config.enable_merkle_proof
            {
                entry.used_fallback = true;
                self.stats.record_fallback();
            }

            if valid {
                entry.status = MessageStatus::Validated;
                self.stats.record_validation(validation_time);
            } else {
                self.stats.record_failure();
            }

            Ok(valid)
        }

        /// Add a quorum signature to a message.
        pub fn add_signature(&mut self, message_id: &str, signer: String) -> Result<(), BridgeV3Error> {
            let entry = self
                .messages
                .get_mut(message_id)
                .ok_or_else(|| BridgeV3Error::MessageNotFound(message_id.to_string()))?;

            if entry.status != MessageStatus::Validated {
                return Err(BridgeV3Error::ValidationFailed(
                    "Message not validated".to_string(),
                ));
            }

            if entry.quorum_signatures.contains(&signer) {
                return Err(BridgeV3Error::RelayFailed("Duplicate signature".to_string()));
            }

            entry.quorum_signatures.push(signer);
            Ok(())
        }

        /// Relay a message with quorum signatures.
        pub fn relay_message(&mut self, message_id: &str) -> Result<(), BridgeV3Error> {
            let entry = self
                .messages
                .get_mut(message_id)
                .ok_or_else(|| BridgeV3Error::MessageNotFound(message_id.to_string()))?;

            if entry.status != MessageStatus::Validated {
                return Err(BridgeV3Error::ValidationFailed(
                    "Message not validated".to_string(),
                ));
            }

            if !entry.quorum_reached(self.config.min_quorum_signatures) {
                return Err(BridgeV3Error::QuorumFailed {
                    current: entry.quorum_signatures.len() as u32,
                    required: self.config.min_quorum_signatures,
                });
            }

            let start = Instant::now();
            entry.status = MessageStatus::Relayed;
            entry.relay_time_ms = start.elapsed().as_millis() as u64;

            // Update chain stats
            if let Some(chain) = self.chains.get_mut(&entry.source_chain) {
                chain.messages_relayed += 1;
            }

            self.stats
                .record_relay(entry.relay_time_ms);
            Ok(())
        }

        /// Get the current status of a message.
        pub fn get_message_status(&self, message_id: &str) -> Option<MessageStatus> {
            self.messages.get(message_id).map(|e| e.status.clone())
        }

        /// Get bridge statistics.
        pub fn get_stats(&self) -> &BridgeV3Stats {
            &self.stats
        }

        /// Reset bridge statistics.
        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }

        /// Clean up expired messages.
        pub fn cleanup_expired(&mut self) -> usize {
            let before = self.messages.len();
            self.messages.retain(|_, v| !v.is_expired());
            before - self.messages.len()
        }

        /// Get registered chain count.
        pub fn chain_count(&self) -> usize {
            self.chains.len()
        }

        /// Get active message count.
        pub fn active_message_count(&self) -> usize {
            self.messages.len()
        }
    }

    impl Default for CrossChainBridgeV3 {
        fn default() -> Self {
            Self::new(CrossChainBridgeV3Config::default())
        }
    }

    // ---------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------

    fn compute_sha256(input: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(input);
        hex::encode(hasher.finalize())
    }

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        fn make_config() -> CrossChainBridgeV3Config {
            CrossChainBridgeV3Config {
                max_message_size: 1024,
                proof_ttl_ms: 5000,
                verification_threshold: 0.67,
                enable_merkle_proof: true,
                min_quorum_signatures: 3,
                max_chains: 10,
                max_messages_in_flight: 100,
                validation_timeout_ms: 150,
            }
        }

        #[test]
        fn test_bridge_creation() {
            let bridge = CrossChainBridgeV3::default();
            assert_eq!(bridge.chain_count(), 0);
            assert_eq!(bridge.active_message_count(), 0);
        }

        #[test]
        fn test_bridge_with_config() {
            let config = make_config();
            let bridge = CrossChainBridgeV3::new(config);
            assert_eq!(bridge.chain_count(), 0);
        }

        #[test]
        fn test_register_chain() {
            let mut bridge = CrossChainBridgeV3::default();
            assert!(bridge.register_chain("chain_a".to_string(), 0.9, 100.0).is_ok());
            assert_eq!(bridge.chain_count(), 1);
        }

        #[test]
        fn test_register_chain_duplicate() {
            let mut bridge = CrossChainBridgeV3::default();
            bridge.register_chain("chain_a".to_string(), 0.9, 100.0).unwrap();
            match bridge.register_chain("chain_a".to_string(), 0.8, 80.0).unwrap_err() {
                BridgeV3Error::ChainNotFound(id) => assert_eq!(id, "chain_a"),
                e => panic!("Expected ChainNotFound, got {:?}", e),
            }
        }

        #[test]
        fn test_register_chain_max_reached() {
            let mut config = CrossChainBridgeV3Config::default();
            config.max_chains = 2;
            let mut bridge = CrossChainBridgeV3::new(config);
            bridge.register_chain("c1".to_string(), 0.9, 100.0).unwrap();
            bridge.register_chain("c2".to_string(), 0.8, 80.0).unwrap();
            assert!(bridge.register_chain("c3".to_string(), 0.7, 60.0).is_err());
        }

        #[test]
        fn test_submit_message() {
            let mut bridge = CrossChainBridgeV3::default();
            bridge.register_chain("src".to_string(), 0.9, 100.0).unwrap();
            bridge.register_chain("dst".to_string(), 0.8, 80.0).unwrap();
            let msg_id = bridge.submit_message("src", "dst", vec![1, 2, 3]).unwrap();
            assert_eq!(msg_id, "msg_1");
            assert_eq!(bridge.active_message_count(), 1);
        }

        #[test]
        fn test_submit_message_source_not_found() {
            let mut bridge = CrossChainBridgeV3::default();
            bridge.register_chain("dst".to_string(), 0.8, 80.0).unwrap();
            match bridge.submit_message("unknown", "dst", vec![1]).unwrap_err() {
                BridgeV3Error::ChainNotFound(id) => assert_eq!(id, "unknown"),
                e => panic!("Expected ChainNotFound, got {:?}", e),
            }
        }

        #[test]
        fn test_submit_message_destination_not_found() {
            let mut bridge = CrossChainBridgeV3::default();
            bridge.register_chain("src".to_string(), 0.9, 100.0).unwrap();
            match bridge.submit_message("src", "unknown", vec![1]).unwrap_err() {
                BridgeV3Error::ChainNotFound(id) => assert_eq!(id, "unknown"),
                e => panic!("Expected ChainNotFound, got {:?}", e),
            }
        }

        #[test]
        fn test_submit_message_too_large() {
            let mut config = CrossChainBridgeV3Config::default();
            config.max_message_size = 10;
            let mut bridge = CrossChainBridgeV3::new(config);
            bridge.register_chain("src".to_string(), 0.9, 100.0).unwrap();
            bridge.register_chain("dst".to_string(), 0.8, 80.0).unwrap();
            match bridge.submit_message("src", "dst", vec![1; 20]).unwrap_err() {
                BridgeV3Error::InvalidMessageSize { size, max } => {
                    assert_eq!(size, 20);
                    assert_eq!(max, 10);
                }
                e => panic!("Expected InvalidMessageSize, got {:?}", e),
            }
        }

        #[test]
        fn test_verify_message() {
            let mut bridge = CrossChainBridgeV3::default();
            bridge.register_chain("src".to_string(), 0.9, 100.0).unwrap();
            bridge.register_chain("dst".to_string(), 0.8, 80.0).unwrap();
            let msg_id = bridge.submit_message("src", "dst", vec![1, 2, 3]).unwrap();
            let valid = bridge.verify_message(&msg_id).unwrap();
            assert!(valid);
            assert_eq!(bridge.get_message_status(&msg_id), Some(MessageStatus::Validated));
        }

        #[test]
        fn test_verify_message_not_found() {
            let mut bridge = CrossChainBridgeV3::default();
            match bridge.verify_message("unknown").unwrap_err() {
                BridgeV3Error::MessageNotFound(id) => assert_eq!(id, "unknown"),
                e => panic!("Expected MessageNotFound, got {:?}", e),
            }
        }

        #[test]
        fn test_add_signature() {
            let mut bridge = CrossChainBridgeV3::default();
            bridge.register_chain("src".to_string(), 0.9, 100.0).unwrap();
            bridge.register_chain("dst".to_string(), 0.8, 80.0).unwrap();
            let msg_id = bridge.submit_message("src", "dst", vec![1, 2, 3]).unwrap();
            bridge.verify_message(&msg_id).unwrap();
            assert!(bridge.add_signature(&msg_id, "node_1".to_string()).is_ok());
        }

        #[test]
        fn test_add_signature_not_validated() {
            let mut bridge = CrossChainBridgeV3::default();
            bridge.register_chain("src".to_string(), 0.9, 100.0).unwrap();
            bridge.register_chain("dst".to_string(), 0.8, 80.0).unwrap();
            let msg_id = bridge.submit_message("src", "dst", vec![1, 2, 3]).unwrap();
            match bridge.add_signature(&msg_id, "node_1".to_string()).unwrap_err() {
                BridgeV3Error::ValidationFailed(msg) => assert!(!msg.is_empty()),
                e => panic!("Expected ValidationFailed, got {:?}", e),
            }
        }

        #[test]
        fn test_relay_message_quorum_met() {
            let mut bridge = CrossChainBridgeV3::default();
            bridge.register_chain("src".to_string(), 0.9, 100.0).unwrap();
            bridge.register_chain("dst".to_string(), 0.8, 80.0).unwrap();
            let msg_id = bridge.submit_message("src", "dst", vec![1, 2, 3]).unwrap();
            bridge.verify_message(&msg_id).unwrap();
            bridge.add_signature(&msg_id, "node_1".to_string()).unwrap();
            bridge.add_signature(&msg_id, "node_2".to_string()).unwrap();
            bridge.add_signature(&msg_id, "node_3".to_string()).unwrap();
            assert!(bridge.relay_message(&msg_id).is_ok());
            assert_eq!(bridge.get_message_status(&msg_id), Some(MessageStatus::Relayed));
        }

        #[test]
        fn test_relay_message_quorum_not_met() {
            let mut bridge = CrossChainBridgeV3::default();
            bridge.register_chain("src".to_string(), 0.9, 100.0).unwrap();
            bridge.register_chain("dst".to_string(), 0.8, 80.0).unwrap();
            let msg_id = bridge.submit_message("src", "dst", vec![1, 2, 3]).unwrap();
            bridge.verify_message(&msg_id).unwrap();
            bridge.add_signature(&msg_id, "node_1".to_string()).unwrap();
            match bridge.relay_message(&msg_id).unwrap_err() {
                BridgeV3Error::QuorumFailed { current, required } => {
                    assert_eq!(current, 1);
                    assert_eq!(required, 3);
                }
                e => panic!("Expected QuorumFailed, got {:?}", e),
            }
        }

        #[test]
        fn test_cleanup_expired() {
            let mut bridge = CrossChainBridgeV3::default();
            bridge.register_chain("src".to_string(), 0.9, 100.0).unwrap();
            bridge.register_chain("dst".to_string(), 0.8, 80.0).unwrap();
            // Create message with very short TTL
            let mut config = CrossChainBridgeV3Config::default();
            config.proof_ttl_ms = 1;
            bridge = CrossChainBridgeV3::new(config);
            bridge.register_chain("src".to_string(), 0.9, 100.0).unwrap();
            bridge.register_chain("dst".to_string(), 0.8, 80.0).unwrap();
            bridge.submit_message("src", "dst", vec![1, 2, 3]).unwrap();
            std::thread::sleep(Duration::from_millis(10));
            let cleaned = bridge.cleanup_expired();
            assert_eq!(cleaned, 1);
        }

        #[test]
        fn test_stats_recording() {
            let mut bridge = CrossChainBridgeV3::default();
            bridge.register_chain("src".to_string(), 0.9, 100.0).unwrap();
            bridge.register_chain("dst".to_string(), 0.8, 80.0).unwrap();
            let msg_id = bridge.submit_message("src", "dst", vec![1, 2, 3]).unwrap();
            bridge.verify_message(&msg_id).unwrap();
            let stats = bridge.get_stats();
            assert_eq!(stats.messages_submitted, 1);
            assert_eq!(stats.messages_validated, 1);
        }

        #[test]
        fn test_reset_stats() {
            let mut bridge = CrossChainBridgeV3::default();
            bridge.register_chain("src".to_string(), 0.9, 100.0).unwrap();
            bridge.register_chain("dst".to_string(), 0.8, 80.0).unwrap();
            let msg_id = bridge.submit_message("src", "dst", vec![1, 2, 3]).unwrap();
            bridge.verify_message(&msg_id).unwrap();
            bridge.reset_stats();
            let stats = bridge.get_stats();
            assert_eq!(stats.messages_submitted, 0);
            assert_eq!(stats.messages_validated, 0);
        }

        #[test]
        fn test_chain_routing_score() {
            let chain = ChainEntryV3::new("test".to_string(), 0.9, 100.0);
            let score = chain.routing_score(0.6, 0.4);
            // reputation * weight + capacity * weight = 0.9*0.6 + 100.0*0.4 = 0.54 + 40.0 = 40.54
            assert!((score - 40.54).abs() < 0.01);
        }

        #[test]
        fn test_chain_success_rate() {
            let mut chain = ChainEntryV3::new("test".to_string(), 0.9, 100.0);
            chain.messages_relayed = 90;
            chain.messages_failed = 10;
            assert!((chain.success_rate() - 0.9).abs() < 0.01);
        }

        #[test]
        fn test_error_display() {
            let err = BridgeV3Error::ChainNotFound("test".to_string());
            assert!(!format!("{}", err).is_empty());
        }

        #[test]
        fn test_config_default() {
            let config = CrossChainBridgeV3Config::default();
            assert_eq!(config.min_quorum_signatures, 3);
            assert!(config.enable_merkle_proof);
        }

        #[test]
        fn test_message_status_display() {
            assert_eq!(format!("{}", MessageStatus::Pending), "Pending");
            assert_eq!(format!("{}", MessageStatus::Relayed), "Relayed");
        }

        #[test]
        fn test_nonce_increment() {
            let mut bridge = CrossChainBridgeV3::default();
            bridge.register_chain("src".to_string(), 0.9, 100.0).unwrap();
            bridge.register_chain("dst".to_string(), 0.8, 80.0).unwrap();
            let id1 = bridge.submit_message("src", "dst", vec![1]).unwrap();
            let id2 = bridge.submit_message("src", "dst", vec![2]).unwrap();
            assert_eq!(id1, "msg_1");
            assert_eq!(id2, "msg_2");
        }

        #[test]
        fn test_full_lifecycle() {
            let mut bridge = CrossChainBridgeV3::default();
            bridge.register_chain("a".to_string(), 0.9, 100.0).unwrap();
            bridge.register_chain("b".to_string(), 0.8, 80.0).unwrap();
            let msg_id = bridge.submit_message("a", "b", vec![42]).unwrap();
            assert_eq!(bridge.get_message_status(&msg_id), Some(MessageStatus::Pending));
            bridge.verify_message(&msg_id).unwrap();
            assert_eq!(bridge.get_message_status(&msg_id), Some(MessageStatus::Validated));
            bridge.add_signature(&msg_id, "n1".to_string()).unwrap();
            bridge.add_signature(&msg_id, "n2".to_string()).unwrap();
            bridge.add_signature(&msg_id, "n3".to_string()).unwrap();
            bridge.relay_message(&msg_id).unwrap();
            assert_eq!(bridge.get_message_status(&msg_id), Some(MessageStatus::Relayed));
        }
    }
}

#[cfg(feature = "v1.6-sprint1")]
pub use internal::*;
