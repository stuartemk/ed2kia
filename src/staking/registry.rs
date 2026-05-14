//! Resource Registry - Registro de compromisos de recursos para staking
//!
//! Mantiene el registro de recursos comprometidos por cada nodo
//! en la red ed2kIA para gobernanza y asignación de trabajo.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

use super::proof::{StakingProof, ProofVerifier, VerificationResult};

/// Recursos comprometidos por un nodo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCommitment {
    /// ID del nodo
    pub node_id: String,
    /// CPU cores comprometidos
    pub cpu_cores: u32,
    /// RAM comprometida (GB)
    pub ram_gb: f64,    /// GPU disponible (true/false)
    pub has_gpu: bool,
    /// Ancho de banda comprometido (Mbps)
    pub bandwidth_mbps: f64,
    /// Almacenamiento para modelos SAE (GB)
    pub storage_gb: f64,
    /// Timestamp de registro (Unix ms)
    pub registered_at: u64,
    /// Último heartbeat (Unix ms)
    pub last_heartbeat: u64,
    /// Pruebas verificadas exitosamente
    pub proofs_verified: usize,
    /// Score de reputación (0.0 - 1.0)
    pub reputation_score: f64,
    /// Estado del nodo
    pub status: NodeStatus,
}

/// Estado del nodo
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeStatus {
    /// Registrado y activo
    Active,
    /// Inactivo (sin heartbeat reciente)
    Inactive,
    /// Sancionado (pruebas inválidas)
    Slashed,
    /// Descartado voluntariamente
    Unregistered,
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeStatus::Active => write!(f, "active"),
            NodeStatus::Inactive => write!(f, "inactive"),
            NodeStatus::Slashed => write!(f, "slashed"),
            NodeStatus::Unregistered => write!(f, "unregistered"),
        }
    }
}

impl ResourceCommitment {
    pub fn new(
        node_id: String,
        cpu_cores: u32,
        ram_gb: f64,
        has_gpu: bool,
        bandwidth_mbps: f64,
        storage_gb: f64,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            node_id,
            cpu_cores,
            ram_gb,
            has_gpu,
            bandwidth_mbps,
            storage_gb,
            registered_at: now,
            last_heartbeat: now,
            proofs_verified: 0,
            reputation_score: 1.0,
            status: NodeStatus::Active,
        }
    }

    /// Calcular score de recursos (para asignación ponderada)
    pub fn resource_score(&self) -> f64 {
        let cpu_score = self.cpu_cores as f64 * 10.0;
        let ram_score = self.ram_gb * 5.0;
        let gpu_bonus = if self.has_gpu { 100.0 } else { 0.0 };
        let bandwidth_score = self.bandwidth_mbps * 0.1;
        let storage_score = self.storage_gb * 0.5;

        (cpu_score + ram_score + gpu_bonus + bandwidth_score + storage_score)
            * self.reputation_score
    }

    /// Actualizar heartbeat
    pub fn heartbeat(&mut self) {
        self.last_heartbeat = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        if self.status == NodeStatus::Inactive {
            self.status = NodeStatus::Active;
            info!("Nodo {} reactivado por heartbeat", self.node_id);
        }
    }

    /// Verificar si el heartbeat está expirado
    pub fn is_heartbeat_expired(&self, max_age_seconds: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        (now - self.last_heartbeat) / 1000 > max_age_seconds
    }
}

/// Registro de recursos de la red
pub struct ResourceRegistry {
    /// Compromisos por nodo
    commitments: HashMap<String, ResourceCommitment>,
    /// Verificador de pruebas
    verifier: ProofVerifier,
    /// Edad máxima de heartbeat (segundos)
    max_heartbeat_age: u64,
    /// Umbral de sanciones (pruebas inválidas consecutivas)
    slash_threshold: usize,
}

