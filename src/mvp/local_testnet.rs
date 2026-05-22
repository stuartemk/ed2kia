//! Local Testnet — Orquestador de 3 nodos para MVP local.
//!
//! Instancia 3 nodos en memoria con simulación de GossipSub.
//! Soporta `--dry-run` para CI (sin binding de red real).
//!
//! Ley 3 (Cero desperdicio): Simulación en memoria, cero overhead de red.

use std::time::Instant;
use thiserror::Error;

use crate::mvp::consensus_runner::{ConsensusError, ConsensusMetrics, ConsensusRunner};
use crate::mvp::sae_simulator::{NodeProfile, SaePayload, SaeSimError, SaeSimulator};

/// Error del testnet local.
#[derive(Debug, Error)]
pub enum MvpError {
    #[error("Consensus error: {0}")]
    Consensus(#[from] ConsensusError),

    #[error("SAE simulator error: {0}")]
    SaeSim(#[from] SaeSimError),

    #[error("Node {node_id} failed to initialize")]
    NodeInitFailed { node_id: String },

    #[error("Payload injection failed for node {node_id}")]
    InjectionFailed { node_id: String },

    #[error("Simulation timeout after {elapsed_ms:.0}ms")]
    Timeout { elapsed_ms: f64 },
}

/// Estado de un nodo en el cluster MVP.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeState {
    /// Nodo inicializado pero sin conectar.
    Initialized,
    /// Nodo conectado al mesh.
    Connected,
    /// Nodo activo, procesando payloads.
    Active,
    /// Nodo rechazado por SCT.
    Slashed,
}

impl std::fmt::Display for NodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeState::Initialized => write!(f, "Initialized"),
            NodeState::Connected => write!(f, "Connected"),
            NodeState::Active => write!(f, "Active"),
            NodeState::Slashed => write!(f, "Slashed"),
        }
    }
}

/// Nodo MVP simulado.
#[derive(Debug, Clone)]
pub struct MvpNode {
    /// Identificador único.
    pub id: String,
    /// Multiaddr (para info).
    pub address: String,
    /// Estado actual.
    pub state: NodeState,
    /// Perfil del nodo.
    pub profile: NodeProfile,
    /// Payloads generados.
    pub payloads: Vec<SaePayload>,
}

impl MvpNode {
    pub fn new(id: &str, address: &str, profile: NodeProfile) -> Self {
        Self {
            id: id.to_string(),
            address: address.to_string(),
            state: NodeState::Initialized,
            profile,
            payloads: Vec::new(),
        }
    }

    pub fn connect(&mut self) {
        self.state = NodeState::Connected;
    }

    pub fn activate(&mut self) {
        self.state = NodeState::Active;
    }

    pub fn slash(&mut self) {
        self.state = NodeState::Slashed;
    }
}

/// Resultado completo de la simulación MVP.
#[derive(Debug)]
pub struct MvpResult {
    /// ¿Fue dry-run?
    pub dry_run: bool,
    /// Nodos en el cluster.
    pub nodes: Vec<MvpNode>,
    /// Métricas de consenso.
    pub consensus_metrics: ConsensusMetrics,
    /// Duración total en ms.
    pub total_duration_ms: f64,
    /// ¿Simulación exitosa?
    pub success: bool,
    /// Timestamp ISO 8601.
    pub timestamp: String,
}

/// Orquestador del cluster MVP local.
pub struct LocalTestnet {
    /// Nodos del cluster.
    nodes: Vec<MvpNode>,
    /// Simulador SAE.
    simulator: SaeSimulator,
    /// Ejecutor de consenso.
    consensus: ConsensusRunner,
    /// Modo dry-run.
    dry_run: bool,
    /// Topic GossipSub simulado.
    topic: String,
}

impl LocalTestnet {
    /// Construye un nuevo LocalTestnet.
    pub fn new(dry_run: bool) -> Self {
        Self {
            nodes: Vec::new(),
            simulator: SaeSimulator::default(),
            consensus: ConsensusRunner::default(),
            dry_run,
            topic: "ed2kia/mvp-payloads".to_string(),
        }
    }

    /// Añade un nodo al cluster.
    pub fn add_node(&mut self, id: &str, address: &str, profile: NodeProfile) {
        self.nodes.push(MvpNode::new(id, address, profile));
    }

    /// Configura el cluster estándar de 3 nodos:
    /// - Alpha (simbiótico) @ 127.0.0.1:8001
    /// - Beta (perverso) @ 127.0.0.1:8002
    /// - Gamma (steward) @ 127.0.0.1:8003
    pub fn setup_default_cluster(&mut self) {
        self.add_node("alpha", "/ip4/127.0.0.1/tcp/8001", NodeProfile::Symbiotic);
        self.add_node("beta", "/ip4/127.0.0.1/tcp/8002", NodeProfile::Perverse);
        self.add_node("gamma", "/ip4/127.0.0.1/tcp/8003", NodeProfile::Symbiotic);
    }

