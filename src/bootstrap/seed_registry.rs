//! Seed Registry - Descubrimiento y validación de seed nodes
//!
//! Lista inicial de nodos seed (hardcoded + DNS) con validación de salud
//! vía /api/health. Soporte para múltiples fuentes de descubrimiento.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
// CLEANUP: removed unused import Instant
use thiserror::Error;
use tracing::info;
// CLEANUP: removed unused imports error, warn

/// Error del seed registry
#[derive(Debug, Error)]
pub enum SeedRegistryError {
    #[error("No healthy seeds available")]
    NoHealthySeeds,
    #[error("DNS resolution failed for {host}: {msg}")]
    DnsResolutionFailed { host: String, msg: String },
    #[error("Health check failed for {addr}: {msg}")]
    HealthCheckFailed { addr: String, msg: String },
    #[error("Invalid seed address: {0}")]
    InvalidAddress(String),
    #[error("All seeds unhealthy")]
    AllSeedsUnhealthy,
}

/// Estado de salud del seed node
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SeedHealth {
    /// Respondiendo a health checks
    Healthy,
    /// Latencia alta o respuestas intermitentes
    Degraded,
    /// No responde o responde con error
    Unhealthy,
    /// No verificado aún
    Unknown,
}

impl std::fmt::Display for SeedHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SeedHealth::Healthy => write!(f, "Healthy"),
            SeedHealth::Degraded => write!(f, "Degraded"),
            SeedHealth::Unhealthy => write!(f, "Unhealthy"),
            SeedHealth::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Fuente de descubrimiento del seed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SeedSource {
    /// Hardcoded en el binario
    Hardcoded,
    /// Resuelto vía DNS
    Dns,
    /// Configurado por el usuario
    UserConfigured,
    /// Descubierto por la red (peer recommendation)
    PeerDiscovered,
}

impl std::fmt::Display for SeedSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SeedSource::Hardcoded => write!(f, "Hardcoded"),
            SeedSource::Dns => write!(f, "DNS"),
            SeedSource::UserConfigured => write!(f, "UserConfigured"),
            SeedSource::PeerDiscovered => write!(f, "PeerDiscovered"),
        }
    }
}

/// Entrada de seed node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedNode {
    /// Multiaddress libp2p (ej: "/ip4/.../tcp/.../p2p/...")
    pub multiaddress: String,
    /// Socket address para health check HTTP
    pub http_addr: Option<SocketAddr>,
    /// URL de health check
    pub health_url: String,
    /// Fuente de descubrimiento
    pub source: SeedSource,
    /// Estado de salud actual
    pub health: SeedHealth,
    /// Latencia promedio en ms
    pub avg_latency_ms: f64,
    /// Timestamp del último health check (epoch seconds)
    pub last_check: u64,
    /// Contador de checks exitosos consecutivos
    pub consecutive_successes: u32,
    /// Contador de fallos consecutivos
    pub consecutive_failures: u32,
    /// Peso para selección (más alto = más probable de ser elegido)
    pub weight: f64,
    /// Metadata adicional
    pub metadata: HashMap<String, String>,
}

impl SeedNode {
    pub fn new(multiaddress: String, http_addr: Option<SocketAddr>, source: SeedSource) -> Self {
        let health_url = http_addr
            .map(|addr| format!("http://{}/api/health", addr))
            .unwrap_or_else(|| "http://localhost/api/health".to_string());

        Self {
            multiaddress,
            http_addr,
            health_url,
            source,
            health: SeedHealth::Unknown,
            avg_latency_ms: 0.0,
            last_check: 0,
            consecutive_successes: 0,
            consecutive_failures: 0,
            weight: 1.0,
            metadata: HashMap::new(),
        }
    }

    /// Actualizar salud después de health check
    pub fn update_health(&mut self, is_healthy: bool, latency_ms: f64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.last_check = now;

        // Actualizar latencia promedio (exponential moving average)
        if self.avg_latency_ms == 0.0 {
            self.avg_latency_ms = latency_ms;
        } else {
            self.avg_latency_ms = 0.7 * self.avg_latency_ms + 0.3 * latency_ms;
        }

        if is_healthy {
            self.consecutive_successes += 1;
            self.consecutive_failures = 0;

            if self.consecutive_successes >= 3 {
                self.health = SeedHealth::Healthy;
                self.weight = (1.0 + self.consecutive_successes as f64 * 0.1)
                    / (1.0 + self.avg_latency_ms / 100.0);
            } else if self.health == SeedHealth::Unhealthy {
                self.health = SeedHealth::Degraded;
            }
        } else {
            self.consecutive_failures += 1;
            self.consecutive_successes = 0;

            if self.consecutive_failures >= 3 {
                self.health = SeedHealth::Unhealthy;
                self.weight = 0.1;
            } else if self.health == SeedHealth::Healthy {
                self.health = SeedHealth::Degraded;
            }
        }

        info!(
            address = %self.multiaddress,
            health = %self.health,
            latency_ms = self.avg_latency_ms,
            successes = self.consecutive_successes,
            failures = self.consecutive_failures,
            "Seed health updated"
        );
    }

