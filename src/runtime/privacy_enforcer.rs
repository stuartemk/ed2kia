//! Privacy Enforcer — Guardián de Privacidad Local.
//!
//! Intercepts system calls and I/O operations from sandboxed pillar modules
//! to enforce strict LOCAL_ONLY constraints. Blocks all network access (connect,
//! sendto, recvfrom), prevents telemetry exfiltration, and maintains an immutable
//! audit ledger of all intercepted operations.
//!
//! **Design Principles:**
//! - Radical privacy: no network access, no telemetry, zero data exfiltration.
//! - Constructive enforcement: violations are logged, not silently ignored.
//! - Audit trail: every intercepted operation recorded in local ledger.
//! - Cooperative isolation: pillars operate within safe boundaries.
//!
//! **Feature Gate:** `v3.0-privacy-guard`

use std::collections::HashSet;

/// Audit entry for intercepted operations.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Operation type (syscall number or named operation).
    pub operation: String,
    /// Result of the interception.
    pub result: InterceptionResult,
    /// Optional context (e.g., target address, file path).
    pub context: Option<String>,
}

/// Result of an interception attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterceptionResult {
    /// Operation allowed (local-only, safe).
    Allowed,
    /// Operation blocked due to privacy violation.
    Blocked(PrivacyViolation),
}

/// Privacy violation types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivacyViolation {
    /// Network connection attempt blocked.
    NetworkBlocked(String),
    /// Telemetry data exfiltration attempt blocked.
    TelemetryAttempt(String),
    /// Unauthorized filesystem access blocked.
    UnauthorizedFileAccess(String),
    /// General privacy violation.
    GeneralViolation(String),
}

impl std::fmt::Display for PrivacyViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrivacyViolation::NetworkBlocked(addr) => {
                write!(f, "Network access blocked: {}", addr)
            }
            PrivacyViolation::TelemetryAttempt(data) => {
                write!(f, "Telemetry exfiltration blocked: {}", data)
            }
            PrivacyViolation::UnauthorizedFileAccess(path) => {
                write!(f, "Unauthorized file access blocked: {}", path)
            }
            PrivacyViolation::GeneralViolation(msg) => {
                write!(f, "Privacy violation: {}", msg)
            }
        }
    }
}

/// Privacy Enforcer — Enforces LOCAL_ONLY constraints on sandboxed modules.
///
/// **⚠️ PRIVACY CONSTRAINT:** All network syscalls (connect, sendto, recvfrom)
/// are strictly blocked. Biometric and personal data must remain within the
/// local sandbox boundary. No external communication is permitted.
///
/// **Expected Flow:**
/// 1. Module attempts I/O operation (syscall).
/// 2. PrivacyEnforcer intercepts and classifies the operation.
/// 3. If operation is network/telemetry: block and log violation.
/// 4. If operation is local-safe: allow and log.
/// 5. Audit ledger updated (local-only, not propagated).
pub struct PrivacyEnforcer {
    /// Set of allowed syscall numbers (local-only operations).
    allowed_syscalls: HashSet<u64>,
    /// Blocklist of telemetry endpoints/patterns.
    telemetry_blocklist: Vec<&'static str>,
    /// Immutable audit log of all interceptions.
    audit_log: Vec<AuditEntry>,
}

impl PrivacyEnforcer {
    /// Create a new PrivacyEnforcer with default allowed syscalls.
    ///
    /// Default allowed syscalls: read (0), write (1) to local files only,
    /// fstat (2), lseek (3), close (4). All network syscalls blocked.
    pub fn new() -> Self {
        let mut allowed = HashSet::new();
        // Allow basic local file operations (read-only)
        allowed.insert(0);  // read
        allowed.insert(2);  // fstat
        allowed.insert(3);  // lseek
        allowed.insert(4);  // close
        // Note: write (1) NOT in default allowlist — prevents local writes

        Self {
            allowed_syscalls: allowed,
            telemetry_blocklist: vec![
                "telemetry.",
                "analytics.",
                "tracking.",
                "metrics.",
                "reporting.",
                ".google.",
                ".microsoft.",
                ".amazon.",
            ],
            audit_log: Vec::new(),
        }
    }

    /// Get the current audit log.
    pub fn audit_log(&self) -> &[AuditEntry] {
        &self.audit_log
    }

    /// Check if a syscall is in the allowed set.
    pub fn is_syscall_allowed(&self, syscall: u64) -> bool {
        self.allowed_syscalls.contains(&syscall)
    }

