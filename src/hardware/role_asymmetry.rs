//! Hardware Role Asymmetry — Sprint 76: Ontological Debugging & Thermodynamic Pivots
//!
//! Resuelve el bug ontológico: LLM en navegador → OOM por límites WASM.
//!
//! Implementa asimetría de roles: WASM = SAE + Routing ligero.
//! Native (Tauri/Rust) = LLM Inference (CUDA/Metal). Separación estricta
//! de cargas para evitar colapso de memoria en entornos restringidos.
//!
//! # Garantías
//!
//! - WASM: máximo 512MB, solo SAE forward + routing
//! - Native: sin límite estricto, LLM inference con GPU
//! - Bloqueo estricto de cargas cruzadas

use std::fmt;

/// Error types for Role Asymmetry
#[derive(Debug, Clone, PartialEq)]
pub enum RoleError {
    /// Memory exceeds WASM limit
    WasmMemoryExceeded(u32),
    /// LLM inference not allowed in WASM
    LlmInWasmForbidden,
    /// SAE not allowed in native-only mode
    SaeInNativeForbidden,
    /// Insufficient memory for requested role
    InsufficientMemory { required: u32, available: u32 },
    /// GPU required but not available
    GpuRequired,
    /// Invalid node type
    InvalidNodeType(String),
    /// Role constraint violation
    ConstraintViolation(String),
}

impl fmt::Display for RoleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RoleError::WasmMemoryExceeded(mb) => {
                write!(f, "WASM memory limit exceeded: {}MB", mb)
            }
            RoleError::LlmInWasmForbidden => {
                write!(f, "LLM inference forbidden in WASM environment")
            }
            RoleError::SaeInNativeForbidden => {
                write!(f, "SAE operation forbidden in native-only mode")
            }
            RoleError::InsufficientMemory {
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient memory: {}MB required, {}MB available",
                    required, available
                )
            }
            RoleError::GpuRequired => write!(f, "GPU required for this operation"),
            RoleError::InvalidNodeType(t) => write!(f, "Invalid node type: {}", t),
            RoleError::ConstraintViolation(msg) => {
                write!(f, "Role constraint violation: {}", msg)
            }
        }
    }
}

impl std::error::Error for RoleError {}

/// Node type determining execution environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeType {
    /// WASM node (browser/edge) — lightweight SAE + routing
    Wasm,
    /// Native node (Tauri/Rust) — full LLM inference
    Native,
    /// Hybrid node — can switch based on load
    Hybrid,
}

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeType::Wasm => write!(f, "Wasm"),
            NodeType::Native => write!(f, "Native"),
            NodeType::Hybrid => write!(f, "Hybrid"),
        }
    }
}

/// Workload type to be executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Workload {
    /// SAE forward pass (lightweight)
    SaeForward,
    /// SAE training (moderate)
    SaeTraining,
    /// Token routing (lightweight)
    Routing,
    /// LLM inference (heavy, GPU preferred)
    LlmInference,
    /// LLM training (very heavy, GPU required)
    LlmTraining,
    /// Topology computation (moderate)
    Topology,
}

impl fmt::Display for Workload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Workload::SaeForward => write!(f, "SaeForward"),
            Workload::SaeTraining => write!(f, "SaeTraining"),
            Workload::Routing => write!(f, "Routing"),
            Workload::LlmInference => write!(f, "LlmInference"),
            Workload::LlmTraining => write!(f, "LlmTraining"),
            Workload::Topology => write!(f, "Topology"),
        }
    }
}

impl Workload {
    /// Estimated memory requirement in MB.
    pub fn estimated_memory_mb(&self) -> u32 {
        match self {
            Workload::SaeForward => 64,
            Workload::SaeTraining => 256,
            Workload::Routing => 32,
            Workload::LlmInference => 2048,
            Workload::LlmTraining => 8192,
            Workload::Topology => 128,
        }
    }

    /// Whether this workload requires GPU.
    pub fn requires_gpu(&self) -> bool {
        matches!(self, Workload::LlmInference | Workload::LlmTraining)
    }

    /// Whether this workload is allowed in WASM.
    pub fn allowed_in_wasm(&self) -> bool {
        matches!(
            self,
            Workload::SaeForward | Workload::Routing | Workload::Topology
        )
    }
}

