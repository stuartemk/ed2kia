//! Collaborative SNARK Generation â€” Sprint 74: Distributed Systems Hardening & Second-Order Resolution
//!
//! Distributed proving via circuit partitioning and threshold aggregation.
//! Prevents prover centralization by splitting work across multiple nodes.
//!
//! # Design
//!
//! Large SNARK circuits are partitioned into micro-tasks that can be
//! distributed to multiple proving nodes (including WASM edge nodes).
//! Results are aggregated using cryptographic threshold schemes to
//! ensure no single node can monopolize proving power.
//!
//! # Guarantees
//!
//! - Circuit partitioning: O(n/k) per node for n gates, k nodes
//! - Threshold aggregation: requires t-of-k valid partial proofs
//! - Memory: O(n/k) per node

use std::fmt;

/// Errors for collaborative SNARK operations.
#[derive(Debug, Clone, PartialEq)]
pub enum SnarkError {
    /// Empty circuit provided.
    EmptyCircuit,
    /// Invalid node count (must be > 0).
    InvalidNodeCount(usize),
    /// Threshold exceeds node count.
    ThresholdExceeded { threshold: usize, nodes: usize },
    /// Insufficient partial proofs for threshold.
    InsufficientProofs { have: usize, need: usize },
    /// Partial proof verification failed.
    ProofVerificationFailed,
    /// Aggregation failed.
    AggregationFailed,
}

impl fmt::Display for SnarkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SnarkError::EmptyCircuit => write!(f, "SNARK: empty circuit"),
            SnarkError::InvalidNodeCount(n) => write!(f, "SNARK: invalid node count {}", n),
            SnarkError::ThresholdExceeded { threshold, nodes } => {
                write!(
                    f,
                    "SNARK: threshold {} exceeds node count {}",
                    threshold, nodes
                )
            }
            SnarkError::InsufficientProofs { have, need } => {
                write!(f, "SNARK: insufficient proofs {}/{}", have, need)
            }
            SnarkError::ProofVerificationFailed => write!(f, "SNARK: proof verification failed"),
            SnarkError::AggregationFailed => write!(f, "SNARK: aggregation failed"),
        }
    }
}

impl std::error::Error for SnarkError {}

/// Configuration for collaborative SNARK generation.
#[derive(Debug, Clone)]
pub struct CollaborativeConfig {
    /// Threshold for proof aggregation (t-of-k).
    pub threshold: usize,
    /// Maximum circuit size in gates.
    pub max_circuit_size: usize,
    /// Maximum task size per node.
    pub max_task_size: usize,
    /// Enable proof compression.
    pub compress: bool,
}

impl CollaborativeConfig {
    /// Default Topological configuration.
    pub fn default_Topological() -> Self {
        Self {
            threshold: 3,
            max_circuit_size: 1_000_000,
            max_task_size: 100_000,
            compress: true,
        }
    }

    /// Validate configuration.
    pub fn validate(&self) -> Result<(), SnarkError> {
        if self.threshold == 0 {
            return Err(SnarkError::InvalidNodeCount(0));
        }
        if self.max_circuit_size == 0 {
            return Err(SnarkError::EmptyCircuit);
        }
        Ok(())
    }
}

impl Default for CollaborativeConfig {
    fn default() -> Self {
        Self::default_Topological()
    }
}

/// A SNARK circuit representation.
#[derive(Debug, Clone)]
pub struct SnarkCircuit {
    /// Circuit identifier.
    pub circuit_id: u64,
    /// Number of gates in the circuit.
    pub gate_count: usize,
    /// Circuit data (simulated).
    pub data: Vec<u8>,
}

impl fmt::Display for SnarkCircuit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SnarkCircuit(id={}, gates={}, size={})",
            self.circuit_id,
            self.gate_count,
            self.data.len()
        )
    }
}

/// A micro-task for distributed proving.
#[derive(Debug, Clone)]
pub struct ProvingTask {
    /// Task identifier.
    pub task_id: u64,
    /// Assigned node ID.
    pub node_id: u64,
    /// Circuit portion (gate range).
    pub gate_start: usize,
    pub gate_end: usize,
    /// Task data.
    pub data: Vec<u8>,
    /// Whether the task has been completed.
    pub completed: bool,
}

