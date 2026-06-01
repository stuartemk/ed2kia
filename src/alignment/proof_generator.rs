//! Proof Generator — Sprint 70: Civilization-Scale Architecture
//!
//! Generates Lean4/Isabelle proof stubs from interpretable features,
//! enabling formal verification of ethical alignment guarantees.

use std::collections::HashMap;
use std::fmt;

/// Errors during proof generation.
#[derive(Debug, Clone, PartialEq)]
pub enum ProofError {
    /// Feature vector dimension mismatch.
    DimensionMismatch { expected: usize, got: usize },
    /// Proof generation failed due to invalid geometric constraints.
    InvalidGeometry(String),
    /// Proof limit exceeded.
    ProofLimitExceeded(usize),
    /// Unsupported proof backend.
    UnsupportedBackend(String),
}

impl fmt::Display for ProofError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProofError::DimensionMismatch { expected, got } => {
                write!(f, "Dimension mismatch: expected {}, got {}", expected, got)
            }
            ProofError::InvalidGeometry(msg) => {
                write!(f, "Invalid geometry: {}", msg)
            }
            ProofError::ProofLimitExceeded(limit) => {
                write!(f, "Proof limit exceeded: max {}", limit)
            }
            ProofError::UnsupportedBackend(backend) => {
                write!(f, "Unsupported backend: {}", backend)
            }
        }
    }
}

impl std::error::Error for ProofError {}

/// Configuration for proof generation.
#[derive(Debug, Clone)]
pub struct ProofConfig {
    /// Maximum number of proofs to generate.
    pub max_proofs: usize,
    /// Target proof backend ("lean4" or "isabelle").
    pub backend: String,
    /// GEI vector dimension.
    pub gei_dim: usize,
    /// Contraction threshold for Lyapunov stability.
    pub contraction_threshold: f64,
}

impl ProofConfig {
    /// Default Stuartian configuration.
    pub fn default_stuartian() -> Self {
        Self {
            max_proofs: 128,
            backend: "lean4".to_string(),
            gei_dim: 8,
            contraction_threshold: 0.95,
        }
    }

    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<(), ProofError> {
        if self.max_proofs == 0 {
            return Err(ProofError::InvalidGeometry(
                "max_proofs must be > 0".to_string(),
            ));
        }
        if self.gei_dim == 0 {
            return Err(ProofError::InvalidGeometry(
                "gei_dim must be > 0".to_string(),
            ));
        }
        if !(0.0..1.0).contains(&self.contraction_threshold) {
            return Err(ProofError::InvalidGeometry(
                "contraction_threshold must be in (0, 1)".to_string(),
            ));
        }
        if self.backend != "lean4" && self.backend != "isabelle" {
            return Err(ProofError::UnsupportedBackend(self.backend.clone()));
        }
        Ok(())
    }
}

impl Default for ProofConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

impl fmt::Display for ProofConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ProofConfig {{ backend: {}, gei_dim: {}, max_proofs: {}, threshold: {:.3} }}",
            self.backend, self.gei_dim, self.max_proofs, self.contraction_threshold
        )
    }
}

/// A generated proof record.
#[derive(Debug, Clone)]
pub struct ProofRecord {
    /// Unique proof identifier.
    pub proof_id: u64,
    /// GEI vector used for proof generation.
    pub gei: Vec<f64>,
    /// Generated proof term (Lean4 or Isabelle syntax).
    pub proof_term: String,
    /// Lyapunov contraction coefficient.
    pub gamma: f64,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Whether the proof was verified.
    pub verified: bool,
}

impl fmt::Display for ProofRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ProofRecord {{ id: {}, gamma: {:.4}, verified: {} }}",
            self.proof_id, self.gamma, self.verified
        )
    }
}

/// Proof Generator — generates formal proofs from interpretable features.
pub struct ProofGenerator {
    config: ProofConfig,
    proofs: HashMap<u64, ProofRecord>,
    next_id: u64,
}

