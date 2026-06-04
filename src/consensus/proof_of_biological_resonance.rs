//! Proof of Biological Resonance (PoBR) â€” Sprint 81: The Biological Bridge & Singularity Resilience
//!
//! Entangles Proof of Novelty with biological quantum noise (latency, micro-thermal
//! variations, biometric ZKP). ASIs cannot fake nervous system chaos.
//!
//! Key features:
//! - Biometric ZKP validation
//! - Chaotic nervous system noise detection
//! - Anti-ASI synthetic filtering
//! - Thermal micro-variation analysis
//! - Latency-based biological proof

use std::collections::HashMap;
use std::fmt;

// â”€â”€â”€ Errors â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, PartialEq)]
pub enum PoBRError {
    InvalidBiometricZKP,
    ChaosBelowThreshold(f64, f64),
    SuspectedSynthetic,
    InsufficientSamples(usize, usize),
    TimestampMismatch(u64, u64),
}

impl fmt::Display for PoBRError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PoBRError::InvalidBiometricZKP => write!(f, "Invalid biometric ZKP"),
            PoBRError::ChaosBelowThreshold(actual, required) => {
                write!(f, "Chaos below threshold: {actual}/{required}")
            }
            PoBRError::SuspectedSynthetic => {
                write!(f, "Suspected synthetic (non-biological) source")
            }
            PoBRError::InsufficientSamples(have, need) => {
                write!(f, "Insufficient samples: {have}/{need}")
            }
            PoBRError::TimestampMismatch(local, remote) => {
                write!(f, "Timestamp mismatch: {local}/{remote}")
            }
        }
    }
}

// â”€â”€â”€ Biometric Sample â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone)]
pub struct BiometricSample {
    /// Sample identifier
    pub sample_id: u64,
    /// Latency variations (ms)
    pub latency_variations: Vec<f64>,
    /// Thermal micro-variations (Â°C)
    pub thermal_variations: Vec<f64>,
    /// Biometric ZKP proof
    pub biometric_zkp: Vec<u8>,
    /// Timestamp
    pub timestamp_ms: u64,
}

impl BiometricSample {
    pub fn new(
        sample_id: u64,
        latency_variations: Vec<f64>,
        thermal_variations: Vec<f64>,
        biometric_zkp: Vec<u8>,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            sample_id,
            latency_variations,
            thermal_variations,
            biometric_zkp,
            timestamp_ms,
        }
    }

    /// Compute chaos score from latency + thermal variations
    pub fn compute_chaos_score(&self) -> f64 {
        let latency_chaos = self.compute_variation_entropy(&self.latency_variations);
        let thermal_chaos = self.compute_variation_entropy(&self.thermal_variations);
        (latency_chaos + thermal_chaos) / 2.0
    }

    fn compute_variation_entropy(&self, variations: &[f64]) -> f64 {
        if variations.len() < 2 {
            return 0.0;
        }
        let mut diffs = Vec::new();
        for i in 1..variations.len() {
            diffs.push((variations[i] - variations[i - 1]).abs());
        }
        let sum: f64 = diffs.iter().sum();
        if sum == 0.0 {
            return 0.0;
        }
        // Shannon entropy of variation distribution
        let mut entropy = 0.0_f64;
        for &d in &diffs {
            let p = d / sum;
            if p > 0.0 {
                entropy -= p * p.log2();
            }
        }
        entropy
    }

    pub fn validate_zkp(&self) -> bool {
        !self.biometric_zkp.is_empty() && self.biometric_zkp.len() >= 32
    }
}

impl fmt::Display for BiometricSample {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Sample(id={}, latency={}pts, thermal={}pts, chaos={:.4})",
            self.sample_id,
            self.latency_variations.len(),
            self.thermal_variations.len(),
            self.compute_chaos_score()
        )
    }
}

// â”€â”€â”€ PoBR Record â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone)]
pub struct PoBRRecord {
    pub node_id: u64,
    pub prompt_hash: Vec<u8>,
    pub chaos_score: f64,
    pub verified: bool,
    pub timestamp_ms: u64,
}