impl ResourceRegistry {
    pub fn new(max_heartbeat_age: u64, slash_threshold: usize) -> Self {
        Self {
            commitments: HashMap::new(),
            verifier: ProofVerifier::with_defaults(),
            max_heartbeat_age,
            slash_threshold,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(60, 3) // 60s heartbeat, 3 strikes
    }

    /// Registrar nuevo nodo
    pub fn register(&mut self, commitment: ResourceCommitment) -> Result<()> {
        if self.commitments.contains_key(&commitment.node_id) {
            return Err(anyhow::anyhow!(
                "Node {} already registered",
                commitment.node_id
            ));
        }

        info!(
            "Nodo registrado: {} (CPU={}, RAM={}GB, GPU={})",
            commitment.node_id,
            commitment.cpu_cores,
            commitment.ram_gb,
            commitment.has_gpu
        );

        self.commitments.insert(commitment.node_id.clone(), commitment);
        Ok(())
    }

    /// Procesar heartbeat de nodo
    pub fn process_heartbeat(&mut self, node_id: &str) -> Result<()> {
        let commitment = self.commitments
            .get_mut(node_id)
            .with_context(|| format!("Node {} not registered", node_id))?;

        commitment.heartbeat();
        debug!("Heartbeat procesado: {}", node_id);
        Ok(())
    }

    /// Verificar prueba de staking
    pub fn verify_proof(&mut self, proof: &StakingProof) -> Result<VerificationResult> {
        let result = self.verifier.verify(proof)?;

        if result.valid {
            if let Some(commitment) = self.commitments.get_mut(&proof.node_id) {
                commitment.proofs_verified += 1;
                commitment.reputation_score = (commitment.reputation_score + 0.01).min(1.0);
            }
        }

        Ok(result)
    }

    /// Marcar nodo como inactivo si heartbeat expirado
    pub fn check_expired_nodes(&mut self) -> Vec<String> {
        let mut expired = Vec::new();

        for (node_id, commitment) in self.commitments.iter_mut() {
            if commitment.is_heartbeat_expired(self.max_heartbeat_age)
                && commitment.status == NodeStatus::Active
            {
                commitment.status = NodeStatus::Inactive;
                warn!("Nodo {} marcado como inactivo", node_id);
                expired.push(node_id.clone());
            }
        }

        expired
    }

    /// Sancionar nodo (slashing)
    pub fn slash_node(&mut self, node_id: &str, reason: &str) -> Result<()> {
        let commitment = self.commitments
            .get_mut(node_id)
            .with_context(|| format!("Node {} not registered", node_id))?;

        commitment.status = NodeStatus::Slashed;
        commitment.reputation_score = 0.0;

        warn!("Nodo {} sancionado: {}", node_id, reason);
        Ok(())
    }

    /// Desregistrar nodo
    pub fn unregister(&mut self, node_id: &str) -> Result<()> {
        let commitment = self.commitments
            .get_mut(node_id)
            .with_context(|| format!("Node {} not registered", node_id))?;

        commitment.status = NodeStatus::Unregistered;
        info!("Nodo {} desregistrado", node_id);
        Ok(())
    }

    /// Obtener compromiso de nodo
    pub fn get_commitment(&self, node_id: &str) -> Option<&ResourceCommitment> {
        self.commitments.get(node_id)
    }

    /// Obtener nodos activos ordenados por score
    pub fn get_active_nodes(&self) -> Vec<&ResourceCommitment> {
        let mut active: Vec<_> = self.commitments
            .values()
            .filter(|c| c.status == NodeStatus::Active)
            .collect();

        active.sort_by(|a, b| {
            b.resource_score()
                .partial_cmp(&a.resource_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        active
    }

    /// Estadísticas del registro
    pub fn stats(&self) -> RegistryStats {
        let all: Vec<_> = self.commitments.values().collect();
        RegistryStats {
            total_nodes: all.len(),
            active_nodes: all.iter().filter(|c| c.status == NodeStatus::Active).count(),
            inactive_nodes: all.iter().filter(|c| c.status == NodeStatus::Inactive).count(),
            slashed_nodes: all.iter().filter(|c| c.status == NodeStatus::Slashed).count(),
            total_cpu_cores: all.iter().map(|c| c.cpu_cores).sum(),
            total_ram_gb: all.iter().map(|c| c.ram_gb).sum(),
            gpu_nodes: all.iter().filter(|c| c.has_gpu).count(),
            total_proofs_verified: all.iter().map(|c| c.proofs_verified).sum(),
        }
    }
}

/// Estadísticas del registro
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStats {
    pub total_nodes: usize,
    pub active_nodes: usize,
    pub inactive_nodes: usize,
    pub slashed_nodes: usize,
    pub total_cpu_cores: u32,
    pub total_ram_gb: f64,
    pub gpu_nodes: usize,
    pub total_proofs_verified: usize,
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::staking::proof::{ProofGenerator, ComputeMetrics};

    #[test]
    fn test_node_registration() {
        let mut registry = ResourceRegistry::with_defaults();
        let commitment = ResourceCommitment::new(
            "node1".to_string(),
            8,
            32.0,
            true,
            1000.0,
            500.0,
        );

        registry.register(commitment).unwrap();
        assert!(registry.get_commitment("node1").is_some());
    }

    #[test]
    fn test_duplicate_registration() {
        let mut registry = ResourceRegistry::with_defaults();
        let c1 = ResourceCommitment::new("node1".to_string(), 4, 16.0, false, 100.0, 100.0);
        let c2 = ResourceCommitment::new("node1".to_string(), 4, 16.0, false, 100.0, 100.0);

        registry.register(c1).unwrap();
        let err = registry.register(c2).unwrap_err();
        assert!(err.to_string().contains("already registered"));
    }

    #[test]
    fn test_heartbeat() {
        let mut registry = ResourceRegistry::with_defaults();
        let commitment = ResourceCommitment::new("node1".to_string(), 4, 16.0, false, 100.0, 100.0);
        registry.register(commitment).unwrap();

        registry.process_heartbeat("node1").unwrap();
        let c = registry.get_commitment("node1").unwrap();
        assert!(c.last_heartbeat >= c.registered_at);
    }

    #[test]
    fn test_resource_score() {
        let commitment = ResourceCommitment::new("node1".to_string(), 8, 32.0, true, 1000.0, 500.0);
        let score = commitment.resource_score();
        assert!(score > 0.0);
        // GPU bonus should make this high
        assert!(score > 100.0);
    }

    #[test]
    fn test_slash_node() {
        let mut registry = ResourceRegistry::with_defaults();
        let commitment = ResourceCommitment::new("bad_node".to_string(), 4, 16.0, false, 100.0, 100.0);
        registry.register(commitment).unwrap();

        registry.slash_node("bad_node", "Invalid proofs").unwrap();
        let c = registry.get_commitment("bad_node").unwrap();
        assert_eq!(c.status, NodeStatus::Slashed);
        assert_eq!(c.reputation_score, 0.0);
    }

    #[test]
    fn test_registry_stats() {
        let mut registry = ResourceRegistry::with_defaults();

        for i in 0..5 {
            let commitment = ResourceCommitment::new(
                format!("node{}", i),
                4,
                16.0,
                i % 2 == 0,
                100.0,
                100.0,
            );
            registry.register(commitment).unwrap();
        }

        let stats = registry.stats();
        assert_eq!(stats.total_nodes, 5);
        assert_eq!(stats.active_nodes, 5);
        assert_eq!(stats.gpu_nodes, 3);
        assert_eq!(stats.total_cpu_cores, 20);
    }

    #[test]
    fn test_active_nodes_sorted() {
        let mut registry = ResourceRegistry::with_defaults();

        // Node with GPU should rank higher
        registry.register(ResourceCommitment::new("gpu".to_string(), 4, 16.0, true, 100.0, 100.0)).unwrap();
        registry.register(ResourceCommitment::new("cpu".to_string(), 4, 16.0, false, 100.0, 100.0)).unwrap();

        let active = registry.get_active_nodes();
        assert_eq!(active[0].node_id, "gpu");
        assert_eq!(active[1].node_id, "cpu");
    }
}