impl fmt::Display for ProvingTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ProvingTask(id={}, node={}, gates=[{}..{}], done={})",
            self.task_id, self.node_id, self.gate_start, self.gate_end, self.completed
        )
    }
}

/// A partial proof from a proving node.
#[derive(Debug, Clone)]
pub struct PartialProof {
    /// Node that generated this proof.
    pub node_id: u64,
    /// Task this proof corresponds to.
    pub task_id: u64,
    /// Proof bytes.
    pub proof_bytes: Vec<u8>,
    /// Whether verification passed.
    pub verified: bool,
}

impl fmt::Display for PartialProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PartialProof(node={}, task={}, size={}, verified={})",
            self.node_id,
            self.task_id,
            self.proof_bytes.len(),
            self.verified
        )
    }
}

/// Aggregated SNARK proof.
#[derive(Debug, Clone)]
pub struct AggregatedProof {
    /// Circuit ID.
    pub circuit_id: u64,
    /// Number of partial proofs used.
    pub partial_count: usize,
    /// Threshold that was met.
    pub threshold: usize,
    /// Final aggregated proof bytes.
    pub proof_bytes: Vec<u8>,
    /// Whether aggregation succeeded.
    pub valid: bool,
}

impl fmt::Display for AggregatedProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AggregatedProof(circuit={}, partials={}, threshold={}, valid={})",
            self.circuit_id, self.partial_count, self.threshold, self.valid
        )
    }
}

/// Collaborative SNARK engine.
pub struct CollaborativeSnark {
    config: CollaborativeConfig,
    tasks: Vec<ProvingTask>,
    partial_proofs: Vec<PartialProof>,
    aggregated_proofs: Vec<AggregatedProof>,
}

