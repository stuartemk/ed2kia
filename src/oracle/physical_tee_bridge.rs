//! Physical TEE Bridge — Sprint 79: Quantum-Physical Bridge & God-Level Resilience
//!
//! TEE (Trusted Execution Environment) oracles with thermodynamic proof-of-work.
//! Bridges physical hardware attestation (SGX/TDX/SEV) with consensus layer.
//!
//! Key features:
//! - Hardware attestation verification (simulated for portability)
//! - Thermodynamic proof-of-work coupling
//! - Quote validation with nonce binding
//! - Replay protection via monotonic counters
//! - Multi-TEE aggregation (SGX + TDX + SEV)

use std::collections::HashMap;
use std::fmt;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum TeeError {
    InvalidAttestation,
    QuoteExpired,
    NonceMismatch,
    CounterRegression,
    UnsupportedTeeType,
    InsufficientAggregation(usize, usize),
    ThermodynamicThresholdExceeded(f64),
    InvalidQuoteFormat,
}

impl fmt::Display for TeeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TeeError::InvalidAttestation => write!(f, "Invalid TEE attestation"),
            TeeError::QuoteExpired => write!(f, "TEE quote expired"),
            TeeError::NonceMismatch => write!(f, "Nonce mismatch in quote"),
            TeeError::CounterRegression => write!(f, "Monotonic counter regression detected"),
            TeeError::UnsupportedTeeType => write!(f, "Unsupported TEE type"),
            TeeError::InsufficientAggregation(have, need) => {
                write!(f, "Insufficient TEE aggregation: {have}/{need}")
            }
            TeeError::ThermodynamicThresholdExceeded(val) => {
                write!(f, "Thermodynamic threshold exceeded: {val}")
            }
            TeeError::InvalidQuoteFormat => write!(f, "Invalid quote format"),
        }
    }
}

// ─── TEE Types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TeeType {
    Sgx,
    Tdx,
    Sev,
}

impl fmt::Display for TeeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TeeType::Sgx => write!(f, "SGX"),
            TeeType::Tdx => write!(f, "TDX"),
            TeeType::Sev => write!(f, "SEV"),
        }
    }
}

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TeeBridgeConfig {
    /// Maximum quote age in milliseconds
    pub max_quote_age_ms: u64,
    /// Minimum TEEs required for aggregation quorum
    pub min_aggregation_count: usize,
    /// Thermodynamic work threshold (joules simulated)
    pub thermodynamic_threshold: f64,
    /// Monotonic counter enforcement (reject regression)
    pub enforce_counter: bool,
    /// Supported TEE types
    pub supported_tees: Vec<TeeType>,
}

impl TeeBridgeConfig {
    pub fn default_stuartian() -> Self {
        Self {
            max_quote_age_ms: 30_000,
            min_aggregation_count: 3,
            thermodynamic_threshold: 0.001,
            enforce_counter: true,
            supported_tees: vec![TeeType::Sgx, TeeType::Tdx, TeeType::Sev],
        }
    }

    pub fn validate(&self) -> Result<(), TeeError> {
        if self.max_quote_age_ms == 0 {
            return Err(TeeError::InvalidQuoteFormat);
        }
        if self.min_aggregation_count == 0 {
            return Err(TeeError::InsufficientAggregation(0, 1));
        }
        if self.thermodynamic_threshold < 0.0 {
            return Err(TeeError::ThermodynamicThresholdExceeded(
                self.thermodynamic_threshold,
            ));
        }
        if self.supported_tees.is_empty() {
            return Err(TeeError::UnsupportedTeeType);
        }
        Ok(())
    }
}

impl Default for TeeBridgeConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

// ─── TEE Quote ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TeeQuote {
    pub tee_type: TeeType,
    pub enclave_id: u64,
    pub measurement: Vec<u8>,
    pub nonce: u64,
    pub timestamp_ms: u64,
    pub monotonic_counter: u64,
    pub signature: Vec<u8>,
}