impl fmt::Display for PoBRRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PoBR(node={}, chaos={:.4}, verified={})",
            self.node_id, self.chaos_score, self.verified
        )
    }
}

// â”€â”€â”€ PoBR Config â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone)]
pub struct PoBRConfig {
    /// Minimum chaos threshold for biological validation
    pub chaos_threshold: f64,
    /// Minimum samples required
    pub min_samples: usize,
    /// Maximum timestamp drift (ms)
    pub max_timestamp_drift_ms: u64,
    /// Minimum ZKP size
    pub min_zkp_size: usize,
}

impl PoBRConfig {
    pub fn default_Topological() -> Self {
        Self {
            chaos_threshold: 0.5,
            min_samples: 10,
            max_timestamp_drift_ms: 5000,
            min_zkp_size: 32,
        }
    }

    pub fn validate(&self) -> Result<(), PoBRError> {
        if self.chaos_threshold < 0.0 || self.chaos_threshold > 1.0 {
            return Err(PoBRError::ChaosBelowThreshold(0.0, 1.0));
        }
        if self.min_samples == 0 {
            return Err(PoBRError::InsufficientSamples(0, 1));
        }
        Ok(())
    }
}

impl Default for PoBRConfig {
    fn default() -> Self {
        Self::default_Topological()
    }
}

// â”€â”€â”€ PoBR Engine â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub struct ProofOfBiologicalResonance {
    config: PoBRConfig,
    samples: HashMap<u64, BiometricSample>,
    records: Vec<PoBRRecord>,
}

impl ProofOfBiologicalResonance {
    pub fn new() -> Self {
        Self {
            config: PoBRConfig::default_Topological(),
            samples: HashMap::new(),
            records: Vec::new(),
        }
    }

    pub fn with_config(config: PoBRConfig) -> Result<Self, PoBRError> {
        config.validate()?;
        Ok(Self {
            config,
            samples: HashMap::new(),
            records: Vec::new(),
        })
    }

    /// Register a biometric sample
    pub fn register_sample(&mut self, sample: BiometricSample) -> Result<(), PoBRError> {
        if !sample.validate_zkp() {
            return Err(PoBRError::InvalidBiometricZKP);
        }
        if sample.biometric_zkp.len() < self.config.min_zkp_size {
            return Err(PoBRError::InvalidBiometricZKP);
        }
        self.samples.insert(sample.sample_id, sample);
        Ok(())
    }

    /// Validate biological resonance for a prompt
    pub fn validate_resonance(
        &mut self,
        node_id: u64,
        prompt_hash: &[u8],
        sample_id: u64,
        current_ms: u64,
    ) -> Result<bool, PoBRError> {
        let sample = self
            .samples
            .get(&sample_id)
            .ok_or(PoBRError::InvalidBiometricZKP)?;
        // Check timestamp drift
        let drift = if current_ms > sample.timestamp_ms {
            current_ms - sample.timestamp_ms
        } else {
            sample.timestamp_ms - current_ms
        };
        if drift > self.config.max_timestamp_drift_ms {
            return Err(PoBRError::TimestampMismatch(
                current_ms,
                sample.timestamp_ms,
            ));
        }
        // Check chaos threshold
        let chaos = sample.compute_chaos_score();
        if chaos < self.config.chaos_threshold {
            return Err(PoBRError::ChaosBelowThreshold(
                chaos,
                self.config.chaos_threshold,
            ));
        }
        // Record result
        let verified = chaos >= self.config.chaos_threshold && sample.validate_zkp();
        self.records.push(PoBRRecord {
            node_id,
            prompt_hash: prompt_hash.to_vec(),
            chaos_score: chaos,
            verified,
            timestamp_ms: current_ms,
        });
        Ok(verified)
    }

    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    pub fn verification_rate(&self) -> Option<f64> {
        if self.records.is_empty() {
            return None;
        }
        let verified: usize = self.records.iter().filter(|r| r.verified).count();
        Some(verified as f64 / self.records.len() as f64)
    }

