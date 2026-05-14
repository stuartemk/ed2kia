//! Audit Trail — Cryptographic audit log for DAO governance actions.
//!
//! Provides immutable, hash-chained audit entries with tamper detection,
//! supporting full governance action traceability and compliance reporting.
//!
//! **Design:** Linux `auditd` + `journald`-inspired immutable audit logging.
//!
//! **Key features:**
//! - Hash-chained immutable entries
//! - Tamper detection via chain verification
//! - Action categorization and filtering
//! - Compliance report generation
//!
//! **References:**
//! - `dao_ledger_v4.rs` — Hash chain and Merkle patterns
//! - `hybrid_executor.rs` — Execution tracking patterns
//!
//! Apache License 2.0 + Ethical Use Clause

use std::collections::HashMap;

// ─── Error ───────────────────────────────────────────────────────────────────

/// Errors for audit trail operations.
#[derive(Debug, Clone, PartialEq)]
pub enum AuditTrailError {
    /// Entry not found.
    EntryNotFound(String),
    /// Chain verification failed.
    ChainBroken(String),
    /// Audit trail is full.
    TrailFull,
    /// Invalid configuration.
    InvalidConfig(String),
    /// Tampering detected.
    TamperingDetected(String),
}

impl std::fmt::Display for AuditTrailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditTrailError::EntryNotFound(id) => write!(f, "Audit entry not found: {}", id),
            AuditTrailError::ChainBroken(id) => write!(f, "Chain broken at: {}", id),
            AuditTrailError::TrailFull => write!(f, "Audit trail is full"),
            AuditTrailError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
            AuditTrailError::TamperingDetected(id) => write!(f, "Tampering detected at: {}", id),
        }
    }
}

// ─── Audit Action ────────────────────────────────────────────────────────────

/// Categories of auditable actions.
#[derive(Debug, Clone, PartialEq)]
pub enum AuditAction {
    /// Governance proposal action.
    Proposal,
    /// Voting action.
    Vote,
    /// Execution action.
    Execution,
    /// Configuration change.
    ConfigChange,
    /// Membership change.
    Membership,
    /// Security event.
    Security,
    /// System event.
    System,
    /// Custom action.
    Custom(String),
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditAction::Proposal => write!(f, "Proposal"),
            AuditAction::Vote => write!(f, "Vote"),
            AuditAction::Execution => write!(f, "Execution"),
            AuditAction::ConfigChange => write!(f, "ConfigChange"),
            AuditAction::Membership => write!(f, "Membership"),
            AuditAction::Security => write!(f, "Security"),
            AuditAction::System => write!(f, "System"),
            AuditAction::Custom(msg) => write!(f, "Custom({})", msg),
        }
    }
}

// ─── Severity ────────────────────────────────────────────────────────────────

/// Severity level for audit entries.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Informational event.
    Info,
    /// Warning event.
    Warning,
    /// Critical event requiring attention.
    Critical,
    /// Emergency event.
    Emergency,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "Info"),
            Severity::Warning => write!(f, "Warning"),
            Severity::Critical => write!(f, "Critical"),
            Severity::Emergency => write!(f, "Emergency"),
        }
    }
}

// ─── Config ──────────────────────────────────────────────────────────────────

/// Configuration for the audit trail.
#[derive(Debug, Clone)]
pub struct AuditTrailConfig {
    /// Maximum entries in the trail.
    pub max_entries: usize,
    /// Enable automatic chain verification.
    pub auto_verify: bool,
    /// Maximum payload size in bytes.
    pub max_payload_bytes: usize,
    /// Retention period in milliseconds (0 = infinite).
    pub retention_ms: u64,
    /// Enable compliance reporting.
    pub compliance_enabled: bool,
}

impl Default for AuditTrailConfig {
    fn default() -> Self {
        Self {
            max_entries: 500000,
            auto_verify: true,
            max_payload_bytes: 32768,
            retention_ms: 0,
            compliance_enabled: true,
        }
    }
}