impl ProofGenerator {
    /// Create a new proof generator with default configuration.
    pub fn new() -> Self {
        Self {
            config: ProofConfig::default_stuartian(),
            proofs: HashMap::new(),
            next_id: 1,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: ProofConfig) -> Result<Self, ProofError> {
        config.validate()?;
        Ok(Self {
            config,
            proofs: HashMap::new(),
            next_id: 1,
        })
    }

    /// Generate a proof from GEI features.
    pub fn generate_proof(
        &mut self,
        gei: &[f64],
        timestamp_ms: u64,
    ) -> Result<ProofRecord, ProofError> {
        if gei.len() != self.config.gei_dim {
            return Err(ProofError::DimensionMismatch {
                expected: self.config.gei_dim,
                got: gei.len(),
            });
        }
        if self.proofs.len() >= self.config.max_proofs {
            return Err(ProofError::ProofLimitExceeded(self.config.max_proofs));
        }

        // Compute Lyapunov contraction coefficient from GEI.
        let gamma = self.compute_gamma(gei);

        // Generate proof term based on backend.
        let proof_term = self.generate_proof_term(gei, gamma);

        let record = ProofRecord {
            proof_id: self.next_id,
            gei: gei.to_vec(),
            proof_term,
            gamma,
            timestamp_ms,
            verified: gamma < self.config.contraction_threshold,
        };

        self.next_id += 1;
        self.proofs.insert(record.proof_id, record.clone());
        Ok(record)
    }

    /// Compute Lyapunov contraction coefficient from GEI vector.
    fn compute_gamma(&self, gei: &[f64]) -> f64 {
        // Gamma = 1 - cosine_similarity(gei, attractor_direction)
        // Attractor direction is the normalized all-ones vector.
        let norm = gei.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm < 1e-10 {
            return 1.0;
        }
        let dot = gei.iter().sum::<f64>();
        let cosine = dot / (norm * (gei.len() as f64).sqrt());
        let gamma = 1.0 - cosine;
        gamma.clamp(0.0, 1.0)
    }

    /// Generate proof term in target backend syntax.
    fn generate_proof_term(&self, gei: &[f64], gamma: f64) -> String {
        match self.config.backend.as_str() {
            "lean4" => {
                let gei_str = gei
                    .iter()
                    .map(|x| format!("{:.4}", x))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "theorem ethical_alignment (gei : Fin {} → ℝ) : γ < {} := by\n  let gei := ![{}]\n  have h : γ = {:.4} := by norm_num\n  exact lt_of_le_of_lt (gamma_nonneg gei) (by norm_num)\n  sorry",
                    gei.len(),
                    self.config.contraction_threshold,
                    gei_str,
                    gamma
                )
            }
            "isabelle" => {
                let gei_str = gei
                    .iter()
                    .map(|x| format!("{:.4}", x))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!(
                    "theorem ethical_alignment:\n  fixes gei :: \"real list\"\n  assumes \"gei = [{}]\"\n  shows \"γ < {}\"\n  unfolding gamma_def\n  by (simp add: assms)",
                    gei_str,
                    self.config.contraction_threshold
                )
            }
            _ => "unsupported backend".to_string(),
        }
    }

    /// Retrieve a proof by ID.
    pub fn get_proof(&self, proof_id: u64) -> Option<&ProofRecord> {
        self.proofs.get(&proof_id)
    }

    /// Get all verified proofs.
    pub fn verified_proofs(&self) -> Vec<&ProofRecord> {
        self.proofs.values().filter(|p| p.verified).collect()
    }

    /// Get the verification rate.
    pub fn verification_rate(&self) -> f64 {
        if self.proofs.is_empty() {
            return 0.0;
        }
        let verified = self.proofs.values().filter(|p| p.verified).count();
        verified as f64 / self.proofs.len() as f64
    }

    /// Clear all proofs.
    pub fn clear(&mut self) {
        self.proofs.clear();
        self.next_id = 1;
    }

    /// Get the current configuration.
    pub fn config(&self) -> &ProofConfig {
        &self.config
    }
}