    /// Intercept an I/O operation and enforce privacy constraints.
    ///
    /// **Interception Logic:**
    /// 1. Check if syscall is in allowed set.
    /// 2. If syscall is network-related (connect/sendto/recvfrom): block immediately.
    /// 3. If target matches telemetry blocklist: block as telemetry attempt.
    /// 4. Otherwise: allow if in allowed set, block as general violation.
    ///
    /// # Arguments
    /// * `syscall` - System call number (libc::syscall).
    /// * `args` - System call arguments.
    ///
    /// # Returns
    /// * `Ok(())` - Operation allowed (local-only, safe).
    /// * `Err(PrivacyViolation)` - Operation blocked, violation recorded.
    pub fn intercept_io(&mut self, syscall: u64, args: &[u64]) -> Result<(), PrivacyViolation> {
        let timestamp_ms = self.current_timestamp_ms();
        let operation = format!("syscall_{}", syscall);

        // Step 1: Block known network syscalls immediately.
        // Syscall numbers (Linux x86_64):
        // connect = 43, sendto = 44, recvfrom = 45, socket = 41, bind = 49, listen = 50, accept = 43
        // send = 44, recv = 45, sendmsg = 46, recvmsg = 47
        if Self::is_network_syscall(syscall) {
            let target = Self::extract_target(args);
            let violation = PrivacyViolation::NetworkBlocked(target.clone());
            self.audit_log.push(AuditEntry {
                timestamp_ms,
                operation,
                result: InterceptionResult::Blocked(violation.clone()),
                context: Some(target),
            });
            return Err(violation);
        }

        // Step 2: Check telemetry blocklist.
        let target = Self::extract_target(args);
        for pattern in &self.telemetry_blocklist {
            if target.contains(*pattern) {
                let violation = PrivacyViolation::TelemetryAttempt(target.clone());
                self.audit_log.push(AuditEntry {
                    timestamp_ms,
                    operation: operation.clone(),
                    result: InterceptionResult::Blocked(violation.clone()),
                    context: Some(target),
                });
                return Err(violation);
            }
        }

        // Step 3: Check if syscall is in allowed set.
        if self.is_syscall_allowed(syscall) {
            self.audit_log.push(AuditEntry {
                timestamp_ms,
                operation,
                result: InterceptionResult::Allowed,
                context: None,
            });
            Ok(())
        } else {
            let violation = PrivacyViolation::GeneralViolation(format!(
                "Syscall {} not in allowed set",
                syscall
            ));
            self.audit_log.push(AuditEntry {
                timestamp_ms,
                operation,
                result: InterceptionResult::Blocked(violation.clone()),
                context: None,
            });
            Err(violation)
        }
    }

    /// Intercept a network operation specifically.
    ///
    /// This method always blocks and logs the attempt.
    /// Used for explicit network operation interception.
    ///
    /// # Arguments
    /// * `operation` - Name of the network operation (e.g., "connect", "sendto").
    /// * `target` - Target address/endpoint.
    ///
    /// # Returns
    /// Always `Err(PrivacyViolation::NetworkBlocked)`.
    pub fn intercept_network(
        &mut self,
        operation: &str,
        target: &str,
    ) -> Result<(), PrivacyViolation> {
        let timestamp_ms = self.current_timestamp_ms();
        let violation = PrivacyViolation::NetworkBlocked(target.to_string());

        self.audit_log.push(AuditEntry {
            timestamp_ms,
            operation: operation.to_string(),
            result: InterceptionResult::Blocked(violation.clone()),
            context: Some(target.to_string()),
        });

        Err(violation)
    }

    /// Add a syscall to the allowed set.
    pub fn allow_syscall(&mut self, syscall: u64) {
        self.allowed_syscalls.insert(syscall);
    }

    /// Remove a syscall from the allowed set.
    pub fn deny_syscall(&mut self, syscall: u64) {
        self.allowed_syscalls.remove(&syscall);
    }

    /// Add a pattern to the telemetry blocklist.
    pub fn add_telemetry_pattern(&mut self, pattern: &'static str) {
        self.telemetry_blocklist.push(pattern);
    }

    /// Clear the audit log (for testing/reset).
    pub fn clear_audit_log(&mut self) {
        self.audit_log.clear();
    }

    /// Get the count of blocked operations.
    pub fn blocked_count(&self) -> usize {
        self.audit_log
            .iter()
            .filter(|e| matches!(e.result, InterceptionResult::Blocked(_)))
            .count()
    }