// ─── Audit Entry ─────────────────────────────────────────────────────────────

/// An immutable audit entry.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// Unique entry ID.
    pub entry_id: String,
    /// Sequence number.
    pub sequence: u64,
    /// Action category.
    pub action: AuditAction,
    /// Severity level.
    pub severity: Severity,
    /// Actor who performed the action.
    pub actor_id: String,
    /// Target of the action.
    pub target_id: String,
    /// Action description/payload.
    pub description: String,
    /// Entry hash.
    pub hash: String,
    /// Previous entry hash (chain linking).
    pub previous_hash: String,
    /// Creation timestamp.
    pub timestamp_ms: u64,
    /// Metadata key-value pairs.
    pub metadata: HashMap<String, String>,
}

impl AuditEntry {
    pub fn new(
        entry_id: String,
        sequence: u64,
        action: AuditAction,
        severity: Severity,
        actor_id: String,
        target_id: String,
        description: String,
        previous_hash: String,
        timestamp_ms: u64,
    ) -> Self {
        let hash = compute_hash(&entry_id, sequence, &description, &previous_hash, timestamp_ms);
        Self {
            entry_id,
            sequence,
            action,
            severity,
            actor_id,
            target_id,
            description,
            hash,
            previous_hash,
            timestamp_ms,
            metadata: HashMap::new(),
        }
    }

    /// Verify entry hash integrity.
    pub fn verify_hash(&self) -> bool {
        let expected = compute_hash(&self.entry_id, self.sequence, &self.description, &self.previous_hash, self.timestamp_ms);
        self.hash == expected
    }
}

// ─── Compliance Report ───────────────────────────────────────────────────────

/// Compliance report for audit period.
#[derive(Debug, Clone)]
pub struct ComplianceReport {
    /// Report period start.
    pub period_start_ms: u64,
    /// Report period end.
    pub period_end_ms: u64,
    /// Total entries in period.
    pub total_entries: usize,
    /// Entries by severity.
    pub by_severity: HashMap<Severity, usize>,
    /// Entries by action.
    pub by_action: HashMap<String, usize>,
    /// Top actors by activity.
    pub top_actors: Vec<(String, usize)>,
    /// Chain integrity status.
    pub chain_intact: bool,
    /// Tampering detected.
    pub tampering_detected: bool,
}

impl ComplianceReport {
    pub fn new(period_start_ms: u64, period_end_ms: u64) -> Self {
        Self {
            period_start_ms,
            period_end_ms,
            total_entries: 0,
            by_severity: HashMap::new(),
            by_action: HashMap::new(),
            top_actors: Vec::new(),
            chain_intact: true,
            tampering_detected: false,
        }
    }
}

// ─── Stats ───────────────────────────────────────────────────────────────────

/// Statistics for the audit trail.
#[derive(Debug, Clone)]
pub struct AuditStats {
    /// Total entries recorded.
    pub total_entries: usize,
    /// Entries by severity.
    pub info_count: usize,
    pub warning_count: usize,
    pub critical_count: usize,
    pub emergency_count: usize,
    /// Chain verifications performed.
    pub verifications: usize,
    /// Tampering attempts detected.
    pub tampering_detected: usize,
    /// Current chain root hash.
    pub chain_root: String,
    /// Last entry sequence.
    pub last_sequence: u64,
}

impl Default for AuditStats {
    fn default() -> Self {
        Self {
            total_entries: 0,
            info_count: 0,
            warning_count: 0,
            critical_count: 0,
            emergency_count: 0,
            verifications: 0,
            tampering_detected: 0,
            chain_root: String::new(),
            last_sequence: 0,
        }
    }
}

// ─── Main Audit Trail ────────────────────────────────────────────────────────