    /// Verificar si el seed es usable
    pub fn is_usable(&self) -> bool {
        matches!(self.health, SeedHealth::Healthy | SeedHealth::Degraded)
    }
}

/// Seed nodes hardcoded por defecto
pub fn default_seed_nodes() -> Vec<SeedNode> {
    vec![
        // FIX: E0308 - parse().ok() returns Option, not the inner type. Use unwrap_or_else for literals.
        SeedNode::new(
            "/ip4/104.244.76.13/tcp/9001/p2p/12D3KooWSeedNode1".to_string(),
            Some("104.244.76.13:9001".parse().unwrap()),
            SeedSource::Hardcoded,
        ),
        SeedNode::new(
            "/ip4/185.199.108.133/tcp/9001/p2p/12D3KooWSeedNode2".to_string(),
            Some("185.199.108.133:9001".parse().unwrap()),
            SeedSource::Hardcoded,
        ),
        SeedNode::new(
            "/ip4/140.82.121.4/tcp/9001/p2p/12D3KooWSeedNode3".to_string(),
            Some("140.82.121.4:9001".parse().unwrap()),
            SeedSource::Hardcoded,
        ),
        SeedNode::new(
            "/dns/seed1.ed2kia.network/tcp/9001/p2p/12D3KooWSeedNode4".to_string(),
            None,
            SeedSource::Dns,
        ),
        SeedNode::new(
            "/dns/seed2.ed2kia.network/tcp/9001/p2p/12D3KooWSeedNode5".to_string(),
            None,
            SeedSource::Dns,
        ),
    ]
}

/// Gestor de seed registry
pub struct SeedRegistry {
    /// Seeds por multiaddress
    seeds: HashMap<String, SeedNode>,
    /// Intervalo de health checks
    health_check_interval: Duration,
    /// Timeout por health check
    health_check_timeout: Duration,
    /// Umínimo de seeds saludables
    minimum_healthy_seeds: usize,
}