/// Execution policy returned by role enforcement.
#[derive(Debug, Clone)]
pub struct ExecutionPolicy {
    /// Whether the workload is allowed.
    pub allowed: bool,
    /// Recommended node type for this workload.
    pub recommended_node: NodeType,
    /// Memory requirement in MB.
    pub memory_mb: u32,
    /// GPU required.
    pub gpu_required: bool,
    /// Reason for denial (if not allowed).
    pub denial_reason: Option<String>,
}

impl fmt::Display for ExecutionPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ExecutionPolicy {{ allowed={}, node={}, mem={}MB, gpu={} }}",
            self.allowed, self.recommended_node, self.memory_mb, self.gpu_required
        )
    }
}

/// Configuration for role asymmetry enforcement.
#[derive(Debug, Clone)]
pub struct RoleConfig {
    /// Maximum WASM memory in MB.
    pub wasm_memory_limit_mb: u32,
    /// Maximum native memory in MB (0 = unlimited).
    pub native_memory_limit_mb: u32,
    /// Enable strict mode (deny cross-role workloads).
    pub strict_mode: bool,
    /// Fallback to hybrid when possible.
    pub hybrid_fallback: bool,
}

impl RoleConfig {
    /// Default Stuartian configuration.
    pub fn default_stuartian() -> Self {
        Self {
            wasm_memory_limit_mb: 512,
            native_memory_limit_mb: 0, // Unlimited
            strict_mode: true,
            hybrid_fallback: true,
        }
    }

    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<(), RoleError> {
        if self.wasm_memory_limit_mb == 0 {
            return Err(RoleError::WasmMemoryExceeded(0));
        }
        Ok(())
    }
}

impl Default for RoleConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

/// Stateful engine for role asymmetry enforcement.
#[derive(Debug, Clone)]
pub struct RoleAsymmetry {
    config: RoleConfig,
    node_registry: Vec<NodeRecord>,
}

/// Record of a registered node.
#[derive(Debug, Clone)]
pub struct NodeRecord {
    /// Node identifier.
    pub node_id: u64,
    /// Node type.
    pub node_type: NodeType,
    /// Available memory in MB.
    pub available_memory_mb: u32,
    /// Has GPU.
    pub has_gpu: bool,
    /// Current workload (if any).
    pub current_workload: Option<Workload>,
}

impl RoleAsymmetry {
    /// Create a new engine with default Stuartian configuration.
    pub fn new() -> Self {
        Self {
            config: RoleConfig::default_stuartian(),
            node_registry: Vec::new(),
        }
    }

    /// Create a new engine with custom configuration.
    pub fn with_config(config: RoleConfig) -> Result<Self, RoleError> {
        config.validate()?;
        Ok(Self {
            config,
            node_registry: Vec::new(),
        })
    }

    /// Register a node in the system.
    pub fn register_node(
        &mut self,
        node_id: u64,
        node_type: NodeType,
        available_memory_mb: u32,
        has_gpu: bool,
    ) {
        self.node_registry.push(NodeRecord {
            node_id,
            node_type,
            available_memory_mb,
            has_gpu,
            current_workload: None,
        });
    }

    /// Enforce role constraints for a workload.
    pub fn enforce_role_constraints(
        &self,
        node_type: NodeType,
        available_memory_mb: u32,
        has_gpu: bool,
        workload: Workload,
    ) -> ExecutionPolicy {
        let memory_mb = workload.estimated_memory_mb();
        let gpu_required = workload.requires_gpu();

        // Check WASM constraints
        if node_type == NodeType::Wasm {
            if !workload.allowed_in_wasm() {
                return ExecutionPolicy {
                    allowed: false,
                    recommended_node: NodeType::Native,
                    memory_mb,
                    gpu_required,
                    denial_reason: Some(format!("{} not allowed in WASM environment", workload)),
                };
            }
            if memory_mb > self.config.wasm_memory_limit_mb {
                return ExecutionPolicy {
                    allowed: false,
                    recommended_node: NodeType::Native,
                    memory_mb,
                    gpu_required,
                    denial_reason: Some(format!(
                        "{}MB exceeds WASM limit {}MB",
                        memory_mb, self.config.wasm_memory_limit_mb
                    )),
                };
            }
        }

        // Check GPU requirement
        if gpu_required && !has_gpu {
            if self.config.strict_mode {
                return ExecutionPolicy {
                    allowed: false,
                    recommended_node: NodeType::Native,
                    memory_mb,
                    gpu_required,
                    denial_reason: Some("GPU required but not available".to_string()),
                };
            }
        }

        // Check memory availability
        if available_memory_mb > 0 && memory_mb > available_memory_mb {
            return ExecutionPolicy {
                allowed: false,
                recommended_node: NodeType::Native,
                memory_mb,
                gpu_required,
                denial_reason: Some(format!(
                    "{}MB required, {}MB available",
                    memory_mb, available_memory_mb
                )),
            };
        }

        // Determine recommended node
        let recommended_node = if workload.requires_gpu() {
            NodeType::Native
        } else if workload.allowed_in_wasm() {
            match node_type {
                NodeType::Wasm => NodeType::Wasm,
                NodeType::Hybrid => NodeType::Hybrid,
                NodeType::Native => NodeType::Native,
            }
        } else {
            NodeType::Native
        };

        ExecutionPolicy {
            allowed: true,
            recommended_node,
            memory_mb,
            gpu_required,
            denial_reason: None,
        }
    }