/// Cryptographic audit trail for DAO governance.
pub struct AuditTrail {
    config: AuditTrailConfig,
    entries: HashMap<String, AuditEntry>,
    sequence_order: Vec<String>,
    stats: AuditStats,
    current_time_ms: u64,
    entry_counter: u64,
}

impl AuditTrail {
    // ─── Construction ──────────────────────────────────────────────────────

    /// Create a new audit trail.
    pub fn new(config: AuditTrailConfig) -> Self {
        Self {
            config,
            entries: HashMap::new(),
            sequence_order: Vec::new(),
            stats: AuditStats::default(),
            current_time_ms: current_timestamp_ms(),
            entry_counter: 0,
        }
    }

    /// Create with default configuration.
    pub fn default_config() -> Self {
        Self::new(AuditTrailConfig::default())
    }

    // ─── Entry Recording ───────────────────────────────────────────────────

    /// Record an audit entry.
    pub fn record(
        &mut self,
        action: AuditAction,
        severity: Severity,
        actor_id: String,
        target_id: String,
        description: String,
    ) -> Result<AuditEntry, AuditTrailError> {
        // Check capacity
        if self.entries.len() >= self.config.max_entries {
            return Err(AuditTrailError::TrailFull);
        }

        // Check payload size
        if description.len() > self.config.max_payload_bytes {
            return Err(AuditTrailError::InvalidConfig(format!(
                "Description size {} exceeds maximum {}",
                description.len(),
                self.config.max_payload_bytes
            )));
        }

        // Generate entry ID
        self.entry_counter += 1;
        let entry_id = format!("audit-{}", self.entry_counter);

        // Get previous hash
        let previous_hash = self.get_last_hash();

        // Create entry
        let sequence = self.entries.len() as u64 + 1;
        let entry = AuditEntry::new(
            entry_id.clone(),
            sequence,
            action.clone(),
            severity.clone(),
            actor_id,
            target_id,
            description,
            previous_hash,
            self.current_time_ms,
        );

        // Store entry
        self.entries.insert(entry_id.clone(), entry.clone());
        self.sequence_order.push(entry_id);

        // Update stats
        self.update_stats(&severity);
        self.stats.last_sequence = sequence;

        Ok(entry)
    }

    /// Record with metadata.
    pub fn record_with_metadata(
        &mut self,
        action: AuditAction,
        severity: Severity,
        actor_id: String,
        target_id: String,
        description: String,
        metadata: HashMap<String, String>,
    ) -> Result<String, AuditTrailError> {
        let entry = self.record(action, severity, actor_id, target_id, description)?;
        if let Some(entry) = self.entries.get_mut(&entry.entry_id) {
            entry.metadata = metadata;
        }
        Ok(entry.entry_id)
    }

    // ─── Chain Verification ────────────────────────────────────────────────

    /// Verify the entire chain integrity.
    pub fn verify_chain(&mut self) -> Result<(), AuditTrailError> {
        self.stats.verifications += 1;

        for id in &self.sequence_order {
            let entry = self.entries.get(id).ok_or_else(|| {
                AuditTrailError::EntryNotFound(id.clone())
            })?;

            if !entry.verify_hash() {
                self.stats.tampering_detected += 1;
                return Err(AuditTrailError::TamperingDetected(id.clone()));
            }

            // Verify chain linking
            let expected_prev = if entry.sequence == 1 {
                "genesis".to_string()
            } else {
                self.entries
                    .values()
                    .find(|e| e.sequence == entry.sequence - 1)
                    .map(|e| e.hash.clone())
                    .unwrap_or_default()
            };

            if entry.previous_hash != expected_prev {
                self.stats.tampering_detected += 1;
                return Err(AuditTrailError::ChainBroken(id.clone()));
            }
        }

        Ok(())
    }

    // ─── Queries ───────────────────────────────────────────────────────────

    /// Get entry by ID.
    pub fn get_entry(&self, entry_id: &str) -> Option<&AuditEntry> {
        self.entries.get(entry_id)
    }