impl SeedRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            seeds: HashMap::new(),
            health_check_interval: Duration::from_secs(60),
            health_check_timeout: Duration::from_secs(10),
            minimum_healthy_seeds: 2,
        };

        // Cargar seeds por defecto
        for seed in default_seed_nodes() {
            registry.seeds.insert(seed.multiaddress.clone(), seed);
        }

        info!(
            seed_count = registry.seeds.len(),
            "Seed registry initialized"
        );

        registry
    }

    /// Agregar seed custom
    // FIX: borrow/move - Clone addr before insert to avoid move | borrow/move
    pub fn add_seed(&mut self, seed: SeedNode) {
        let addr = seed.multiaddress.clone();
        self.seeds.insert(addr.clone(), seed);
        info!(address = %addr, "Custom seed added");
    }

    /// Remover seed
    pub fn remove_seed(&mut self, multiaddress: &str) -> Option<SeedNode> {
        let removed = self.seeds.remove(multiaddress);
        if removed.is_some() {
            info!(address = %multiaddress, "Seed removed");
        }
        removed
    }

    /// Obtener seeds saludables
    pub fn get_healthy_seeds(&self) -> Vec<&SeedNode> {
        self.seeds
            .values()
            .filter(|s| s.health == SeedHealth::Healthy)
            .collect()
    }

    /// Obtener seeds usables (healthy + degraded)
    pub fn get_usable_seeds(&self) -> Vec<&SeedNode> {
        self.seeds.values().filter(|s| s.is_usable()).collect()
    }

    /// Obtener seed con mejor peso (para conexión inicial)
    pub fn get_best_seed(&self) -> Option<&SeedNode> {
        self.seeds
            .values()
            .filter(|s| s.is_usable())
            .max_by(|a, b| {
                a.weight
                    .partial_cmp(&b.weight)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Seleccionar seed aleatorio ponderado
    pub fn select_weighted_seed(&self) -> Option<&SeedNode> {
        let usable: Vec<&SeedNode> = self.seeds.values().filter(|s| s.is_usable()).collect();
        if usable.is_empty() {
            return None;
        }

        let total_weight: f64 = usable.iter().map(|s| s.weight).sum();
        // FIX: E0308 - usable.first() returns Option<&&SeedNode>, need Option<&SeedNode>
        if total_weight <= 0.0 {
            return usable.first().copied(); // CLEANUP: map(|v| *v) -> copied()
        }

        // Simple weighted selection
        let mut rng = fastrand::f64() * total_weight;
        for seed in &usable {
            rng -= seed.weight;
            if rng <= 0.0 {
                return Some(seed);
            }
        }

        Some(usable.last().unwrap())
    }

    /// Ejecutar health checks en todos los seeds
    ///
    /// TODO: Phase 6 - Implementar health checks reales con reqwest async
    // FIX: borrow/move - Extract health check results before mutable iteration | borrow/move
    pub fn run_health_checks(&mut self) -> usize {
        let mut checked = 0;

        // Collect health check results first to avoid immutable borrow during mutable iteration
        let health_results: Vec<(String, bool)> = self
            .seeds
            .values()
            .map(|seed| (seed.multiaddress.clone(), self.simulate_health_check(seed)))
            .collect();

        for (addr, is_healthy) in health_results {
            if let Some(seed) = self.seeds.get_mut(&addr) {
                let latency = if is_healthy {
                    fastrand::f64() * 200.0 + 10.0
                } else {
                    0.0
                };

                seed.update_health(is_healthy, latency);
                checked += 1;
            }
        }

        info!(checked, "Health checks completed");
        checked
    }

    /// Simular health check (placeholder para integración real)
    fn simulate_health_check(&self, seed: &SeedNode) -> bool {
        // TODO: Phase 6 - Implementar con reqwest:
        // reqwest::get(&seed.health_url).await
        //     .map(|r| r.status().is_success())
        //     .unwrap_or(false)

        // Por ahora, seeds hardcoded son asumidos como disponibles
        matches!(seed.source, SeedSource::Hardcoded)
    }

    /// Verificar que hay suficientes seeds saludables
    pub fn has_minimum_seeds(&self) -> bool {
        let healthy_count = self.get_healthy_seeds().len();
        healthy_count >= self.minimum_healthy_seeds
    }

    /// Obtener seed para conexión inicial
    pub fn get_bootstrap_seed(&self) -> Result<SeedNode, SeedRegistryError> {
        // Intentar mejor seed primero
        if let Some(seed) = self.get_best_seed() {
            return Ok(seed.clone());
        }

        // Intentar cualquier seed usable
        if let Some(seed) = self.select_weighted_seed() {
            return Ok(seed.clone());
        }

        // Si todos unknown, intentar el primero
        if let Some(seed) = self.seeds.values().next() {
            return Ok(seed.clone());
        }

        Err(SeedRegistryError::NoHealthySeeds)
    }

    /// Estadísticas del registry
    pub fn stats(&self) -> SeedRegistryStats {
        let total = self.seeds.len();
        let healthy = self
            .seeds
            .values()
            .filter(|s| s.health == SeedHealth::Healthy)
            .count();
        let degraded = self
            .seeds
            .values()
            .filter(|s| s.health == SeedHealth::Degraded)
            .count();
        let unhealthy = self
            .seeds
            .values()
            .filter(|s| s.health == SeedHealth::Unhealthy)
            .count();
        let unknown = self
            .seeds
            .values()
            .filter(|s| s.health == SeedHealth::Unknown)
            .count();

        let avg_latency = if healthy > 0 {
            self.get_healthy_seeds()
                .iter()
                .map(|s| s.avg_latency_ms)
                .sum::<f64>()
                / healthy as f64
        } else {
            0.0
        };

        SeedRegistryStats {
            total,
            healthy,
            degraded,
            unhealthy,
            unknown,
            avg_latency_ms: avg_latency,
            minimum_required: self.minimum_healthy_seeds,
            has_minimum: healthy >= self.minimum_healthy_seeds,
        }
    }

    /// Listar todos los seeds
    pub fn list_all(&self) -> Vec<&SeedNode> {
        self.seeds.values().collect()
    }
}

impl Default for SeedRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Estadísticas del seed registry
#[derive(Debug, Serialize, Deserialize)]
pub struct SeedRegistryStats {
    pub total: usize,
    pub healthy: usize,
    pub degraded: usize,
    pub unhealthy: usize,
    pub unknown: usize,
    pub avg_latency_ms: f64,
    pub minimum_required: usize,
    pub has_minimum: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_seeds() {
        let seeds = default_seed_nodes();
        assert_eq!(seeds.len(), 5);
        assert!(seeds.iter().all(|s| s.health == SeedHealth::Unknown));
    }

    #[test]
    fn test_seed_registry_init() {
        let registry = SeedRegistry::new();
        assert!(registry.seeds.len() >= 5);
    }

    #[test]
    fn test_seed_health_update() {
        let mut seed = SeedNode::new(
            "/ip4/127.0.0.1/tcp/9001/p2p/test".to_string(),
            None,
            SeedSource::UserConfigured,
        );

        // 3 successes -> Healthy
        seed.update_health(true, 50.0);
        seed.update_health(true, 45.0);
        seed.update_health(true, 55.0);
        assert_eq!(seed.health, SeedHealth::Healthy);

        // 3 failures -> Unhealthy
        seed.update_health(false, 0.0);
        seed.update_health(false, 0.0);
        seed.update_health(false, 0.0);
        assert_eq!(seed.health, SeedHealth::Unhealthy);
    }

    #[test]
    fn test_seed_registry_stats() {
        let registry = SeedRegistry::new();
        let stats = registry.stats();
        assert_eq!(stats.total, registry.seeds.len());
    }
}