    /// Find the best node for a workload.
    pub fn find_best_node(&self, workload: Workload) -> Option<u64> {
        for node in &self.node_registry {
            let policy = self.enforce_role_constraints(
                node.node_type,
                node.available_memory_mb,
                node.has_gpu,
                workload,
            );
            if policy.allowed {
                return Some(node.node_id);
            }
        }
        None
    }

    /// Total registered nodes.
    pub fn total_nodes(&self) -> usize {
        self.node_registry.len()
    }

    /// Nodes by type.
    pub fn nodes_by_type(&self, node_type: NodeType) -> Vec<u64> {
        self.node_registry
            .iter()
            .filter(|n| n.node_type == node_type)
            .map(|n| n.node_id)
            .collect()
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        self.node_registry.clear();
    }
}

impl Default for RoleAsymmetry {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RoleAsymmetry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RoleAsymmetry {{ nodes={}, wasm={}, native={} }}",
            self.total_nodes(),
            self.nodes_by_type(NodeType::Wasm).len(),
            self.nodes_by_type(NodeType::Native).len()
        )
    }
}

// ─── Public Standalone Function ────────────────────────────────────────────────

/// Enforce role constraints (standalone).
pub fn enforce_role_constraints(
    node_type: NodeType,
    available_memory_mb: u32,
    has_gpu: bool,
    workload: Workload,
) -> ExecutionPolicy {
    let engine = RoleAsymmetry::new();
    engine.enforce_role_constraints(node_type, available_memory_mb, has_gpu, workload)
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = RoleConfig::default_stuartian();
        assert!(config.validate().is_ok());
        assert_eq!(config.wasm_memory_limit_mb, 512);
    }

    #[test]
    fn test_config_zero_wasm_limit() {
        let config = RoleConfig {
            wasm_memory_limit_mb: 0,
            ..RoleConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_workload_memory() {
        assert_eq!(Workload::Routing.estimated_memory_mb(), 32);
        assert_eq!(Workload::LlmInference.estimated_memory_mb(), 2048);
    }

    #[test]
    fn test_workload_gpu_required() {
        assert!(!Workload::SaeForward.requires_gpu());
        assert!(Workload::LlmInference.requires_gpu());
    }

    #[test]
    fn test_workload_wasm_allowed() {
        assert!(Workload::SaeForward.allowed_in_wasm());
        assert!(Workload::Routing.allowed_in_wasm());
        assert!(!Workload::LlmInference.allowed_in_wasm());
    }

    #[test]
    fn test_engine_creation() {
        let engine = RoleAsymmetry::new();
        assert_eq!(engine.total_nodes(), 0);
    }

    #[test]
    fn test_register_node() {
        let mut engine = RoleAsymmetry::new();
        engine.register_node(1, NodeType::Wasm, 512, false);
        assert_eq!(engine.total_nodes(), 1);
    }

    #[test]
    fn test_enforce_sae_in_wasm() {
        let engine = RoleAsymmetry::new();
        let policy =
            engine.enforce_role_constraints(NodeType::Wasm, 512, false, Workload::SaeForward);
        assert!(policy.allowed);
    }

    #[test]
    fn test_enforce_llm_in_wasm_denied() {
        let engine = RoleAsymmetry::new();
        let policy =
            engine.enforce_role_constraints(NodeType::Wasm, 512, false, Workload::LlmInference);
        assert!(!policy.allowed);
        assert!(policy.denial_reason.is_some());
    }

    #[test]
    fn test_enforce_llm_in_native_allowed() {
        let engine = RoleAsymmetry::new();
        let policy =
            engine.enforce_role_constraints(NodeType::Native, 8192, true, Workload::LlmInference);
        assert!(policy.allowed);
    }

    #[test]
    fn test_enforce_insufficient_memory() {
        let engine = RoleAsymmetry::new();
        let policy =
            engine.enforce_role_constraints(NodeType::Native, 100, false, Workload::LlmInference);
        assert!(!policy.allowed);
    }

    #[test]
    fn test_enforce_gpu_required() {
        let engine = RoleAsymmetry::new();
        let policy =
            engine.enforce_role_constraints(NodeType::Native, 8192, false, Workload::LlmInference);
        assert!(!policy.allowed);
    }

    #[test]
    fn test_find_best_node() {
        let mut engine = RoleAsymmetry::new();
        engine.register_node(1, NodeType::Wasm, 512, false);
        engine.register_node(2, NodeType::Native, 8192, true);
        let node = engine.find_best_node(Workload::LlmInference);
        assert_eq!(node, Some(2));
    }

    #[test]
    fn test_find_best_node_sae() {
        let mut engine = RoleAsymmetry::new();
        engine.register_node(1, NodeType::Wasm, 512, false);
        let node = engine.find_best_node(Workload::SaeForward);
        assert_eq!(node, Some(1));
    }

    #[test]
    fn test_find_best_node_none() {
        let mut engine = RoleAsymmetry::new();
        engine.register_node(1, NodeType::Wasm, 512, false);
        let node = engine.find_best_node(Workload::LlmInference);
        assert_eq!(node, None);
    }

    #[test]
    fn test_nodes_by_type() {
        let mut engine = RoleAsymmetry::new();
        engine.register_node(1, NodeType::Wasm, 512, false);
        engine.register_node(2, NodeType::Native, 8192, true);
        let wasm = engine.nodes_by_type(NodeType::Wasm);
        let native = engine.nodes_by_type(NodeType::Native);
        assert_eq!(wasm, vec![1]);
        assert_eq!(native, vec![2]);
    }

    #[test]
    fn test_reset() {
        let mut engine = RoleAsymmetry::new();
        engine.register_node(1, NodeType::Wasm, 512, false);
        engine.reset();
        assert_eq!(engine.total_nodes(), 0);
    }

    #[test]
    fn test_display() {
        let engine = RoleAsymmetry::new();
        let s = format!("{}", engine);
        assert!(s.contains("RoleAsymmetry"));
    }

    #[test]
    fn test_policy_display() {
        let policy = ExecutionPolicy {
            allowed: true,
            recommended_node: NodeType::Native,
            memory_mb: 2048,
            gpu_required: true,
            denial_reason: None,
        };
        let s = format!("{}", policy);
        assert!(s.contains("ExecutionPolicy"));
    }

    #[test]
    fn test_standalone_enforce() {
        let policy = enforce_role_constraints(NodeType::Wasm, 512, false, Workload::Routing);
        assert!(policy.allowed);

        let policy = enforce_role_constraints(NodeType::Wasm, 512, false, Workload::LlmInference);
        assert!(!policy.allowed);
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = RoleAsymmetry::new();

        // Register nodes
        engine.register_node(1, NodeType::Wasm, 512, false);
        engine.register_node(2, NodeType::Native, 8192, true);
        engine.register_node(3, NodeType::Hybrid, 2048, false);

        // Route SAE to WASM
        let sae_node = engine.find_best_node(Workload::SaeForward);
        assert_eq!(sae_node, Some(1));

        // Route LLM to Native
        let llm_node = engine.find_best_node(Workload::LlmInference);
        assert_eq!(llm_node, Some(2));

        // Route routing to WASM
        let route_node = engine.find_best_node(Workload::Routing);
        assert_eq!(route_node, Some(1));

        assert_eq!(engine.total_nodes(), 3);
    }

    #[test]
    fn test_error_display() {
        let err = RoleError::LlmInWasmForbidden;
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }
}
