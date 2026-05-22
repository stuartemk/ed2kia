//! Multi-Curve ZKP Setup v1 — Foundation for v2.0 multi-curve proof systems.
//!
//! Supports BN254, BLS12-381, BLS12-377 and Pasta curves with circuit parameters,
//! proof aggregation v2 hooks, and benchmark integration for criterion.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────┐
//! │         Multi-Curve ZKP v2               │
//! │  ┌────────┐  ┌────────┐  ┌────────────┐  │
//! │  │ BN254  │  │ BLS12  │  │ BLS12-377  │  │
//! │  │ -fast  │  │ -381   │  │ -pairing   │  │
//! │  │  proof │  │ -vault │  │ -research  │  │
//! │  └────────┘  └────────┘  └────────────┘  │
//! │  ┌────────┐  ┌────────────────────────┐  │
//! │  │ Pasta  │  │  Proof Aggregation v2  │  │
//! │  │ -Mina  │  │  -Batch verify         │  │
//! │  │  -zk   │  │  -Cross-curve bridge   │  │
//! │  └────────┘  └────────────────────────┘  │
//! └──────────────────────────────────────────┘
//! ```

mod internal {
    use serde::{Deserialize, Serialize};

    /// Curve selection error
    #[derive(Debug, Clone, PartialEq)]
    pub enum CurveError {
        /// Unsupported curve
        UnsupportedCurve(String),
        /// Invalid circuit parameter
        InvalidParameter(String),
        /// Proof generation failed
        ProofFailed(String),
        /// Verification failed
        VerificationFailed(String),
    }