impl TeeQuote {
    pub fn new(
        tee_type: TeeType,
        enclave_id: u64,
        measurement: Vec<u8>,
        nonce: u64,
        timestamp_ms: u64,
        monotonic_counter: u64,
    ) -> Self {
        let signature = Self::compute_signature(
            tee_type,
            enclave_id,
            &measurement,
            nonce,
            timestamp_ms,
            monotonic_counter,
        );
        Self {
            tee_type,
            enclave_id,
            measurement,
            nonce,
            timestamp_ms,
            monotonic_counter,
            signature,
        }
    }

    fn compute_signature(
        tee_type: TeeType,
        enclave_id: u64,
        measurement: &[u8],
        nonce: u64,
        timestamp_ms: u64,
        counter: u64,
    ) -> Vec<u8> {
        let mut data = Vec::new();
        let type_val = match tee_type {
            TeeType::Sgx => 1u64,
            TeeType::Tdx => 2u64,
            TeeType::Sev => 3u64,
        };
        data.extend_from_slice(&type_val.to_le_bytes());
        data.extend_from_slice(&enclave_id.to_le_bytes());
        data.extend_from_slice(measurement);
        data.extend_from_slice(&nonce.to_le_bytes());
        data.extend_from_slice(&timestamp_ms.to_le_bytes());
        data.extend_from_slice(&counter.to_le_bytes());
        fnv_hash_256(&data)
    }

    pub fn verify_signature(&self) -> bool {
        let expected = Self::compute_signature(
            self.tee_type,
            self.enclave_id,
            &self.measurement,
            self.nonce,
            self.timestamp_ms,
            self.monotonic_counter,
        );
        self.signature == expected
    }

    pub fn is_expired(&self, current_ms: u64, max_age_ms: u64) -> bool {
        if current_ms >= self.timestamp_ms {
            current_ms - self.timestamp_ms > max_age_ms
        } else {
            self.timestamp_ms - current_ms > max_age_ms
        }
    }
}

impl fmt::Display for TeeQuote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TeeQuote({} enclave={} counter={} ts={})",
            self.tee_type, self.enclave_id, self.monotonic_counter, self.timestamp_ms
        )
    }
}

// ─── TEE Record ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct TeeRecord {
    pub enclave_id: u64,
    pub tee_type: TeeType,
    pub verified: bool,
    pub thermodynamic_work: f64,
    pub timestamp_ms: u64,
}

impl fmt::Display for TeeRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TeeRecord({} enclave={} verified={} work={:.6})",
            self.tee_type, self.enclave_id, self.verified, self.thermodynamic_work
        )
    }
}

// ─── Bridge Engine ────────────────────────────────────────────────────────────

pub struct PhysicalTeeBridge {
    config: TeeBridgeConfig,
    counters: HashMap<u64, u64>,
    records: Vec<TeeRecord>,
    total_work: f64,
    verified_count: usize,
    rejected_count: usize,
}

impl PhysicalTeeBridge {
    pub fn new() -> Self {
        Self {
            config: TeeBridgeConfig::default_stuartian(),
            counters: HashMap::new(),
            records: Vec::new(),
            total_work: 0.0,
            verified_count: 0,
            rejected_count: 0,
        }
    }

    pub fn with_config(config: TeeBridgeConfig) -> Result<Self, TeeError> {
        config.validate()?;
        Ok(Self {
            config,
            counters: HashMap::new(),
            records: Vec::new(),
            total_work: 0.0,
            verified_count: 0,
            rejected_count: 0,
        })
    }

