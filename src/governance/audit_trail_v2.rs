//! Audit Trail v2 — Cryptographic audit trail with immutable event logging and chain verification.
//!
//! Provides tamper-evident audit logging with SHA-256 hash chaining,
//! event categorization, and verification capabilities.

#[cfg(feature = "v1.5-sprint1")]
mod internal {
    use std::collections::HashMap;
    use std::collections::VecDeque;

    // ---------------------------------------------------------------------
    // Public types
    // ---------------------------------------------------------------------

    /// Error types for audit trail operations.
    #[derive(Debug, Clone, PartialEq)]
    pub enum AuditError {
        /// Entry not found for the given ID.
        EntryNotFound,
        /// Hash chain verification failed.
        ChainVerificationFailed,
        /// Maximum entries reached.
        MaxEntriesReached,
        /// Payload exceeds maximum size.
        PayloadTooLarge,
        /// Duplicate entry ID detected.
        DuplicateEntry,
        /// Tampering detected in the chain.
        TamperingDetected(String),
        /// Invalid entry type.
        InvalidEntryType,
    }

    impl std::fmt::Display for AuditError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                AuditError::EntryNotFound => write!(f, "Audit entry not found"),
                AuditError::ChainVerificationFailed => {
                    write!(f, "Chain verification failed")
                }
                AuditError::MaxEntriesReached => write!(f, "Max entries reached"),
                AuditError::PayloadTooLarge => write!(f, "Payload exceeds maximum size"),
                AuditError::DuplicateEntry => write!(f, "Duplicate entry ID"),
                AuditError::TamperingDetected(msg) => {
                    write!(f, "Tampering detected: {}", msg)
                }
                AuditError::InvalidEntryType => write!(f, "Invalid entry type"),
            }
        }
    }

    /// Categories for audit events.
    #[derive(Debug, Clone, PartialEq)]
    pub enum AuditCategory {
        /// Governance-related events.
        Governance,
        /// Security-related events.
        Security,
        /// System operations.
        System,
        /// Configuration changes.
        Configuration,
        /// Access control events.
        AccessControl,
        /// Custom category.
        Custom(String),
    }

    impl std::fmt::Display for AuditCategory {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                AuditCategory::Governance => write!(f, "Governance"),
                AuditCategory::Security => write!(f, "Security"),
                AuditCategory::System => write!(f, "System"),
                AuditCategory::Configuration => write!(f, "Configuration"),
                AuditCategory::AccessControl => write!(f, "AccessControl"),
                AuditCategory::Custom(msg) => write!(f, "Custom: {}", msg),
            }
        }
    }

    /// Severity levels for audit entries.
    #[derive(Debug, Clone, PartialEq)]
    pub enum AuditSeverity {
        /// Informational event.
        Info,
        /// Warning event.
        Warning,
        /// Critical event requiring attention.
        Critical,
    }

    impl std::fmt::Display for AuditSeverity {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                AuditSeverity::Info => write!(f, "Info"),
                AuditSeverity::Warning => write!(f, "Warning"),
                AuditSeverity::Critical => write!(f, "Critical"),
            }
        }
    }

    /// Configuration for the audit trail engine.
    #[derive(Debug, Clone)]
    pub struct AuditTrailConfig {
        /// Maximum number of entries to retain.
        pub max_entries: usize,
        /// Enable hash chain verification.
        pub chain_verification: bool,
        /// Enable signature tracking.
        pub signature_tracking: bool,
        /// Maximum payload size in bytes.
        pub max_payload_bytes: usize,
        /// Enable automatic compaction.
        pub auto_compact: bool,
        /// Compaction threshold ratio (0.0 - 1.0).
        pub compaction_threshold: f64,
    }

    impl Default for AuditTrailConfig {
        fn default() -> Self {
            Self {
                max_entries: 10000,
                chain_verification: true,
                signature_tracking: true,
                max_payload_bytes: 4096,
                auto_compact: false,
                compaction_threshold: 0.8,
            }
        }
    }

    /// Single audit log entry with cryptographic linking.
    #[derive(Debug, Clone, PartialEq)]
    pub struct AuditEntry {
        /// Unique entry identifier.
        pub entry_id: String,
        /// Sequence number in the chain.
        pub sequence: u64,
        /// Event category.
        pub category: AuditCategory,
        /// Severity level.
        pub severity: AuditSeverity,
        /// Actor or system that triggered the event.
        pub actor_id: String,
        /// Event payload/description.
        pub payload: String,
        /// SHA-256 hash of this entry.
        pub hash: String,
        /// Hash of the previous entry in the chain.
        pub previous_hash: String,
        /// Cryptographic signature (simulated).
        pub signature: Vec<u8>,
        /// Timestamp in milliseconds.
        pub timestamp_ms: u64,
        /// Optional metadata.
        pub metadata: HashMap<String, String>,
    }

    /// Metrics for audit trail operations.
    #[derive(Debug, Clone, Default)]
    pub struct AuditMetrics {
        /// Total entries recorded.
        pub total_entries: usize,
        /// Entries by category.
        pub entries_by_category: HashMap<String, usize>,
        /// Entries by severity.
        pub entries_by_severity: HashMap<String, usize>,
        /// Chain verification count.
        pub verification_count: usize,
        /// Chain verification failures.
        pub verification_failures: usize,
        /// Compaction count.
        pub compaction_count: usize,
        /// Current chain length.
        pub chain_length: usize,
    }

    impl AuditMetrics {
        /// Record a new entry in the metrics.
        pub fn record_entry(&mut self, category: &AuditCategory, severity: &AuditSeverity) {
            self.total_entries += 1;
            self.chain_length += 1;

            let cat_key = format!("{}", category);
            let count = self.entries_by_category.entry(cat_key).or_insert(0);
            *count += 1;

            let sev_key = format!("{}", severity);
            let count = self.entries_by_severity.entry(sev_key).or_insert(0);
            *count += 1;
        }

        /// Record a verification attempt.
        pub fn record_verification(&mut self, success: bool) {
            self.verification_count += 1;
            if !success {
                self.verification_failures += 1;
            }
        }

        /// Record a compaction event.
        pub fn record_compaction(&mut self) {
            self.compaction_count += 1;
        }
    }

    /// Audit Trail Engine — Manages immutable audit log with cryptographic chain.
    pub struct AuditTrailV2 {
        config: AuditTrailConfig,
        entries: VecDeque<AuditEntry>,
        index: HashMap<String, usize>,
        metrics: AuditMetrics,
        next_sequence: u64,
    }

    impl AuditTrailV2 {
        /// Create a new audit trail engine with the given configuration.
        pub fn new(config: AuditTrailConfig) -> Self {
            Self {
                config,
                entries: VecDeque::new(),
                index: HashMap::new(),
                metrics: AuditMetrics::default(),
                next_sequence: 1,
            }
        }

        /// Append a new audit entry to the chain.
        pub fn append_entry(
            &mut self,
            entry_id: String,
            category: AuditCategory,
            severity: AuditSeverity,
            actor_id: String,
            payload: String,
            timestamp_ms: u64,
        ) -> Result<AuditEntry, AuditError> {
            if self.index.contains_key(&entry_id) {
                return Err(AuditError::DuplicateEntry);
            }
            if payload.len() > self.config.max_payload_bytes {
                return Err(AuditError::PayloadTooLarge);
            }
            if self.entries.len() >= self.config.max_entries {
                return Err(AuditError::MaxEntriesReached);
            }

            let previous_hash = self.get_last_hash();
            let hash = compute_hash(
                &entry_id,
                self.next_sequence,
                &payload,
                &previous_hash,
                timestamp_ms,
            );

            let signature = if self.config.signature_tracking {
                compute_signature(&entry_id, &hash)
            } else {
                Vec::new()
            };

            let entry = AuditEntry {
                entry_id: entry_id.clone(),
                sequence: self.next_sequence,
                category: category.clone(),
                severity: severity.clone(),
                actor_id,
                payload,
                hash: hash.clone(),
                previous_hash,
                signature,
                timestamp_ms,
                metadata: HashMap::new(),
            };

            self.index.insert(entry_id, self.entries.len());
            self.entries.push_back(entry.clone());
            self.next_sequence += 1;
            self.metrics.record_entry(&category, &severity);

            Ok(entry)
        }

        /// Retrieve an entry by ID.
        pub fn get_entry(&self, entry_id: &str) -> Option<&AuditEntry> {
            let idx = self.index.get(entry_id).copied()?;
            self.entries.get(idx)
        }

        /// Verify the integrity of the entire hash chain.
        pub fn verify_chain(&mut self) -> Result<(), AuditError> {
            if !self.config.chain_verification {
                self.metrics.record_verification(true);
                return Ok(());
            }

            let mut prev_hash = "0".repeat(64);
            for (i, entry) in self.entries.iter().enumerate() {
                if entry.previous_hash != prev_hash {
                    self.metrics.record_verification(false);
                    return Err(AuditError::TamperingDetected(format!(
                        "Chain break at entry {} (index {})",
                        entry.entry_id, i
                    )));
                }

                let expected_hash = compute_hash(
                    &entry.entry_id,
                    entry.sequence,
                    &entry.payload,
                    &entry.previous_hash,
                    entry.timestamp_ms,
                );
                if entry.hash != expected_hash {
                    self.metrics.record_verification(false);
                    return Err(AuditError::TamperingDetected(format!(
                        "Hash mismatch at entry {}",
                        entry.entry_id
                    )));
                }

                prev_hash = entry.hash.clone();
            }

            self.metrics.record_verification(true);
            Ok(())
        }

        /// Get entries filtered by category.
        pub fn get_entries_by_category(&self, category: &AuditCategory) -> Vec<&AuditEntry> {
            self.entries
                .iter()
                .filter(|e| &e.category == category)
                .collect()
        }

        /// Get entries filtered by severity.
        pub fn get_entries_by_severity(&self, severity: &AuditSeverity) -> Vec<&AuditEntry> {
            self.entries
                .iter()
                .filter(|e| &e.severity == severity)
                .collect()
        }

        /// Get entries filtered by actor.
        pub fn get_entries_by_actor(&self, actor_id: &str) -> Vec<&AuditEntry> {
            self.entries
                .iter()
                .filter(|e| e.actor_id == actor_id)
                .collect()
        }

        /// Get the most recent entries up to a count.
        pub fn get_recent_entries(&self, count: usize) -> Vec<&AuditEntry> {
            self.entries.iter().rev().take(count).collect()
        }

        /// Add metadata to an existing entry.
        pub fn add_metadata(
            &mut self,
            entry_id: &str,
            key: String,
            value: String,
        ) -> Result<(), AuditError> {
            let idx = *self.index.get(entry_id).ok_or(AuditError::EntryNotFound)?;
            let entry = self.entries.get_mut(idx).ok_or(AuditError::EntryNotFound)?;
            entry.metadata.insert(key, value);
            Ok(())
        }

        /// Compact old entries when approaching capacity threshold.
        pub fn compact(&mut self, retain_count: usize) -> usize {
            if self.entries.len() <= retain_count {
                return 0;
            }

            let removed = self.entries.len() - retain_count;
            self.entries.drain(..retain_count);
            self.index.clear();
            for (i, entry) in self.entries.iter().enumerate() {
                self.index.insert(entry.entry_id.clone(), i);
            }
            self.metrics.chain_length = self.entries.len();
            self.metrics.record_compaction();

            removed
        }

        /// Check if auto-compaction should trigger.
        pub fn should_compact(&self) -> bool {
            if !self.config.auto_compact {
                return false;
            }
            let ratio = self.entries.len() as f64 / self.config.max_entries as f64;
            ratio >= self.config.compaction_threshold
        }

        /// Get current metrics.
        pub fn metrics(&self) -> &AuditMetrics {
            &self.metrics
        }

        /// Reset metrics to default.
        pub fn reset_metrics(&mut self) {
            self.metrics = AuditMetrics::default();
        }

        /// Get the current chain length.
        pub fn chain_length(&self) -> usize {
            self.entries.len()
        }

        /// Get the latest entry.
        pub fn latest_entry(&self) -> Option<&AuditEntry> {
            self.entries.back()
        }

        /// Get the first entry in the chain.
        pub fn first_entry(&self) -> Option<&AuditEntry> {
            self.entries.front()
        }

        /// Clear all entries (emergency reset).
        pub fn clear(&mut self) {
            self.entries.clear();
            self.index.clear();
            self.metrics.chain_length = 0;
        }
    }

    impl Default for AuditTrailV2 {
        fn default() -> Self {
            Self::new(AuditTrailConfig::default())
        }
    }

    impl AuditTrailV2 {
        /// Private helper to get the hash of the last entry.
        fn get_last_hash(&self) -> String {
            match self.entries.back() {
                Some(entry) => entry.hash.clone(),
                None => "0".repeat(64),
            }
        }
    }

    // ---------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------

    fn compute_hash(
        entry_id: &str,
        sequence: u64,
        payload: &str,
        previous_hash: &str,
        timestamp_ms: u64,
    ) -> String {
        let data = format!(
            "{}:{}:{}:{}:{}",
            entry_id, sequence, payload, previous_hash, timestamp_ms
        );
        compute_sha256(&data)
    }

    fn compute_sha256(data: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    fn compute_signature(entry_id: &str, hash: &str) -> Vec<u8> {
        let data = format!("{}:{}", entry_id, hash);
        compute_sha256(&data).as_bytes().to_vec()
    }

    // ---------------------------------------------------------------------
    // Unit tests
    // ---------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        fn current_time_ms() -> u64 {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        }

        #[test]
        fn test_engine_creation() {
            let engine = AuditTrailV2::default();
            assert_eq!(engine.chain_length(), 0);
        }

        #[test]
        fn test_engine_with_config() {
            let config = AuditTrailConfig {
                max_entries: 5000,
                chain_verification: false,
                ..Default::default()
            };
            let engine = AuditTrailV2::new(config);
            assert_eq!(engine.chain_length(), 0);
        }

        #[test]
        fn test_append_entry() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            let entry = engine
                .append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Test event".to_string(),
                    time,
                )
                .unwrap();
            assert_eq!(entry.entry_id, "e1");
            assert_eq!(entry.sequence, 1);
            assert_eq!(engine.chain_length(), 1);
        }

        #[test]
        fn test_append_entry_duplicate() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            engine
                .append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Test".to_string(),
                    time,
                )
                .unwrap();
            assert_eq!(
                engine.append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Test".to_string(),
                    time,
                ),
                Err(AuditError::DuplicateEntry)
            );
        }

        #[test]
        fn test_append_entry_payload_too_large() {
            let config = AuditTrailConfig {
                max_payload_bytes: 10,
                ..Default::default()
            };
            let mut engine = AuditTrailV2::new(config);
            let time = current_time_ms();
            assert_eq!(
                engine.append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "This is a very long payload".to_string(),
                    time,
                ),
                Err(AuditError::PayloadTooLarge)
            );
        }

        #[test]
        fn test_append_entry_max_reached() {
            let config = AuditTrailConfig {
                max_entries: 2,
                ..Default::default()
            };
            let mut engine = AuditTrailV2::new(config);
            let time = current_time_ms();
            engine
                .append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 1".to_string(),
                    time,
                )
                .unwrap();
            engine
                .append_entry(
                    "e2".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 2".to_string(),
                    time,
                )
                .unwrap();
            assert_eq!(
                engine.append_entry(
                    "e3".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 3".to_string(),
                    time,
                ),
                Err(AuditError::MaxEntriesReached)
            );
        }

        #[test]
        fn test_chain_verification() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            engine
                .append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 1".to_string(),
                    time,
                )
                .unwrap();
            engine
                .append_entry(
                    "e2".to_string(),
                    AuditCategory::Security,
                    AuditSeverity::Warning,
                    "actor2".to_string(),
                    "Event 2".to_string(),
                    time + 1,
                )
                .unwrap();
            assert!(engine.verify_chain().is_ok());
        }

        #[test]
        fn test_chain_linking() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            engine
                .append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 1".to_string(),
                    time,
                )
                .unwrap();
            engine
                .append_entry(
                    "e2".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 2".to_string(),
                    time + 1,
                )
                .unwrap();
            let e1 = engine.get_entry("e1").unwrap();
            let e2 = engine.get_entry("e2").unwrap();
            assert_eq!(e2.previous_hash, e1.hash);
        }

        #[test]
        fn test_get_entry() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            engine
                .append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 1".to_string(),
                    time,
                )
                .unwrap();
            let entry = engine.get_entry("e1").unwrap();
            assert_eq!(entry.entry_id, "e1");
        }

        #[test]
        fn test_get_entry_not_found() {
            let engine = AuditTrailV2::default();
            assert!(engine.get_entry("nonexistent").is_none());
        }

        #[test]
        fn test_get_entries_by_category() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            engine
                .append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 1".to_string(),
                    time,
                )
                .unwrap();
            engine
                .append_entry(
                    "e2".to_string(),
                    AuditCategory::Security,
                    AuditSeverity::Warning,
                    "actor2".to_string(),
                    "Event 2".to_string(),
                    time + 1,
                )
                .unwrap();
            let governance_entries = engine.get_entries_by_category(&AuditCategory::Governance);
            assert_eq!(governance_entries.len(), 1);
        }

        #[test]
        fn test_get_entries_by_severity() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            engine
                .append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 1".to_string(),
                    time,
                )
                .unwrap();
            engine
                .append_entry(
                    "e2".to_string(),
                    AuditCategory::Security,
                    AuditSeverity::Critical,
                    "actor2".to_string(),
                    "Event 2".to_string(),
                    time + 1,
                )
                .unwrap();
            let critical_entries = engine.get_entries_by_severity(&AuditSeverity::Critical);
            assert_eq!(critical_entries.len(), 1);
        }

        #[test]
        fn test_get_entries_by_actor() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            engine
                .append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 1".to_string(),
                    time,
                )
                .unwrap();
            engine
                .append_entry(
                    "e2".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor2".to_string(),
                    "Event 2".to_string(),
                    time + 1,
                )
                .unwrap();
            let actor1_entries = engine.get_entries_by_actor("actor1");
            assert_eq!(actor1_entries.len(), 1);
        }

        #[test]
        fn test_get_recent_entries() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            for i in 0..5 {
                engine
                    .append_entry(
                        format!("e{}", i),
                        AuditCategory::Governance,
                        AuditSeverity::Info,
                        "actor1".to_string(),
                        format!("Event {}", i),
                        time + i,
                    )
                    .unwrap();
            }
            let recent = engine.get_recent_entries(3);
            assert_eq!(recent.len(), 3);
            assert_eq!(recent[0].entry_id, "e4");
        }

        #[test]
        fn test_add_metadata() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            engine
                .append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 1".to_string(),
                    time,
                )
                .unwrap();
            engine
                .add_metadata("e1", "key1".to_string(), "value1".to_string())
                .unwrap();
            let entry = engine.get_entry("e1").unwrap();
            assert_eq!(entry.metadata.get("key1"), Some(&"value1".to_string()));
        }

        #[test]
        fn test_add_metadata_not_found() {
            let mut engine = AuditTrailV2::default();
            assert_eq!(
                engine.add_metadata("nonexistent", "k".to_string(), "v".to_string()),
                Err(AuditError::EntryNotFound)
            );
        }

        #[test]
        fn test_compact() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            for i in 0..10 {
                engine
                    .append_entry(
                        format!("e{}", i),
                        AuditCategory::Governance,
                        AuditSeverity::Info,
                        "actor1".to_string(),
                        format!("Event {}", i),
                        time + i,
                    )
                    .unwrap();
            }
            let removed = engine.compact(5);
            assert_eq!(removed, 5);
            assert_eq!(engine.chain_length(), 5);
        }

        #[test]
        fn test_should_compact() {
            let config = AuditTrailConfig {
                max_entries: 10,
                auto_compact: true,
                compaction_threshold: 0.8,
                ..Default::default()
            };
            let mut engine = AuditTrailV2::new(config);
            let time = current_time_ms();
            for i in 0..7 {
                engine
                    .append_entry(
                        format!("e{}", i),
                        AuditCategory::Governance,
                        AuditSeverity::Info,
                        "actor1".to_string(),
                        format!("Event {}", i),
                        time + i,
                    )
                    .unwrap();
            }
            assert!(!engine.should_compact());
            engine
                .append_entry(
                    "e8".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 8".to_string(),
                    time + 8,
                )
                .unwrap();
            assert!(engine.should_compact());
        }

        #[test]
        fn test_metrics_tracking() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            engine
                .append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 1".to_string(),
                    time,
                )
                .unwrap();
            engine
                .append_entry(
                    "e2".to_string(),
                    AuditCategory::Security,
                    AuditSeverity::Critical,
                    "actor2".to_string(),
                    "Event 2".to_string(),
                    time + 1,
                )
                .unwrap();
            assert_eq!(engine.metrics().total_entries, 2);
            assert_eq!(engine.metrics().chain_length, 2);
        }

        #[test]
        fn test_reset_metrics() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            engine
                .append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 1".to_string(),
                    time,
                )
                .unwrap();
            engine.reset_metrics();
            assert_eq!(engine.metrics().total_entries, 0);
        }

        #[test]
        fn test_latest_entry() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            engine
                .append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 1".to_string(),
                    time,
                )
                .unwrap();
            let latest = engine.latest_entry().unwrap();
            assert_eq!(latest.entry_id, "e1");
        }

        #[test]
        fn test_first_entry() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            engine
                .append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 1".to_string(),
                    time,
                )
                .unwrap();
            engine
                .append_entry(
                    "e2".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 2".to_string(),
                    time + 1,
                )
                .unwrap();
            let first = engine.first_entry().unwrap();
            assert_eq!(first.entry_id, "e1");
        }

        #[test]
        fn test_clear() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            engine
                .append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 1".to_string(),
                    time,
                )
                .unwrap();
            engine.clear();
            assert_eq!(engine.chain_length(), 0);
        }

        #[test]
        fn test_config_default() {
            let config = AuditTrailConfig::default();
            assert_eq!(config.max_entries, 10000);
            assert!(config.chain_verification);
            assert!(config.signature_tracking);
            assert_eq!(config.max_payload_bytes, 4096);
        }

        #[test]
        fn test_metrics_default() {
            let metrics = AuditMetrics::default();
            assert_eq!(metrics.total_entries, 0);
            assert_eq!(metrics.chain_length, 0);
        }

        #[test]
        fn test_error_display() {
            let err = AuditError::EntryNotFound;
            assert!(!format!("{}", err).is_empty());
        }

        #[test]
        fn test_category_display() {
            let cat = AuditCategory::Governance;
            assert_eq!(format!("{}", cat), "Governance");
        }

        #[test]
        fn test_severity_display() {
            let sev = AuditSeverity::Critical;
            assert_eq!(format!("{}", sev), "Critical");
        }

        #[test]
        fn test_signature_generated() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            let entry = engine
                .append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 1".to_string(),
                    time,
                )
                .unwrap();
            assert!(!entry.signature.is_empty());
        }

        #[test]
        fn test_signature_disabled() {
            let config = AuditTrailConfig {
                signature_tracking: false,
                ..Default::default()
            };
            let mut engine = AuditTrailV2::new(config);
            let time = current_time_ms();
            let entry = engine
                .append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 1".to_string(),
                    time,
                )
                .unwrap();
            assert!(entry.signature.is_empty());
        }

        #[test]
        fn test_verification_disabled() {
            let config = AuditTrailConfig {
                chain_verification: false,
                ..Default::default()
            };
            let mut engine = AuditTrailV2::new(config);
            assert!(engine.verify_chain().is_ok());
        }

        #[test]
        fn test_hash_length() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            let entry = engine
                .append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 1".to_string(),
                    time,
                )
                .unwrap();
            assert_eq!(entry.hash.len(), 64); // SHA-256 hex
        }

        #[test]
        fn test_sequence_increment() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            let e1 = engine
                .append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 1".to_string(),
                    time,
                )
                .unwrap();
            let e2 = engine
                .append_entry(
                    "e2".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 2".to_string(),
                    time + 1,
                )
                .unwrap();
            assert_eq!(e1.sequence, 1);
            assert_eq!(e2.sequence, 2);
        }

        #[test]
        fn test_empty_chain_verification() {
            let mut engine = AuditTrailV2::default();
            assert!(engine.verify_chain().is_ok());
        }

        #[test]
        fn test_verification_metrics() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            engine
                .append_entry(
                    "e1".to_string(),
                    AuditCategory::Governance,
                    AuditSeverity::Info,
                    "actor1".to_string(),
                    "Event 1".to_string(),
                    time,
                )
                .unwrap();
            engine.verify_chain().unwrap();
            assert_eq!(engine.metrics().verification_count, 1);
            assert_eq!(engine.metrics().verification_failures, 0);
        }

        #[test]
        fn test_compaction_metrics() {
            let mut engine = AuditTrailV2::default();
            let time = current_time_ms();
            for i in 0..10 {
                engine
                    .append_entry(
                        format!("e{}", i),
                        AuditCategory::Governance,
                        AuditSeverity::Info,
                        "actor1".to_string(),
                        format!("Event {}", i),
                        time + i,
                    )
                    .unwrap();
            }
            engine.compact(5);
            assert_eq!(engine.metrics().compaction_count, 1);
        }

        #[test]
        fn test_custom_category_display() {
            let cat = AuditCategory::Custom("MyCategory".to_string());
            assert_eq!(format!("{}", cat), "Custom: MyCategory");
        }
    }
}

#[cfg(feature = "v1.5-sprint1")]
pub use internal::*;