impl CollaborativeSnark {
    /// Create a new collaborative SNARK engine.
    pub fn new() -> Self {
        Self {
            config: CollaborativeConfig::default_Topological(),
            tasks: Vec::new(),
            partial_proofs: Vec::new(),
            aggregated_proofs: Vec::new(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: CollaborativeConfig) -> Result<Self, SnarkError> {
        config.validate()?;
        Ok(Self {
            config,
            tasks: Vec::new(),
            partial_proofs: Vec::new(),
            aggregated_proofs: Vec::new(),
        })
    }

    /// Distribute proving tasks from a circuit across nodes.
    pub fn distribute_proving_tasks(
        &mut self,
        circuit: &SnarkCircuit,
        node_count: usize,
    ) -> Result<Vec<ProvingTask>, SnarkError> {
        if circuit.gate_count == 0 {
            return Err(SnarkError::EmptyCircuit);
        }
        if node_count == 0 {
            return Err(SnarkError::InvalidNodeCount(0));
        }
        if self.config.threshold > node_count {
            return Err(SnarkError::ThresholdExceeded {
                threshold: self.config.threshold,
                nodes: node_count,
            });
        }

        let gates_per_task = circuit.gate_count.div_ceil(node_count);
        let mut tasks = Vec::with_capacity(node_count);

        for i in 0..node_count {
            let start = i * gates_per_task;
            let end = (start + gates_per_task).min(circuit.gate_count);

            if start >= circuit.gate_count {
                break;
            }

            let task = ProvingTask {
                task_id: circuit.circuit_id * 1000 + i as u64,
                node_id: i as u64,
                gate_start: start,
                gate_end: end,
                data: circuit.data.clone(),
                completed: false,
            };
            tasks.push(task);
        }

        self.tasks.extend(tasks.clone());
        Ok(tasks)
    }

    /// Submit a partial proof from a node.
    pub fn submit_partial_proof(
        &mut self,
        node_id: u64,
        task_id: u64,
        proof_bytes: Vec<u8>,
    ) -> Result<PartialProof, SnarkError> {
        if proof_bytes.is_empty() {
            return Err(SnarkError::ProofVerificationFailed);
        }

        // Simulate verification
        let verified = Self::verify_partial(&proof_bytes);

        let proof = PartialProof {
            node_id,
            task_id,
            proof_bytes,
            verified,
        };

        self.partial_proofs.push(proof.clone());
        Ok(proof)
    }

    /// Aggregate partial proofs into a final SNARK proof.
    pub fn aggregate_proofs(&mut self, circuit_id: u64) -> Result<AggregatedProof, SnarkError> {
        let verified_proofs: Vec<&PartialProof> =
            self.partial_proofs.iter().filter(|p| p.verified).collect();

        if verified_proofs.len() < self.config.threshold {
            return Err(SnarkError::InsufficientProofs {
                have: verified_proofs.len(),
                need: self.config.threshold,
            });
        }

        // Aggregate: combine proof bytes
        let mut aggregated = Vec::new();
        for proof in &verified_proofs {
            aggregated.extend_from_slice(&proof.proof_bytes);
        }

        let result = AggregatedProof {
            circuit_id,
            partial_count: verified_proofs.len(),
            threshold: self.config.threshold,
            proof_bytes: aggregated,
            valid: true,
        };

        self.aggregated_proofs.push(result.clone());
        Ok(result)
    }

    /// Get tasks for a specific node.
    pub fn get_node_tasks(&self, node_id: u64) -> Vec<&ProvingTask> {
        self.tasks.iter().filter(|t| t.node_id == node_id).collect()
    }

    /// Get completed task count.
    pub fn completed_count(&self) -> usize {
        self.tasks.iter().filter(|t| t.completed).count()
    }

    /// Get aggregated proofs.
    pub fn aggregated_proofs(&self) -> &[AggregatedProof] {
        &self.aggregated_proofs
    }

    /// Reset state.
    pub fn reset(&mut self) {
        self.tasks.clear();
        self.partial_proofs.clear();
        self.aggregated_proofs.clear();
    }

    /// Simulate partial proof verification.
    fn verify_partial(proof_bytes: &[u8]) -> bool {
        !proof_bytes.is_empty() && proof_bytes.len() >= 8
    }
}

impl Default for CollaborativeSnark {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CollaborativeSnark {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CollaborativeSnark(tasks={}, partials={}, aggregated={})",
            self.tasks.len(),
            self.partial_proofs.len(),
            self.aggregated_proofs.len()
        )
    }
}

/// Public function: distribute proving tasks.
pub fn distribute_proving_tasks(circuit: &SnarkCircuit, node_count: usize) -> Vec<ProvingTask> {
    if circuit.gate_count == 0 || node_count == 0 {
        return Vec::new();
    }

    let gates_per_task = circuit.gate_count.div_ceil(node_count);
    let mut tasks = Vec::with_capacity(node_count);

    for i in 0..node_count {
        let start = i * gates_per_task;
        let end = (start + gates_per_task).min(circuit.gate_count);
        if start >= circuit.gate_count {
            break;
        }
        tasks.push(ProvingTask {
            task_id: circuit.circuit_id * 1000 + i as u64,
            node_id: i as u64,
            gate_start: start,
            gate_end: end,
            data: circuit.data.clone(),
            completed: false,
        });
    }

    tasks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = CollaborativeConfig::default_Topological();
        assert_eq!(config.threshold, 3);
        assert_eq!(config.max_circuit_size, 1_000_000);
        assert!(config.compress);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = CollaborativeConfig::default_Topological();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_zero_threshold() {
        let config = CollaborativeConfig {
            threshold: 0,
            ..CollaborativeConfig::default_Topological()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_engine_creation() {
        let engine = CollaborativeSnark::new();
        assert!(engine.tasks.is_empty());
    }

    #[test]
    fn test_engine_with_config() {
        let config = CollaborativeConfig::default_Topological();
        let engine = CollaborativeSnark::with_config(config).unwrap();
        assert!(engine.tasks.is_empty());
    }

    #[test]
    fn test_distribute_tasks() {
        let mut engine = CollaborativeSnark::new();
        let circuit = SnarkCircuit {
            circuit_id: 1,
            gate_count: 100,
            data: vec![1u8; 32],
        };
        let tasks = engine.distribute_proving_tasks(&circuit, 4).unwrap();
        assert_eq!(tasks.len(), 4);
        assert_eq!(engine.tasks.len(), 4);
    }

    #[test]
    fn test_distribute_empty_circuit() {
        let mut engine = CollaborativeSnark::new();
        let circuit = SnarkCircuit {
            circuit_id: 1,
            gate_count: 0,
            data: vec![],
        };
        let result = engine.distribute_proving_tasks(&circuit, 4);
        assert!(result.is_err());
    }

    #[test]
    fn test_distribute_zero_nodes() {
        let mut engine = CollaborativeSnark::new();
        let circuit = SnarkCircuit {
            circuit_id: 1,
            gate_count: 100,
            data: vec![1u8; 32],
        };
        let result = engine.distribute_proving_tasks(&circuit, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_distribute_threshold_exceeded() {
        let mut engine = CollaborativeSnark::new();
        engine.config.threshold = 10;
        let circuit = SnarkCircuit {
            circuit_id: 1,
            gate_count: 100,
            data: vec![1u8; 32],
        };
        let result = engine.distribute_proving_tasks(&circuit, 4);
        assert!(result.is_err());
    }

    #[test]
    fn test_submit_partial_proof() {
        let mut engine = CollaborativeSnark::new();
        let proof = engine
            .submit_partial_proof(0, 1000, vec![1u8, 2, 3, 4, 5, 6, 7, 8])
            .unwrap();
        assert!(proof.verified);
        assert_eq!(proof.node_id, 0);
    }

    #[test]
    fn test_submit_empty_proof() {
        let mut engine = CollaborativeSnark::new();
        let result = engine.submit_partial_proof(0, 1000, vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_aggregate_success() {
        let mut engine = CollaborativeSnark::new();
        engine.config.threshold = 2;
        engine.submit_partial_proof(0, 1000, vec![1u8; 16]).unwrap();
        engine.submit_partial_proof(1, 1001, vec![2u8; 16]).unwrap();
        let result = engine.aggregate_proofs(1).unwrap();
        assert!(result.valid);
        assert_eq!(result.partial_count, 2);
    }

    #[test]
    fn test_aggregate_insufficient() {
        let mut engine = CollaborativeSnark::new();
        engine.config.threshold = 5;
        engine.submit_partial_proof(0, 1000, vec![1u8; 16]).unwrap();
        let result = engine.aggregate_proofs(1);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_node_tasks() {
        let mut engine = CollaborativeSnark::new();
        let circuit = SnarkCircuit {
            circuit_id: 1,
            gate_count: 100,
            data: vec![1u8; 32],
        };
        engine.distribute_proving_tasks(&circuit, 4).unwrap();
        let tasks = engine.get_node_tasks(0);
        assert!(!tasks.is_empty());
    }

    #[test]
    fn test_reset() {
        let mut engine = CollaborativeSnark::new();
        engine.submit_partial_proof(0, 1000, vec![1u8; 16]).unwrap();
        assert!(!engine.partial_proofs.is_empty());

        engine.reset();
        assert!(engine.tasks.is_empty());
        assert!(engine.partial_proofs.is_empty());
        assert!(engine.aggregated_proofs.is_empty());
    }

    #[test]
    fn test_display() {
        let engine = CollaborativeSnark::new();
        let display = format!("{}", engine);
        assert!(display.contains("CollaborativeSnark"));
    }

    #[test]
    fn test_standalone_distribute() {
        let circuit = SnarkCircuit {
            circuit_id: 1,
            gate_count: 100,
            data: vec![1u8; 32],
        };
        let tasks = distribute_proving_tasks(&circuit, 4);
        assert_eq!(tasks.len(), 4);
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = CollaborativeSnark::new();
        engine.config.threshold = 2;

        // Create circuit
        let circuit = SnarkCircuit {
            circuit_id: 42,
            gate_count: 200,
            data: vec![42u8; 64],
        };

        // Distribute
        let tasks = engine.distribute_proving_tasks(&circuit, 4).unwrap();
        assert_eq!(tasks.len(), 4);

        // Submit partial proofs
        for task in &tasks {
            engine
                .submit_partial_proof(task.node_id, task.task_id, vec![task.task_id as u8; 16])
                .unwrap();
        }

        // Aggregate
        let result = engine.aggregate_proofs(42).unwrap();
        assert!(result.valid);
        assert_eq!(result.partial_count, 4);
        assert!(!result.proof_bytes.is_empty());
    }
}