    impl std::fmt::Display for CurveError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                CurveError::UnsupportedCurve(curve) => write!(f, "Unsupported curve: {}", curve),
                CurveError::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
                CurveError::ProofFailed(msg) => write!(f, "Proof failed: {}", msg),
                CurveError::VerificationFailed(msg) => write!(f, "Verification failed: {}", msg),
            }
        }
    }

    /// Supported elliptic curves for ZKP
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub enum ZKPCurve {
        /// BN254 — Fast proof generation, general purpose
        BN254,
        /// BLS12-381 — Aggregation, vault applications
        BLS12_381,
        /// BLS12-377 — Pairing-friendly, research
        BLS12_377,
        /// Pasta (Vesta/Imogen) — Mina protocol, recursive proofs
        Pasta,
    }

    impl std::fmt::Display for ZKPCurve {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ZKPCurve::BN254 => write!(f, "BN254"),
                ZKPCurve::BLS12_381 => write!(f, "BLS12-381"),
                ZKPCurve::BLS12_377 => write!(f, "BLS12-377"),
                ZKPCurve::Pasta => write!(f, "Pasta"),
            }
        }
    }

    impl ZKPCurve {
        /// Get curve properties
        pub fn properties(&self) -> CurveProperties {
            match self {
                ZKPCurve::BN254 => CurveProperties {
                    scalar_bits: 254,
                    proof_size_bytes: 64,
                    verification_key_bytes: 64,
                    estimated_proof_ms: 50.0,
                    estimated_verify_ms: 0.5,
                    aggregation_supported: true,
                },
                ZKPCurve::BLS12_381 => CurveProperties {
                    scalar_bits: 255,
                    proof_size_bytes: 48,
                    verification_key_bytes: 96,
                    estimated_proof_ms: 80.0,
                    estimated_verify_ms: 0.8,
                    aggregation_supported: true,
                },
                ZKPCurve::BLS12_377 => CurveProperties {
                    scalar_bits: 255,
                    proof_size_bytes: 48,
                    verification_key_bytes: 96,
                    estimated_proof_ms: 85.0,
                    estimated_verify_ms: 0.9,
                    aggregation_supported: true,
                },
                ZKPCurve::Pasta => CurveProperties {
                    scalar_bits: 255,
                    proof_size_bytes: 32,
                    verification_key_bytes: 64,
                    estimated_proof_ms: 120.0,
                    estimated_verify_ms: 1.2,
                    aggregation_supported: false,
                },
            }
        }
    }

    /// Curve properties
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CurveProperties {
        /// Scalar field size in bits
        pub scalar_bits: usize,
        /// Proof size in bytes
        pub proof_size_bytes: usize,
        /// Verification key size in bytes
        pub verification_key_bytes: usize,
        /// Estimated proof generation time (ms)
        pub estimated_proof_ms: f64,
        /// Estimated verification time (ms)
        pub estimated_verify_ms: f64,
        /// Whether proof aggregation is supported
        pub aggregation_supported: bool,
    }

    /// Circuit parameters for proof generation
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct CircuitParams {
        /// Number of constraints
        pub constraint_count: usize,
        /// Circuit complexity (0.0 — 1.0)
        pub complexity: f64,
        /// Public input count
        pub public_inputs: usize,
        /// Private input count
        pub private_inputs: usize,
    }

    impl CircuitParams {
        pub fn new(constraint_count: usize, complexity: f64) -> Result<Self, CurveError> {
            if constraint_count == 0 {
                return Err(CurveError::InvalidParameter(
                    "Constraint count must be > 0".to_string(),
                ));
            }
            if !(0.0..=1.0).contains(&complexity) {
                return Err(CurveError::InvalidParameter(
                    "Complexity must be between 0.0 and 1.0".to_string(),
                ));
            }
            Ok(Self {
                constraint_count,
                complexity,
                public_inputs: 1,
                private_inputs: constraint_count / 10,
            })
        }

        pub fn estimated_proof_time(&self, curve: &ZKPCurve) -> f64 {
            let base = curve.properties().estimated_proof_ms;
            base * (1.0 + self.complexity * 9.0)
        }
    }

    /// Multi-curve configuration
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MultiCurveConfig {
        /// Primary curve for proof generation
        pub primary_curve: ZKPCurve,
        /// Secondary curve for aggregation
        pub aggregation_curve: Option<ZKPCurve>,
        /// Maximum batch size for aggregation
        pub max_batch_size: usize,
        /// Enable cross-curve verification
        pub cross_curve_verify: bool,
    }

    impl MultiCurveConfig {
        pub fn new(primary: ZKPCurve) -> Self {
            Self {
                primary_curve: primary,
                aggregation_curve: None,
                max_batch_size: 16,
                cross_curve_verify: false,
            }
        }

        pub fn with_aggregation(mut self, curve: ZKPCurve) -> Self {
            self.aggregation_curve = Some(curve);
            self
        }

        pub fn supports_aggregation(&self) -> bool {
            self.aggregation_curve.is_some()
                && self.primary_curve.properties().aggregation_supported
        }
    }

    impl Default for MultiCurveConfig {
        fn default() -> Self {
            Self::new(ZKPCurve::BN254)
        }
    }

    /// Proof aggregation v2 entry
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AggregationEntryV2 {
        pub proof_id: String,
        pub curve: ZKPCurve,
        pub proof_size: usize,
        pub verified: bool,
    }

    /// Proof aggregation v2 batch
    pub struct AggregationBatchV2 {
        pub batch_id: String,
        pub entries: Vec<AggregationEntryV2>,
        pub max_size: usize,
    }

    impl AggregationBatchV2 {
        pub fn new(batch_id: String, max_size: usize) -> Self {
            Self {
                batch_id,
                entries: Vec::new(),
                max_size,
            }
        }

        pub fn add(&mut self, entry: AggregationEntryV2) -> Result<(), CurveError> {
            if self.entries.len() >= self.max_size {
                return Err(CurveError::InvalidParameter("Batch full".to_string()));
            }
            self.entries.push(entry);
            Ok(())
        }

        pub fn size(&self) -> usize {
            self.entries.len()
        }

        pub fn is_full(&self) -> bool {
            self.entries.len() >= self.max_size
        }

        pub fn verified_count(&self) -> usize {
            self.entries.iter().filter(|e| e.verified).count()
        }
    }

    /// Benchmark result for curve comparison
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CurveBenchmark {
        pub curve: ZKPCurve,
        pub proof_time_ms: f64,
        pub verify_time_ms: f64,
        pub proof_size_bytes: usize,
        pub constraint_count: usize,
    }

    /// Multi-curve manager
    pub struct MultiCurveManager {
        config: MultiCurveConfig,
        benchmarks: Vec<CurveBenchmark>,
    }

    impl MultiCurveManager {
        pub fn new(config: MultiCurveConfig) -> Self {
            Self {
                config,
                benchmarks: Vec::new(),
            }
        }

        pub fn config(&self) -> &MultiCurveConfig {
            &self.config
        }

        pub fn record_benchmark(&mut self, benchmark: CurveBenchmark) {
            self.benchmarks.push(benchmark);
        }

        pub fn best_curve_for_constraints(&self, constraint_count: usize) -> &ZKPCurve {
            // Simple heuristic: BN254 for small circuits, BLS12-381 for large
            if constraint_count < 1000 {
                &ZKPCurve::BN254
            } else {
                &ZKPCurve::BLS12_381
            }
        }

        pub fn benchmark_count(&self) -> usize {
            self.benchmarks.len()
        }
    }

    impl Default for MultiCurveManager {
        fn default() -> Self {
            Self::new(MultiCurveConfig::default())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_curve_display() {
            assert_eq!(format!("{}", ZKPCurve::BN254), "BN254");
            assert_eq!(format!("{}", ZKPCurve::BLS12_381), "BLS12-381");
            assert_eq!(format!("{}", ZKPCurve::BLS12_377), "BLS12-377");
            assert_eq!(format!("{}", ZKPCurve::Pasta), "Pasta");
        }

        #[test]
        fn test_bn254_properties() {
            let props = ZKPCurve::BN254.properties();
            assert_eq!(props.scalar_bits, 254);
            assert!(props.aggregation_supported);
        }

        #[test]
        fn test_bls12_381_properties() {
            let props = ZKPCurve::BLS12_381.properties();
            assert_eq!(props.scalar_bits, 255);
            assert!(props.aggregation_supported);
        }

        #[test]
        fn test_pasta_properties() {
            let props = ZKPCurve::Pasta.properties();
            assert!(!props.aggregation_supported);
        }

        #[test]
        fn test_circuit_params_valid() {
            let params = CircuitParams::new(100, 0.5).unwrap();
            assert_eq!(params.constraint_count, 100);
            assert_eq!(params.complexity, 0.5);
        }

        #[test]
        fn test_circuit_params_zero_constraints() {
            assert_eq!(
                CircuitParams::new(0, 0.5),
                Err(CurveError::InvalidParameter(
                    "Constraint count must be > 0".to_string()
                ))
            );
        }

        #[test]
        fn test_circuit_params_invalid_complexity() {
            assert_eq!(
                CircuitParams::new(100, 1.5),
                Err(CurveError::InvalidParameter(
                    "Complexity must be between 0.0 and 1.0".to_string()
                ))
            );
        }

        #[test]
        fn test_estimated_proof_time() {
            let params = CircuitParams::new(100, 0.0).unwrap();
            let time = params.estimated_proof_time(&ZKPCurve::BN254);
            assert!((time - 50.0).abs() < 0.01);
        }

        #[test]
        fn test_config_default() {
            let config = MultiCurveConfig::default();
            assert_eq!(config.primary_curve, ZKPCurve::BN254);
        }

        #[test]
        fn test_config_with_aggregation() {
            let config =
                MultiCurveConfig::new(ZKPCurve::BN254).with_aggregation(ZKPCurve::BLS12_381);
            assert!(config.supports_aggregation());
        }

        #[test]
        fn test_batch_creation() {
            let batch = AggregationBatchV2::new("b1".to_string(), 4);
            assert_eq!(batch.size(), 0);
            assert!(!batch.is_full());
        }

        #[test]
        fn test_batch_add() {
            let mut batch = AggregationBatchV2::new("b1".to_string(), 2);
            batch
                .add(AggregationEntryV2 {
                    proof_id: "p1".to_string(),
                    curve: ZKPCurve::BN254,
                    proof_size: 64,
                    verified: true,
                })
                .unwrap();
            assert_eq!(batch.size(), 1);
            assert_eq!(batch.verified_count(), 1);
        }

        #[test]
        fn test_batch_full() {
            let mut batch = AggregationBatchV2::new("b1".to_string(), 1);
            batch
                .add(AggregationEntryV2 {
                    proof_id: "p1".to_string(),
                    curve: ZKPCurve::BN254,
                    proof_size: 64,
                    verified: false,
                })
                .unwrap();
            assert!(batch.is_full());
            assert_eq!(
                batch.add(AggregationEntryV2 {
                    proof_id: "p2".to_string(),
                    curve: ZKPCurve::BN254,
                    proof_size: 64,
                    verified: false,
                }),
                Err(CurveError::InvalidParameter("Batch full".to_string()))
            );
        }

        #[test]
        fn test_manager_best_curve_small() {
            let manager = MultiCurveManager::default();
            let curve = manager.best_curve_for_constraints(100);
            assert_eq!(*curve, ZKPCurve::BN254);
        }

        #[test]
        fn test_manager_best_curve_large() {
            let manager = MultiCurveManager::default();
            let curve = manager.best_curve_for_constraints(5000);
            assert_eq!(*curve, ZKPCurve::BLS12_381);
        }

        #[test]
        fn test_manager_record_benchmark() {
            let mut manager = MultiCurveManager::default();
            manager.record_benchmark(CurveBenchmark {
                curve: ZKPCurve::BN254,
                proof_time_ms: 45.0,
                verify_time_ms: 0.4,
                proof_size_bytes: 64,
                constraint_count: 100,
            });
            assert_eq!(manager.benchmark_count(), 1);
        }

        #[test]
        fn test_error_display() {
            let err = CurveError::UnsupportedCurve("X".to_string());
            assert!(err.to_string().contains("X"));
        }

        #[test]
        fn test_full_lifecycle() {
            let config =
                MultiCurveConfig::new(ZKPCurve::BN254).with_aggregation(ZKPCurve::BLS12_381);
            let mut manager = MultiCurveManager::new(config);

            let params = CircuitParams::new(500, 0.7).unwrap();
            let time = params.estimated_proof_time(&ZKPCurve::BN254);
            assert!(time > 50.0);

            let mut batch = AggregationBatchV2::new("batch1".to_string(), 16);
            for i in 0..5 {
                batch
                    .add(AggregationEntryV2 {
                        proof_id: format!("p{}", i),
                        curve: ZKPCurve::BN254,
                        proof_size: 64,
                        verified: true,
                    })
                    .unwrap();
            }
            assert_eq!(batch.size(), 5);
            assert_eq!(batch.verified_count(), 5);
            assert!(!batch.is_full());

            manager.record_benchmark(CurveBenchmark {
                curve: ZKPCurve::BN254,
                proof_time_ms: time,
                verify_time_ms: 0.5,
                proof_size_bytes: 64,
                constraint_count: 500,
            });
            assert_eq!(manager.benchmark_count(), 1);
        }
    }
}

pub use internal::{
    AggregationBatchV2, AggregationEntryV2, CircuitParams, CurveBenchmark, CurveError,
    CurveProperties, MultiCurveConfig, MultiCurveManager, ZKPCurve,
};