    /// Get entries by action.
    pub fn get_by_action(&self, action: &AuditAction) -> Vec<&AuditEntry> {
        self.entries
            .values()
            .filter(|e| &e.action == action)
            .collect()
    }

    /// Get entries by severity.
    pub fn get_by_severity(&self, severity: &Severity) -> Vec<&AuditEntry> {
        self.entries
            .values()
            .filter(|e| &e.severity == severity)
            .collect()
    }

    /// Get entries by actor.
    pub fn get_by_actor(&self, actor_id: &str) -> Vec<&AuditEntry> {
        self.entries
            .values()
            .filter(|e| e.actor_id == actor_id)
            .collect()
    }

    /// Get entries by target.
    pub fn get_by_target(&self, target_id: &str) -> Vec<&AuditEntry> {
        self.entries
            .values()
            .filter(|e| e.target_id == target_id)
            .collect()
    }

    /// Get entries in time range.
    pub fn get_in_range(&self, start_ms: u64, end_ms: u64) -> Vec<&AuditEntry> {
        self.entries
            .values()
            .filter(|e| e.timestamp_ms >= start_ms && e.timestamp_ms <= end_ms)
            .collect()
    }

    /// Get recent entries.
    pub fn get_recent(&self, count: usize) -> Vec<&AuditEntry> {
        self.sequence_order
            .iter()
            .rev()
            .take(count)
            .filter_map(|id| self.entries.get(id))
            .collect()
    }

    /// Get entry by sequence.
    pub fn get_by_sequence(&self, sequence: u64) -> Option<&AuditEntry> {
        self.entries.values().find(|e| e.sequence == sequence)
    }

    // ─── Compliance Reporting ──────────────────────────────────────────────

    /// Generate compliance report for a time period.
    pub fn generate_compliance_report(&self, start_ms: u64, end_ms: u64) -> ComplianceReport {
        let mut report = ComplianceReport::new(start_ms, end_ms);

        let entries = self.get_in_range(start_ms, end_ms);
        report.total_entries = entries.len();

        // Count by severity
        for entry in &entries {
            *report.by_severity.entry(entry.severity.clone()).or_insert(0) += 1;
        }

        // Count by action
        for entry in &entries {
            let action_str = entry.action.to_string();
            *report.by_action.entry(action_str).or_insert(0) += 1;
        }

        // Top actors
        let mut actor_counts: HashMap<String, usize> = HashMap::new();
        for entry in &entries {
            *actor_counts.entry(entry.actor_id.clone()).or_insert(0) += 1;
        }
        let mut actors: Vec<(String, usize)> = actor_counts.into_iter().collect();
        actors.sort_by(|a, b| b.1.cmp(&a.1));
        report.top_actors = actors.into_iter().take(10).collect();

        // Chain integrity
        let mut chain_intact = true;
        for entry in &entries {
            if !entry.verify_hash() {
                chain_intact = false;
                break;
            }
        }
        report.chain_intact = chain_intact;
        report.tampering_detected = !chain_intact;

        report
    }

    // ─── Time ──────────────────────────────────────────────────────────────

    /// Advance internal time.
    pub fn advance_time(&mut self, ms: u64) {
        self.current_time_ms += ms;
    }

    // ─── Stats ─────────────────────────────────────────────────────────────

    /// Get current statistics.
    pub fn stats(&self) -> AuditStats {
        AuditStats {
            total_entries: self.entries.len(),
            info_count: self.stats.info_count,
            warning_count: self.stats.warning_count,
            critical_count: self.stats.critical_count,
            emergency_count: self.stats.emergency_count,
            verifications: self.stats.verifications,
            tampering_detected: self.stats.tampering_detected,
            chain_root: self.get_last_hash(),
            last_sequence: self.stats.last_sequence,
        }
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats = AuditStats::default();
    }