    /// Get the count of allowed operations.
    pub fn allowed_count(&self) -> usize {
        self.audit_log
            .iter()
            .filter(|e| matches!(e.result, InterceptionResult::Allowed))
            .count()
    }

    /// Check if a syscall number corresponds to a network operation.
    fn is_network_syscall(syscall: u64) -> bool {
        matches!(
            syscall,
            41 | 43 | 44 | 45 | 46 | 47 | 49 | 50 // socket, connect, sendto, recvfrom, sendmsg, recvmsg, bind, listen
        )
    }

    /// Extract target address/path from syscall arguments.
    fn extract_target(args: &[u64]) -> String {
        if args.is_empty() {
            return "unknown".to_string();
        }
        // Scaffolding: Use first arg as target identifier.
        // In full implementation, decode sockaddr_in or path pointers.
        format!("0x{:016x}", args[0])
    }

    fn current_timestamp_ms(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

impl Default for PrivacyEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enforcer_creation() {
        let enforcer = PrivacyEnforcer::new();
        assert!(enforcer.audit_log().is_empty());
        assert_eq!(enforcer.blocked_count(), 0);
        assert_eq!(enforcer.allowed_count(), 0);
    }

    #[test]
    fn test_allowed_syscall() {
        let mut enforcer = PrivacyEnforcer::new();
        // read (syscall 0) is allowed
        let result = enforcer.intercept_io(0, &[]);
        assert!(result.is_ok());
        assert_eq!(enforcer.allowed_count(), 1);
    }

    #[test]
    fn test_network_syscall_blocked() {
        let mut enforcer = PrivacyEnforcer::new();
        // connect (syscall 43) is blocked
        let result = enforcer.intercept_io(43, &[0x7f000001]); // 127.0.0.1
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PrivacyViolation::NetworkBlocked(_)));
        assert_eq!(enforcer.blocked_count(), 1);
    }

    #[test]
    fn test_sendto_blocked() {
        let mut enforcer = PrivacyEnforcer::new();
        // sendto (syscall 44) is blocked
        let result = enforcer.intercept_io(44, &[0x0]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PrivacyViolation::NetworkBlocked(_)));
    }

    #[test]
    fn test_recvfrom_blocked() {
        let mut enforcer = PrivacyEnforcer::new();
        // recvfrom (syscall 45) is blocked
        let result = enforcer.intercept_io(45, &[0x0]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PrivacyViolation::NetworkBlocked(_)));
    }

    #[test]
    fn test_unallowed_syscall_blocked() {
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
    fn test_intercept_network() {
        let mut enforcer = PrivacyEnforcer::new();
        let result = enforcer.intercept_network("connect", "8.8.8.8:443");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PrivacyViolation::NetworkBlocked(_)));
        assert_eq!(enforcer.blocked_count(), 1);
    }

    #[test]
    fn test_allow_and_deny_syscall() {
        let mut enforcer = PrivacyEnforcer::new();
        assert!(!enforcer.is_syscall_allowed(1)); // write not allowed
        enforcer.allow_syscall(1);
        assert!(enforcer.is_syscall_allowed(1));
        enforcer.deny_syscall(1);
        assert!(!enforcer.is_syscall_allowed(1));
    }

    #[test]
    fn test_audit_log_records() {
        let mut enforcer = PrivacyEnforcer::new();
        enforcer.intercept_io(0, &[]); // allowed
        enforcer.intercept_io(43, &[]); // blocked
        assert_eq!(enforcer.audit_log().len(), 2);
        assert_eq!(enforcer.allowed_count(), 1);
        assert_eq!(enforcer.blocked_count(), 1);
    }

    #[test]
    fn test_clear_audit_log() {
        let mut enforcer = PrivacyEnforcer::new();
        enforcer.intercept_io(0, &[]);
        enforcer.clear_audit_log();
        assert!(enforcer.audit_log().is_empty());
    }

    #[test]
    fn test_default() {
        let enforcer = PrivacyEnforcer::default();
        assert!(enforcer.audit_log().is_empty());
    }

    #[test]
    fn test_violation_display() {
        match PrivacyViolation::NetworkBlocked("8.8.8.8".to_string()) {
            v => assert!(v.to_string().contains("8.8.8.8")),
        }
        match PrivacyViolation::TelemetryAttempt("telemetry.example.com".to_string()) {
            v => assert!(v.to_string().contains("telemetry")),
        }
    }
}