    /// Ejecuta la simulación MVP completa.
    pub async fn run(mut self) -> Result<MvpResult, MvpError> {
        let start = Instant::now();

        println!("╔══════════════════════════════════════════════════════════╗");
        println!("║          ed2kIA MVP — End-to-End Local Simulation       ║");
        println!("╚══════════════════════════════════════════════════════════╝");
        println!();

        // Phase 1: Initialize nodes
        println!("[Phase 1] Initializing {} nodes...", self.nodes.len());
        let mut nodes = self.nodes.clone();

        if self.dry_run {
            println!("  ⚡ DRY-RUN MODE: Simulating in-memory events (no network binding)");
        }

        for node in &mut nodes {
            println!(
                "  [+] Node '{}' @ {} — Profile: {}, State: {}",
                node.id, node.address, node.profile, node.state
            );
        }

        // Phase 2: Connect nodes (simulate GossipSub mesh)
        println!("\n[Phase 2] Connecting nodes to mesh...");
        println!("  Topic: {}", self.topic);

        for node in &mut nodes {
            node.connect();
            if self.dry_run {
                println!(
                    "  [DRY-RUN] Simulating ConnectionEstablished for '{}'",
                    node.id
                );
            }
            println!("  [✓] Node '{}' connected — State: {}", node.id, node.state);
        }

        // Phase 3: Generate and inject payloads
        println!("\n[Phase 3] Generating SAE payloads...");
        let mut all_payloads: Vec<SaePayload> = Vec::new();

        for node in &mut nodes {
            let payload = if node.profile == NodeProfile::Symbiotic {
                self.simulator
                    .generate_symbiotic(&node.id)
                    .map_err(MvpError::SaeSim)?
            } else {
                self.simulator
                    .generate_perverse(&node.id)
                    .map_err(MvpError::SaeSim)?
            };

            println!(
                "  [{}] Node '{}' generated payload: {} gradient values, expected Z={:+.1}",
                if self.dry_run { "DRY-RUN" } else { "LIVE" },
                node.id,
                payload.gradient.len(),
                payload.expected_z
            );

            node.payloads.push(payload.clone());
            all_payloads.push(payload);
        }

        // Phase 4: Activate nodes
        println!("\n[Phase 4] Activating nodes...");
        for node in &mut nodes {
            node.activate();
            println!("  [✓] Node '{}' active — State: {}", node.id, node.state);
        }

        // Phase 5: Consensus (SCT + BFT)
        println!("\n[Phase 5] Running consensus (SCT Guard + BFT Aggregator)...");
        println!("─────────────────────────────────────────────────────────");

        let metrics = self.consensus.run_consensus(&all_payloads)?;

        println!("─────────────────────────────────────────────────────────");

        // Update node states based on SCT verdicts
        for eval in &metrics.evaluations {
            if let Some(node) = nodes.iter_mut().find(|n| n.id == eval.node_id) {
                if !eval.approved {
                    node.slash();
                    println!("  [⚠] Node '{}' slashed — State: {}", node.id, node.state);
                }
            }
        }

        let total_duration_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Generate timestamp
        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false);

        let success = metrics.approved_count > 0
            && metrics.rejected_count > 0
            && metrics.bft_result.is_some()
            && total_duration_ms < 5000.0;

        println!();
        println!("╔══════════════════════════════════════════════════════════╗");
        println!("║                    MVP RESULTS                          ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!(
            "║ Total Payloads:    {:<30} ║",
            format!("{} payloads", metrics.total_payloads)
        );
        println!(
            "║ Approved:          {:<30} ║",
            format!("{} (Symbiotic)", metrics.approved_count)
        );
        println!(
            "║ Rejected:          {:<30} ║",
            format!("{} (Perverse)", metrics.rejected_count)
        );
        println!(
            "║ BFT Converged:     {:<30} ║",
            if metrics.bft_result.is_some() {
                "YES"
            } else {
                "NO"
            }
        );
        println!(
            "║ Total Latency:     {:<30} ║",
            format!("{:.1}ms", total_duration_ms)
        );
        println!(
            "║ Status:            {:<30} ║",
            if success { "✅ PASS" } else { "❌ FAIL" }
        );
        println!("╚══════════════════════════════════════════════════════════╝");

        Ok(MvpResult {
            dry_run: self.dry_run,
            nodes,
            consensus_metrics: metrics,
            total_duration_ms,
            success,
            timestamp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_testnet_creation() {
        let testnet = LocalTestnet::new(true);
        assert!(testnet.dry_run);
        assert_eq!(testnet.topic, "ed2kia/mvp-payloads");
    }

    #[tokio::test]
    async fn test_setup_default_cluster() {
        let mut testnet = LocalTestnet::new(true);
        testnet.setup_default_cluster();
        assert_eq!(testnet.nodes.len(), 3);
        assert_eq!(testnet.nodes[0].id, "alpha");
        assert_eq!(testnet.nodes[1].id, "beta");
        assert_eq!(testnet.nodes[2].id, "gamma");
    }

    #[tokio::test]
    async fn test_node_lifecycle() {
        let mut node = MvpNode::new("test", "/ip4/127.0.0.1/tcp/9001", NodeProfile::Symbiotic);
        assert_eq!(node.state, NodeState::Initialized);
        node.connect();
        assert_eq!(node.state, NodeState::Connected);
        node.activate();
        assert_eq!(node.state, NodeState::Active);
        node.slash();
        assert_eq!(node.state, NodeState::Slashed);
    }

    #[tokio::test]
    async fn test_full_dry_run() {
        let mut testnet = LocalTestnet::new(true);
        testnet.setup_default_cluster();
        let result = testnet.run().await.unwrap();
        assert!(result.dry_run);
        assert_eq!(result.nodes.len(), 3);
        assert!(result.success);
        assert!(result.total_duration_ms < 5000.0);
    }

    #[test]
    fn test_node_state_display() {
        assert_eq!(format!("{}", NodeState::Initialized), "Initialized");
        assert_eq!(format!("{}", NodeState::Connected), "Connected");
        assert_eq!(format!("{}", NodeState::Active), "Active");
        assert_eq!(format!("{}", NodeState::Slashed), "Slashed");
    }
}