    pub fn verify_quote(
        &mut self,
        quote: &TeeQuote,
        current_ms: u64,
    ) -> Result<TeeRecord, TeeError> {
        // Check TEE type supported
        if !self.config.supported_tees.contains(&quote.tee_type) {
            return Err(TeeError::UnsupportedTeeType);
        }

        // Verify signature
        if !quote.verify_signature() {
            return Err(TeeError::InvalidAttestation);
        }

        // Check expiration
        if quote.is_expired(current_ms, self.config.max_quote_age_ms) {
            return Err(TeeError::QuoteExpired);
        }

        // Check monotonic counter
        if self.config.enforce_counter {
            if let Some(&last_counter) = self.counters.get(&quote.enclave_id) {
                if quote.monotonic_counter <= last_counter {
                    return Err(TeeError::CounterRegression);
                }
            }
            self.counters
                .insert(quote.enclave_id, quote.monotonic_counter);
        }

        // Compute thermodynamic work
        let work = Self::compute_thermodynamic_work(&quote.measurement);
        if work > self.config.thermodynamic_threshold {
            return Err(TeeError::ThermodynamicThresholdExceeded(work));
        }

        // Record success
        self.total_work += work;
        self.verified_count += 1;

        let record = TeeRecord {
            enclave_id: quote.enclave_id,
            tee_type: quote.tee_type,
            verified: true,
            thermodynamic_work: work,
            timestamp_ms: current_ms,
        };
        self.records.push(record.clone());
        Ok(record)
    }

    pub fn verify_quote_with_nonce(
        &mut self,
        quote: &TeeQuote,
        expected_nonce: u64,
        current_ms: u64,
    ) -> Result<TeeRecord, TeeError> {
        if quote.nonce != expected_nonce {
            return Err(TeeError::NonceMismatch);
        }
        self.verify_quote(quote, current_ms)
    }

    pub fn aggregate_quotes(
        &self,
        quotes: &[TeeQuote],
        current_ms: u64,
    ) -> Result<Vec<TeeRecord>, TeeError> {
        let mut records = Vec::new();
        let mut types_seen = HashMap::new();

        for quote in quotes {
            // Count by type
            let count = types_seen.entry(quote.tee_type).or_insert(0usize);
            *count += 1;

            // Quick validation (full verification in verify_quote)
            if !quote.verify_signature() {
                continue;
            }
            if quote.is_expired(current_ms, self.config.max_quote_age_ms) {
                continue;
            }

            let work = Self::compute_thermodynamic_work(&quote.measurement);
            records.push(TeeRecord {
                enclave_id: quote.enclave_id,
                tee_type: quote.tee_type,
                verified: true,
                thermodynamic_work: work,
                timestamp_ms: current_ms,
            });
        }

        if records.len() < self.config.min_aggregation_count {
            return Err(TeeError::InsufficientAggregation(
                records.len(),
                self.config.min_aggregation_count,
            ));
        }

        Ok(records)
    }

    fn compute_thermodynamic_work(measurement: &[u8]) -> f64 {
        // Simulated thermodynamic work based on measurement entropy
        let mut entropy_sum: f64 = 0.0;
        for &byte in measurement {
            let bit_count = byte.count_ones() as f64;
            entropy_sum += 1.0 - (bit_count - 4.0).abs() / 4.0;
        }
        entropy_sum / measurement.len().max(1) as f64 * 0.0001
    }

    pub fn get_counter(&self, enclave_id: u64) -> Option<u64> {
        self.counters.get(&enclave_id).copied()
    }

    pub fn total_work(&self) -> f64 {
        self.total_work
    }

    pub fn verified_count(&self) -> usize {
        self.verified_count
    }

    pub fn rejected_count(&self) -> usize {
        self.rejected_count
    }

    pub fn records(&self) -> &[TeeRecord] {
        &self.records
    }

    pub fn reset(&mut self) {
        self.counters.clear();
        self.records.clear();
        self.total_work = 0.0;
        self.verified_count = 0;
        self.rejected_count = 0;
    }
}

impl Default for PhysicalTeeBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PhysicalTeeBridge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PhysicalTeeBridge(verified={} rejected={} work={:.6})",
            self.verified_count, self.rejected_count, self.total_work
        )
    }
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Create a TEE quote with auto-generated signature
pub fn create_tee_quote(
    tee_type: TeeType,
    enclave_id: u64,
    measurement: Vec<u8>,
    nonce: u64,
    timestamp_ms: u64,
    counter: u64,
) -> TeeQuote {
    TeeQuote::new(
        tee_type,
        enclave_id,
        measurement,
        nonce,
        timestamp_ms,
        counter,
    )
}

/// Verify a TEE quote signature standalone
pub fn verify_tee_quote(quote: &TeeQuote) -> bool {
    quote.verify_signature()
}