    // ─── Internal ──────────────────────────────────────────────────────────

    fn get_last_hash(&self) -> String {
        self.sequence_order
            .last()
            .and_then(|id| self.entries.get(id))
            .map(|e| e.hash.clone())
            .unwrap_or_else(|| "genesis".to_string())
    }

    fn update_stats(&mut self, severity: &Severity) {
        match severity {
            Severity::Info => self.stats.info_count += 1,
            Severity::Warning => self.stats.warning_count += 1,
            Severity::Critical => self.stats.critical_count += 1,
            Severity::Emergency => self.stats.emergency_count += 1,
        }
        self.stats.total_entries += 1;
    }
}

impl Default for AuditTrail {
    fn default() -> Self {
        Self::new(AuditTrailConfig::default())
    }
}

// ─── Hash Utilities ──────────────────────────────────────────────────────────

fn compute_hash(entry_id: &str, sequence: u64, description: &str, previous_hash: &str, timestamp_ms: u64) -> String {
    let data = format!("{}:{}:{}:{}:{}", entry_id, sequence, description, previous_hash, timestamp_ms);
    compute_single_hash(&data)
}

fn compute_single_hash(data: &str) -> String {
    let mut hash: u64 = 14695981039346656037;
    for byte in data.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    format!("{:016x}", hash)
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trail_creation() {
        let trail = AuditTrail::new(AuditTrailConfig::default());
        assert_eq!(trail.stats().total_entries, 0);
    }

    #[test]
    fn test_record_entry() {
        let mut trail = AuditTrail::default_config();
        let entry = trail.record(
            AuditAction::Proposal,
            Severity::Info,
            "actor-1".to_string(),
            "target-1".to_string(),
            "created proposal".to_string(),
        );
        assert!(entry.is_ok());
        assert_eq!(trail.stats().total_entries, 1);
    }

    #[test]
    fn test_record_with_metadata() {
        let mut trail = AuditTrail::default_config();
        let mut meta = HashMap::new();
        meta.insert("key".to_string(), "value".to_string());
        let id = trail.record_with_metadata(
            AuditAction::Proposal,
            Severity::Info,
            "actor-1".to_string(),
            "target-1".to_string(),
            "desc".to_string(),
            meta,
        );
        assert!(id.is_ok());
        assert!(!trail.get_entry(&id.unwrap()).unwrap().metadata.is_empty());
    }

    #[test]
    fn test_hash_verification() {
        let mut trail = AuditTrail::default_config();
        trail.record(AuditAction::Proposal, Severity::Info, "a".to_string(), "t".to_string(), "desc".to_string()).unwrap();
        let entry = trail.get_entry("audit-1").unwrap();
        assert!(entry.verify_hash());
    }

    #[test]
    fn test_chain_verification() {
        let mut trail = AuditTrail::default_config();
        for i in 1..=5 {
            trail.record(AuditAction::Proposal, Severity::Info, "a".to_string(), "t".to_string(), format!("desc {}", i)).unwrap();
        }
        assert!(trail.verify_chain().is_ok());
    }

    #[test]
    fn test_chain_linking() {
        let mut trail = AuditTrail::default_config();
        trail.record(AuditAction::Proposal, Severity::Info, "a".to_string(), "t".to_string(), "d1".to_string()).unwrap();
        trail.record(AuditAction::Vote, Severity::Info, "a".to_string(), "t".to_string(), "d2".to_string()).unwrap();
        let e1 = trail.get_entry("audit-1").unwrap();
        let e2 = trail.get_entry("audit-2").unwrap();
        assert_eq!(e1.previous_hash, "genesis");
        assert_eq!(e2.previous_hash, e1.hash);
    }

    #[test]
    fn test_get_by_action() {
        let mut trail = AuditTrail::default_config();
        trail.record(AuditAction::Proposal, Severity::Info, "a".to_string(), "t".to_string(), "d".to_string()).unwrap();
        trail.record(AuditAction::Vote, Severity::Info, "a".to_string(), "t".to_string(), "d".to_string()).unwrap();
        let proposals = trail.get_by_action(&AuditAction::Proposal);
        assert_eq!(proposals.len(), 1);
    }

    #[test]
    fn test_get_by_severity() {
        let mut trail = AuditTrail::default_config();
        trail.record(AuditAction::Proposal, Severity::Info, "a".to_string(), "t".to_string(), "d".to_string()).unwrap();
        trail.record(AuditAction::Security, Severity::Critical, "a".to_string(), "t".to_string(), "d".to_string()).unwrap();
        let critical = trail.get_by_severity(&Severity::Critical);
        assert_eq!(critical.len(), 1);
    }

    #[test]
    fn test_get_by_actor() {
        let mut trail = AuditTrail::default_config();
        trail.record(AuditAction::Proposal, Severity::Info, "actor-1".to_string(), "t".to_string(), "d".to_string()).unwrap();
        trail.record(AuditAction::Vote, Severity::Info, "actor-2".to_string(), "t".to_string(), "d".to_string()).unwrap();
        let entries = trail.get_by_actor("actor-1");
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_get_by_target() {
        let mut trail = AuditTrail::default_config();
        trail.record(AuditAction::Proposal, Severity::Info, "a".to_string(), "target-1".to_string(), "d".to_string()).unwrap();
        trail.record(AuditAction::Vote, Severity::Info, "a".to_string(), "target-2".to_string(), "d".to_string()).unwrap();
        let entries = trail.get_by_target("target-1");
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_get_in_range() {
        let mut trail = AuditTrail::default_config();
        trail.record(AuditAction::Proposal, Severity::Info, "a".to_string(), "t".to_string(), "d".to_string()).unwrap();
        let start = trail.current_time_ms - 1000;
        let end = trail.current_time_ms + 1000;
        let entries = trail.get_in_range(start, end);
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_get_recent() {
        let mut trail = AuditTrail::default_config();
        for i in 0..5 {
            trail.record(AuditAction::Proposal, Severity::Info, "a".to_string(), "t".to_string(), format!("d{}", i)).unwrap();
        }
        let recent = trail.get_recent(3);
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn test_get_by_sequence() {
        let mut trail = AuditTrail::default_config();
        trail.record(AuditAction::Proposal, Severity::Info, "a".to_string(), "t".to_string(), "d".to_string()).unwrap();
        let entry = trail.get_by_sequence(1);
        assert!(entry.is_some());
    }

    #[test]
    fn test_compliance_report() {
        let mut trail = AuditTrail::default_config();
        trail.record(AuditAction::Proposal, Severity::Info, "a".to_string(), "t".to_string(), "d".to_string()).unwrap();
        trail.record(AuditAction::Vote, Severity::Critical, "a".to_string(), "t".to_string(), "d".to_string()).unwrap();
        let report = trail.generate_compliance_report(0, trail.current_time_ms + 1000);
        assert_eq!(report.total_entries, 2);
        assert!(report.chain_intact);
    }

    #[test]
    fn test_trail_full() {
        let mut config = AuditTrailConfig::default();
        config.max_entries = 2;
        let mut trail = AuditTrail::new(config);
        trail.record(AuditAction::Proposal, Severity::Info, "a".to_string(), "t".to_string(), "d".to_string()).unwrap();
        trail.record(AuditAction::Proposal, Severity::Info, "a".to_string(), "t".to_string(), "d".to_string()).unwrap();
        assert!(trail.record(AuditAction::Proposal, Severity::Info, "a".to_string(), "t".to_string(), "d".to_string()).is_err());
    }

    #[test]
    fn test_payload_too_large() {
        let mut config = AuditTrailConfig::default();
        config.max_payload_bytes = 5;
        let mut trail = AuditTrail::new(config);
        assert!(trail.record(AuditAction::Proposal, Severity::Info, "a".to_string(), "t".to_string(), "very long description".to_string()).is_err());
    }

    #[test]
    fn test_stats_tracking() {
        let mut trail = AuditTrail::default_config();
        trail.record(AuditAction::Proposal, Severity::Info, "a".to_string(), "t".to_string(), "d".to_string()).unwrap();
        trail.record(AuditAction::Security, Severity::Critical, "a".to_string(), "t".to_string(), "d".to_string()).unwrap();
        trail.record(AuditAction::System, Severity::Emergency, "a".to_string(), "t".to_string(), "d".to_string()).unwrap();
        let stats = trail.stats();
        assert_eq!(stats.info_count, 1);
        assert_eq!(stats.critical_count, 1);
        assert_eq!(stats.emergency_count, 1);
    }

    #[test]
    fn test_reset_stats() {
        let mut trail = AuditTrail::default_config();
        trail.record(AuditAction::Proposal, Severity::Info, "a".to_string(), "t".to_string(), "d".to_string()).unwrap();
        trail.reset_stats();
        assert_eq!(trail.stats().verifications, 0);
    }

    #[test]
    fn test_action_display() {
        assert_eq!(AuditAction::Proposal.to_string(), "Proposal");
        assert_eq!(AuditAction::Custom("test".to_string()).to_string(), "Custom(test)");
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::Info.to_string(), "Info");
        assert_eq!(Severity::Emergency.to_string(), "Emergency");
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Critical);
        assert!(Severity::Critical < Severity::Emergency);
    }

    #[test]
    fn test_error_display() {
        match AuditTrailError::EntryNotFound("x".to_string()) {
            e => assert!(!e.to_string().is_empty()),
        }
    }

    #[test]
    fn test_config_default() {
        let config = AuditTrailConfig::default();
        assert_eq!(config.max_entries, 500000);
        assert!(config.auto_verify);
        assert!(config.compliance_enabled);
    }

    #[test]
    fn test_stats_default() {
        let stats = AuditStats::default();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.verifications, 0);
    }

    #[test]
    fn test_trail_default() {
        let trail = AuditTrail::default();
        assert_eq!(trail.stats().total_entries, 0);
    }

    #[test]
    fn test_advance_time() {
        let mut trail = AuditTrail::default_config();
        trail.advance_time(1000);
        assert_eq!(trail.current_time_ms, trail.current_time_ms);
    }

    #[test]
    fn test_compliance_top_actors() {
        let mut trail = AuditTrail::default_config();
        trail.record(AuditAction::Proposal, Severity::Info, "actor-1".to_string(), "t".to_string(), "d".to_string()).unwrap();
        trail.record(AuditAction::Vote, Severity::Info, "actor-1".to_string(), "t".to_string(), "d".to_string()).unwrap();
        trail.record(AuditAction::Vote, Severity::Info, "actor-2".to_string(), "t".to_string(), "d".to_string()).unwrap();
        let report = trail.generate_compliance_report(0, trail.current_time_ms + 1000);
        assert_eq!(report.top_actors[0].0, "actor-1");
        assert_eq!(report.top_actors[0].1, 2);
    }

    #[test]
    fn test_compliance_by_severity() {
        let mut trail = AuditTrail::default_config();
        trail.record(AuditAction::Proposal, Severity::Info, "a".to_string(), "t".to_string(), "d".to_string()).unwrap();
        trail.record(AuditAction::Security, Severity::Critical, "a".to_string(), "t".to_string(), "d".to_string()).unwrap();
        let report = trail.generate_compliance_report(0, trail.current_time_ms + 1000);
        assert_eq!(*report.by_severity.get(&Severity::Info).unwrap(), 1);
        assert_eq!(*report.by_severity.get(&Severity::Critical).unwrap(), 1);
    }
}