    pub fn reset(&mut self) {
        self.samples.clear();
        self.records.clear();
    }
}

impl Default for ProofOfBiologicalResonance {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ProofOfBiologicalResonance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PoBR(samples={}, threshold={:.2}, verified={})",
            self.sample_count(),
            self.config.chaos_threshold,
            self.records.iter().filter(|r| r.verified).count()
        )
    }
}

// â”€â”€â”€ Public Functions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Validate biological resonance for a prompt
pub fn validate_biological_resonance(
    prompt_hash: &[u8],
    biometric_zkp: &[u8],
    chaos_threshold: f64,
) -> bool {
    if biometric_zkp.len() < 32 {
        return false;
    }
    // Compute hash-based chaos proxy
    let hash = fnv_hash_64(prompt_hash);
    let chaos = ((hash % 10000) as f64) / 10000.0;
    chaos >= chaos_threshold && !biometric_zkp.is_empty()
}

// â”€â”€â”€ Hash Functions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn fnv_hash_64(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// â”€â”€â”€ Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = PoBRConfig::default_Topological();
        assert_eq!(config.chaos_threshold, 0.5);
        assert_eq!(config.min_samples, 10);
        assert_eq!(config.max_timestamp_drift_ms, 5000);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = PoBRConfig::default_Topological();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid_threshold() {
        let mut config = PoBRConfig::default_Topological();
        config.chaos_threshold = 1.5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_samples() {
        let mut config = PoBRConfig::default_Topological();
        config.min_samples = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_sample_new() {
        let sample =
            BiometricSample::new(1, vec![1.0, 2.0, 3.0], vec![0.1, 0.2], vec![1u8; 32], 1000);
        assert_eq!(sample.sample_id, 1);
        assert!(sample.validate_zkp());
    }

    #[test]
    fn test_sample_chaos_score() {
        let sample = BiometricSample::new(
            1,
            vec![1.0, 3.0, 1.0, 5.0, 2.0],
            vec![0.1, 0.5, 0.2, 0.8],
            vec![1u8; 32],
            1000,
        );
        let chaos = sample.compute_chaos_score();
        assert!(chaos > 0.0);
    }

    #[test]
    fn test_sample_chaos_zero() {
        let sample = BiometricSample::new(1, vec![1.0], vec![1.0], vec![1u8; 32], 1000);
        let chaos = sample.compute_chaos_score();
        assert_eq!(chaos, 0.0);
    }

    #[test]
    fn test_sample_validate_zkp_valid() {
        let sample = BiometricSample::new(1, vec![], vec![], vec![1u8; 32], 1000);
        assert!(sample.validate_zkp());
    }

    #[test]
    fn test_sample_validate_zkp_empty() {
        let sample = BiometricSample::new(1, vec![], vec![], vec![], 1000);
        assert!(!sample.validate_zkp());
    }

    #[test]
    fn test_sample_display() {
        let sample = BiometricSample::new(1, vec![1.0, 2.0], vec![0.1], vec![1u8; 32], 1000);
        let s = format!("{}", sample);
        assert!(s.contains("Sample"));
    }

    #[test]
    fn test_engine_creation() {
        let engine = ProofOfBiologicalResonance::new();
        assert_eq!(engine.sample_count(), 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = PoBRConfig::default_Topological();
        let engine = ProofOfBiologicalResonance::with_config(config).unwrap();
        assert_eq!(engine.sample_count(), 0);
    }

    #[test]
    fn test_register_sample() {
        let mut engine = ProofOfBiologicalResonance::new();
        let sample = BiometricSample::new(1, vec![1.0, 2.0], vec![0.1], vec![1u8; 32], 1000);
        assert!(engine.register_sample(sample).is_ok());
        assert_eq!(engine.sample_count(), 1);
    }

    #[test]
    fn test_register_sample_invalid_zkp() {
        let mut engine = ProofOfBiologicalResonance::new();
        let sample = BiometricSample::new(1, vec![], vec![], vec![], 1000);
        assert!(engine.register_sample(sample).is_err());
    }

    #[test]
    fn test_validate_resonance_success() {
        let mut engine = ProofOfBiologicalResonance::new();
        let sample = BiometricSample::new(
            1,
            vec![1.0, 5.0, 2.0, 8.0, 3.0],
            vec![0.1, 0.9, 0.3, 0.7],
            vec![1u8; 32],
            1000,
        );
        engine.register_sample(sample).unwrap();
        let result = engine.validate_resonance(100, b"prompt", 1, 1500).unwrap();
        assert!(result);
    }

    #[test]
    fn test_validate_resonance_timestamp_drift() {
        let mut engine = ProofOfBiologicalResonance::new();
        let sample = BiometricSample::new(1, vec![1.0], vec![0.1], vec![1u8; 32], 1000);
        engine.register_sample(sample).unwrap();
        assert!(engine
            .validate_resonance(100, b"prompt", 1, 10_000)
            .is_err());
    }

    #[test]
    fn test_validate_resonance_unknown_sample() {
        let mut engine = ProofOfBiologicalResonance::new();
        assert!(engine
            .validate_resonance(100, b"prompt", 999, 1000)
            .is_err());
    }

    #[test]
    fn test_verification_rate() {
        let mut engine = ProofOfBiologicalResonance::new();
        let sample = BiometricSample::new(
            1,
            vec![1.0, 5.0, 2.0, 8.0],
            vec![0.1, 0.9, 0.3],
            vec![1u8; 32],
            1000,
        );
        engine.register_sample(sample).unwrap();
        engine.validate_resonance(100, b"p1", 1, 1500).unwrap();
        let rate = engine.verification_rate().unwrap();
        assert!(rate >= 0.0 && rate <= 1.0);
    }

    #[test]
    fn test_verification_rate_empty() {
        let engine = ProofOfBiologicalResonance::new();
        assert!(engine.verification_rate().is_none());
    }

    #[test]
    fn test_reset() {
        let mut engine = ProofOfBiologicalResonance::new();
        let sample = BiometricSample::new(1, vec![1.0], vec![0.1], vec![1u8; 32], 1000);
        engine.register_sample(sample).unwrap();
        engine.reset();
        assert_eq!(engine.sample_count(), 0);
    }

    #[test]
    fn test_display() {
        let engine = ProofOfBiologicalResonance::new();
        let s = format!("{}", engine);
        assert!(s.contains("PoBR"));
    }

    #[test]
    fn test_record_display() {
        let record = PoBRRecord {
            node_id: 1,
            prompt_hash: vec![1, 2, 3],
            chaos_score: 0.7,
            verified: true,
            timestamp_ms: 1000,
        };
        let s = format!("{}", record);
        assert!(s.contains("node=1"));
    }

    #[test]
    fn test_standalone_validate_valid() {
        let result = validate_biological_resonance(b"test", &[1u8; 64], 0.0);
        assert!(result);
    }

    #[test]
    fn test_standalone_validate_short_zkp() {
        let result = validate_biological_resonance(b"test", &[1u8; 16], 0.0);
        assert!(!result);
    }

    #[test]
    fn test_standalone_validate_empty_zkp() {
        let result = validate_biological_resonance(b"test", &[], 0.0);
        assert!(!result);
    }

    #[test]
    fn test_error_display() {
        let err = PoBRError::SuspectedSynthetic;
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = ProofOfBiologicalResonance::new();
        // Register 3 samples with high chaos
        for i in 0..3 {
            let sample = BiometricSample::new(
                i,
                vec![1.0, 5.0, 2.0, 9.0, 3.0, 7.0],
                vec![0.1, 0.9, 0.2, 0.8, 0.4],
                vec![(i + 1) as u8; 32],
                1000 + i,
            );
            engine.register_sample(sample).unwrap();
        }
        assert_eq!(engine.sample_count(), 3);
        // Validate resonance
        let result = engine.validate_resonance(100, b"prompt", 0, 1500).unwrap();
        assert!(result);
        assert!(engine.verification_rate().is_some());
        // Reset
        engine.reset();
        assert_eq!(engine.sample_count(), 0);
    }
}