/// Compute thermodynamic work from measurement bytes
pub fn compute_thermodynamic_work(measurement: &[u8]) -> f64 {
    PhysicalTeeBridge::compute_thermodynamic_work(measurement)
}

// ─── Utilities ────────────────────────────────────────────────────────────────

fn fnv_hash_256(data: &[u8]) -> Vec<u8> {
    let mut hash = [0u8; 32];
    let mut h: u64 = 0xcbf29ce484222325;
    for &byte in data {
        h ^= byte as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    hash[0..8].copy_from_slice(&h.to_le_bytes());

    let mut h2: u64 = 0x6c62272e07bb0142;
    for &byte in data.iter().rev() {
        h2 ^= byte as u64;
        h2 = h2.wrapping_mul(0x100000001b3);
    }
    hash[8..16].copy_from_slice(&h2.to_le_bytes());

    let mut h3: u64 = 0x43b0cdb3c8e7d4a5;
    for (i, &byte) in data.iter().enumerate() {
        h3 ^= (byte.wrapping_mul(i as u8 + 1)) as u64;
        h3 = h3.wrapping_mul(0x100000001b3);
    }
    hash[16..24].copy_from_slice(&h3.to_le_bytes());

    let mut h4: u64 = 0x89abc123def45678;
    for (i, &byte) in data.iter().enumerate().rev() {
        h4 ^= (byte.wrapping_mul(i as u8 + 1)) as u64;
        h4 = h4.wrapping_mul(0x100000001b3);
    }
    hash[24..32].copy_from_slice(&h4.to_le_bytes());

    hash.to_vec()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = TeeBridgeConfig::default_stuartian();
        assert_eq!(config.max_quote_age_ms, 30_000);
        assert_eq!(config.min_aggregation_count, 3);
        assert!(config.enforce_counter);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = TeeBridgeConfig::default_stuartian();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_zero_age() {
        let config = TeeBridgeConfig {
            max_quote_age_ms: 0,
            ..TeeBridgeConfig::default_stuartian()
        };
        assert_eq!(config.validate(), Err(TeeError::InvalidQuoteFormat));
    }

    #[test]
    fn test_config_zero_aggregation() {
        let config = TeeBridgeConfig {
            min_aggregation_count: 0,
            ..TeeBridgeConfig::default_stuartian()
        };
        assert_eq!(
            config.validate(),
            Err(TeeError::InsufficientAggregation(0, 1))
        );
    }

    #[test]
    fn test_config_negative_threshold() {
        let config = TeeBridgeConfig {
            thermodynamic_threshold: -1.0,
            ..TeeBridgeConfig::default_stuartian()
        };
        assert!(matches!(
            config.validate(),
            Err(TeeError::ThermodynamicThresholdExceeded(_))
        ));
    }

    #[test]
    fn test_config_empty_tees() {
        let config = TeeBridgeConfig {
            supported_tees: vec![],
            ..TeeBridgeConfig::default_stuartian()
        };
        assert_eq!(config.validate(), Err(TeeError::UnsupportedTeeType));
    }

    #[test]
    fn test_quote_creation() {
        let quote = TeeQuote::new(TeeType::Sgx, 1, vec![1, 2, 3], 42, 1000, 1);
        assert_eq!(quote.tee_type, TeeType::Sgx);
        assert_eq!(quote.enclave_id, 1);
        assert_eq!(quote.nonce, 42);
        assert_eq!(quote.monotonic_counter, 1);
    }

    #[test]
    fn test_quote_signature_valid() {
        let quote = TeeQuote::new(TeeType::Tdx, 2, vec![5, 6, 7], 99, 2000, 3);
        assert!(quote.verify_signature());
    }

    #[test]
    fn test_quote_signature_tampered() {
        let mut quote = TeeQuote::new(TeeType::Sev, 3, vec![8, 9], 10, 3000, 2);
        quote.signature = vec![0, 0, 0];
        assert!(!quote.verify_signature());
    }

    #[test]
    fn test_quote_not_expired() {
        let quote = TeeQuote::new(TeeType::Sgx, 1, vec![1], 0, 10000, 1);
        assert!(!quote.is_expired(10050, 30_000));
    }

    #[test]
    fn test_quote_expired() {
        let quote = TeeQuote::new(TeeType::Sgx, 1, vec![1], 0, 10000, 1);
        assert!(quote.is_expired(50000, 30_000));
    }

    #[test]
    fn test_quote_display() {
        let quote = TeeQuote::new(TeeType::Sgx, 42, vec![1], 0, 1000, 5);
        let s = format!("{quote}");
        assert!(s.contains("SGX"));
        assert!(s.contains("enclave=42"));
    }

    #[test]
    fn test_engine_creation() {
        let engine = PhysicalTeeBridge::new();
        assert_eq!(engine.verified_count(), 0);
        assert_eq!(engine.rejected_count(), 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = TeeBridgeConfig::default_stuartian();
        let engine = PhysicalTeeBridge::with_config(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_verify_quote_success() {
        let mut engine = PhysicalTeeBridge::new();
        let quote = TeeQuote::new(TeeType::Sgx, 1, vec![1, 2, 3], 42, 10000, 1);
        let result = engine.verify_quote(&quote, 10050);
        assert!(result.is_ok());
        assert!(result.unwrap().verified);
    }

    #[test]
    fn test_verify_quote_unsupported_type() {
        let config = TeeBridgeConfig {
            supported_tees: vec![TeeType::Sgx],
            ..TeeBridgeConfig::default_stuartian()
        };
        let mut engine = PhysicalTeeBridge::with_config(config).unwrap();
        let quote = TeeQuote::new(TeeType::Tdx, 1, vec![1], 0, 10000, 1);
        assert_eq!(
            engine.verify_quote(&quote, 10050),
            Err(TeeError::UnsupportedTeeType)
        );
    }

    #[test]
    fn test_verify_quote_expired() {
        let mut engine = PhysicalTeeBridge::new();
        let quote = TeeQuote::new(TeeType::Sgx, 1, vec![1], 0, 1000, 1);
        assert_eq!(
            engine.verify_quote(&quote, 100_000),
            Err(TeeError::QuoteExpired)
        );
    }

    #[test]
    fn test_verify_quote_counter_regression() {
        let mut engine = PhysicalTeeBridge::new();
        let q1 = TeeQuote::new(TeeType::Sgx, 1, vec![1], 0, 10000, 5);
        engine.verify_quote(&q1, 10050).unwrap();
        let q2 = TeeQuote::new(TeeType::Sgx, 1, vec![1], 0, 10060, 3);
        assert_eq!(
            engine.verify_quote(&q2, 10070),
            Err(TeeError::CounterRegression)
        );
    }

    #[test]
    fn test_verify_quote_counter_increasing() {
        let mut engine = PhysicalTeeBridge::new();
        let q1 = TeeQuote::new(TeeType::Sgx, 1, vec![1], 0, 10000, 1);
        engine.verify_quote(&q1, 10050).unwrap();
        let q2 = TeeQuote::new(TeeType::Sgx, 1, vec![1], 0, 10060, 2);
        assert!(engine.verify_quote(&q2, 10070).is_ok());
    }

    #[test]
    fn test_verify_with_nonce_match() {
        let mut engine = PhysicalTeeBridge::new();
        let quote = TeeQuote::new(TeeType::Sgx, 1, vec![1], 123, 10000, 1);
        let result = engine.verify_quote_with_nonce(&quote, 123, 10050);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_with_nonce_mismatch() {
        let mut engine = PhysicalTeeBridge::new();
        let quote = TeeQuote::new(TeeType::Sgx, 1, vec![1], 123, 10000, 1);
        assert_eq!(
            engine.verify_quote_with_nonce(&quote, 456, 10050),
            Err(TeeError::NonceMismatch)
        );
    }

    #[test]
    fn test_aggregate_quotes_success() {
        let mut engine = PhysicalTeeBridge::new();
        let quotes = vec![
            TeeQuote::new(TeeType::Sgx, 1, vec![1], 0, 10000, 1),
            TeeQuote::new(TeeType::Tdx, 2, vec![2], 0, 10000, 1),
            TeeQuote::new(TeeType::Sev, 3, vec![3], 0, 10000, 1),
        ];
        let result = engine.aggregate_quotes(&quotes, 10050);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 3);
    }

    #[test]
    fn test_aggregate_quotes_insufficient() {
        let config = TeeBridgeConfig {
            min_aggregation_count: 5,
            ..TeeBridgeConfig::default_stuartian()
        };
        let mut engine = PhysicalTeeBridge::with_config(config).unwrap();
        let quotes = vec![
            TeeQuote::new(TeeType::Sgx, 1, vec![1], 0, 10000, 1),
            TeeQuote::new(TeeType::Tdx, 2, vec![2], 0, 10000, 1),
        ];
        assert!(matches!(
            engine.aggregate_quotes(&quotes, 10050),
            Err(TeeError::InsufficientAggregation(2, 5))
        ));
    }

    #[test]
    fn test_get_counter() {
        let mut engine = PhysicalTeeBridge::new();
        let quote = TeeQuote::new(TeeType::Sgx, 1, vec![1], 0, 10000, 7);
        engine.verify_quote(&quote, 10050).unwrap();
        assert_eq!(engine.get_counter(1), Some(7));
        assert_eq!(engine.get_counter(999), None);
    }

    #[test]
    fn test_thermodynamic_work_positive() {
        let work = compute_thermodynamic_work(&[1, 2, 3, 4, 5]);
        assert!(work > 0.0);
        assert!(work < 0.001);
    }

    #[test]
    fn test_reset() {
        let mut engine = PhysicalTeeBridge::new();
        let quote = TeeQuote::new(TeeType::Sgx, 1, vec![1], 0, 10000, 1);
        engine.verify_quote(&quote, 10050).unwrap();
        engine.reset();
        assert_eq!(engine.verified_count(), 0);
        assert_eq!(engine.total_work(), 0.0);
        assert_eq!(engine.records().len(), 0);
    }

    #[test]
    fn test_display() {
        let engine = PhysicalTeeBridge::new();
        let s = format!("{engine}");
        assert!(s.contains("PhysicalTeeBridge"));
    }

    #[test]
    fn test_record_display() {
        let record = TeeRecord {
            enclave_id: 1,
            tee_type: TeeType::Sgx,
            verified: true,
            thermodynamic_work: 0.0005,
            timestamp_ms: 1000,
        };
        let s = format!("{record}");
        assert!(s.contains("SGX"));
        assert!(s.contains("verified=true"));
    }

    #[test]
    fn test_standalone_create_quote() {
        let quote = create_tee_quote(TeeType::Sgx, 1, vec![1, 2], 42, 1000, 1);
        assert!(verify_tee_quote(&quote));
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = PhysicalTeeBridge::new();

        // Create and verify quotes from multiple TEEs
        let q1 = TeeQuote::new(TeeType::Sgx, 1, vec![1, 2, 3], 100, 10000, 1);
        let q2 = TeeQuote::new(TeeType::Tdx, 2, vec![4, 5, 6], 101, 10010, 1);
        let q3 = TeeQuote::new(TeeType::Sev, 3, vec![7, 8, 9], 102, 10020, 1);

        assert!(engine.verify_quote(&q1, 10050).is_ok());
        assert!(engine.verify_quote(&q2, 10060).is_ok());
        assert!(engine.verify_quote(&q3, 10070).is_ok());

        assert_eq!(engine.verified_count(), 3);
        assert!(engine.total_work() > 0.0);

        // Aggregate
        let quotes = vec![q1, q2, q3];
        let records = engine.aggregate_quotes(&quotes, 10050).unwrap();
        assert_eq!(records.len(), 3);

        // Reset
        engine.reset();
        assert_eq!(engine.verified_count(), 0);
    }

    #[test]
    fn test_error_display() {
        let err = TeeError::InvalidAttestation;
        assert!(!format!("{err}").is_empty());
    }
}