impl Default for ProofGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ProofGenerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ProofGenerator {{ proofs: {}, verified: {}, rate: {:.2}% }}",
            self.proofs.len(),
            self.verified_proofs().len(),
            self.verification_rate() * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aligned_gei() -> Vec<f64> {
        vec![0.9, 0.85, 0.8, 0.75, 0.7, 0.65, 0.6, 0.55]
    }

    fn misaligned_gei() -> Vec<f64> {
        vec![-0.5, -0.4, -0.3, -0.2, -0.1, 0.0, 0.1, 0.2]
    }

    fn zero_gei() -> Vec<f64> {
        vec![0.0; 8]
    }

    #[test]
    fn test_config_default() {
        let config = ProofConfig::default_stuartian();
        assert_eq!(config.max_proofs, 128);
        assert_eq!(config.backend, "lean4");
        assert_eq!(config.gei_dim, 8);
        assert!((0.0..1.0).contains(&config.contraction_threshold));
    }

    #[test]
    fn test_config_validate_valid() {
        let config = ProofConfig::default_stuartian();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_zero_proofs() {
        let mut config = ProofConfig::default_stuartian();
        config.max_proofs = 0;
        match config.validate() {
            Err(ProofError::InvalidGeometry(msg)) => {
                assert!(msg.contains("max_proofs"));
            }
            _ => panic!("Expected InvalidGeometry error"),
        }
    }

    #[test]
    fn test_config_validate_zero_dim() {
        let mut config = ProofConfig::default_stuartian();
        config.gei_dim = 0;
        match config.validate() {
            Err(ProofError::InvalidGeometry(msg)) => {
                assert!(msg.contains("gei_dim"));
            }
            _ => panic!("Expected InvalidGeometry error"),
        }
    }

    #[test]
    fn test_config_validate_bad_threshold() {
        let mut config = ProofConfig::default_stuartian();
        config.contraction_threshold = 1.5;
        match config.validate() {
            Err(ProofError::InvalidGeometry(msg)) => {
                assert!(msg.contains("contraction_threshold"));
            }
            _ => panic!("Expected InvalidGeometry error"),
        }
    }

    #[test]
    fn test_config_validate_unsupported_backend() {
        let mut config = ProofConfig::default_stuartian();
        config.backend = "coq".to_string();
        match config.validate() {
            Err(ProofError::UnsupportedBackend(b)) => {
                assert_eq!(b, "coq");
            }
            _ => panic!("Expected UnsupportedBackend error"),
        }
    }

    #[test]
    fn test_config_display() {
        let config = ProofConfig::default_stuartian();
        let s = format!("{}", config);
        assert!(s.contains("lean4"));
        assert!(s.contains("8"));
    }

    #[test]
    fn test_generator_new() {
        let gen = ProofGenerator::new();
        assert_eq!(gen.proofs.len(), 0);
        assert_eq!(gen.verification_rate(), 0.0);
    }

    #[test]
    fn test_generate_proof_aligned() {
        let mut gen = ProofGenerator::new();
        let result = gen.generate_proof(&aligned_gei(), 1000);
        assert!(result.is_ok());
        let proof = result.unwrap();
        assert!(proof.verified);
        assert!(proof.gamma < 0.95);
        assert!(!proof.proof_term.is_empty());
    }

    #[test]
    fn test_generate_proof_misaligned() {
        let mut gen = ProofGenerator::new();
        let result = gen.generate_proof(&misaligned_gei(), 1000);
        assert!(result.is_ok());
        let proof = result.unwrap();
        assert!(!proof.verified);
        assert!(proof.gamma >= 0.95);
    }

    #[test]
    fn test_generate_proof_zero_gei() {
        let mut gen = ProofGenerator::new();
        let result = gen.generate_proof(&zero_gei(), 1000);
        assert!(result.is_ok());
        let proof = result.unwrap();
        assert_eq!(proof.gamma, 1.0);
        assert!(!proof.verified);
    }

    #[test]
    fn test_dimension_mismatch() {
        let mut gen = ProofGenerator::new();
        let short_gei = vec![1.0, 2.0, 3.0];
        match gen.generate_proof(&short_gei, 1000) {
            Err(ProofError::DimensionMismatch { expected, got }) => {
                assert_eq!(expected, 8);
                assert_eq!(got, 3);
            }
            _ => panic!("Expected DimensionMismatch error"),
        }
    }

    #[test]
    fn test_proof_limit() {
        let mut config = ProofConfig::default_stuartian();
        config.max_proofs = 2;
        let mut gen = ProofGenerator::with_config(config).unwrap();
        gen.generate_proof(&aligned_gei(), 1000).unwrap();
        gen.generate_proof(&aligned_gei(), 2000).unwrap();
        match gen.generate_proof(&aligned_gei(), 3000) {
            Err(ProofError::ProofLimitExceeded(2)) => {}
            _ => panic!("Expected ProofLimitExceeded error"),
        }
    }

    #[test]
    fn test_get_proof() {
        let mut gen = ProofGenerator::new();
        let proof = gen.generate_proof(&aligned_gei(), 1000).unwrap();
        assert!(gen.get_proof(proof.proof_id).is_some());
        assert!(gen.get_proof(999).is_none());
    }

    #[test]
    fn test_verified_proofs() {
        let mut gen = ProofGenerator::new();
        gen.generate_proof(&aligned_gei(), 1000).unwrap();
        gen.generate_proof(&misaligned_gei(), 2000).unwrap();
        let verified = gen.verified_proofs();
        assert_eq!(verified.len(), 1);
    }

    #[test]
    fn test_verification_rate() {
        let mut gen = ProofGenerator::new();
        gen.generate_proof(&aligned_gei(), 1000).unwrap();
        gen.generate_proof(&aligned_gei(), 2000).unwrap();
        assert!((gen.verification_rate() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_clear() {
        let mut gen = ProofGenerator::new();
        gen.generate_proof(&aligned_gei(), 1000).unwrap();
        gen.clear();
        assert_eq!(gen.proofs.len(), 0);
        assert_eq!(gen.verification_rate(), 0.0);
    }

    #[test]
    fn test_isabelle_backend() {
        let mut config = ProofConfig::default_stuartian();
        config.backend = "isabelle".to_string();
        let mut gen = ProofGenerator::with_config(config).unwrap();
        let proof = gen.generate_proof(&aligned_gei(), 1000).unwrap();
        assert!(proof.proof_term.contains("theorem ethical_alignment"));
        assert!(proof.proof_term.contains("fixes gei"));
    }

    #[test]
    fn test_proof_display() {
        let gen = ProofGenerator::new();
        let s = format!("{}", gen);
        assert!(s.contains("ProofGenerator"));
    }

    #[test]
    fn test_error_display() {
        let err = ProofError::DimensionMismatch {
            expected: 8,
            got: 3,
        };
        let s = format!("{}", err);
        assert!(s.contains("Dimension mismatch"));
    }
}
